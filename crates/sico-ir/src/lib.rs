//! Typed, deterministic compiler IR and its independent verifier.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sico_semantics::{Analysis, AnalyzeError, SemanticDiagnostic, analyze};
use sico_source::{SourceFile, TextRange};

mod lower;

pub use lower::{CoreLowerError, lower_core};

pub const SCHEMA: &str = "sico.ir.v0";
pub const MAX_IR_DIAGNOSTICS: usize = 100;
pub const MAX_FUNCTIONS: usize = 10_000;
pub const MAX_BLOCKS_PER_FUNCTION: usize = 100_000;
pub const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 1_000_000;

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
    Float64,
    String,
    Named(String),
    Option(Box<Self>),
    Result { ok: Box<Self>, error: Box<Self> },
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
}

impl Module {
    #[must_use]
    pub fn new(source_name: impl Into<String>, source_len: u32) -> Self {
        Self {
            schema: SCHEMA.to_owned(),
            source_name: source_name.into(),
            source_len,
            functions: Vec::new(),
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
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub range: SourceRange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parameter {
    pub id: ValueId,
    pub name: String,
    pub ty: Type,
    pub range: SourceRange,
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
#[serde(tag = "op", content = "data", rename_all = "snake_case")]
pub enum Operation {
    ConstInt(String),
    ConstBool(bool),
    ConstString(String),
    Copy(ValueId),
    AddInt {
        left: ValueId,
        right: ValueId,
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
        fields: Vec<ValueId>,
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
}

impl Operation {
    fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::ConstInt(_) | Self::ConstBool(_) | Self::ConstString(_) => Vec::new(),
            Self::Copy(value)
            | Self::ResourceMove(value)
            | Self::ResourceBorrow(value)
            | Self::ResourceDrop(value)
            | Self::Try(value)
            | Self::Await(value)
            | Self::StreamNext(value) => vec![*value],
            Self::AddInt { left, right } => vec![*left, *right],
            Self::Call { arguments, .. }
            | Self::Intrinsic { arguments, .. }
            | Self::Construct {
                fields: arguments, ..
            }
            | Self::Variant {
                payload: arguments, ..
            }
            | Self::EffectCall { arguments, .. } => arguments.clone(),
            Self::ResourceCall {
                resource,
                arguments,
                ..
            } => std::iter::once(*resource)
                .chain(arguments.iter().copied())
                .collect(),
            Self::Project { base, .. } => vec![*base],
            Self::RevisionCheck { value, expected } => vec![*value, *expected],
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
    signatures: BTreeMap<FunctionId, (&'a [Parameter], &'a Type)>,
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
                .insert(function.id, (&function.parameters, &function.return_type))
                .is_some()
            {
                self.error(
                    format!("function[{}]", function.id.0),
                    VerifyErrorKind::DuplicateId,
                );
            }
        }
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
        {
            self.error(&root, VerifyErrorKind::Limit);
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
        }
    }

    fn verify_operation(
        &mut self,
        path: &str,
        instruction: &Instruction,
        available: &BTreeMap<ValueId, Type>,
        effects: &[String],
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
            Operation::ConstBool(_) if instruction.ty != Type::Bool => {
                self.error(path, VerifyErrorKind::TypeMismatch);
            }
            Operation::ConstString(_) if instruction.ty != Type::String => {
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
            Operation::Call {
                function,
                arguments,
            } => {
                let Some((parameters, return_type)) = self.signatures.get(function).copied() else {
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
                if name == "Float64.from_int" {
                    if arguments.len() != 1 || instruction.ty != Type::Float64 {
                        self.error(path, VerifyErrorKind::TypeMismatch);
                    } else {
                        self.expect_type(path, available.get(&arguments[0]), &Type::Int);
                    }
                } else {
                    self.error(path, VerifyErrorKind::UnknownTarget);
                }
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

fn canonical_integer(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && !digits.starts_with('0')
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}
