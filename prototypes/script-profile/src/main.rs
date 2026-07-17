use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use wasm_encoder::{
    ComponentBuilder, ComponentExportKind, ComponentTypeRef, ComponentValType, ExportKind,
    InstanceType, ModuleArg, PrimitiveValType, TypeBounds,
};
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

const STDIN_LIMIT: usize = 8 * 1024 * 1024;
const WARM_ITERATIONS: usize = 40;
const TYPES_IMPORT: &str = "sico:script/types@0.1.0";
const PROGRAM_RUN_IMPORT: &str = "program-run";
const ADAPTER_ID: &str = "sico:script/adapter@0.1.0";

type AnyError = Box<dyn Error + Send + Sync>;
type AnyResult<T> = Result<T, AnyError>;

#[derive(Debug, Deserialize)]
struct CaseFile {
    cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    JoinArgs,
    EchoStdin,
    SplitChannels,
    ScriptError,
}

#[derive(Clone, Debug, Deserialize)]
struct Case {
    name: String,
    mode: Mode,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    stdin_hex: String,
    stdin_pattern_hex: Option<String>,
    stdin_size: Option<usize>,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize)]
struct Expected {
    stdout_hex: Option<String>,
    stderr_hex: Option<String>,
    exit: Option<i64>,
    error_kind: Option<String>,
    error_message: Option<String>,
    echo_stdin: Option<bool>,
    runner_error: Option<String>,
    tool_exit: Option<u8>,
}

#[derive(Clone, Debug)]
struct ScriptInput {
    args: Vec<String>,
    stdin: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScriptResult {
    Output {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        exit: i64,
    },
    Error {
        kind: String,
        message: String,
    },
}

struct Components {
    join_args: Component,
    echo_stdin: Component,
    split_channels: Component,
    script_error: Component,
}

fn component_for<'a>(components: &'a Components, mode: &Mode) -> &'a Component {
    match mode {
        Mode::JoinArgs => &components.join_args,
        Mode::EchoStdin => &components.echo_stdin,
        Mode::SplitChannels => &components.split_channels,
        Mode::ScriptError => &components.script_error,
    }
}

fn main() -> AnyResult<()> {
    let case_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cases.json");
    let cases: CaseFile = serde_json::from_slice(&fs::read(case_path)?)?;

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    let adapter_bytes = build_adapter_component();
    if adapter_bytes != build_adapter_component() {
        return Err("adapter encoding is not deterministic".into());
    }

    let direct = compile_components(&engine, None)?;
    let composed = compile_components(&engine, Some(&adapter_bytes))?;
    let direct_passed = run_cases(&engine, &direct.components, &cases.cases)?;
    let composed_passed = run_cases(&engine, &composed.components, &cases.cases)?;

    let benchmark_case = cases
        .cases
        .iter()
        .find(|case| case.name == "binary-stdin")
        .ok_or("missing binary-stdin benchmark case")?;
    let benchmark_input = case_input(benchmark_case)?;
    let direct_warm = benchmark_warm(
        &engine,
        &direct.components,
        benchmark_case,
        &benchmark_input,
    )?;
    let composed_warm = benchmark_warm(
        &engine,
        &composed.components,
        benchmark_case,
        &benchmark_input,
    )?;

    let report = serde_json::json!({
        "prototype": "script-profile-program-adapter-composition-v0",
        "wasmtime": "46.0.1",
        "adapter": {
            "identity": ADAPTER_ID,
            "bytes": adapter_bytes.len(),
            "sha256": hex_digest(&adapter_bytes),
        },
        "direct": path_report(&direct, direct_passed, cases.cases.len(), &direct_warm),
        "composed": path_report(&composed, composed_passed, cases.cases.len(), &composed_warm),
        "decision": "composition-go",
        "scope": "hard-coded guest Program Components composed with a deterministic versioned Adapter Component; the Adapter canonical-lowers the Program function into an isolated core trampoline and canonical-lifts its run export; both paths use the same Script values and Wasmtime 46.0.1 in-process runner"
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

struct CompiledPath {
    components: Components,
    cold_compile: Duration,
    component_bytes: usize,
    component_sizes: BTreeMap<String, usize>,
}

fn compile_components(engine: &Engine, adapter: Option<&[u8]>) -> AnyResult<CompiledPath> {
    let started = Instant::now();
    let mut component_bytes = 0usize;
    let mut component_sizes = BTreeMap::new();
    let components = Components {
        join_args: compile_component(
            engine,
            Mode::JoinArgs,
            "join-args",
            adapter,
            &mut component_bytes,
            &mut component_sizes,
        )?,
        echo_stdin: compile_component(
            engine,
            Mode::EchoStdin,
            "echo-stdin",
            adapter,
            &mut component_bytes,
            &mut component_sizes,
        )?,
        split_channels: compile_component(
            engine,
            Mode::SplitChannels,
            "split-channels",
            adapter,
            &mut component_bytes,
            &mut component_sizes,
        )?,
        script_error: compile_component(
            engine,
            Mode::ScriptError,
            "script-error",
            adapter,
            &mut component_bytes,
            &mut component_sizes,
        )?,
    };
    Ok(CompiledPath {
        components,
        cold_compile: started.elapsed(),
        component_bytes,
        component_sizes,
    })
}

fn run_cases(engine: &Engine, components: &Components, cases: &[Case]) -> AnyResult<usize> {
    for case in cases {
        run_case(engine, components, case)?;
    }
    Ok(cases.len())
}

fn benchmark_warm(
    engine: &Engine,
    components: &Components,
    case: &Case,
    input: &ScriptInput,
) -> AnyResult<Vec<Duration>> {
    let mut samples = Vec::with_capacity(WARM_ITERATIONS);
    for _ in 0..WARM_ITERATIONS {
        let started = Instant::now();
        let result = invoke(engine, component_for(components, &case.mode), input)?;
        validate_result(case, &input.stdin, &result)?;
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    Ok(samples)
}

fn path_report(
    path: &CompiledPath,
    passed: usize,
    total: usize,
    warm: &[Duration],
) -> serde_json::Value {
    serde_json::json!({
        "cases_passed": passed,
        "cases_total": total,
        "cold_component_compile_ms": milliseconds(path.cold_compile),
        "component_bytes": path.component_bytes,
        "components": path.component_sizes,
        "warm_instantiate_call_median_ms": milliseconds(percentile(warm, 50)),
        "warm_instantiate_call_p95_ms": milliseconds(percentile(warm, 95)),
        "warm_iterations": WARM_ITERATIONS,
    })
}

fn compile_component(
    engine: &Engine,
    mode: Mode,
    name: &str,
    adapter: Option<&[u8]>,
    component_bytes: &mut usize,
    component_sizes: &mut BTreeMap<String, usize>,
) -> AnyResult<Component> {
    let program = build_program_component(&mode);
    let bytes = match adapter {
        Some(adapter) => build_composed_component(&program, adapter),
        None => program,
    };
    let rebuilt = match adapter {
        Some(adapter) => build_composed_component(&build_program_component(&mode), adapter),
        None => build_program_component(&mode),
    };
    if bytes != rebuilt {
        return Err(format!("{name}: component encoding is not deterministic").into());
    }
    let component = Component::new(engine, &bytes)?;
    *component_bytes += bytes.len();
    component_sizes.insert(name.to_owned(), bytes.len());
    Ok(component)
}

/// Builds the versioned Adapter candidate. It imports the Program function,
/// canonical-lowers it into the Adapter's private memory, calls it through a
/// core trampoline, and canonical-lifts the trampoline as the Adapter `run`.
/// The trampoline is required because Wasmtime 46.0.1 cannot re-export an
/// imported component function.
fn build_adapter_component() -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let types_ty = builder.type_instance(Some("script-types"), &script_types_instance());
    let types_instance = builder.import(TYPES_IMPORT, ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));
    let program_run = builder.import(PROGRAM_RUN_IMPORT, ComponentTypeRef::Func(run_ty));

    let memory_module = wat::parse_str(adapter_memory_wat()).expect("adapter memory WAT is valid");
    let memory_module = builder.core_module_raw(Some("adapter-memory"), &memory_module);
    let memory_instance = builder.core_instantiate(
        Some("adapter-memory-instance"),
        memory_module,
        Vec::<(&str, ModuleArg)>::new(),
    );
    let memory = builder.core_alias_export(
        Some("adapter-memory"),
        memory_instance,
        "memory",
        ExportKind::Memory,
    );
    let realloc = builder.core_alias_export(
        Some("adapter-realloc"),
        memory_instance,
        "realloc",
        ExportKind::Func,
    );
    let lowered = builder.lower_func(
        Some("program-run-lowered"),
        program_run,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(memory),
            wasm_encoder::CanonicalOption::Realloc(realloc),
        ],
    );
    let adapter_imports = builder.core_instantiate_exports(
        Some("adapter-imports"),
        [
            ("memory", ExportKind::Memory, memory),
            ("realloc", ExportKind::Func, realloc),
        ],
    );
    let program_imports = builder.core_instantiate_exports(
        Some("program-imports"),
        [("run", ExportKind::Func, lowered)],
    );
    let trampoline =
        wat::parse_str(adapter_trampoline_wat()).expect("adapter trampoline WAT is valid");
    let trampoline = builder.core_module_raw(Some("adapter-trampoline"), &trampoline);
    let trampoline_instance = builder.core_instantiate(
        Some("adapter-trampoline-instance"),
        trampoline,
        [
            ("adapter", ModuleArg::Instance(adapter_imports)),
            ("program", ModuleArg::Instance(program_imports)),
        ],
    );
    let run_core = builder.core_alias_export(
        Some("adapter-run"),
        trampoline_instance,
        "run",
        ExportKind::Func,
    );
    let run = builder.lift_func(
        Some("run"),
        run_core,
        run_ty,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(memory),
            wasm_encoder::CanonicalOption::Realloc(realloc),
        ],
    );
    builder.export("run", ComponentExportKind::Func, run, None);
    builder.finish()
}

fn adapter_memory_wat() -> &'static str {
    r#"(module
  (memory (export "memory") 64)
  (global $heap (mut i32) (i32.const 2048))
  (func (export "realloc") (param $old-ptr i32) (param $old-size i32) (param $align i32) (param $new-size i32) (result i32)
    (local $new-ptr i32)
    (local.set $new-ptr
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get $align) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get $align))))
    (if (i32.ne (local.get $old-ptr) (i32.const 0))
      (then
        (memory.copy
          (local.get $new-ptr)
          (local.get $old-ptr)
          (select
            (local.get $old-size)
            (local.get $new-size)
            (i32.lt_u (local.get $old-size) (local.get $new-size))))))
    (global.set $heap (i32.add (local.get $new-ptr) (local.get $new-size)))
    (local.get $new-ptr)))"#
}

fn adapter_trampoline_wat() -> &'static str {
    r#"(module
  (import "adapter" "memory" (memory 64))
  (import "adapter" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
  (import "program" "run" (func $program-run (param i32 i32 i32 i32 i32)))
  (func (export "run") (param $args-ptr i32) (param $args-len i32) (param $stdin-ptr i32) (param $stdin-len i32) (result i32)
    (local $ret i32)
    (local.set $ret (call $realloc (i32.const 0) (i32.const 0) (i32.const 8) (i32.const 32)))
    (call $program-run
      (local.get $args-ptr)
      (local.get $args-len)
      (local.get $stdin-ptr)
      (local.get $stdin-len)
      (local.get $ret))
    (local.get $ret)))"#
}

fn build_composed_component(program: &[u8], adapter: &[u8]) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let types_ty = builder.type_instance(Some("script-types"), &script_types_instance());
    let types_instance = builder.import(TYPES_IMPORT, ComponentTypeRef::Instance(types_ty));
    let program_component = builder.component_raw(Some("program"), program);
    let program_instance = builder.instantiate(
        Some("program-instance"),
        program_component,
        [(TYPES_IMPORT, ComponentExportKind::Instance, types_instance)],
    );
    let program_run = builder.alias_export(program_instance, "run", ComponentExportKind::Func);
    let adapter_component = builder.component_raw(Some("adapter"), adapter);
    let adapter_instance = builder.instantiate(
        Some("adapter-instance"),
        adapter_component,
        [
            (TYPES_IMPORT, ComponentExportKind::Instance, types_instance),
            (PROGRAM_RUN_IMPORT, ComponentExportKind::Func, program_run),
        ],
    );
    let run = builder.alias_export(adapter_instance, "run", ComponentExportKind::Func);
    builder.export("run", ComponentExportKind::Func, run, None);
    builder.finish()
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Declares the frozen `sico:script/types@0.1.0` value shape. Component-level
/// functions may only reference named types, and exporting a type inside an
/// instance type occupies one type index holding the named id, so each type
/// is exported right after its declaration and every later reference points
/// at the export index, not the anonymous declaration index.
fn script_types_instance() -> InstanceType {
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

/// Builds a self-contained Program Component: a hard-coded guest Core Wasm
/// module implementing `run` for one fixture mode, lifted to the frozen
/// Script signature with the real Canonical ABI (guest memory + realloc).
fn build_program_component(mode: &Mode) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let types_ty = builder.type_instance(Some("script-types"), &script_types_instance());
    let types_instance = builder.import(TYPES_IMPORT, ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    let module = wat::parse_str(guest_wat(mode)).expect("guest WAT must be valid");
    let module = builder.core_module_raw(Some("guest"), &module);
    let instance = builder.core_instantiate(
        Some("guest-instance"),
        module,
        Vec::<(&str, ModuleArg)>::new(),
    );
    let memory =
        builder.core_alias_export(Some("guest-memory"), instance, "memory", ExportKind::Memory);
    let realloc =
        builder.core_alias_export(Some("guest-realloc"), instance, "realloc", ExportKind::Func);
    let run_core = builder.core_alias_export(Some("guest-run"), instance, "run", ExportKind::Func);
    let run = builder.lift_func(
        Some("run"),
        run_core,
        run_ty,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(memory),
            wasm_encoder::CanonicalOption::Realloc(realloc),
        ],
    );
    builder.export("run", ComponentExportKind::Func, run, None);
    builder.finish()
}

/// Guest Core Wasm for one fixture mode. Canonical ABI for
/// `run: func(input: { arguments: list<string>, stdin: list<u8> }) -> result<{ stdout: list<u8>, stderr: list<u8>, exit-code: s64 }, { code: enum, message: string }>`:
/// flattened params are (args_ptr, args_len, stdin_ptr, stdin_len); the
/// flattened result is too wide, so the core function returns a single i32
/// pointer to the 32-byte result area (discriminant u8 at +0, payload at
/// +8). `string-list` entries are (ptr, len) pairs.
fn guest_wat(mode: &Mode) -> String {
    let (body, data) = match mode {
        Mode::EchoStdin => (
            r#"
  (func (export "run") (param $args_ptr i32) (param $args_len i32) (param $stdin_ptr i32) (param $stdin_len i32) (result i32)
    (local $ret i32)
    (local.set $ret (call $alloc (i32.const 32)))
    (i32.store8 (local.get $ret) (i32.const 0))
    (i32.store (i32.add (local.get $ret) (i32.const 8)) (local.get $stdin_ptr))
    (i32.store (i32.add (local.get $ret) (i32.const 12)) (local.get $stdin_len))
    (i32.store (i32.add (local.get $ret) (i32.const 16)) (i32.const 0))
    (i32.store (i32.add (local.get $ret) (i32.const 20)) (i32.const 0))
    (i64.store (i32.add (local.get $ret) (i32.const 24)) (i64.const 0))
    (local.get $ret))
"#,
            "",
        ),
        Mode::SplitChannels => (
            r#"
  (func (export "run") (param $args_ptr i32) (param $args_len i32) (param $stdin_ptr i32) (param $stdin_len i32) (result i32)
    (local $ret i32)
    (local.set $ret (call $alloc (i32.const 32)))
    (i32.store8 (local.get $ret) (i32.const 0))
    (i32.store (i32.add (local.get $ret) (i32.const 8)) (local.get $stdin_ptr))
    (i32.store (i32.add (local.get $ret) (i32.const 12)) (local.get $stdin_len))
    (i32.store (i32.add (local.get $ret) (i32.const 16)) (i32.const 1024))
    (i32.store (i32.add (local.get $ret) (i32.const 20)) (i32.const 7))
    (i64.store (i32.add (local.get $ret) (i32.const 24)) (i64.const 7))
    (local.get $ret))
"#,
            r#"
  (data (i32.const 1024) "warning")
"#,
        ),
        Mode::ScriptError => (
            r#"
  (func (export "run") (param $args_ptr i32) (param $args_len i32) (param $stdin_ptr i32) (param $stdin_len i32) (result i32)
    (local $ret i32)
    (local.set $ret (call $alloc (i32.const 32)))
    (i32.store8 (local.get $ret) (i32.const 1))
    (i32.store8 (i32.add (local.get $ret) (i32.const 8)) (i32.const 0))
    (i32.store (i32.add (local.get $ret) (i32.const 12)) (i32.const 1056))
    (i32.store (i32.add (local.get $ret) (i32.const 16)) (i32.const 22))
    (local.get $ret))
"#,
            r#"
  (data (i32.const 1056) "fixture rejected input")
"#,
        ),
        Mode::JoinArgs => (
            r#"
  (func (export "run") (param $args_ptr i32) (param $args_len i32) (param $stdin_ptr i32) (param $stdin_len i32) (result i32)
    (local $ret i32) (local $i i32) (local $total i32) (local $out i32) (local $dst i32) (local $len i32)
    (local.set $ret (call $alloc (i32.const 32)))
    (block $sum_done
      (loop $sum
        (br_if $sum_done (i32.ge_u (local.get $i) (local.get $args_len)))
        (local.set $total
          (i32.add
            (local.get $total)
            (i32.load offset=4
              (i32.add (local.get $args_ptr)
                (i32.mul (local.get $i) (i32.const 8))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $sum)))
    (if (i32.gt_u (local.get $args_len) (i32.const 0))
      (then
        (local.set $total
          (i32.add (local.get $total)
            (i32.sub (local.get $args_len) (i32.const 1))))))
    (local.set $out (call $alloc (local.get $total)))
    (local.set $dst (local.get $out))
    (local.set $i (i32.const 0))
    (block $copy_done
      (loop $copy
        (br_if $copy_done (i32.ge_u (local.get $i) (local.get $args_len)))
        (local.set $len
          (i32.load offset=4
            (i32.add (local.get $args_ptr)
              (i32.mul (local.get $i) (i32.const 8)))))
        (memory.copy
          (local.get $dst)
          (i32.load
            (i32.add (local.get $args_ptr)
              (i32.mul (local.get $i) (i32.const 8))))
          (local.get $len))
        (local.set $dst (i32.add (local.get $dst) (local.get $len)))
        (if (i32.lt_u (i32.add (local.get $i) (i32.const 1)) (local.get $args_len))
          (then
            (i32.store8 (local.get $dst) (i32.const 124))
            (local.set $dst (i32.add (local.get $dst) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)))
    (i32.store8 (local.get $ret) (i32.const 0))
    (i32.store (i32.add (local.get $ret) (i32.const 8)) (local.get $out))
    (i32.store (i32.add (local.get $ret) (i32.const 12)) (local.get $total))
    (i32.store (i32.add (local.get $ret) (i32.const 16)) (i32.const 0))
    (i32.store (i32.add (local.get $ret) (i32.const 20)) (i32.const 0))
    (i64.store (i32.add (local.get $ret) (i32.const 24)) (i64.const 0))
    (local.get $ret))
"#,
            "",
        ),
    };
    format!(
        r#"(module
  (memory (export "memory") 64)
  (global $heap (mut i32) (i32.const 2048))
  (func $alloc (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (i32.and (i32.add (global.get $heap) (i32.const 7)) (i32.const -8)))
    (global.set $heap (i32.add (local.get $ptr) (local.get $size)))
    (local.get $ptr))
  (func (export "realloc") (param $old_ptr i32) (param $old_size i32) (param $align i32) (param $new_size i32) (result i32)
    (local $new_ptr i32)
    (local.set $new_ptr
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get $align) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get $align))))
    (if (i32.ne (local.get $old_ptr) (i32.const 0))
      (then
        (memory.copy
          (local.get $new_ptr)
          (local.get $old_ptr)
          (select
            (local.get $old_size)
            (local.get $new_size)
            (i32.lt_u (local.get $old_size) (local.get $new_size))))))
    (global.set $heap (i32.add (local.get $new_ptr) (local.get $new_size)))
    (local.get $new_ptr))
{body}{data})"#
    )
}

fn run_case(engine: &Engine, components: &Components, case: &Case) -> AnyResult<()> {
    let input = case_input(case)?;
    if input.stdin.len() > STDIN_LIMIT {
        if case.expected.runner_error.as_deref() != Some("stdin-limit")
            || case.expected.tool_exit != Some(125)
        {
            return Err(format!("{}: unexpected stdin limit expectation", case.name).into());
        }
        return Ok(());
    }
    let result = invoke(engine, component_for(components, &case.mode), &input)?;
    validate_result(case, &input.stdin, &result)
}

fn invoke(engine: &Engine, component: &Component, input: &ScriptInput) -> AnyResult<ScriptResult> {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker.instantiate(&mut store, component)?;
    let run = instance
        .get_func(&mut store, "run")
        .ok_or("component does not export run")?;
    let params = [input_to_val(input)];
    let mut results = [Val::Result(Ok(None))];
    run.call(&mut store, &params, &mut results)?;
    val_to_result(&results[0])
}

fn case_input(case: &Case) -> AnyResult<ScriptInput> {
    let stdin = if let Some(size) = case.stdin_size {
        let pattern = decode_hex(case.stdin_pattern_hex.as_deref().unwrap_or_default())?;
        if pattern.is_empty() && size != 0 {
            return Err(format!("{}: non-empty generated stdin needs a pattern", case.name).into());
        }
        pattern.into_iter().cycle().take(size).collect()
    } else {
        decode_hex(&case.stdin_hex)?
    };
    Ok(ScriptInput {
        args: case.args.clone(),
        stdin,
    })
}

fn input_to_val(input: &ScriptInput) -> Val {
    Val::Record(vec![
        (
            "arguments".to_owned(),
            Val::List(input.args.iter().cloned().map(Val::String).collect()),
        ),
        (
            "stdin".to_owned(),
            Val::List(input.stdin.iter().copied().map(Val::U8).collect()),
        ),
    ])
}

fn field<'a>(fields: &'a [(String, Val)], name: &str) -> wasmtime::Result<&'a Val> {
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .ok_or_else(|| wasmtime::Error::msg(format!("missing field {name}")))
}

fn val_to_bytes(value: &Val) -> wasmtime::Result<Vec<u8>> {
    let Val::List(values) = value else {
        return Err(wasmtime::Error::msg("bytes is not a list"));
    };
    values
        .iter()
        .map(|value| match value {
            Val::U8(value) => Ok(*value),
            _ => Err(wasmtime::Error::msg("bytes item is not u8")),
        })
        .collect()
}

fn val_to_result(value: &Val) -> AnyResult<ScriptResult> {
    match value {
        Val::Result(Ok(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err("script output is not a record".into());
            };
            let exit = match field(fields, "exit-code")? {
                Val::S64(exit) => *exit,
                _ => return Err("script output exit is not s64".into()),
            };
            Ok(ScriptResult::Output {
                stdout: val_to_bytes(field(fields, "stdout")?)?,
                stderr: val_to_bytes(field(fields, "stderr")?)?,
                exit,
            })
        }
        Val::Result(Err(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err("script error is not a record".into());
            };
            let kind = match field(fields, "code")? {
                Val::Enum(kind) => kind.clone(),
                _ => return Err("script error code is not an enum".into()),
            };
            let message = match field(fields, "message")? {
                Val::String(message) => message.clone(),
                _ => return Err("script error message is not text".into()),
            };
            Ok(ScriptResult::Error { kind, message })
        }
        _ => Err("script result has an invalid shape".into()),
    }
}

fn validate_result(case: &Case, stdin: &[u8], result: &ScriptResult) -> AnyResult<()> {
    match result {
        ScriptResult::Output {
            stdout,
            stderr,
            exit,
        } => {
            let expected_stdout = if case.expected.echo_stdin == Some(true) {
                stdin.to_vec()
            } else {
                decode_hex(case.expected.stdout_hex.as_deref().unwrap_or_default())?
            };
            let expected_stderr =
                decode_hex(case.expected.stderr_hex.as_deref().unwrap_or_default())?;
            if stdout != &expected_stdout
                || stderr != &expected_stderr
                || Some(*exit) != case.expected.exit
            {
                return Err(format!("{}: output mismatch", case.name).into());
            }
        }
        ScriptResult::Error { kind, message } => {
            if Some(kind.as_str()) != case.expected.error_kind.as_deref()
                || Some(message.as_str()) != case.expected.error_message.as_deref()
            {
                return Err(format!("{}: error mismatch", case.name).into());
            }
        }
    }
    Ok(())
}

fn decode_hex(value: &str) -> AnyResult<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex input has odd length".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| Ok(u8::from_str_radix(&value[index..index + 2], 16)?))
        .collect()
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let index = ((samples.len() - 1) * percentile).div_ceil(100);
    samples[index]
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
