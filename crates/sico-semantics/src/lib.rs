//! Static semantic analysis over deterministic Sico HIR.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, fmt};

use sico_hir::{HirId, HirToken, Line, LineKind, LowerError, Module, lower};
use sico_lexer::TokenKind;
use sico_parser::DeclarationKind;
use sico_source::{SourceFile, TextRange};

pub const MAX_SEMANTIC_DIAGNOSTICS: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Analysis {
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub facts: Vec<SemanticFact>,
}

impl Analysis {
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalyzeError {
    Lower(LowerError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDiagnostic {
    pub code: &'static str,
    pub key: &'static str,
    pub message: String,
    pub arguments: BTreeMap<String, String>,
    pub range: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FactId {
    pub node: HirId,
    pub slot: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticFactKind {
    Type,
    Function,
    Parameter,
    Field,
    Local,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticFact {
    pub id: FactId,
    pub kind: SemanticFactKind,
    pub name: String,
    pub ty: Option<Type>,
    pub range: TextRange,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Type {
    Named(String),
    Generic { name: String, arguments: Vec<Type> },
    Unknown,
}

impl Type {
    fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => formatter.write_str(name),
            Self::Generic { name, arguments } => write!(
                formatter,
                "{}[{}]",
                name,
                arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Unknown => formatter.write_str("<unknown>"),
        }
    }
}

#[derive(Clone, Debug)]
enum TypeDefinition {
    Newtype {
        base: Type,
    },
    Record {
        fields: BTreeMap<String, FieldDefinition>,
        invariant: Option<Invariant>,
    },
}

#[derive(Clone, Debug)]
struct FieldDefinition {
    ty: Type,
    range: TextRange,
}

#[derive(Clone, Debug)]
struct Invariant {
    left: String,
    right: String,
    expression: String,
}

#[derive(Clone, Debug)]
struct FunctionDefinition {
    name: String,
    parameters: Vec<Parameter>,
    returns: Type,
    body: Vec<Line>,
    range: TextRange,
}

#[derive(Clone, Debug)]
struct Parameter {
    name: String,
    ty: Type,
    range: TextRange,
}

#[derive(Clone, Debug, Default)]
struct Model {
    types: BTreeMap<String, TypeDefinition>,
    functions: BTreeMap<String, FunctionDefinition>,
}

#[derive(Clone, Debug)]
struct Value {
    ty: Type,
    range: TextRange,
    integer: Option<i128>,
}

#[derive(Clone, Debug)]
struct Argument<'a> {
    name: Option<String>,
    tokens: &'a [HirToken],
}

/// Performs the implemented M2 checks over a successful HIR module.
///
/// # Errors
///
/// Returns [`AnalyzeError`] when lexical/parser errors prevent HIR lowering.
pub fn analyze(source: &SourceFile) -> Result<Analysis, AnalyzeError> {
    let module = lower(source).map_err(AnalyzeError::Lower)?;
    let (model, mut facts) = build_model(&module);
    let mut diagnostics = Vec::new();
    for function in model.functions.values() {
        analyze_function(function, &model, &mut diagnostics, &mut facts);
    }
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range.start(), diagnostic.code));
    facts.sort_by_key(|fact| fact.id);
    Ok(Analysis { diagnostics, facts })
}

fn build_model(module: &Module) -> (Model, Vec<SemanticFact>) {
    let mut model = Model::default();
    let mut facts = Vec::new();
    for declaration in &module.declarations {
        match declaration.kind {
            DeclarationKind::Newtype => {
                let tokens = &declaration.lines[0].tokens;
                let base = tokens
                    .iter()
                    .position(|token| token.kind == TokenKind::From)
                    .map_or(Type::Unknown, |index| parse_type(&tokens[index + 1..]));
                model
                    .types
                    .insert(declaration.name.clone(), TypeDefinition::Newtype { base });
                facts.push(SemanticFact {
                    id: FactId {
                        node: declaration.id,
                        slot: 0,
                    },
                    kind: SemanticFactKind::Type,
                    name: declaration.name.clone(),
                    ty: None,
                    range: declaration.range,
                });
            }
            DeclarationKind::Record => {
                let mut fields = BTreeMap::new();
                let mut invariant = None;
                for line in &declaration.lines {
                    if line.kind == LineKind::Field && line.tokens.len() >= 4 {
                        let name = line.tokens[1].text.clone();
                        let ty = parse_type(&line.tokens[3..]);
                        fields.insert(
                            name.clone(),
                            FieldDefinition {
                                ty: ty.clone(),
                                range: line.range,
                            },
                        );
                        facts.push(SemanticFact {
                            id: FactId {
                                node: line.id,
                                slot: 0,
                            },
                            kind: SemanticFactKind::Field,
                            name: format!("{}.{}", declaration.name, name),
                            ty: Some(ty),
                            range: line.range,
                        });
                    } else if line.kind == LineKind::Invariant {
                        invariant = parse_invariant(&line.tokens);
                    }
                }
                model.types.insert(
                    declaration.name.clone(),
                    TypeDefinition::Record { fields, invariant },
                );
                facts.push(SemanticFact {
                    id: FactId {
                        node: declaration.id,
                        slot: 0,
                    },
                    kind: SemanticFactKind::Type,
                    name: declaration.name.clone(),
                    ty: None,
                    range: declaration.range,
                });
            }
            DeclarationKind::Function => {
                if let Some(function) = parse_function(declaration) {
                    facts.push(SemanticFact {
                        id: FactId {
                            node: declaration.id,
                            slot: 0,
                        },
                        kind: SemanticFactKind::Function,
                        name: function.name.clone(),
                        ty: Some(function.returns.clone()),
                        range: function.range,
                    });
                    for (index, parameter) in function.parameters.iter().enumerate() {
                        facts.push(SemanticFact {
                            id: FactId {
                                node: declaration.id,
                                slot: u16::try_from(index + 1).expect("parameter count is bounded"),
                            },
                            kind: SemanticFactKind::Parameter,
                            name: format!("{}.{}", function.name, parameter.name),
                            ty: Some(parameter.ty.clone()),
                            range: parameter.range,
                        });
                    }
                    model.functions.insert(function.name.clone(), function);
                }
            }
            _ => {}
        }
    }
    (model, facts)
}

fn parse_function(declaration: &sico_hir::Declaration) -> Option<FunctionDefinition> {
    let header = &declaration.lines.first()?.tokens;
    let function = header
        .iter()
        .position(|token| token.kind == TokenKind::Function)?;
    let name_token = header.get(function + 1)?;
    let left = header[function + 2..]
        .iter()
        .position(|token| token.kind == TokenKind::LeftParen)?
        + function
        + 2;
    let right = matching_close(header, left)?;
    let returns = header
        .iter()
        .position(|token| token.kind == TokenKind::Returns)?;
    let parameters = split_top_level(&header[left + 1..right], TokenKind::Comma)
        .into_iter()
        .filter(|tokens| !tokens.is_empty())
        .filter_map(parse_parameter)
        .collect();
    let end = header
        .iter()
        .rposition(|token| token.kind != TokenKind::Colon)
        .unwrap_or(header.len() - 1);
    Some(FunctionDefinition {
        name: name_token.text.clone(),
        parameters,
        returns: parse_type(&header[returns + 1..=end]),
        body: declaration.lines.iter().skip(1).cloned().collect(),
        range: declaration.range,
    })
}

fn parse_parameter(tokens: &[HirToken]) -> Option<Parameter> {
    let colon = top_level_kind(tokens, TokenKind::Colon)?;
    let name = tokens[..colon]
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Identifier | TokenKind::SelfKeyword))?;
    Some(Parameter {
        name: name.text.clone(),
        ty: parse_type(&tokens[colon + 1..]),
        range: TextRange::new(name.range.start(), tokens.last()?.range.end()),
    })
}

fn parse_type(tokens: &[HirToken]) -> Type {
    let Some(first) = tokens.first() else {
        return Type::Unknown;
    };
    if first.kind != TokenKind::Identifier {
        return Type::Unknown;
    }
    if tokens
        .get(1)
        .is_some_and(|token| token.kind == TokenKind::LeftBracket)
    {
        let Some(close) = matching_close(tokens, 1) else {
            return Type::Unknown;
        };
        let arguments = split_top_level(&tokens[2..close], TokenKind::Comma)
            .into_iter()
            .map(parse_type)
            .collect();
        Type::Generic {
            name: first.text.clone(),
            arguments,
        }
    } else {
        Type::named(first.text.clone())
    }
}

fn parse_invariant(tokens: &[HirToken]) -> Option<Invariant> {
    if tokens.len() == 4 && tokens[2].kind == TokenKind::LessEqual {
        Some(Invariant {
            left: tokens[1].text.clone(),
            right: tokens[3].text.clone(),
            expression: format!("{} <= {}", tokens[1].text, tokens[3].text),
        })
    } else {
        None
    }
}

fn analyze_function(
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let mut locals: BTreeMap<String, Type> = function
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
        .collect();
    for line in &function.body {
        match line.kind {
            LineKind::Let if line.tokens.len() >= 4 => {
                let value = infer_expression(&line.tokens[3..], &locals, model, diagnostics);
                locals.insert(line.tokens[1].text.clone(), value.ty.clone());
                facts.push(SemanticFact {
                    id: FactId {
                        node: line.id,
                        slot: 0,
                    },
                    kind: SemanticFactKind::Local,
                    name: format!("{}.{}", function.name, line.tokens[1].text),
                    ty: Some(value.ty),
                    range: line.tokens[1].range,
                });
            }
            LineKind::Return if line.tokens.len() >= 2 => {
                let value = infer_expression(&line.tokens[1..], &locals, model, diagnostics);
                require_type(&function.returns, &value, diagnostics);
            }
            LineKind::Expression => {
                let _ = infer_expression(&line.tokens, &locals, model, diagnostics);
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_lines)]
fn infer_expression(
    tokens: &[HirToken],
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let range = token_range(tokens);
    if tokens.is_empty() {
        return unknown(range);
    }
    if let Some(index) = top_level_any(tokens, &[TokenKind::EqualEqual, TokenKind::LessEqual]) {
        let _left = infer_expression(&tokens[..index], locals, model, diagnostics);
        let _right = infer_expression(&tokens[index + 1..], locals, model, diagnostics);
        return Value {
            ty: Type::named("Bool"),
            range,
            integer: None,
        };
    }
    if let Some(index) = top_level_kind(tokens, TokenKind::Plus) {
        let left = infer_expression(&tokens[..index], locals, model, diagnostics);
        let right = infer_expression(&tokens[index + 1..], locals, model, diagnostics);
        if left.ty == right.ty && !left.ty.is_unknown() {
            return Value {
                ty: left.ty,
                range,
                integer: left
                    .integer
                    .zip(right.integer)
                    .and_then(|(a, b)| a.checked_add(b)),
            };
        }
        return unknown(range);
    }

    if matches!(tokens[0].kind, TokenKind::Move | TokenKind::Borrow) {
        return infer_expression(&tokens[1..], locals, model, diagnostics);
    }
    if tokens[0].kind == TokenKind::LeftParen && matching_close(tokens, 0) == Some(tokens.len() - 1)
    {
        return infer_expression(&tokens[1..tokens.len() - 1], locals, model, diagnostics);
    }
    if let Some(left) = top_level_kind(tokens, TokenKind::LeftParen)
        && matching_close(tokens, left) == Some(tokens.len() - 1)
    {
        return infer_call(tokens, left, locals, model, diagnostics);
    }
    if tokens.len() == 1 {
        let token = &tokens[0];
        return match token.kind {
            TokenKind::Integer => Value {
                ty: Type::named("Int"),
                range,
                integer: token.text.parse().ok(),
            },
            TokenKind::String => Value {
                ty: Type::named("Text"),
                range,
                integer: None,
            },
            TokenKind::True | TokenKind::False => Value {
                ty: Type::named("Bool"),
                range,
                integer: None,
            },
            TokenKind::Identifier if token.text == "Unit" => Value {
                ty: Type::named("Unit"),
                range,
                integer: None,
            },
            TokenKind::Identifier => Value {
                ty: locals.get(&token.text).cloned().unwrap_or(Type::Unknown),
                range,
                integer: None,
            },
            _ => unknown(range),
        };
    }
    if tokens.len() == 3 && tokens[1].kind == TokenKind::Dot {
        let base = infer_expression(&tokens[..1], locals, model, diagnostics);
        if let Type::Named(type_name) = &base.ty
            && let Some(TypeDefinition::Record { fields, .. }) = model.types.get(type_name)
            && let Some(field) = fields.get(&tokens[2].text)
        {
            return Value {
                ty: field.ty.clone(),
                range,
                integer: None,
            };
        }
    }
    unknown(range)
}

fn infer_call(
    tokens: &[HirToken],
    left: usize,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let range = token_range(tokens);
    let callee = tokens[..left]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<String>();
    let arguments = split_arguments(&tokens[left + 1..tokens.len() - 1]);
    let values: Vec<_> = arguments
        .iter()
        .map(|argument| infer_expression(argument.tokens, locals, model, diagnostics))
        .collect();

    if callee == "Float64.from_int" {
        if let Some(value) = values.first() {
            require_type(&Type::named("Int"), value, diagnostics);
        }
        return Value {
            ty: Type::named("Float64"),
            range,
            integer: None,
        };
    }
    if let Some(function) = model.functions.get(&callee) {
        for (parameter, value) in function.parameters.iter().zip(&values) {
            require_type(&parameter.ty, value, diagnostics);
        }
        return Value {
            ty: function.returns.clone(),
            range,
            integer: None,
        };
    }
    if let Some(definition) = model.types.get(&callee) {
        match definition {
            TypeDefinition::Newtype { base } => {
                if let Some(value) = values.first() {
                    require_type(base, value, diagnostics);
                }
            }
            TypeDefinition::Record { fields, invariant } => {
                check_record_constructor(
                    &callee,
                    fields,
                    invariant.as_ref(),
                    &arguments,
                    &values,
                    range,
                    diagnostics,
                );
            }
        }
        return Value {
            ty: Type::named(callee),
            range,
            integer: None,
        };
    }
    unknown(range)
}

fn check_record_constructor(
    record: &str,
    fields: &BTreeMap<String, FieldDefinition>,
    invariant: Option<&Invariant>,
    arguments: &[Argument<'_>],
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    let mut provided = BTreeMap::new();
    for (argument, value) in arguments.iter().zip(values) {
        let Some(name) = &argument.name else {
            continue;
        };
        if let Some(field) = fields.get(name) {
            require_type(&field.ty, value, diagnostics);
            provided.insert(name.clone(), value.clone());
        } else {
            push_diagnostic(
                diagnostics,
                "E2011",
                "UNKNOWN_FIELD",
                format!("unknown field: {name}"),
                [("field", name.as_str())],
                argument.tokens.first().map_or(range, |token| token.range),
            );
        }
    }
    for (name, field) in fields {
        if !provided.contains_key(name) {
            push_diagnostic(
                diagnostics,
                "E2010",
                "MISSING_FIELD",
                format!("missing field: {name}"),
                [("field", name.as_str())],
                TextRange::empty(range.end()),
            );
            let _ = field.range;
        }
    }
    if let Some(invariant) = invariant
        && let (Some(left), Some(right)) = (
            provided
                .get(&invariant.left)
                .and_then(|value| value.integer),
            provided
                .get(&invariant.right)
                .and_then(|value| value.integer),
        )
        && left > right
    {
        push_diagnostic(
            diagnostics,
            "E2020",
            "INVARIANT_VIOLATION",
            format!("{record} invariant is false: {}", invariant.expression),
            [
                ("record", record),
                ("expression", invariant.expression.as_str()),
            ],
            range,
        );
    }
}

fn require_type(expected: &Type, value: &Value, diagnostics: &mut Vec<SemanticDiagnostic>) {
    if expected == &value.ty || expected.is_unknown() || value.ty.is_unknown() {
        return;
    }
    let expected_name = expected.to_string();
    let found_name = value.ty.to_string();
    let numeric = matches!(expected, Type::Named(name) if matches!(name.as_str(), "Float32" | "Float64"))
        && value.ty == Type::named("Int");
    let (code, key, message) = if numeric {
        (
            "E2002",
            "IMPLICIT_NUMERIC_CONVERSION",
            format!("expected {expected_name}, found {found_name}; convert explicitly"),
        )
    } else {
        (
            "E2001",
            "TYPE_MISMATCH",
            format!("expected {expected_name}, found {found_name}"),
        )
    };
    push_diagnostic(
        diagnostics,
        code,
        key,
        message,
        [
            ("expected", expected_name.as_str()),
            ("found", found_name.as_str()),
        ],
        value.range,
    );
}

fn push_diagnostic<'a, const N: usize>(
    diagnostics: &mut Vec<SemanticDiagnostic>,
    code: &'static str,
    key: &'static str,
    message: String,
    arguments: [(&'a str, &'a str); N],
    range: TextRange,
) {
    if diagnostics.len() == MAX_SEMANTIC_DIAGNOSTICS {
        return;
    }
    diagnostics.push(SemanticDiagnostic {
        code,
        key,
        message,
        arguments: arguments
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
        range,
    });
}

fn split_arguments(tokens: &[HirToken]) -> Vec<Argument<'_>> {
    split_top_level(tokens, TokenKind::Comma)
        .into_iter()
        .filter(|tokens| !tokens.is_empty())
        .map(|tokens| {
            if let Some(colon) = top_level_kind(tokens, TokenKind::Colon) {
                Argument {
                    name: tokens.first().map(|token| token.text.clone()),
                    tokens: &tokens[colon + 1..],
                }
            } else {
                Argument { name: None, tokens }
            }
        })
        .collect()
}

fn split_top_level(tokens: &[HirToken], separator: TokenKind) -> Vec<&[HirToken]> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => depth += 1,
            TokenKind::RightParen | TokenKind::RightBracket => depth = depth.saturating_sub(1),
            kind if kind == separator && depth == 0 => {
                result.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    result.push(&tokens[start..]);
    result
}

fn top_level_any(tokens: &[HirToken], kinds: &[TokenKind]) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => {
                if depth == 0 && kinds.contains(&token.kind) {
                    return Some(index);
                }
                depth += 1;
            }
            TokenKind::RightParen | TokenKind::RightBracket => depth = depth.saturating_sub(1),
            kind if depth == 0 && kinds.contains(&kind) => return Some(index),
            _ => {}
        }
    }
    None
}

fn top_level_kind(tokens: &[HirToken], kind: TokenKind) -> Option<usize> {
    top_level_any(tokens, &[kind])
}

fn matching_close(tokens: &[HirToken], open: usize) -> Option<usize> {
    let expected = match tokens.get(open)?.kind {
        TokenKind::LeftParen => TokenKind::RightParen,
        TokenKind::LeftBracket => TokenKind::RightBracket,
        _ => return None,
    };
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.kind == tokens[open].kind {
            depth += 1;
        } else if token.kind == expected {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn token_range(tokens: &[HirToken]) -> TextRange {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => TextRange::new(first.range.start(), last.range.end()),
        _ => TextRange::empty(0.into()),
    }
}

fn unknown(range: TextRange) -> Value {
    Value {
        ty: Type::Unknown,
        range,
        integer: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::Value as JsonValue;
    use sico_source::SourceId;

    use super::*;

    #[test]
    fn numbers_and_nominal_cases_match_the_registered_oracle() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let map: JsonValue = serde_json::from_str(
            &fs::read_to_string(repository.join("diagnostics/semantic-case-map.json")).unwrap(),
        )
        .unwrap();
        let expected: BTreeMap<_, _> = map["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| {
                case["case"].as_str().unwrap().starts_with("NUM-")
                    || case["case"].as_str().unwrap().starts_with("NOM-")
            })
            .map(|case| (case["case"].as_str().unwrap().to_owned(), case.clone()))
            .collect();
        assert_eq!(expected.len(), 9);

        let mut paths = Vec::new();
        collect_sico(
            &repository.join("syntax-candidates/b/numbers-units"),
            &mut paths,
        );
        collect_sico(
            &repository.join("syntax-candidates/b/nominal-invariants"),
            &mut paths,
        );
        paths.sort();
        assert_eq!(paths.len(), 16);
        let mut accepted = 0;
        let mut rejected = 0;
        for (index, path) in paths.iter().enumerate() {
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            let analysis = analyze(&source).unwrap();
            assert_eq!(analysis, analyze(&source).unwrap());
            assert!(!analysis.facts.is_empty(), "{}", path.display());
            assert!(
                analysis
                    .facts
                    .windows(2)
                    .all(|pair| pair[0].id < pair[1].id)
            );
            assert!(
                analysis
                    .facts
                    .iter()
                    .all(|fact| source.span(fact.range).is_some())
            );
            if source.text().contains("// expect: accept") {
                accepted += 1;
                assert!(
                    analysis.diagnostics.is_empty(),
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
            } else {
                rejected += 1;
                let case_id = metadata(source.text(), "case");
                let oracle = &expected[case_id];
                assert_eq!(
                    analysis.diagnostics.len(),
                    1,
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
                let diagnostic = &analysis.diagnostics[0];
                assert_eq!(diagnostic.code, oracle["code"]);
                assert_eq!(diagnostic.key, oracle["key"]);
                assert_eq!(diagnostic.message, oracle["expected_message"]);
                for (name, value) in oracle["arguments"].as_object().unwrap() {
                    assert_eq!(diagnostic.arguments[name], value.as_str().unwrap());
                }
                assert!(source.span(diagnostic.range).is_some());
            }
        }
        assert_eq!((accepted, rejected), (7, 9));
    }

    #[test]
    fn remaining_b_groups_do_not_receive_unowned_diagnostics() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-candidates/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 54);
        for (index, path) in paths.iter().enumerate() {
            if path.to_string_lossy().contains("numbers-units")
                || path.to_string_lossy().contains("nominal-invariants")
            {
                continue;
            }
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            let analysis = analyze(&source).unwrap();
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .all(|diagnostic| !diagnostic.code.starts_with("E2")),
                "{}: {:?}",
                path.display(),
                analysis.diagnostics
            );
        }
    }

    fn metadata<'a>(text: &'a str, key: &str) -> &'a str {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("// {key}: ")))
            .unwrap()
    }

    fn collect_sico(root: &Path, output: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_sico(&path, output);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "sico")
            {
                output.push(path);
            }
        }
    }
}
