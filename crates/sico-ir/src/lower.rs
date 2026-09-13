use std::collections::{BTreeMap, BTreeSet};

use sico_hir::{Declaration, HirToken, Line, LineKind, lower};
use sico_lexer::TokenKind;
use sico_parser::DeclarationKind;
use sico_semantics::{
    AnalyzeError, ImportedFunction, PackageInterface, SemanticDiagnostic, Type as SemanticType,
    analyze_with_interfaces, exported_functions,
};
use sico_source::{SourceFile, TextRange};

use crate::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, Local, MatchArm, Module,
    Operation, Parameter, Pattern, SourceRange, TaskScope, Terminator, Type, ValueId, VerifyError,
    verify,
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
    /// RFC-0039 §2.4 (STEP-0147): package-import signatures keyed by the
    /// qualified source callee `<package>.<function>`, carrying the full
    /// import name the IR intrinsic records.
    package_functions: BTreeMap<String, PackageSignature>,
}

/// One package import's lowering signature (RFC-0039 §2.4, STEP-0147).
#[derive(Clone)]
struct PackageSignature {
    import_name: String,
    parameters: Vec<Type>,
    result: Type,
}

/// RFC-0039 §2.3/§2.4 (STEP-0147): one resolved package dependency, as
/// supplied by the CLI link pass. `functions` are the exposed interface's
/// signatures in the semantic surface; lowering converts them to IR types
/// (fail-closed on anything outside the frozen v0 conversion).
#[derive(Clone, Copy, Debug)]
pub struct PackageImport<'a> {
    pub name: &'a str,
    pub version: u64,
    pub interface: &'a str,
    pub functions: &'a BTreeMap<String, ImportedFunction>,
}

/// Converts a semantic surface type into the IR import type. The check
/// gate (E8017) already confines versioned interfaces to the v0 value
/// set; this conversion refuses anything else instead of guessing.
fn unsupported_package_type(
    package: &PackageImport<'_>,
    function_name: &str,
    offender: &str,
) -> CoreLowerError {
    unsupported_error(
        format!(
            "package {}@{} function {}: type {offender} is outside the v0 user-WIT value set",
            package.name, package.version, function_name
        ),
        TextRange::up_to(0.into()),
    )
}

fn import_type(ty: &SemanticType) -> Result<Type, String> {
    match ty {
        SemanticType::Named(name) => match name.as_str() {
            "Bool" => Ok(Type::Bool),
            "I64" => Ok(Type::I64),
            "U64" => Ok(Type::U64),
            "Text" => Ok(Type::String),
            "Bytes" => Ok(Type::Bytes),
            "Unit" => Ok(Type::Unit),
            other => Err(other.to_owned()),
        },
        SemanticType::Generic { name, arguments } => match (name.as_str(), arguments.as_slice()) {
            ("List", [inner]) => Ok(Type::List(Box::new(import_type(inner)?))),
            ("Option", [inner]) => Ok(Type::Option(Box::new(import_type(inner)?))),
            ("Result", [ok, error]) => Ok(Type::Result {
                ok: Box::new(import_type(ok)?),
                error: Box::new(import_type(error)?),
            }),
            _ => Err(name.clone()),
        },
        SemanticType::Unknown => Err("<unknown>".to_owned()),
    }
}

struct FunctionBuilder<'a> {
    definitions: &'a Definitions,
    next_value: u32,
    parameter_count: u32,
    bindings: BTreeMap<String, (ValueId, Type)>,
    declared_effects: Vec<String>,
    /// Open task-scope ids, innermost last (RFC-0036 §5.1).
    scope_stack: Vec<u32>,
    /// Owning scope of each spawned `Task` value, so `collect_tasks` can
    /// name the scope its list belongs to.
    spawn_scopes: BTreeMap<ValueId, u32>,
    /// Mutable local cells (M14 STEP-0130): name -> (local id, type).
    cells: BTreeMap<String, (u32, Type)>,
    /// Cell declarations in allocation order (becomes `Function.locals`).
    locals: Vec<Local>,
}

/// Canonical task-scope declaration plan for one function (RFC-0036 §5.1):
/// the table itself plus the lines that open and close each scope region.
#[derive(Default)]
struct TaskScopePlan {
    scopes: Vec<TaskScope>,
    opens: BTreeMap<usize, u32>,
    closes: BTreeMap<usize, u32>,
}

impl TaskScopePlan {
    /// Pairs every `task group` line with its `end task` line; the HIR
    /// annotates each `End` line with the construct it closes.
    fn of(declaration: &Declaration) -> Self {
        let mut plan = Self::default();
        let mut open = Vec::<u32>::new();
        for (index, line) in declaration.lines.iter().enumerate() {
            match line.kind {
                LineKind::TaskGroup => {
                    let scope =
                        u32::try_from(plan.scopes.len()).expect("source limits bound scopes");
                    let parent = open.last().copied();
                    plan.scopes.push(TaskScope { scope, parent });
                    plan.opens.insert(index, scope);
                    open.push(scope);
                }
                LineKind::End
                    if line
                        .tokens
                        .get(1)
                        .is_some_and(|token| token.kind == TokenKind::Task) =>
                {
                    if let Some(scope) = open.pop() {
                        plan.closes.insert(index, scope);
                    }
                }
                _ => {}
            }
        }
        plan
    }
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
    lower_core_modules(source, &[], &[])
}

/// One RFC-0039 module import: its prefix and already-discovered source
/// (STEP-0143). Order must be the CLI link pass's deterministic discovery
/// order; cycle freedom and `use`-site verification happen there.
#[derive(Clone, Copy, Debug)]
pub struct ModuleImport<'a> {
    pub name: &'a str,
    pub source: &'a SourceFile,
}

/// Lowers the entry source plus RFC-0039 module imports into one verified
/// IR `Module` (STEP-0143).
///
/// Every file keeps its single-file lowering path; functions receive
/// globally unique ids from per-file bases (entry first, imports in the
/// given order), so cross-module qualified calls — resolved through the
/// `module.item` aliases in the caller's definition table — need no
/// post-pass rewriting. Imported function display names become
/// `module.name`; entry names stay bare, so single-module programs lower
/// byte-identically to [`lower_core`].
///
/// # Errors
///
/// Same contract as [`lower_core`], applied to every file, plus the
/// independent verifier over the merged module.
///
/// # Panics
///
/// Panics only if import discovery produced more than `u32::MAX` source
/// files or functions, which the CLI link limits make unreachable.
#[allow(clippy::too_many_lines)] // the merged lowering drives the whole set
pub fn lower_core_modules(
    entry: &SourceFile,
    imports: &[ModuleImport<'_>],
    packages: &[PackageImport<'_>],
) -> Result<Module, CoreLowerError> {
    let mut declarations: Vec<(Option<&str>, Vec<Declaration>)> = Vec::new();
    // RFC-0039 §2.3/§2.4 (STEP-0147): the package surface is identical for
    // every file in the set — modules may call package functions too.
    let package_surfaces: Vec<(String, PackageInterface)> = packages
        .iter()
        .map(|package| {
            (
                package.name.to_owned(),
                PackageInterface {
                    package: package.name.to_owned(),
                    version: package.version,
                    interface: package.interface.to_owned(),
                    functions: package.functions.clone(),
                },
            )
        })
        .collect();
    let package_map: BTreeMap<String, PackageInterface> =
        package_surfaces.iter().cloned().collect();
    // RFC-0039 §2.2 (STEP-0144): each file's semantic gate sees the
    // module-import surface (every import's exported functions), so
    // qualified cross-module calls resolve exactly as they did at check
    // time instead of failing the gate as unresolved callees.
    let import_surfaces: Vec<BTreeMap<String, ImportedFunction>> = imports
        .iter()
        .map(|import| exported_functions(import.source))
        .collect();
    for (file_index, (prefix, source)) in std::iter::once((None, entry))
        .chain(
            imports
                .iter()
                .map(|import| (Some(import.name), import.source)),
        )
        .enumerate()
    {
        let mut modules: BTreeMap<String, BTreeMap<String, ImportedFunction>> = BTreeMap::new();
        for (other, import) in imports.iter().enumerate() {
            let surface_index = other + 1;
            if surface_index == file_index {
                continue;
            }
            if let Some(surface) = import_surfaces.get(other) {
                modules.insert(import.name.to_owned(), surface.clone());
            }
        }
        let analysis = analyze_with_interfaces(source, &modules, &package_map)
            .map_err(CoreLowerError::Frontend)?;
        if !analysis.is_success() {
            return Err(CoreLowerError::Semantic(analysis.diagnostics));
        }
        let hir =
            lower(source).map_err(|error| CoreLowerError::Frontend(AnalyzeError::Lower(error)))?;
        declarations.push((prefix, hir.declarations));
    }

    let mut bases = Vec::new();
    let mut next_function = 1_u32;
    for (_, decls) in &declarations {
        bases.push(next_function);
        next_function += u32::try_from(
            decls
                .iter()
                .filter(|declaration| declaration.kind == DeclarationKind::Function)
                .count(),
        )
        .expect("source limits bound functions per module");
    }

    let mut functions = Vec::new();
    let mut task_scopes = BTreeMap::new();
    let mut module_import_signatures = BTreeMap::new();
    for (index, ((prefix, decls), base)) in declarations.iter().zip(&bases).enumerate() {
        let mut definitions = Definitions::from_declarations(decls, *prefix, *base);
        for package in packages {
            for (function_name, imported) in package.functions {
                let parameters = imported
                    .parameters
                    .iter()
                    .map(import_type)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|name| unsupported_package_type(package, function_name, &name))?;
                let result = import_type(&imported.returns)
                    .map_err(|name| unsupported_package_type(package, function_name, &name))?;
                // RFC-0039 §2.4 (STEP-0147): source identifiers map to
                // boundary kebab-case (`parse_line` -> `parse-line`);
                // WIT names are kebab and the mapping is injective.
                let boundary_name = crate::boundary_kebab(function_name);
                definitions.package_functions.insert(
                    format!("{}.{}", package.name, function_name),
                    PackageSignature {
                        import_name: format!(
                            "sico:user/{}@{}.0.0.{}",
                            crate::boundary_kebab(package.interface),
                            package.version,
                            boundary_name
                        ),
                        parameters: parameters.clone(),
                        result: result.clone(),
                    },
                );
                let full = format!(
                    "sico:user/{}@{}.0.0.{}",
                    crate::boundary_kebab(package.interface),
                    package.version,
                    boundary_name
                );
                module_import_signatures
                    .entry(full)
                    .or_insert((parameters.clone(), result.clone()));
            }
        }
        for (other_index, (other_prefix, other_decls)) in declarations.iter().enumerate() {
            if other_index == index {
                continue;
            }
            if let Some(other_prefix) = other_prefix {
                definitions.add_qualified_aliases(other_decls, other_prefix, bases[other_index]);
            }
        }
        for declaration in decls {
            if declaration.kind != DeclarationKind::Function {
                continue;
            }
            let (mut function, scopes) = lower_function(declaration, &definitions)?;
            if index > 0 {
                function.name = format!(
                    "{}.{}",
                    prefix.expect("import files carry a module prefix"),
                    function.name
                );
            }
            if !scopes.is_empty() {
                task_scopes.insert(function.id, scopes);
            }
            functions.push(function);
        }
    }

    let mut module = Module::new(entry.name(), entry.len().into());
    // The merged module's "source" is the file set: verifier range bounds
    // widen to the combined length (identical to the entry length for
    // single-file programs, whose lowering path is unchanged).
    let mut combined_len = u64::from(u32::from(entry.len()));
    for import in imports {
        combined_len += u64::from(u32::from(import.source.len()));
    }
    module.source_len = u32::try_from(combined_len)
        .expect("combined source length stays inside the u32 source bound");
    module.functions = functions;
    module.import_signatures = module_import_signatures;
    if !task_scopes.is_empty() {
        module.task_scopes = Some(task_scopes);
    }
    if module.functions.is_empty() {
        return unsupported(
            "module without core function",
            TextRange::up_to(entry.len()),
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
    fn from_declarations(
        declarations: &[Declaration],
        module_prefix: Option<&str>,
        first_function_id: u32,
    ) -> Self {
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
                // RFC-0039 (STEP-0143): module/use carry no type surface.
                DeclarationKind::Function
                | DeclarationKind::Interface
                | DeclarationKind::Module
                | DeclarationKind::Use => {}
            }
        }
        let mut next_function = first_function_id;
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
                    if let Some((name, mut signature)) =
                        function_signature(&definitions, declaration, FunctionId(next_function))
                    {
                        if let Some(prefix) = module_prefix {
                            let mut qualified = signature.clone();
                            qualified.id = FunctionId(next_function);
                            definitions
                                .functions
                                .insert(format!("{prefix}.{name}"), qualified);
                        }
                        signature.id = FunctionId(next_function);
                        definitions.functions.insert(name, signature);
                        next_function += 1;
                    }
                }
                DeclarationKind::Module | DeclarationKind::Use => {}
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

    /// RFC-0039 (STEP-0143): registers `prefix.name` aliases for another
    /// file's functions so qualified cross-module calls resolve against
    /// that file's own globally-based ids. Only the qualified key is
    /// inserted; bare names stay file-local.
    fn add_qualified_aliases(
        &mut self,
        declarations: &[Declaration],
        prefix: &str,
        first_function_id: u32,
    ) {
        let mut next_function = first_function_id;
        for declaration in declarations {
            if declaration.kind != DeclarationKind::Function {
                continue;
            }
            if let Some((name, mut signature)) =
                function_signature(self, declaration, FunctionId(next_function))
            {
                signature.id = FunctionId(next_function);
                self.functions.insert(format!("{prefix}.{name}"), signature);
                next_function += 1;
            }
        }
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

/// RFC-0039 (STEP-0143): computes a function declaration's resolved
/// signature without inserting it, so `from_declarations` and
/// `add_qualified_aliases` share one code path.
fn function_signature(
    definitions: &Definitions,
    declaration: &Declaration,
    id: FunctionId,
) -> Option<(String, Signature)> {
    if declaration.kind != DeclarationKind::Function {
        return None;
    }
    let header = &declaration.lines[0].tokens;
    let (mut parameters, return_type) = parse_signature(header);
    for (_, ty, _) in &mut parameters {
        *ty = definitions.resolve_type(ty.clone());
    }
    let async_function = header.iter().any(|token| token.kind == TokenKind::Async);
    let return_type = definitions.resolve_type(return_type);
    // RFC-0036 §5.2: an async function's IR signature returns
    // `Future[T]`; the body resolves it by returning `T`.
    let return_type = if async_function {
        Type::Future(Box::new(return_type))
    } else {
        return_type
    };
    Some((
        declaration.name.clone(),
        Signature {
            id,
            parameters,
            return_type,
            async_function,
        },
    ))
}

fn lower_function(
    declaration: &Declaration,
    definitions: &Definitions,
) -> Result<(Function, Vec<TaskScope>), CoreLowerError> {
    let signature = &definitions.functions[&declaration.name];
    // Structured concurrency (RFC-0036): `spawn`/`await`/`collect_tasks`
    // lower to explicit typed operations inside declared task-scope regions;
    // the affine discipline is enforced by semantic analysis and re-proven
    // by the independent IR verifier. The sequential profile (STEP-0104
    // phase E) projects these operations to the exact M9 behavior.
    let body_return_type = match (&signature.return_type, signature.async_function) {
        (Type::Future(inner), true) => inner.as_ref().clone(),
        _ => signature.return_type.clone(),
    };
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
        scope_stack: Vec::new(),
        spawn_scopes: BTreeMap::new(),
        cells: BTreeMap::new(),
        locals: Vec::new(),
    };
    let scope_plan = TaskScopePlan::of(declaration);
    let match_index = declaration
        .lines
        .iter()
        .position(|line| line.kind == LineKind::Match);
    let if_index = declaration
        .lines
        .iter()
        .position(|line| line.kind == LineKind::If);
    let has_general_control = declaration.lines.iter().skip(1).any(|line| {
        matches!(
            line.kind,
            LineKind::While | LineKind::Break | LineKind::Continue | LineKind::Set | LineKind::Else
        )
    });
    let blocks = if let Some(index) = if_index {
        if is_revision_if_shape(declaration, index) {
            // RFC-0044: the frozen revision-guard path declines non-Revision
            // equality (the common `if a == b: return ...` shape); those
            // fall back to the general CFG, which lowers infix equality.
            match lower_revision_if(declaration, index, &mut builder, &body_return_type)? {
                Some(blocks) => blocks,
                None => lower_general(declaration, &mut builder, &body_return_type, &scope_plan)?,
            }
        } else {
            lower_general(declaration, &mut builder, &body_return_type, &scope_plan)?
        }
    } else if let Some(index) = match_index {
        if is_strict_match_shape(declaration, index) {
            lower_match(declaration, index, &mut builder, &body_return_type)?
        } else {
            lower_general(declaration, &mut builder, &body_return_type, &scope_plan)?
        }
    } else if has_general_control {
        lower_general(declaration, &mut builder, &body_return_type, &scope_plan)?
    } else {
        vec![lower_straight_line(
            declaration,
            &mut builder,
            &body_return_type,
            &scope_plan,
        )?]
    };
    Ok((
        Function {
            id: signature.id,
            name: declaration.name.clone(),
            parameters,
            return_type: signature.return_type.clone(),
            effects,
            locals: std::mem::take(&mut builder.locals),
            entry: BlockId(0),
            blocks,
            range: source_range(declaration.range),
        },
        scope_plan.scopes,
    ))
}

/// True when the function matches the frozen revision-guard shape exactly
/// (metadata/expressions prefix, one top-level `==` if, single-return
/// branches), keeping it on the specialized `RevisionCheck` path.
fn is_revision_if_shape(declaration: &Declaration, if_index: usize) -> bool {
    if declaration.lines[1..if_index].iter().any(|line| {
        !matches!(
            line.kind,
            LineKind::Effects | LineKind::Capabilities | LineKind::Expression
        )
    }) {
        return false;
    }
    let if_line = &declaration.lines[if_index];
    let condition_tokens = &if_line.tokens[1..if_line.tokens.len().saturating_sub(1)];
    if top_level_position(condition_tokens, TokenKind::EqualEqual).is_none() {
        return false;
    }
    let Some(then_line) = declaration.lines.get(if_index + 1) else {
        return false;
    };
    if then_line.kind != LineKind::Return {
        return false;
    }
    let Some(end_index) = declaration
        .lines
        .iter()
        .enumerate()
        .skip(if_index + 2)
        .find_map(|(index, line)| (line.kind == LineKind::End).then_some(index))
    else {
        return false;
    };
    declaration
        .lines
        .get(end_index + 1)
        .is_some_and(|line| line.kind == LineKind::Return)
}

/// True when the function matches the frozen all-return match shape
/// (metadata prefix, every arm exactly one `return` line) so lowering stays
/// on the byte-stable specialized path.
fn is_strict_match_shape(declaration: &Declaration, match_index: usize) -> bool {
    if declaration.lines[1..match_index]
        .iter()
        .any(|line| !matches!(line.kind, LineKind::Effects | LineKind::Capabilities))
    {
        return false;
    }
    let depth = declaration.lines[match_index].depth;
    let mut cursor = match_index + 1;
    let mut saw_arm = false;
    while cursor < declaration.lines.len() {
        let line = &declaration.lines[cursor];
        if line.kind == LineKind::End && line.depth == depth {
            return saw_arm;
        }
        if !(line.kind == LineKind::MatchArm && line.depth == depth + 1) {
            return false;
        }
        saw_arm = true;
        match declaration.lines.get(cursor + 1) {
            Some(body) if body.kind == LineKind::Return && body.depth > depth => {
                cursor += 1;
            }
            _ => return false,
        }
        cursor += 1;
    }
    false
}

/// One in-flight CFG block; the id equals the index in the materialized
/// `Vec<Block>`. Sealed exactly once.
struct PendingBlock {
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
    range: TextRange,
    /// Creation index (pre-renumbering), for reference remapping.
    creation: usize,
    /// When the block first became the lowering target; `ValueId`s are
    /// assigned in this order, so materialization must follow it.
    became_current: Option<u32>,
}

#[derive(Clone, Copy)]
struct LoopFrame {
    header: usize,
    after: usize,
}

struct RegionClose {
    else_index: Option<usize>,
    end_index: usize,
}

/// Finds the close of an `if`/`while` region opened at `depth`: the first
/// same-depth `else` (when allowed) and the same-depth `end`.
fn find_region_close(
    lines: &[Line],
    start: usize,
    end: usize,
    depth: u16,
    allow_else: bool,
    error_range: TextRange,
) -> Result<RegionClose, CoreLowerError> {
    let mut else_index = None;
    for (index, line) in lines.iter().enumerate().take(end).skip(start) {
        match line.kind {
            // `else` sits at the body level, one below the `if` itself; a
            // nested if's own `else` is deeper and cannot match here.
            LineKind::Else if allow_else && else_index.is_none() && line.depth == depth + 1 => {
                else_index = Some(index);
            }
            LineKind::End if line.depth == depth => {
                return Ok(RegionClose {
                    else_index,
                    end_index: index,
                });
            }
            _ => {}
        }
    }
    Err(unsupported_error("unterminated region", error_range))
}

/// General structured lowering (M14 STEP-0130): arbitrary nesting of
/// `if`/`else`, `while` loops with `break`/`continue`, fall-through match
/// arms, and `set` reassignment through IR local cells. Bindings introduced
/// inside a region are dropped when the region closes; cells persist.
struct GeneralLowering<'a, 'b> {
    builder: &'a mut FunctionBuilder<'b>,
    blocks: Vec<PendingBlock>,
    current: Option<usize>,
    loop_stack: Vec<LoopFrame>,
    return_type: &'a Type,
    scope_plan: &'a TaskScopePlan,
    metadata: bool,
    using_resource: Option<ValueId>,
    nested: usize,
    next_creation: usize,
    emission_seq: u32,
}

impl GeneralLowering<'_, '_> {
    fn new_block(&mut self, range: TextRange) -> usize {
        let creation = self.next_creation;
        self.next_creation += 1;
        self.blocks.push(PendingBlock {
            instructions: Vec::new(),
            terminator: None,
            range,
            creation,
            became_current: None,
        });
        self.blocks.len() - 1
    }

    fn seal_current(&mut self, terminator: Terminator) {
        if let Some(index) = self.current.take() {
            self.blocks[index].terminator = Some(terminator);
        }
    }

    fn set_current(&mut self, index: usize) {
        let seq = self.emission_seq;
        if self.blocks[index].became_current.is_none() {
            self.blocks[index].became_current = Some(seq);
            self.emission_seq += 1;
        }
        self.current = Some(index);
    }

    fn ensure_current(&mut self, range: TextRange) {
        if self.current.is_none() {
            let id = self.new_block(range);
            self.set_current(id);
        }
    }

    /// Lowers `lines[start..end]` (absolute indices) into the open block.
    #[allow(clippy::too_many_lines)]
    fn run(&mut self, lines: &[Line], start: usize, end: usize) -> Result<(), CoreLowerError> {
        let mut index = start;
        while index < end {
            let line = &lines[index];
            match line.kind {
                LineKind::Let => {
                    self.ensure_current(line.range);
                    let equal = position(&line.tokens, TokenKind::Equal)
                        .ok_or_else(|| unsupported_error("let without value", line.range))?;
                    let name = line.tokens.get(1).map_or("", |token| token.text.as_str());
                    let (value, ty) = self.builder.expression(
                        &line.tokens[equal + 1..],
                        None,
                        &mut self.blocks[self.current.expect("current")].instructions,
                    )?;
                    if self.builder.cells.contains_key(name) {
                        return unsupported("cell redeclared", line.range);
                    }
                    // Inside a general CFG every `let` becomes a mutable
                    // local cell: IR values are block-scoped, so a binding
                    // read in a later block (loop body, arm, join) must live
                    // in a cell, whether or not the source reassigns it.
                    // Straight-line bodies keep plain bindings via
                    // `lower_straight_line`, so frozen shapes are unchanged.
                    if name.is_empty() || !name.starts_with('#') {
                        let local = u32::try_from(self.builder.locals.len())
                            .map_err(|_| unsupported_error("local limit", line.range))?;
                        self.builder.locals.push(Local {
                            name: name.to_owned(),
                            ty: ty.clone(),
                            range: source_range(line.tokens[1].range),
                        });
                        self.builder
                            .cells
                            .insert(name.to_owned(), (local, ty.clone()));
                        self.builder.emit(
                            Type::Unit,
                            Operation::WriteLocal { local, value },
                            line.range,
                            &mut self.blocks[self.current.expect("current")].instructions,
                        );
                    } else {
                        self.builder.bindings.insert(name.to_owned(), (value, ty));
                    }
                }
                LineKind::Set => {
                    self.ensure_current(line.range);
                    let equal = position(&line.tokens, TokenKind::Equal)
                        .ok_or_else(|| unsupported_error("set without value", line.range))?;
                    let name = line.tokens.get(1).map_or("", |token| token.text.as_str());
                    let Some((local, ty)) = self.builder.cells.get(name).cloned() else {
                        return unsupported("set without prior let", line.range);
                    };
                    let (value, value_ty) = self.builder.expression(
                        &line.tokens[equal + 1..],
                        Some(&ty),
                        &mut self.blocks[self.current.expect("current")].instructions,
                    )?;
                    if value_ty != ty {
                        return unsupported("set type mismatch", line.range);
                    }
                    self.builder.emit(
                        Type::Unit,
                        Operation::WriteLocal { local, value },
                        line.range,
                        &mut self.blocks[self.current.expect("current")].instructions,
                    );
                }
                LineKind::Return => {
                    self.ensure_current(line.range);
                    self.metadata = false;
                    let current = self.current.expect("current");
                    let mut instructions = std::mem::take(&mut self.blocks[current].instructions);
                    let terminator = if *self.return_type == Type::Unit
                        && (line.tokens.len() == 1
                            || line.tokens.get(1).is_some_and(|token| token.text == "Unit"))
                    {
                        if let Some(resource) = self.using_resource.take() {
                            self.builder.emit(
                                Type::Unit,
                                Operation::ResourceDrop(resource),
                                line.range,
                                &mut instructions,
                            );
                        }
                        Terminator::Return(None)
                    } else {
                        let (value, _) = self.builder.expression(
                            &line.tokens[1..],
                            Some(self.return_type),
                            &mut instructions,
                        )?;
                        if let Some(resource) = self.using_resource.take() {
                            self.builder.emit(
                                Type::Unit,
                                Operation::ResourceDrop(resource),
                                line.range,
                                &mut instructions,
                            );
                        }
                        Terminator::Return(Some(value))
                    };
                    self.blocks[current].instructions = instructions;
                    self.seal_current(terminator);
                }
                LineKind::Expression => {
                    if self.metadata {
                        index += 1;
                        continue;
                    }
                    self.ensure_current(line.range);
                    let (_, ty) = self.builder.expression(
                        &line.tokens,
                        None,
                        &mut self.blocks[self.current.expect("current")].instructions,
                    )?;
                    if ty != Type::Unit {
                        return unsupported("non-Unit expression statement", line.range);
                    }
                }
                LineKind::Effects | LineKind::Capabilities => {
                    self.metadata = !line
                        .tokens
                        .iter()
                        .any(|token| token.kind == TokenKind::None);
                }
                LineKind::TaskGroup => {
                    self.ensure_current(line.range);
                    let scope = self
                        .scope_plan
                        .opens
                        .get(&index)
                        .copied()
                        .expect("task group lines are planned");
                    self.builder.emit(
                        Type::Unit,
                        Operation::TaskScopeOpen { scope },
                        line.range,
                        &mut self.blocks[self.current.expect("current")].instructions,
                    );
                    self.builder.scope_stack.push(scope);
                }
                LineKind::End => {
                    if let Some(scope) = self.scope_plan.closes.get(&index).copied() {
                        self.ensure_current(line.range);
                        self.builder.emit(
                            Type::Unit,
                            Operation::TaskScopeClose { scope },
                            line.range,
                            &mut self.blocks[self.current.expect("current")].instructions,
                        );
                        let popped = self.builder.scope_stack.pop();
                        debug_assert_eq!(popped, Some(scope));
                    }
                    // Non-task `end` lines (`end function`, leftovers) are
                    // structural markers; the region walkers consume their
                    // own and the rest is ignored, as in the linear paths.
                }
                LineKind::Using => {
                    if self.nested > 0 {
                        return unsupported("using inside control flow", line.range);
                    }
                    self.ensure_current(line.range);
                    let Some(name) = line.tokens.get(1) else {
                        return unsupported("using scope", line.range);
                    };
                    let Some((value, Type::OwnedResource(_))) =
                        self.builder.bindings.get(&name.text)
                    else {
                        return unsupported("using non-resource", line.range);
                    };
                    self.using_resource = Some(*value);
                }
                LineKind::While => {
                    let depth = line.depth;
                    let condition_tokens =
                        strip_outer_parens(&line.tokens[1..line.tokens.len().saturating_sub(1)]);
                    let header = self.new_block(line.range);
                    self.seal_current(Terminator::Jump(BlockId(
                        u32::try_from(header).expect("block bound"),
                    )));
                    self.set_current(header);
                    let (condition, condition_ty) = self.builder.expression(
                        condition_tokens,
                        None,
                        &mut self.blocks[header].instructions,
                    )?;
                    if condition_ty != Type::Bool {
                        return unsupported("non-Bool loop condition", line.range);
                    }
                    let body_entry = self.new_block(line.range);
                    let after = self.new_block(line.range);
                    self.blocks[header].terminator = Some(Terminator::Branch {
                        condition,
                        then_block: BlockId(u32::try_from(body_entry).expect("block bound")),
                        else_block: BlockId(u32::try_from(after).expect("block bound")),
                    });
                    self.set_current(body_entry);
                    self.loop_stack.push(LoopFrame { header, after });
                    self.nested += 1;
                    let close = find_region_close(lines, index + 1, end, depth, false, line.range)?;
                    let snapshot = self.builder.bindings.clone();
                    self.run(lines, index + 1, close.end_index)?;
                    self.builder.bindings = snapshot;
                    self.nested -= 1;
                    self.loop_stack.pop();
                    self.seal_current(Terminator::Jump(BlockId(
                        u32::try_from(header).expect("block bound"),
                    )));
                    self.set_current(after);
                    index = close.end_index;
                }
                LineKind::If => {
                    let depth = line.depth;
                    let condition_tokens =
                        strip_outer_parens(&line.tokens[1..line.tokens.len().saturating_sub(1)]);
                    if let Some(position) =
                        top_level_position(condition_tokens, TokenKind::EqualEqual)
                    {
                        // RFC-0044 (language v1 batch 1): infix equality on
                        // two same-width fixed operands lowers to the
                        // EqualFixed comparison — the exact shape the
                        // typed `.equal` dispatch emits.
                        let left_tokens = &condition_tokens[..position];
                        let right_tokens = &condition_tokens[position + 1..];
                        self.ensure_current(line.range);
                        let current = self.current.expect("current");
                        let (left_value, left_ty) = self.builder.expression(
                            left_tokens,
                            None,
                            &mut self.blocks[current].instructions,
                        )?;
                        let (right_value, right_ty) = self.builder.expression(
                            right_tokens,
                            None,
                            &mut self.blocks[current].instructions,
                        )?;
                        if left_ty != right_ty || !matches!(left_ty, Type::I64 | Type::U64) {
                            return unsupported(
                                "infix equality condition operand types",
                                line.range,
                            );
                        }
                        let condition = self.builder.emit(
                            Type::Bool,
                            Operation::EqualFixed {
                                left: left_value,
                                right: right_value,
                            },
                            line.range,
                            &mut self.blocks[current].instructions,
                        );
                        let then_block = self.new_block(line.range);
                        let else_block = self.new_block(line.range);
                        let join = self.new_block(line.range);
                        self.seal_current(Terminator::Branch {
                            condition,
                            then_block: BlockId(u32::try_from(then_block).expect("block bound")),
                            else_block: BlockId(u32::try_from(else_block).expect("block bound")),
                        });
                        let close =
                            find_region_close(lines, index + 1, end, depth, true, line.range)?;
                        self.nested += 1;
                        let snapshot = self.builder.bindings.clone();
                        self.set_current(then_block);
                        self.run(
                            lines,
                            index + 1,
                            close.else_index.unwrap_or(close.end_index),
                        )?;
                        self.seal_current(Terminator::Jump(BlockId(
                            u32::try_from(join).expect("block bound"),
                        )));
                        if let Some(else_index) = close.else_index {
                            let arm_snapshot = self.builder.bindings.clone();
                            self.set_current(else_block);
                            self.run(lines, else_index + 1, close.end_index)?;
                            self.builder.bindings = arm_snapshot;
                            self.seal_current(Terminator::Jump(BlockId(
                                u32::try_from(join).expect("block bound"),
                            )));
                        } else {
                            self.blocks[else_block].terminator = Some(Terminator::Jump(BlockId(
                                u32::try_from(join).expect("block bound"),
                            )));
                        }
                        self.builder.bindings = snapshot;
                        self.nested -= 1;
                        self.set_current(join);
                        index = close.end_index;
                        continue;
                    }
                    self.ensure_current(line.range);
                    let (condition, condition_ty) = self.builder.expression(
                        condition_tokens,
                        None,
                        &mut self.blocks[self.current.expect("current")].instructions,
                    )?;
                    if condition_ty != Type::Bool {
                        return unsupported("non-Bool if condition", line.range);
                    }
                    let then_block = self.new_block(line.range);
                    let else_block = self.new_block(line.range);
                    let join = self.new_block(line.range);
                    self.seal_current(Terminator::Branch {
                        condition,
                        then_block: BlockId(u32::try_from(then_block).expect("block bound")),
                        else_block: BlockId(u32::try_from(else_block).expect("block bound")),
                    });
                    let close = find_region_close(lines, index + 1, end, depth, true, line.range)?;
                    self.nested += 1;
                    let snapshot = self.builder.bindings.clone();
                    self.set_current(then_block);
                    self.run(
                        lines,
                        index + 1,
                        close.else_index.unwrap_or(close.end_index),
                    )?;
                    self.seal_current(Terminator::Jump(BlockId(
                        u32::try_from(join).expect("block bound"),
                    )));
                    if let Some(else_index) = close.else_index {
                        let arm_snapshot = self.builder.bindings.clone();
                        self.set_current(else_block);
                        self.run(lines, else_index + 1, close.end_index)?;
                        self.builder.bindings = arm_snapshot;
                        self.seal_current(Terminator::Jump(BlockId(
                            u32::try_from(join).expect("block bound"),
                        )));
                    } else {
                        self.blocks[else_block].terminator = Some(Terminator::Jump(BlockId(
                            u32::try_from(join).expect("block bound"),
                        )));
                    }
                    self.builder.bindings = snapshot;
                    self.nested -= 1;
                    self.set_current(join);
                    index = close.end_index;
                }
                LineKind::Match => {
                    let depth = line.depth;
                    let subject_tokens =
                        strip_outer_parens(&line.tokens[1..line.tokens.len().saturating_sub(1)]);
                    self.ensure_current(line.range);
                    let expression_parts = split_top_level(subject_tokens, TokenKind::Comma);
                    let mut values = Vec::new();
                    let mut value_types = Vec::new();
                    for expression in expression_parts {
                        let (value, ty) = self.builder.expression(
                            expression,
                            None,
                            &mut self.blocks[self.current.expect("current")].instructions,
                        )?;
                        values.push(value);
                        value_types.push(ty);
                    }
                    // Scan arm regions first: (arm line index, body end).
                    // Arms sit one level below the match (indentation
                    // blocks); the `end match` closes at the match depth.
                    let mut arm_ranges = Vec::new();
                    let mut cursor = index + 1;
                    while cursor < end {
                        let arm_line = &lines[cursor];
                        if arm_line.depth == depth && arm_line.kind == LineKind::End {
                            break;
                        }
                        if !(arm_line.depth == depth + 1 && arm_line.kind == LineKind::MatchArm) {
                            return unsupported("match arm body", arm_line.range);
                        }
                        let mut body_end = cursor + 1;
                        while body_end < end {
                            let body_line = &lines[body_end];
                            if body_line.depth == depth && body_line.kind == LineKind::End {
                                break;
                            }
                            if body_line.depth == depth + 1 && body_line.kind == LineKind::MatchArm
                            {
                                break;
                            }
                            if body_line.depth <= depth {
                                return unsupported("match arm body", body_line.range);
                            }
                            body_end += 1;
                        }
                        arm_ranges.push((cursor, body_end));
                        cursor = body_end;
                    }
                    if arm_ranges.is_empty() {
                        return unsupported("empty match", line.range);
                    }
                    let match_entry = self.current.expect("current");
                    // Payload-binding arms read the subject inside their own
                    // block; IR values are block-scoped, so a non-parameter
                    // subject spills into a compiler-generated cell first.
                    // STEP-0137: the spill triggers on the payload binding
                    // itself, not on ok/error tokens in arm bodies — the
                    // token heuristic refused payload bindings in
                    // plain-value matches (e.g. recursive functions
                    // returning scalars). Programs that compiled before
                    // either took the parameter path or already spilled, so
                    // no previously accepted shape changes.
                    let binds_payload = arm_ranges.iter().any(|(arm_index, _)| {
                        let arm_line = &lines[*arm_index];
                        let pattern_tokens =
                            strip_outer_parens(&arm_line.tokens[1..arm_line.tokens.len() - 1]);
                        let patterns: Vec<_> = split_top_level(pattern_tokens, TokenKind::Comma)
                            .into_iter()
                            .map(parse_pattern)
                            .collect();
                        patterns.len() == 1
                            && matches!(
                                &patterns[0],
                                Pattern::Variant { name, payload }
                                    if matches!(name.as_str(), "ok" | "error")
                                        && matches!(payload.as_slice(), [Pattern::Binding(_)])
                            )
                    });
                    let needs_spill = binds_payload
                        && values
                            .first()
                            .is_some_and(|value| value.0 >= self.builder.parameter_count);
                    let spill_cell = if needs_spill {
                        let Some(subject_type) = value_types.first().cloned() else {
                            return unsupported("match payload binding", lines[index].range);
                        };
                        let name = format!("#match{}", self.builder.locals.len());
                        let local = u32::try_from(self.builder.locals.len())
                            .map_err(|_| unsupported_error("local limit", lines[index].range))?;
                        self.builder.locals.push(Local {
                            name,
                            ty: subject_type.clone(),
                            range: source_range(lines[index].range),
                        });
                        let value = values.first().copied().expect("match subject");
                        self.builder.emit(
                            Type::Unit,
                            Operation::WriteLocal { local, value },
                            lines[index].range,
                            &mut self.blocks[match_entry].instructions,
                        );
                        Some(local)
                    } else {
                        None
                    };
                    let join = self.new_block(lines[index].range);
                    let mut arms = Vec::new();
                    for (arm_index, body_end) in arm_ranges {
                        let arm_line = &lines[arm_index];
                        let pattern_tokens =
                            strip_outer_parens(&arm_line.tokens[1..arm_line.tokens.len() - 1]);
                        let patterns: Vec<_> = split_top_level(pattern_tokens, TokenKind::Comma)
                            .into_iter()
                            .map(parse_pattern)
                            .collect();
                        let mut payload_binding = None;
                        let mut payload_variant = String::new();
                        for pattern in &patterns {
                            let single_result_binding = match pattern {
                                Pattern::Variant { name, payload }
                                    if matches!(name.as_str(), "ok" | "error") =>
                                {
                                    payload_variant.clone_from(name);
                                    match payload.as_slice() {
                                        [Pattern::Binding(name)] => Some(name.clone()),
                                        _ => None,
                                    }
                                }
                                _ => None,
                            };
                            match single_result_binding {
                                Some(binding)
                                    if patterns.len() == 1 && payload_binding.is_none() =>
                                {
                                    payload_binding = Some(binding);
                                }
                                _ if pattern_binds(pattern) => {
                                    return unsupported("match payload binding", arm_line.range);
                                }
                                _ => {}
                            }
                        }
                        let arm_block = self.new_block(arm_line.range);
                        arms.push(MatchArm {
                            patterns,
                            target: BlockId(u32::try_from(arm_block).expect("block bound")),
                            range: source_range(arm_line.range),
                        });
                        self.set_current(arm_block);
                        let arm_snapshot = self.builder.bindings.clone();
                        if let Some(binding) = payload_binding {
                            let Some(Type::Result { ok, error }) = value_types.first() else {
                                return unsupported(
                                    "match payload binding on non-Result",
                                    arm_line.range,
                                );
                            };
                            let base = if let Some(local) = spill_cell {
                                let Some(subject_type) = value_types.first().cloned() else {
                                    return unsupported("match payload binding", arm_line.range);
                                };
                                self.builder.emit(
                                    subject_type,
                                    Operation::ReadLocal { local },
                                    arm_line.range,
                                    &mut self.blocks[arm_block].instructions,
                                )
                            } else {
                                let Some(base) = values.first().copied() else {
                                    return unsupported("match payload binding", arm_line.range);
                                };
                                if base.0 >= self.builder.parameter_count {
                                    return unsupported("match payload binding", arm_line.range);
                                }
                                base
                            };
                            let payload_type = if payload_variant == "ok" {
                                (**ok).clone()
                            } else {
                                (**error).clone()
                            };
                            let payload = self.builder.emit(
                                payload_type.clone(),
                                Operation::Project {
                                    base,
                                    field: payload_variant,
                                },
                                arm_line.range,
                                &mut self.blocks[arm_block].instructions,
                            );
                            // A payload binding may be read after the match
                            // joins (IR values are block-scoped), so it spills
                            // into a compiler-generated cell; the binding then
                            // resolves through `ReadLocal` in every block.
                            let cell_name = format!("#match{binding}");
                            let local = u32::try_from(self.builder.locals.len())
                                .map_err(|_| unsupported_error("local limit", arm_line.range))?;
                            self.builder.locals.push(Local {
                                name: cell_name,
                                ty: payload_type.clone(),
                                range: source_range(arm_line.range),
                            });
                            self.builder
                                .cells
                                .insert(binding, (local, payload_type.clone()));
                            self.builder.emit(
                                Type::Unit,
                                Operation::WriteLocal {
                                    local,
                                    value: payload,
                                },
                                arm_line.range,
                                &mut self.blocks[arm_block].instructions,
                            );
                        }
                        self.nested += 1;
                        self.run(lines, arm_index + 1, body_end)?;
                        self.nested -= 1;
                        self.builder.bindings = arm_snapshot;
                        self.seal_current(Terminator::Jump(BlockId(
                            u32::try_from(join).expect("block bound"),
                        )));
                    }
                    self.blocks[match_entry].terminator = Some(Terminator::Match { values, arms });
                    self.set_current(join);
                    index = cursor;
                }
                LineKind::Break => {
                    let Some(frame) = self.loop_stack.last().copied() else {
                        return unsupported("break outside loop", line.range);
                    };
                    self.ensure_current(line.range);
                    self.seal_current(Terminator::Jump(BlockId(
                        u32::try_from(frame.after).expect("block bound"),
                    )));
                }
                LineKind::Continue => {
                    let Some(frame) = self.loop_stack.last().copied() else {
                        return unsupported("continue outside loop", line.range);
                    };
                    self.ensure_current(line.range);
                    self.seal_current(Terminator::Jump(BlockId(
                        u32::try_from(frame.header).expect("block bound"),
                    )));
                }
                LineKind::DeclarationHeader | LineKind::FunctionSignature => {}
                LineKind::Field | LineKind::Invariant | LineKind::Variant => {
                    return unsupported("non-function line", line.range);
                }
                LineKind::MatchArm => return unsupported("match arm body", line.range),
                LineKind::Else => return unsupported("else outside if", line.range),
            }
            index += 1;
        }
        Ok(())
    }
}

/// Lowers one function body through the general structured CFG path.
fn lower_general(
    declaration: &Declaration,
    builder: &mut FunctionBuilder<'_>,
    return_type: &Type,
    scope_plan: &TaskScopePlan,
) -> Result<Vec<Block>, CoreLowerError> {
    let mut lowering = GeneralLowering {
        builder,
        blocks: Vec::new(),
        current: None,
        loop_stack: Vec::new(),
        return_type,
        scope_plan,
        metadata: false,
        using_resource: None,
        nested: 0,
        next_creation: 0,
        emission_seq: 0,
    };
    let entry = lowering.new_block(declaration.range);
    lowering.set_current(entry);
    lowering.run(&declaration.lines, 1, declaration.lines.len())?;
    lowering.seal_current(Terminator::Unreachable);
    // Materialize blocks in first-emission order so instruction results stay
    // sequential per block (the verifier's canonical-id rule), then remap
    // every terminator reference from creation ids to final ids.
    // Stamped (emitted-into) blocks first in emission order; never-emitted
    // blocks (empty joins/else arms) trail in creation order. The entry —
    // stamped first — therefore stays at index 0.
    lowering.blocks.sort_by_key(|block| {
        (
            block.became_current.is_none(),
            block.became_current.unwrap_or(0),
            block.creation,
        )
    });
    let mut id_map = BTreeMap::<usize, usize>::new();
    for (index, block) in lowering.blocks.iter().enumerate() {
        id_map.insert(block.creation, index);
    }
    let remap = |id: BlockId| -> BlockId {
        let creation = id.0 as usize;
        let target = id_map.get(&creation).copied().unwrap_or(creation);
        BlockId(u32::try_from(target).expect("block bound"))
    };
    let blocks = lowering
        .blocks
        .iter()
        .enumerate()
        .map(|(index, pending)| {
            let terminator = pending
                .terminator
                .as_ref()
                .map(|terminator| match terminator {
                    Terminator::Jump(target) => Terminator::Jump(remap(*target)),
                    Terminator::Branch {
                        condition,
                        then_block,
                        else_block,
                    } => Terminator::Branch {
                        condition: *condition,
                        then_block: remap(*then_block),
                        else_block: remap(*else_block),
                    },
                    Terminator::Match { values, arms } => Terminator::Match {
                        values: values.clone(),
                        arms: arms
                            .iter()
                            .map(|arm| MatchArm {
                                patterns: arm.patterns.clone(),
                                target: remap(arm.target),
                                range: arm.range,
                            })
                            .collect(),
                    },
                    other => other.clone(),
                });
            Block {
                id: BlockId(u32::try_from(index).expect("block bound")),
                instructions: pending.instructions.clone(),
                terminator: terminator.unwrap_or(Terminator::Unreachable),
                range: source_range(pending.range),
            }
        })
        .collect();
    let entry_id = remap(BlockId(u32::try_from(entry).expect("block bound")));
    debug_assert_eq!(entry_id, BlockId(0));
    Ok(blocks)
}

#[allow(clippy::too_many_lines)]
fn lower_straight_line(
    declaration: &Declaration,
    builder: &mut FunctionBuilder<'_>,
    return_type: &Type,
    scope_plan: &TaskScopePlan,
) -> Result<Block, CoreLowerError> {
    let mut instructions = Vec::new();
    let mut terminator = None;
    let mut using_resource: Option<ValueId> = None;
    let mut metadata_lines = false;
    for (line_index, line) in declaration.lines.iter().enumerate().skip(1) {
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
            // A task group opens an explicit scope region (RFC-0036 §5.2);
            // the region closes at its paired `end task` line, which the HIR
            // annotates with the `Task` token.
            LineKind::TaskGroup => {
                let scope = scope_plan
                    .opens
                    .get(&line_index)
                    .copied()
                    .expect("task group lines are planned");
                builder.emit(
                    Type::Unit,
                    Operation::TaskScopeOpen { scope },
                    line.range,
                    &mut instructions,
                );
                builder.scope_stack.push(scope);
            }
            LineKind::End => {
                if let Some(scope) = scope_plan.closes.get(&line_index).copied() {
                    builder.emit(
                        Type::Unit,
                        Operation::TaskScopeClose { scope },
                        line.range,
                        &mut instructions,
                    );
                    let popped = builder.scope_stack.pop();
                    debug_assert_eq!(popped, Some(scope));
                }
            }
            LineKind::DeclarationHeader | LineKind::FunctionSignature => {}
            LineKind::If
            | LineKind::Else
            | LineKind::While
            | LineKind::Break
            | LineKind::Continue
            | LineKind::Set => return unsupported("nested if control flow", line.range),
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

    // STEP-0137: a computed match subject with payload-binding arms spills
    // into a cell; the WriteLocal must be emitted into the entry block
    // BEFORE any arm instructions allocate value ids (the verifier requires
    // canonical id order across the whole function).
    let mut spill = None;
    {
        let binds_payload = declaration.lines[match_index + 1..]
            .iter()
            .take_while(|line| line.kind != LineKind::End)
            .any(|line| {
                if line.kind != LineKind::MatchArm {
                    return false;
                }
                let pattern_tokens = strip_outer_parens(&line.tokens[1..line.tokens.len() - 1]);
                let patterns: Vec<_> = split_top_level(pattern_tokens, TokenKind::Comma)
                    .into_iter()
                    .map(parse_pattern)
                    .collect();
                patterns.len() == 1
                    && matches!(
                        &patterns[0],
                        Pattern::Variant { name, payload }
                            if matches!(name.as_str(), "ok" | "error")
                                && matches!(payload.as_slice(), [Pattern::Binding(_)])
                    )
            });
        let subject_computed = values
            .first()
            .is_some_and(|value| value.0 >= builder.parameter_count);
        if binds_payload && subject_computed {
            let Some(subject_type) = value_types.first().cloned() else {
                return unsupported(
                    "match payload binding",
                    declaration.lines[match_index].range,
                );
            };
            let name = format!("#match{}", builder.locals.len());
            let local = u32::try_from(builder.locals.len()).map_err(|_| {
                unsupported_error("local limit", declaration.lines[match_index].range)
            })?;
            builder.locals.push(Local {
                name,
                ty: subject_type,
                range: source_range(declaration.lines[match_index].range),
            });
            let subject = values.first().copied().expect("match subject");
            builder.emit(
                Type::Unit,
                Operation::WriteLocal {
                    local,
                    value: subject,
                },
                declaration.lines[match_index].range,
                &mut entry_instructions,
            );
            spill = Some(local);
        }
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
            // IR values are block-scoped: a match subject that is a function
            // parameter stays visible inside the arm block; a computed
            // subject reads the entry-block spill cell (allocated before the
            // arm loop so value ids stay canonical; STEP-0137).
            let base = if base.0 >= builder.parameter_count {
                let Some(local) = spill else {
                    return unsupported("match payload binding", line.range);
                };
                builder.emit(
                    value_types.first().expect("match subject").clone(),
                    Operation::ReadLocal { local },
                    line.range,
                    &mut instructions,
                )
            } else {
                *base
            };
            let payload_type = match variant.as_str() {
                "ok" => ok.as_ref().clone(),
                _ => error.as_ref().clone(),
            };
            let value = builder.emit(
                payload_type.clone(),
                Operation::Project {
                    base,
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
) -> Result<Option<Vec<Block>>, CoreLowerError> {
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
        // Not an equality guard at all: fall back to the general CFG.
        return Ok(None);
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
        // RFC-0044: non-Revision equality (e.g. bare-integer comparisons)
        // routes to the general CFG, which lowers EqualFixed directly.
        return Ok(None);
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
    Ok(Some(vec![
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
    ]))
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
            // RFC-0036 §5.2: spawn is an eager structured creation inside
            // the innermost open task scope, producing a `Task[T]` handle.
            return self.spawn(&tokens[1..], token_range(tokens), output);
        }
        if tokens
            .first()
            .is_some_and(|token| token.kind == TokenKind::Await)
        {
            // RFC-0036 §5.2: await consumes a `Task[T]`/`Future[T]` handle
            // and yields `T`; awaiting anything else (the `collect_tasks`
            // result surface) stays the sequential identity.
            let (value, ty) = self.expression(&tokens[1..], expected, output)?;
            return match ty {
                Type::Task(inner) | Type::Future(inner) => {
                    let awaited = self.emit(
                        inner.as_ref().clone(),
                        Operation::Await(value),
                        token_range(tokens),
                        output,
                    );
                    Ok((awaited, inner.as_ref().clone()))
                }
                _ => Ok((value, ty)),
            };
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
                TokenKind::Identifier => self.resolve_name(&token.text, token.range, output),
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
            if self.cells.contains_key(&tokens[0].text) {
                let (base, base_type) =
                    self.resolve_name(&tokens[0].text, tokens[0].range, output)?;
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
    /// Lowers `spawn f(args)`: `f` must be a known async function (so its IR
    /// signature returns `Future[T]`), arguments evaluate in RFC-0009 order,
    /// and the resulting `Task[T]` handle belongs to the innermost open
    /// task scope (RFC-0036 §3).
    fn spawn(
        &mut self,
        tokens: &[HirToken],
        range: TextRange,
        output: &mut Vec<Instruction>,
    ) -> Result<(ValueId, Type), CoreLowerError> {
        let Some(open) = top_level_call_open(tokens) else {
            return unsupported("spawn target", range);
        };
        let callee = join_path(&tokens[..open]);
        let Some(signature) = self.definitions.functions.get(&callee) else {
            return unsupported(format!("spawn target {callee}"), range);
        };
        if !signature.async_function {
            return unsupported(format!("spawn of non-async function {callee}"), range);
        }
        let Type::Future(inner) = &signature.return_type else {
            return unsupported(format!("spawn target {callee}"), range);
        };
        let arguments = split_top_level(&tokens[open + 1..tokens.len() - 1], TokenKind::Comma);
        if arguments.len() != signature.parameters.len() {
            return unsupported("spawn arity", range);
        }
        let mut values = Vec::new();
        for (argument, (_, ty, _)) in arguments.iter().zip(&signature.parameters) {
            values.push(
                self.expression(argument_value(argument), Some(ty), output)?
                    .0,
            );
        }
        let Some(&scope) = self.scope_stack.last() else {
            // Semantic analysis rejects detached spawns with E5104 first.
            return unsupported("detached spawn", range);
        };
        let ty = Type::Task(inner.clone());
        let value = self.emit(
            ty.clone(),
            Operation::Spawn {
                scope,
                callee: signature.id,
                arguments: values,
            },
            range,
            output,
        );
        self.spawn_scopes.insert(value, scope);
        Ok((value, ty))
    }

    /// Lowers `collect_tasks([handles], order: input)`: the list literal
    /// becomes a positional `Construct` of `Task[T]` handles and the collect
    /// consumes it in creation order, yielding `List[T]` (RFC-0036 §4).
    fn collect_tasks(
        &mut self,
        arguments: &[&[HirToken]],
        expected: Option<&Type>,
        range: TextRange,
        output: &mut Vec<Instruction>,
    ) -> Result<(ValueId, Type), CoreLowerError> {
        let [tasks_argument, order_argument] = arguments else {
            return unsupported("collect_tasks arity", range);
        };
        let order_ok = named_argument(order_argument) == Some("order")
            && matches!(
                argument_value(order_argument),
                [token] if token.kind == TokenKind::Identifier && token.text == "input"
            );
        if !order_ok {
            return unsupported("collect_tasks order", range);
        }
        let list = strip_outer_parens(tasks_argument);
        if list.len() < 2
            || list[0].kind != TokenKind::LeftBracket
            || matching_close(list, 0) != Some(list.len() - 1)
        {
            return unsupported("collect_tasks list literal", range);
        }
        let elements = split_top_level(&list[1..list.len() - 1], TokenKind::Comma);
        let mut inner: Option<Type> = None;
        let mut fields = Vec::with_capacity(elements.len());
        let mut owner: Option<u32> = None;
        for (index, element) in elements.iter().enumerate() {
            let (value, ty) = self.expression(element, None, output)?;
            let Type::Task(element_inner) = ty else {
                return unsupported("collect_tasks element", range);
            };
            match &inner {
                None => inner = Some(element_inner.as_ref().clone()),
                Some(existing) if *existing == *element_inner => {}
                Some(_) => return unsupported("mixed collect_tasks elements", range),
            }
            match (owner, self.spawn_scopes.get(&value).copied()) {
                (None, Some(scope)) => owner = Some(scope),
                (Some(existing), Some(scope)) if existing == scope => {}
                // Handles from different scopes never mix (RFC-0036 §5.3);
                // an untracked handle means it crossed a scope boundary.
                _ => return unsupported("collect_tasks scope mismatch", range),
            }
            fields.push(ConstructField {
                name: index.to_string(),
                value,
            });
        }
        let inner = match (inner, expected) {
            (Some(inner), _) => inner,
            // An empty list takes its element type from the expected
            // `List[T]` result type (RFC-0036 §4: empty collect is legal).
            (None, Some(Type::List(element))) => element.as_ref().clone(),
            (None, _) => return unsupported("unconstrained empty collect_tasks", range),
        };
        let scope = match owner {
            Some(scope) => scope,
            None => match self.scope_stack.last() {
                Some(&scope) => scope,
                None => return unsupported("detached collect_tasks", range),
            },
        };
        let tasks = self.emit(
            Type::List(Box::new(Type::Task(Box::new(inner.clone())))),
            Operation::Construct {
                name: String::new(),
                fields,
            },
            range,
            output,
        );
        let ty = Type::List(Box::new(inner));
        let value = self.emit(
            ty.clone(),
            Operation::TaskCollect { scope, tasks },
            range,
            output,
        );
        Ok((value, ty))
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
        if callee == "collect_tasks" {
            return self.collect_tasks(&arguments, expected, token_range(tokens), output);
        }
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
                "checked_add"
                    | "checked_sub"
                    | "checked_mul"
                    | "checked_div"
                    | "equal"
                    | "less_than"
                    | "bit_and"
                    | "bit_or"
                    | "bit_xor"
                    | "shl"
                    | "shr"
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
                "checked_mul" => (
                    checked_fixed_result(fixed.clone()),
                    Operation::CheckedMul { left, right },
                ),
                "checked_div" => (
                    checked_fixed_result(fixed.clone()),
                    Operation::CheckedDiv { left, right },
                ),
                "equal" => (Type::Bool, Operation::EqualFixed { left, right }),
                "less_than" => (Type::Bool, Operation::LessFixed { left, right }),
                // Bit operations are infallible and stay in the operand type
                // (STEP-0132). Shift amounts share the operand type; wasm
                // masks counts to [0, 63].
                "bit_and" => (fixed.clone(), Operation::BitAnd { left, right }),
                "bit_or" => (fixed.clone(), Operation::BitOr { left, right }),
                "bit_xor" => (fixed.clone(), Operation::BitXor { left, right }),
                "shl" => (
                    fixed.clone(),
                    Operation::Shl {
                        value: left,
                        amount: right,
                    },
                ),
                "shr" => (
                    fixed.clone(),
                    Operation::Shr {
                        value: left,
                        amount: right,
                    },
                ),
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
        if let Some(package) = self.definitions.package_functions.get(&callee) {
            if package.parameters.len() != arguments.len() {
                return unsupported("package call arity", token_range(tokens));
            }
            let mut values = Vec::with_capacity(arguments.len());
            for (argument, parameter) in arguments.iter().zip(&package.parameters) {
                values.push(
                    self.expression(argument_value(argument), Some(parameter), output)?
                        .0,
                );
            }
            let ty = package.result.clone();
            let value = self.emit(
                ty.clone(),
                Operation::Intrinsic {
                    name: package.import_name.clone(),
                    arguments: values,
                },
                token_range(tokens),
                output,
            );
            return Ok((value, ty));
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
            let receiver = if let Some((index, ty)) = self.cells.get(receiver_name).cloned() {
                Some((
                    self.emit(
                        ty.clone(),
                        Operation::ReadLocal { local: index },
                        tokens[0].range,
                        output,
                    ),
                    ty,
                ))
            } else {
                self.bindings.get(receiver_name).cloned()
            };
            if let Some((receiver, receiver_type)) = receiver {
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

    /// Resolves one name: a mutable cell reads through `ReadLocal`, any
    /// other binding is its SSA value.
    fn resolve_name(
        &mut self,
        name: &str,
        range: TextRange,
        output: &mut Vec<Instruction>,
    ) -> Result<(ValueId, Type), CoreLowerError> {
        if let Some((index, ty)) = self.cells.get(name).cloned() {
            let value = self.emit(
                ty.clone(),
                Operation::ReadLocal { local: index },
                range,
                output,
            );
            return Ok((value, ty));
        }
        self.bindings
            .get(name)
            .cloned()
            .ok_or_else(|| unsupported_error(format!("unresolved value {name}"), range))
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
        "Map" if arguments.len() == 2 => Type::Map {
            key: Box::new(arguments[0].clone()),
            value: Box::new(arguments[1].clone()),
        },
        "Set" if arguments.len() == 1 => Type::Set(Box::new(arguments[0].clone())),
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
    let body = text
        .strip_prefix('\"')
        .and_then(|text| text.strip_suffix('\"'))
        .unwrap_or(text);
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        #[allow(
            clippy::match_same_arms,
            reason = "an escaped backslash and a trailing lone backslash both keep one backslash"
        )]
        match chars.next() {
            Some('\"') => out.push('\"'),
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
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
