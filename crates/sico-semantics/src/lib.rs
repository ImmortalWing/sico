//! Static semantic analysis over deterministic Sico HIR.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

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
    pub arguments: BTreeMap<String, DiagnosticArgument>,
    pub range: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticArgument {
    Text(String),
    List(Vec<String>),
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
    Variant,
    Match,
    Effect,
    Capability,
    ComponentCall,
    ResourceState,
    AsyncState,
    StreamOperation,
    Revision,
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
    Enum {
        variants: Vec<VariantDefinition>,
    },
    Resource {
        methods: BTreeMap<String, ResourceMethod>,
    },
}

#[derive(Clone, Debug)]
struct VariantDefinition {
    name: String,
    payload: Vec<Type>,
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
    declared_effects: Vec<(String, HirId, TextRange)>,
    declared_capabilities: Vec<(String, HirId, TextRange)>,
    component_version_range: Option<TextRange>,
    is_async: bool,
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
    /// RFC-0039 §2.2 (STEP-0144): qualified `module.item` signatures of
    /// the analyzed file's imports; consulted after local functions.
    /// Package functions (STEP-0147) ride the same table under their
    /// `<package>.<function>` qualified name.
    imported_functions: BTreeMap<String, ImportedFunction>,
    capabilities: BTreeMap<String, BTreeMap<String, CapabilityMethod>>,
    /// RFC-0039 §2.4 (STEP-0147): user WIT interfaces declared in this
    /// file, keyed by interface name.
    interfaces: BTreeMap<String, ModelInterface>,
}

/// One user WIT interface's check-time shape (RFC-0039 §2.4, STEP-0147).
/// A `None` version is the accepted version-less corpus form: registered
/// and shape-parsed, with no boundary meaning in v0.
#[derive(Clone, Debug, Default)]
pub struct ModelInterface {
    pub version: Option<u64>,
    pub functions: BTreeMap<String, ImportedFunction>,
}

/// One resolved package's check-time surface (RFC-0039 §2.3/§2.4,
/// STEP-0147): the exposed interface identity plus its qualified function
/// signatures.
#[derive(Clone, Debug)]
pub struct PackageInterface {
    pub package: String,
    pub version: u64,
    pub interface: String,
    pub functions: BTreeMap<String, ImportedFunction>,
}

/// One resource method's check-time signature (RFC-0039 §2.2,
/// STEP-0144).
#[derive(Clone, Debug)]
struct ResourceMethod {
    parameters: Vec<Parameter>,
    returns: Type,
    consumes_self: bool,
}

/// One capability method's check-time signature (RFC-0039 §2.2,
/// STEP-0144).
#[derive(Clone, Debug)]
struct CapabilityMethod {
    parameters: Vec<Parameter>,
    returns: Type,
}

/// One imported module function's check-time signature (RFC-0039 §2.2,
/// STEP-0144).
#[derive(Clone, Debug)]
pub struct ImportedFunction {
    pub parameters: Vec<Type>,
    pub returns: Type,
    pub is_async: bool,
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
    analyze_with_modules(source, &BTreeMap::new())
}

/// RFC-0039 §2.2 (STEP-0144): analysis with the module-import surface
/// injected, so qualified `module.item` calls resolve and type-check at
/// check time. [`analyze`] is this with an empty surface.
///
/// # Errors
///
/// Same contract as [`analyze`].
pub fn analyze_with_modules(
    source: &SourceFile,
    modules: &BTreeMap<String, BTreeMap<String, ImportedFunction>>,
) -> Result<Analysis, AnalyzeError> {
    analyze_with_interfaces(source, modules, &BTreeMap::new())
}

/// RFC-0039 §2.3/§2.4 (STEP-0147): analysis with the module-import surface
/// and the resolved package surfaces injected. Package functions bind
/// qualified `<package>.<function>` calls exactly like module imports;
/// the interfaces the file declares are registered on the model and —
/// when versioned — value-set checked.
///
/// # Errors
///
/// Same contract as [`analyze`].
pub fn analyze_with_interfaces(
    source: &SourceFile,
    modules: &BTreeMap<String, BTreeMap<String, ImportedFunction>>,
    packages: &BTreeMap<String, PackageInterface>,
) -> Result<Analysis, AnalyzeError> {
    let module = lower(source).map_err(AnalyzeError::Lower)?;
    let mut diagnostics = Vec::new();
    let (mut model, mut facts) = build_model(&module, &mut diagnostics);
    for (module_name, functions) in modules {
        for (name, imported) in functions {
            model
                .imported_functions
                .insert(format!("{module_name}.{name}"), imported.clone());
        }
    }
    for (package_name, package) in packages {
        for (name, imported) in &package.functions {
            model
                .imported_functions
                .insert(format!("{package_name}.{name}"), imported.clone());
        }
    }
    for function in model.functions.values() {
        analyze_function(function, &model, &mut diagnostics, &mut facts);
    }
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range.start(), diagnostic.code));
    facts.sort_by_key(|fact| fact.id);
    Ok(Analysis { diagnostics, facts })
}

/// RFC-0039 §2.2 (STEP-0144): the top-level functions one module file
/// exports, keyed by name, for qualified cross-module call resolution.
/// The source must parse (the CLI link pass guarantees it); a failed
/// lowering yields an empty surface.
/// RFC-0039 §2.4 (STEP-0147): the user WIT interfaces one file declares,
/// keyed by interface name, for package-expose validation. The source must
/// parse (the CLI link pass guarantees it); a failed lowering yields an
/// empty map.
#[must_use]
pub fn declared_interfaces(source: &SourceFile) -> BTreeMap<String, ModelInterface> {
    let Ok(module) = lower(source) else {
        return BTreeMap::new();
    };
    let (model, _facts) = build_model(&module, &mut Vec::new());
    model
        .interfaces
        .into_iter()
        .filter(|(_, interface)| interface.version.is_some())
        .collect()
}

#[must_use]
pub fn exported_functions(source: &SourceFile) -> BTreeMap<String, ImportedFunction> {
    let Ok(module) = lower(source) else {
        return BTreeMap::new();
    };
    module
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == DeclarationKind::Function)
        .filter_map(|declaration| {
            parse_function(declaration).map(|function| {
                (
                    function.name.clone(),
                    ImportedFunction {
                        parameters: function.parameters.iter().map(|p| p.ty.clone()).collect(),
                        returns: function.returns.clone(),
                        is_async: function.is_async,
                    },
                )
            })
        })
        .collect()
}

#[allow(clippy::too_many_lines)] // the model construction is one coherent table
fn build_model(
    module: &Module,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> (Model, Vec<SemanticFact>) {
    let mut model = Model::default();
    let mut facts = Vec::new();
    let mut export_prefixed: Vec<(String, TextRange)> = Vec::new();
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
                push_type_fact(declaration, &mut facts);
            }
            DeclarationKind::Record => {
                let mut fields = BTreeMap::new();
                let mut invariant = None;
                for line in &declaration.lines {
                    // RFC-0047 D1 (STEP-0271): the script-profile surface
                    // uses bare `name: Type` field lines; the pre-existing
                    // component/WIT surface uses `field name: Type`. Both
                    // collect into the same nominal record; a bare line is
                    // an Expression-classified `Identifier : Type…` shape.
                    let bare_field = line.kind == LineKind::Expression
                        && line.tokens.len() >= 3
                        && line.tokens[0].kind == TokenKind::Identifier
                        && line.tokens[1].kind == TokenKind::Colon;
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
                    } else if bare_field {
                        let name = line.tokens[0].text.clone();
                        let ty = parse_type(&line.tokens[2..]);
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
                // RFC-0047 EC-1: an empty record declaration is a typed
                // refusal (E2022), never a vacuous type.
                if fields.is_empty() {
                    push_diagnostic(
                        diagnostics,
                        "E2022",
                        "EMPTY_RECORD",
                        format!("record {} declares no fields", declaration.name),
                        [("record", declaration.name.as_str())],
                        declaration.range,
                    );
                }
                model.types.insert(
                    declaration.name.clone(),
                    TypeDefinition::Record { fields, invariant },
                );
                push_type_fact(declaration, &mut facts);
            }
            DeclarationKind::Enum => {
                let variants = build_enum_variants(declaration, &mut facts);
                model
                    .types
                    .insert(declaration.name.clone(), TypeDefinition::Enum { variants });
                push_type_fact(declaration, &mut facts);
            }
            DeclarationKind::Resource => {
                let methods = build_resource_methods(declaration);
                model.types.insert(
                    declaration.name.clone(),
                    TypeDefinition::Resource { methods },
                );
                push_type_fact(declaration, &mut facts);
            }
            DeclarationKind::Capability => {
                model.capabilities.insert(
                    declaration.name.clone(),
                    build_capability_methods(declaration),
                );
            }
            DeclarationKind::Function => {
                // RFC-0039 §2.5 D1 (STEP-0147, implementation refinement):
                // `export function` has no v0 meaning — the Script `run`
                // export is the only export. The refusal fires only for
                // script-profile programs (those defining the `main`
                // entry); the M2-era boundary-style oracles use the same
                // prefix without a `main` and stay accepted per corpus.
                if declaration
                    .lines
                    .first()
                    .and_then(|line| line.tokens.first())
                    .is_some_and(|token| token.kind == TokenKind::Export)
                {
                    export_prefixed.push((declaration.name.clone(), declaration.range));
                }
                insert_function_definition(declaration, &mut model, &mut facts);
            }
            // RFC-0039 §2.4 (STEP-0147): interfaces are registered with
            // their shape. A versioned interface is the boundary surface —
            // its functions are value-set checked with typed refusals. The
            // version-less corpus form is registered without a boundary
            // meaning, replacing the former silent drop with a declared
            // classification. `module`/`use` carry no per-file semantics —
            // the CLI link pass verifies them across the assembled set.
            DeclarationKind::Interface => {
                let version = match &declaration.detail {
                    sico_parser::DeclarationDetail::InterfaceVersion(version) => Some(*version),
                    _ => None,
                };
                let mut functions = BTreeMap::new();
                for (name, imported, line_range) in build_interface_functions(declaration) {
                    if version.is_some()
                        && let Some(offender) = interface_value_offender(&imported)
                    {
                        push_diagnostic(
                            diagnostics,
                            "E8017",
                            "WIT_UNSUPPORTED_TYPE",
                            format!(
                                "interface {} function {name}: {offender} is outside the v0 user-WIT value set",
                                declaration.name
                            ),
                            [],
                            line_range,
                        );
                    }
                    functions.insert(name, imported);
                }
                model.interfaces.insert(
                    declaration.name.clone(),
                    ModelInterface { version, functions },
                );
            }
            DeclarationKind::Module | DeclarationKind::Use => {}
        }
    }
    // D1 refusal, scoped to script-profile programs (those with `main`).
    if model.functions.contains_key("main") {
        for (name, range) in &export_prefixed {
            push_diagnostic(
                diagnostics,
                "E8019",
                "EXPORT_USER_INTERFACE",
                format!(
                    "function {name} carries an export prefix; user-interface exports are refused in v0"
                ),
                [],
                *range,
            );
        }
    }
    (model, facts)
}

fn insert_function_definition(
    declaration: &sico_hir::Declaration,
    model: &mut Model,
    facts: &mut Vec<SemanticFact>,
) {
    let Some(function) = parse_function(declaration) else {
        return;
    };
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

fn push_type_fact(declaration: &sico_hir::Declaration, facts: &mut Vec<SemanticFact>) {
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

fn build_resource_methods(declaration: &sico_hir::Declaration) -> BTreeMap<String, ResourceMethod> {
    declaration
        .lines
        .iter()
        .filter(|line| line.kind == LineKind::FunctionSignature)
        .filter_map(|line| {
            let name = line.tokens.get(1)?.text.clone();
            let left = line
                .tokens
                .iter()
                .position(|token| token.kind == TokenKind::LeftParen)?;
            let right = matching_close(&line.tokens, left)?;
            let borrowed = line.tokens[left + 1..right]
                .iter()
                .any(|token| token.kind == TokenKind::Borrow);
            // RFC-0039 section 2.2 (STEP-0144): methods carry parameters and
            // return types so resource calls resolve fully at check time.
            // The receiver (`self` / `borrow self`) is not a guest argument.
            let parameters = split_top_level(&line.tokens[left + 1..right], TokenKind::Comma)
                .into_iter()
                .filter(|tokens| !tokens.is_empty())
                .filter_map(parse_parameter)
                .collect();
            let returns = line
                .tokens
                .iter()
                .position(|token| token.kind == TokenKind::Returns)
                .map_or(Type::named("Unit"), |returns| {
                    parse_type(&line.tokens[returns + 1..])
                });
            Some((
                name,
                ResourceMethod {
                    parameters,
                    returns,
                    consumes_self: !borrowed,
                },
            ))
        })
        .collect()
}

fn build_capability_methods(
    declaration: &sico_hir::Declaration,
) -> BTreeMap<String, CapabilityMethod> {
    declaration
        .lines
        .iter()
        .filter(|line| line.kind == LineKind::FunctionSignature)
        .filter_map(|line| {
            let name = line.tokens.get(1)?.text.clone();
            let left = line
                .tokens
                .iter()
                .position(|token| token.kind == TokenKind::LeftParen)?;
            let right = matching_close(&line.tokens, left)?;
            let parameters = split_top_level(&line.tokens[left + 1..right], TokenKind::Comma)
                .into_iter()
                .filter(|tokens| !tokens.is_empty())
                .filter_map(parse_parameter)
                .collect();
            // RFC-0039 section 2.2 (STEP-0144): methods carry their return
            // type so capability calls resolve fully at check time.
            let returns = line
                .tokens
                .iter()
                .position(|token| token.kind == TokenKind::Returns)
                .map_or(Type::named("Unit"), |returns| {
                    parse_type(&line.tokens[returns + 1..])
                });
            Some((
                name,
                CapabilityMethod {
                    parameters,
                    returns,
                },
            ))
        })
        .collect()
}

/// RFC-0039 §2.4 (STEP-0147): parses one interface body's function
/// signatures with their line ranges. Mirrors the capability-method shape:
/// `function <name>(<params>) returns <type>`.
fn build_interface_functions(
    declaration: &sico_hir::Declaration,
) -> Vec<(String, ImportedFunction, TextRange)> {
    let mut result = Vec::new();
    for line in &declaration.lines {
        if line.kind != LineKind::FunctionSignature {
            continue;
        }
        let Some(function_index) = line
            .tokens
            .iter()
            .position(|token| token.kind == TokenKind::Function)
        else {
            continue;
        };
        let Some(name) = line
            .tokens
            .get(function_index + 1)
            .map(|token| token.text.clone())
        else {
            continue;
        };
        let Some(left) = line
            .tokens
            .iter()
            .position(|token| token.kind == TokenKind::LeftParen)
        else {
            continue;
        };
        let Some(right) = matching_close(&line.tokens, left) else {
            continue;
        };
        let parameters: Vec<Type> =
            split_top_level(&line.tokens[left + 1..right], TokenKind::Comma)
                .into_iter()
                .filter(|tokens| !tokens.is_empty())
                .filter_map(parse_parameter)
                .map(|parameter| parameter.ty)
                .collect();
        let returns = line
            .tokens
            .iter()
            .position(|token| token.kind == TokenKind::Returns)
            .map_or_else(
                || Type::named("Unit"),
                |returns| parse_type(&line.tokens[returns + 1..]),
            );
        let is_async = line.tokens[0].kind == TokenKind::Async;
        result.push((
            name,
            ImportedFunction {
                parameters,
                returns,
                is_async,
            },
            line.range,
        ));
    }
    result
}

/// RFC-0039 §2.4 amendment A7 (STEP-0147): the v0 user-WIT value set is the
/// flat subset — parameters and returns are scalars (`Bool`, `I64`, `U64`,
/// `Text`, `Bytes`) or one level of `List[T]`/`Option[T]` over a scalar;
/// returns may additionally be `Result[ok, error]` with `error` a scalar.
/// Nested compositions, records/enums, fixed-width crossing of `Int`,
/// floats, maps/sets, async, and resource-bearing shapes stay outside v0
/// and are refused. Returns the offending type rendered, if any.
fn interface_value_offender(imported: &ImportedFunction) -> Option<String> {
    fn scalar(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Named(name)
                if matches!(name.as_str(), "Bool" | "I64" | "U64" | "Text" | "Bytes")
        )
    }
    fn level1(ty: &Type) -> bool {
        scalar(ty)
            || matches!(ty, Type::Generic { name, arguments }
                if name == "List"
                    && arguments.len() == 1
                    && matches!(&arguments[0], Type::Named(unit) if unit == "Text"))
    }
    if imported.is_async {
        return Some("async functions".to_owned());
    }
    for parameter in &imported.parameters {
        if !level1(parameter) {
            return Some(parameter.to_string());
        }
    }
    let returns = &imported.returns;
    if level1(returns) || matches!(returns, Type::Named(name) if name == "Unit") {
        return None;
    }
    if let Type::Generic { name, arguments } = returns
        && name == "Result"
        && arguments.len() == 2
    {
        let [ok, error] = arguments.as_slice() else {
            return Some(returns.to_string());
        };
        let ok_ok = level1(ok)
            || matches!(ok, Type::Named(unit) if unit == "Unit")
            || matches!(ok, Type::Named(unit) if unit == "Bool");
        if ok_ok && scalar(error) {
            return None;
        }
    }
    Some(returns.to_string())
}

fn build_enum_variants(
    declaration: &sico_hir::Declaration,
    facts: &mut Vec<SemanticFact>,
) -> Vec<VariantDefinition> {
    let mut variants = Vec::new();
    for line in &declaration.lines {
        if line.kind != LineKind::Variant || line.tokens.len() < 2 {
            continue;
        }
        let name = line.tokens[1].text.clone();
        let payload = line
            .tokens
            .iter()
            .position(|token| token.kind == TokenKind::LeftParen)
            .and_then(|left| matching_close(&line.tokens, left).map(|right| (left, right)))
            .map_or_else(Vec::new, |(left, right)| {
                split_top_level(&line.tokens[left + 1..right], TokenKind::Comma)
                    .into_iter()
                    .filter(|tokens| !tokens.is_empty())
                    .filter_map(parse_parameter)
                    .map(|parameter| parameter.ty)
                    .collect()
            });
        variants.push(VariantDefinition {
            name: name.clone(),
            payload,
        });
        facts.push(SemanticFact {
            id: FactId {
                node: line.id,
                slot: 0,
            },
            kind: SemanticFactKind::Variant,
            name: format!("{}.{}", declaration.name, name),
            ty: Some(Type::named(declaration.name.clone())),
            range: line.range,
        });
    }
    variants
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
    let body: Vec<_> = declaration.lines.iter().skip(1).cloned().collect();
    Some(FunctionDefinition {
        name: name_token.text.clone(),
        parameters,
        returns: parse_type(&header[returns + 1..=end]),
        declared_effects: parse_boundary_items(&body, LineKind::Effects),
        declared_capabilities: parse_boundary_items(&body, LineKind::Capabilities),
        component_version_range: header
            .iter()
            .find(|token| token.kind == TokenKind::At)
            .map(|token| token.range),
        is_async: header.iter().any(|token| token.kind == TokenKind::Async),
        body,
        range: declaration.range,
    })
}

fn parse_boundary_items(lines: &[Line], section: LineKind) -> Vec<(String, HirId, TextRange)> {
    let Some(index) = lines.iter().position(|line| line.kind == section) else {
        return Vec::new();
    };
    if lines[index]
        .tokens
        .iter()
        .any(|token| token.kind == TokenKind::None)
    {
        return Vec::new();
    }
    lines[index + 1..]
        .iter()
        .take_while(|line| line.kind == LineKind::Expression)
        .map(|line| {
            (
                line.tokens
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect(),
                line.id,
                line.range,
            )
        })
        .collect()
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

#[allow(clippy::too_many_lines)]
fn analyze_function(
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    check_boundary_semantics(function, diagnostics, facts);
    check_resource_async_stream(function, model, diagnostics, facts);
    check_revision_contracts(function, model, diagnostics, facts);
    let mut locals: BTreeMap<String, Type> = function
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
        .collect();
    let spawned: BTreeSet<String> = function
        .body
        .iter()
        .filter(|line| {
            line.kind == LineKind::Let
                && line
                    .tokens
                    .get(3)
                    .is_some_and(|token| token.kind == TokenKind::Spawn)
        })
        .map(|line| line.tokens[1].text.clone())
        .collect();
    // M14 STEP-0130 scoped walk: names bound inside a while/if/match region
    // are removed when the region closes (mirrors the general lowering).
    // Each entry tracks the region's close index and the names bound inside.
    let mut open_scopes: Vec<(usize, Vec<String>)> = Vec::new();
    // Close indices of open `while` regions, for break/continue validation.
    let mut open_loops: Vec<usize> = Vec::new();
    let region_closes = region_close_map(&function.body);
    for (line_index, line) in function.body.iter().enumerate() {
        open_scopes.retain(|(close_index, _)| *close_index > line_index);
        open_loops.retain(|close_index| *close_index > line_index);
        match line.kind {
            LineKind::While | LineKind::If => {
                let condition_tokens = &line.tokens[1..line.tokens.len().saturating_sub(1)];
                let condition = infer_expression(condition_tokens, &locals, model, diagnostics);
                require_type(&Type::named("Bool"), &condition, diagnostics);
            }
            LineKind::For => {
                // RFC-0046 (STEP-0175): the executable iterable set is
                // `List[Text|I64|U64]` values plus `Map[K,V]`/`Set[K]`
                // (iterating first-insertion key order); the loop variable
                // binds the element/key type until the region closes.
                let in_position = line
                    .tokens
                    .iter()
                    .position(|token| token.kind == TokenKind::In);
                let Some(in_position) = in_position else {
                    continue;
                };
                let subject = infer_expression(
                    &line.tokens[in_position + 1..line.tokens.len().saturating_sub(1)],
                    &locals,
                    model,
                    diagnostics,
                );
                let executable_element = |ty: &Type| {
                    matches!(
                        ty,
                        Type::Named(name) if matches!(name.as_str(), "Text" | "I64" | "U64")
                    ) || matches!(
                        ty,
                        Type::Named(name)
                            if matches!(
                                model.types.get(name),
                                Some(TypeDefinition::Record { fields, .. })
                                    if fields.values().all(|field| matches!(&field.ty, Type::Named(n) if n == "I64" || n == "U64"))
                            )
                    )
                };
                let binding_ty = match &subject.ty {
                    Type::Generic { name, arguments }
                        if name == "List"
                            && arguments.len() == 1
                            && executable_element(&arguments[0]) =>
                    {
                        arguments[0].clone()
                    }
                    Type::Generic { name, arguments }
                        if name == "Map"
                            && arguments.len() == 2
                            && matches!(&arguments[0], Type::Named(key) if key == "Text") =>
                    {
                        arguments[0].clone()
                    }
                    Type::Generic { name, arguments }
                        if name == "Set"
                            && arguments.len() == 1
                            && matches!(&arguments[0], Type::Named(key) if key == "Text") =>
                    {
                        arguments[0].clone()
                    }
                    _ => {
                        push_diagnostic(
                            diagnostics,
                            "E2001",
                            "TYPE_MISMATCH",
                            "for-loop subject must be an executable iterable (List[Text|I64|U64], List[record] with I64/U64 fields, Map, Set)".to_owned(),
                            [],
                            subject.range,
                        );
                        continue;
                    }
                };
                if let Some(name) = line.tokens.get(1) {
                    locals.insert(name.text.clone(), binding_ty);
                    for scope in &mut open_scopes {
                        scope.1.push(name.text.clone());
                    }
                }
            }
            LineKind::Set => {
                let Some(equal) = line
                    .tokens
                    .iter()
                    .position(|token| token.kind == TokenKind::Equal)
                else {
                    continue;
                };
                // RFC-0047 D3 (STEP-0273): per-field `set p.x = …` is a
                // typed refusal (E2023) — fields are immutable in v0; the
                // misleading whole-cell type check stands down.
                if line
                    .tokens
                    .get(2)
                    .is_some_and(|token| token.kind == TokenKind::Dot)
                    && let Some(field) = line.tokens.get(3)
                {
                    push_diagnostic(
                        diagnostics,
                        "E2023",
                        "FIELD_IMMUTABLE",
                        format!(
                            "field {}.{} is immutable; replace the whole record with set",
                            line.tokens[1].text, field.text
                        ),
                        [
                            ("record", line.tokens[1].text.as_str()),
                            ("field", field.text.as_str()),
                        ],
                        field.range,
                    );
                    continue;
                }
                let name = line.tokens.get(1).map_or("", |token| token.text.as_str());
                let declared = locals.get(name).cloned();
                let value =
                    infer_expression(&line.tokens[equal + 1..], &locals, model, diagnostics);
                if let Some(declared) = declared {
                    require_type(&declared, &value, diagnostics);
                    locals.insert(name.to_owned(), value.ty);
                }
            }
            LineKind::Let if line.tokens.len() >= 4 => {
                let propagates = line.tokens[3].kind == TokenKind::Try
                    || (line
                        .tokens
                        .last()
                        .is_some_and(|token| token.kind == TokenKind::QuestionMark)
                        && line.tokens.len() >= 5);
                let value = if line.tokens[3].kind == TokenKind::Try {
                    infer_try(
                        &line.tokens[4..],
                        &function.returns,
                        &locals,
                        model,
                        diagnostics,
                    )
                } else if propagates {
                    infer_propagate(
                        &line.tokens[3..line.tokens.len() - 1],
                        &function.returns,
                        &locals,
                        model,
                        diagnostics,
                    )
                } else {
                    infer_expression(&line.tokens[3..], &locals, model, diagnostics)
                };
                // RFC-0046 D2: a propagating `let` yields the ok payload
                // (numeric in both supported forms), never the Result
                // shape itself.
                locals.insert(line.tokens[1].text.clone(), value.ty.clone());
                for scope in &mut open_scopes {
                    scope.1.push(line.tokens[1].text.clone());
                }
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
                if is_task_type(&value.ty)
                    && task_escapes_via_return(&line.tokens[1..], line.depth, &spawned)
                {
                    push_diagnostic(
                        diagnostics,
                        "E5102",
                        "TASK_ESCAPES_SCOPE",
                        "task cannot leave its task group".to_owned(),
                        [],
                        line.range,
                    );
                }
                if line.depth <= 1 {
                    require_type(&function.returns, &value, diagnostics);
                }
            }
            LineKind::Expression => {
                check_expression_statement(&line.tokens, &locals, model, diagnostics, line.range);
            }
            LineKind::Match => {
                infer_match_scrutinee(line, &locals, model, diagnostics);
                check_match(line_index, function, model, diagnostics, facts);
            }
            LineKind::End => {
                if let Some(close) = region_closes.get(&line_index) {
                    open_loops.retain(|loop_close| *loop_close != line_index);
                    if let Some((_, names)) = open_scopes.iter().find(|(c, _)| c == close) {
                        for name in names {
                            locals.remove(name);
                        }
                    }
                    open_scopes.retain(|(c, _)| c != close);
                }
            }
            _ => {}
        }
    }
}

/// RFC-0039 section 2.2 (STEP-0144): infers a bare expression statement and
/// enforces the affine result-handling rule.
fn check_expression_statement(
    tokens: &[HirToken],
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    range: TextRange,
) {
    let value = infer_expression(tokens, locals, model, diagnostics);
    if result_parts(&value.ty).is_some() {
        push_diagnostic(
            diagnostics,
            "E3104",
            "UNHANDLED_RESULT",
            "Result must be handled, returned, or propagated".to_owned(),
            [],
            range,
        );
    }
}

/// RFC-0039 section 2.2 (STEP-0144): the match scrutinee is a normal
/// expression -- route it through inference so its callees resolve and
/// type-check at check time (If/While conditions already do).
fn infer_match_scrutinee(
    line: &Line,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    if line.tokens.len() >= 3 {
        let scrutinee = &line.tokens[1..line.tokens.len() - 1];
        infer_expression(scrutinee, locals, model, diagnostics);
    }
}

/// Maps each `end` line index to the absolute index of the construct it
/// closes, for while/if/match regions (M14 STEP-0130).
fn region_close_map(body: &[Line]) -> BTreeMap<usize, usize> {
    let mut opens: Vec<(u16, usize)> = Vec::new();
    let mut closes = BTreeMap::new();
    for (index, line) in body.iter().enumerate() {
        match line.kind {
            LineKind::While | LineKind::If | LineKind::Match => {
                opens.push((line.depth, index));
            }
            LineKind::End => {
                if let Some(position) = opens.iter().rposition(|(depth, _)| *depth == line.depth) {
                    let (_, open_index) = opens.remove(position);
                    closes.insert(index, open_index);
                }
            }
            _ => {}
        }
    }
    closes
}

fn check_revision_contracts(
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    check_revision_calls(function, model, diagnostics, facts);
    check_loaded_revision_guards(function, model, diagnostics, facts);
}

fn check_revision_calls(
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    for line in &function.body {
        let Some((receiver, method, _)) = receiver_method(line) else {
            continue;
        };
        let Some(Type::Named(capability)) = function
            .parameters
            .iter()
            .find(|parameter| parameter.name == receiver)
            .map(|parameter| &parameter.ty)
        else {
            continue;
        };
        let Some(parameters) = model
            .capabilities
            .get(capability)
            .and_then(|methods| methods.get(&method))
        else {
            continue;
        };
        if !matches!(
            parameters.parameters.first().map(|parameter| &parameter.ty),
            Some(Type::Named(name)) if name == "Revision"
        ) {
            continue;
        }
        facts.push(flow_fact(
            line,
            0,
            SemanticFactKind::Revision,
            format!("{receiver}.{method}:expected-revision"),
            Some(Type::named("Revision")),
        ));
        if !call_has_revision_argument(line, function) {
            push_diagnostic(
                diagnostics,
                "E7001",
                "MISSING_REVISION",
                "commit requires the expected revision".to_owned(),
                [],
                line.range,
            );
        }
    }
}

fn call_has_revision_argument(line: &Line, function: &FunctionDefinition) -> bool {
    let Some(left) = line
        .tokens
        .iter()
        .position(|token| token.kind == TokenKind::LeftParen)
    else {
        return false;
    };
    let Some(right) = matching_close(&line.tokens, left) else {
        return false;
    };
    let arguments = split_arguments(&line.tokens[left + 1..right]);
    let Some(first) = arguments
        .first()
        .and_then(|argument| argument.tokens.first())
    else {
        return false;
    };
    function
        .parameters
        .iter()
        .any(|parameter| parameter.name == first.text && parameter.ty == Type::named("Revision"))
}

fn check_loaded_revision_guards(
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let versioned_payloads: Vec<_> = function
        .parameters
        .iter()
        .filter_map(|parameter| {
            let Type::Named(type_name) = &parameter.ty else {
                return None;
            };
            let Some(TypeDefinition::Record { fields, .. }) = model.types.get(type_name) else {
                return None;
            };
            (fields.contains_key("text")
                && matches!(fields.get("revision").map(|field| &field.ty), Some(Type::Named(name)) if name == "Revision"))
            .then(|| parameter.name.clone())
        })
        .collect();
    for (payload_index, payload) in versioned_payloads.into_iter().enumerate() {
        let slot = u16::try_from(payload_index).expect("parameter count is bounded");
        let guards = revision_guard_ranges(function, &payload, slot, facts);
        for (index, line) in function.body.iter().enumerate() {
            if !contains_field_access(&line.tokens, &payload, "text") {
                continue;
            }
            facts.push(flow_fact(
                line,
                slot,
                SemanticFactKind::Revision,
                format!("{payload}.text:apply"),
                None,
            ));
            if !guards
                .iter()
                .any(|(start, end)| *start < index && index < *end)
            {
                push_diagnostic(
                    diagnostics,
                    "E7002",
                    "UNCHECKED_STALE_RESULT",
                    "compare loaded.revision before applying loaded.text".to_owned(),
                    [],
                    line.range,
                );
            }
        }
    }
}

fn revision_guard_ranges(
    function: &FunctionDefinition,
    payload: &str,
    slot: u16,
    facts: &mut Vec<SemanticFact>,
) -> Vec<(usize, usize)> {
    let mut guards = Vec::new();
    for (index, line) in function.body.iter().enumerate() {
        if line.kind != LineKind::If
            || !contains_field_access(&line.tokens, payload, "revision")
            || !line
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::EqualEqual)
        {
            continue;
        }
        let end = function.body[index + 1..]
            .iter()
            .position(|candidate| {
                candidate.kind == LineKind::End
                    && candidate.depth == line.depth
                    && candidate
                        .tokens
                        .get(1)
                        .is_some_and(|token| token.kind == TokenKind::If)
            })
            .map_or(function.body.len(), |offset| index + 1 + offset);
        guards.push((index, end));
        facts.push(flow_fact(
            line,
            slot,
            SemanticFactKind::Revision,
            format!("{payload}.revision:guard"),
            Some(Type::named("Revision")),
        ));
    }
    guards
}

fn contains_field_access(tokens: &[HirToken], base: &str, field: &str) -> bool {
    tokens.windows(3).any(|window| {
        window[0].text == base && window[1].kind == TokenKind::Dot && window[2].text == field
    })
}

#[derive(Clone, Debug)]
enum ResourceStatus {
    Available,
    Moved(String),
    Closed,
}

const MAX_TASK_SCOPE_NESTING: usize = 64;
const MAX_SPAWNS_PER_SCOPE: usize = 1024;
const MAX_COLLECT_TASKS: usize = 1024;

#[derive(Clone, Debug)]
struct FutureState {
    consumed: bool,
    scope: Option<usize>,
}

#[derive(Clone, Debug)]
struct SpawnState {
    consumed: bool,
    binding: TextRange,
}

#[derive(Clone, Debug)]
struct TaskScope {
    id: usize,
    spawns: BTreeMap<String, SpawnState>,
    spawn_count: usize,
}

#[derive(Clone, Debug)]
struct BorrowState {
    resource: String,
    depth: u16,
}

#[allow(clippy::too_many_lines)]
fn check_resource_async_stream(
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let mut resources: BTreeMap<String, (String, ResourceStatus)> = function
        .parameters
        .iter()
        .filter_map(|parameter| {
            let Type::Named(type_name) = &parameter.ty else {
                return None;
            };
            matches!(
                model.types.get(type_name),
                Some(TypeDefinition::Resource { .. })
            )
            .then(|| {
                (
                    parameter.name.clone(),
                    (type_name.clone(), ResourceStatus::Available),
                )
            })
        })
        .collect();
    let streams: Vec<_> = function
        .parameters
        .iter()
        .filter(|parameter| matches!(&parameter.ty, Type::Generic { name, .. } if name == "Stream"))
        .map(|parameter| parameter.name.clone())
        .collect();
    let mut futures = BTreeMap::<String, FutureState>::new();
    let mut scopes = Vec::<TaskScope>::new();
    let mut next_scope = 0_usize;
    let mut spawned = BTreeSet::<String>::new();
    let mut borrows = BTreeMap::<String, BorrowState>::new();

    for line in &function.body {
        borrows.retain(|_, borrow| borrow.depth <= line.depth);
        track_resource_move(line, &mut resources, facts);
        check_resource_use(line, &mut resources, model, diagnostics, facts);
        borrows.retain(|_, borrow| {
            matches!(
                resources.get(&borrow.resource),
                Some((_, ResourceStatus::Available))
            )
        });
        track_borrow_binding(line, &resources, &mut borrows);
        check_borrow_across_suspension(line, &borrows, diagnostics);
        if line.kind == LineKind::TaskGroup {
            let id = next_scope;
            next_scope += 1;
            scopes.push(TaskScope {
                id,
                spawns: BTreeMap::new(),
                spawn_count: 0,
            });
            if scopes.len() > MAX_TASK_SCOPE_NESTING {
                push_diagnostic(
                    diagnostics,
                    "E5105",
                    "TASK_SCOPE_LIMIT",
                    "task scope limit exceeded: nesting".to_owned(),
                    [("dimension", "nesting")],
                    line.range,
                );
            }
        }
        for token in &line.tokens {
            if token.kind != TokenKind::Spawn {
                continue;
            }
            if let Some(scope) = scopes.last_mut() {
                scope.spawn_count += 1;
                if scope.spawn_count > MAX_SPAWNS_PER_SCOPE {
                    push_diagnostic(
                        diagnostics,
                        "E5105",
                        "TASK_SCOPE_LIMIT",
                        "task scope limit exceeded: spawns".to_owned(),
                        [("dimension", "spawns")],
                        token.range,
                    );
                }
            } else {
                push_diagnostic(
                    diagnostics,
                    "E5104",
                    "TASK_DETACHED",
                    "spawn requires an open task group".to_owned(),
                    [],
                    token.range,
                );
            }
        }
        track_future_binding(line, model, &mut scopes, &mut futures, &mut spawned, facts);
        check_future_await(line, &mut scopes, &mut futures, diagnostics, facts);
        check_collect_tasks(line, &mut scopes, &mut futures, diagnostics, facts);
        check_task_argument_escape(line, &spawned, diagnostics);
        check_stream_operation(line, &streams, diagnostics, facts);
        if line.kind == LineKind::End
            && line
                .tokens
                .get(1)
                .is_some_and(|token| token.kind == TokenKind::Task)
            && let Some(scope) = scopes.pop()
        {
            close_task_scope(&scope, &mut futures, diagnostics);
        }
    }
}

fn close_task_scope(
    scope: &TaskScope,
    futures: &mut BTreeMap<String, FutureState>,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    for (name, spawn) in &scope.spawns {
        if spawn.consumed {
            continue;
        }
        push_diagnostic(
            diagnostics,
            "E5103",
            "TASK_NOT_CONSUMED",
            format!("{name} was not consumed before its task group closed"),
            [("task", name.as_str())],
            spawn.binding,
        );
        futures.remove(name);
    }
}

fn track_borrow_binding(
    line: &Line,
    resources: &BTreeMap<String, (String, ResourceStatus)>,
    borrows: &mut BTreeMap<String, BorrowState>,
) {
    if line.kind != LineKind::Let
        || line
            .tokens
            .get(3)
            .is_none_or(|token| token.kind != TokenKind::Borrow)
    {
        return;
    }
    let Some(source) = line.tokens.get(4) else {
        return;
    };
    if !resources.contains_key(&source.text) {
        return;
    }
    borrows.insert(
        line.tokens[1].text.clone(),
        BorrowState {
            resource: source.text.clone(),
            depth: line.depth,
        },
    );
}

fn check_borrow_across_suspension(
    line: &Line,
    borrows: &BTreeMap<String, BorrowState>,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    for token in &line.tokens {
        if token.kind != TokenKind::Await {
            continue;
        }
        for borrow in borrows.values() {
            push_diagnostic(
                diagnostics,
                "E5003",
                "BORROW_ACROSS_SUSPENSION",
                format!("borrow of {} is live across await", borrow.resource),
                [("resource", borrow.resource.as_str())],
                token.range,
            );
        }
    }
}

fn track_resource_move(
    line: &Line,
    resources: &mut BTreeMap<String, (String, ResourceStatus)>,
    facts: &mut Vec<SemanticFact>,
) {
    if line.kind != LineKind::Let
        || line
            .tokens
            .get(3)
            .is_none_or(|token| token.kind != TokenKind::Move)
    {
        return;
    }
    let Some(source) = line.tokens.get(4).map(|token| token.text.clone()) else {
        return;
    };
    let target = line.tokens[1].text.clone();
    let Some((type_name, status)) = resources.get_mut(&source) else {
        return;
    };
    if matches!(status, ResourceStatus::Available) {
        *status = ResourceStatus::Moved(target.clone());
        let moved_type = type_name.clone();
        resources.insert(
            target.clone(),
            (moved_type.clone(), ResourceStatus::Available),
        );
        facts.push(flow_fact(
            line,
            1,
            SemanticFactKind::ResourceState,
            format!("{source}->moved:{target}"),
            Some(Type::named(moved_type)),
        ));
    }
}

fn check_resource_use(
    line: &Line,
    resources: &mut BTreeMap<String, (String, ResourceStatus)>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let Some((receiver, method, _)) = receiver_method(line) else {
        return;
    };
    let Some((type_name, status)) = resources.get_mut(&receiver) else {
        return;
    };
    match status {
        ResourceStatus::Moved(target) => push_diagnostic(
            diagnostics,
            "E5001",
            "RESOURCE_MOVED",
            format!("{receiver} was moved to {target}"),
            [("resource", receiver.as_str()), ("target", target.as_str())],
            line.range,
        ),
        ResourceStatus::Closed => push_diagnostic(
            diagnostics,
            "E5002",
            "RESOURCE_CLOSED",
            format!("{receiver} was already closed"),
            [("resource", receiver.as_str())],
            line.range,
        ),
        ResourceStatus::Available => {
            let consuming = matches!(
                model.types.get(type_name),
                Some(TypeDefinition::Resource { methods })
                    if methods
                        .get(&method)
                        .is_some_and(|method| method.consumes_self)
            );
            if consuming {
                *status = ResourceStatus::Closed;
                facts.push(flow_fact(
                    line,
                    0,
                    SemanticFactKind::ResourceState,
                    format!("{receiver}->closed"),
                    Some(Type::named(type_name.clone())),
                ));
            }
        }
    }
}

fn track_future_binding(
    line: &Line,
    model: &Model,
    scopes: &mut [TaskScope],
    futures: &mut BTreeMap<String, FutureState>,
    spawned: &mut BTreeSet<String>,
    facts: &mut Vec<SemanticFact>,
) {
    if line.kind != LineKind::Let || line.tokens.len() < 5 {
        return;
    }
    let spawn = line.tokens[3].kind == TokenKind::Spawn;
    let asynchronous = spawn
        || model
            .functions
            .get(&line.tokens[3].text)
            .is_some_and(|function| function.is_async);
    if !asynchronous {
        return;
    }
    let name = line.tokens[1].text.clone();
    let scope = scopes.last().map(|current| current.id);
    futures.insert(
        name.clone(),
        FutureState {
            consumed: false,
            scope,
        },
    );
    if spawn {
        spawned.insert(name.clone());
        if let Some(current) = scopes.last_mut() {
            current.spawns.insert(
                name.clone(),
                SpawnState {
                    consumed: false,
                    binding: line.tokens[1].range,
                },
            );
        }
    }
    facts.push(flow_fact(
        line,
        1,
        SemanticFactKind::AsyncState,
        async_fact_name(scope, &name, "pending"),
        None,
    ));
}

fn async_fact_name(scope: Option<usize>, name: &str, state: &str) -> String {
    scope.map_or_else(
        || format!("{name}->{state}"),
        |id| format!("scope-{id}:{name}->{state}"),
    )
}

fn mark_spawn_consumed(scopes: &mut [TaskScope], name: &str) {
    for scope in scopes.iter_mut().rev() {
        if let Some(spawn) = scope.spawns.get_mut(name) {
            spawn.consumed = true;
            return;
        }
    }
}

fn check_future_await(
    line: &Line,
    scopes: &mut [TaskScope],
    futures: &mut BTreeMap<String, FutureState>,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    for window in line.tokens.windows(2) {
        if window[0].kind != TokenKind::Await {
            continue;
        }
        let name = &window[1].text;
        let Some(state) = futures.get_mut(name) else {
            continue;
        };
        if state.consumed {
            push_diagnostic(
                diagnostics,
                "E5101",
                "FUTURE_CONSUMED",
                format!("{name} was already awaited"),
                [("future", name.as_str())],
                window[1].range,
            );
        } else {
            state.consumed = true;
            let scope = state.scope;
            mark_spawn_consumed(scopes, name);
            let slot = u16::from(line.kind == LineKind::Let);
            facts.push(flow_fact(
                line,
                slot,
                SemanticFactKind::AsyncState,
                async_fact_name(scope, name, "awaited"),
                None,
            ));
        }
    }
}

fn check_collect_tasks(
    line: &Line,
    scopes: &mut [TaskScope],
    futures: &mut BTreeMap<String, FutureState>,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let Some(collect) = line
        .tokens
        .iter()
        .position(|token| token.text == "collect_tasks")
    else {
        return;
    };
    if line
        .tokens
        .get(collect + 1)
        .is_none_or(|token| token.kind != TokenKind::LeftParen)
    {
        return;
    }
    let Some(right) = matching_close(&line.tokens, collect + 1) else {
        return;
    };
    let arguments = split_arguments(&line.tokens[collect + 2..right]);
    let Some(tasks) = arguments
        .first()
        .and_then(|argument| list_elements(argument.tokens))
    else {
        return;
    };
    if tasks.len() > MAX_COLLECT_TASKS {
        push_diagnostic(
            diagnostics,
            "E5105",
            "TASK_SCOPE_LIMIT",
            "task scope limit exceeded: collect".to_owned(),
            [("dimension", "collect")],
            line.range,
        );
    }
    for (index, element) in tasks.iter().enumerate() {
        let [name] = *element else {
            continue;
        };
        let Some(state) = futures.get_mut(&name.text) else {
            continue;
        };
        if state.consumed {
            push_diagnostic(
                diagnostics,
                "E5101",
                "FUTURE_CONSUMED",
                format!("{} was already awaited", name.text),
                [("future", name.text.as_str())],
                name.range,
            );
            continue;
        }
        state.consumed = true;
        let scope = state.scope;
        mark_spawn_consumed(scopes, &name.text);
        let slot = u16::try_from(index + 2).expect("collect list length is bounded");
        facts.push(flow_fact(
            line,
            slot,
            SemanticFactKind::AsyncState,
            async_fact_name(scope, &name.text, "collected"),
            None,
        ));
    }
}

fn list_elements(tokens: &[HirToken]) -> Option<Vec<&[HirToken]>> {
    if tokens.first()?.kind != TokenKind::LeftBracket {
        return None;
    }
    let close = matching_close(tokens, 0)?;
    if close != tokens.len() - 1 {
        return None;
    }
    Some(
        split_top_level(&tokens[1..close], TokenKind::Comma)
            .into_iter()
            .filter(|tokens| !tokens.is_empty())
            .collect(),
    )
}

fn check_task_argument_escape(
    line: &Line,
    spawned: &BTreeSet<String>,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    if spawned.is_empty() {
        return;
    }
    for (index, token) in line.tokens.iter().enumerate() {
        if token.kind != TokenKind::LeftParen {
            continue;
        }
        let Some(callee) = index
            .checked_sub(1)
            .and_then(|previous| line.tokens.get(previous))
            .filter(|token| token.kind == TokenKind::Identifier)
            .map(|token| token.text.as_str())
        else {
            continue;
        };
        if callee == "collect_tasks" {
            continue;
        }
        let Some(right) = matching_close(&line.tokens, index) else {
            continue;
        };
        for argument in split_arguments(&line.tokens[index + 1..right]) {
            if argument.tokens.len() == 1 && spawned.contains(&argument.tokens[0].text) {
                push_diagnostic(
                    diagnostics,
                    "E5102",
                    "TASK_ESCAPES_SCOPE",
                    "task cannot leave its task group".to_owned(),
                    [],
                    line.range,
                );
            }
        }
    }
}

fn task_escapes_via_return(tokens: &[HirToken], depth: u16, spawned: &BTreeSet<String>) -> bool {
    tokens.iter().any(|token| spawned.contains(&token.text))
        || (depth > 0 && tokens.iter().any(|token| token.kind == TokenKind::Spawn))
}

fn is_task_type(ty: &Type) -> bool {
    matches!(ty, Type::Generic { name, arguments } if name == "Task" && arguments.len() == 1)
}

fn check_stream_operation(
    line: &Line,
    streams: &[String],
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let Some((receiver, operation, receiver_index)) = receiver_method(line) else {
        return;
    };
    if !streams.contains(&receiver) || !matches!(operation.as_str(), "next" | "collect") {
        return;
    }
    facts.push(flow_fact(
        line,
        0,
        SemanticFactKind::StreamOperation,
        format!("{receiver}.{operation}"),
        None,
    ));
    let awaited = line.tokens[..receiver_index]
        .iter()
        .any(|token| token.kind == TokenKind::Await);
    if operation == "next" && !awaited {
        let full_name = format!("{receiver}.{operation}");
        push_diagnostic(
            diagnostics,
            "E5202",
            "ASYNC_VALUE_REQUIRES_AWAIT",
            format!("{full_name} returns a Future; await it"),
            [("operation", full_name.as_str())],
            line.range,
        );
    } else if operation == "collect" && !line.tokens.iter().any(|token| token.text == "limit") {
        push_diagnostic(
            diagnostics,
            "E5201",
            "UNBOUNDED_STREAM_COLLECT",
            "stream collection requires an explicit limit".to_owned(),
            [],
            line.range,
        );
    }
}

fn receiver_method(line: &Line) -> Option<(String, String, usize)> {
    let dot = line
        .tokens
        .iter()
        .position(|token| token.kind == TokenKind::Dot)?;
    let receiver_index = dot.checked_sub(1)?;
    Some((
        line.tokens.get(receiver_index)?.text.clone(),
        line.tokens.get(dot + 1)?.text.clone(),
        receiver_index,
    ))
}

fn flow_fact(
    line: &Line,
    slot: u16,
    kind: SemanticFactKind,
    name: String,
    ty: Option<Type>,
) -> SemanticFact {
    SemanticFact {
        id: FactId {
            node: line.id,
            slot,
        },
        kind,
        name,
        ty,
        range: line.range,
    }
}

#[derive(Clone, Debug)]
struct BoundaryCall {
    effect: String,
    capability: String,
    component: bool,
    node: HirId,
    range: TextRange,
}

fn check_boundary_semantics(
    function: &FunctionDefinition,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    push_boundary_facts(function, facts);
    check_component_version(function, diagnostics);

    let has_boundary = function
        .body
        .iter()
        .any(|line| matches!(line.kind, LineKind::Effects | LineKind::Capabilities));
    if !has_boundary {
        return;
    }
    for call in function.body.iter().filter_map(boundary_call) {
        check_boundary_call(function, &call, diagnostics, facts);
    }
}

fn push_boundary_facts(function: &FunctionDefinition, facts: &mut Vec<SemanticFact>) {
    for (name, node, range) in &function.declared_effects {
        facts.push(SemanticFact {
            id: FactId {
                node: *node,
                slot: 0,
            },
            kind: SemanticFactKind::Effect,
            name: format!("{}.effect.{name}", function.name),
            ty: None,
            range: *range,
        });
    }
    for (name, node, range) in &function.declared_capabilities {
        facts.push(SemanticFact {
            id: FactId {
                node: *node,
                slot: 0,
            },
            kind: SemanticFactKind::Capability,
            name: format!("{}.capability.{name}", function.name),
            ty: None,
            range: *range,
        });
    }
}

fn check_component_version(
    function: &FunctionDefinition,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    if function
        .parameters
        .iter()
        .any(|parameter| matches!(&parameter.ty, Type::Generic { name, .. } if name == "Component"))
        && let Some(range) = function.component_version_range
    {
        push_diagnostic(
            diagnostics,
            "E6001",
            "COMPONENT_VERSION_IN_TYPE",
            "component version is package metadata, not a type argument".to_owned(),
            [],
            range,
        );
    }
}

fn check_boundary_call(
    function: &FunctionDefinition,
    call: &BoundaryCall,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    if call.component {
        facts.push(SemanticFact {
            id: FactId {
                node: call.node,
                slot: 0,
            },
            kind: SemanticFactKind::ComponentCall,
            name: format!("{}.component-call", function.name),
            ty: Some(function.returns.clone()),
            range: call.range,
        });
        if !matches!(&function.returns, Type::Generic { name, .. } if name == "ComponentCall") {
            push_diagnostic(
                diagnostics,
                "E6002",
                "COMPONENT_FAILURE_COLLAPSE",
                "ComponentCall cannot be returned as a domain Result".to_owned(),
                [],
                call.range,
            );
        }
    }
    if !function
        .declared_capabilities
        .iter()
        .any(|(name, _, _)| name == &call.capability)
    {
        push_diagnostic(
            diagnostics,
            "E4001",
            "UNDECLARED_CAPABILITY",
            format!(
                "capability {} is not declared at this boundary",
                call.capability
            ),
            [("capability", call.capability.as_str())],
            call.range,
        );
    }
    let explicitly_pure = function.body.iter().any(|line| {
        line.kind == LineKind::Effects
            && line
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::None)
    });
    if explicitly_pure {
        push_diagnostic(
            diagnostics,
            "E4002",
            "UNDECLARED_EFFECT",
            format!("missing declared effect: {}", call.effect),
            [("effect", call.effect.as_str())],
            call.range,
        );
    }
}

fn boundary_call(line: &Line) -> Option<BoundaryCall> {
    let dot = line
        .tokens
        .iter()
        .position(|token| token.kind == TokenKind::Dot)?;
    if !line
        .tokens
        .iter()
        .any(|token| token.kind == TokenKind::LeftParen)
    {
        return None;
    }
    let receiver = line.tokens.get(dot.checked_sub(1)?)?.text.clone();
    let method = line.tokens.get(dot + 1)?.text.clone();
    let component = line
        .tokens
        .iter()
        .any(|token| token.kind == TokenKind::Call);
    Some(BoundaryCall {
        effect: if component {
            "component.call".to_owned()
        } else {
            format!("{receiver}.{method}")
        },
        capability: receiver,
        component,
        node: line.id,
        range: line.range,
    })
}

/// RFC-0046 D2: `let x = expr?` typing. The source must be
/// `Result[U, NumericError]` and the enclosing function must return
/// `Result[T, NumericError]`: the lowering desugars to a match whose error
/// arm forwards the `NumericError` payload verbatim, so both error types are
/// frozen to `NumericError` (E3101 family, same identity as `infer_try`).
fn infer_propagate(
    tokens: &[HirToken],
    target_return: &Type,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let source = infer_expression(tokens, locals, model, diagnostics);
    let Some((ok_type, source_error)) = result_parts(&source.ty) else {
        return unknown(source.range);
    };
    let numeric_error = Type::named("NumericError");
    if !types_compatible(source_error, &numeric_error) {
        let source_name = source_error.to_string();
        push_diagnostic(
            diagnostics,
            "E3101",
            "ERROR_TYPE_MISMATCH",
            format!("cannot propagate {source_name} as NumericError"),
            [
                ("source_error", source_name.as_str()),
                ("target_error", "NumericError"),
            ],
            source.range,
        );
    }
    let Some((target_ok, target_error)) = result_parts(target_return) else {
        push_diagnostic(
            diagnostics,
            "E3101",
            "ERROR_TYPE_MISMATCH",
            "? is only valid in a function returning Result[T, NumericError]".to_owned(),
            [
                ("expected", "Result[T, NumericError]"),
                ("found", "non-Result return"),
            ],
            source.range,
        );
        return unknown(source.range);
    };
    if !types_compatible(target_error, &numeric_error) {
        let target_name = target_error.to_string();
        push_diagnostic(
            diagnostics,
            "E3101",
            "ERROR_TYPE_MISMATCH",
            format!("cannot propagate NumericError as {target_name}"),
            [
                ("source_error", "NumericError"),
                ("target_error", target_name.as_str()),
            ],
            source.range,
        );
    }
    if !types_compatible(ok_type, target_ok) {
        let source_name = ok_type.to_string();
        let target_name = target_ok.to_string();
        push_diagnostic(
            diagnostics,
            "E3101",
            "ERROR_TYPE_MISMATCH",
            format!("cannot propagate {source_name} as {target_name}"),
            [
                ("source_ok", source_name.as_str()),
                ("target_ok", target_name.as_str()),
            ],
            source.range,
        );
    }
    Value {
        ty: ok_type.clone(),
        range: source.range,
        integer: None,
    }
}

fn infer_try(
    tokens: &[HirToken],
    target_return: &Type,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let source = infer_expression(tokens, locals, model, diagnostics);
    let Some((ok_type, source_error)) = result_parts(&source.ty) else {
        return unknown(source.range);
    };
    if let Some((_, target_error)) = result_parts(target_return)
        && !types_compatible(source_error, target_error)
    {
        let source_name = source_error.to_string();
        let target_name = target_error.to_string();
        push_diagnostic(
            diagnostics,
            "E3101",
            "ERROR_TYPE_MISMATCH",
            format!("cannot propagate {source_name} as {target_name}"),
            [
                ("source_error", source_name.as_str()),
                ("target_error", target_name.as_str()),
            ],
            source.range,
        );
    }
    Value {
        ty: ok_type.clone(),
        range: source.range,
        integer: None,
    }
}

#[derive(Clone, Debug)]
struct MatchPattern {
    key: String,
    variants: Vec<(String, String)>,
    wildcard: bool,
    result_ok: bool,
    result_error: bool,
    range: TextRange,
}

fn check_match(
    line_index: usize,
    function: &FunctionDefinition,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
    facts: &mut Vec<SemanticFact>,
) {
    let line = &function.body[line_index];
    let patterns = collect_match_patterns(line_index, function);
    if patterns.is_empty() {
        return;
    }
    facts.push(SemanticFact {
        id: FactId {
            node: line.id,
            slot: 0,
        },
        kind: SemanticFactKind::Match,
        name: format!("{}.match", function.name),
        ty: None,
        range: line.range,
    });

    let Some(covered) = unique_covered_patterns(&patterns, diagnostics) else {
        return;
    };

    let is_error_map = function.body[..line_index].iter().any(|candidate| {
        candidate.depth < line.depth
            && candidate
                .tokens
                .iter()
                .any(|token| token.text == "map_error")
    });
    let is_result_match = patterns
        .iter()
        .any(|pattern| pattern.result_ok || pattern.result_error);
    let enum_names = ordered_enum_names(&patterns);
    let wildcard = patterns.iter().find(|pattern| pattern.wildcard);

    if is_error_map {
        check_error_map(line, &patterns, &enum_names, wildcard, model, diagnostics);
        return;
    }

    if let Some(wildcard) = wildcard {
        check_sealed_wildcard(wildcard, &patterns, &enum_names, model, diagnostics);
        return;
    }

    if is_result_match {
        return;
    }
    check_match_exhaustiveness(line, &patterns, &enum_names, &covered, model, diagnostics);
}

fn check_error_map(
    line: &Line,
    patterns: &[MatchPattern],
    enum_names: &[String],
    wildcard: Option<&MatchPattern>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    let Some(enum_name) = enum_names.first() else {
        return;
    };
    let missing = missing_enum_variants(enum_name, patterns, model);
    if let Some(wildcard) = wildcard {
        if !missing.is_empty() {
            push_list_diagnostic(
                diagnostics,
                "E3103",
                "SEALED_ERROR_WILDCARD",
                format!(
                    "wildcard hides sealed error variants: {}",
                    missing.join(", ")
                ),
                "hidden",
                missing,
                wildcard.range,
            );
        }
    } else if !missing.is_empty() {
        push_list_diagnostic(
            diagnostics,
            "E3102",
            "INCOMPLETE_ERROR_MAP",
            format!("missing error mapping: {}", missing.join(", ")),
            "missing",
            missing,
            line.range,
        );
    }
}

fn check_sealed_wildcard(
    wildcard: &MatchPattern,
    patterns: &[MatchPattern],
    enum_names: &[String],
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    if enum_names.len() > 1 {
        push_diagnostic(
            diagnostics,
            "E3002",
            "SEALED_MATCH_WILDCARD",
            "wildcard hides sealed state/event combinations".to_owned(),
            [("hidden", "state/event combinations")],
            wildcard.range,
        );
    } else if let Some(enum_name) = enum_names.first() {
        let missing = missing_enum_variants(enum_name, patterns, model);
        if !missing.is_empty() {
            let hidden = format!("variants: {}", missing.join(", "));
            push_diagnostic(
                diagnostics,
                "E3002",
                "SEALED_MATCH_WILDCARD",
                format!("wildcard hides sealed {hidden}"),
                [("hidden", hidden.as_str())],
                wildcard.range,
            );
        }
    }
}

fn check_match_exhaustiveness(
    line: &Line,
    patterns: &[MatchPattern],
    enum_names: &[String],
    covered: &[String],
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    // Bool-literal matches have no enum variants: require both literals.
    if enum_names.is_empty() && patterns.iter().all(|pattern| pattern.variants.is_empty()) {
        let missing: Vec<String> = ["true", "false"]
            .into_iter()
            .filter(|literal| !covered.iter().any(|case| *case == *literal))
            .map(str::to_owned)
            .collect();
        if !missing.is_empty() {
            push_list_diagnostic(
                diagnostics,
                "E3001",
                "NON_EXHAUSTIVE_MATCH",
                format!("missing case: {}", missing.join(", ")),
                "missing",
                missing,
                line.range,
            );
        }
        return;
    }
    let missing = if enum_names.len() == 1 {
        missing_enum_variants(&enum_names[0], patterns, model)
    } else {
        product_patterns(enum_names, model)
            .into_iter()
            .filter(|pattern| !covered.contains(pattern))
            .collect()
    };
    if !missing.is_empty() {
        push_list_diagnostic(
            diagnostics,
            "E3001",
            "NON_EXHAUSTIVE_MATCH",
            format!("missing case: {}", missing.join(", ")),
            "missing",
            missing,
            line.range,
        );
    }
}

fn collect_match_patterns(line_index: usize, function: &FunctionDefinition) -> Vec<MatchPattern> {
    let line = &function.body[line_index];
    let mut patterns = Vec::new();
    for candidate in function.body.iter().skip(line_index + 1) {
        if candidate.kind == LineKind::End
            && candidate.depth == line.depth
            && candidate
                .tokens
                .get(1)
                .is_some_and(|token| token.kind == TokenKind::Match)
        {
            break;
        }
        if candidate.kind == LineKind::MatchArm && candidate.depth == line.depth + 1 {
            patterns.push(parse_match_pattern(candidate));
        }
    }
    patterns
}

fn unique_covered_patterns(
    patterns: &[MatchPattern],
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Option<Vec<String>> {
    let mut covered = Vec::new();
    for pattern in patterns {
        if !pattern.wildcard && covered.contains(&pattern.key) {
            let case_name = pattern
                .variants
                .last()
                .map_or(pattern.key.as_str(), |(_, variant)| variant.as_str());
            push_diagnostic(
                diagnostics,
                "E3003",
                "UNREACHABLE_MATCH_ARM",
                format!("match arm is already covered: {case_name}"),
                [("case", case_name)],
                pattern.range,
            );
            return None;
        }
        if !pattern.wildcard {
            covered.push(pattern.key.clone());
        }
    }
    Some(covered)
}

fn parse_match_pattern(line: &Line) -> MatchPattern {
    let end = line
        .tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::Colon)
        .unwrap_or(line.tokens.len());
    let tokens = &line.tokens[1..end];
    let wildcard = tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Else);
    let result_ok = tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::OkKeyword);
    let result_error = tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::ErrorKeyword);
    let variants: Vec<_> = tokens
        .windows(3)
        .filter(|window| window[1].kind == TokenKind::Dot)
        .map(|window| (window[0].text.clone(), window[2].text.clone()))
        .collect();
    let key = if result_ok {
        "ok".to_owned()
    } else if result_error {
        format!(
            "error:{}",
            variants
                .last()
                .map_or("<unknown>", |(_, variant)| variant.as_str())
        )
    } else if variants.is_empty()
        && let Some(token) = tokens
            .iter()
            .find(|token| matches!(token.kind, TokenKind::True | TokenKind::False))
    {
        token.text.clone()
    } else {
        variants
            .iter()
            .map(|(enum_name, variant)| format!("{enum_name}.{variant}"))
            .collect::<Vec<_>>()
            .join("|")
    };
    MatchPattern {
        key,
        variants,
        wildcard,
        result_ok,
        result_error,
        range: line.range,
    }
}

fn ordered_enum_names(patterns: &[MatchPattern]) -> Vec<String> {
    let mut names = Vec::new();
    for pattern in patterns {
        for (enum_name, _) in &pattern.variants {
            if !names.contains(enum_name) {
                names.push(enum_name.clone());
            }
        }
    }
    names
}

fn missing_enum_variants(enum_name: &str, patterns: &[MatchPattern], model: &Model) -> Vec<String> {
    let Some(TypeDefinition::Enum { variants }) = model.types.get(enum_name) else {
        return Vec::new();
    };
    variants
        .iter()
        .filter(|variant| {
            !patterns.iter().any(|pattern| {
                pattern
                    .variants
                    .iter()
                    .any(|(name, covered)| name == enum_name && covered == &variant.name)
            })
        })
        .map(|variant| variant.name.clone())
        .collect()
}

fn product_patterns(enum_names: &[String], model: &Model) -> Vec<String> {
    let mut products = vec![String::new()];
    for enum_name in enum_names {
        let Some(TypeDefinition::Enum { variants }) = model.types.get(enum_name) else {
            return Vec::new();
        };
        products = products
            .into_iter()
            .flat_map(|prefix| {
                variants.iter().map(move |variant| {
                    let item = format!("{enum_name}.{}", variant.name);
                    if prefix.is_empty() {
                        item
                    } else {
                        format!("{prefix}|{item}")
                    }
                })
            })
            .collect();
    }
    products
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
    if tokens[0].kind == TokenKind::Await {
        let awaited = infer_expression(&tokens[1..], locals, model, diagnostics);
        if let Type::Generic { name, arguments } = awaited.ty
            && matches!(name.as_str(), "Future" | "Task")
            && arguments.len() == 1
        {
            return Value {
                ty: arguments[0].clone(),
                range,
                integer: None,
            };
        }
        return unknown(range);
    }
    if tokens[0].kind == TokenKind::Spawn {
        let spawned = infer_expression(&tokens[1..], locals, model, diagnostics);
        let result = match spawned.ty {
            Type::Generic { name, arguments } if name == "Future" && arguments.len() == 1 => {
                arguments[0].clone()
            }
            _ => Type::Unknown,
        };
        return Value {
            ty: Type::Generic {
                name: "Task".to_owned(),
                arguments: vec![result],
            },
            range,
            integer: None,
        };
    }
    if let [minus, integer] = tokens
        && minus.kind == TokenKind::Minus
        && integer.kind == TokenKind::Integer
    {
        return Value {
            ty: Type::named("Int"),
            range,
            integer: integer
                .text
                .parse::<i128>()
                .ok()
                .and_then(i128::checked_neg),
        };
    }
    if let Some(index) = top_level_any(tokens, &[TokenKind::EqualEqual, TokenKind::LessEqual]) {
        let left = infer_expression(&tokens[..index], locals, model, diagnostics);
        let right = infer_expression(&tokens[index + 1..], locals, model, diagnostics);
        // RFC-0047 D3 (STEP-0273): record equality/ordering is refused in
        // v0 (no consumer; field-wise comparison composes explicitly).
        // One diagnostic per comparison, on the first record-typed side.
        let record_side = [&left, &right].into_iter().find(|side| {
            matches!(&side.ty, Type::Named(name) if matches!(model.types.get(name), Some(TypeDefinition::Record { .. })))
        });
        if let Some(side) = record_side
            && let Type::Named(name) = &side.ty
        {
            push_diagnostic(
                diagnostics,
                "E2024",
                "RECORD_COMPARISON",
                format!(
                    "records do not support {}; compare fields explicitly",
                    if tokens[index].kind == TokenKind::EqualEqual {
                        "=="
                    } else {
                        "<="
                    }
                ),
                [("type", name.as_str())],
                side.range,
            );
        }
        return Value {
            ty: Type::named("Bool"),
            range,
            integer: None,
        };
    }
    if let Some(index) = top_level_kind(tokens, TokenKind::Plus) {
        let left = infer_expression(&tokens[..index], locals, model, diagnostics);
        let right = infer_expression(&tokens[index + 1..], locals, model, diagnostics);
        if left.ty == right.ty
            && matches!(&left.ty, Type::Named(name) if matches!(name.as_str(), "I64" | "U64"))
        {
            let expected = format!("{}.checked_add", left.ty);
            push_diagnostic(
                diagnostics,
                "E2001",
                "TYPE_MISMATCH",
                format!("expected {expected}, found unchecked +"),
                [("expected", expected.as_str()), ("found", "unchecked +")],
                range,
            );
            return unknown(range);
        }
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
        if enum_variant(model, &tokens[0].text, &tokens[2].text).is_some() {
            return Value {
                ty: Type::named(tokens[0].text.clone()),
                range,
                integer: None,
            };
        }
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
        // RFC-0047 D7 (STEP-0271): dot access on a record-typed value with
        // an unknown field is a typed refusal (E2011), never a silent
        // unknown-type hole.
        if let Type::Named(type_name) = &base.ty
            && let Some(TypeDefinition::Record { .. }) = model.types.get(type_name)
        {
            push_diagnostic(
                diagnostics,
                "E2011",
                "UNKNOWN_FIELD",
                format!("unknown field: {}", tokens[2].text),
                [("field", tokens[2].text.as_str())],
                tokens[2].range,
            );
            return unknown(range);
        }
    }
    unknown(range)
}

/// RFC-0039 §2.2 (STEP-0144): resolves `receiver.method` when the receiver
/// local is capability- or resource-typed, against the declared method
/// surface. A Revision-headed capability method called without its head is
/// the revision-guard surface: E7001 owns that diagnosis
/// (`check_revision_calls`), so argument checking stands down (REV-101).
/// Returns `Some` when the call resolved (with any diagnostics), `None`
/// when the receiver is not a capability/resource.
fn infer_capability_or_resource_method(
    receiver_name: &str,
    method: &str,
    values: &[Value],
    range: TextRange,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Option<Value> {
    let receiver = locals.get(receiver_name)?;
    let Type::Named(type_name) = receiver else {
        return None;
    };
    if let Some(cap_method) = model
        .capabilities
        .get(type_name)
        .and_then(|methods| methods.get(method))
    {
        // Capability-typed parameters stay `Named` in semantics; the
        // capability table is the authority for the method surface.
        let revision_managed = cap_method.parameters.first().is_some_and(
            |parameter| matches!(&parameter.ty, Type::Named(name) if name == "Revision"),
        ) && values.len() + 1 == cap_method.parameters.len();
        if !revision_managed {
            for (parameter, value) in cap_method.parameters.iter().zip(values) {
                require_type(&parameter.ty, value, diagnostics);
            }
        }
        return Some(Value {
            ty: cap_method.returns.clone(),
            range,
            integer: None,
        });
    }
    if let Some(TypeDefinition::Resource { methods }) = model.types.get(type_name)
        && let Some(resource_method) = methods.get(method)
    {
        // The affine `using`/move discipline is enforced by the
        // resource-state facts; here only the method surface is resolved
        // and type-checked.
        for (parameter, value) in resource_method.parameters.iter().zip(values) {
            require_type(&parameter.ty, value, diagnostics);
        }
        return Some(Value {
            ty: resource_method.returns.clone(),
            range,
            integer: None,
        });
    }
    None
}

/// RFC-0039 section 2.2 (STEP-0144): the check-time value of a qualified
/// imported call, with argument types enforced against the module surface.
fn imported_call_value(
    imported: &ImportedFunction,
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    for (parameter, value) in imported.parameters.iter().zip(values) {
        require_type(parameter, value, diagnostics);
    }
    Value {
        ty: if imported.is_async {
            Type::Generic {
                name: "Future".to_owned(),
                arguments: vec![imported.returns.clone()],
            }
        } else {
            imported.returns.clone()
        },
        range,
        integer: None,
    }
}

/// RFC-0039 section 2.2 (STEP-0144): the value of an enum variant call,
/// with payload types enforced.
fn variant_call_value(
    enum_name: &str,
    variant: &VariantDefinition,
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    for (expected, value) in variant.payload.iter().zip(values) {
        require_type(expected, value, diagnostics);
    }
    Value {
        ty: Type::named(enum_name),
        range,
        integer: None,
    }
}

/// RFC-0039 section 2.2 (STEP-0144): checks a newtype/record constructor
/// call against its declared shape; enum/resource names construct nothing.
fn type_constructor_call(
    callee: &str,
    definition: &TypeDefinition,
    arguments: &[Argument<'_>],
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    match definition {
        TypeDefinition::Newtype { base } => {
            if let Some(value) = values.first() {
                require_type(base, value, diagnostics);
            }
        }
        TypeDefinition::Record { fields, invariant } => {
            check_record_constructor(
                callee,
                fields,
                invariant.as_ref(),
                arguments,
                values,
                range,
                diagnostics,
            );
        }
        TypeDefinition::Enum { .. } | TypeDefinition::Resource { .. } => {}
    }
}

/// RFC-0039 section 2.2 (STEP-0144): the `Float64.from_int` conversion
/// call, extracted from `infer_call` for clarity.
fn float_from_int_call(
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    if let Some(value) = values.first() {
        require_type(&Type::named("Int"), value, diagnostics);
    }
    Value {
        ty: Type::named("Float64"),
        range,
        integer: None,
    }
}

#[allow(clippy::too_many_lines)]
fn infer_call(
    tokens: &[HirToken],
    left: usize,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let range = token_range(tokens);
    // The M5 component-call prefix (`call receiver.method(args)`) lexes as
    // a keyword; it is not part of the callee path.
    let mut callee = tokens[..left]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<String>();
    if tokens[0].kind == TokenKind::Call
        && let Some(stripped) = callee.strip_prefix("call").map(str::to_owned)
    {
        callee = stripped;
    }
    let arguments = split_arguments(&tokens[left + 1..tokens.len() - 1]);
    let values: Vec<_> = arguments
        .iter()
        .map(|argument| infer_expression(argument.tokens, locals, model, diagnostics))
        .collect();

    if let Some(result) = infer_result_constructor(&callee, &values, range) {
        return result;
    }

    if let Some(value) = infer_fixed_width_call(&callee, &values, range, diagnostics) {
        return value;
    }

    if let Some(value) = infer_stdlib_call(&callee, &values, range, diagnostics) {
        return value;
    }

    // RFC-0047 D5 (STEP-0274): `List[record]` monomorphs. Only record
    // types whose fields are all flat `I64`/`U64` get an executable
    // typing here; anything else keeps its typed unknown-callee refusal
    // (no check/build gap, mirroring `flat_ir_types`).
    if let Some((operation, record)) = parse_record_list_suffix(&callee)
        && let Some(TypeDefinition::Record { fields, .. }) = model.types.get(record)
        && fields
            .values()
            .all(|field| matches!(&field.ty, Type::Named(name) if name == "I64" || name == "U64"))
    {
        return infer_record_list_call(operation, record, &values, range, diagnostics);
    }

    if callee == "collect_tasks" {
        return infer_collect_tasks(&arguments, &values, range, locals, model, diagnostics);
    }

    if callee == "Float64.from_int" {
        return float_from_int_call(&values, range, diagnostics);
    }
    if let Some(function) = model.functions.get(&callee) {
        for (parameter, value) in function.parameters.iter().zip(&values) {
            require_type(&parameter.ty, value, diagnostics);
        }
        return Value {
            ty: if function.is_async {
                Type::Generic {
                    name: "Future".to_owned(),
                    arguments: vec![function.returns.clone()],
                }
            } else {
                function.returns.clone()
            },
            range,
            integer: None,
        };
    }
    if let Some(imported) = model.imported_functions.get(&callee) {
        return imported_call_value(imported, &values, range, diagnostics);
    }
    if let Some((enum_name, variant_name)) = callee.split_once('.')
        && let Some(variant) = enum_variant(model, enum_name, variant_name)
    {
        return variant_call_value(enum_name, variant, &values, range, diagnostics);
    }
    if let Some(definition) = model.types.get(&callee) {
        type_constructor_call(&callee, definition, &arguments, &values, range, diagnostics);
        return Value {
            ty: Type::named(callee),
            range,
            integer: None,
        };
    }
    // RFC-0039 section 2.2 (STEP-0144): capability and resource method
    // calls resolve against the declared surface instead of falling
    // through as silent Unknowns.
    if let Some((receiver_name, method)) = callee.split_once('.')
        && let Some(resolved) = infer_capability_or_resource_method(
            receiver_name,
            method,
            &values,
            range,
            locals,
            model,
            diagnostics,
        )
    {
        return resolved;
    }
    // Declared tolerance (until the user-WIT slice, RFC-0039 section 2.4):
    // the M5 component-call surface (call receiver.method(...) on
    // Component[Interface] values) and the M9 stream-helper candidates
    // (stream.collect/stream.next on Stream values) have no check-time
    // model yet and keep the historical check-pass/build-refuse behavior.
    if let Some((receiver_name, _)) = callee.split_once('.')
        && matches!(
            locals.get(receiver_name),
            Some(Type::Generic { name, .. }) if matches!(name.as_str(), "Component" | "Stream")
        )
    {
        return unknown(range);
    }
    // RFC-0039 section 2.2 (STEP-0144): check-time call-target resolution.
    // An unbound callee is a typed diagnostic with the callee identity,
    // never a silent Unknown that defers to the build backend.
    push_diagnostic(
        diagnostics,
        "E2031",
        "UNRESOLVED_CALL_TARGET",
        format!("unresolved call target {callee}"),
        [("callee", callee.as_str())],
        range,
    );
    unknown(range)
}

fn infer_collect_tasks(
    arguments: &[Argument<'_>],
    values: &[Value],
    range: TextRange,
    locals: &BTreeMap<String, Type>,
    model: &Model,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let list_of = |element: Type| Type::Generic {
        name: "List".to_owned(),
        arguments: vec![element],
    };
    if arguments.len() != 2 {
        let found = format!("{} arguments", arguments.len());
        push_diagnostic(
            diagnostics,
            "E2001",
            "TYPE_MISMATCH",
            format!("expected 2 arguments, found {found}"),
            [("expected", "2 arguments"), ("found", found.as_str())],
            range,
        );
        return unknown(range);
    }
    // `order` only accepts the prelude TaskOrder literal `input` (RFC-0036 §4).
    let order = &arguments[1];
    let is_input = order.name.as_deref() == Some("order")
        && order.tokens.len() == 1
        && order.tokens[0].kind == TokenKind::Identifier
        && order.tokens[0].text == "input";
    if !is_input {
        require_type(&Type::named("TaskOrder"), &values[1], diagnostics);
    }
    let mut element = Type::Unknown;
    if let Some(elements) = list_elements(arguments[0].tokens) {
        for element_tokens in elements {
            let value = infer_expression(element_tokens, locals, model, diagnostics);
            let expected = Type::Generic {
                name: "Task".to_owned(),
                arguments: vec![element.clone()],
            };
            require_type(&expected, &value, diagnostics);
            if element.is_unknown()
                && let Type::Generic { name, arguments } = &value.ty
                && name == "Task"
                && arguments.len() == 1
            {
                element = arguments[0].clone();
            }
        }
    } else {
        let expected = list_of(Type::Generic {
            name: "Task".to_owned(),
            arguments: vec![Type::Unknown],
        });
        require_type(&expected, &values[0], diagnostics);
        if let Type::Generic { name, arguments } = &values[0].ty
            && name == "List"
            && arguments.len() == 1
            && let Type::Generic {
                name: inner,
                arguments: inner_arguments,
            } = &arguments[0]
            && inner == "Task"
            && inner_arguments.len() == 1
        {
            element = inner_arguments[0].clone();
        }
    }
    Value {
        ty: list_of(element),
        range,
        integer: None,
    }
}

fn infer_fixed_width_call(
    callee: &str,
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Option<Value> {
    if matches!(callee, "I64.literal" | "U64.literal") {
        if values.len() != 1 {
            let found = format!("{} arguments", values.len());
            push_diagnostic(
                diagnostics,
                "E2001",
                "TYPE_MISMATCH",
                format!("expected 1 argument, found {found}"),
                [("expected", "1 argument"), ("found", found.as_str())],
                range,
            );
            return Some(unknown(range));
        }
        let value = &values[0];
        require_type(&Type::named("Int"), value, diagnostics);
        let target = callee.trim_end_matches(".literal");
        let in_range = match (target, value.integer) {
            ("I64", Some(integer)) => i64::try_from(integer).is_ok(),
            ("U64", Some(integer)) => u64::try_from(integer).is_ok(),
            _ => false,
        };
        if !in_range {
            let expected = format!("{target} literal in range");
            let found = value.integer.map_or_else(
                || "non-literal Int".to_owned(),
                |integer| integer.to_string(),
            );
            push_diagnostic(
                diagnostics,
                "E2001",
                "TYPE_MISMATCH",
                format!("expected {expected}, found {found}"),
                [("expected", expected.as_str()), ("found", found.as_str())],
                value.range,
            );
        }
        return Some(Value {
            ty: Type::named(target),
            range,
            integer: value.integer,
        });
    }

    let (target, operation) = callee.split_once('.')?;
    if !matches!(target, "I64" | "U64")
        || !matches!(
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
        return None;
    }
    if values.len() != 2 {
        let found = format!("{} arguments", values.len());
        push_diagnostic(
            diagnostics,
            "E2001",
            "TYPE_MISMATCH",
            format!("expected 2 arguments, found {found}"),
            [("expected", "2 arguments"), ("found", found.as_str())],
            range,
        );
        return Some(unknown(range));
    }
    let fixed = Type::named(target);
    for value in values {
        require_type(&fixed, value, diagnostics);
    }
    let ty = if matches!(operation, "equal" | "less_than") {
        Type::named("Bool")
    } else if matches!(operation, "bit_and" | "bit_or" | "bit_xor" | "shl" | "shr") {
        // Bit operations are infallible and stay in the operand type
        // (STEP-0132).
        fixed
    } else {
        Type::Generic {
            name: "Result".to_owned(),
            arguments: vec![fixed, Type::named("NumericError")],
        }
    };
    Some(Value {
        ty,
        range,
        integer: None,
    })
}

/// STEP-0131 canonical `sico.map.*[K,V]` / `sico.set.*[K]` collection
/// intrinsics. The suffix grammar mirrors `sico_ir::collection_intrinsic`
/// (the authoritative compiler-side parser); `sico-semantics` lives in the
/// language module and must not depend on the compiler module, so the closed
/// grammar is intentionally duplicated here — keep the two in sync. An
/// instantiation that parses but is outside the v0 executable surface
/// (non-fixed-width `map.get` values, non-`Text` `keys`/`to_list` key
/// elements) returns `None` so the call falls through to the unknown-callee
/// diagnostic instead of an undeclared check/build gap.
/// RFC-0047 D5 (STEP-0274): parses `sico.list.{empty,length,get,append}[Ident]`
/// into its operation and record element. The five scalar element names are
/// excluded so they stay on the closed scalar grammar; anything else returns
/// `None` and keeps its typed unknown-callee refusal.
fn parse_record_list_suffix(callee: &str) -> Option<(&str, &str)> {
    const OPERATIONS: &[&str] = &["empty", "length", "get", "append"];
    let path = callee.strip_prefix("sico.list.")?;
    let (operation, suffix) = path.split_once('[')?;
    if !OPERATIONS.contains(&operation) || !suffix.ends_with(']') {
        return None;
    }
    let record = &suffix[..suffix.len() - 1];
    let mut chars = record.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        || matches!(record, "Text" | "Bytes" | "Bool" | "I64" | "U64")
    {
        return None;
    }
    Some((operation, record))
}

/// RFC-0047 D5 (STEP-0274): executable typing for one
/// `sico.list.{empty,length,get,append}[Record]` monomorph. The caller has
/// already established that the record exists and is flat (`I64`/`U64`
/// fields only), so every operand shape materializes here.
fn infer_record_list_call(
    operation: &str,
    record: &str,
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Value {
    let named = |name: &str| Type::Named(name.to_owned());
    let element = named(record);
    let list_of = Type::Generic {
        name: "List".to_owned(),
        arguments: vec![element.clone()],
    };
    let numeric_result = Type::Generic {
        name: "Result".to_owned(),
        arguments: vec![element.clone(), named("NumericError")],
    };
    let (parameters, result) = match operation {
        "empty" => (Vec::new(), list_of),
        "length" => (vec![list_of], named("U64")),
        "get" => (vec![list_of.clone(), named("U64")], numeric_result),
        "append" => (vec![list_of.clone(), element], list_of),
        _ => unreachable!("parse_record_list_suffix only yields the four list operations"),
    };
    if parameters.len() != values.len() {
        let expected = format!("{} arguments", parameters.len());
        let found = format!("{} arguments", values.len());
        push_diagnostic(
            diagnostics,
            "E2001",
            "TYPE_MISMATCH",
            format!("expected {expected}, found {found}"),
            [("expected", expected.as_str()), ("found", found.as_str())],
            range,
        );
        return unknown(range);
    }
    for (parameter, value) in parameters.iter().zip(values) {
        require_type(parameter, value, diagnostics);
    }
    Value {
        ty: result,
        range,
        integer: None,
    }
}

fn infer_collection_call(
    callee: &str,
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Option<Value> {
    let named = |name: &str| Type::Named(name.to_owned());
    let generic = |name: &str, arguments: Vec<Type>| Type::Generic {
        name: name.to_owned(),
        arguments,
    };
    let numeric_result = |ok: Type| generic("Result", vec![ok, named("NumericError")]);
    let (operation, key, value) = parse_collection_suffix(callee)?;
    let key_type = named(key);
    let list_of = |element: Type| generic("List", vec![element]);
    let map_type = |value: Option<&str>| match value {
        Some(value) => generic("Map", vec![key_type.clone(), named(value)]),
        None => generic("Set", vec![key_type.clone()]),
    };
    let (parameters, result) = match operation {
        "map.empty" | "set.empty" => {
            if !values.is_empty() {
                let found = format!("{} arguments", values.len());
                push_diagnostic(
                    diagnostics,
                    "E2001",
                    "TYPE_MISMATCH",
                    format!("expected 0 arguments, found {found}"),
                    [("expected", "0 arguments"), ("found", found.as_str())],
                    range,
                );
                return Some(unknown(range));
            }
            (Vec::new(), map_type(value.as_deref()))
        }
        "map.put" => (
            vec![
                map_type(value.as_deref()),
                key_type.clone(),
                named(value.as_deref()?),
            ],
            map_type(value.as_deref()),
        ),
        "map.get" => (
            vec![map_type(value.as_deref()), key_type],
            numeric_result(named(value.as_deref()?)),
        ),
        "map.has" | "set.has" => (vec![map_type(value.as_deref()), key_type], named("Bool")),
        // STEP-0144: set.add was previously never resolved by semantics (it
        // slipped through the silent-Unknown fallback); the surface mirrors
        // the IR registry: (set, key) -> set.
        "set.add" => (
            vec![map_type(value.as_deref()), key_type.clone()],
            map_type(value.as_deref()),
        ),
        "map.length" | "set.length" => (vec![map_type(value.as_deref())], named("U64")),
        "map.keys" | "set.to_list" => (
            vec![map_type(value.as_deref())],
            generic("List", vec![key_type]),
        ),
        "map.values" => (
            vec![map_type(value.as_deref())],
            generic("List", vec![named(value.as_deref()?)]),
        ),
        // RFC-0046 D4 (STEP-0175): numeric list monomorphs. The element is
        // the single suffix element; `List[Text]` keeps the frozen plain
        // `sico.list.*` names and is not accepted in bracket spelling.
        "list.length" => (vec![list_of(named(key))], named("U64")),
        "list.get" => (
            vec![list_of(named(key)), named("U64")],
            numeric_result(named(key)),
        ),
        "list.append" => (vec![list_of(named(key)), named(key)], list_of(named(key))),
        "list.sort" => (vec![list_of(named(key))], list_of(named(key))),
        "list.min" | "list.max" => (vec![list_of(named(key)), named(key)], named(key)),
        "list.empty" => (Vec::new(), list_of(named(key))),
        _ => return None,
    };
    if parameters.len() != values.len() {
        let expected = format!("{} arguments", parameters.len());
        let found = format!("{} arguments", values.len());
        push_diagnostic(
            diagnostics,
            "E2001",
            "TYPE_MISMATCH",
            format!("expected {expected}, found {found}"),
            [("expected", expected.as_str()), ("found", found.as_str())],
            range,
        );
        return Some(unknown(range));
    }
    for (parameter, value) in parameters.iter().zip(values) {
        require_type(parameter, value, diagnostics);
    }
    Some(Value {
        ty: result,
        range,
        integer: None,
    })
}

/// Parses one canonical suffixed collection intrinsic name, e.g.
/// `sico.map.put[Text,I64]` or `sico.set.add[I64]`, into its operation and
/// element spellings. Returns `None` for every non-canonical spelling.
fn parse_collection_suffix(callee: &str) -> Option<(&str, &str, Option<String>)> {
    const OPERATIONS: &[(&str, bool)] = &[
        ("map.empty", true),
        ("map.put", true),
        ("map.get", true),
        ("map.has", true),
        ("map.length", true),
        ("map.keys", true),
        ("map.values", true),
        ("set.empty", false),
        ("set.add", false),
        ("set.has", false),
        ("set.length", false),
        ("set.to_list", false),
        // RFC-0046 D4 (STEP-0175): numeric list monomorphs, one element.
        ("list.empty", false),
        ("list.length", false),
        ("list.get", false),
        ("list.append", false),
        ("list.sort", false),
        ("list.min", false),
        ("list.max", false),
    ];
    const ELEMENTS: &[&str] = &["Text", "Bytes", "Bool", "I64", "U64"];
    // STEP-0144: strip the `sico.` family prefix so the canonical
    // collection intrinsics genuinely resolve at check time (previously
    // they slipped through the silent-Unknown fallback and were only
    // resolved by the IR's own parser at lowering).
    let path = callee.strip_prefix("sico.")?;
    let (path, suffix) = path.split_once('[')?;
    if !suffix.ends_with(']') {
        return None;
    }
    let (_operation, carries_value) = OPERATIONS.iter().find(|entry| entry.0 == path)?;
    let mut elements = suffix[..suffix.len() - 1].split(',');
    let key = elements.next()?;
    if !ELEMENTS.contains(&key) {
        return None;
    }
    let value = if *carries_value {
        let value = elements.next()?;
        if !ELEMENTS.contains(&value) {
            return None;
        }
        Some(value.to_owned())
    } else {
        None
    };
    if elements.next().is_some() {
        return None;
    }
    // v0 executable surface: `map.get` materializes only the fixed-width
    // `Result[I64|U64, NumericError]` local layout; traversal helpers
    // (`map.keys`/`set.to_list`) materialize only `List[Text]`;
    // `map.values` materializes `List[V]` for the executable list element
    // set (RFC-0046 D4 adds `List[I64]`/`List[U64]`, keyed `Text` maps only
    // — no frozen corpus builds other keyed maps); numeric list monomorphs
    // carry `I64`/`U64`. Everything else returns `None` so the call falls
    // to the unknown-callee diagnostic instead of an undeclared
    // check/build gap.
    let executable = match path {
        "map.get" => matches!(value.as_deref(), Some("I64" | "U64")),
        "map.keys" | "set.to_list" => key == "Text",
        "map.values" => key == "Text" && matches!(value.as_deref(), Some("Text" | "I64" | "U64")),
        "list.length" | "list.get" | "list.append" | "list.sort" | "list.min" | "list.max" => {
            matches!(key, "I64" | "U64")
        }
        "list.empty" => matches!(key, "I64" | "U64"),
        _ => true,
    };
    if !executable {
        return None;
    }
    Some((path, key, value))
}

#[allow(clippy::too_many_lines)]
fn infer_stdlib_call(
    callee: &str,
    values: &[Value],
    range: TextRange,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Option<Value> {
    if let Some(value) = infer_collection_call(callee, values, range, diagnostics) {
        return Some(value);
    }
    let named = |name: &str| Type::Named(name.to_owned());
    let generic = |name: &str, argument: Type| Type::Generic {
        name: name.to_owned(),
        arguments: vec![argument],
    };
    let list_text = || generic("List", named("Text"));
    let numeric_result = |ok: Type| Type::Generic {
        name: "Result".to_owned(),
        arguments: vec![ok, named("NumericError")],
    };
    let (parameters, result) = match callee {
        "sico.bytes.length" => (vec![named("Bytes")], named("U64")),
        "sico.bytes.concat" => (vec![named("Bytes"), named("Bytes")], named("Bytes")),
        "sico.bytes.slice" => (
            vec![named("Bytes"), named("U64"), named("U64")],
            numeric_result(named("Bytes")),
        ),
        "sico.bytes.is_utf8" => (vec![named("Bytes")], named("Bool")),
        "sico.bytes.utf8_decode" => (vec![named("Bytes")], named("Text")),
        "sico.text.encode" => (vec![named("Text")], named("Bytes")),
        "sico.text.length" | "sico.text.leading_spaces" => (vec![named("Text")], named("U64")),
        "sico.text.concat" | "sico.json.get" => (vec![named("Text"), named("Text")], named("Text")),
        "sico.text.trim" | "sico.json.quote" => (vec![named("Text")], named("Text")),
        "sico.text.contains" | "sico.text.starts_with" | "sico.text.ends_with" => {
            (vec![named("Text"), named("Text")], named("Bool"))
        }
        "sico.text.split_lines" | "sico.text.split_words" => (vec![named("Text")], list_text()),
        "sico.text.replace" => (
            vec![named("Text"), named("Text"), named("Text")],
            named("Text"),
        ),
        "sico.text.join" | "sico.list.min" | "sico.list.max" => {
            (vec![list_text(), named("Text")], named("Text"))
        }
        "sico.json.is_valid" => (vec![named("Text")], named("Bool")),

        "sico.json.has" => (vec![named("Text"), named("Text")], named("Bool")),

        "sico.list.length" => (vec![list_text()], named("U64")),
        "sico.list.get" => (
            vec![list_text(), named("U64")],
            numeric_result(named("Text")),
        ),
        "sico.list.append" => (vec![list_text(), named("Text")], list_text()),
        // RFC-0045 stdlib batch 2 (STEP-0174).
        "sico.bytes.at" => (
            vec![named("Bytes"), named("U64"), named("I64")],
            named("I64"),
        ),
        "sico.bytes.equal" => (vec![named("Bytes"), named("Bytes")], named("Bool")),
        "sico.text.compare" => (vec![named("Text"), named("Text")], named("I64")),
        "sico.text.char_at" => (
            vec![named("Text"), named("U64")],
            numeric_result(named("Text")),
        ),
        "sico.text.format" => (vec![named("Text"), list_text()], named("Text")),
        "sico.list.sort" => (vec![list_text()], list_text()),
        "sico.u64.to_text" => (vec![named("U64")], named("Text")),
        "sico.i64.to_text" => (vec![named("I64")], named("Text")),
        "sico.fs.read" => (
            vec![named("Text")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Bytes"), named("Text")],
            },
        ),
        "sico.fs.exists" => (
            vec![named("Text")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Bool"), named("Text")],
            },
        ),
        "sico.fs.write" => (
            vec![named("Text"), named("Bytes")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Bool"), named("Text")],
            },
        ),
        "sico.http.request" => (
            vec![named("Text"), named("Text"), named("Bytes")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("HttpResponse"), named("Text")],
            },
        ),
        // STEP-0136: buffered one-shot over http@0.2.0 with Host-default
        // options; the typed http-error enum surfaces as its case-name Text.
        "sico.http2.request" => (
            vec![named("Text"), named("Text"), named("Bytes")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Http2Response"), named("Text")],
            },
        ),
        "sico.stream.stdin" => (vec![], named("InputStream")),
        "sico.stream.stdout" | "sico.stream.stderr" => (vec![], named("OutputStream")),
        "sico.stream.read" => (
            vec![named("InputStream"), named("U64")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Bytes"), named("Text")],
            },
        ),
        "sico.stream.write" => (
            vec![named("OutputStream"), named("Bytes")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Bool"), named("Text")],
            },
        ),
        "sico.stream.flush" => (
            vec![named("OutputStream")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("Bool"), named("Text")],
            },
        ),
        "sico.stream.pump" => (
            vec![named("InputStream"), named("OutputStream")],
            Type::Generic {
                name: "Result".to_owned(),
                arguments: vec![named("U64"), named("Text")],
            },
        ),
        "sico.stream.close_input" => (vec![named("InputStream")], named("Bool")),
        "sico.stream.close_output" => (vec![named("OutputStream")], named("Bool")),
        _ => return None,
    };
    for (parameter, value) in parameters.iter().zip(values) {
        require_type(parameter, value, diagnostics);
    }
    Some(Value {
        ty: result,
        range,
        integer: None,
    })
}

fn infer_result_constructor(callee: &str, values: &[Value], range: TextRange) -> Option<Value> {
    let value_type = values
        .first()
        .map_or(Type::Unknown, |value| value.ty.clone());
    let arguments = match callee {
        "ok" => vec![value_type, Type::Unknown],
        "error" => vec![Type::Unknown, value_type],
        _ => return None,
    };
    Some(Value {
        ty: Type::Generic {
            name: "Result".to_owned(),
            arguments,
        },
        range,
        integer: None,
    })
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
            // RFC-0047 D7 (STEP-0271): a duplicate literal field is a
            // typed refusal (E2021), never a silent last-write-wins.
            if provided.insert(name.clone(), value.clone()).is_some() {
                push_diagnostic(
                    diagnostics,
                    "E2021",
                    "DUPLICATE_FIELD",
                    format!("duplicate field: {name}"),
                    [("field", name.as_str())],
                    argument.tokens.first().map_or(range, |token| token.range),
                );
            }
            require_type(&field.ty, value, diagnostics);
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
    if types_compatible(expected, &value.ty) {
        return;
    }
    let expected_name = expected.to_string();
    let found_name = value.ty.to_string();
    let numeric = matches!(expected, Type::Named(name) if matches!(name.as_str(), "Float32" | "Float64" | "I64" | "U64"))
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

fn types_compatible(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Named(left), Type::Named(right)) => left == right,
        (
            Type::Generic {
                name: left_name,
                arguments: left_arguments,
            },
            Type::Generic {
                name: right_name,
                arguments: right_arguments,
            },
        ) => {
            left_name == right_name
                && left_arguments.len() == right_arguments.len()
                && left_arguments
                    .iter()
                    .zip(right_arguments)
                    .all(|(left, right)| types_compatible(left, right))
        }
        _ => false,
    }
}

fn result_parts(ty: &Type) -> Option<(&Type, &Type)> {
    let Type::Generic { name, arguments } = ty else {
        return None;
    };
    if name != "Result" || arguments.len() != 2 {
        return None;
    }
    Some((&arguments[0], &arguments[1]))
}

fn enum_variant<'a>(
    model: &'a Model,
    enum_name: &str,
    variant_name: &str,
) -> Option<&'a VariantDefinition> {
    let TypeDefinition::Enum { variants } = model.types.get(enum_name)? else {
        return None;
    };
    variants.iter().find(|variant| variant.name == variant_name)
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
            .map(|(name, value)| (name.to_owned(), DiagnosticArgument::Text(value.to_owned())))
            .collect(),
        range,
    });
}

fn push_list_diagnostic(
    diagnostics: &mut Vec<SemanticDiagnostic>,
    code: &'static str,
    key: &'static str,
    message: String,
    argument_name: &str,
    values: Vec<String>,
    range: TextRange,
) {
    if diagnostics.len() == MAX_SEMANTIC_DIAGNOSTICS {
        return;
    }
    diagnostics.push(SemanticDiagnostic {
        code,
        key,
        message,
        arguments: BTreeMap::from([(argument_name.to_owned(), DiagnosticArgument::List(values))]),
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
                    .all(|pair| pair[0].id < pair[1].id),
                "{}: {:?}",
                path.display(),
                analysis.facts
            );
            assert!(
                analysis
                    .facts
                    .iter()
                    .all(|fact| source.span(fact.range).is_some())
            );
            assert!(
                analysis
                    .facts
                    .windows(2)
                    .all(|pair| pair[0].id < pair[1].id)
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
                assert_diagnostic_matches(diagnostic, oracle);
                assert!(source.span(diagnostic.range).is_some());
            }
        }
        assert_eq!((accepted, rejected), (7, 9));
    }

    #[test]
    fn match_and_result_cases_match_the_registered_oracle() {
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
                let id = case["case"].as_str().unwrap();
                id.starts_with("MATCH-") || id.starts_with("RESULT-")
            })
            .map(|case| (case["case"].as_str().unwrap().to_owned(), case.clone()))
            .collect();
        assert_eq!(expected.len(), 8);

        let mut paths = Vec::new();
        collect_sico(
            &repository.join("syntax-candidates/b/exhaustive-match"),
            &mut paths,
        );
        collect_sico(
            &repository.join("syntax-candidates/b/result-mapping"),
            &mut paths,
        );
        paths.sort();
        assert_eq!(paths.len(), 14);
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
                let oracle = &expected[metadata(source.text(), "case")];
                assert_eq!(
                    analysis.diagnostics.len(),
                    1,
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
                assert_diagnostic_matches(&analysis.diagnostics[0], oracle);
                assert!(source.span(analysis.diagnostics[0].range).is_some());
            }
            assert!(
                analysis
                    .facts
                    .iter()
                    .all(|fact| source.span(fact.range).is_some())
            );
        }
        assert_eq!((accepted, rejected), (6, 8));
    }

    #[test]
    fn capability_and_component_cases_match_the_registered_oracle() {
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
                let id = case["case"].as_str().unwrap();
                id.starts_with("CAP-") || id.starts_with("COMP-")
            })
            .map(|case| (case["case"].as_str().unwrap().to_owned(), case.clone()))
            .collect();
        assert_eq!(expected.len(), 4);

        let mut paths = Vec::new();
        collect_sico(
            &repository.join("syntax-candidates/b/effects-capabilities"),
            &mut paths,
        );
        collect_sico(
            &repository.join("syntax-candidates/b/component-call"),
            &mut paths,
        );
        paths.sort();
        assert_eq!(paths.len(), 8);
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
                let oracle = &expected[metadata(source.text(), "case")];
                assert_eq!(
                    analysis.diagnostics.len(),
                    1,
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
                assert_diagnostic_matches(&analysis.diagnostics[0], oracle);
                assert!(source.span(analysis.diagnostics[0].range).is_some());
            }
            assert!(
                analysis
                    .facts
                    .windows(2)
                    .all(|pair| pair[0].id < pair[1].id),
                "{}: {:?}",
                path.display(),
                analysis.facts
            );
            assert!(
                analysis
                    .facts
                    .iter()
                    .all(|fact| source.span(fact.range).is_some())
            );
        }
        assert_eq!((accepted, rejected), (4, 4));
    }

    #[test]
    fn resource_task_and_stream_cases_match_the_registered_oracle() {
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
                let id = case["case"].as_str().unwrap();
                id.starts_with("RES-") || id.starts_with("TASK-") || id.starts_with("STREAM-")
            })
            .map(|case| (case["case"].as_str().unwrap().to_owned(), case.clone()))
            .collect();
        assert_eq!(expected.len(), 10);

        let mut paths = Vec::new();
        for group in ["affine-resources", "future-task", "stream"] {
            collect_sico(
                &repository.join("syntax-candidates/b").join(group),
                &mut paths,
            );
        }
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
                let oracle = &expected[metadata(source.text(), "case")];
                assert_eq!(
                    analysis.diagnostics.len(),
                    1,
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
                assert_diagnostic_matches(&analysis.diagnostics[0], oracle);
                assert!(source.span(analysis.diagnostics[0].range).is_some());
            }
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
        }
        assert_eq!((accepted, rejected), (6, 10));
    }

    #[test]
    fn revision_cases_match_the_registered_oracle() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let map: JsonValue = serde_json::from_str(
            &fs::read_to_string(repository.join("diagnostics/semantic-case-map.json")).unwrap(),
        )
        .unwrap();
        let expected: BTreeMap<_, _> = map["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["case"].as_str().unwrap().starts_with("REV-"))
            .map(|case| (case["case"].as_str().unwrap().to_owned(), case.clone()))
            .collect();
        assert_eq!(expected.len(), 2);

        let mut paths = Vec::new();
        collect_sico(&repository.join("syntax-candidates/b/revision"), &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 4);
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
                let oracle = &expected[metadata(source.text(), "case")];
                assert_eq!(
                    analysis.diagnostics.len(),
                    1,
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
                assert_diagnostic_matches(&analysis.diagnostics[0], oracle);
                assert!(source.span(analysis.diagnostics[0].range).is_some());
            }
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
        }
        assert_eq!((accepted, rejected), (2, 2));
    }

    #[test]
    fn remaining_b_groups_do_not_receive_unowned_diagnostics() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-candidates/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 58);
        for (index, path) in paths.iter().enumerate() {
            let path_text = path.to_string_lossy();
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            let analysis = analyze(&source).unwrap();
            if !path_text.contains("numbers-units") && !path_text.contains("nominal-invariants") {
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
            if !path_text.contains("exhaustive-match") && !path_text.contains("result-mapping") {
                assert!(
                    analysis
                        .diagnostics
                        .iter()
                        .all(|diagnostic| !diagnostic.code.starts_with("E3")),
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
            }
            if !path_text.contains("effects-capabilities") && !path_text.contains("component-call")
            {
                assert!(
                    analysis.diagnostics.iter().all(|diagnostic| {
                        !diagnostic.code.starts_with("E4") && !diagnostic.code.starts_with("E6")
                    }),
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
            }
            if !path_text.contains("affine-resources")
                && !path_text.contains("future-task")
                && !path_text.contains("stream")
            {
                assert!(
                    analysis
                        .diagnostics
                        .iter()
                        .all(|diagnostic| !diagnostic.code.starts_with("E5")),
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
            }
            if !path_text.contains("revision") {
                assert!(
                    analysis
                        .diagnostics
                        .iter()
                        .all(|diagnostic| !diagnostic.code.starts_with("E7")),
                    "{}: {:?}",
                    path.display(),
                    analysis.diagnostics
                );
            }
        }
    }

    fn metadata<'a>(text: &'a str, key: &str) -> &'a str {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("// {key}: ")))
            .unwrap()
    }

    fn assert_diagnostic_matches(diagnostic: &SemanticDiagnostic, oracle: &JsonValue) {
        assert_eq!(diagnostic.code, oracle["code"]);
        assert_eq!(diagnostic.key, oracle["key"]);
        assert_eq!(diagnostic.message, oracle["expected_message"]);
        let expected_arguments: BTreeMap<_, _> = oracle["arguments"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| {
                let value = if let Some(text) = value.as_str() {
                    DiagnosticArgument::Text(text.to_owned())
                } else {
                    DiagnosticArgument::List(
                        value
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|item| item.as_str().unwrap().to_owned())
                            .collect(),
                    )
                };
                (name.clone(), value)
            })
            .collect();
        assert_eq!(diagnostic.arguments, expected_arguments);
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
