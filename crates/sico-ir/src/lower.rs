use std::collections::{BTreeMap, BTreeSet};

use sico_hir::{Declaration, HirToken, LineKind, lower};
use sico_lexer::TokenKind;
use sico_parser::DeclarationKind;
use sico_semantics::{AnalyzeError, SemanticDiagnostic};
use sico_source::{SourceFile, TextRange};

use crate::{
    Block, BlockId, ConstructField, EntryError, Function, FunctionId, Instruction, MatchArm,
    Module, Operation, Parameter, Pattern, SourceRange, Terminator, Type, ValueId, VerifyError,
    require_semantic_success, verify,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreLowerError {
    Frontend(AnalyzeError),
    Semantic(Vec<SemanticDiagnostic>),
    Unsupported { feature: String, range: SourceRange },
    InvalidIr(Vec<VerifyError>),
}

#[derive(Clone)]
struct Signature {
    id: FunctionId,
    parameters: Vec<(String, Type, SourceRange)>,
    return_type: Type,
    async_function: bool,
}

#[derive(Clone)]
struct MethodSignature {
    parameters: Vec<Type>,
    return_type: Type,
}

#[derive(Default)]
struct Definitions {
    functions: BTreeMap<String, Signature>,
    constructors: BTreeMap<String, Type>,
    variants: BTreeMap<String, Type>,
    fields: BTreeMap<(String, String), Type>,
    capabilities: BTreeSet<String>,
    resources: BTreeSet<String>,
    methods: BTreeMap<(String, String), MethodSignature>,
    /// User-declared newtype/record/enum names that shadow profile types
    /// such as `Bytes`/`List` (STEP-0080).
    declared_types: BTreeSet<String>,
}

struct FunctionBuilder<'a> {
    definitions: &'a Definitions,
    next_value: u32,
    parameter_count: u32,
    bindings: BTreeMap<String, (ValueId, Type)>,
    declared_effects: Vec<String>,
}

/// Lowers the STEP-0031 core subset after the full M2 gate succeeds.
///
/// Evaluation follows RFC-0009 source order. Unsupported constructs return a
/// typed refusal and never produce partial IR.
///
/// # Errors
///
/// Returns frontend/semantic diagnostics, a typed unsupported feature, or
/// independent verifier errors.
pub fn lower_core(source: &SourceFile) -> Result<Module, CoreLowerError> {
    match require_semantic_success(source) {
        Ok(_) => {}
        Err(EntryError::Frontend(error)) => return Err(CoreLowerError::Frontend(error)),
        Err(EntryError::Semantic(diagnostics)) => {
            return Err(CoreLowerError::Semantic(diagnostics));
        }
    }
    let hir =
        lower(source).map_err(|error| CoreLowerError::Frontend(AnalyzeError::Lower(error)))?;
    let definitions = Definitions::from_declarations(&hir.declarations);
    let mut module = Module::new(source.name(), source.len().into());
    for declaration in &hir.declarations {
        if declaration.kind == DeclarationKind::Function {
            module
                .functions
                .push(lower_function(declaration, &definitions)?);
        }
    }
    if module.functions.is_empty() {
        return unsupported(
            "module without core function",
            TextRange::up_to(source.len()),
        );
    }
    let errors = verify(&module);
    if errors.is_empty() {
        Ok(module)
    } else {
        Err(CoreLowerError::InvalidIr(errors))
    }
}

impl Definitions {
    fn from_declarations(declarations: &[Declaration]) -> Self {
        let mut definitions = Self::default();
        for declaration in declarations {
            match declaration.kind {
                DeclarationKind::Newtype | DeclarationKind::Record => {
                    definitions.constructors.insert(
                        declaration.name.clone(),
                        Type::Named(declaration.name.clone()),
                    );
                    definitions.declared_types.insert(declaration.name.clone());
                }
                DeclarationKind::Capability => {
                    definitions.capabilities.insert(declaration.name.clone());
                }
                DeclarationKind::Resource => {
                    definitions.resources.insert(declaration.name.clone());
                }
                DeclarationKind::Enum => {
                    definitions.declared_types.insert(declaration.name.clone());
                }
                DeclarationKind::Function | DeclarationKind::Interface => {}
            }
        }
        let mut next_function = 1_u32;
        for declaration in declarations {
            match declaration.kind {
                DeclarationKind::Newtype | DeclarationKind::Record => {
                    if declaration.kind == DeclarationKind::Record {
                        for line in &declaration.lines {
                            if line.kind == LineKind::Field && line.tokens.len() >= 4 {
                                let ty = definitions.resolve_type(parse_type(&line.tokens[3..]));
                                definitions.fields.insert(
                                    (declaration.name.clone(), line.tokens[1].text.clone()),
                                    ty,
                                );
                            }
                        }
                    }
                }
                DeclarationKind::Enum => {
                    for line in &declaration.lines {
                        if line.kind == LineKind::Variant && line.tokens.len() >= 2 {
                            definitions.variants.insert(
                                format!("{}.{}", declaration.name, line.tokens[1].text),
                                Type::Named(declaration.name.clone()),
                            );
                        }
                    }
                }
                DeclarationKind::Function => {
                    let header = &declaration.lines[0].tokens;
                    let (mut parameters, return_type) = parse_signature(header);
                    for (_, ty, _) in &mut parameters {
                        *ty = definitions.resolve_type(ty.clone());
                    }
                    let return_type = definitions.resolve_type(return_type);
                    definitions.functions.insert(
                        declaration.name.clone(),
                        Signature {
                            id: FunctionId(next_function),
                            parameters,
                            return_type,
                            async_function: header
                                .iter()
                                .any(|token| token.kind == TokenKind::Async),
                        },
                    );
                    next_function += 1;
                }
                DeclarationKind::Capability
                | DeclarationKind::Resource
                | DeclarationKind::Interface => {
                    for line in &declaration.lines {
                        if line.kind != LineKind::FunctionSignature {
                            continue;
                        }
                        let method = method_name(&line.tokens);
                        let parameters = parse_method_parameters(&line.tokens)
                            .into_iter()
                            .map(|ty| definitions.resolve_type(ty))
                            .collect();
                        let return_type = definitions.resolve_type(parse_return_type(&line.tokens));
                        definitions.methods.insert(
                            (declaration.name.clone(), method),
                            MethodSignature {
                                parameters,
                                return_type,
                            },
                        );
                    }
                }
            }
        }
        definitions
    }

    fn resolve_type(&self, ty: Type) -> Type {
        match ty {
            Type::Named(name) if self.capabilities.contains(&name) => Type::Capability(name),
            Type::Named(name) if self.resources.contains(&name) => Type::OwnedResource(name),
            Type::Bytes if self.declared_types.contains("Bytes") => Type::Named("Bytes".into()),
            Type::List(_) if self.declared_types.contains("List") => Type::Named("List".into()),
            Type::Option(value) => Type::Option(Box::new(self.resolve_type(*value))),
            Type::Result { ok, error } => Type::Result {
                ok: Box::new(self.resolve_type(*ok)),
                error: Box::new(self.resolve_type(*error)),
            },
            Type::Task(value) => Type::Task(Box::new(self.resolve_type(*value))),
            Type::Future(value) => Type::Future(Box::new(self.resolve_type(*value))),
            Type::Stream(value) => Type::Stream(Box::new(self.resolve_type(*value))),
            Type::List(value) => Type::List(Box::new(self.resolve_type(*value))),
            other => other,
        }
    }
}

fn lower_function(
    declaration: &Declaration,
    definitions: &Definitions,
) -> Result<Function, CoreLowerError> {
    let signature = &definitions.functions[&declaration.name];
    // Sequential executor (STEP-0087): async functions lower as ordinary
    // functions; `spawn` is an eager call and `await` an identity copy.
    // The structured discipline (await-once, no task escaping its scope) is
    // already enforced by semantic analysis before lowering.
    let _ = signature.async_function;
    let parameters: Vec<_> = signature
        .parameters
        .iter()
        .enumerate()
        .map(|(index, (name, ty, range))| Parameter {
            id: ValueId(u32::try_from(index).expect("source limits bound parameters")),
            name: name.clone(),
            ty: ty.clone(),
            range: *range,
        })
        .collect();
    let mut effects = parse_effects(declaration)?;
    effects.sort();
    effects.dedup();
    let bindings = parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), (parameter.id, parameter.ty.clone())))
        .collect();
    let mut builder = FunctionBuilder {
        definitions,
        next_value: u32::try_from(parameters.len()).expect("source limits bound parameters"),
        parameter_count: u32::try_from(parameters.len()).expect("source limits bound parameters"),
        bindings,
        declared_effects: effects.clone(),
    };
    let match_index = declaration
        .lines
        .iter()
        .position(|line| line.kind == LineKind::Match);
    let if_index = declaration
        .lines
        .iter()
        .position(|line| line.kind == LineKind::If);
    let blocks = if let Some(index) = if_index {
        lower_revision_if(declaration, index, &mut builder, &signature.return_type)?
    } else if let Some(index) = match_index {
        lower_match(declaration, index, &mut builder, &signature.return_type)?
    } else {
        vec![lower_straight_line(
            declaration,
            &mut builder,
            &signature.return_type,
        )?]
    };
    Ok(Function {
        id: signature.id,
        name: declaration.name.clone(),
        parameters,
        return_type: signature.return_type.clone(),
        effects,
        entry: BlockId(0),
        blocks,
        range: source_range(declaration.range),
    })
}

fn lower_straight_line(
    declaration: &Declaration,
    builder: &mut FunctionBuilder<'_>,
    return_type: &Type,
) -> Result<Block, CoreLowerError> {
    let mut instructions = Vec::new();
    let mut terminator = None;
    let mut using_resource: Option<ValueId> = None;
    let mut metadata_lines = false;
    for line in declaration.lines.iter().skip(1) {
        match line.kind {
            LineKind::Let => {
                let equal = position(&line.tokens, TokenKind::Equal)
                    .ok_or_else(|| unsupported_error("let without value", line.range))?;
                let name = line.tokens.get(1).map_or("", |token| token.text.as_str());
                let (value, ty) =
                    builder.expression(&line.tokens[equal + 1..], None, &mut instructions)?;
                builder.bindings.insert(name.to_owned(), (value, ty));
            }
            LineKind::Return => {
                metadata_lines = false;
                if *return_type == Type::Unit
                    && (line.tokens.len() == 1
                        || line.tokens.get(1).is_some_and(|token| token.text == "Unit"))
                {
                    if let Some(resource) = using_resource.take() {
                        builder.emit(
                            Type::Unit,
                            Operation::ResourceDrop(resource),
                            line.range,
                            &mut instructions,
                        );
                    }
                    terminator = Some(Terminator::Return(None));
                } else {
                    let (value, _) = builder.expression(
                        &line.tokens[1..],
                        Some(return_type),
                        &mut instructions,
                    )?;
                    if let Some(resource) = using_resource.take() {
                        builder.emit(
                            Type::Unit,
                            Operation::ResourceDrop(resource),
                            line.range,
                            &mut instructions,
                        );
                    }
                    terminator = Some(Terminator::Return(Some(value)));
                }
            }
            LineKind::Effects | LineKind::Capabilities => {
                metadata_lines = !line
                    .tokens
                    .iter()
                    .any(|token| token.kind == TokenKind::None);
            }
            // Sequential executor (STEP-0087): a task group is a
            // straight-line scope; semantic analysis already enforced
            // its structure.
            LineKind::End
            | LineKind::DeclarationHeader
            | LineKind::FunctionSignature
            | LineKind::TaskGroup => {}
            LineKind::If => return unsupported("nested if control flow", line.range),
            LineKind::Using => {
                let Some(name) = line.tokens.get(1) else {
                    return unsupported("using scope", line.range);
                };
                let Some((value, Type::OwnedResource(_))) = builder.bindings.get(&name.text) else {
                    return unsupported("using non-resource", line.range);
                };
                using_resource = Some(*value);
            }
            LineKind::Match | LineKind::MatchArm => {
                return unsupported("nested match", line.range);
            }
            LineKind::Expression if metadata_lines => {}
            LineKind::Expression => {
                let (_, ty) = builder.expression(&line.tokens, None, &mut instructions)?;
                if ty != Type::Unit {
                    return unsupported("non-Unit expression statement", line.range);
                }
            }
            LineKind::Field | LineKind::Invariant | LineKind::Variant => {
                return unsupported("non-function line", line.range);
            }
        }
    }
    let terminator = terminator.unwrap_or(Terminator::Unreachable);
    Ok(Block {
        id: BlockId(0),
        instructions,
        terminator,
        range: source_range(declaration.range),
    })
}

#[allow(clippy::too_many_lines)]
fn lower_match(
    declaration: &Declaration,
    match_index: usize,
    builder: &mut FunctionBuilder<'_>,
    return_type: &Type,
) -> Result<Vec<Block>, CoreLowerError> {
    if declaration.lines[1..match_index]
        .iter()
        .any(|line| !matches!(line.kind, LineKind::Effects | LineKind::Capabilities))
    {
        return unsupported("pre-match statements", declaration.lines[match_index].range);
    }
    let match_line = &declaration.lines[match_index];
    let expressions = strip_outer_parens(&match_line.tokens[1..match_line.tokens.len() - 1]);
    let expression_parts = split_top_level(expressions, TokenKind::Comma);
    let mut entry_instructions = Vec::new();
    let mut values = Vec::new();
    let mut value_types = Vec::new();
    for expression in expression_parts {
        let (value, ty) = builder.expression(expression, None, &mut entry_instructions)?;
        values.push(value);
        value_types.push(ty);
    }

    let mut blocks = Vec::new();
    let mut arms = Vec::new();
    let mut cursor = match_index + 1;
    while cursor < declaration.lines.len() {
        let line = &declaration.lines[cursor];
        if line.kind == LineKind::End {
            break;
        }
        if line.kind != LineKind::MatchArm {
            return unsupported("match arm body", line.range);
        }
        let pattern_tokens = strip_outer_parens(&line.tokens[1..line.tokens.len() - 1]);
        let patterns: Vec<_> = split_top_level(pattern_tokens, TokenKind::Comma)
            .into_iter()
            .map(parse_pattern)
            .collect();
        // `case ok(x)` / `case error(x)` bind the variant payload through a
        // Project instruction; every other binding shape stays refused.
        let mut payload_binding = None;
        for pattern in &patterns {
            let single_result_binding = match pattern {
                Pattern::Variant { name, payload } if matches!(name.as_str(), "ok" | "error") => {
                    match payload.as_slice() {
                        [Pattern::Binding(name)] => Some((pattern, name.clone())),
                        _ => None,
                    }
                }
                _ => None,
            };
            match single_result_binding {
                Some((pattern, name)) if patterns.len() == 1 && payload_binding.is_none() => {
                    let _ = pattern;
                    payload_binding = Some(name);
                }
                _ if pattern_binds(pattern) => {
                    return unsupported("match payload binding", line.range);
                }
                _ => {}
            }
        }
        cursor += 1;
        let Some(body) = declaration.lines.get(cursor) else {
            return unsupported("empty match arm", line.range);
        };
        if body.kind != LineKind::Return {
            return unsupported("non-return match arm", body.range);
        }
        let mut instructions = Vec::new();
        if let Some(binding) = payload_binding {
            let [Pattern::Variant { name: variant, .. }] = patterns.as_slice() else {
                return unsupported("match payload binding", line.range);
            };
            let (Some(Type::Result { ok, error }), [base]) =
                (value_types.first(), values.as_slice())
            else {
                return unsupported("match payload binding on non-Result", line.range);
            };
            // IR values are block-scoped: only a match subject that is a
            // function parameter stays visible inside the arm block.
            if base.0 >= builder.parameter_count {
                return unsupported("match payload binding", line.range);
            }
            let payload_type = match variant.as_str() {
                "ok" => ok.as_ref().clone(),
                _ => error.as_ref().clone(),
            };
            let value = builder.emit(
                payload_type.clone(),
                Operation::Project {
                    base: *base,
                    field: variant.clone(),
                },
                line.range,
                &mut instructions,
            );
            builder.bindings.insert(binding, (value, payload_type));
        }
        let (value, _) =
            builder.expression(&body.tokens[1..], Some(return_type), &mut instructions)?;
        let target = BlockId(u32::try_from(blocks.len() + 1).expect("source limits bound blocks"));
        arms.push(MatchArm {
            patterns,
            target,
            range: source_range(line.range),
        });
        blocks.push(Block {
            id: target,
            instructions,
            terminator: Terminator::Return(Some(value)),
            range: source_range(TextRange::new(line.range.start(), body.range.end())),
        });
        cursor += 1;
    }
    let entry = Block {
        id: BlockId(0),
        instructions: entry_instructions,
        terminator: Terminator::Match { values, arms },
        range: source_range(match_line.range),
    };
    blocks.insert(0, entry);
    Ok(blocks)
}

fn lower_revision_if(
    declaration: &Declaration,
    if_index: usize,
    builder: &mut FunctionBuilder<'_>,
    return_type: &Type,
) -> Result<Vec<Block>, CoreLowerError> {
    if declaration.lines[1..if_index].iter().any(|line| {
        !matches!(
            line.kind,
            LineKind::Effects | LineKind::Capabilities | LineKind::Expression
        )
    }) {
        return unsupported("pre-if statements", declaration.lines[if_index].range);
    }
    let if_line = &declaration.lines[if_index];
    let condition_tokens = &if_line.tokens[1..if_line.tokens.len() - 1];
    let Some(equal) = top_level_position(condition_tokens, TokenKind::EqualEqual) else {
        return unsupported("non-revision if condition", if_line.range);
    };
    let mut entry_instructions = Vec::new();
    let (left, left_type) =
        builder.expression(&condition_tokens[..equal], None, &mut entry_instructions)?;
    let (right, right_type) = builder.expression(
        &condition_tokens[equal + 1..],
        None,
        &mut entry_instructions,
    )?;
    if left_type != right_type || !matches!(left_type, Type::Named(ref name) if name == "Revision")
    {
        return unsupported("non-revision equality guard", if_line.range);
    }
    let condition = builder.emit(
        Type::Bool,
        Operation::RevisionCheck {
            value: left,
            expected: right,
        },
        token_range(condition_tokens),
        &mut entry_instructions,
    );
    let Some(then_line) = declaration.lines.get(if_index + 1) else {
        return unsupported("empty revision branch", if_line.range);
    };
    if then_line.kind != LineKind::Return {
        return unsupported("non-return revision branch", then_line.range);
    }
    let end_index = declaration
        .lines
        .iter()
        .enumerate()
        .skip(if_index + 2)
        .find_map(|(index, line)| (line.kind == LineKind::End).then_some(index))
        .ok_or_else(|| unsupported_error("unterminated revision branch", if_line.range))?;
    let Some(else_line) = declaration.lines.get(end_index + 1) else {
        return unsupported("missing revision fallback", if_line.range);
    };
    if else_line.kind != LineKind::Return {
        return unsupported("non-return revision fallback", else_line.range);
    }
    let mut then_instructions = Vec::new();
    let then_value = builder
        .expression(
            &then_line.tokens[1..],
            Some(return_type),
            &mut then_instructions,
        )?
        .0;
    let mut else_instructions = Vec::new();
    let else_value = builder
        .expression(
            &else_line.tokens[1..],
            Some(return_type),
            &mut else_instructions,
        )?
        .0;
    Ok(vec![
        Block {
            id: BlockId(0),
            instructions: entry_instructions,
            terminator: Terminator::Branch {
                condition,
                then_block: BlockId(1),
                else_block: BlockId(2),
            },
            range: source_range(if_line.range),
        },
        Block {
            id: BlockId(1),
            instructions: then_instructions,
            terminator: Terminator::Return(Some(then_value)),
            range: source_range(then_line.range),
        },
        Block {
            id: BlockId(2),
            instructions: else_instructions,
            terminator: Terminator::Return(Some(else_value)),
            range: source_range(else_line.range),
        },
    ])
}

impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_lines)]
    fn expression(
        &mut self,
        tokens: &[HirToken],
        expected: Option<&Type>,
        output: &mut Vec<Instruction>,
    ) -> Result<(ValueId, Type), CoreLowerError> {
        let tokens = strip_outer_parens(tokens);
        if tokens
            .first()
            .is_some_and(|token| token.kind == TokenKind::Spawn)
        {
            // Sequential executor (STEP-0087): spawn is an eager call; the
            // bound value is the async function's inner result.
            return self.expression(&tokens[1..], expected, output);
        }
        if tokens
            .first()
            .is_some_and(|token| token.kind == TokenKind::Await)
        {
            // Sequential executor: await is the identity over the eagerly
            // produced value (completion already happened at spawn).
            return self.expression(&tokens[1..], expected, output);
        }
        if tokens
            .first()
            .is_some_and(|token| token.kind == TokenKind::Try)
        {
            let (source, source_type) = self.expression(&tokens[1..], None, output)?;
            let Type::Result { ok, .. } = source_type else {
                return unsupported("try on non-Result", token_range(tokens));
            };
            let ty = *ok;
            let value = self.emit(
                ty.clone(),
                Operation::Try(source),
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
        }
        if let Some(index) = top_level_position(tokens, TokenKind::Plus) {
            let (left, _) = self.expression(&tokens[..index], Some(&Type::Int), output)?;
            let (right, _) = self.expression(&tokens[index + 1..], Some(&Type::Int), output)?;
            let value = self.emit(
                Type::Int,
                Operation::AddInt { left, right },
                token_range(tokens),
                output,
            );
            return Ok((value, Type::Int));
        }
        if tokens.len() == 1 {
            let token = &tokens[0];
            return match token.kind {
                TokenKind::Integer => {
                    let value = self.emit(
                        Type::Int,
                        Operation::ConstInt(token.text.clone()),
                        token.range,
                        output,
                    );
                    Ok((value, Type::Int))
                }
                TokenKind::String => {
                    let value = self.emit(
                        Type::String,
                        Operation::ConstString(unquote(&token.text)),
                        token.range,
                        output,
                    );
                    Ok((value, Type::String))
                }
                TokenKind::True | TokenKind::False => {
                    let value = self.emit(
                        Type::Bool,
                        Operation::ConstBool(token.kind == TokenKind::True),
                        token.range,
                        output,
                    );
                    Ok((value, Type::Bool))
                }
                TokenKind::Identifier => self.bindings.get(&token.text).cloned().ok_or_else(|| {
                    unsupported_error(format!("unresolved value {}", token.text), token.range)
                }),
                _ => Err(unsupported_error("atomic expression", token.range)),
            };
        }
        if let Some(open) = top_level_call_open(tokens) {
            return self.call(tokens, open, expected, output);
        }
        if tokens.len() == 3 && tokens[1].kind == TokenKind::Dot {
            let joined = format!("{}.{}", tokens[0].text, tokens[2].text);
            if let Some(ty) = self.definitions.variants.get(&joined).cloned() {
                let value = self.emit(
                    ty.clone(),
                    Operation::Variant {
                        name: joined,
                        payload: Vec::new(),
                    },
                    token_range(tokens),
                    output,
                );
                return Ok((value, ty));
            }
            if let Some((base, base_type)) = self.bindings.get(&tokens[0].text).cloned() {
                let Type::Named(record) = base_type else {
                    return unsupported("field projection", token_range(tokens));
                };
                let Some(ty) = self
                    .definitions
                    .fields
                    .get(&(record, tokens[2].text.clone()))
                    .cloned()
                else {
                    return unsupported("unknown field projection", token_range(tokens));
                };
                let value = self.emit(
                    ty.clone(),
                    Operation::Project {
                        base,
                        field: tokens[2].text.clone(),
                    },
                    token_range(tokens),
                    output,
                );
                return Ok((value, ty));
            }
        }
        Err(unsupported_error("core expression", token_range(tokens)))
    }

    #[allow(clippy::too_many_lines)]
    fn call(
        &mut self,
        tokens: &[HirToken],
        open: usize,
        expected: Option<&Type>,
        output: &mut Vec<Instruction>,
    ) -> Result<(ValueId, Type), CoreLowerError> {
        let callee = join_path(&tokens[..open]);
        let arguments = split_top_level(&tokens[open + 1..tokens.len() - 1], TokenKind::Comma);
        if matches!(callee.as_str(), "I64.literal" | "U64.literal") {
            let [argument] = arguments.as_slice() else {
                return unsupported("fixed literal arity", token_range(tokens));
            };
            let literal = strip_outer_parens(argument_value(argument));
            let (text, range) = match literal {
                [token] if token.kind == TokenKind::Integer => (token.text.clone(), token.range),
                [minus, token]
                    if minus.kind == TokenKind::Minus && token.kind == TokenKind::Integer =>
                {
                    (format!("-{}", token.text), token_range(literal))
                }
                _ => return unsupported("non-literal fixed conversion", token_range(tokens)),
            };
            return if callee.starts_with("I64") {
                let parsed = text
                    .parse::<i64>()
                    .map_err(|_| unsupported_error("I64 literal outside range", range))?;
                let value = self.emit(Type::I64, Operation::ConstI64(parsed), range, output);
                Ok((value, Type::I64))
            } else {
                let parsed = text
                    .parse::<u64>()
                    .map_err(|_| unsupported_error("U64 literal outside range", range))?;
                let value = self.emit(Type::U64, Operation::ConstU64(parsed), range, output);
                Ok((value, Type::U64))
            };
        }
        if let Some((target, operation)) = callee.split_once('.')
            && matches!(target, "I64" | "U64")
            && matches!(
                operation,
                "checked_add" | "checked_sub" | "equal" | "less_than"
            )
        {
            let [left, right] = arguments.as_slice() else {
                return unsupported("fixed intrinsic arity", token_range(tokens));
            };
            let fixed = if target == "I64" {
                Type::I64
            } else {
                Type::U64
            };
            let left = self
                .expression(argument_value(left), Some(&fixed), output)?
                .0;
            let right = self
                .expression(argument_value(right), Some(&fixed), output)?
                .0;
            let (ty, operation) = match operation {
                "checked_add" => (
                    checked_fixed_result(fixed.clone()),
                    Operation::CheckedAdd { left, right },
                ),
                "checked_sub" => (
                    checked_fixed_result(fixed.clone()),
                    Operation::CheckedSub { left, right },
                ),
                "equal" => (Type::Bool, Operation::EqualFixed { left, right }),
                "less_than" => (Type::Bool, Operation::LessFixed { left, right }),
                _ => unreachable!("matched fixed intrinsic above"),
            };
            let value = self.emit(ty.clone(), operation, token_range(tokens), output);
            return Ok((value, ty));
        }
        if callee.starts_with("sico.")
            && let Some((parameters, result)) = crate::intrinsic_signature(&callee)
        {
            if parameters.len() != arguments.len() {
                return unsupported("stdlib intrinsic arity", token_range(tokens));
            }
            let mut values = Vec::with_capacity(arguments.len());
            for (argument, parameter) in arguments.iter().zip(&parameters) {
                values.push(
                    self.expression(argument_value(argument), Some(parameter), output)?
                        .0,
                );
            }
            let value = self.emit(
                result.clone(),
                Operation::Intrinsic {
                    name: callee,
                    arguments: values,
                },
                token_range(tokens),
                output,
            );
            return Ok((value, result));
        }
        if let Some(signature) = self.definitions.functions.get(&callee) {
            if arguments.len() != signature.parameters.len() {
                return unsupported("call arity", token_range(tokens));
            }
            let mut values = Vec::new();
            for (argument, (_, ty, _)) in arguments.iter().zip(&signature.parameters) {
                values.push(
                    self.expression(argument_value(argument), Some(ty), output)?
                        .0,
                );
            }
            let ty = signature.return_type.clone();
            let value = self.emit(
                ty.clone(),
                Operation::Call {
                    function: signature.id,
                    arguments: values,
                },
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
        }
        if callee == "Float64.from_int" {
            let [argument] = arguments.as_slice() else {
                return unsupported("Float64.from_int arity", token_range(tokens));
            };
            let argument = self
                .expression(argument_value(argument), Some(&Type::Int), output)?
                .0;
            let value = self.emit(
                Type::Float64,
                Operation::Intrinsic {
                    name: callee,
                    arguments: vec![argument],
                },
                token_range(tokens),
                output,
            );
            return Ok((value, Type::Float64));
        }
        if tokens[..open].len() == 3 && tokens[1].kind == TokenKind::Dot {
            let receiver_name = &tokens[0].text;
            let method = &tokens[2].text;
            if let Some((receiver, receiver_type)) = self.bindings.get(receiver_name).cloned() {
                let owner = match &receiver_type {
                    Type::Capability(name) | Type::OwnedResource(name) => name.clone(),
                    _ => String::new(),
                };
                if let Some(signature) = self
                    .definitions
                    .methods
                    .get(&(owner.clone(), method.clone()))
                    .cloned()
                {
                    if arguments.len() != signature.parameters.len() {
                        return unsupported("method call arity", token_range(tokens));
                    }
                    let mut values = Vec::new();
                    for (argument, ty) in arguments.iter().zip(&signature.parameters) {
                        values.push(
                            self.expression(argument_value(argument), Some(ty), output)?
                                .0,
                        );
                    }
                    match receiver_type {
                        Type::Capability(_) => {
                            values.insert(0, receiver);
                            let ty = signature.return_type;
                            let effect = self
                                .declared_effects
                                .iter()
                                .find(|effect| effect.starts_with(&format!("{receiver_name}.")))
                                .cloned()
                                .unwrap_or(callee);
                            let value = self.emit(
                                ty.clone(),
                                Operation::EffectCall {
                                    effect,
                                    arguments: values,
                                },
                                token_range(tokens),
                                output,
                            );
                            return Ok((value, ty));
                        }
                        Type::OwnedResource(_) if method == "close" => {
                            let value = self.emit(
                                Type::Unit,
                                Operation::ResourceDrop(receiver),
                                token_range(tokens),
                                output,
                            );
                            return Ok((value, Type::Unit));
                        }
                        Type::OwnedResource(name) => {
                            let borrowed_type = Type::BorrowedResource(name);
                            let borrowed = self.emit(
                                borrowed_type,
                                Operation::ResourceBorrow(receiver),
                                token_range(tokens),
                                output,
                            );
                            let ty = signature.return_type;
                            let value = self.emit(
                                ty.clone(),
                                Operation::ResourceCall {
                                    resource: borrowed,
                                    method: format!("{owner}.{method}"),
                                    arguments: values,
                                },
                                token_range(tokens),
                                output,
                            );
                            return Ok((value, ty));
                        }
                        _ => {}
                    }
                }
            }
        }
        if callee == "ok" {
            let Some(Type::Result { ok, .. }) = expected else {
                return unsupported("unconstrained ok", token_range(tokens));
            };
            let [argument] = arguments.as_slice() else {
                return unsupported("ok arity", token_range(tokens));
            };
            let argument = self
                .expression(argument_value(argument), Some(ok), output)?
                .0;
            let ty = expected.expect("checked above").clone();
            let value = self.emit(
                ty.clone(),
                Operation::Variant {
                    name: "ok".into(),
                    payload: vec![argument],
                },
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
        }
        if callee == "error" {
            let Some(Type::Result { error, .. }) = expected else {
                return unsupported("unconstrained error", token_range(tokens));
            };
            let [argument] = arguments.as_slice() else {
                return unsupported("error arity", token_range(tokens));
            };
            let argument = self
                .expression(argument_value(argument), Some(error), output)?
                .0;
            let ty = expected.expect("checked above").clone();
            let value = self.emit(
                ty.clone(),
                Operation::Variant {
                    name: "error".into(),
                    payload: vec![argument],
                },
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
        }
        if let Some(ty) = self.definitions.constructors.get(&callee).cloned() {
            let mut values = Vec::new();
            for (index, argument) in arguments.into_iter().enumerate() {
                let field =
                    named_argument(argument).map_or_else(|| format!("#{index}"), str::to_owned);
                let expected_field = named_argument(argument).and_then(|field| {
                    self.definitions
                        .fields
                        .get(&(callee.clone(), field.to_owned()))
                });
                values.push(ConstructField {
                    name: field,
                    value: self
                        .expression(argument_value(argument), expected_field, output)?
                        .0,
                });
            }
            let value = self.emit(
                ty.clone(),
                Operation::Construct {
                    name: callee,
                    fields: values,
                },
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
        }
        if let Some(ty) = self.definitions.variants.get(&callee).cloned() {
            let mut payload = Vec::new();
            for argument in arguments {
                payload.push(self.expression(argument_value(argument), None, output)?.0);
            }
            let value = self.emit(
                ty.clone(),
                Operation::Variant {
                    name: callee,
                    payload,
                },
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
        }
        Err(unsupported_error(
            format!("call target {callee}"),
            token_range(tokens),
        ))
    }

    fn emit(
        &mut self,
        ty: Type,
        operation: Operation,
        range: TextRange,
        output: &mut Vec<Instruction>,
    ) -> ValueId {
        let result = ValueId(self.next_value);
        self.next_value = self
            .next_value
            .checked_add(1)
            .expect("source limits bound value ids");
        output.push(Instruction {
            result,
            ty,
            operation,
            range: source_range(range),
        });
        result
    }
}

fn parse_signature(tokens: &[HirToken]) -> (Vec<(String, Type, SourceRange)>, Type) {
    let open = position(tokens, TokenKind::LeftParen).expect("parser guarantees function params");
    let close = matching_close(tokens, open).expect("parser balances params");
    let parameters = split_top_level(&tokens[open + 1..close], TokenKind::Comma)
        .into_iter()
        .filter(|tokens| !tokens.is_empty())
        .map(|tokens| {
            let colon = position(tokens, TokenKind::Colon).expect("parser guarantees param type");
            (
                tokens[0].text.clone(),
                parse_type(&tokens[colon + 1..]),
                source_range(tokens[0].range),
            )
        })
        .collect();
    let returns = position(tokens, TokenKind::Returns).expect("parser guarantees return type");
    let end = tokens
        .iter()
        .enumerate()
        .skip(returns + 1)
        .find_map(|(index, token)| (token.kind == TokenKind::Colon).then_some(index))
        .unwrap_or(tokens.len());
    (parameters, parse_type(&tokens[returns + 1..end]))
}

fn method_name(tokens: &[HirToken]) -> String {
    let function = position(tokens, TokenKind::Function).expect("method signature has function");
    tokens[function + 1].text.clone()
}

fn parse_method_parameters(tokens: &[HirToken]) -> Vec<Type> {
    let open = position(tokens, TokenKind::LeftParen).expect("method signature has params");
    let close = matching_close(tokens, open).expect("parser balances method params");
    split_top_level(&tokens[open + 1..close], TokenKind::Comma)
        .into_iter()
        .filter(|parameter| {
            !parameter.is_empty()
                && !parameter
                    .iter()
                    .any(|token| token.kind == TokenKind::SelfKeyword)
        })
        .filter_map(|parameter| {
            position(parameter, TokenKind::Colon).map(|colon| parse_type(&parameter[colon + 1..]))
        })
        .collect()
}

fn parse_return_type(tokens: &[HirToken]) -> Type {
    let returns = position(tokens, TokenKind::Returns).expect("signature has return type");
    let end = tokens
        .iter()
        .enumerate()
        .skip(returns + 1)
        .find_map(|(index, token)| (token.kind == TokenKind::Colon).then_some(index))
        .unwrap_or(tokens.len());
    parse_type(&tokens[returns + 1..end])
}

fn parse_type(tokens: &[HirToken]) -> Type {
    let Some(first) = tokens.first() else {
        return Type::Unit;
    };
    let name = first.text.as_str();
    let arguments = if tokens
        .get(1)
        .is_some_and(|token| token.kind == TokenKind::LeftBracket)
    {
        split_top_level(&tokens[2..tokens.len() - 1], TokenKind::Comma)
            .into_iter()
            .map(parse_type)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    match name {
        "Unit" => Type::Unit,
        "Bool" => Type::Bool,
        "Int" => Type::Int,
        "I64" => Type::I64,
        "U64" => Type::U64,
        "Float64" => Type::Float64,
        "Text" => Type::String,
        "Bytes" => Type::Bytes,
        "List" if arguments.len() == 1 => Type::List(Box::new(arguments[0].clone())),
        "Option" if arguments.len() == 1 => Type::Option(Box::new(arguments[0].clone())),
        "Result" if arguments.len() == 2 => Type::Result {
            ok: Box::new(arguments[0].clone()),
            error: Box::new(arguments[1].clone()),
        },
        "Task" if arguments.len() == 1 => Type::Task(Box::new(arguments[0].clone())),
        "Future" if arguments.len() == 1 => Type::Future(Box::new(arguments[0].clone())),
        "Stream" if !arguments.is_empty() => Type::Stream(Box::new(arguments[0].clone())),
        _ => Type::Named(name.to_owned()),
    }
}

fn checked_fixed_result(fixed: Type) -> Type {
    Type::Result {
        ok: Box::new(fixed),
        error: Box::new(Type::Named(crate::NUMERIC_ERROR_TYPE.to_owned())),
    }
}

fn parse_effects(declaration: &Declaration) -> Result<Vec<String>, CoreLowerError> {
    let mut effects = Vec::new();
    let mut collecting = false;
    for line in &declaration.lines {
        if line.kind == LineKind::Effects {
            collecting = true;
            let tail = &line.tokens[2..];
            if tail.iter().any(|token| token.kind == TokenKind::None) {
                collecting = false;
            } else if !tail.is_empty() {
                effects.push(join_path(tail));
                collecting = false;
            }
            continue;
        }
        if collecting {
            if line.kind == LineKind::Capabilities {
                collecting = false;
            } else if line.kind == LineKind::Expression {
                effects.push(join_path(&line.tokens));
            } else {
                return unsupported("effect declaration", line.range);
            }
        }
    }
    Ok(effects)
}

fn parse_pattern(tokens: &[HirToken]) -> Pattern {
    let tokens = strip_outer_parens(tokens);
    if tokens.len() == 1 {
        return match tokens[0].kind {
            TokenKind::Ignore | TokenKind::Underscore => Pattern::Wildcard,
            TokenKind::True => Pattern::Bool(true),
            TokenKind::False => Pattern::Bool(false),
            _ => Pattern::Binding(tokens[0].text.clone()),
        };
    }
    if let Some(open) = top_level_call_open(tokens) {
        let name = join_path(&tokens[..open]);
        let payload = split_top_level(&tokens[open + 1..tokens.len() - 1], TokenKind::Comma)
            .into_iter()
            .map(parse_pattern)
            .collect();
        return Pattern::Variant { name, payload };
    }
    Pattern::Variant {
        name: join_path(tokens),
        payload: Vec::new(),
    }
}

fn pattern_binds(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Binding(_) => true,
        Pattern::Variant { payload, .. } => payload.iter().any(pattern_binds),
        Pattern::Wildcard | Pattern::Bool(_) => false,
    }
}

fn split_top_level(tokens: &[HirToken], separator: TokenKind) -> Vec<&[HirToken]> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut depth = 0_i32;
    let mut start = 0;
    let mut result = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => depth += 1,
            TokenKind::RightParen | TokenKind::RightBracket => depth -= 1,
            _ if depth == 0 && token.kind == separator => {
                result.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    result.push(&tokens[start..]);
    result
}

fn strip_outer_parens(mut tokens: &[HirToken]) -> &[HirToken] {
    while tokens.len() >= 2
        && tokens[0].kind == TokenKind::LeftParen
        && matching_close(tokens, 0) == Some(tokens.len() - 1)
    {
        tokens = &tokens[1..tokens.len() - 1];
    }
    tokens
}

fn matching_close(tokens: &[HirToken], open: usize) -> Option<usize> {
    let (left, right) = match tokens.get(open)?.kind {
        TokenKind::LeftParen => (TokenKind::LeftParen, TokenKind::RightParen),
        TokenKind::LeftBracket => (TokenKind::LeftBracket, TokenKind::RightBracket),
        _ => return None,
    };
    let mut depth = 0_i32;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.kind == left {
            depth += 1;
        } else if token.kind == right {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn top_level_position(tokens: &[HirToken], kind: TokenKind) -> Option<usize> {
    let mut depth = 0_i32;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => depth += 1,
            TokenKind::RightParen | TokenKind::RightBracket => depth -= 1,
            _ if depth == 0 && token.kind == kind => return Some(index),
            _ => {}
        }
    }
    None
}

fn top_level_call_open(tokens: &[HirToken]) -> Option<usize> {
    let mut depth = 0_i32;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::LeftParen {
            if depth == 0 && matching_close(tokens, index) == Some(tokens.len() - 1) {
                return Some(index);
            }
            depth += 1;
        } else if token.kind == TokenKind::RightParen {
            depth -= 1;
        }
    }
    None
}

fn argument_value(tokens: &[HirToken]) -> &[HirToken] {
    named_argument(tokens).map_or(tokens, |_| {
        let colon = position(tokens, TokenKind::Colon).expect("named argument has colon");
        &tokens[colon + 1..]
    })
}

fn named_argument(tokens: &[HirToken]) -> Option<&str> {
    (tokens.len() >= 3 && tokens[1].kind == TokenKind::Colon).then(|| tokens[0].text.as_str())
}

fn position(tokens: &[HirToken], kind: TokenKind) -> Option<usize> {
    tokens.iter().position(|token| token.kind == kind)
}

fn join_path(tokens: &[HirToken]) -> String {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn token_range(tokens: &[HirToken]) -> TextRange {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => TextRange::new(first.range.start(), last.range.end()),
        _ => TextRange::empty(0.into()),
    }
}

fn source_range(range: TextRange) -> SourceRange {
    SourceRange::from_text_range(range)
}

fn unquote(text: &str) -> String {
    text.strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text)
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

fn unsupported<T>(feature: impl Into<String>, range: TextRange) -> Result<T, CoreLowerError> {
    Err(unsupported_error(feature, range))
}

fn unsupported_error(feature: impl Into<String>, range: TextRange) -> CoreLowerError {
    CoreLowerError::Unsupported {
        feature: feature.into(),
        range: source_range(range),
    }
}
