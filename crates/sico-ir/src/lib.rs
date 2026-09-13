//! Typed, deterministic compiler IR and its independent verifier.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sico_semantics::{Analysis, AnalyzeError, SemanticDiagnostic, analyze};
use sico_source::{SourceFile, TextRange};

mod lower;

pub use lower::{CoreLowerError, ModuleImport, PackageImport, lower_core, lower_core_modules};

pub const SCHEMA: &str = "sico.ir.v0";
pub const MAX_IR_DIAGNOSTICS: usize = 100;
pub const MAX_FUNCTIONS: usize = 10_000;
pub const MAX_BLOCKS_PER_FUNCTION: usize = 100_000;
pub const MAX_LOCALS_PER_FUNCTION: usize = 256;
pub const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 1_000_000;
pub const MAX_TASK_SCOPE_DEPTH: usize = 64;
pub const MAX_TASK_SPAWNS_PER_SCOPE: usize = 1024;
pub const MAX_TASK_COLLECT_LENGTH: usize = 1024;
pub const NUMERIC_ERROR_TYPE: &str = "NumericError";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct FunctionId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BlockId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceRange {
    pub start: u32,
    pub end: u32,
}

impl SourceRange {
    #[must_use]
    pub fn from_text_range(range: TextRange) -> Self {
        Self {
            start: range.start().into(),
            end: range.end().into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Type {
    Unit,
    Bool,
    Int,
    I64,
    U64,
    Float64,
    String,
    Bytes,
    List(Box<Self>),
    /// Insertion-ordered map (M14 STEP-0131): functional copy-on-put, keys
    /// keep their first-insertion position, iteration follows insertion
    /// order. Keys and values are the scalar collection surface
    /// (`String`/`Bytes`/`Bool`/`I64`/`U64`).
    Map {
        key: Box<Self>,
        value: Box<Self>,
    },
    /// Insertion-ordered set (M14 STEP-0131): `Map { key, value: Unit }`
    /// sharing the map cell machinery without a value slot.
    Set(Box<Self>),
    Named(String),
    Option(Box<Self>),
    Result {
        ok: Box<Self>,
        error: Box<Self>,
    },
    Capability(String),
    OwnedResource(String),
    BorrowedResource(String),
    Task(Box<Self>),
    Future(Box<Self>),
    Stream(Box<Self>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Module {
    pub schema: String,
    pub source_name: String,
    pub source_len: u32,
    pub functions: Vec<Function>,
    /// Canonical per-function task-scope declaration tables (RFC-0036 §5.1),
    /// keyed by function id. Modules without task structure omit the table
    /// entirely and keep their exact previous serialization shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_scopes: Option<BTreeMap<FunctionId, Vec<TaskScope>>>,
    /// RFC-0039 §2.4 (STEP-0147): user-WIT package-import signatures keyed
    /// by the full import name `sico:user/<interface>@<version>.<function>`.
    /// Consulted by the verifier after the fixed intrinsic table; empty for
    /// programs without package imports, so frozen serializations are
    /// byte-identical.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub import_signatures: BTreeMap<String, (Vec<Type>, Type)>,
}

impl Module {
    #[must_use]
    pub fn new(source_name: impl Into<String>, source_len: u32) -> Self {
        Self {
            schema: SCHEMA.to_owned(),
            source_name: source_name.into(),
            source_len,
            functions: Vec::new(),
            task_scopes: None,
            import_signatures: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub effects: Vec<String>,
    /// Mutable local cells (M14 STEP-0130): names reassigned via `set` or
    /// across loop iterations. Empty for every pre-STEP-0130 function.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locals: Vec<Local>,
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub range: SourceRange,
}

/// One mutable local cell. Reads emit [`Operation::ReadLocal`], writes
/// [`Operation::WriteLocal`]; ids ascend from 0 in first-assignment order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Local {
    pub name: String,
    pub ty: Type,
    pub range: SourceRange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parameter {
    pub id: ValueId,
    pub name: String,
    pub ty: Type,
    pub range: SourceRange,
}

/// One entry of a function's canonical task-scope table (RFC-0036 §5.1):
/// ids ascend from 0 in declaration order and a parent is always declared
/// before its children.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaskScope {
    pub scope: u32,
    pub parent: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Block {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub range: SourceRange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Instruction {
    pub result: ValueId,
    pub ty: Type,
    pub operation: Operation,
    pub range: SourceRange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructField {
    pub name: String,
    pub value: ValueId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", content = "data", rename_all = "snake_case")]
pub enum Operation {
    ConstInt(String),
    ConstI64(i64),
    ConstU64(u64),
    ConstBool(bool),
    ConstString(String),
    ConstBytes(Vec<u8>),
    Copy(ValueId),
    AddInt {
        left: ValueId,
        right: ValueId,
    },
    CheckedAdd {
        left: ValueId,
        right: ValueId,
    },
    CheckedSub {
        left: ValueId,
        right: ValueId,
    },
    CheckedMul {
        left: ValueId,
        right: ValueId,
    },
    CheckedDiv {
        left: ValueId,
        right: ValueId,
    },
    EqualFixed {
        left: ValueId,
        right: ValueId,
    },
    LessFixed {
        left: ValueId,
        right: ValueId,
    },
    BitAnd {
        left: ValueId,
        right: ValueId,
    },
    BitOr {
        left: ValueId,
        right: ValueId,
    },
    BitXor {
        left: ValueId,
        right: ValueId,
    },
    Shl {
        value: ValueId,
        amount: ValueId,
    },
    Shr {
        value: ValueId,
        amount: ValueId,
    },
    Call {
        function: FunctionId,
        arguments: Vec<ValueId>,
    },
    Intrinsic {
        name: String,
        arguments: Vec<ValueId>,
    },
    Construct {
        name: String,
        fields: Vec<ConstructField>,
    },
    Project {
        base: ValueId,
        field: String,
    },
    Variant {
        name: String,
        payload: Vec<ValueId>,
    },
    EffectCall {
        effect: String,
        arguments: Vec<ValueId>,
    },
    ResourceCall {
        resource: ValueId,
        method: String,
        arguments: Vec<ValueId>,
    },
    ResourceMove(ValueId),
    ResourceBorrow(ValueId),
    ResourceDrop(ValueId),
    RevisionCheck {
        value: ValueId,
        expected: ValueId,
    },
    Try(ValueId),
    Await(ValueId),
    StreamNext(ValueId),
    ReadLocal {
        local: u32,
    },
    WriteLocal {
        local: u32,
        value: ValueId,
    },
    TaskScopeOpen {
        scope: u32,
    },
    TaskScopeClose {
        scope: u32,
    },
    Spawn {
        scope: u32,
        callee: FunctionId,
        arguments: Vec<ValueId>,
    },
    TaskCollect {
        scope: u32,
        tasks: ValueId,
    },
}

impl Operation {
    fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::ConstInt(_)
            | Self::ConstI64(_)
            | Self::ConstU64(_)
            | Self::ConstBool(_)
            | Self::ConstString(_)
            | Self::ConstBytes(_)
            | Self::TaskScopeOpen { .. }
            | Self::TaskScopeClose { .. }
            | Self::ReadLocal { .. } => Vec::new(),
            Self::Copy(value)
            | Self::ResourceMove(value)
            | Self::ResourceBorrow(value)
            | Self::ResourceDrop(value)
            | Self::Try(value)
            | Self::Await(value)
            | Self::StreamNext(value)
            | Self::WriteLocal { value, .. } => vec![*value],
            Self::AddInt { left, right }
            | Self::CheckedAdd { left, right }
            | Self::CheckedSub { left, right }
            | Self::CheckedMul { left, right }
            | Self::CheckedDiv { left, right }
            | Self::EqualFixed { left, right }
            | Self::LessFixed { left, right }
            | Self::BitAnd { left, right }
            | Self::BitOr { left, right }
            | Self::BitXor { left, right }
            | Self::Shl {
                value: left,
                amount: right,
            }
            | Self::Shr {
                value: left,
                amount: right,
            } => vec![*left, *right],
            Self::Call { arguments, .. }
            | Self::Intrinsic { arguments, .. }
            | Self::Variant {
                payload: arguments, ..
            }
            | Self::Spawn { arguments, .. }
            | Self::EffectCall { arguments, .. } => arguments.clone(),
            Self::Construct { fields, .. } => fields.iter().map(|field| field.value).collect(),
            Self::ResourceCall {
                resource,
                arguments,
                ..
            } => std::iter::once(*resource)
                .chain(arguments.iter().copied())
                .collect(),
            Self::Project { base, .. } => vec![*base],
            Self::RevisionCheck { value, expected } => vec![*value, *expected],
            Self::TaskCollect { tasks, .. } => vec![*tasks],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Terminator {
    Return(Option<ValueId>),
    Jump(BlockId),
    Branch {
        condition: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Match {
        values: Vec<ValueId>,
        arms: Vec<MatchArm>,
    },
    Unreachable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MatchArm {
    pub patterns: Vec<Pattern>,
    pub target: BlockId,
    pub range: SourceRange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Pattern {
    Wildcard,
    Binding(String),
    Bool(bool),
    Variant { name: String, payload: Vec<Self> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyError {
    pub path: String,
    pub kind: VerifyErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifyErrorKind {
    Schema,
    Limit,
    DuplicateId,
    NonCanonicalId,
    NonCanonicalOrder,
    InvalidRange,
    MissingEntry,
    UnknownTarget,
    UndefinedValue,
    TypeMismatch,
    InvalidConstant,
    UndeclaredEffect,
    ResourceViolation,
    ResourceLeak,
    BorrowEscape,
    RevisionGuard,
    TaskViolation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryError {
    Frontend(AnalyzeError),
    Semantic(Vec<SemanticDiagnostic>),
}

/// Proves that a source passed M2 before any IR construction begins.
///
/// # Errors
///
/// Returns frontend failure or the compiler-produced semantic diagnostics.
pub fn require_semantic_success(source: &SourceFile) -> Result<Analysis, EntryError> {
    let analysis = analyze(source).map_err(EntryError::Frontend)?;
    if analysis.is_success() {
        Ok(analysis)
    } else {
        Err(EntryError::Semantic(analysis.diagnostics))
    }
}

/// Independently verifies a module, retaining at most 100 root errors.
#[must_use]
pub fn verify(module: &Module) -> Vec<VerifyError> {
    let mut verifier = Verifier::new(module);
    verifier.run();
    verifier.errors
}

/// Verifies and then emits stable compact JSON in declaration order.
///
/// # Errors
///
/// Returns verifier errors and never serializes invalid IR.
///
/// # Panics
///
/// Panics only if serde cannot serialize the closed IR data model after verification.
pub fn canonical_json(module: &Module) -> Result<String, Vec<VerifyError>> {
    let errors = verify(module);
    if errors.is_empty() {
        Ok(serde_json::to_string(module).expect("IR JSON serialization cannot fail"))
    } else {
        Err(errors)
    }
}

struct Verifier<'a> {
    module: &'a Module,
    errors: Vec<VerifyError>,
    signatures: BTreeMap<FunctionId, (&'a [Parameter], &'a Type, &'a [String])>,
}

impl<'a> Verifier<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            errors: Vec::new(),
            signatures: BTreeMap::new(),
        }
    }

    fn run(&mut self) {
        if self.module.schema != SCHEMA {
            self.error("module", VerifyErrorKind::Schema);
        }
        if self.module.functions.len() > MAX_FUNCTIONS {
            self.error("module.functions", VerifyErrorKind::Limit);
        }
        for function in &self.module.functions {
            if self
                .signatures
                .insert(
                    function.id,
                    (
                        &function.parameters,
                        &function.return_type,
                        &function.effects,
                    ),
                )
                .is_some()
            {
                self.error(
                    format!("function[{}]", function.id.0),
                    VerifyErrorKind::DuplicateId,
                );
            }
        }
        self.verify_task_scope_tables();
        for (index, function) in self.module.functions.iter().enumerate() {
            let expected = u32::try_from(index + 1).unwrap_or(u32::MAX);
            if function.id != FunctionId(expected) {
                self.error(
                    format!("function[{index}].id"),
                    VerifyErrorKind::NonCanonicalId,
                );
            }
            self.verify_function(function, index);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn verify_function(&mut self, function: &Function, function_index: usize) {
        let root = format!("function[{function_index}]");
        self.range(&format!("{root}.range"), function.range);
        if function.blocks.len() > MAX_BLOCKS_PER_FUNCTION
            || function
                .blocks
                .iter()
                .map(|block| block.instructions.len())
                .sum::<usize>()
                > MAX_INSTRUCTIONS_PER_FUNCTION
            || function.locals.len() > MAX_LOCALS_PER_FUNCTION
        {
            self.error(&root, VerifyErrorKind::Limit);
        }
        for (index, local) in function.locals.iter().enumerate() {
            self.range(&format!("{root}.local[{index}].range"), local.range);
            if local.name.is_empty() {
                self.error(
                    format!("{root}.local[{index}].name"),
                    VerifyErrorKind::Limit,
                );
            }
        }
        if function.effects.windows(2).any(|pair| pair[0] >= pair[1]) {
            self.error(
                format!("{root}.effects"),
                VerifyErrorKind::NonCanonicalOrder,
            );
        }
        let mut block_ids = BTreeSet::new();
        for (index, block) in function.blocks.iter().enumerate() {
            if !block_ids.insert(block.id) {
                self.error(
                    format!("{root}.block[{index}].id"),
                    VerifyErrorKind::DuplicateId,
                );
            }
            if block.id != BlockId(u32::try_from(index).unwrap_or(u32::MAX)) {
                self.error(
                    format!("{root}.block[{index}].id"),
                    VerifyErrorKind::NonCanonicalId,
                );
            }
        }
        if !block_ids.contains(&function.entry) {
            self.error(format!("{root}.entry"), VerifyErrorKind::MissingEntry);
        }
        let task_scopes: BTreeMap<u32, Option<u32>> = self
            .module
            .task_scopes
            .as_ref()
            .and_then(|tables| tables.get(&function.id))
            .map_or_else(BTreeMap::new, |scopes| {
                scopes
                    .iter()
                    .map(|scope| (scope.scope, scope.parent))
                    .collect()
            });
        let mut parameters = BTreeMap::new();
        for (index, parameter) in function.parameters.iter().enumerate() {
            self.range(&format!("{root}.parameter[{index}].range"), parameter.range);
            if parameter.id != ValueId(u32::try_from(index).unwrap_or(u32::MAX))
                || parameters
                    .insert(parameter.id, parameter.ty.clone())
                    .is_some()
            {
                self.error(
                    format!("{root}.parameter[{index}].id"),
                    VerifyErrorKind::NonCanonicalId,
                );
            }
        }
        let mut spawn_counts: BTreeMap<u32, usize> = BTreeMap::new();
        for (block_index, block) in function.blocks.iter().enumerate() {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                if let Operation::Spawn { scope, .. } = &instruction.operation {
                    let count = spawn_counts.entry(*scope).or_insert(0);
                    *count += 1;
                    if *count == MAX_TASK_SPAWNS_PER_SCOPE + 1 {
                        self.error(
                            format!("{root}.block[{block_index}].instruction[{instruction_index}]"),
                            VerifyErrorKind::Limit,
                        );
                    }
                }
            }
        }
        let mut next_value = u32::try_from(function.parameters.len()).unwrap_or(u32::MAX);
        for (block_index, block) in function.blocks.iter().enumerate() {
            let block_root = format!("{root}.block[{block_index}]");
            self.range(&format!("{block_root}.range"), block.range);
            let mut available = parameters.clone();
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                let instruction_root = format!("{block_root}.instruction[{instruction_index}]");
                self.range(&format!("{instruction_root}.range"), instruction.range);
                if instruction.result != ValueId(next_value) {
                    self.error(
                        format!("{instruction_root}.result"),
                        VerifyErrorKind::NonCanonicalId,
                    );
                }
                next_value = next_value.saturating_add(1);
                self.verify_operation(
                    &instruction_root,
                    instruction,
                    &available,
                    &function.effects,
                    &function.locals,
                );
                available.insert(instruction.result, instruction.ty.clone());
            }
            self.verify_terminator(
                &block_root,
                &block.terminator,
                &available,
                &block_ids,
                &function.return_type,
            );
            self.verify_affine_and_revision(&block_root, block, &parameters);
            self.verify_task_flow(
                &block_root,
                block,
                &parameters,
                &task_scopes,
                &function.effects,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn verify_operation(
        &mut self,
        path: &str,
        instruction: &Instruction,
        available: &BTreeMap<ValueId, Type>,
        effects: &[String],
        locals: &[Local],
    ) {
        for operand in instruction.operation.operands() {
            if !available.contains_key(&operand) {
                self.error(path, VerifyErrorKind::UndefinedValue);
            }
        }
        match &instruction.operation {
            Operation::ConstInt(value) => {
                if instruction.ty != Type::Int || !canonical_integer(value) {
                    self.error(path, VerifyErrorKind::InvalidConstant);
                }
            }
            Operation::ConstI64(_) if instruction.ty != Type::I64 => {
                self.error(path, VerifyErrorKind::InvalidConstant);
            }
            Operation::ConstU64(_) if instruction.ty != Type::U64 => {
                self.error(path, VerifyErrorKind::InvalidConstant);
            }
            Operation::ConstBool(_) if instruction.ty != Type::Bool => {
                self.error(path, VerifyErrorKind::TypeMismatch);
            }
            Operation::ConstString(_) if instruction.ty != Type::String => {
                self.error(path, VerifyErrorKind::TypeMismatch);
            }
            Operation::ConstBytes(_) if instruction.ty != Type::Bytes => {
                self.error(path, VerifyErrorKind::TypeMismatch);
            }
            Operation::Copy(value) => self.expect_type(path, available.get(value), &instruction.ty),
            Operation::AddInt { left, right } => {
                self.expect_type(path, available.get(left), &Type::Int);
                self.expect_type(path, available.get(right), &Type::Int);
                if instruction.ty != Type::Int {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                }
            }
            Operation::CheckedAdd { left, right }
            | Operation::CheckedSub { left, right }
            | Operation::CheckedMul { left, right }
            | Operation::CheckedDiv { left, right } => {
                self.verify_checked_fixed(path, instruction, available, *left, *right);
            }
            Operation::EqualFixed { left, right } | Operation::LessFixed { left, right } => {
                self.verify_fixed_comparison(path, instruction, available, *left, *right);
            }
            Operation::BitAnd { left, right }
            | Operation::BitOr { left, right }
            | Operation::BitXor { left, right }
            | Operation::Shl {
                value: left,
                amount: right,
            }
            | Operation::Shr {
                value: left,
                amount: right,
            } => {
                // Bit operations are pure: both operands share the result's
                // fixed-width type, and the result is not a Result.
                if !matches!(instruction.ty, Type::I64 | Type::U64) {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                }
                for operand in [*left, *right] {
                    self.expect_type(path, available.get(&operand), &instruction.ty);
                }
            }
            Operation::Call {
                function,
                arguments,
            } => {
                let Some((parameters, return_type, _)) = self.signatures.get(function).copied()
                else {
                    self.error(path, VerifyErrorKind::UnknownTarget);
                    return;
                };
                if parameters.len() != arguments.len() || return_type != &instruction.ty {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                } else {
                    for (parameter, argument) in parameters.iter().zip(arguments) {
                        self.expect_type(path, available.get(argument), &parameter.ty);
                    }
                }
            }
            Operation::Intrinsic { name, arguments } => {
                // Fixed intrinsics first; RFC-0039 §2.4 user-WIT package
                // imports (STEP-0147) resolve through the module's own
                // import-signature table.
                let fixed = intrinsic_signature(name);
                let signature = match &fixed {
                    Some((parameters, result)) => Some((parameters, result)),
                    None => self.module.import_signatures.get(name).map(|(p, r)| (p, r)),
                };
                let Some((parameters, result)) = signature else {
                    self.error(path, VerifyErrorKind::UnknownTarget);
                    return;
                };
                if parameters.len() != arguments.len() || *result != instruction.ty {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                    return;
                }
                for (parameter, argument) in parameters.iter().zip(arguments) {
                    self.expect_type(path, available.get(argument), parameter);
                }
            }
            Operation::Construct { .. } | Operation::Project { .. } | Operation::Variant { .. } => {
                self.verify_data_operation(path, instruction, available);
            }
            Operation::Try(value) => match available.get(value) {
                Some(Type::Result { ok, .. }) if ok.as_ref() == &instruction.ty => {}
                Some(_) => self.error(path, VerifyErrorKind::TypeMismatch),
                None => self.error(path, VerifyErrorKind::UndefinedValue),
            },
            Operation::EffectCall { effect, .. } if effects.binary_search(effect).is_err() => {
                self.error(path, VerifyErrorKind::UndeclaredEffect);
            }
            Operation::ResourceCall { resource, .. } => {
                if !matches!(available.get(resource), Some(Type::BorrowedResource(_))) {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                }
            }
            Operation::ReadLocal { local } => {
                let Some(decl) = locals.get(*local as usize) else {
                    self.error(path, VerifyErrorKind::UndefinedValue);
                    return;
                };
                if instruction.ty != decl.ty {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                }
            }
            Operation::WriteLocal { local, value } => {
                let Some(decl) = locals.get(*local as usize) else {
                    self.error(path, VerifyErrorKind::UndefinedValue);
                    return;
                };
                if instruction.ty != Type::Unit {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                    return;
                }
                self.expect_type(path, available.get(value), &decl.ty);
            }
            Operation::RevisionCheck { value, expected } => {
                match (available.get(value), available.get(expected)) {
                    (Some(left), Some(right)) if left == right && instruction.ty == Type::Bool => {}
                    (Some(_), Some(_)) => self.error(path, VerifyErrorKind::TypeMismatch),
                    _ => self.error(path, VerifyErrorKind::UndefinedValue),
                }
            }
            _ => {}
        }
    }

    fn verify_data_operation(
        &mut self,
        path: &str,
        instruction: &Instruction,
        available: &BTreeMap<ValueId, Type>,
    ) {
        let valid = match &instruction.operation {
            Operation::Construct { name, fields } => {
                let named = instruction.ty == Type::Named(name.clone())
                    && fields.iter().all(|field| !field.name.is_empty())
                    && fields
                        .iter()
                        .map(|field| &field.name)
                        .collect::<BTreeSet<_>>()
                        .len()
                        == fields.len();
                // List literal (RFC-0036 §4): the construct that builds the
                // `List[Task[T]]` feeding a `TaskCollect` uses an empty name
                // and canonical positional field names, each element-typed.
                let list = match &instruction.ty {
                    Type::List(element) => {
                        name.is_empty()
                            && fields.iter().enumerate().all(|(index, field)| {
                                field.name == index.to_string()
                                    && available.get(&field.value) == Some(element.as_ref())
                            })
                    }
                    _ => false,
                };
                named || list
            }
            Operation::Project { base, field } => match available.get(base) {
                Some(Type::Named(_)) => !field.is_empty(),
                // Variant payload projection (`case ok(x)` lowering, STEP-0083).
                Some(Type::Result { ok, error }) => {
                    (field == "ok" && ok.as_ref() == &instruction.ty)
                        || (field == "error" && error.as_ref() == &instruction.ty)
                }
                _ => false,
            },
            Operation::Variant { name, .. } => {
                matches!(
                    (&instruction.ty, name.as_str()),
                    (Type::Result { .. }, "ok" | "error") | (Type::Option(_), "some" | "none")
                ) || (matches!(&instruction.ty, Type::Named(_)) && name.split_once('.').is_some())
            }
            _ => unreachable!("data operation caller"),
        };
        if !valid {
            self.error(path, VerifyErrorKind::TypeMismatch);
        }
    }

    fn verify_checked_fixed(
        &mut self,
        path: &str,
        instruction: &Instruction,
        available: &BTreeMap<ValueId, Type>,
        left: ValueId,
        right: ValueId,
    ) {
        let Some(operand) = matching_fixed_operands(available.get(&left), available.get(&right))
        else {
            self.error(path, VerifyErrorKind::TypeMismatch);
            return;
        };
        let expected = Type::Result {
            ok: Box::new(operand.clone()),
            error: Box::new(Type::Named(NUMERIC_ERROR_TYPE.to_owned())),
        };
        if instruction.ty != expected {
            self.error(path, VerifyErrorKind::TypeMismatch);
        }
    }

    fn verify_fixed_comparison(
        &mut self,
        path: &str,
        instruction: &Instruction,
        available: &BTreeMap<ValueId, Type>,
        left: ValueId,
        right: ValueId,
    ) {
        if matching_fixed_operands(available.get(&left), available.get(&right)).is_none()
            || instruction.ty != Type::Bool
        {
            self.error(path, VerifyErrorKind::TypeMismatch);
        }
    }

    fn verify_affine_and_revision(
        &mut self,
        path: &str,
        block: &Block,
        parameters: &BTreeMap<ValueId, Type>,
    ) {
        let mut types = parameters.clone();
        let mut live_owned: BTreeSet<_> = parameters
            .iter()
            .filter_map(|(id, ty)| matches!(ty, Type::OwnedResource(_)).then_some(*id))
            .collect();
        let mut consumed = BTreeSet::new();
        let mut revision_checks = BTreeSet::new();
        for instruction in &block.instructions {
            for operand in instruction.operation.operands() {
                if consumed.contains(&operand) {
                    self.error(path, VerifyErrorKind::ResourceViolation);
                }
            }
            match &instruction.operation {
                Operation::Copy(value)
                    if matches!(
                        types.get(value),
                        Some(Type::OwnedResource(_) | Type::BorrowedResource(_))
                    ) =>
                {
                    self.error(path, VerifyErrorKind::ResourceViolation);
                }
                Operation::ResourceMove(value) => match types.get(value) {
                    Some(Type::OwnedResource(name))
                        if instruction.ty == Type::OwnedResource(name.clone()) =>
                    {
                        consumed.insert(*value);
                        live_owned.remove(value);
                    }
                    _ => self.error(path, VerifyErrorKind::ResourceViolation),
                },
                Operation::ResourceBorrow(value) => match types.get(value) {
                    Some(Type::OwnedResource(name))
                        if instruction.ty == Type::BorrowedResource(name.clone()) => {}
                    _ => self.error(path, VerifyErrorKind::ResourceViolation),
                },
                Operation::ResourceDrop(value) => match types.get(value) {
                    Some(Type::OwnedResource(_)) if instruction.ty == Type::Unit => {
                        consumed.insert(*value);
                        live_owned.remove(value);
                    }
                    _ => self.error(path, VerifyErrorKind::ResourceViolation),
                },
                Operation::RevisionCheck { .. } => {
                    revision_checks.insert(instruction.result);
                }
                _ => {}
            }
            if matches!(instruction.ty, Type::OwnedResource(_)) {
                live_owned.insert(instruction.result);
            }
            types.insert(instruction.result, instruction.ty.clone());
        }
        if let Terminator::Return(Some(value)) = block.terminator {
            match types.get(&value) {
                Some(Type::OwnedResource(_)) => {
                    live_owned.remove(&value);
                }
                Some(Type::BorrowedResource(_)) => self.error(path, VerifyErrorKind::BorrowEscape),
                _ => {}
            }
        }
        if let Terminator::Branch { condition, .. } = block.terminator {
            revision_checks.remove(&condition);
        }
        if !revision_checks.is_empty() {
            self.error(path, VerifyErrorKind::RevisionGuard);
        }
        if !live_owned.is_empty() {
            self.error(path, VerifyErrorKind::ResourceLeak);
        }
    }

    fn verify_task_scope_tables(&mut self) {
        let Some(tables) = &self.module.task_scopes else {
            return;
        };
        for (function_id, scopes) in tables {
            let root = format!("module.task_scopes[{}]", function_id.0);
            if !self.signatures.contains_key(function_id) {
                self.error(&root, VerifyErrorKind::UnknownTarget);
            }
            let mut declared = BTreeSet::new();
            for (index, scope) in scopes.iter().enumerate() {
                let path = format!("{root}[{index}]");
                if scope.scope != u32::try_from(index).unwrap_or(u32::MAX) {
                    self.error(&path, VerifyErrorKind::NonCanonicalId);
                }
                if !declared.insert(scope.scope) {
                    self.error(&path, VerifyErrorKind::DuplicateId);
                }
                // A parent must be declared before its children.
                if scope.parent.is_some_and(|parent| parent >= scope.scope) {
                    self.error(&path, VerifyErrorKind::TaskViolation);
                }
            }
            // Parents are declared before their children, so depths fold in
            // one pass; malformed links are already rejected above and fall
            // back to depth 1 here.
            let mut depths = Vec::with_capacity(scopes.len());
            for (index, scope) in scopes.iter().enumerate() {
                let depth = match scope.parent {
                    Some(parent) if (parent as usize) < index => depths[parent as usize] + 1,
                    _ => 1,
                };
                if depth > MAX_TASK_SCOPE_DEPTH {
                    self.error(format!("{root}[{index}]"), VerifyErrorKind::TaskViolation);
                }
                depths.push(depth);
            }
        }
    }

    /// Task-region and affine-handle discipline (RFC-0036 §5.3): LIFO
    /// scope regions contained in one linear block, scope-qualified
    /// spawn/collect, effect closure, and exactly-once in-region
    /// consumption of `Task` handles.
    #[allow(clippy::too_many_lines)]
    fn verify_task_flow(
        &mut self,
        path: &str,
        block: &Block,
        parameters: &BTreeMap<ValueId, Type>,
        scopes: &BTreeMap<u32, Option<u32>>,
        effects: &[String],
    ) {
        let mut types = parameters.clone();
        // LIFO stack plus membership indexes, so pathological inputs stay
        // near-linear instead of scanning an unbounded open stack per op.
        let mut open: Vec<(u32, u64)> = Vec::new();
        let mut open_scopes: BTreeMap<u32, u64> = BTreeMap::new();
        let mut open_regions = BTreeSet::new();
        let mut region_scope = BTreeMap::new();
        let mut next_region = 0_u64;
        let mut task_region: BTreeMap<ValueId, u64> = BTreeMap::new();
        let mut region_tasks: BTreeMap<u64, Vec<ValueId>> = BTreeMap::new();
        let mut region_lists: BTreeMap<u64, Vec<ValueId>> = BTreeMap::new();
        let mut consumed = BTreeSet::new();
        let mut list_region: BTreeMap<ValueId, Option<u64>> = BTreeMap::new();
        let mut collected = BTreeSet::new();
        for instruction in &block.instructions {
            // A Task handle (or its collect list) may only reach `Await`,
            // the `Construct` building its list, or `TaskCollect`; any
            // other use escapes its scope.
            if !matches!(
                instruction.operation,
                Operation::Await(_) | Operation::Construct { .. } | Operation::TaskCollect { .. }
            ) {
                for operand in instruction.operation.operands() {
                    if task_region.contains_key(&operand) || list_region.contains_key(&operand) {
                        self.error(path, VerifyErrorKind::TaskViolation);
                    }
                }
            }
            match &instruction.operation {
                Operation::TaskScopeOpen { scope } => {
                    if instruction.ty != Type::Unit {
                        self.error(path, VerifyErrorKind::TypeMismatch);
                    }
                    if !scopes.contains_key(scope) || open_scopes.contains_key(scope) {
                        self.error(path, VerifyErrorKind::TaskViolation);
                    } else {
                        open.push((*scope, next_region));
                        open_scopes.insert(*scope, next_region);
                        open_regions.insert(next_region);
                        region_scope.insert(next_region, *scope);
                        next_region += 1;
                    }
                }
                Operation::TaskScopeClose { scope } => {
                    if instruction.ty != Type::Unit {
                        self.error(path, VerifyErrorKind::TypeMismatch);
                    }
                    if !scopes.contains_key(scope)
                        || open.last().map(|(open_scope, _)| open_scope) != Some(scope)
                    {
                        self.error(path, VerifyErrorKind::TaskViolation);
                    } else {
                        let (closed_scope, region) =
                            open.pop().expect("scope stack top was checked");
                        open_scopes.remove(&closed_scope);
                        open_regions.remove(&region);
                        let unconsumed = region_tasks
                            .get(&region)
                            .is_some_and(|tasks| tasks.iter().any(|task| !consumed.contains(task)));
                        if unconsumed {
                            // Every spawned handle must be consumed before
                            // its scope closes (E5103 analogue).
                            self.error(path, VerifyErrorKind::TaskViolation);
                        }
                        if let Some(lists) = region_lists.get(&region) {
                            for list in lists {
                                if !collected.contains(list) {
                                    self.error(path, VerifyErrorKind::TaskViolation);
                                }
                            }
                        }
                    }
                }
                Operation::Spawn {
                    scope,
                    callee,
                    arguments,
                } => {
                    let region = open_scopes.get(scope).copied();
                    if !scopes.contains_key(scope) || region.is_none() {
                        // Spawn requires a declared, currently open scope
                        // (E5104 analogue).
                        self.error(path, VerifyErrorKind::TaskViolation);
                    }
                    match self.signatures.get(callee).copied() {
                        None => self.error(path, VerifyErrorKind::UnknownTarget),
                        Some((parameters, return_type, callee_effects)) => {
                            if let Type::Future(inner) = return_type {
                                if instruction.ty != Type::Task(inner.clone()) {
                                    self.error(path, VerifyErrorKind::TypeMismatch);
                                }
                                if parameters.len() == arguments.len() {
                                    for (parameter, argument) in parameters.iter().zip(arguments) {
                                        if let Some(actual) = types.get(argument)
                                            && actual != &parameter.ty
                                        {
                                            self.error(path, VerifyErrorKind::TypeMismatch);
                                        }
                                    }
                                } else {
                                    self.error(path, VerifyErrorKind::TypeMismatch);
                                }
                                // Effect closure: a spawned callee may not
                                // exceed the caller's declared effects.
                                if callee_effects
                                    .iter()
                                    .any(|effect| effects.binary_search(effect).is_err())
                                {
                                    self.error(path, VerifyErrorKind::TaskViolation);
                                }
                            } else {
                                self.error(path, VerifyErrorKind::TaskViolation);
                            }
                        }
                    }
                    if matches!(&instruction.ty, Type::Task(_)) {
                        let region = region.unwrap_or(DETACHED_REGION);
                        task_region.insert(instruction.result, region);
                        region_tasks
                            .entry(region)
                            .or_default()
                            .push(instruction.result);
                    }
                }
                Operation::TaskCollect { scope, tasks } => {
                    if !scopes.contains_key(scope) || !open_scopes.contains_key(scope) {
                        self.error(path, VerifyErrorKind::TaskViolation);
                    }
                    match types.get(tasks) {
                        Some(Type::List(element)) => match element.as_ref() {
                            Type::Task(inner) if instruction.ty == Type::List(inner.clone()) => {}
                            _ => self.error(path, VerifyErrorKind::TaskViolation),
                        },
                        Some(_) => self.error(path, VerifyErrorKind::TaskViolation),
                        // Undefined operands are reported by the generic pass.
                        None => {}
                    }
                    if let Some(owner) = list_region.get(tasks) {
                        if !collected.insert(*tasks) {
                            self.error(path, VerifyErrorKind::TaskViolation);
                        }
                        if let Some(region) = owner
                            && region_scope.get(region) != Some(scope)
                        {
                            self.error(path, VerifyErrorKind::TaskViolation);
                        }
                    }
                }
                Operation::Await(value) => {
                    if let Some(region) = task_region.get(value) {
                        if !consumed.insert(*value) {
                            // A Task handle is affine: exactly one
                            // consumption (E5101 analogue).
                            self.error(path, VerifyErrorKind::TaskViolation);
                        }
                        if !open_regions.contains(region) {
                            self.error(path, VerifyErrorKind::TaskViolation);
                        }
                    }
                }
                Operation::Construct { fields, .. } => {
                    let task_list = matches!(&instruction.ty, Type::List(element) if matches!(element.as_ref(), Type::Task(_)));
                    if task_list {
                        if fields.len() > MAX_TASK_COLLECT_LENGTH {
                            self.error(path, VerifyErrorKind::Limit);
                        }
                        let mut owner = None;
                        let mut mixed = false;
                        for field in fields {
                            let Some(region) = task_region.get(&field.value).copied() else {
                                continue;
                            };
                            if !consumed.insert(field.value) {
                                self.error(path, VerifyErrorKind::TaskViolation);
                            }
                            if !open_regions.contains(&region) {
                                self.error(path, VerifyErrorKind::TaskViolation);
                            }
                            match owner {
                                None => owner = Some(region),
                                Some(owner_region) if owner_region == region => {}
                                Some(_) => mixed = true,
                            }
                        }
                        if mixed {
                            // A collect list may not mix handles from
                            // different scope regions.
                            self.error(path, VerifyErrorKind::TaskViolation);
                        }
                        if let Some(region) = owner {
                            region_lists
                                .entry(region)
                                .or_default()
                                .push(instruction.result);
                        }
                        list_region.insert(instruction.result, owner);
                    } else {
                        // Task handles must not be stored into escaping
                        // aggregates (E5102 analogue).
                        for field in fields {
                            if task_region.contains_key(&field.value) {
                                self.error(path, VerifyErrorKind::TaskViolation);
                            }
                        }
                    }
                }
                _ => {}
            }
            types.insert(instruction.result, instruction.ty.clone());
        }
        match &block.terminator {
            Terminator::Return(Some(value)) if task_region.contains_key(value) => {
                // A Task handle must not leave its scope as a return value.
                self.error(path, VerifyErrorKind::TaskViolation);
            }
            Terminator::Match { values, .. } => {
                for value in values {
                    if task_region.contains_key(value) {
                        self.error(path, VerifyErrorKind::TaskViolation);
                    }
                }
            }
            _ => {}
        }
        if !open.is_empty() {
            // IR v1: a scope region must open and close in one linear block.
            self.error(path, VerifyErrorKind::TaskViolation);
        }
        for (task, region) in &task_region {
            if *region == DETACHED_REGION && !consumed.contains(task) {
                self.error(path, VerifyErrorKind::TaskViolation);
            }
        }
        for (list, owner) in &list_region {
            if owner.is_none() && !collected.contains(list) {
                self.error(path, VerifyErrorKind::TaskViolation);
            }
        }
    }

    fn verify_terminator(
        &mut self,
        path: &str,
        terminator: &Terminator,
        available: &BTreeMap<ValueId, Type>,
        block_ids: &BTreeSet<BlockId>,
        return_type: &Type,
    ) {
        match terminator {
            Terminator::Return(value) => match (return_type, value) {
                (Type::Unit, None) => {}
                // RFC-0036 §5.2: an async function declares `Future[T]` and
                // resolves it by returning the computed `T`; returning an
                // already-`Future[T]` value (async pass-through) is equally
                // valid under representation identity.
                (Type::Future(inner), Some(value)) => match available.get(value) {
                    Some(actual) if actual == inner.as_ref() || actual == return_type => {}
                    Some(_) => self.error(path, VerifyErrorKind::TypeMismatch),
                    None => self.error(path, VerifyErrorKind::UndefinedValue),
                },
                (_, Some(value)) => self.expect_type(path, available.get(value), return_type),
                _ => self.error(path, VerifyErrorKind::TypeMismatch),
            },
            Terminator::Jump(target) => {
                if !block_ids.contains(target) {
                    self.error(path, VerifyErrorKind::UnknownTarget);
                }
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                self.expect_type(path, available.get(condition), &Type::Bool);
                if !block_ids.contains(then_block) || !block_ids.contains(else_block) {
                    self.error(path, VerifyErrorKind::UnknownTarget);
                }
            }
            Terminator::Match { values, arms } => {
                for value in values {
                    if !available.contains_key(value) {
                        self.error(path, VerifyErrorKind::UndefinedValue);
                    }
                }
                if arms.is_empty() {
                    self.error(path, VerifyErrorKind::TypeMismatch);
                }
                for arm in arms {
                    if arm.patterns.len() != values.len() {
                        self.error(path, VerifyErrorKind::TypeMismatch);
                    }
                    if !block_ids.contains(&arm.target) {
                        self.error(path, VerifyErrorKind::UnknownTarget);
                    }
                    self.range(path, arm.range);
                    for (pattern, value) in arm.patterns.iter().zip(values) {
                        let compatible = match pattern {
                            Pattern::Wildcard | Pattern::Binding(_) => true,
                            Pattern::Bool(_) => available.get(value) == Some(&Type::Bool),
                            Pattern::Variant { .. } => matches!(
                                available.get(value),
                                Some(Type::Named(_) | Type::Option(_) | Type::Result { .. })
                            ),
                        };
                        if !compatible {
                            self.error(path, VerifyErrorKind::TypeMismatch);
                        }
                    }
                }
            }
            Terminator::Unreachable => {}
        }
    }

    fn expect_type(&mut self, path: &str, actual: Option<&Type>, expected: &Type) {
        match actual {
            None => self.error(path, VerifyErrorKind::UndefinedValue),
            Some(actual) if actual != expected => self.error(path, VerifyErrorKind::TypeMismatch),
            Some(_) => {}
        }
    }

    fn range(&mut self, path: &str, range: SourceRange) {
        if range.start > range.end || range.end > self.module.source_len {
            self.error(path, VerifyErrorKind::InvalidRange);
        }
    }

    fn error(&mut self, path: impl Into<String>, kind: VerifyErrorKind) {
        if self.errors.len() < MAX_IR_DIAGNOSTICS {
            self.errors.push(VerifyError {
                path: path.into(),
                kind,
            });
        }
    }
}

fn matching_fixed_operands<'a>(left: Option<&'a Type>, right: Option<&Type>) -> Option<&'a Type> {
    match (left, right) {
        (Some(left @ (Type::I64 | Type::U64)), Some(right)) if left == right => Some(left),
        _ => None,
    }
}

/// Region instance that no open task scope owns; handles tied to it can
/// never be consumed legally.
const DETACHED_REGION: u64 = u64::MAX;

/// RFC-0039 §2.4 (STEP-0147): source identifiers map to boundary
/// kebab-case — `Csv` -> `csv`, `TableStats` -> `table-stats`,
/// `parse_line` -> `parse-line`. Deterministic and injective: source
/// names cannot contain dashes, so no two source names collide.
#[must_use]
pub fn boundary_kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index != 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_' {
            out.push('-');
        } else {
            out.push(ch);
        }
    }
    out
}

/// The closed, versioned Script standard-library intrinsic registry
/// (STEP-0083). Every signature is pinned; unknown names are rejected.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn intrinsic_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    let result_of = |ok: Type| Type::Result {
        ok: Box::new(ok),
        error: Box::new(Type::Named(NUMERIC_ERROR_TYPE.to_owned())),
    };
    let list_of = |element: Type| Type::List(Box::new(element));
    let (parameters, result) = match name {
        "Float64.from_int" => (vec![Type::Int], Type::Float64),
        "sico.bytes.length" => (vec![Type::Bytes], Type::U64),
        "sico.bytes.concat" => (vec![Type::Bytes, Type::Bytes], Type::Bytes),
        "sico.bytes.slice" => (
            vec![Type::Bytes, Type::U64, Type::U64],
            result_of(Type::Bytes),
        ),
        "sico.bytes.is_utf8" => (vec![Type::Bytes], Type::Bool),
        "sico.bytes.utf8_decode" => (vec![Type::Bytes], Type::String),
        "sico.text.encode" => (vec![Type::String], Type::Bytes),
        "sico.text.length" => (vec![Type::String], Type::U64),
        "sico.text.concat" | "sico.json.get" => (vec![Type::String, Type::String], Type::String),
        "sico.text.trim" | "sico.json.quote" => (vec![Type::String], Type::String),
        "sico.text.contains" | "sico.text.starts_with" => {
            (vec![Type::String, Type::String], Type::Bool)
        }
        "sico.text.split_lines" | "sico.text.split_words" => {
            (vec![Type::String], list_of(Type::String))
        }
        "sico.text.replace" => (vec![Type::String, Type::String, Type::String], Type::String),
        "sico.text.join" => (vec![list_of(Type::String), Type::String], Type::String),
        "sico.json.is_valid" => (vec![Type::String], Type::Bool),

        "sico.json.has" => (vec![Type::String, Type::String], Type::Bool),

        "sico.list.length" => (vec![list_of(Type::String)], Type::U64),
        "sico.list.get" => (
            vec![list_of(Type::String), Type::U64],
            result_of(Type::String),
        ),
        "sico.list.append" => (
            vec![list_of(Type::String), Type::String],
            list_of(Type::String),
        ),
        "sico.u64.to_text" => (vec![Type::U64], Type::String),
        "sico.i64.to_text" => (vec![Type::I64], Type::String),
        "sico.fs.read" => (
            vec![Type::String],
            Type::Result {
                ok: Box::new(Type::Bytes),
                error: Box::new(Type::String),
            },
        ),
        "sico.fs.exists" => (
            vec![Type::String],
            Type::Result {
                ok: Box::new(Type::Bool),
                error: Box::new(Type::String),
            },
        ),
        "sico.fs.write" => (
            vec![Type::String, Type::Bytes],
            Type::Result {
                ok: Box::new(Type::Bool),
                error: Box::new(Type::String),
            },
        ),
        "sico.http.request" => (
            vec![Type::String, Type::String, Type::Bytes],
            Type::Result {
                ok: Box::new(Type::Named("HttpResponse".to_owned())),
                error: Box::new(Type::String),
            },
        ),
        // STEP-0136: buffered one-shot over `sico:script/http@0.2.0` with
        // Host-default options; the typed http-error enum surfaces as its
        // case-name Text (guest-side static table, no Host text crosses).
        "sico.http2.request" => (
            vec![Type::String, Type::String, Type::Bytes],
            Type::Result {
                ok: Box::new(Type::Named("Http2Response".to_owned())),
                error: Box::new(Type::String),
            },
        ),
        "sico.stream.stdin" => (vec![], Type::Named("InputStream".to_owned())),
        "sico.stream.stdout" | "sico.stream.stderr" => {
            (vec![], Type::Named("OutputStream".to_owned()))
        }
        "sico.stream.read" => (
            vec![Type::Named("InputStream".to_owned()), Type::U64],
            Type::Result {
                ok: Box::new(Type::Bytes),
                error: Box::new(Type::String),
            },
        ),
        "sico.stream.write" => (
            vec![Type::Named("OutputStream".to_owned()), Type::Bytes],
            Type::Result {
                ok: Box::new(Type::Bool),
                error: Box::new(Type::String),
            },
        ),
        "sico.stream.flush" => (
            vec![Type::Named("OutputStream".to_owned())],
            Type::Result {
                ok: Box::new(Type::Bool),
                error: Box::new(Type::String),
            },
        ),
        "sico.stream.pump" => (
            vec![
                Type::Named("InputStream".to_owned()),
                Type::Named("OutputStream".to_owned()),
            ],
            Type::Result {
                ok: Box::new(Type::U64),
                error: Box::new(Type::String),
            },
        ),
        "sico.stream.close_input" => (vec![Type::Named("InputStream".to_owned())], Type::Bool),
        "sico.stream.close_output" => (vec![Type::Named("OutputStream".to_owned())], Type::Bool),
        _ => match collection_intrinsic(name) {
            Some(CollectionIntrinsic {
                operation,
                key,
                value,
            }) => collection_signature(operation, key, value),
            None => return None,
        },
    };
    Some((parameters, result))
}

/// One scalar collection element kind accepted by the STEP-0131 surface:
/// the five fixed-width comparable types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionElement {
    Text,
    Bytes,
    Bool,
    I64,
    U64,
}

impl CollectionElement {
    #[must_use]
    pub fn to_type(self) -> Type {
        match self {
            Self::Text => Type::String,
            Self::Bytes => Type::Bytes,
            Self::Bool => Type::Bool,
            Self::I64 => Type::I64,
            Self::U64 => Type::U64,
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "Text" => Some(Self::Text),
            "Bytes" => Some(Self::Bytes),
            "Bool" => Some(Self::Bool),
            "I64" => Some(Self::I64),
            "U64" => Some(Self::U64),
            _ => None,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Bytes => "Bytes",
            Self::Bool => "Bool",
            Self::I64 => "I64",
            Self::U64 => "U64",
        }
    }
}

/// One parsed `sico.map.*`/`sico.set.*` intrinsic with its instantiation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionIntrinsic {
    pub operation: CollectionOperation,
    pub key: CollectionElement,
    pub value: Option<CollectionElement>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionOperation {
    MapEmpty,
    MapPut,
    MapGet,
    MapHas,
    MapLength,
    MapKeys,
    SetEmpty,
    SetAdd,
    SetHas,
    SetLength,
    SetToList,
}

impl CollectionOperation {
    fn parse(family: &str, operation: &str) -> Option<Self> {
        match (family, operation) {
            ("map", "empty") => Some(Self::MapEmpty),
            ("map", "put") => Some(Self::MapPut),
            ("map", "get") => Some(Self::MapGet),
            ("map", "has") => Some(Self::MapHas),
            ("map", "length") => Some(Self::MapLength),
            ("map", "keys") => Some(Self::MapKeys),
            ("set", "empty") => Some(Self::SetEmpty),
            ("set", "add") => Some(Self::SetAdd),
            ("set", "has") => Some(Self::SetHas),
            ("set", "length") => Some(Self::SetLength),
            ("set", "to_list") => Some(Self::SetToList),
            _ => None,
        }
    }

    fn family(self) -> &'static str {
        match self {
            Self::MapEmpty
            | Self::MapPut
            | Self::MapGet
            | Self::MapHas
            | Self::MapLength
            | Self::MapKeys => "map",
            Self::SetEmpty | Self::SetAdd | Self::SetHas | Self::SetLength | Self::SetToList => {
                "set"
            }
        }
    }

    fn operation(self) -> &'static str {
        match self {
            Self::MapPut => "put",
            Self::MapGet => "get",
            Self::MapKeys => "keys",
            Self::SetAdd => "add",
            Self::SetToList => "to_list",
            Self::MapEmpty | Self::SetEmpty => "empty",
            Self::MapHas | Self::SetHas => "has",
            Self::MapLength | Self::SetLength => "length",
        }
    }
}

/// Parses a canonical suffixed collection intrinsic name such as
/// `sico.map.put[Text,I64]` or `sico.set.add[I64]` (M14 STEP-0131). The
/// suffix is the canonical instantiation composed by the lowering; any
/// other spelling (spaces, unknown elements, bare names) is rejected so
/// the registry stays closed.
#[must_use]
pub fn collection_intrinsic(name: &str) -> Option<CollectionIntrinsic> {
    let (path, suffix) = name.split_once('[')?;
    if !suffix.ends_with(']') {
        return None;
    }
    let mut parts = path.split('.');
    let prefix = parts.next()?;
    if prefix != "sico" {
        return None;
    }
    let family = parts.next()?;
    let operation = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let operation = CollectionOperation::parse(family, operation)?;
    let mut elements = suffix[..suffix.len() - 1].split(',');
    let key = CollectionElement::parse(elements.next()?)?;
    let value = match operation {
        CollectionOperation::MapEmpty
        | CollectionOperation::MapPut
        | CollectionOperation::MapGet
        | CollectionOperation::MapHas
        | CollectionOperation::MapLength
        | CollectionOperation::MapKeys => Some(CollectionElement::parse(elements.next()?)?),
        CollectionOperation::SetEmpty
        | CollectionOperation::SetAdd
        | CollectionOperation::SetHas
        | CollectionOperation::SetLength
        | CollectionOperation::SetToList => None,
    };
    if elements.next().is_some() {
        return None;
    }
    // `sico.map.get` is only executable for fixed-width values; the 2-slot
    // `Result[I64|U64, NumericError]` local layout is the one shape the
    // Script profile can materialize today (M14 STEP-0131 matrix row).
    if operation == CollectionOperation::MapGet
        && !matches!(value, Some(CollectionElement::I64 | CollectionElement::U64))
    {
        return None;
    }
    Some(CollectionIntrinsic {
        operation,
        key,
        value,
    })
}

/// Composes the canonical suffixed intrinsic name for one instantiation,
/// e.g. `sico.map.put[Text,I64]` or `sico.set.add[I64]`.
#[must_use]
pub fn collection_intrinsic_name(
    operation: CollectionOperation,
    key: CollectionElement,
    value: Option<CollectionElement>,
) -> String {
    let mut name = String::from("sico.");
    name.push_str(operation.family());
    name.push('.');
    name.push_str(operation.operation());
    name.push('[');
    name.push_str(key.suffix());
    if let Some(value) = value {
        name.push(',');
        name.push_str(value.suffix());
    }
    name.push(']');
    name
}

fn collection_signature(
    operation: CollectionOperation,
    key: CollectionElement,
    value: Option<CollectionElement>,
) -> (Vec<Type>, Type) {
    let key_type = key.to_type();
    let map_of = |element: Option<CollectionElement>| match operation.family() {
        "map" => Type::Map {
            key: Box::new(key_type.clone()),
            value: Box::new(
                element
                    .expect("map operations carry a value element")
                    .to_type(),
            ),
        },
        _ => Type::Set(Box::new(key_type.clone())),
    };
    let result_of = |ok: Type| Type::Result {
        ok: Box::new(ok),
        error: Box::new(Type::Named(NUMERIC_ERROR_TYPE.to_owned())),
    };
    match operation {
        CollectionOperation::MapEmpty | CollectionOperation::SetEmpty => {
            (Vec::new(), map_of(value.or(Some(key))))
        }
        CollectionOperation::MapPut => (
            vec![
                map_of(value),
                key_type.clone(),
                value.expect("map.put carries a value").to_type(),
            ],
            map_of(value),
        ),
        CollectionOperation::MapGet => (
            vec![map_of(value), key_type],
            result_of(value.expect("map.get carries a value").to_type()),
        ),
        CollectionOperation::MapHas | CollectionOperation::SetHas => {
            (vec![map_of(value), key_type], Type::Bool)
        }
        CollectionOperation::MapLength | CollectionOperation::SetLength => {
            (vec![map_of(value)], Type::U64)
        }
        CollectionOperation::MapKeys | CollectionOperation::SetToList => {
            (vec![map_of(value)], Type::List(Box::new(key_type.clone())))
        }
        CollectionOperation::SetAdd => (vec![map_of(value), key_type.clone()], map_of(value)),
    }
}

fn canonical_integer(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && !digits.starts_with('0')
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}
