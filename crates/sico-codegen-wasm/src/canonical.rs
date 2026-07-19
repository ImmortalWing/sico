//! WIT-driven Script v0 Canonical ABI layout and Component wrapping.
//!
//! The layout table frozen in `docs/development/script-canonical-abi-layout-v0.md`
//! is derived here from the embedded `wit/script-profile-v0/world.wit` text; the
//! backend never hand-codes a byte offset that the WIT does not determine.

use std::collections::BTreeMap;

use wasm_encoder::{
    BlockType, CanonicalOption, CodeSection, ComponentBuilder, ComponentExportKind,
    ComponentTypeRef, ComponentValType, ConstExpr, ExportKind, Function, FunctionSection,
    GlobalSection, GlobalType, InstanceType, Instruction, MemorySection, MemoryType, Module,
    PrimitiveValType, TypeBounds, TypeSection, ValType,
};
use wit_parser::{PackageId, Resolve, SizeAlign, Type, TypeDefKind, TypeId, WorldItem};

pub const SCRIPT_WIT: &str = include_str!("../../../wit/script-profile-v0/world.wit");
pub(crate) const TYPES_INSTANCE: &str = "sico:script/types@0.1.0";
/// RFC-0029 guest linear memory ceiling: 64 MiB = 1,024 Wasm pages.
pub(crate) const ARENA_LIMIT: u32 = 64 * 1024 * 1024;
pub(crate) const MEMORY_PAGES: u32 = 1024;

/// Component import name of the scoped read-side file channel.
pub(crate) const FS_READ_INTERFACE: &str = "sico:script/fs-read@0.1.0";
/// Component import name of the scoped write-side file channel.
pub(crate) const FS_WRITE_INTERFACE: &str = "sico:script/fs-write@0.1.0";
/// Component import name of the M9 streaming channel (RFC-0030).
pub(crate) const STREAMS_INTERFACE: &str = "sico:script/streams@0.1.0";
/// Core module name carrying the shared transport memory and bump allocator
/// for the fs-enabled Script program shape (STEP-0083).
pub(crate) const FS_TRANSPORT_MODULE: &str = "sico:script/fs-transport";
/// Canonical ABI result area shared by every fs call: variant tag at byte 0
/// and the largest payload (`list`/`string` pointer+length) at bytes 4/8.
pub(crate) const FS_RESULT_SIZE: u32 = 12;
/// Canonical ABI result area for `input-stream.read` (same 12-byte shape as
/// fs: tag at 0, payload pointer/length at 4/8, error enum at 4).
pub(crate) const STREAM_READ_RESULT_SIZE: u32 = 12;
/// Canonical ABI result area for `output-stream.write/flush` (tag at 0,
/// error enum at 4).
pub(crate) const STREAM_WRITE_RESULT_SIZE: u32 = 8;
/// Stream error discriminants in WIT declaration order (RFC-0030).
pub(crate) const STREAM_ERROR_TEXTS: &[&str] = &["io", "cancelled", "closed", "resource-limit"];
/// Component import name of the scoped HTTP provider (RFC-0031).
pub(crate) const HTTP_INTERFACE: &str = "sico:script/http@0.1.0";
/// Canonical ABI result area for `http.request` (tag at 0, 16-byte response
/// record or string payload at 8, size 24).
pub(crate) const HTTP_RESULT_SIZE: u32 = 24;

/// Which `sico:script/fs-*@0.1.0` functions one program calls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FsUse {
    pub read: bool,
    pub exists: bool,
    pub write: bool,
}

impl FsUse {
    #[must_use]
    pub fn any(self) -> bool {
        self.read || self.exists || self.write
    }

    fn read_interface(self) -> bool {
        self.read || self.exists
    }

    /// Core import function count: the transport realloc plus one per call.
    #[must_use]
    pub fn import_count(self) -> u32 {
        u32::from(self.any())
            + u32::from(self.read)
            + u32::from(self.exists)
            + u32::from(self.write)
    }
}

/// One frozen fs function import: intrinsic name, Component interface and
/// flattened core parameters (including the trailing result-area pointer).
pub(crate) struct FsImport {
    pub intrinsic: &'static str,
    pub interface: &'static str,
    pub function: &'static str,
    pub core_params: &'static [Flat],
}

/// Frozen fs import table in deterministic core-import order (transport
/// realloc first, then `fs-read` functions, then `fs-write`).
pub(crate) const FS_IMPORTS: &[FsImport] = &[
    FsImport {
        intrinsic: "sico.fs.exists",
        interface: FS_READ_INTERFACE,
        function: "exists",
        core_params: &[Flat::I32, Flat::I32, Flat::I32],
    },
    FsImport {
        intrinsic: "sico.fs.read",
        interface: FS_READ_INTERFACE,
        function: "read",
        core_params: &[Flat::I32, Flat::I32, Flat::I32],
    },
    FsImport {
        intrinsic: "sico.fs.write",
        interface: FS_WRITE_INTERFACE,
        function: "write",
        core_params: &[Flat::I32, Flat::I32, Flat::I32, Flat::I32, Flat::I32],
    },
];

fn fs_used(usage: FsUse, import: &FsImport) -> bool {
    match import.function {
        "read" => usage.read,
        "exists" => usage.exists,
        "write" => usage.write,
        _ => false,
    }
}

/// Which `sico:script/streams@0.1.0` functions one program calls (RFC-0030).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct StreamUse {
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,
    pub read: bool,
    pub write: bool,
    pub flush: bool,
    pub pump: bool,
    pub close_input: bool,
    pub close_output: bool,
}

impl StreamUse {
    #[must_use]
    pub fn any(self) -> bool {
        self.stdin
            || self.stdout
            || self.stderr
            || self.read
            || self.write
            || self.flush
            || self.pump
            || self.close_input
            || self.close_output
    }

    #[must_use]
    pub fn import_count(self) -> u32 {
        u32::from(self.stdin)
            + u32::from(self.stdout)
            + u32::from(self.stderr)
            + u32::from(self.read)
            + u32::from(self.write)
            + u32::from(self.flush)
            + u32::from(self.pump)
            + u32::from(self.close_input)
            + u32::from(self.close_output)
    }
}

/// One frozen streams function import: intrinsic, core field name and
/// flattened core parameters (including the result-area pointer).
pub(crate) struct StreamImport {
    pub intrinsic: &'static str,
    pub function: &'static str,
    pub core_params: &'static [Flat],
    pub core_results: &'static [Flat],
}

/// Frozen streams import table in deterministic core-import order.
pub(crate) const STREAM_IMPORTS: &[StreamImport] = &[
    StreamImport {
        intrinsic: "sico.stream.stdin",
        function: "stdin",
        core_params: &[],
        core_results: &[Flat::I32],
    },
    StreamImport {
        intrinsic: "sico.stream.stdout",
        function: "stdout",
        core_params: &[],
        core_results: &[Flat::I32],
    },
    StreamImport {
        intrinsic: "sico.stream.stderr",
        function: "stderr",
        core_params: &[],
        core_results: &[Flat::I32],
    },
    StreamImport {
        intrinsic: "sico.stream.read",
        function: "read",
        core_params: &[Flat::I32, Flat::I64, Flat::I32],
        core_results: &[],
    },
    StreamImport {
        intrinsic: "sico.stream.write",
        function: "write",
        core_params: &[Flat::I32, Flat::I32, Flat::I32, Flat::I32],
        core_results: &[],
    },
    StreamImport {
        intrinsic: "sico.stream.flush",
        function: "flush",
        core_params: &[Flat::I32, Flat::I32],
        core_results: &[],
    },
    StreamImport {
        intrinsic: "sico.stream.pump",
        function: "pump",
        core_params: &[Flat::I32, Flat::I32, Flat::I32],
        core_results: &[],
    },
    StreamImport {
        intrinsic: "sico.stream.close_input",
        function: "drop-input",
        core_params: &[Flat::I32],
        core_results: &[],
    },
    StreamImport {
        intrinsic: "sico.stream.close_output",
        function: "drop-output",
        core_params: &[Flat::I32],
        core_results: &[],
    },
];

/// The single frozen http import (RFC-0031): method/url/body plus the
/// result-area pointer.
pub(crate) const HTTP_IMPORT: StreamImport = StreamImport {
    intrinsic: "sico.http.request",
    function: "request",
    core_params: &[
        Flat::I32,
        Flat::I32,
        Flat::I32,
        Flat::I32,
        Flat::I32,
        Flat::I32,
        Flat::I32,
    ],
    core_results: &[],
};

pub(crate) fn stream_used(usage: StreamUse, import: &StreamImport) -> bool {
    match import.intrinsic {
        "sico.stream.stdin" => usage.stdin,
        "sico.stream.stdout" => usage.stdout,
        "sico.stream.stderr" => usage.stderr,
        "sico.stream.read" => usage.read,
        "sico.stream.write" => usage.write,
        "sico.stream.flush" => usage.flush,
        "sico.stream.pump" => usage.pump,
        "sico.stream.close_input" => usage.close_input,
        "sico.stream.close_output" => usage.close_output,
        _ => false,
    }
}

/// Flattened Canonical ABI slot kind for a boundary value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Flat {
    I32,
    I64,
}

/// One record field: its Canonical ABI byte offset and its flat-slot range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldLayout {
    pub name: String,
    pub byte_offset: u32,
    pub slot_start: usize,
    pub slot_len: usize,
}

/// Canonical ABI layout of one WIT record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecordLayout {
    pub size: u32,
    pub align: u32,
    pub flat: Vec<Flat>,
    pub fields: Vec<FieldLayout>,
}

impl RecordLayout {}

/// The frozen Script v0 boundary derived from `wit/script-profile-v0`.
pub(crate) struct ScriptAbi {
    pub input: RecordLayout,
    pub output: RecordLayout,
    pub error: RecordLayout,
    /// RFC-0031 `http.response` record layout (status s64, body list<u8>).
    pub http_response: RecordLayout,
    /// `script-error-code` discriminants by bare case name, in declaration order.
    pub error_tags: BTreeMap<String, i32>,
    pub result_size: u32,
    pub result_payload_offset: u32,
}

impl ScriptAbi {
    /// Parses the embedded Script WIT and derives the frozen layout table.
    ///
    /// # Panics
    ///
    /// Panics only if the repository-frozen WIT no longer declares the exact
    /// `sico:script/program@0.1.0` boundary; unit tests pin that contract.
    pub(crate) fn load() -> Self {
        let mut resolve = Resolve::default();
        let package = resolve
            .push_str("world.wit", SCRIPT_WIT)
            .expect("frozen Script WIT must parse");
        let package_name = &resolve.packages[package].name;
        assert!(
            package_name.namespace == "sico"
                && package_name.name == "script"
                && package_name.version.as_ref().is_some_and(|version| {
                    version.major == 0 && version.minor == 1 && version.patch == 0
                }),
            "Script WIT package must be exactly sico:script@0.1.0"
        );
        let interface = resolve.packages[package]
            .interfaces
            .iter()
            .find_map(|(name, id)| (name == "types").then_some(*id))
            .expect("Script WIT must declare the types interface");
        let interface = &resolve.interfaces[interface];
        let type_id = |name: &str| -> TypeId {
            interface
                .types
                .get(name)
                .copied()
                .unwrap_or_else(|| panic!("Script WIT must declare {name}"))
        };
        let input_id = type_id("script-input");
        let output_id = type_id("script-output");
        let error_id = type_id("script-error");
        let code_id = type_id("script-error-code");
        assert!(
            interface.types.len() == 4,
            "Script WIT types interface must stay minimal"
        );

        let mut sizes = SizeAlign::default();
        sizes.fill(&resolve);
        let input = record_layout(&resolve, &sizes, input_id);
        let output = record_layout(&resolve, &sizes, output_id);
        let error = record_layout(&resolve, &sizes, error_id);
        let error_tags = enum_tags(&resolve, code_id);
        let http_response = http_response_layout(&resolve, &sizes, package);

        let (result_size, result_payload_offset) = {
            let payload_align = output.align.max(error.align);
            let payload_offset = align_to(1, payload_align);
            let payload_size = output.size.max(error.size);
            (
                align_to(payload_offset + payload_size, payload_align),
                payload_offset,
            )
        };

        validate_program_world(&resolve, package, input_id, output_id, error_id);

        Self {
            input,
            output,
            error,
            http_response,
            error_tags,
            result_size,
            result_payload_offset,
        }
    }

    /// Flat local slots for one IR type inside a Script program, or `None`
    /// when the type is outside the accepted Script v0 boundary shapes.
    pub(crate) fn flat_ir_types(&self, ty: &sico_ir::Type) -> Option<Vec<Flat>> {
        match ty {
            sico_ir::Type::Bool => Some(vec![Flat::I32]),
            sico_ir::Type::I64 | sico_ir::Type::U64 => Some(vec![Flat::I64]),
            sico_ir::Type::String | sico_ir::Type::Bytes => Some(vec![Flat::I32, Flat::I32]),
            sico_ir::Type::List(element) if element.as_ref() == &sico_ir::Type::String => {
                Some(vec![Flat::I32, Flat::I32])
            }
            sico_ir::Type::Named(name) => match name.as_str() {
                "ScriptInput" => Some(self.input.flat.clone()),
                "ScriptOutput" => Some(self.output.flat.clone()),
                "ScriptError" => Some(self.error.flat.clone()),
                "HttpResponse" => Some(self.http_response.flat.clone()),
                // RFC-0030 stream handles are Canonical ABI resource indices.
                "ScriptErrorCode" | "InputStream" | "OutputStream" => Some(vec![Flat::I32]),
                _ => None,
            },
            sico_ir::Type::Result { ok, error } => {
                let mut flat = vec![Flat::I32];
                flat.extend(self.flat_ir_types(ok)?);
                flat.extend(self.flat_ir_types(error)?);
                Some(flat)
            }
            _ => None,
        }
    }

    /// Record field layout for a named Script record type.
    pub(crate) fn record(&self, name: &str) -> Option<&RecordLayout> {
        match name {
            "ScriptInput" => Some(&self.input),
            "ScriptOutput" => Some(&self.output),
            "ScriptError" => Some(&self.error),
            "HttpResponse" => Some(&self.http_response),
            _ => None,
        }
    }
}

fn http_response_layout(resolve: &Resolve, sizes: &SizeAlign, package: PackageId) -> RecordLayout {
    let http_interface = resolve.packages[package]
        .interfaces
        .iter()
        .find_map(|(name, id)| (name == "http").then_some(*id))
        .expect("Script WIT must declare the http interface");
    let response_id = resolve.interfaces[http_interface]
        .types
        .get("response")
        .copied()
        .expect("Script WIT http must declare response");
    let response = record_layout(resolve, sizes, response_id);
    assert!(
        response.flat == [Flat::I64, Flat::I32, Flat::I32]
            && response.fields[0].name == "status"
            && response.fields[1].name == "body",
        "Script WIT http response must stay status/body"
    );
    response
}

fn validate_program_world(
    resolve: &Resolve,
    package: PackageId,
    input_id: TypeId,
    output_id: TypeId,
    error_id: TypeId,
) {
    let world = resolve.packages[package]
        .worlds
        .iter()
        .find_map(|(name, id)| (name == "program").then_some(*id))
        .expect("Script WIT must declare the program world");
    let run = resolve.worlds[world]
        .exports
        .iter()
        .find_map(|(key, item)| match key {
            wit_parser::WorldKey::Name(name) if name == "run" => Some(item),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Script WIT world program must export run"));
    let WorldItem::Function(run) = run else {
        panic!("Script WIT run export must be a function");
    };
    assert!(
        run.params.len() == 1
            && run.params[0].name == "input"
            && matches!(&run.params[0].ty, Type::Id(id) if unwrap_alias(resolve, *id) == input_id),
        "Script WIT run must take input: script-input"
    );
    let Some(Type::Id(result_id)) = run.result else {
        panic!("Script WIT run must return a named result");
    };
    let TypeDefKind::Result(result) = &resolve.types[unwrap_alias(resolve, result_id)].kind else {
        panic!("Script WIT run result must be a result type");
    };
    assert!(
        result.ok.as_ref().is_some_and(
            |ok| matches!(ok, Type::Id(id) if unwrap_alias(resolve, *id) == output_id)
        ) && result.err.as_ref().is_some_and(
            |err| matches!(err, Type::Id(id) if unwrap_alias(resolve, *id) == error_id)
        ),
        "Script WIT run must return result<script-output, script-error>"
    );
}

fn align_to(value: u32, align: u32) -> u32 {
    debug_assert!(align.is_power_of_two());
    (value + (align - 1)) & !(align - 1)
}

/// Follows `use`-introduced type aliases to the underlying definition.
fn unwrap_alias(resolve: &Resolve, id: TypeId) -> TypeId {
    let mut current = id;
    while let TypeDefKind::Type(Type::Id(inner)) = resolve.types[current].kind {
        current = inner;
    }
    current
}

fn primitive_flat(ty: &Type) -> Option<(&'static str, u32, u32, Vec<Flat>)> {
    match ty {
        Type::String => Some(("string", 8, 4, vec![Flat::I32, Flat::I32])),
        Type::S64 => Some(("s64", 8, 8, vec![Flat::I64])),
        _ => None,
    }
}

fn record_layout(resolve: &Resolve, sizes: &SizeAlign, id: TypeId) -> RecordLayout {
    let TypeDefKind::Record(record) = &resolve.types[id].kind else {
        panic!("Script WIT type must be a record");
    };
    let mut flat = Vec::new();
    let mut fields = Vec::new();
    let mut offset = 0_u32;
    let mut max_align = 1_u32;
    for field in &record.fields {
        let (size, align, field_flat) = field_layout(resolve, &field.ty);
        offset = align_to(offset, align);
        let slot_start = flat.len();
        flat.extend_from_slice(&field_flat);
        fields.push(FieldLayout {
            name: field.name.clone(),
            byte_offset: offset,
            slot_start,
            slot_len: field_flat.len(),
        });
        offset += size;
        max_align = max_align.max(align);
    }
    let size = align_to(offset, max_align);
    let expected = sizes.size(&Type::Id(id)).size_wasm32();
    assert!(
        u32::try_from(expected).is_ok_and(|expected| expected == size),
        "record layout must match wit-parser SizeAlign"
    );
    RecordLayout {
        size,
        align: max_align,
        flat,
        fields,
    }
}

fn field_layout(resolve: &Resolve, ty: &Type) -> (u32, u32, Vec<Flat>) {
    if let Some((_, size, align, flat)) = primitive_flat(ty) {
        return (size, align, flat);
    }
    let Type::Id(id) = ty else {
        panic!("unsupported Script WIT field type {ty:?}");
    };
    match &resolve.types[*id].kind {
        TypeDefKind::List(element) => {
            assert!(
                matches!(element, Type::U8 | Type::String),
                "Script WIT lists stay bounded to u8 and string elements"
            );
            (8, 4, vec![Flat::I32, Flat::I32])
        }
        TypeDefKind::Enum(enum_type) => {
            assert!(
                enum_type.cases.len() <= 256,
                "Script WIT enums keep one-byte discriminants"
            );
            (1, 1, vec![Flat::I32])
        }
        other => panic!("unsupported Script WIT field type {other:?}"),
    }
}

fn enum_tags(resolve: &Resolve, id: TypeId) -> BTreeMap<String, i32> {
    let TypeDefKind::Enum(enum_type) = &resolve.types[id].kind else {
        panic!("script-error-code must be an enum");
    };
    enum_type
        .cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            (
                case.name.clone(),
                i32::try_from(index).expect("enum case count is bounded"),
            )
        })
        .collect()
}

/// Declares the frozen `sico:script/types@0.1.0` instance type. Mirrors the
/// STEP-0076 prototype: exporting a type inside an instance type occupies one
/// type index holding the named id, so each type is exported right after its
/// declaration and every later reference points at the export index.
#[must_use]
pub fn script_types_instance() -> InstanceType {
    let mut types = InstanceType::new();
    types.ty().defined_type().list(PrimitiveValType::String);
    types.export("string-list", ComponentTypeRef::Type(TypeBounds::Eq(0)));
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.export("byte-list", ComponentTypeRef::Type(TypeBounds::Eq(2)));
    types.ty().defined_type().record([
        ("arguments", ComponentValType::Type(1)),
        ("stdin", ComponentValType::Type(3)),
    ]);
    types.export("script-input", ComponentTypeRef::Type(TypeBounds::Eq(4)));
    types.ty().defined_type().record([
        ("stdout", ComponentValType::Type(3)),
        ("stderr", ComponentValType::Type(3)),
        (
            "exit-code",
            ComponentValType::Primitive(PrimitiveValType::S64),
        ),
    ]);
    types.export("script-output", ComponentTypeRef::Type(TypeBounds::Eq(6)));
    types.ty().defined_type().enum_type([
        "invalid-input",
        "resource-limit",
        "domain-error",
        "cancelled",
    ]);
    types.export(
        "script-error-code",
        ComponentTypeRef::Type(TypeBounds::Eq(8)),
    );
    types.ty().defined_type().record([
        ("code", ComponentValType::Type(9)),
        (
            "message",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
    ]);
    types.export("script-error", ComponentTypeRef::Type(TypeBounds::Eq(10)));
    types.ty().defined_type().result(
        Some(ComponentValType::Type(7)),
        Some(ComponentValType::Type(11)),
    );
    types.export("script-result", ComponentTypeRef::Type(TypeBounds::Eq(12)));
    types
}

/// Shared transport memory and bump allocator for the fs-enabled program
/// shape. The program core imports both, so host-lowered fs calls marshal
/// strings/lists through the same linear memory the guest owns; `heap_base`
/// keeps the allocator above the program's static data segment.
fn fs_transport_module(heap_base: u32) -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    let mut functions = FunctionSection::new();
    functions.function(0);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: u64::from(MEMORY_PAGES),
        maximum: Some(u64::from(MEMORY_PAGES)),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(i32::try_from(heap_base).unwrap_or(i32::MAX)),
    );
    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("realloc", ExportKind::Func, 0);
    // aligned = (bump + align - 1) & -align; trap on wrap or ceiling cross.
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
        Instruction::I32Const(i32::try_from(ARENA_LIMIT).unwrap_or(i32::MAX)),
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
    let mut code = CodeSection::new();
    code.function(&realloc);
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    module.finish()
}

/// Instance type of `sico:script/fs-read@0.1.0` (read + exists).
fn fs_read_instance() -> InstanceType {
    let mut types = InstanceType::new();
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.ty().defined_type().result(
        Some(ComponentValType::Type(0)),
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    types.ty().defined_type().result(
        Some(ComponentValType::Primitive(PrimitiveValType::Bool)),
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    types
        .ty()
        .function()
        .params([(
            "path",
            ComponentValType::Primitive(PrimitiveValType::String),
        )])
        .result(Some(ComponentValType::Type(1)));
    types
        .ty()
        .function()
        .params([(
            "path",
            ComponentValType::Primitive(PrimitiveValType::String),
        )])
        .result(Some(ComponentValType::Type(2)));
    types.export("read", ComponentTypeRef::Func(3));
    types.export("exists", ComponentTypeRef::Func(4));
    types
}

/// Instance type of `sico:script/fs-write@0.1.0` (write).
fn fs_write_instance() -> InstanceType {
    let mut types = InstanceType::new();
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.ty().defined_type().result(
        None,
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    types
        .ty()
        .function()
        .params([
            (
                "path",
                ComponentValType::Primitive(PrimitiveValType::String),
            ),
            ("content", ComponentValType::Type(0)),
        ])
        .result(Some(ComponentValType::Type(1)));
    types.export("write", ComponentTypeRef::Func(2));
    types
}

/// Instance type of `sico:script/http@0.1.0` (RFC-0031).
fn http_instance() -> InstanceType {
    let mut types = InstanceType::new();
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.export("byte-list", ComponentTypeRef::Type(TypeBounds::Eq(0)));
    types.ty().defined_type().record([
        ("status", ComponentValType::Primitive(PrimitiveValType::S64)),
        ("body", ComponentValType::Type(1)),
    ]);
    types.export("response", ComponentTypeRef::Type(TypeBounds::Eq(2)));
    types.ty().defined_type().result(
        Some(ComponentValType::Type(3)),
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    types
        .ty()
        .function()
        .params([
            (
                "method",
                ComponentValType::Primitive(PrimitiveValType::String),
            ),
            ("url", ComponentValType::Primitive(PrimitiveValType::String)),
            ("body", ComponentValType::Type(1)),
        ])
        .result(Some(ComponentValType::Type(4)));
    types.export("request", ComponentTypeRef::Func(5));
    types
}

/// Instance type of `sico:script/streams@0.1.0` (RFC-0030). Resources are
/// exported with `SubResource` bounds (the host supplies the concrete type at
/// link time); function signatures reference the export-alias indices.
fn streams_instance() -> InstanceType {
    let mut types = InstanceType::new();
    types.export(
        "input-stream",
        ComponentTypeRef::Type(TypeBounds::SubResource),
    );
    types.export(
        "output-stream",
        ComponentTypeRef::Type(TypeBounds::SubResource),
    );
    types
        .ty()
        .defined_type()
        .enum_type(["io", "cancelled", "closed", "resource-limit"]);
    types.export("stream-error", ComponentTypeRef::Type(TypeBounds::Eq(2)));
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.ty().defined_type().result(
        Some(ComponentValType::Type(4)),
        Some(ComponentValType::Type(3)),
    );
    types
        .ty()
        .defined_type()
        .result(None, Some(ComponentValType::Type(3)));
    types.ty().defined_type().result(
        Some(ComponentValType::Primitive(PrimitiveValType::U64)),
        Some(ComponentValType::Type(3)),
    );
    types.ty().defined_type().borrow(0);
    types.ty().defined_type().borrow(1);
    types.ty().defined_type().own(0);
    types.ty().defined_type().own(1);
    types
        .ty()
        .function()
        .params([] as [(&str, ComponentValType); 0])
        .result(Some(ComponentValType::Type(10)));
    types.export("stdin", ComponentTypeRef::Func(12));
    types
        .ty()
        .function()
        .params([] as [(&str, ComponentValType); 0])
        .result(Some(ComponentValType::Type(11)));
    types.export("stdout", ComponentTypeRef::Func(13));
    types
        .ty()
        .function()
        .params([] as [(&str, ComponentValType); 0])
        .result(Some(ComponentValType::Type(11)));
    types.export("stderr", ComponentTypeRef::Func(14));
    types
        .ty()
        .function()
        .params([
            ("self", ComponentValType::Type(8)),
            ("max", ComponentValType::Primitive(PrimitiveValType::U64)),
        ])
        .result(Some(ComponentValType::Type(5)));
    types.export("[method]input-stream.read", ComponentTypeRef::Func(15));
    types
        .ty()
        .function()
        .params([
            ("self", ComponentValType::Type(9)),
            ("bytes", ComponentValType::Type(4)),
        ])
        .result(Some(ComponentValType::Type(6)));
    types.export("[method]output-stream.write", ComponentTypeRef::Func(16));
    types
        .ty()
        .function()
        .params([("self", ComponentValType::Type(9))])
        .result(Some(ComponentValType::Type(6)));
    types.export("[method]output-stream.flush", ComponentTypeRef::Func(17));
    types
        .ty()
        .function()
        .params([
            ("input", ComponentValType::Type(8)),
            ("output", ComponentValType::Type(9)),
        ])
        .result(Some(ComponentValType::Type(7)));
    types.export("pump", ComponentTypeRef::Func(18));
    types
}

/// Wraps a Script core module as a `sico:script/program@0.1.0` Component.
///
/// The Component imports only the types-only instance and exports `run`,
/// lifted with the guest `memory`, `cabi_realloc` and `cabi_post_run`. When
/// the program calls `sico.fs.*` intrinsics, the Component also imports the
/// used `sico:script/fs-read/fs-write@0.1.0` interfaces, lowered through the
/// shared transport memory module (`heap_base` sits above the static data).
///
/// # Panics
///
/// Panics only if the frozen fs import table no longer matches the declared
/// interface usage (a build-time inconsistency, pinned by tests).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn wrap_script_component(
    core: &[u8],
    fs: FsUse,
    streams: StreamUse,
    http: bool,
    heap_base: u32,
) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let types_ty = builder.type_instance(Some("script-types"), &script_types_instance());
    let types_instance = builder.import(TYPES_INSTANCE, ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    let mut fs_read = None;
    if fs.read_interface() {
        let read_ty = builder.type_instance(Some("fs-read"), &fs_read_instance());
        fs_read = Some(builder.import(FS_READ_INTERFACE, ComponentTypeRef::Instance(read_ty)));
    }
    let mut fs_write = None;
    if fs.write {
        let write_ty = builder.type_instance(Some("fs-write"), &fs_write_instance());
        fs_write = Some(builder.import(FS_WRITE_INTERFACE, ComponentTypeRef::Instance(write_ty)));
    }
    let mut streams_imported = None;
    if streams.any() {
        let streams_ty = builder.type_instance(Some("streams"), &streams_instance());
        streams_imported =
            Some(builder.import(STREAMS_INTERFACE, ComponentTypeRef::Instance(streams_ty)));
    }

    let mut http_imported = None;
    if http {
        let http_ty = builder.type_instance(Some("http"), &http_instance());
        http_imported = Some(builder.import(HTTP_INTERFACE, ComponentTypeRef::Instance(http_ty)));
    }

    let mut transport = None;
    if fs.any() || streams.any() || http {
        let module = builder.core_module_raw(Some("fs-transport"), &fs_transport_module(heap_base));
        let instance = builder.core_instantiate(
            Some("fs-transport"),
            module,
            std::iter::empty::<(&str, wasm_encoder::ModuleArg)>(),
        );
        let memory =
            builder.core_alias_export(Some("memory"), instance, "memory", ExportKind::Memory);
        let realloc =
            builder.core_alias_export(Some("realloc"), instance, "realloc", ExportKind::Func);
        transport = Some((memory, realloc));
    }

    // Lower every used fs import through the transport memory and gather
    // each interface's lowered functions into one core instance.
    let mut lowered: BTreeMap<&str, Vec<(&str, u32)>> = BTreeMap::new();
    if let Some((memory, realloc)) = transport {
        for import in FS_IMPORTS {
            if !fs_used(fs, import) {
                continue;
            }
            let instance = match import.interface {
                FS_READ_INTERFACE => fs_read.expect("read interface imported when used"),
                _ => fs_write.expect("write interface imported when used"),
            };
            let function =
                builder.alias_export(instance, import.function, ComponentExportKind::Func);
            let lowered_function = builder.lower_func(
                Some(import.function),
                function,
                [
                    CanonicalOption::UTF8,
                    CanonicalOption::Memory(memory),
                    CanonicalOption::Realloc(realloc),
                ],
            );
            lowered
                .entry(import.interface)
                .or_default()
                .push((import.function, lowered_function));
        }
        if http {
            let instance = http_imported.expect("http interface imported when used");
            let function = builder.alias_export(instance, "request", ComponentExportKind::Func);
            let lowered_function = builder.lower_func(
                Some("request"),
                function,
                [
                    CanonicalOption::UTF8,
                    CanonicalOption::Memory(memory),
                    CanonicalOption::Realloc(realloc),
                ],
            );
            lowered
                .entry(HTTP_INTERFACE)
                .or_default()
                .push(("request", lowered_function));
        }
        if streams.any() {
            let instance = streams_imported.expect("streams interface imported when used");
            let input_stream =
                builder.alias_export(instance, "input-stream", ComponentExportKind::Type);
            let output_stream =
                builder.alias_export(instance, "output-stream", ComponentExportKind::Type);
            for import in STREAM_IMPORTS {
                if !stream_used(streams, import) {
                    continue;
                }
                let lowered_function = match import.intrinsic {
                    "sico.stream.close_input" => builder.resource_drop(input_stream),
                    "sico.stream.close_output" => builder.resource_drop(output_stream),
                    _ => {
                        let export = match import.function {
                            "read" => "[method]input-stream.read",
                            "write" => "[method]output-stream.write",
                            "flush" => "[method]output-stream.flush",
                            other => other,
                        };
                        let function =
                            builder.alias_export(instance, export, ComponentExportKind::Func);
                        builder.lower_func(
                            Some(import.function),
                            function,
                            [
                                CanonicalOption::UTF8,
                                CanonicalOption::Memory(memory),
                                CanonicalOption::Realloc(realloc),
                            ],
                        )
                    }
                };
                lowered
                    .entry(STREAMS_INTERFACE)
                    .or_default()
                    .push((import.function, lowered_function));
            }
        }
    }

    let mut arguments: Vec<(&str, wasm_encoder::ModuleArg)> = Vec::new();
    if let Some((memory, realloc)) = transport {
        let exports = builder.core_instantiate_exports(
            Some("fs-transport-exports"),
            [
                ("memory", ExportKind::Memory, memory),
                ("realloc", ExportKind::Func, realloc),
            ],
        );
        arguments.push((
            FS_TRANSPORT_MODULE,
            wasm_encoder::ModuleArg::Instance(exports),
        ));
    }
    for (interface, functions) in lowered {
        let instance = builder.core_instantiate_exports(
            Some("fs-lowered"),
            functions
                .into_iter()
                .map(|(name, index)| (name, ExportKind::Func, index)),
        );
        arguments.push((interface, wasm_encoder::ModuleArg::Instance(instance)));
    }

    let module = builder.core_module_raw(Some("sico-script-core"), core);
    let instance = builder.core_instantiate(Some("sico-script-core"), module, arguments);
    let memory = builder.core_alias_export(Some("memory"), instance, "memory", ExportKind::Memory);
    let realloc = builder.core_alias_export(
        Some("cabi_realloc"),
        instance,
        "cabi_realloc",
        ExportKind::Func,
    );
    let post_return = builder.core_alias_export(
        Some("cabi_post_run"),
        instance,
        "cabi_post_run",
        ExportKind::Func,
    );
    let run_core = builder.core_alias_export(Some("run"), instance, "run", ExportKind::Func);
    let run = builder.lift_func(
        Some("run"),
        run_core,
        run_ty,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(memory),
            wasm_encoder::CanonicalOption::Realloc(realloc),
            wasm_encoder::CanonicalOption::PostReturn(post_return),
        ],
    );
    builder.export("run", ComponentExportKind::Func, run, None);
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_layout_table_matches_the_documented_values() {
        let abi = ScriptAbi::load();
        assert_eq!((abi.input.size, abi.input.align), (16, 4));
        assert_eq!(abi.input.flat, [Flat::I32, Flat::I32, Flat::I32, Flat::I32]);
        assert_eq!(abi.input.fields[0].byte_offset, 0);
        assert_eq!(abi.input.fields[0].name, "arguments");
        assert_eq!(
            (abi.input.fields[0].slot_start, abi.input.fields[0].slot_len),
            (0, 2)
        );
        assert_eq!(abi.input.fields[1].byte_offset, 8);
        assert_eq!(abi.input.fields[1].name, "stdin");
        assert_eq!(
            (abi.input.fields[1].slot_start, abi.input.fields[1].slot_len),
            (2, 2)
        );

        assert_eq!((abi.output.size, abi.output.align), (24, 8));
        assert_eq!(
            abi.output.flat,
            [Flat::I32, Flat::I32, Flat::I32, Flat::I32, Flat::I64]
        );
        assert_eq!(abi.output.fields[0].byte_offset, 0);
        assert_eq!(abi.output.fields[1].byte_offset, 8);
        assert_eq!(abi.output.fields[2].byte_offset, 16);
        assert_eq!(abi.output.fields[2].name, "exit-code");

        assert_eq!((abi.error.size, abi.error.align), (12, 4));
        assert_eq!(abi.error.flat, [Flat::I32, Flat::I32, Flat::I32]);
        assert_eq!(abi.error.fields[0].byte_offset, 0);
        assert_eq!(abi.error.fields[0].name, "code");
        assert_eq!(abi.error.fields[1].byte_offset, 4);
        assert_eq!(abi.error.fields[1].name, "message");

        assert_eq!(
            abi.error_tags,
            [
                ("invalid-input".to_owned(), 0),
                ("resource-limit".to_owned(), 1),
                ("domain-error".to_owned(), 2),
                ("cancelled".to_owned(), 3),
            ]
            .into_iter()
            .collect()
        );
        assert_eq!((abi.result_size, abi.result_payload_offset), (32, 8));
    }

    #[test]
    fn ir_boundary_types_map_to_flat_slots() {
        let abi = ScriptAbi::load();
        assert_eq!(
            abi.flat_ir_types(&sico_ir::Type::String),
            Some(vec![Flat::I32, Flat::I32])
        );
        assert_eq!(
            abi.flat_ir_types(&sico_ir::Type::Bytes),
            Some(vec![Flat::I32, Flat::I32])
        );
        assert_eq!(
            abi.flat_ir_types(&sico_ir::Type::List(Box::new(sico_ir::Type::String))),
            Some(vec![Flat::I32, Flat::I32])
        );
        assert_eq!(
            abi.flat_ir_types(&sico_ir::Type::List(Box::new(sico_ir::Type::Bytes))),
            None
        );
        assert_eq!(abi.flat_ir_types(&sico_ir::Type::Int), None);
        let result = sico_ir::Type::Result {
            ok: Box::new(sico_ir::Type::Named("ScriptOutput".into())),
            error: Box::new(sico_ir::Type::Named("ScriptError".into())),
        };
        assert_eq!(abi.flat_ir_types(&result).map(|flat| flat.len()), Some(9));
    }
}
