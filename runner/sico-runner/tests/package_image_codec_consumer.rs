//! M24 #2 (`sico:user/image-codec@1`, RFC-0050 as amended by A1, ADR-0019)
//! **registry-path** consumer evidence — the registry half companion to
//! `image_codec_package.rs` (STEP-0331), closing the gap STEP-0593 explicitly
//! recorded as "未在本片补断言".
//!
//! STEP-0593 fixed the runner's per-Store table budget (`tables()` was `0`,
//! now `package_instances_capacity`) and `table_growing` (which wasmtime also
//! calls at table *creation* time), and registered `image-codec@1` as an
//! **expected beneficiary**: its committed core (`image-codec-core.wasm`)
//! carries one defined funcref table (`min == max == 71`), so it was blocked
//! from the M7 registry path by the same `tables() = 0` as the roster
//! packages. STEP-0593 recorded that it did **not** assert the codec on the
//! registry path and left the frozen `decode-ok-2x2-rgba` case as the starting
//! point. This file closes that registered gap.
//!
//! What is proved here, through the M7 registry install path
//! ([`Runner::prepare_program_with_packages`] + [`PreparedProgram::run`], the
//! same `prepare_package` → `link_packages` → `instantiate_packages` sequence
//! `package_roster_consumer.rs` proves for the roster packages and
//! `package_consumer_forward.rs` for `csv`):
//!
//! 1. a hand-written `wasm-encoder` Program imports the package's real
//!    `sico:user/image-codec@1.0.0` interface, the runner links it against the
//!    prepared package, and the guest receives the package's **real return
//!    bytes** through the dynamic `Val` bridge — `decode-png`'s ok arm
//!    (16-byte little-endian header + RFC-0041 BGRA8 pixels) byte-exact against
//!    the frozen `decode-ok-2x2-rgba` payload, `encode-png`'s bytes byte-stable
//!    through two runs and round-tripping back to the same frozen payload, and
//!    the error arm's RFC-0050 case-name text (`bad-signature`) byte-exact;
//! 2. the same install path **fails closed** on the refusal surface
//!    `package_consumer_authority.rs` establishes: an unprepared package
//!    import, an arity mismatch (for both the 1-param `decode-png` and the
//!    3-param `encode-png` shapes), a same-arity type mismatch with no partial
//!    call, and an impure package.
//!
//! Evidence level is `internal-fixture` (package/Component level),
//! `contract-verified` + runtime-verified at the runner bridge. It moves no
//! M24 §3 gate (the aggregate gate-3 ruling is the exit audit's). Zero compile:
//! the package is a pre-compiled, digest-pinned Component (the producer embeds
//! the committed `image-codec-core.wasm` via `include_bytes!`); the consumer
//! path only instantiates it — no Rust-side rebuild, no guest core rebuild, no
//! frozen corpus edit.

use std::path::PathBuf;

use sico_codegen_wasm::package_builder::image_codec_package;
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PackageBinary, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use wasm_encoder::{
    BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ComponentTypeRef,
    ComponentValType, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, InstanceType, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeBounds, TypeSection,
    ValType,
};

const INTERFACE: &str = "sico:user/image-codec@1.0.0";
const TRANSPORT_INSTANCE: &str = "codec-transport";
const INPUT_PTR: u32 = 1024;
const EMPTY_LIST_PTR: u32 = 8192;
const RESULT_AREA: u32 = 16_384;
const OUTPUT_AREA: u32 = 20_480;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;

/// The frozen `decode-ok-2x2-rgba` case name (STEP-0593's registered starting
/// point; the expectation is read from the frozen manifest, not hardcoded).
const DECODE_OK_CASE: &str = "decode-ok-2x2-rgba";

/// The declared component-level parameter kind for one guest-import parameter.
/// `Bytes` is a `list<u8>` (the package's real `Bytes` shape); `U64` is the
/// primitive u64. The runner's `link_packages` matches by name + arity only, so
/// `U64` in place of `Bytes` is the same-arity type-mismatch knob.
#[derive(Clone, Copy)]
enum ParamKind {
    Bytes,
    U64,
}

/// One registry-path consumer scenario for the codec package.
struct CodecCall {
    /// The package export the guest imports: `decode-png` (1 param) or
    /// `encode-png` (3 params).
    function: &'static str,
    /// The declared param kinds, in order.
    params: Vec<ParamKind>,
    /// Static input byte array the guest forwards (PNG bytes for `decode-png`,
    /// BGRA8 pixels for `encode-png`). Unused when the single param is `U64`.
    input: Vec<u8>,
    /// The `U64` parameter values the guest forwards (encode width/height, or
    /// the single wrong-typed value in the type-mismatch scenario).
    u64_args: Vec<u64>,
    /// The `result` discriminant the guest expects: 0 = ok, 1 = err. The guest
    /// returns the payload bytes (the ok `list<u8>`, or the error case-name
    /// text) as stdout only when the discriminant matches, and a non-zero exit
    /// otherwise.
    expected_disc: u8,
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/image-codec")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(corpus_dir().join(name))
        .unwrap_or_else(|error| panic!("read fixture {name}: {error}"))
}

/// The frozen `decode-ok-2x2-rgba` expectation, read from the committed
/// manifest so the test and the independent oracle cannot drift apart.
fn decode_ok_payload() -> Vec<u8> {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("corpus/image-codec/manifest.json"))
            .expect("manifest parses");
    for case in manifest["decode_cases"].as_array().expect("decode_cases") {
        if case["name"].as_str() == Some(DECODE_OK_CASE) {
            return unhex(case["expect"]["payload_hex"].as_str().expect("payload_hex"));
        }
    }
    panic!("{DECODE_OK_CASE} missing from the frozen manifest");
}

/// The component-level instance type of `sico:user/image-codec@1.0.0` as the
/// guest declares it: a single function over the declared params returning
/// `result<list<u8>, string>`. Type index bookkeeping mirrors
/// `package_roster_consumer.rs` (each `ty()` consumes one index).
fn codec_instance_type(call: &CodecCall) -> InstanceType {
    let mut types = InstanceType::new();
    // 0: list<u8>; 1: result<list<u8>, string>; 2: func.
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.ty().defined_type().result(
        Some(ComponentValType::Type(0)),
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    let names = ["p0", "p1", "p2"];
    let mut function = types.ty().function();
    function.params(
        call.params
            .iter()
            .zip(names)
            .map(|(kind, name)| {
                let ty = match kind {
                    ParamKind::Bytes => ComponentValType::Type(0),
                    ParamKind::U64 => ComponentValType::Primitive(PrimitiveValType::U64),
                };
                (name, ty)
            })
            .collect::<Vec<_>>(),
    );
    function.result(Some(ComponentValType::Type(1)));
    types.export(call.function, ComponentTypeRef::Func(2));
    types
}

fn script_types() -> InstanceType {
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

/// The core signature of the lowered package export as the guest module
/// declares it: the flattened params plus the trailing i32 return-area pointer,
/// returning nothing. `Bytes` flattens to `(ptr, len)`; `U64` to a single i64.
fn lowered_core_params(call: &CodecCall) -> Vec<ValType> {
    let mut params = Vec::new();
    for kind in &call.params {
        match kind {
            ParamKind::Bytes => {
                params.push(ValType::I32);
                params.push(ValType::I32);
            }
            ParamKind::U64 => params.push(ValType::I64),
        }
    }
    params.push(ValType::I32);
    params
}

/// Encodes the consumer Program: imports `sico:script/types` (type-only) and
/// the codec package interface, lowers the package export against a shared
/// transport (memory + checked bump allocator), and wires the guest core
/// module that forwards the static input and returns the payload bytes.
fn consumer_component(call: &CodecCall) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();

    let types_ty = builder.type_instance(Some("script-types"), &script_types());
    let types_instance = builder.import("sico:script/types", ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    // The codec package — the runner advertises it through `link_packages`.
    let pkg_ty = builder.type_instance(Some("pkg"), &codec_instance_type(call));
    let pkg_instance = builder.import(INTERFACE, ComponentTypeRef::Instance(pkg_ty));

    // Transport core module: shared memory + checked bump allocator + the
    // static input the guest forwards.
    let transport_module_index =
        builder.core_module_raw(Some(TRANSPORT_INSTANCE), &transport_module(call));
    let transport_instance = builder.core_instantiate(
        Some(TRANSPORT_INSTANCE),
        transport_module_index,
        std::iter::empty::<(&str, ModuleArg)>(),
    );
    let transport_memory = builder.core_alias_export(
        Some("memory"),
        transport_instance,
        "memory",
        ExportKind::Memory,
    );
    let transport_realloc = builder.core_alias_export(
        Some("realloc"),
        transport_instance,
        "realloc",
        ExportKind::Func,
    );
    let transport_exports = builder.core_instantiate_exports(
        Some("transport-exports"),
        [
            ("memory", ExportKind::Memory, transport_memory),
            ("realloc", ExportKind::Func, transport_realloc),
        ],
    );

    // Lower the package export against the transport memory. Its
    // `result<list<u8>, string>` return flattens to more than one core type,
    // so the canonical ABI uses the return-area (trailing i32 retptr) form.
    let pkg_function = builder.alias_export(pkg_instance, call.function, ComponentExportKind::Func);
    let lowered = builder.lower_func(
        Some("lowered"),
        pkg_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let lowered_instance = builder.core_instantiate_exports(
        Some("pkg-lowered"),
        [(call.function, ExportKind::Func, lowered)],
    );

    // Guest core module.
    let core = builder.core_module_raw(Some("guest"), &guest_module(call));
    let instance = builder.core_instantiate(
        Some("guest"),
        core,
        [
            (TRANSPORT_INSTANCE, ModuleArg::Instance(transport_exports)),
            (INTERFACE, ModuleArg::Instance(lowered_instance)),
        ],
    );
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
    let run_function = builder.lift_func(
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
    builder.export("run", ComponentExportKind::Func, run_function, None);
    builder.finish()
}

/// Shared transport: one memory + a checked bump allocator and the static
/// input byte array the guest forwards.
fn transport_module(call: &CodecCall) -> Vec<u8> {
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
        &ConstExpr::i32_const(EMPTY_LIST_PTR as i32),
    );
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("realloc", ExportKind::Func, 0);
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
    let mut data = DataSection::new();
    if !call.input.is_empty() {
        data.active(
            0,
            &ConstExpr::i32_const(INPUT_PTR as i32),
            call.input.clone(),
        );
    }
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    let mut code = CodeSection::new();
    code.function(&realloc);
    module.section(&code);
    module.section(&data);
    module.finish()
}

fn load8(offset: u32) -> Instruction<'static> {
    Instruction::I32Load8U(MemArg {
        offset: u64::from(offset),
        align: 0,
        memory_index: 0,
    })
}

fn load32(offset: u32) -> Instruction<'static> {
    Instruction::I32Load(MemArg {
        offset: u64::from(offset),
        align: 2,
        memory_index: 0,
    })
}

fn store32(offset: u32) -> Instruction<'static> {
    Instruction::I32Store(MemArg {
        offset: u64::from(offset),
        align: 2,
        memory_index: 0,
    })
}

/// Emits a zero-tag (ok) script-output whose stdout is `(ptr_local, len_local)`
/// and whose exit-code is `exit_local`, then leaves the area pointer for the
/// function return. Script-result area layout (a `result<record, error>`
/// variant whose record payload has i64 alignment): tag i32 @0; stdout ptr/len
/// @8/12; stderr ptr/len @16/20; exit-code i64 @24.
fn push_ok_return(
    instructions: &mut Vec<Instruction<'static>>,
    exit_local: u32,
    stdout_ptr_local: u32,
    stdout_len_local: u32,
) {
    for instruction in [
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::I32Const(0),
        Instruction::I32Store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::LocalGet(stdout_ptr_local),
        store32(8),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::LocalGet(stdout_len_local),
        store32(12),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        store32(16),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::I32Const(0),
        store32(20),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::LocalGet(exit_local),
        Instruction::I64Store(MemArg {
            offset: 24,
            align: 3,
            memory_index: 0,
        }),
        Instruction::I32Const(OUTPUT_AREA as i32),
    ] {
        instructions.push(instruction);
    }
}

/// Guest core module. Function index space: 0 = imported realloc, 1 = imported
/// lowered package export, then `run`, then the no-op post-return.
fn guest_module(call: &CodecCall) -> Vec<u8> {
    let mut types = TypeSection::new();
    // 0: realloc
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 1: lowered package export, `(flattened params…, retptr) -> ()`.
    types.ty().function(lowered_core_params(call), []);
    // 2: run
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 3: post-run
    types.ty().function([ValType::I32], []);

    let mut imports = ImportSection::new();
    imports.import(
        TRANSPORT_INSTANCE,
        "memory",
        EntityType::Memory(MemoryType {
            minimum: u64::from(MEMORY_PAGES),
            maximum: Some(u64::from(MEMORY_PAGES)),
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    imports.import(TRANSPORT_INSTANCE, "realloc", EntityType::Function(0));
    imports.import(INTERFACE, call.function, EntityType::Function(1));

    let mut function_section = FunctionSection::new();
    function_section.function(2);
    function_section.function(3);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, 2);
    exports.export("cabi_post_run", ExportKind::Func, 3);

    let mut code = CodeSection::new();
    code.function(&run_body(call));
    let mut nop = Function::new(vec![]);
    nop.instruction(&Instruction::End);
    code.function(&nop);

    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&function_section);
    module.section(&exports);
    module.section(&code);
    module.finish()
}

/// Guest `run`: forward one codec call and report the payload bytes as stdout
/// when the result discriminant matches the expected polarity, else a non-zero
/// exit. Locals: params 0..3 (input, ignored), 4 = exit i64, 5 = disc i32,
/// 6 = payload ptr i32, 7 = payload len i32.
fn run_body(call: &CodecCall) -> Function {
    let mut body = Function::new(vec![(1, ValType::I64), (3, ValType::I32)]);
    let mut instructions: Vec<Instruction<'static>> = Vec::new();
    let mut u64_iter = call.u64_args.iter();
    for kind in &call.params {
        match kind {
            ParamKind::Bytes => {
                instructions.push(Instruction::I32Const(INPUT_PTR as i32));
                instructions.push(Instruction::I32Const(call.input.len() as i32));
            }
            ParamKind::U64 => {
                instructions.push(Instruction::I64Const(
                    i64::try_from(*u64_iter.next().expect("u64 arg")).expect("u64 fits i64"),
                ));
            }
        }
    }
    instructions.extend([
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::Call(1),
        // disc = load8u(RESULT_AREA + 0)
        Instruction::I32Const(RESULT_AREA as i32),
        load8(0),
        Instruction::LocalSet(5),
        // if disc == expected: stdout = payload (ptr@+4, len@+8), exit = 0;
        // else: stdout = empty, exit = 1000 + disc (non-zero, never partial).
        Instruction::LocalGet(5),
        Instruction::I32Const(i32::from(call.expected_disc)),
        Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(RESULT_AREA as i32),
        load32(4),
        Instruction::LocalSet(6),
        Instruction::I32Const(RESULT_AREA as i32),
        load32(8),
        Instruction::LocalSet(7),
        Instruction::I64Const(0),
        Instruction::LocalSet(4),
        Instruction::Else,
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        Instruction::LocalSet(6),
        Instruction::I32Const(0),
        Instruction::LocalSet(7),
        Instruction::LocalGet(5),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(1000),
        Instruction::I64Add,
        Instruction::LocalSet(4),
        Instruction::End,
    ]);
    push_ok_return(&mut instructions, 4, 6, 7);
    instructions.push(Instruction::End);
    for instruction in instructions {
        body.instruction(&instruction);
    }
    body
}

/// Installs `package` along the registry path and runs the consumer for `call`,
/// returning the guest's `Output`.
fn run_consumer(call: &CodecCall, package: Vec<u8>) -> sico_runner::ScriptOutput {
    let component = consumer_component(call);
    let packages = [PackageBinary { bytes: package }];
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_packages(
            &component,
            &FsGrants::default(),
            &NetGrants::default(),
            &packages,
        )
        .expect("consumer component links against the prepared package");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    let RunOutcome::Output(output) = outcome else {
        panic!("expected guest output, got {outcome:?}");
    };
    output
}

// ---------------------------------------------------------------------------
// Happy path: `sico:user/image-codec@1` installed along the registry path
// returns the frozen corpus payload bytes through the real consumer.
// ---------------------------------------------------------------------------

#[test]
fn image_codec_decode_ok_reaches_the_consumer_through_the_registry() {
    // Frozen `decode-ok-2x2-rgba` case: 16-byte header (2x2) + 16 BGRA8 pixels.
    let want = decode_ok_payload();
    assert_eq!(
        want.len(),
        32,
        "2x2 RGBA payload = 16-byte header + 16 pixels"
    );
    let call = CodecCall {
        function: "decode-png",
        params: vec![ParamKind::Bytes],
        input: fixture("ok-2x2-rgba.png"),
        u64_args: Vec::new(),
        expected_disc: 0,
    };
    let output = run_consumer(&call, image_codec_package());
    assert!(output.stderr.is_empty(), "the guest writes no stderr");
    assert_eq!(output.exit_code, 0, "ok polarity reached the guest");
    assert_eq!(
        output.stdout, want,
        "decode-png ok payload must be byte-exact through the registry path"
    );
}

#[test]
fn image_codec_encode_round_trips_through_the_registry() {
    // Encode the frozen 2x2 RGBA pixels (the decode payload's 16-byte tail)
    // through the registry path: two encodes byte-stable, and the encoded PNG
    // re-decodes back to the same frozen payload. Anchored on the frozen
    // manifest, not on a hardcoded encode byte string.
    let payload = decode_ok_payload();
    let width = u64::from_le_bytes(payload[0..8].try_into().unwrap());
    let height = u64::from_le_bytes(payload[8..16].try_into().unwrap());
    let pixels = payload[16..].to_vec();
    assert_eq!((width, height), (2, 2));
    assert_eq!(pixels.len(), (width * height * 4) as usize);

    let call = CodecCall {
        function: "encode-png",
        params: vec![ParamKind::Bytes, ParamKind::U64, ParamKind::U64],
        input: pixels.clone(),
        u64_args: vec![width, height],
        expected_disc: 0,
    };
    let first = run_consumer(&call, image_codec_package());
    assert_eq!(first.exit_code, 0, "encode ok polarity reached the guest");
    let second = run_consumer(&call, image_codec_package());
    assert_eq!(
        first.stdout, second.stdout,
        "encode-png is byte-stable across registry-path runs"
    );
    assert!(!first.stdout.is_empty(), "encode produced PNG bytes");

    let decode_call = CodecCall {
        function: "decode-png",
        params: vec![ParamKind::Bytes],
        input: first.stdout.clone(),
        u64_args: Vec::new(),
        expected_disc: 0,
    };
    let round = run_consumer(&decode_call, image_codec_package());
    assert_eq!(
        round.stdout, payload,
        "decode(encode(x)) == x at the payload level through the registry path"
    );
}

#[test]
fn image_codec_error_case_name_reaches_the_consumer_through_the_registry() {
    // Frozen `decode-bad-signature` case: the RFC-0050 `bad-signature` refusal.
    let call = CodecCall {
        function: "decode-png",
        params: vec![ParamKind::Bytes],
        input: fixture("bad-signature.png"),
        u64_args: Vec::new(),
        expected_disc: 1,
    };
    let output = run_consumer(&call, image_codec_package());
    assert_eq!(output.exit_code, 0, "err polarity reached the guest");
    assert_eq!(
        output.stdout,
        b"bad-signature".to_vec(),
        "the error arm must carry the typed case-name text through the registry path"
    );
}

/// The consumer run instantiates the already-compiled package Component into
/// each fresh Store (the `instantiate_packages` per-Store path, never a cached
/// global), so the same Program + package must return the same bytes again.
#[test]
fn codec_consumer_run_is_repeatable() {
    let call = CodecCall {
        function: "decode-png",
        params: vec![ParamKind::Bytes],
        input: fixture("ok-2x2-rgba.png"),
        u64_args: Vec::new(),
        expected_disc: 0,
    };
    let first = run_consumer(&call, image_codec_package());
    let second = run_consumer(&call, image_codec_package());
    assert_eq!(
        first.stdout, second.stdout,
        "registry consumer is byte-stable"
    );
    assert_eq!(first.exit_code, second.exit_code);
}

// ---------------------------------------------------------------------------
// Fail-closed: the refusal surface `package_consumer_authority.rs` establishes
// for `csv`, asserted per codec export shape along the same registry path.
// ---------------------------------------------------------------------------

/// A Program component that imports `INTERFACE` and declares `function` with
/// `params` core-visible parameters and exactly one `string` result. The
/// runner matches the declaration against the prepared package's export by
/// name + arity, so `params` is the knob under test. Type index 0 is a
/// `list<u8>`; `params` refer to it via `bytes_type()`.
fn guest_importing(function: &str, params: Vec<(&str, ComponentValType)>) -> Vec<u8> {
    let mut instance = InstanceType::new();
    instance.ty().defined_type().list(PrimitiveValType::U8);
    let mut fn_ty = instance.ty().function();
    fn_ty.params(params);
    fn_ty.result(Some(ComponentValType::Primitive(PrimitiveValType::String)));
    instance.export(function, ComponentTypeRef::Func(1));

    let mut builder = ComponentBuilder::default();
    let instance_ty = builder.type_instance(Some("pkg"), &instance);
    builder.import(INTERFACE, ComponentTypeRef::Instance(instance_ty));
    builder.finish()
}

fn launch_message(outcome: Result<sico_runner::PreparedProgram, RunOutcome>) -> String {
    match outcome {
        Ok(_) => panic!("expected a fail-closed launch refusal, got a prepared program"),
        Err(RunOutcome::Launch(message)) => message,
        Err(other) => panic!("expected a Launch refusal, got {other:?}"),
    }
}

fn bytes_type() -> ComponentValType {
    ComponentValType::Type(0)
}

#[test]
fn guest_importing_an_unprepared_codec_package_fails_closed() {
    for (function, params) in [
        ("decode-png", vec![("p0", bytes_type())]),
        (
            "encode-png",
            vec![
                ("p0", bytes_type()),
                ("p1", ComponentValType::Primitive(PrimitiveValType::U64)),
                ("p2", ComponentValType::Primitive(PrimitiveValType::U64)),
            ],
        ),
    ] {
        let runner = Runner::new().expect("runner builds");
        let message = launch_message(runner.prepare_program_with_packages(
            &guest_importing(function, params),
            &FsGrants::default(),
            &NetGrants::default(),
            &[],
        ));
        assert!(
            message.contains("no prepared package exports it"),
            "{function}: an import with no matching package must be a typed launch refusal, got: {message}"
        );
    }
}

#[test]
fn codec_package_import_arity_mismatch_fails_closed() {
    // The bridge forwards dynamically typed `Val`s straight to the package
    // export, so an arity mismatch is unsafe and must be refused at link time.
    // `decode-png` takes exactly 1 param and `encode-png` exactly 3; a 2-param
    // declaration must be refused for both.
    for function in ["decode-png", "encode-png"] {
        let runner = Runner::new().expect("runner builds");
        let packages = [PackageBinary {
            bytes: image_codec_package(),
        }];
        let message = launch_message(runner.prepare_program_with_packages(
            &guest_importing(function, vec![("p0", bytes_type()), ("p1", bytes_type())]),
            &FsGrants::default(),
            &NetGrants::default(),
            &packages,
        ));
        assert!(
            message.contains("arity mismatch"),
            "{function}: a guest import whose arity differs from the package export must fail closed, got: {message}"
        );
    }
}

#[test]
fn codec_package_same_arity_type_mismatch_fails_closed_without_a_partial_call() {
    // The bridge checks arity, not the full type. A same-arity type mismatch
    // must not produce a partial/incorrect call: the consumer declares
    // `decode-png` as `func(u64) -> result<list<u8>, string>` (arity 1 like the
    // real `func(list<u8>)`, but the param type differs). Link time accepts it
    // (arity matches); the mismatch surfaces when the guest actually calls the
    // export, as a typed failure — never a successful Output carrying bytes the
    // guest could mistake for a result.
    let call = CodecCall {
        function: "decode-png",
        params: vec![ParamKind::U64],
        input: Vec::new(),
        u64_args: vec![1],
        expected_disc: 0,
    };
    let runner = Runner::new().expect("runner builds");
    let packages = [PackageBinary {
        bytes: image_codec_package(),
    }];
    let prepared = runner.prepare_program_with_packages(
        &consumer_component(&call),
        &FsGrants::default(),
        &NetGrants::default(),
        &packages,
    );
    match prepared {
        Ok(prepared) => {
            // Link time accepted it; the mismatch must still fail when the
            // guest calls, never yielding a partial/incorrect Output.
            let outcome = prepared
                .run(
                    &ScriptInput {
                        arguments: Vec::new(),
                        stdin: Vec::new(),
                    },
                    &RunnerLimits::default(),
                    &CancelToken::new(),
                )
                .expect("input bounds hold");
            assert!(
                !matches!(outcome, RunOutcome::Output(_)),
                "a same-arity type mismatch must not yield a successful Output, got {outcome:?}"
            );
        }
        Err(RunOutcome::Launch(message)) => {
            assert!(
                message.contains("arity mismatch"),
                "a same-arity type mismatch must fail closed, got: {message}"
            );
        }
        Err(other) => panic!("expected a typed launch refusal, got {other:?}"),
    }
}

#[test]
fn impure_codec_package_is_refused_at_preparation() {
    // A "package" that itself imports something is not a pure v0 package;
    // preparation must refuse it before it is ever linked into a run, so it
    // can never smuggle ambient authority into the consumer's Store.
    let runner = Runner::new().expect("runner builds");
    let packages = [PackageBinary {
        bytes: guest_importing("decode-png", vec![("p0", bytes_type())]),
    }];
    let message = launch_message(runner.prepare_program_with_packages(
        &ComponentBuilder::default().finish(),
        &FsGrants::default(),
        &NetGrants::default(),
        &packages,
    ));
    assert!(
        message.contains("must not import"),
        "a package with its own imports must be refused as impure, got: {message}"
    );
}
