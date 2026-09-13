//! RFC-0039 §2.4 (STEP-0147): user-WIT package Component builder.
//!
//! Builds Components that export one `sico:user/<interface>@<version>`
//! instance whose functions are canonically lifted from a self-contained core
//! module produced by [`build_package_core`]. Everything is emitted
//! programmatically with `wasm-encoder` (no `unsafe`, no external toolchain)
//! so identical inputs yield byte-identical Components.
//!
//! The core-module ABI mirrors the fs/stream transport conventions already
//! frozen in [`crate::canonical`]: one shared 64 MiB linear memory, a bump
//! `realloc`, flat parameters with `(ptr, len)` aggregates, and a
//! caller-allocated return area whenever the result does not fit into one
//! flat slot (`String`/`Bytes`/`List` and `Result` results). `Bool`/`I64`/
//! `U64` results ride the single flat result slot instead.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use sico_ir::Type;
use wasm_encoder::{
    Alias, BlockType, CanonicalFunctionSection, CanonicalOption, CodeSection, Component,
    ComponentAliasSection, ComponentExportKind, ComponentExportSection, ComponentInstanceSection,
    ComponentSectionId, ComponentTypeRef, ComponentTypeSection, ComponentValType, ConstExpr,
    CustomSection, DataSection, DataSegment, DataSegmentMode, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, InstanceSection, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, RawSection, TypeSection,
    ValType,
};

use crate::canonical::{self, MEMORY_PAGES, UserFunction, UserImport};

/// One function of a user-WIT package interface.
///
/// `name` is used verbatim across every boundary (core export, instance
/// export, body lookup), so it must already be kebab-case.
#[derive(Clone, Debug)]
pub struct PackageFunctionSpec {
    pub name: String,
    pub parameters: Vec<Type>,
    pub result: Type,
}

/// Builds the self-contained core module behind one package Component.
///
/// Layout: `memory` (64 MiB, fixed), `realloc` (bump allocator mirroring the
/// fs transport module, including alignment and out-of-bounds traps), then one
/// export per function. Every core signature is `params..., retptr -> ()`
/// except single-flat results (`Bool`/`I64`/`U64`), which ride the flat
/// result slot. `bodies` must contain exactly one body per spec, keyed by the
/// function name; `static_data` is placed at address 0 so bodies can embed
/// absolute string constants (error case names and the like).
///
/// # Panics
///
/// Panics if a parameter/result type is outside the frozen package v0 value
/// set or the `bodies` map does not match `functions` exactly; both are
/// caller-side contract breaks, not runtime conditions.
///
/// Lowered arguments occupy the low guest memory (wasmtime lowers caller
/// `Val`s there), so package heap allocations must start above the
/// [`LOWERED_ARGS_RESERVE`] floor or outputs clobber inputs (observed in
/// the vision corpus as spurious extra column lights).
const LOWERED_ARGS_RESERVE: u32 = 64 * 1024;

/// Builds the self-contained core module behind one package Component
/// (heap base floored at [`LOWERED_ARGS_RESERVE`]).
///
/// # Panics
///
/// Panics if a parameter/result type is outside the frozen package v0 value
/// set or the `bodies` map does not match `functions` exactly; both are
/// caller-side contract breaks, not runtime conditions.
#[must_use]
pub fn build_package_core(
    functions: &[PackageFunctionSpec],
    bodies: &BTreeMap<String, Function>,
    static_data: &[u8],
) -> Vec<u8> {
    let mut sorted: Vec<&PackageFunctionSpec> = functions.iter().collect();
    sorted.sort_by(|left, right| left.name.cmp(&right.name));
    assert_eq!(
        sorted.len(),
        bodies.len(),
        "package core: function/body count mismatch"
    );
    for spec in &sorted {
        assert!(
            bodies.contains_key(&spec.name),
            "package core: missing body for {}",
            spec.name
        );
    }

    let mut types = TypeSection::new();
    // Type 0 is the realloc signature; one type per function follows so the
    // type index always equals the function index.
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    for spec in &sorted {
        let mut params = Vec::new();
        for parameter in &spec.parameters {
            params.extend(package_core_param(parameter));
        }
        types
            .ty()
            .function(params, package_core_result(&spec.result));
    }

    let mut functions_section = FunctionSection::new();
    functions_section.function(0);
    for index in 0..sorted.len() {
        functions_section.function(index_u32(index) + 1);
    }

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: u64::from(MEMORY_PAGES),
        maximum: Some(u64::from(MEMORY_PAGES)),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });

    // The bump pointer starts above the static data AND above a 64 KiB
    // reservation for lowered arguments (see LOWERED_ARGS_RESERVE).
    let heap_base =
        ((u32::try_from(static_data.len()).unwrap_or(u32::MAX) + 7) & !7).max(LOWERED_ARGS_RESERVE);
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(i32::try_from(heap_base).unwrap_or(i32::MAX)),
    );

    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("realloc", ExportKind::Func, 0);
    for (index, spec) in sorted.iter().enumerate() {
        exports.export(&spec.name, ExportKind::Func, index_u32(index) + 1);
    }

    let mut code = CodeSection::new();
    code.function(&package_realloc());
    for spec in &sorted {
        code.function(&bodies[&spec.name]);
    }

    let mut module = Module::new();
    module.section(&types);
    module.section(&functions_section);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    if !static_data.is_empty() {
        let mut data = DataSection::new();
        data.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: &ConstExpr::i32_const(0),
            },
            data: static_data.to_vec(),
        });
        module.section(&data);
    }
    module.finish()
}

/// Builds a Component exporting one `sico:user/<interface>@<version>` instance
/// whose functions are canonically lifted from `core`.
///
/// `core` must come from [`build_package_core`] (or match its ABI exactly).
/// The instance export is ascribed with the interface instance type derived
/// from the same signatures via [`canonical::user_interface_instance`].
/// `version` is the interface major version; component extern names with URL
/// syntax require full semver, so it is rendered as `<version>.0.0`
/// (`sico:user/csv@1` becomes the export name `sico:user/csv@1.0.0`).
///
/// # Panics
///
/// Panics if any signature is outside the frozen package v0 value set (the
/// same gate `user_interface_instance` applies); package signatures are a
/// The canonical interface specification text a package stamp carries:
/// the identity line, then one line per function in sorted order —
/// `name (p1, p2) -> result`. The CLI renders the same form from the
/// source-declared interface; any drift on either side fails the E8016
/// digest gate closed.
#[must_use]
pub fn interface_spec_text(identity: &str, functions: &[PackageFunctionSpec]) -> String {
    fn render(ty: &Type) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::I64 => "s64".into(),
            Type::U64 => "u64".into(),
            Type::String => "string".into(),
            Type::Bytes => "list<u8>".into(),
            Type::Unit => "unit".into(),
            Type::List(inner) => format!("list<{}>", render(inner)),
            Type::Result { ok, error } => {
                let error = render(error);
                if matches!(ok.as_ref(), Type::Unit) {
                    format!("result<_, {error}>")
                } else {
                    format!("result<{}, {error}>", render(ok))
                }
            }
            other => format!("{other:?}"),
        }
    }
    let mut sorted = functions.to_vec();
    sorted.sort_by(|left, right| left.name.cmp(&right.name));
    let mut text = format!(
        "{identity}
"
    );
    for function in sorted {
        let params = function
            .parameters
            .iter()
            .map(render)
            .collect::<Vec<_>>()
            .join(", ");
        let rendered = render(&function.result);
        let params_joined = params;
        text.push_str(&function.name);
        text.push_str(" (");
        text.push_str(&params_joined);
        text.push_str(") -> ");
        text.push_str(&rendered);
        text.push('\n');
    }
    text
}

/// The interface stamp `(identity, digest)` embedded in the
/// `sico:user-interface` custom section (RFC-0039 §2.4 E8016 gate).
#[must_use]
pub fn interface_stamp(identity: &str, functions: &[PackageFunctionSpec]) -> (String, String) {
    let spec = interface_spec_text(identity, functions);
    let digest = format!("{:x}", Sha256::digest(spec.as_bytes()));
    (identity.to_owned(), digest)
}

/// Builds a Component exporting one `sico:user/<interface>@<version>` instance
/// whose functions are canonically lifted from `core`.
///
/// `core` must come from [`build_package_core`] (or match its ABI exactly).
/// The instance export is ascribed with the interface instance type derived
/// from the same signatures via [`canonical::user_interface_instance`].
/// `version` is the interface major version; component extern names with URL
/// syntax require full semver, so it is rendered as `<version>.0.0`
/// (`sico:user/csv@1` becomes the export name `sico:user/csv@1.0.0`).
///
/// # Panics
///
/// Panics if any signature is outside the frozen package v0 value set (the
/// same gate `user_interface_instance` applies); package signatures are a
/// closed grammar, so this is a build-time contract break.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_package_component(
    interface: &str,
    version: u32,
    functions: &[PackageFunctionSpec],
    core: &[u8],
) -> Vec<u8> {
    let full_name = format!("sico:user/{interface}@{version}.0.0");
    let import = UserImport {
        name: full_name.clone(),
        functions: functions
            .iter()
            .map(|spec| UserFunction {
                name: spec.name.clone(),
                parameters: spec.parameters.clone(),
                result: spec.result.clone(),
            })
            .collect(),
    };
    let instance_type = canonical::user_interface_instance(&import).unwrap_or_else(|| {
        panic!("package {full_name}: signature outside the frozen v0 value set")
    });

    // Deterministic (sorted) function order for every index space below.
    let ordered: BTreeMap<&str, &PackageFunctionSpec> = functions
        .iter()
        .map(|spec| (spec.name.as_str(), spec))
        .collect();

    // Component type section: defined types first (discovery order, which is
    // topological), then one func type per function, then the interface
    // instance type used to ascribe the export.
    let mut types = ComponentTypeSection::new();
    let mut defined = (BTreeMap::<String, u32>::new(), 0_u32);
    let mut plans = Vec::with_capacity(ordered.len());
    for spec in ordered.values() {
        let mut params = Vec::with_capacity(spec.parameters.len());
        for (index, parameter) in spec.parameters.iter().enumerate() {
            let value = intern_component_type(parameter, &mut defined, &mut types);
            params.push((format!("p{index}"), value));
        }
        let result = match &spec.result {
            Type::Unit => None,
            other => Some(intern_component_type(other, &mut defined, &mut types)),
        };
        plans.push((params, result));
    }
    let defined_count = defined.1;
    let mut func_type_ids = Vec::with_capacity(plans.len());
    for (params, result) in &plans {
        let mut encoder = types.function();
        encoder.params(params.iter().map(|(name, value)| (name.as_str(), *value)));
        encoder.result(*result);
        func_type_ids.push(defined_count + index_u32(func_type_ids.len()));
    }
    let instance_type_index = defined_count + index_u32(func_type_ids.len());
    types.instance(&instance_type);

    // Core module (raw embed) and its single instantiation.
    let mut component = Component::new();
    component.section(&types);
    component.section(&RawSection {
        id: ComponentSectionId::CoreModule.into(),
        data: core,
    });
    let mut core_instances = InstanceSection::new();
    core_instances.instantiate(0, std::iter::empty::<(&str, ModuleArg)>());
    component.section(&core_instances);

    // Aliases: the shared memory, the realloc, and one core function per
    // package function (core func 0 is realloc, package funcs start at 1).
    let mut aliases = ComponentAliasSection::new();
    aliases.alias(Alias::CoreInstanceExport {
        instance: 0,
        kind: ExportKind::Memory,
        name: "memory",
    });
    aliases.alias(Alias::CoreInstanceExport {
        instance: 0,
        kind: ExportKind::Func,
        name: "realloc",
    });
    for name in ordered.keys() {
        aliases.alias(Alias::CoreInstanceExport {
            instance: 0,
            kind: ExportKind::Func,
            name,
        });
    }
    component.section(&aliases);

    // Canonical lifts: UTF-8 text, shared memory, bump realloc; no PostReturn
    // (the bump allocator intentionally outlives one call).
    let mut canonical = CanonicalFunctionSection::new();
    for (index, type_id) in func_type_ids.iter().enumerate() {
        canonical.lift(
            index_u32(index) + 1,
            *type_id,
            [
                CanonicalOption::UTF8,
                CanonicalOption::Memory(0),
                CanonicalOption::Realloc(0),
            ],
        );
    }
    component.section(&canonical);

    // The exported instance groups the lifted functions under the frozen
    // `sico:user/<interface>@<version>` name.
    let mut instances = ComponentInstanceSection::new();
    instances.export_items(
        ordered
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (*name, ComponentExportKind::Func, index_u32(index))),
    );
    component.section(&instances);

    // The producer stamp binds the artifact to the exact interface
    // specification (identity + canonical text digest).
    let (stamp_identity, stamp_digest) = interface_stamp(&full_name, functions);
    let mut stamp_text = String::new();
    stamp_text.push_str(&stamp_identity);
    stamp_text.push(NL);
    stamp_text.push_str(&stamp_digest);
    stamp_text.push(NL);
    component.section(&CustomSection {
        name: "sico:user-interface".into(),
        data: stamp_text.into_bytes().into(),
    });

    let mut exports = ComponentExportSection::new();
    exports.export(
        full_name.as_str(),
        ComponentExportKind::Instance,
        0,
        Some(ComponentTypeRef::Instance(instance_type_index)),
    );
    component.section(&exports);
    component.finish()
}

/// [`csv_package`] with the caller's interface name (source-facing names
/// are verbatim; the stamp and export identity follow them).
#[must_use]
pub fn csv_package_named(interface: &str) -> Vec<u8> {
    csv_package_core(interface)
}

/// Builds the `sico:user/csv@1` package Component.
///
/// Exports `parse-line(line: string) -> result<list<string>, string>`: an
/// RFC-4180 subset splitter (comma separated, quoted fields, `""` escapes,
/// empty fields; a trailing CRLF is stripped). Fail-closed errors:
/// `unterminated-quote` and `stray-quote`.
#[must_use]
pub fn csv_package() -> Vec<u8> {
    csv_package_core("csv")
}

fn csv_package_core(interface: &str) -> Vec<u8> {
    const UNTERMINATED: (u32, u32) = (0, 18);
    const STRAY: (u32, u32) = (18, 11);
    let mut static_data = Vec::new();
    static_data.extend_from_slice(b"unterminated-quote");
    static_data.extend_from_slice(b"stray-quote");
    debug_assert_eq!(static_data.len(), (STRAY.0 + STRAY.1) as usize);

    let functions = [PackageFunctionSpec {
        name: "parse-line".into(),
        parameters: vec![Type::String],
        result: Type::Result {
            ok: Box::new(Type::List(Box::new(Type::String))),
            error: Box::new(Type::String),
        },
    }];
    let mut bodies = BTreeMap::new();
    bodies.insert(
        "parse-line".into(),
        emit_csv_parse_line(UNTERMINATED, STRAY),
    );
    let core = build_package_core(&functions, &bodies, &static_data);
    build_package_component(interface, 1, &functions, &core)
}

/// [`table_stats_package`] with the caller's interface name.
#[must_use]
pub fn table_stats_package_named(interface: &str) -> Vec<u8> {
    table_stats_package_core(interface)
}

/// Builds the `sico:user/table-stats@1` package Component.
///
/// Exports `aggregate(values: list<string>, op: string)
/// -> result<list<string>, string>` with the frozen operation set `count`,
/// `min`, `max` and `mean` (floor of `sum * 1000 / n`, overflow-checked).
/// Elements are parsed as decimal `s64` inside the package (optional leading
/// `-`, ASCII digits only); the first malformed element fails with
/// `malformed-number@<index>`. The aggregate value is rendered as decimal
/// text. `count` and `unknown-op` short-circuit before parsing. Other
/// errors: `empty-column`, `overflow`.
#[must_use]
pub fn table_stats_package() -> Vec<u8> {
    table_stats_package_core("table-stats")
}

fn table_stats_package_core(interface: &str) -> Vec<u8> {
    const COUNT: (u32, u32) = (0, 5);
    const MIN: (u32, u32) = (5, 3);
    const MAX: (u32, u32) = (8, 3);
    const MEAN: (u32, u32) = (11, 4);
    const UNKNOWN: (u32, u32) = (15, 10);
    const EMPTY: (u32, u32) = (25, 12);
    const OVERFLOW: (u32, u32) = (37, 8);
    const MALFORMED_PREFIX: (u32, u32) = (45, 17);
    let mut static_data = Vec::new();
    for text in [
        "count",
        "min",
        "max",
        "mean",
        "unknown-op",
        "empty-column",
        "overflow",
        "malformed-number@",
    ] {
        static_data.extend_from_slice(text.as_bytes());
    }
    debug_assert_eq!(
        static_data.len(),
        (MALFORMED_PREFIX.0 + MALFORMED_PREFIX.1) as usize,
        "table-stats static string table drifted"
    );

    let functions = [PackageFunctionSpec {
        name: "aggregate".into(),
        parameters: vec![Type::List(Box::new(Type::String)), Type::String],
        result: Type::Result {
            ok: Box::new(Type::List(Box::new(Type::String))),
            error: Box::new(Type::String),
        },
    }];
    let mut bodies = BTreeMap::new();
    bodies.insert(
        "aggregate".into(),
        emit_table_stats_aggregate(
            COUNT,
            MIN,
            MAX,
            MEAN,
            UNKNOWN,
            EMPTY,
            OVERFLOW,
            MALFORMED_PREFIX,
        ),
    );
    let core = build_package_core(&functions, &bodies, &static_data);
    build_package_component(interface, 1, &functions, &core)
}

/// Flattened core parameter slots of one package v0 parameter.
fn package_core_param(ty: &Type) -> Vec<ValType> {
    match ty {
        Type::Bool => vec![ValType::I32],
        Type::I64 | Type::U64 => vec![ValType::I64],
        Type::String | Type::Bytes | Type::List(_) => vec![ValType::I32, ValType::I32],
        other => panic!("package v0 parameter value set: {other:?}"),
    }
}

/// Core result slots of one package v0 result shape.
///
/// Single-flat results (`Bool`/`I64`/`U64`) ride the flat result slot; `Unit`
/// has no result. Every aggregate or `Result` shape exceeds the Canonical ABI
/// flat-result budget, so the core function returns one `i32`: the pointer to
/// the return area the callee allocated through its own `realloc`.
fn package_core_result(result: &Type) -> Vec<ValType> {
    match result {
        Type::I64 | Type::U64 => vec![ValType::I64],
        Type::Unit => Vec::new(),
        // `Bool` rides the single flat slot; the aggregate shapes exceed the
        // flat budget and return the return-area pointer instead. Both are
        // one `i32` core result.
        Type::Bool | Type::String | Type::Bytes | Type::List(_) | Type::Result { .. } => {
            vec![ValType::I32]
        }
        other => panic!("package v0 result value set: {other:?}"),
    }
}

/// Bump allocator mirroring the frozen fs transport `realloc`: aligned bump,
/// wrap and ceiling traps, no per-call reclaim.
fn package_realloc() -> Function {
    let mut realloc = Function::new(vec![(2, ValType::I32)]);
    for instruction in [
        Instruction::GlobalGet(0),
        Instruction::LocalGet(2),
        Instruction::I32Add,
        Instruction::I32Const(-1),
        Instruction::I32Add,
        Instruction::I32Const(0),
        Instruction::LocalGet(2),
        Instruction::I32Sub,
        Instruction::I32And,
        Instruction::LocalTee(4),
        Instruction::LocalGet(3),
        Instruction::I32Add,
        Instruction::LocalTee(5),
        Instruction::LocalGet(4),
        Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::Unreachable,
        Instruction::End,
        Instruction::LocalGet(5),
        Instruction::I32Const(i32::try_from(canonical::ARENA_LIMIT).unwrap_or(i32::MAX)),
        Instruction::I32GtU,
        Instruction::If(BlockType::Empty),
        Instruction::Unreachable,
        Instruction::End,
        Instruction::LocalGet(5),
        Instruction::GlobalSet(0),
        Instruction::LocalGet(4),
        Instruction::End,
    ] {
        realloc.instruction(&instruction);
    }
    realloc
}

/// Interns one package v0 type as a component type, returning its reference.
/// Defined types (lists, results) are appended to the component type section
/// in discovery order (inner before outer) and deduplicated by shape.
fn intern_component_type(
    ty: &Type,
    defined: &mut (BTreeMap<String, u32>, u32),
    types: &mut ComponentTypeSection,
) -> ComponentValType {
    match ty {
        Type::Bool => ComponentValType::Primitive(PrimitiveValType::Bool),
        Type::I64 => ComponentValType::Primitive(PrimitiveValType::S64),
        Type::U64 => ComponentValType::Primitive(PrimitiveValType::U64),
        Type::String => ComponentValType::Primitive(PrimitiveValType::String),
        Type::Bytes | Type::List(_) | Type::Result { .. } => {
            let key = format!("{ty:?}");
            if let Some(id) = defined.0.get(&key) {
                return ComponentValType::Type(*id);
            }
            match ty {
                Type::Bytes => {
                    types
                        .defined_type()
                        .list(ComponentValType::Primitive(PrimitiveValType::U8));
                }
                Type::List(inner) => {
                    let inner = intern_component_type(inner, defined, types);
                    types.defined_type().list(inner);
                }
                Type::Result { ok, error } => {
                    let ok = match ok.as_ref() {
                        Type::Unit => None,
                        other => Some(intern_component_type(other, defined, types)),
                    };
                    let error = intern_component_type(error, defined, types);
                    types.defined_type().result(ok, Some(error));
                }
                _ => unreachable!("outer match guarded the defined-type shapes"),
            }
            let id = defined.1;
            defined.0.insert(key, id);
            defined.1 += 1;
            ComponentValType::Type(id)
        }
        other => unreachable!("package v0 value set gate: {other:?}"),
    }
}

/// Appends one instruction to a function body.
#[allow(clippy::needless_pass_by_value)]
fn push(body: &mut Function, instruction: Instruction) {
    body.instruction(&instruction);
}

/// Package index spaces are bounded by the module size limit; this conversion
/// cannot fail for any constructible module.
const NL: char = '\n';

fn index_u32(value: usize) -> u32 {
    u32::try_from(value).expect("package index space fits u32")
}

/// Static data offsets and lengths fit `i32` by construction (the static
/// table is a handful of short strings); signed constants need this view.
fn const_i32(value: u32) -> i32 {
    i32::try_from(value).expect("static data offsets fit i32")
}

/// Loads/stores against the single shared linear memory.
fn load8() -> MemArg {
    MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }
}

fn store8() -> MemArg {
    MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }
}

fn store32(offset: u64) -> MemArg {
    MemArg {
        offset,
        align: 2,
        memory_index: 0,
    }
}

fn store64(offset: u64) -> MemArg {
    MemArg {
        offset,
        align: 3,
        memory_index: 0,
    }
}

// Core locals start after the core parameters: `(line_ptr, line_len)` means
// the first declared local is index 2.
const CSV_LOCAL_I: u32 = 2;
const CSV_LOCAL_LIM: u32 = 3;
const CSV_LOCAL_COUNT: u32 = 4;
const CSV_LOCAL_TOTAL: u32 = 5;
const CSV_LOCAL_C: u32 = 6;
const CSV_LOCAL_LIST: u32 = 7;
const CSV_LOCAL_STRB: u32 = 8;
const CSV_LOCAL_DST: u32 = 9;
const CSV_LOCAL_FI: u32 = 10;
const CSV_LOCAL_FPTR: u32 = 11;
const CSV_LOCAL_ERR: u32 = 12;
const CSV_LOCAL_STATE: u32 = 13;
const CSV_LOCAL_T1: u32 = 14;
const CSV_LOCAL_AREA: u32 = 15;

/// Emits `parse-line(line: string) -> result<list<string>, string>`.
///
/// Core signature: `(line_ptr, line_len) -> retptr`. The body runs a
/// deterministic two-pass byte state machine: pass one counts fields and
/// content bytes and validates quotes (fail-closed), pass two allocates the
/// list and content buffers and copies the unescaped field bytes. The core
/// result is the pointer to the callee-allocated 12-byte return area:
/// discriminant at byte 0, `(ptr, len)` payload of the ok list or the error
/// text at bytes 4/8.
#[allow(clippy::too_many_lines)]
fn emit_csv_parse_line(unterminated: (u32, u32), stray: (u32, u32)) -> Function {
    // Params: 0=line_ptr, 1=line_len; locals 3..=16 are i32.
    let mut body = Function::new(vec![(14, ValType::I32)]);

    // Local init: i=0, lim=line_len, count=0, total=0, err=0.
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_I));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_COUNT));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_TOTAL));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_ERR));

    // Strip a trailing "\n" and then a trailing "\r" (CRLF suffix).
    push(&mut body, Instruction::Block(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Eqz);
    push(&mut body, Instruction::BrIf(0));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Sub);
    push(&mut body, Instruction::LocalGet(0));
    push(&mut body, Instruction::I32Add);
    push(&mut body, Instruction::I32Load8U(load8()));
    push(&mut body, Instruction::I32Const(0x0A));
    push(&mut body, Instruction::I32Ne);
    push(&mut body, Instruction::BrIf(0));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Sub);
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Eqz);
    push(&mut body, Instruction::BrIf(0));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Sub);
    push(&mut body, Instruction::LocalGet(0));
    push(&mut body, Instruction::I32Add);
    push(&mut body, Instruction::I32Load8U(load8()));
    push(&mut body, Instruction::I32Const(0x0D));
    push(&mut body, Instruction::I32Ne);
    push(&mut body, Instruction::BrIf(0));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Sub);
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_LIM));
    push(&mut body, Instruction::End);

    // Pass 1: validate and count (no allocations, no memory writes).
    emit_csv_scan(&mut body, false);
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_ERR));
    push(&mut body, Instruction::If(BlockType::Empty));
    emit_csv_error(&mut body, unterminated, stray);
    push(&mut body, Instruction::Return);
    push(&mut body, Instruction::End);

    // Allocate the list buffer (8 bytes per field) and the content buffer.
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(4));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_COUNT));
    push(&mut body, Instruction::I32Const(3));
    push(&mut body, Instruction::I32Shl);
    push(&mut body, Instruction::Call(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_LIST));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_TOTAL));
    push(&mut body, Instruction::Call(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_STRB));

    // Reset the cursor set for pass 2: dst=strb, fi=0, i=0, count=0,
    // total=0 (pass 2 recounts while it copies).
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_STRB));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_DST));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_FI));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_I));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_COUNT));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_TOTAL));

    // Pass 2: allocation-free copy of every validated field.
    emit_csv_scan(&mut body, true);
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_ERR));
    push(&mut body, Instruction::If(BlockType::Empty));
    emit_csv_error(&mut body, unterminated, stray);
    push(&mut body, Instruction::Return);
    push(&mut body, Instruction::End);

    // Ok area: allocate, disc=0, list pointer @4, field count @8, return it.
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(4));
    push(&mut body, Instruction::I32Const(12));
    push(&mut body, Instruction::Call(0));
    push(&mut body, Instruction::LocalSet(CSV_LOCAL_AREA));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Store8(store8()));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_LIST));
    push(&mut body, Instruction::I32Store(store32(4)));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_COUNT));
    push(&mut body, Instruction::I32Store(store32(8)));
    push(&mut body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(&mut body, Instruction::End);
    body
}

/// Writes the error variant into a freshly allocated 12-byte return area and
/// leaves its pointer as the core function result. `err` holds 1 for
/// `unterminated` and 2 for `stray`.
fn emit_csv_error(body: &mut Function, unterminated: (u32, u32), stray: (u32, u32)) {
    // area = realloc(0, 0, 4, 12)
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(4));
    push(body, Instruction::I32Const(12));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(CSV_LOCAL_AREA));
    push(body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Store8(store8()));
    // Payload pointer, selected by the error code.
    push(body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(body, Instruction::LocalGet(CSV_LOCAL_ERR));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Eq);
    push(body, Instruction::If(BlockType::Result(ValType::I32)));
    push(body, Instruction::I32Const(const_i32(unterminated.0)));
    push(body, Instruction::Else);
    push(body, Instruction::I32Const(const_i32(stray.0)));
    push(body, Instruction::End);
    push(body, Instruction::I32Store(store32(4)));
    // Payload length, selected the same way.
    push(body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(body, Instruction::LocalGet(CSV_LOCAL_ERR));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Eq);
    push(body, Instruction::If(BlockType::Result(ValType::I32)));
    push(body, Instruction::I32Const(const_i32(unterminated.1)));
    push(body, Instruction::Else);
    push(body, Instruction::I32Const(const_i32(stray.1)));
    push(body, Instruction::End);
    push(body, Instruction::I32Store(store32(8)));
    // Return the return-area pointer as the core result.
    push(body, Instruction::LocalGet(CSV_LOCAL_AREA));
    push(body, Instruction::Return);
}

/// Emits one full CSV scan pass as `block $fin { loop $run { ... } }`.
///
/// State machine over the input bytes: 0 = field start, 1 = inside quotes,
/// 2 = plain field, 3 = after a closing quote, 4 = done. With `copy` the pass
/// writes unescaped content bytes at the destination cursor and records every
/// field's `(ptr, len)` pair; without it, the pass only counts and validates.
#[allow(clippy::too_many_lines)]
fn emit_csv_scan(body: &mut Function, copy: bool) {
    fn emit(body: &mut Function, instruction: Instruction) {
        push(body, instruction);
    }

    // state = lim == 0 ? 4 : 0
    emit(body, Instruction::LocalGet(CSV_LOCAL_LIM));
    emit(body, Instruction::I32Eqz);
    emit(body, Instruction::If(BlockType::Result(ValType::I32)));
    emit(body, Instruction::I32Const(4));
    emit(body, Instruction::Else);
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::End);
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));

    emit(body, Instruction::Block(BlockType::Empty));
    emit(body, Instruction::Loop(BlockType::Empty));

    // ---- state 0: field start -------------------------------------------
    emit(body, Instruction::LocalGet(CSV_LOCAL_STATE));
    emit(body, Instruction::I32Eqz);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::LocalGet(CSV_LOCAL_DST));
    emit(body, Instruction::LocalSet(CSV_LOCAL_FPTR));
    emit(body, Instruction::LocalGet(CSV_LOCAL_COUNT));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_COUNT));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::LocalGet(CSV_LOCAL_LIM));
    emit(body, Instruction::I32GeU);
    emit(body, Instruction::If(BlockType::Empty));
    if copy {
        emit_csv_record_field(body);
    }
    emit(body, Instruction::I32Const(4));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::Else);
    emit(body, Instruction::LocalGet(0));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::LocalSet(CSV_LOCAL_C));
    emit(body, Instruction::LocalGet(CSV_LOCAL_C));
    emit(body, Instruction::I32Const(0x22));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::Else);
    emit(body, Instruction::I32Const(2));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::End);
    emit(body, Instruction::End);
    emit(body, Instruction::Br(1)); // -> $run (0 = state-0 if, 1 = loop)
    emit(body, Instruction::End); // close the state-0 if

    // ---- state 1: inside quotes -----------------------------------------
    emit(body, Instruction::LocalGet(CSV_LOCAL_STATE));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::LocalGet(CSV_LOCAL_LIM));
    emit(body, Instruction::I32GeU);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::LocalSet(CSV_LOCAL_ERR));
    emit(body, Instruction::Br(3)); // -> $fin (0 = this if, 1 = state if, 2 = loop)
    emit(body, Instruction::End);
    emit(body, Instruction::LocalGet(0));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::LocalSet(CSV_LOCAL_C));
    emit(body, Instruction::LocalGet(CSV_LOCAL_C));
    emit(body, Instruction::I32Const(0x22));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    // escaped = i+1 < lim && mem[i+1] == '"'
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalGet(CSV_LOCAL_LIM));
    emit(body, Instruction::I32LtU);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::LocalGet(0));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::I32Const(0x22));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::LocalSet(CSV_LOCAL_T1));
    emit(body, Instruction::Else);
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::LocalSet(CSV_LOCAL_T1));
    emit(body, Instruction::End);
    emit(body, Instruction::Else);
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::LocalSet(CSV_LOCAL_T1));
    emit(body, Instruction::End);
    emit(body, Instruction::LocalGet(CSV_LOCAL_T1));
    emit(body, Instruction::If(BlockType::Empty));
    // escaped quote: one content byte, cursor +2
    emit(body, Instruction::LocalGet(CSV_LOCAL_TOTAL));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_TOTAL));
    if copy {
        emit_csv_copy_byte(body, 0x22);
    }
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(2));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::Else);
    // closing quote
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(3));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::End);
    emit(body, Instruction::Else);
    // ordinary quoted byte
    emit(body, Instruction::LocalGet(CSV_LOCAL_TOTAL));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_TOTAL));
    if copy {
        emit_csv_copy_local(body, CSV_LOCAL_C);
    }
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::End);
    emit(body, Instruction::Br(1)); // -> $run (0 = state-1 if, 1 = loop)
    emit(body, Instruction::End); // close the state-1 if

    // ---- state 2: plain field -------------------------------------------
    emit(body, Instruction::LocalGet(CSV_LOCAL_STATE));
    emit(body, Instruction::I32Const(2));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::LocalGet(CSV_LOCAL_LIM));
    emit(body, Instruction::I32GeU);
    emit(body, Instruction::If(BlockType::Empty));
    if copy {
        emit_csv_record_field(body);
    }
    emit(body, Instruction::I32Const(4));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::Else);
    emit(body, Instruction::LocalGet(0));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::LocalSet(CSV_LOCAL_C));
    emit(body, Instruction::LocalGet(CSV_LOCAL_C));
    emit(body, Instruction::I32Const(0x22));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::I32Const(2));
    emit(body, Instruction::LocalSet(CSV_LOCAL_ERR));
    emit(body, Instruction::Br(4)); // -> $fin
    emit(body, Instruction::End);
    emit(body, Instruction::LocalGet(CSV_LOCAL_C));
    emit(body, Instruction::I32Const(0x2C));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    if copy {
        emit_csv_record_field(body);
    }
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::Else);
    emit(body, Instruction::LocalGet(CSV_LOCAL_TOTAL));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_TOTAL));
    if copy {
        emit_csv_copy_local(body, CSV_LOCAL_C);
    }
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::End);
    emit(body, Instruction::End);
    emit(body, Instruction::Br(1)); // -> $run (0 = state-2 if, 1 = loop)
    emit(body, Instruction::End); // close the state-2 if

    // ---- state 3: after a closing quote ----------------------------------
    emit(body, Instruction::LocalGet(CSV_LOCAL_STATE));
    emit(body, Instruction::I32Const(3));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::LocalGet(CSV_LOCAL_LIM));
    emit(body, Instruction::I32GeU);
    emit(body, Instruction::If(BlockType::Empty));
    if copy {
        emit_csv_record_field(body);
    }
    emit(body, Instruction::I32Const(4));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::Else);
    emit(body, Instruction::LocalGet(0));
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::LocalSet(CSV_LOCAL_C));
    emit(body, Instruction::LocalGet(CSV_LOCAL_C));
    emit(body, Instruction::I32Const(0x2C));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    if copy {
        emit_csv_record_field(body);
    }
    emit(body, Instruction::LocalGet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(CSV_LOCAL_I));
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::LocalSet(CSV_LOCAL_STATE));
    emit(body, Instruction::Else);
    emit(body, Instruction::I32Const(2));
    emit(body, Instruction::LocalSet(CSV_LOCAL_ERR));
    emit(body, Instruction::Br(4)); // -> $fin
    emit(body, Instruction::End);
    emit(body, Instruction::End);
    emit(body, Instruction::Br(1)); // -> $run (0 = state-3 if, 1 = loop)
    emit(body, Instruction::End); // close the state-3 if

    // ---- state 4: done ----------------------------------------------------
    emit(body, Instruction::Br(1)); // -> $fin
    emit(body, Instruction::End); // loop $run
    emit(body, Instruction::End); // block $fin
}

/// Records one field pair: `mem[list + fi*8] = fptr`, `mem[... + 4] = dst -
/// fptr`, then `fi++`.
fn emit_csv_record_field(body: &mut Function) {
    push(body, Instruction::LocalGet(CSV_LOCAL_LIST));
    push(body, Instruction::LocalGet(CSV_LOCAL_FI));
    push(body, Instruction::I32Const(3));
    push(body, Instruction::I32Shl);
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalGet(CSV_LOCAL_FPTR));
    push(body, Instruction::I32Store(store32(0)));
    push(body, Instruction::LocalGet(CSV_LOCAL_LIST));
    push(body, Instruction::LocalGet(CSV_LOCAL_FI));
    push(body, Instruction::I32Const(3));
    push(body, Instruction::I32Shl);
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalGet(CSV_LOCAL_DST));
    push(body, Instruction::LocalGet(CSV_LOCAL_FPTR));
    push(body, Instruction::I32Sub);
    push(body, Instruction::I32Store(store32(4)));
    push(body, Instruction::LocalGet(CSV_LOCAL_FI));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(CSV_LOCAL_FI));
}

/// Copies the constant byte to `mem[dst]` and advances `dst`.
fn emit_csv_copy_byte(body: &mut Function, byte: i32) {
    push(body, Instruction::LocalGet(CSV_LOCAL_DST));
    push(body, Instruction::I32Const(byte));
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(CSV_LOCAL_DST));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(CSV_LOCAL_DST));
}

/// Copies the byte held in `local` to `mem[dst]` and advances `dst`.
fn emit_csv_copy_local(body: &mut Function, local: u32) {
    push(body, Instruction::LocalGet(CSV_LOCAL_DST));
    push(body, Instruction::LocalGet(local));
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(CSV_LOCAL_DST));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(CSV_LOCAL_DST));
}

// Core locals start after the core parameters: `(vals_ptr, vals_len,
// op_ptr, op_len)` means the first declared local is index 4.
const TS_LOCAL_I: u32 = 4;
const TS_LOCAL_OP: u32 = 5;
const TS_LOCAL_MATCH: u32 = 6;
const TS_LOCAL_K: u32 = 7;
const TS_LOCAL_LST: u32 = 8;
const TS_LOCAL_AREA: u32 = 9;
const TS_LOCAL_EPTR: u32 = 10;
const TS_LOCAL_ELEN: u32 = 11;
const TS_LOCAL_NEG: u32 = 12;
const TS_LOCAL_T1: u32 = 13;
const TS_LOCAL_T2: u32 = 14;
const TS_LOCAL_V: u32 = 15;
const TS_LOCAL_ACC: u32 = 16;
const TS_LOCAL_BEST: u32 = 17;
const TS_LOCAL_NEW: u32 = 18;
const TS_LOCAL_PROD: u32 = 19;
const TS_LOCAL_Q: u32 = 20;
const TS_LOCAL_R: u32 = 21;

/// `floor(i64::MIN / 10)`: accumulator bound for checked decimal parsing.
const TS_PARSE_CUTOFF: i64 = -922_337_203_685_477_580;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn emit_table_stats_aggregate(
    count: (u32, u32),
    min: (u32, u32),
    max: (u32, u32),
    mean: (u32, u32),
    unknown: (u32, u32),
    empty: (u32, u32),
    overflow: (u32, u32),
    malformed_prefix: (u32, u32),
) -> Function {
    // Params: 0=vals_ptr, 1=vals_len, 2=op_ptr, 3=op_len.
    // Locals: 5..=15 i32, 16..=22 i64.
    let mut body = Function::new(vec![(11, ValType::I32), (7, ValType::I64)]);

    // op = -1, then match the frozen operation names.
    push(&mut body, Instruction::I32Const(-1));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_OP));
    emit_ts_match(&mut body, count.0, count.1, 0);
    emit_ts_match(&mut body, min.0, min.1, 1);
    emit_ts_match(&mut body, max.0, max.1, 2);
    emit_ts_match(&mut body, mean.0, mean.1, 3);

    // All ok paths compute one s64 into `best` and branch to the shared
    // decimal-text tail; error paths return directly.
    push(&mut body, Instruction::Block(BlockType::Empty));

    // count -> [count as text]; short-circuits before any element parsing.
    push(&mut body, Instruction::LocalGet(TS_LOCAL_OP));
    push(&mut body, Instruction::I32Eqz);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I64ExtendI32S);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_BEST));
    push(&mut body, Instruction::Br(1)); // -> $done
    push(&mut body, Instruction::End);

    // unknown op; also short-circuits before parsing.
    push(&mut body, Instruction::LocalGet(TS_LOCAL_OP));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32LtS);
    push(&mut body, Instruction::If(BlockType::Empty));
    emit_ts_error(&mut body, unknown);
    push(&mut body, Instruction::Return);
    push(&mut body, Instruction::End);

    // min/max/mean on an empty column (nothing to parse).
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I32Eqz);
    push(&mut body, Instruction::If(BlockType::Empty));
    emit_ts_error(&mut body, empty);
    push(&mut body, Instruction::Return);
    push(&mut body, Instruction::End);

    // Parse every element into a fresh s64 buffer. The buffer doubles as the
    // error-text buffer on the malformed exit (we return immediately then).
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::I32Const(8));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I32Const(3));
    push(&mut body, Instruction::I32Shl);
    push(&mut body, Instruction::Call(0));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_LST));
    emit_ts_parse_all(&mut body, malformed_prefix);

    // min / max over the parsed values (op 1 or 2).
    push(&mut body, Instruction::LocalGet(TS_LOCAL_OP));
    push(&mut body, Instruction::I32Const(3));
    push(&mut body, Instruction::I32LtU);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_I));
    push(&mut body, Instruction::Block(BlockType::Empty));
    push(&mut body, Instruction::Loop(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I32GeU);
    push(&mut body, Instruction::BrIf(1));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_LST));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::I32Const(3));
    push(&mut body, Instruction::I32Shl);
    push(&mut body, Instruction::I32Add);
    push(&mut body, Instruction::I64Load(store64(0)));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::I32Eqz);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_BEST));
    push(&mut body, Instruction::Else);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_OP));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Eq);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_BEST));
    push(&mut body, Instruction::I64LtS);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_BEST));
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_OP));
    push(&mut body, Instruction::I32Const(2));
    push(&mut body, Instruction::I32Eq);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_BEST));
    push(&mut body, Instruction::I64GtS);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_BEST));
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Add);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_I));
    push(&mut body, Instruction::Br(0));
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::Br(1)); // -> $done (0 = the op<3 if)
    push(&mut body, Instruction::End);

    // mean: checked sum, checked *1000, floor division
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_I));
    push(&mut body, Instruction::I64Const(0));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(&mut body, Instruction::Block(BlockType::Empty));
    push(&mut body, Instruction::Loop(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I32GeU);
    push(&mut body, Instruction::BrIf(1));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_LST));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::I32Const(3));
    push(&mut body, Instruction::I32Shl);
    push(&mut body, Instruction::I32Add);
    push(&mut body, Instruction::I64Load(store64(0)));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::I64Add);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_NEW));
    // overflow iff (acc ^ new) & (v ^ new) < 0
    push(&mut body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_NEW));
    push(&mut body, Instruction::I64Xor);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_V));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_NEW));
    push(&mut body, Instruction::I64Xor);
    push(&mut body, Instruction::I64And);
    push(&mut body, Instruction::I64Const(0));
    push(&mut body, Instruction::I64LtS);
    push(&mut body, Instruction::If(BlockType::Empty));
    emit_ts_error(&mut body, overflow);
    push(&mut body, Instruction::Return);
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_NEW));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_I));
    push(&mut body, Instruction::I32Const(1));
    push(&mut body, Instruction::I32Add);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_I));
    push(&mut body, Instruction::Br(0));
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::End);
    // prod = acc * 1000, checked by inverting the multiplication
    push(&mut body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(&mut body, Instruction::I64Const(1000));
    push(&mut body, Instruction::I64Mul);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_PROD));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_PROD));
    push(&mut body, Instruction::I64Const(1000));
    push(&mut body, Instruction::I64DivS);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(&mut body, Instruction::I64Ne);
    push(&mut body, Instruction::If(BlockType::Empty));
    emit_ts_error(&mut body, overflow);
    push(&mut body, Instruction::Return);
    push(&mut body, Instruction::End);
    // q = prod / n (n >= 1, no trap); r = prod - q * n
    push(&mut body, Instruction::LocalGet(TS_LOCAL_PROD));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I64ExtendI32S);
    push(&mut body, Instruction::I64DivS);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_Q));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_PROD));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_Q));
    push(&mut body, Instruction::LocalGet(1));
    push(&mut body, Instruction::I64ExtendI32S);
    push(&mut body, Instruction::I64Mul);
    push(&mut body, Instruction::I64Sub);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_R));
    // floor: r < 0 (n > 0 always) lowers the truncation by one
    push(&mut body, Instruction::LocalGet(TS_LOCAL_R));
    push(&mut body, Instruction::I64Const(0));
    push(&mut body, Instruction::I64LtS);
    push(&mut body, Instruction::If(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(TS_LOCAL_Q));
    push(&mut body, Instruction::I64Const(1));
    push(&mut body, Instruction::I64Sub);
    push(&mut body, Instruction::LocalSet(TS_LOCAL_Q));
    push(&mut body, Instruction::End);
    push(&mut body, Instruction::LocalGet(TS_LOCAL_Q));
    push(&mut body, Instruction::LocalSet(TS_LOCAL_BEST));
    push(&mut body, Instruction::End); // $done

    // Shared ok tail: format `best` as decimal text inside a one-element
    // list<string> and return it through a fresh return area.
    emit_ts_ok_text(&mut body);
    push(&mut body, Instruction::End);
    body
}

/// Emits the shared ok tail: formats `best` (s64) as decimal ASCII text in a
/// fresh buffer (negative accumulation keeps `i64::MIN` representable),
/// wraps it as a one-element `list<string>` and returns it through a freshly
/// allocated 12-byte return area (disc 0, list ptr @4, length 1 @8). Leaves
/// the area pointer as the core function result.
#[allow(clippy::too_many_lines)]
fn emit_ts_ok_text(body: &mut Function) {
    // text buffer (max: sign + 20 digits; padded, exact length comes below)
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Const(24));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    // neg = best < 0; acc = neg ? best : -best (acc <= 0, i64::MIN safe)
    push(body, Instruction::I32Const(0));
    push(body, Instruction::LocalSet(TS_LOCAL_NEG));
    push(body, Instruction::LocalGet(TS_LOCAL_BEST));
    push(body, Instruction::I64Const(0));
    push(body, Instruction::I64LtS);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_NEG));
    push(body, Instruction::LocalGet(TS_LOCAL_BEST));
    push(body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(body, Instruction::Else);
    push(body, Instruction::I64Const(0));
    push(body, Instruction::LocalGet(TS_LOCAL_BEST));
    push(body, Instruction::I64Sub);
    push(body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(body, Instruction::End);
    // digit count: t2 = 1; v = acc; while v <= -10 { v /= 10; t2 += 1 }
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::LocalSet(TS_LOCAL_V));
    push(body, Instruction::Block(BlockType::Empty));
    push(body, Instruction::Loop(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_V));
    push(body, Instruction::I64Const(-10));
    push(body, Instruction::I64GtS);
    push(body, Instruction::BrIf(1));
    push(body, Instruction::LocalGet(TS_LOCAL_V));
    push(body, Instruction::I64Const(10));
    push(body, Instruction::I64DivS);
    push(body, Instruction::LocalSet(TS_LOCAL_V));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    push(body, Instruction::Br(0));
    push(body, Instruction::End);
    push(body, Instruction::End);
    // optional '-' sign
    push(body, Instruction::LocalGet(TS_LOCAL_NEG));
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::I32Const(0x2D));
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::End);
    // digits right to left: pos = t2 - 1; while pos >= 0 {
    //   q = acc / 10; mem[text + neg + pos] = '0' + (q*10 - acc);
    //   acc = q; pos -= 1 }
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Sub);
    push(body, Instruction::LocalSet(TS_LOCAL_EPTR));
    push(body, Instruction::Block(BlockType::Empty));
    push(body, Instruction::Loop(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_EPTR));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32LtS);
    push(body, Instruction::BrIf(1));
    // address: text + neg + pos
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::LocalGet(TS_LOCAL_NEG));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalGet(TS_LOCAL_EPTR));
    push(body, Instruction::I32Add);
    // value: '0' + (q*10 - acc), the last decimal digit of acc
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Const(10));
    push(body, Instruction::I64DivS);
    push(body, Instruction::LocalSet(TS_LOCAL_R));
    push(body, Instruction::LocalGet(TS_LOCAL_R));
    push(body, Instruction::I64Const(10));
    push(body, Instruction::I64Mul);
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Sub);
    push(body, Instruction::I32WrapI64);
    push(body, Instruction::I32Const(0x30));
    push(body, Instruction::I32Add);
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(TS_LOCAL_R));
    push(body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(body, Instruction::LocalGet(TS_LOCAL_EPTR));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Sub);
    push(body, Instruction::LocalSet(TS_LOCAL_EPTR));
    push(body, Instruction::Br(0));
    push(body, Instruction::End);
    push(body, Instruction::End);
    // text length = digits + sign
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::LocalGet(TS_LOCAL_NEG));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    // list = realloc(0, 0, 4, 8): one (ptr, len) pair
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(4));
    push(body, Instruction::I32Const(8));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(TS_LOCAL_LST));
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::I32Store(store32(0)));
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Store(store32(4)));
    // return area: disc 0, (list, 1) @4/@8; return the area pointer
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(4));
    push(body, Instruction::I32Const(12));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(TS_LOCAL_AREA));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::I32Store(store32(4)));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Store(store32(8)));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::Return);
}

/// Emits the parse pass: for every element, parse its bytes as a decimal
/// `s64` (optional leading `-`, ASCII digits only) into the parsed buffer.
/// The first malformed element raises `malformed-number@<index>` and returns.
///
/// locals used: 4=i, 7=k, 8=lst(parsed buffer, reused as error-text buffer),
/// 9=area, 10=eptr, 11=elen, 12=neg, 13=t1(malformed/temp), 14=t2(temp),
/// 15=v, 16=acc.
#[allow(clippy::too_many_lines)]
fn emit_ts_parse_all(body: &mut Function, malformed_prefix: (u32, u32)) {
    push(body, Instruction::I32Const(0));
    push(body, Instruction::LocalSet(TS_LOCAL_I));
    push(body, Instruction::Block(BlockType::Empty));
    push(body, Instruction::Loop(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_I));
    push(body, Instruction::LocalGet(1));
    push(body, Instruction::I32GeU);
    push(body, Instruction::BrIf(1));
    // Element (ptr, len) pair lives at vals_ptr + 8*i.
    push(body, Instruction::LocalGet(0));
    push(body, Instruction::LocalGet(TS_LOCAL_I));
    push(body, Instruction::I32Const(3));
    push(body, Instruction::I32Shl);
    push(body, Instruction::I32Add);
    push(body, Instruction::I32Load(store32(0)));
    push(body, Instruction::LocalSet(TS_LOCAL_EPTR));
    push(body, Instruction::LocalGet(0));
    push(body, Instruction::LocalGet(TS_LOCAL_I));
    push(body, Instruction::I32Const(3));
    push(body, Instruction::I32Shl);
    push(body, Instruction::I32Add);
    push(body, Instruction::I32Load(store32(4)));
    push(body, Instruction::LocalSet(TS_LOCAL_ELEN));

    // Parse one element; malformed sets t1 = 1 and leaves via block $bad.
    push(body, Instruction::I32Const(0));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::I64Const(0));
    push(body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::LocalSet(TS_LOCAL_NEG));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::LocalSet(TS_LOCAL_K));
    push(body, Instruction::Block(BlockType::Empty)); // $bad
    // empty string is malformed
    push(body, Instruction::LocalGet(TS_LOCAL_ELEN));
    push(body, Instruction::I32Eqz);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(1)); // -> $bad
    push(body, Instruction::End);
    // optional leading '-'
    push(body, Instruction::LocalGet(TS_LOCAL_EPTR));
    push(body, Instruction::I32Load8U(load8()));
    push(body, Instruction::I32Const(0x2D));
    push(body, Instruction::I32Eq);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_NEG));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_K));
    push(body, Instruction::LocalGet(TS_LOCAL_ELEN));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Eq);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(2)); // -> $bad
    push(body, Instruction::End);
    push(body, Instruction::End);
    // digit loop
    push(body, Instruction::Block(BlockType::Empty)); // $digits_done
    push(body, Instruction::Loop(BlockType::Empty)); // $digits
    push(body, Instruction::LocalGet(TS_LOCAL_K));
    push(body, Instruction::LocalGet(TS_LOCAL_ELEN));
    push(body, Instruction::I32GeU);
    push(body, Instruction::BrIf(1)); // -> $digits_done
    push(body, Instruction::LocalGet(TS_LOCAL_EPTR));
    push(body, Instruction::LocalGet(TS_LOCAL_K));
    push(body, Instruction::I32Add);
    push(body, Instruction::I32Load8U(load8()));
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    // c < '0' or c > '9' is malformed (0 = if, 1 = loop, 2 = done, 3 = $bad)
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(0x30));
    push(body, Instruction::I32LtU);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(3)); // -> $bad
    push(body, Instruction::End);
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(0x39));
    push(body, Instruction::I32GtU);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(3)); // -> $bad
    push(body, Instruction::End);
    // checked accumulate: acc*10 - digit stays >= i64::MIN
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Const(TS_PARSE_CUTOFF));
    push(body, Instruction::I64LtS);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(3)); // -> $bad
    push(body, Instruction::End);
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Const(TS_PARSE_CUTOFF));
    push(body, Instruction::I64Eq);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(0x30));
    push(body, Instruction::I32Sub);
    push(body, Instruction::I32Const(8));
    push(body, Instruction::I32GtU);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(4)); // -> $bad
    push(body, Instruction::End);
    push(body, Instruction::End);
    // acc = acc * 10 - digit
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Const(10));
    push(body, Instruction::I64Mul);
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(0x30));
    push(body, Instruction::I32Sub);
    push(body, Instruction::I64ExtendI32S);
    push(body, Instruction::I64Sub);
    push(body, Instruction::LocalSet(TS_LOCAL_ACC));
    push(body, Instruction::LocalGet(TS_LOCAL_K));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(TS_LOCAL_K));
    push(body, Instruction::Br(0));
    push(body, Instruction::End); // $digits
    push(body, Instruction::End); // $digits_done
    // positive numbers must stay <= i64::MAX: acc == i64::MIN is malformed
    push(body, Instruction::LocalGet(TS_LOCAL_NEG));
    push(body, Instruction::I32Eqz);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Const(i64::MIN));
    push(body, Instruction::I64Eq);
    push(body, Instruction::If(BlockType::Empty));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Br(2)); // -> $bad
    push(body, Instruction::End);
    push(body, Instruction::End);
    push(body, Instruction::End); // $bad
    // malformed? report malformed-number@<i>
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::If(BlockType::Empty));
    emit_ts_malformed(body, malformed_prefix, TS_LOCAL_I);
    push(body, Instruction::Return);
    push(body, Instruction::End);
    // v = neg ? acc : -acc (acc accumulates toward negative)
    push(body, Instruction::LocalGet(TS_LOCAL_NEG));
    push(body, Instruction::I32Eqz);
    push(body, Instruction::If(BlockType::Result(ValType::I64)));
    push(body, Instruction::I64Const(0));
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::I64Sub);
    push(body, Instruction::Else);
    push(body, Instruction::LocalGet(TS_LOCAL_ACC));
    push(body, Instruction::End);
    push(body, Instruction::LocalSet(TS_LOCAL_V));
    // parsed[i] = v
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::LocalGet(TS_LOCAL_I));
    push(body, Instruction::I32Const(3));
    push(body, Instruction::I32Shl);
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalGet(TS_LOCAL_V));
    push(body, Instruction::I64Store(store64(0)));
    push(body, Instruction::LocalGet(TS_LOCAL_I));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(TS_LOCAL_I));
    push(body, Instruction::Br(0));
    push(body, Instruction::End); // loop
    push(body, Instruction::End); // block
}

/// Emits the dynamic error `malformed-number@<index>`: builds the text in a
/// fresh buffer (static prefix + decimal index), wraps it in a freshly
/// allocated return area and leaves the area pointer as the core result.
fn emit_ts_malformed(body: &mut Function, prefix: (u32, u32), index_local: u32) {
    // Digit count: nd = 1; t = index; while t >= 10 { t /= 10; nd += 1 }
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    push(body, Instruction::LocalGet(index_local));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::Block(BlockType::Empty));
    push(body, Instruction::Loop(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::I32Const(10));
    push(body, Instruction::I32LtU);
    push(body, Instruction::BrIf(1));
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::I32Const(10));
    push(body, Instruction::I32DivU);
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    push(body, Instruction::Br(0));
    push(body, Instruction::End);
    push(body, Instruction::End);
    // buffer = realloc(0, 0, 1, prefix.len + nd); keep the total length in
    // the spare elen local (the element data is dead on the error exit).
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(const_i32(prefix.1)));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalSet(TS_LOCAL_ELEN));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::LocalGet(TS_LOCAL_ELEN));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(TS_LOCAL_LST));
    // copy the static prefix into the buffer
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::I32Const(const_i32(prefix.0)));
    push(body, Instruction::I32Const(const_i32(prefix.1)));
    push(
        body,
        Instruction::MemoryCopy {
            dst_mem: 0,
            src_mem: 0,
        },
    );
    // write digits least-significant-last: pos = nd - 1; t = index
    push(body, Instruction::LocalGet(index_local));
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Sub);
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    push(body, Instruction::Block(BlockType::Empty));
    push(body, Instruction::Loop(BlockType::Empty));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32LtS);
    push(body, Instruction::BrIf(1));
    // mem[buffer + prefix.len + pos] = '0' + (t % 10)
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::I32Const(const_i32(prefix.1)));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Add);
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::I32Const(10));
    push(body, Instruction::I32RemU);
    push(body, Instruction::I32Const(0x30));
    push(body, Instruction::I32Add);
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(TS_LOCAL_T1));
    push(body, Instruction::I32Const(10));
    push(body, Instruction::I32DivU);
    push(body, Instruction::LocalSet(TS_LOCAL_T1));
    push(body, Instruction::LocalGet(TS_LOCAL_T2));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Sub);
    push(body, Instruction::LocalSet(TS_LOCAL_T2));
    push(body, Instruction::Br(0));
    push(body, Instruction::End);
    push(body, Instruction::End);
    // return area: disc=1, (buffer, prefix.len + nd)
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(4));
    push(body, Instruction::I32Const(12));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(TS_LOCAL_AREA));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::LocalGet(TS_LOCAL_LST));
    push(body, Instruction::I32Store(store32(4)));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::LocalGet(TS_LOCAL_ELEN));
    push(body, Instruction::I32Store(store32(8)));
    // Return the return-area pointer as the core result.
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::Return);
}

/// Emits the equality check of `(op_ptr, op_len)` against the static string
/// at `text` with `len` bytes, setting `op = code` on a match.
fn emit_ts_match(body: &mut Function, text: u32, len: u32, code: i32) {
    fn emit(body: &mut Function, instruction: Instruction) {
        push(body, instruction);
    }

    emit(body, Instruction::Block(BlockType::Empty));
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::LocalSet(TS_LOCAL_MATCH));
    emit(body, Instruction::LocalGet(3));
    emit(body, Instruction::I32Const(const_i32(len)));
    emit(body, Instruction::I32Ne);
    emit(body, Instruction::BrIf(0));
    emit(body, Instruction::I32Const(0));
    emit(body, Instruction::LocalSet(TS_LOCAL_K));
    emit(body, Instruction::Loop(BlockType::Empty));
    emit(body, Instruction::LocalGet(TS_LOCAL_K));
    emit(body, Instruction::I32Const(const_i32(len)));
    emit(body, Instruction::I32Eq);
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::LocalSet(TS_LOCAL_MATCH));
    emit(body, Instruction::Br(2));
    emit(body, Instruction::End);
    emit(body, Instruction::LocalGet(2));
    emit(body, Instruction::LocalGet(TS_LOCAL_K));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::I32Const(const_i32(text)));
    emit(body, Instruction::LocalGet(TS_LOCAL_K));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::I32Load8U(load8()));
    emit(body, Instruction::I32Ne);
    emit(body, Instruction::BrIf(1));
    emit(body, Instruction::LocalGet(TS_LOCAL_K));
    emit(body, Instruction::I32Const(1));
    emit(body, Instruction::I32Add);
    emit(body, Instruction::LocalSet(TS_LOCAL_K));
    emit(body, Instruction::Br(0));
    emit(body, Instruction::End);
    emit(body, Instruction::End);
    emit(body, Instruction::LocalGet(TS_LOCAL_MATCH));
    emit(body, Instruction::If(BlockType::Empty));
    emit(body, Instruction::I32Const(code));
    emit(body, Instruction::LocalSet(TS_LOCAL_OP));
    emit(body, Instruction::End);
}

/// Writes the error variant pointing at the static string `(off, len)` into
/// a freshly allocated 12-byte return area and leaves the area pointer as
/// the core function result.
fn emit_ts_error(body: &mut Function, text: (u32, u32)) {
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(0));
    push(body, Instruction::I32Const(4));
    push(body, Instruction::I32Const(12));
    push(body, Instruction::Call(0));
    push(body, Instruction::LocalSet(TS_LOCAL_AREA));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::I32Const(1));
    push(body, Instruction::I32Store8(store8()));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::I32Const(const_i32(text.0)));
    push(body, Instruction::I32Store(store32(4)));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::I32Const(const_i32(text.1)));
    push(body, Instruction::I32Store(store32(8)));
    push(body, Instruction::LocalGet(TS_LOCAL_AREA));
    push(body, Instruction::Return);
}

// ---------------------------------------------------------------------------
// M17: image-vision@1 — deterministic CV primitives over BGRA8 bitmaps
// (RFC-0041 data contract: packed rows, top-left origin, stride = w*4;
// buffers ride `Bytes` = `list<u8>`). Three operations share one
// parameter shape `(pixels: Bytes, width: u64, height: u64)`:
// - `to-grey8`  -> BT.601 luma bytes, one per pixel,
//                  floor((299r + 587g + 114b) / 1024) (exact-integer);
// - `threshold` -> one byte per PIXEL COUNT entries, 255 when luma >=
//                  threshold (param 2 carries the threshold, param 3 the
//                  pixel count — the loop bound), else 0;
// - `occupancy` -> width bytes, one per column: 255 when the column
//                  holds at least one luma >= 128 pixel, else 0 (tetris
//                  column detector primitive).
// Buffers are caller-validated per RFC-0041 construction rules; the loop
// covers min(width*height, input_len/4) pixels, so truncated inputs
// clamp instead of trapping.
// ---------------------------------------------------------------------------

/// Locals shared by the three vision emitters (all `i32`; pixels are bytes
/// and the luma math stays inside i32).
mod vision {
    pub const W: u32 = 4; // width (param 2 low half)
    pub const H: u32 = 5; // height (param 3 low half)
    pub const LEN: u32 = 6; // pixel count = width * height
    pub const I: u32 = 7; // pixel index
    pub const IDX: u32 = 8; // input byte index
    pub const OUT: u32 = 9; // output buffer pointer
    pub const P: u32 = 10; // pixel word
    pub const LUMA: u32 = 11; // computed luma byte
    pub const T1: u32 = 12;
    pub const T2: u32 = 13;
    pub const T3: u32 = 14;
    pub const AREA_LOCAL: u32 = 15;
}

/// `realloc(0, 0, 1, n)` bump allocation of `n` bytes into local `dst`
/// (`-1` = width*height pixels, computed at runtime).
fn vision_alloc(body: &mut Function, n: i32, dst: u32) {
    if n >= 0 {
        for instruction in [
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(1),
            Instruction::I32Const(n),
            Instruction::Call(0),
            Instruction::LocalSet(dst),
        ] {
            push(body, instruction);
        }
        return;
    }
    // Runtime-sized: len is already in `vision::LEN`.
    for instruction in [
        Instruction::I32Const(0),
        Instruction::I32Const(0),
        Instruction::I32Const(1),
        Instruction::LocalGet(vision::LEN),
        Instruction::Call(0),
        Instruction::LocalSet(dst),
    ] {
        push(body, instruction);
    }
}

/// Builds the `sico:user/image-vision@1` package Component: three pure
/// deterministic CV primitives over BGRA8 byte buffers (RFC-0041 contract;
/// RFC-0043 roster).
#[must_use]
pub fn image_vision_package() -> Vec<u8> {
    let functions = vec![
        PackageFunctionSpec {
            name: "to-grey8".into(),
            parameters: vec![Type::Bytes, Type::U64, Type::U64],
            result: Type::Bytes,
        },
        PackageFunctionSpec {
            name: "threshold".into(),
            parameters: vec![Type::Bytes, Type::U64, Type::U64],
            result: Type::Bytes,
        },
        PackageFunctionSpec {
            name: "occupancy".into(),
            parameters: vec![Type::Bytes, Type::U64, Type::U64],
            result: Type::Bytes,
        },
        PackageFunctionSpec {
            name: "occupancy-mask".into(),
            parameters: vec![Type::Bytes, Type::U64, Type::U64],
            result: Type::Bytes,
        },
    ];
    let mut bodies = BTreeMap::new();
    bodies.insert("to-grey8".into(), emit_vision_loop(VisionOp::Grey8));
    bodies.insert("threshold".into(), emit_vision_loop(VisionOp::Threshold));
    bodies.insert("occupancy".into(), emit_vision_loop(VisionOp::Occupancy));
    bodies.insert(
        "occupancy-mask".into(),
        emit_vision_loop(VisionOp::OccupancyMask),
    );
    let core = build_package_core(&functions, &bodies, &[]);
    build_package_component("image-vision", 1, &functions, &core)
}

/// The three CV operations sharing the per-pixel loop shape.
#[derive(Clone, Copy)]
enum VisionOp {
    Grey8,
    Threshold,
    Occupancy,
    OccupancyMask,
}

/// Shared per-pixel loop shapes. Params: 0/1 = pixels (ptr,len),
/// 2 = width (u64), 3 = height (u64) — for `threshold`, param 3 carries
/// the threshold value instead (single flat param set for the interface).
#[allow(clippy::too_many_lines)]
fn emit_vision_loop(op: VisionOp) -> Function {
    // Locals 0..=3 params; 4.. all i32.
    let mut body = Function::new(vec![(12, ValType::I32)]);

    // width/height ride the low halves of the u64 params.
    push(&mut body, Instruction::LocalGet(2));
    push(&mut body, Instruction::I32WrapI64);
    push(&mut body, Instruction::LocalSet(vision::W));
    push(&mut body, Instruction::LocalGet(3));
    push(&mut body, Instruction::I32WrapI64);
    push(&mut body, Instruction::LocalSet(vision::H));
    // len = width * height
    // LEN (the loop bound): grey8/occupancy = width * height; threshold
    // and occupancy-mask use the height slot as the PIXEL COUNT (their
    // param 2 slot carries the threshold/width respectively).
    match op {
        VisionOp::Threshold | VisionOp::OccupancyMask => {
            push(&mut body, Instruction::LocalGet(vision::H));
            push(&mut body, Instruction::LocalSet(vision::LEN));
        }
        _ => {
            push(&mut body, Instruction::LocalGet(vision::W));
            push(&mut body, Instruction::LocalGet(vision::H));
            push(&mut body, Instruction::I32Mul);
            push(&mut body, Instruction::LocalSet(vision::LEN));
        }
    }

    // Output allocation: grey/threshold = len bytes; occupancy = width bytes.
    match op {
        VisionOp::Grey8 | VisionOp::Threshold => vision_alloc(&mut body, -1, vision::OUT),
        VisionOp::Occupancy | VisionOp::OccupancyMask => {
            push(&mut body, Instruction::LocalGet(vision::W));
            push(&mut body, Instruction::LocalSet(vision::OUT));
        }
    }

    // Pre-init: occupancy-mask needs every column to start at ASCII '0'
    // (fresh memory is NUL). One bounded pass over OUT for the mask ops.
    if matches!(op, VisionOp::OccupancyMask) {
        push(&mut body, Instruction::I32Const(0));
        push(&mut body, Instruction::LocalSet(vision::T2));
        push(&mut body, Instruction::Block(BlockType::Empty));
        push(&mut body, Instruction::Loop(BlockType::Empty));
        push(&mut body, Instruction::LocalGet(vision::T2));
        push(&mut body, Instruction::LocalGet(vision::W));
        push(&mut body, Instruction::I32GeU);
        push(&mut body, Instruction::BrIf(1));
        push(&mut body, Instruction::LocalGet(vision::OUT));
        push(&mut body, Instruction::LocalGet(vision::T2));
        push(&mut body, Instruction::I32Add);
        push(&mut body, Instruction::I32Const(0x30));
        push(&mut body, Instruction::I32Store8(store8()));
        push(&mut body, Instruction::LocalGet(vision::T2));
        push(&mut body, Instruction::I32Const(1));
        push(&mut body, Instruction::I32Add);
        push(&mut body, Instruction::LocalSet(vision::T2));
        push(&mut body, Instruction::Br(0));
        push(&mut body, Instruction::End);
        push(&mut body, Instruction::End);
    }

    // Per-pixel loop over len (occupancy iterates len too, mapping each
    // pixel into its column slot).
    push(&mut body, Instruction::I32Const(0));
    push(&mut body, Instruction::LocalSet(vision::I));
    push(&mut body, Instruction::Block(BlockType::Empty));
    push(&mut body, Instruction::Loop(BlockType::Empty));
    push(&mut body, Instruction::LocalGet(vision::I));
    push(&mut body, Instruction::LocalGet(vision::LEN));
    push(&mut body, Instruction::I32GeU);
    push(&mut body, Instruction::BrIf(1));

    // idx = i * 4; p = little-endian u32 at pixels+idx.
    for instruction in [
        Instruction::LocalGet(vision::I),
        Instruction::I32Const(4),
        Instruction::I32Mul,
        Instruction::LocalSet(vision::IDX),
        Instruction::LocalGet(0),
        Instruction::LocalGet(vision::IDX),
        Instruction::I32Add,
        Instruction::I32Load(load8()),
        Instruction::LocalSet(vision::P),
    ] {
        push(&mut body, instruction);
    }

    // luma = floor((299r + 587g + 114b) / 1024) — BGRA little-endian:
    // byte0 = b, byte1 = g, byte2 = r. (299+587+114 = 1000 < 1024*255,
    // so the i32 math cannot overflow.)
    for instruction in [
        Instruction::LocalGet(vision::P),
        Instruction::I32Const(0xFF),
        Instruction::I32And,
        Instruction::I32Const(114),
        Instruction::I32Mul,
        Instruction::LocalSet(vision::T1),
        Instruction::LocalGet(vision::P),
        Instruction::I32Const(8),
        Instruction::I32ShrU,
        Instruction::I32Const(0xFF),
        Instruction::I32And,
        Instruction::I32Const(587),
        Instruction::I32Mul,
        Instruction::LocalSet(vision::T2),
        Instruction::LocalGet(vision::P),
        Instruction::I32Const(16),
        Instruction::I32ShrU,
        Instruction::I32Const(0xFF),
        Instruction::I32And,
        Instruction::I32Const(299),
        Instruction::I32Mul,
        Instruction::LocalSet(vision::T3),
        Instruction::LocalGet(vision::T1),
        Instruction::LocalGet(vision::T2),
        Instruction::I32Add,
        Instruction::LocalGet(vision::T3),
        Instruction::I32Add,
        Instruction::I32Const(10),
        Instruction::I32ShrU,
        Instruction::LocalSet(vision::LUMA),
    ] {
        push(&mut body, instruction);
    }

    match op {
        VisionOp::Grey8 => {
            for instruction in [
                Instruction::LocalGet(vision::OUT),
                Instruction::LocalGet(vision::I),
                Instruction::I32Add,
                Instruction::LocalGet(vision::LUMA),
                Instruction::I32Store8(store8()),
            ] {
                push(&mut body, instruction);
            }
        }
        VisionOp::Threshold => {
            // The threshold value rides param 2 (the "width" slot).
            for instruction in [
                Instruction::LocalGet(vision::LUMA),
                Instruction::LocalGet(vision::W),
                Instruction::I32GeU,
                Instruction::If(BlockType::Empty),
                Instruction::LocalGet(vision::OUT),
                Instruction::LocalGet(vision::I),
                Instruction::I32Add,
                Instruction::I32Const(255),
                Instruction::I32Store8(store8()),
                Instruction::Else,
                Instruction::LocalGet(vision::OUT),
                Instruction::LocalGet(vision::I),
                Instruction::I32Add,
                Instruction::I32Const(0),
                Instruction::I32Store8(store8()),
                Instruction::End,
            ] {
                push(&mut body, instruction);
            }
        }
        VisionOp::Occupancy => {
            // Column j = i % width; a luma >= 128 pixel lights its column
            // byte (255). Dark pixels take the empty else arm — the buffer
            // starts zeroed, so no store is needed. (No branch here: the
            // shared tail owns the increment, so every path advances.)
            for instruction in [
                Instruction::LocalGet(vision::I),
                Instruction::LocalGet(vision::W),
                Instruction::I32RemU,
                Instruction::LocalSet(vision::T2),
                Instruction::LocalGet(vision::LUMA),
                Instruction::I32Const(128),
                Instruction::I32GeU,
                Instruction::If(BlockType::Empty),
                Instruction::LocalGet(vision::OUT),
                Instruction::LocalGet(vision::T2),
                Instruction::I32Add,
                Instruction::I32Const(255),
                Instruction::I32Store8(store8()),
                Instruction::End,
            ] {
                push(&mut body, instruction);
            }
        }
        VisionOp::OccupancyMask => {
            // Per-column ASCII mask ("1" = any luma >= 128 pixel in the
            // column, else "0"): the tetris reader consumes this through
            // utf8 decode + text ops (v0 has no byte-at intrinsic).
            for instruction in [
                Instruction::LocalGet(vision::I),
                Instruction::LocalGet(vision::W),
                Instruction::I32RemU,
                Instruction::LocalSet(vision::T2),
                // Mark the column first-come: store '1' when bright; the
                // column starts '0' only if never lit — the mask buffer
                // must be pre-initialized to '0'. Zero-initialized memory
                // gives NUL, not '0' — so bright stores '1', and dark
                // pixels store '0' ONLY when the slot is still NUL (0).
                Instruction::LocalGet(vision::LUMA),
                Instruction::I32Const(128),
                Instruction::I32GeU,
                Instruction::If(BlockType::Empty),
                Instruction::LocalGet(vision::OUT),
                Instruction::LocalGet(vision::T2),
                Instruction::I32Add,
                Instruction::I32Const(0x31),
                Instruction::I32Store8(store8()),
                Instruction::End,
            ] {
                push(&mut body, instruction);
            }
        }
    }

    // i += 1; continue; close block, then write the LIFT-direction result
    // area: the callee allocates an 8-byte area and stores the flat
    // `(list_ptr, list_len)` pair — no discriminant on the lift side —
    // and returns the area pointer as the core result. (The 12-byte
    // discriminant form belongs to lowering/caller reads.)
    for instruction in [
        Instruction::LocalGet(vision::I),
        Instruction::I32Const(1),
        Instruction::I32Add,
        Instruction::LocalSet(vision::I),
        Instruction::Br(0),
        Instruction::End,
        Instruction::End,
    ] {
        push(&mut body, instruction);
    }
    // result area = realloc(0,0,4,8); store (out_ptr, out_len).
    for instruction in [
        Instruction::I32Const(0),
        Instruction::I32Const(0),
        Instruction::I32Const(4),
        Instruction::I32Const(8),
        Instruction::Call(0),
        Instruction::LocalSet(vision::AREA_LOCAL),
        Instruction::LocalGet(vision::AREA_LOCAL),
        Instruction::LocalGet(vision::OUT),
        Instruction::I32Store(store32(0)),
        Instruction::LocalGet(vision::AREA_LOCAL),
        Instruction::LocalGet(match op {
            VisionOp::Occupancy | VisionOp::OccupancyMask => vision::W,
            _ => vision::LEN,
        }),
        Instruction::I32Store(store32(4)),
        Instruction::LocalGet(vision::AREA_LOCAL),
        Instruction::End,
    ] {
        push(&mut body, instruction);
    }
    body
}
