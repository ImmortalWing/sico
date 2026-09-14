//! Deterministic Core Wasm generation from independently verified Sico IR.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sico_ir::{
    Block, BlockId, Function as IrFunction, FunctionId, Module as IrModule, Operation, Pattern,
    Terminator, Type, ValueId, VerifyError, verify,
};
use sico_observability::{
    CompilerIdentity, DEBUG_IDENTITY_SCHEMA, DEBUG_LINK_SCHEMA, DEBUG_MAP_SCHEMA, DebugBinding,
    DebugDocument, DebugFunction, DebugIdentity, DebugLink, DebugMap, DebugMapping, DebugSpan,
    MAX_DEBUG_MAP_BYTES, SourceIdentity, canonical_json, link_component, sha256_hex,
    validate_debug_map, verify_debug_artifacts,
};
use wasm_encoder::{
    BlockType, CanonicalOption, CodeSection, ComponentBuilder, ComponentExportKind,
    ComponentValType, ConstExpr, DataSection, DataSegment, DataSegmentMode, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemArg, MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType,
    TypeSection, ValType,
};
use wasmparser::{Parser, Payload};

mod canonical;
mod json;
pub mod package_builder;
mod stdlib;

use canonical::{Flat, ScriptAbi};
pub use canonical::{FsUse, SCRIPT_WIT, StreamUse, script_types_instance, wrap_script_component};
pub use sico_ir;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodegenError {
    InvalidIr(Vec<VerifyError>),
    ModuleTooLarge {
        functions: usize,
    },
    AsyncUnsupported {
        function: String,
        feature: String,
        contract: &'static str,
    },
    Unsupported {
        function: String,
        feature: String,
    },
    IntegerOutsideProvenI64 {
        function: String,
        bytes: usize,
    },
    DebugArtifact(String),
}

/// Exact source/compiler inputs needed to bind a deterministic debug artifact.
pub struct DebugBuildInput<'a> {
    pub document_id: &'a str,
    pub source_bytes: &'a [u8],
    pub display_uri: Option<&'a str>,
    pub compiler_package: &'a str,
    pub compiler_version: &'a str,
    pub compiler_executable_sha256: &'a str,
    pub adapter_identities: Vec<String>,
    pub wit_identities: Vec<String>,
}

/// A linked Component and its canonical sidecar/identity JSON bytes.
pub struct DebugArtifact {
    pub component: Vec<u8>,
    pub debug_map: Vec<u8>,
    pub identity: Vec<u8>,
}

#[derive(Clone)]
struct LocalDebugMapping {
    start: usize,
    end: usize,
    source: Option<sico_ir::SourceRange>,
    generated: bool,
}

struct CompiledFunction {
    body: Function,
    mappings: Vec<LocalDebugMapping>,
}

struct CoreDebugFunction {
    id: String,
    core_index: u32,
    mappings: Vec<LocalDebugMapping>,
}

struct CoreCompilation {
    bytes: Vec<u8>,
    functions: Vec<CoreDebugFunction>,
}

/// Compiles the proven scalar/control subset to deterministic Core Wasm.
///
/// `Int` parameters and non-constant `Int` results are refused: RFC-0003 has
/// not accepted a fixed-width representation, so this backend only emits an
/// i64 when the mathematical value is compile-time proven to fit.
///
/// # Errors
///
/// Returns verifier errors or a typed unsupported/representation refusal.
pub fn compile(module: &IrModule) -> Result<Vec<u8>, CodegenError> {
    Ok(compile_core(module, CoreAbi::Direct)?.bytes)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CoreAbi {
    Direct,
    ComponentLift,
}

fn compile_core(module: &IrModule, abi: CoreAbi) -> Result<CoreCompilation, CodegenError> {
    let errors = verify(module);
    if !errors.is_empty() {
        return Err(CodegenError::InvalidIr(errors));
    }
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    let mut debug_functions = Vec::with_capacity(module.functions.len());
    let function_indices = module
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            u32::try_from(index)
                .map(|index| (function.id, index))
                .map_err(|_| CodegenError::ModuleTooLarge {
                    functions: module.functions.len(),
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let variant_tags = VariantTags::new(module, None)?;
    for (index, function) in module.functions.iter().enumerate() {
        if !function.effects.is_empty() {
            return Err(unsupported(
                &function.name,
                "effectful function without a Component host adapter",
            ));
        }
        let function_index = u32::try_from(index).map_err(|_| CodegenError::ModuleTooLarge {
            functions: module.functions.len(),
        })?;
        let params = function
            .parameters
            .iter()
            .map(|parameter| lower_parameter_type(&function.name, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        let results = lower_result_type(&function.name, &function.return_type, abi)?;
        types.ty().function(params, results);
        functions.function(function_index);
        exports.export(&function.name, ExportKind::Func, function_index);
        let compiled = compile_function(function, abi, &function_indices, &variant_tags, None)?;
        code.function(&compiled.body);
        debug_functions.push(CoreDebugFunction {
            id: format!("function-{:010}", function.id.0),
            core_index: function_index,
            mappings: compiled.mappings,
        });
    }
    let mut output = Module::new();
    output.section(&types);
    output.section(&functions);
    let needs_memory = abi == CoreAbi::ComponentLift
        && module
            .functions
            .iter()
            .any(|function| is_checked_fixed_result(&function.return_type));
    let mut memories = MemorySection::new();
    if needs_memory {
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        output.section(&memories);
        exports.export("memory", ExportKind::Memory, 0);
    }
    output.section(&exports);
    output.section(&code);
    Ok(CoreCompilation {
        bytes: output.finish(),
        functions: debug_functions,
    })
}

/// Compiles the proven scalar/control subset to a deterministic WebAssembly Component.
///
/// Each Core Wasm export is canonically lifted to a root Component function.
/// The fixed-width numeric `Result` subset uses a private Canonical ABI return
/// area. Other aggregates, resources, strings, and async boundaries remain
/// refused until their memory and ownership adapters are implemented.
///
/// # Errors
///
/// Returns verifier errors or a typed unsupported/representation refusal.
pub fn compile_component(module: &IrModule) -> Result<Vec<u8>, CodegenError> {
    Ok(compile_component_parts(module)?.1)
}

/// Compiles and binds a deterministic debug map and identity to a Component.
///
/// # Errors
///
/// Returns the ordinary code-generation refusals or a closed debug-contract
/// failure if any identity, offset, limit or digest invariant is violated.
pub fn compile_component_with_debug(
    module: &IrModule,
    input: &DebugBuildInput<'_>,
) -> Result<DebugArtifact, CodegenError> {
    let (core, component) = compile_component_parts(module)?;
    build_debug_artifact(&core, &component, "sico-core", input)
}

fn compile_component_parts(module: &IrModule) -> Result<(CoreCompilation, Vec<u8>), CodegenError> {
    let core = compile_core(module, CoreAbi::ComponentLift)?;
    let mut builder = ComponentBuilder::default();
    let core_module = builder.core_module_raw(Some("sico-core"), &core.bytes);
    let core_instance = builder.core_instantiate(
        Some("sico-core"),
        core_module,
        std::iter::empty::<(&str, ModuleArg)>(),
    );
    let core_memory = module
        .functions
        .iter()
        .any(|function| is_checked_fixed_result(&function.return_type))
        .then(|| {
            builder.core_alias_export(Some("memory"), core_instance, "memory", ExportKind::Memory)
        });

    let mut numeric_types = None;
    for function in &module.functions {
        let core_function = builder.core_alias_export(
            Some(&function.name),
            core_instance,
            &function.name,
            ExportKind::Func,
        );
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| {
                component_type(
                    &mut builder,
                    &mut numeric_types,
                    &function.name,
                    &parameter.ty,
                )
                .map(|ty| (component_extern_name(&parameter.name), ty))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let result = match function.return_type {
            Type::Unit => None,
            _ => Some(component_type(
                &mut builder,
                &mut numeric_types,
                &function.name,
                &function.return_type,
            )?),
        };
        let (type_index, mut function_type) = builder.type_function(Some(&function.name));
        function_type.params(parameters.iter().map(|(name, ty)| (name.as_str(), *ty)));
        function_type.result(result);
        let options = core_memory
            .filter(|_| is_checked_fixed_result(&function.return_type))
            .map(CanonicalOption::Memory);
        let lifted = builder.lift_func(Some(&function.name), core_function, type_index, options);
        builder.export(
            component_extern_name(&function.name),
            ComponentExportKind::Func,
            lifted,
            None,
        );
    }

    Ok((core, builder.finish()))
}

#[allow(clippy::too_many_lines)]
fn build_debug_artifact(
    core: &CoreCompilation,
    component_code: &[u8],
    core_module: &str,
    input: &DebugBuildInput<'_>,
) -> Result<DebugArtifact, CodegenError> {
    let source_sha256 = sha256_hex(input.source_bytes);
    let component_code_sha256 = sha256_hex(component_code);
    let core_base = embedded_guest_core_base(component_code)?;
    let body_ranges = Parser::new(0)
        .parse_all(&core.bytes)
        .filter_map(|payload| match payload {
            Ok(Payload::CodeSectionEntry(body)) => Some(Ok(body.range())),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CodegenError::DebugArtifact(format!("invalid emitted core module: {error}"))
        })?;
    if body_ranges.len() < core.functions.len() {
        return Err(CodegenError::DebugArtifact(
            "emitted core module lost a compiled function body".into(),
        ));
    }

    let mut functions = Vec::with_capacity(core.functions.len());
    let mut mappings = Vec::new();
    for (function, body_range) in core.functions.iter().zip(body_ranges) {
        let function_id = function.id.clone();
        functions.push(DebugFunction {
            id: function_id.clone(),
            core_module: core_module.into(),
            component_function: u64::from(function.core_index),
        });
        for local in &function.mappings {
            if local.start >= local.end {
                continue;
            }
            let instruction_start = body_range
                .start
                .checked_add(local.start)
                .and_then(|value| value.checked_add(core_base))
                .and_then(|value| u64::try_from(value).ok())
                .ok_or_else(|| CodegenError::DebugArtifact("instruction offset overflow".into()))?;
            let instruction_end = body_range
                .start
                .checked_add(local.end)
                .and_then(|value| value.checked_add(core_base))
                .and_then(|value| u64::try_from(value).ok())
                .ok_or_else(|| CodegenError::DebugArtifact("instruction offset overflow".into()))?;
            mappings.push(DebugMapping {
                core_module: core_module.into(),
                component_function: u64::from(function.core_index),
                instruction_start,
                instruction_end,
                function_id: function_id.clone(),
                source: local.source.map(|source| DebugSpan {
                    document_id: input.document_id.into(),
                    start: u64::from(source.start),
                    end: u64::from(source.end),
                }),
                call_site: None,
                inline_parent: None,
                generated: local.generated,
            });
        }
    }
    functions.sort_by(|left, right| left.id.cmp(&right.id));
    mappings.sort_by(|left, right| {
        (
            &left.core_module,
            left.component_function,
            left.instruction_start,
            left.instruction_end,
            &left.function_id,
        )
            .cmp(&(
                &right.core_module,
                right.component_function,
                right.instruction_start,
                right.instruction_end,
                &right.function_id,
            ))
    });

    let map = DebugMap {
        schema: DEBUG_MAP_SCHEMA.into(),
        binding: DebugBinding {
            source_sha256: source_sha256.clone(),
            compiler_executable_sha256: input.compiler_executable_sha256.into(),
            component_code_sha256: component_code_sha256.clone(),
        },
        coordinate_system: "utf8-byte-half-open".into(),
        documents: vec![DebugDocument {
            id: input.document_id.into(),
            sha256: source_sha256.clone(),
            byte_length: u64::try_from(input.source_bytes.len())
                .map_err(|_| CodegenError::DebugArtifact("source length overflow".into()))?,
        }],
        functions,
        mappings,
    };
    validate_debug_map(&map).map_err(|error| CodegenError::DebugArtifact(error.to_string()))?;
    let debug_map =
        canonical_json(&map).map_err(|error| CodegenError::DebugArtifact(error.to_string()))?;
    if debug_map.len() > MAX_DEBUG_MAP_BYTES {
        return Err(CodegenError::DebugArtifact(
            "debug map exceeds the 16 MiB contract".into(),
        ));
    }
    let debug_map_sha256 = sha256_hex(&debug_map);
    let link = DebugLink {
        schema: DEBUG_LINK_SCHEMA.into(),
        source_sha256: source_sha256.clone(),
        compiler_executable_sha256: input.compiler_executable_sha256.into(),
        component_code_sha256: component_code_sha256.clone(),
        debug_map_sha256: debug_map_sha256.clone(),
    };
    let component = link_component(component_code, &link)
        .map_err(|error| CodegenError::DebugArtifact(error.to_string()))?;
    let mut adapter_identities = input.adapter_identities.clone();
    let mut wit_identities = input.wit_identities.clone();
    adapter_identities.sort();
    wit_identities.sort();
    let identity = DebugIdentity {
        schema: DEBUG_IDENTITY_SCHEMA.into(),
        source: SourceIdentity {
            document_id: input.document_id.into(),
            sha256: source_sha256,
            byte_length: u64::try_from(input.source_bytes.len())
                .map_err(|_| CodegenError::DebugArtifact("source length overflow".into()))?,
            display_uri: input.display_uri.map(str::to_owned),
        },
        compiler: CompilerIdentity {
            package: input.compiler_package.into(),
            version: input.compiler_version.into(),
            executable_sha256: input.compiler_executable_sha256.into(),
        },
        language_semantics: "sico.semantics.v0".into(),
        ir_schema: sico_ir::SCHEMA.into(),
        component_code_sha256,
        component_sha256: sha256_hex(&component),
        debug_map_sha256,
        adapter_identities,
        wit_identities,
    };
    let identity = canonical_json(&identity)
        .map_err(|error| CodegenError::DebugArtifact(error.to_string()))?;
    verify_debug_artifacts(&component, &debug_map, &identity)
        .map_err(|error| CodegenError::DebugArtifact(error.to_string()))?;
    Ok(DebugArtifact {
        component,
        debug_map,
        identity,
    })
}

fn embedded_guest_core_base(component: &[u8]) -> Result<usize, CodegenError> {
    Parser::new(0)
        .parse_all(component)
        .filter_map(|payload| match payload {
            Ok(Payload::ModuleSection {
                unchecked_range, ..
            }) => Some(Ok(unchecked_range.start)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CodegenError::DebugArtifact(format!("invalid emitted Component: {error}"))
        })?
        .into_iter()
        .last()
        .ok_or_else(|| CodegenError::DebugArtifact("Component has no embedded guest core".into()))
}

fn component_extern_name(name: &str) -> String {
    name.replace('_', "-")
}

/// Script program emission context: the WIT-driven ABI plus the module-wide
/// static data segment and bounded arena layout.
pub(crate) struct ScriptEmit {
    abi: ScriptAbi,
    data_offsets: BTreeMap<Vec<u8>, u32>,
    data_bytes: Vec<u8>,
    arena_base: u32,
    alloc_index: u32,
    realloc_index: u32,
    post_return_index: u32,
    helpers: BTreeMap<&'static str, u32>,
    fs: canonical::FsUse,
    /// Core function index of each used `sico.fs.*` import, by intrinsic.
    fs_indices: BTreeMap<&'static str, u32>,
    streams: canonical::StreamUse,
    /// Core function index of each used `sico.stream.*` import, by intrinsic.
    stream_indices: BTreeMap<&'static str, u32>,
    http: bool,
    /// Core function index of the `sico.http.request` import, when used.
    http_index: Option<u32>,
    /// STEP-0136: the program calls `sico.http2.request` (http@0.2.0).
    http2: bool,
    /// Core function index of the `sico.http2.request` import, when used.
    http2_index: Option<u32>,
    /// Absolute data-segment address of the http-error case-name table
    /// (eight pointer+length pairs in WIT declaration order), when used.
    http2_error_table: Option<u32>,
    /// RFC-0039 §2.4 (STEP-0147): user-WIT package imports, deterministic
    /// (instance-name, function-name) order, with their core indices.
    user_imports: Vec<canonical::UserImport>,
    /// Core import index per full user import name
    /// (`sico:user/<interface>@<version>.<function>`).
    user_indices: BTreeMap<String, u32>,
}

/// Maps an intrinsic to the helper functions it needs at runtime.
fn helper_dependencies(name: &str) -> &'static [&'static str] {
    // STEP-0131 collection helpers are monomorphized per instantiation; each
    // self-contained helper leaks its own canonical name once (bounded by the
    // closed `collection_intrinsic` grammar: 11 operations × ≤25 instantiations).
    if sico_ir::collection_intrinsic(name).is_some() {
        let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
        return Box::leak(vec![leaked].into_boxed_slice());
    }
    match name {
        "sico.json.is_valid" => &[
            "sico.json.ws",
            "sico.json.string",
            "sico.json.number",
            "sico.json.scan",
        ],
        "sico.json.get" | "sico.json.has" => &[
            "sico.json.ws",
            "sico.json.string",
            "sico.json.number",
            "sico.json.scan",
            "sico.json.find",
        ],
        "sico.json.quote" => &["sico.json.quote"],
        // RFC-0045 stdlib batch 2 (STEP-0174). `list.sort`/`min`/`max`
        // share the `sico.text.compare` helper via cross-helper calls.
        "sico.bytes.equal" => &["sico.bytes.equal"],
        "sico.text.compare" => &["sico.text.compare"],
        "sico.text.char_at" => &["sico.text.char_loc"],
        "sico.text.format" => &["sico.text.format"],
        "sico.text.ends_with" => &["sico.text.ends_with"],
        // STEP-0179: the leading-whitespace scan is emitted inline; it
        // registers no helper.
        "sico.text.leading_spaces" => &[],
        "sico.list.sort" => &["sico.list.sort", "sico.text.compare"],
        "sico.list.min" => &["sico.list.min", "sico.text.compare"],
        "sico.list.max" => &["sico.list.max", "sico.text.compare"],
        _ => INTRINSIC_HELPERS
            .iter()
            .find(|helper| **helper == name)
            .map_or(&[], |helper| std::slice::from_ref(helper)),
    }
}

/// Script standard-library intrinsics implemented by emitted helper
/// functions (STEP-0083); every other registered intrinsic is emitted inline.
const INTRINSIC_HELPERS: &[&str] = &[
    "sico.bytes.concat",
    "sico.bytes.utf8_decode",
    "sico.i64.to_text",
    "sico.json.find",
    "sico.json.quote",
    "sico.json.scan",
    "sico.list.append",
    "sico.text.concat",
    "sico.text.join",
    "sico.text.split_lines",
    "sico.text.split_words",
    "sico.text.contains",
    "sico.text.ends_with",
    "sico.text.length",
    "sico.text.leading_spaces",
    "sico.text.starts_with",
    "sico.text.trim",
    "sico.u64.to_text",
];

/// Compiles a verified module to a deterministic `sico:script/program@0.1.0`
/// Program Component.
///
/// Exactly one function named `run` must have the signature
/// `run(input: ScriptInput) -> Result[ScriptOutput, ScriptError]`; every other
/// function stays an internal scalar helper. The core module owns one bounded
/// arena reclaimed per call by `cabi_post_run`, and `run` crosses the boundary
/// through the Canonical ABI layouts derived from the frozen Script WIT.
///
/// # Errors
///
/// Returns verifier errors, a typed refusal for a non-Script signature or an
/// aggregate feature outside the frozen Script v0 boundary.
///
/// # Panics
///
/// Panics only if the repository-frozen Script WIT no longer matches the
/// embedded boundary or the intrinsic helper table is inconsistent.
#[allow(clippy::too_many_lines)]
pub fn compile_script_program(module: &IrModule) -> Result<Vec<u8>, CodegenError> {
    Ok(compile_script_program_parts(module)?.1)
}

/// Compiles a Script Component and emits its deterministic debug sidecars.
///
/// # Errors
///
/// Returns the ordinary Script boundary refusals or a closed debug-contract
/// failure if any identity, offset, limit or digest invariant is violated.
pub fn compile_script_program_with_debug(
    module: &IrModule,
    input: &DebugBuildInput<'_>,
) -> Result<DebugArtifact, CodegenError> {
    let (core, component) = compile_script_program_parts(module)?;
    build_debug_artifact(&core, &component, "sico-script-core", input)
}

#[allow(clippy::too_many_lines, clippy::similar_names)]
fn compile_script_program_parts(
    module: &IrModule,
) -> Result<(CoreCompilation, Vec<u8>), CodegenError> {
    let errors = verify(module);
    if !errors.is_empty() {
        return Err(CodegenError::InvalidIr(errors));
    }
    let run = module
        .functions
        .iter()
        .find(|function| function.name == "run")
        .ok_or_else(|| unsupported("run", "missing Script entry run"))?;
    let boundary_return = Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    };
    if run.parameters.len() != 1
        || run.parameters[0].ty != Type::Named("ScriptInput".into())
        || run.return_type != boundary_return
    {
        return Err(unsupported(
            "run",
            "Script entry must be run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]",
        ));
    }

    let (mut data_offsets, mut data_bytes) = script_data(module)?;
    // `sico.fs.*`/`sico.stream.*` intrinsics become Component imports; the
    // transport realloc import plus one core import per used function shift
    // every defined function index (STEP-0083 scoped files, RFC-0030 streams).
    let mut fs = canonical::FsUse::default();
    let mut streams = canonical::StreamUse::default();
    let mut http = false;
    let mut http2 = false;
    let mut user_call_names = std::collections::BTreeSet::new();
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Operation::Intrinsic { name, .. } = &instruction.operation {
                    match name.as_str() {
                        "sico.fs.read" => fs.read = true,
                        "sico.fs.exists" => fs.exists = true,
                        "sico.fs.write" => fs.write = true,
                        "sico.stream.stdin" => streams.stdin = true,
                        "sico.stream.stdout" => streams.stdout = true,
                        "sico.stream.stderr" => streams.stderr = true,
                        "sico.stream.read" => streams.read = true,
                        "sico.stream.write" => streams.write = true,
                        "sico.stream.flush" => streams.flush = true,
                        "sico.stream.pump" => streams.pump = true,
                        "sico.stream.close_input" => streams.close_input = true,
                        "sico.stream.close_output" => streams.close_output = true,
                        "sico.http.request" => http = true,
                        "sico.http2.request" => http2 = true,
                        _ => {}
                    }
                    // RFC-0039 §2.4 (STEP-0147): user-WIT package imports
                    // carry their full `sico:user/<interface>@<version>`
                    // identity in the intrinsic name.
                    if name.starts_with("sico:user/") {
                        user_call_names.insert(name.clone());
                    }
                }
            }
        }
    }
    // Group the called user functions by instance and attach the frozen
    // signatures from the lowering's import table. Unknown names are a
    // typed refusal (defensive: lowering emits only verified signatures).
    let mut user_import_map: BTreeMap<
        String,
        BTreeMap<String, (Vec<sico_ir::Type>, sico_ir::Type)>,
    > = BTreeMap::new();
    for name in &user_call_names {
        let (instance, function) = split_user_import_name(name)
            .ok_or_else(|| unsupported(name, "user import identity"))?;
        let signature = module
            .import_signatures
            .get(name.as_str())
            .ok_or_else(|| unsupported(name, "user import signature"))?;
        user_import_map
            .entry(instance.to_owned())
            .or_default()
            .insert(
                function.to_owned(),
                (signature.0.clone(), signature.1.clone()),
            );
    }
    let user_imports: Vec<canonical::UserImport> = user_import_map
        .into_iter()
        .map(|(name, functions)| canonical::UserImport {
            name,
            functions: functions
                .into_iter()
                .map(|(name, (parameters, result))| canonical::UserFunction {
                    name,
                    parameters,
                    result,
                })
                .collect(),
        })
        .collect();
    let user_function_count: u32 = user_imports
        .iter()
        .map(|import| u32::try_from(import.functions.len()).unwrap_or(u32::MAX))
        .sum();
    // The stream-error text table sits at the very start of the data segment
    // (four absolute pointer+length pairs followed by the texts).
    if streams.read || streams.write || streams.flush || streams.pump {
        let mut prefix = Vec::new();
        let mut cursor = u32::try_from(canonical::STREAM_ERROR_TEXTS.len() * 8).unwrap_or(0);
        for text in canonical::STREAM_ERROR_TEXTS {
            prefix.extend_from_slice(&cursor.to_le_bytes());
            prefix.extend_from_slice(&u32::try_from(text.len()).unwrap_or(0).to_le_bytes());
            cursor += u32::try_from(text.len()).unwrap_or(0);
        }
        for text in canonical::STREAM_ERROR_TEXTS {
            prefix.extend_from_slice(text.as_bytes());
        }
        let shift = u32::try_from(prefix.len()).unwrap_or(0);
        data_offsets = data_offsets
            .into_iter()
            .map(|(literal, offset)| (literal, offset + shift))
            .collect();
        prefix.extend_from_slice(&data_bytes);
        data_bytes = prefix;
    }
    // The http2 error-name table sits after every literal (the stream table
    // owns address 0): eight absolute pointer+length pairs in WIT
    // declaration order, then the case-name bytes.
    let mut http2_error_table = None;
    if http2 {
        let base = (u32::try_from(data_bytes.len()).unwrap_or(0) + 3) & !3;
        while u64::try_from(data_bytes.len()).unwrap_or(u64::MAX) < u64::from(base) {
            data_bytes.push(0);
        }
        let mut cursor = base
            + u32::try_from(canonical::HTTP2_ERROR_TEXTS.len() * 8)
                .map_err(|_| unsupported("run", "http2 error table exceeds arena"))?;
        for text in canonical::HTTP2_ERROR_TEXTS {
            data_bytes.extend_from_slice(&cursor.to_le_bytes());
            data_bytes.extend_from_slice(&u32::try_from(text.len()).unwrap_or(0).to_le_bytes());
            cursor += u32::try_from(text.len()).unwrap_or(0);
        }
        for text in canonical::HTTP2_ERROR_TEXTS {
            data_bytes.extend_from_slice(text.as_bytes());
        }
        http2_error_table = Some(base);
    }
    let arena_base = (u32::try_from(data_bytes.len()).unwrap_or(0) + 7) & !7;
    let import_count =
        u32::from(fs.any() || streams.any() || http || http2 || !user_imports.is_empty())
            + (fs.import_count() - u32::from(fs.any()))
            + streams.import_count()
            + u32::from(http)
            + u32::from(http2)
            + user_function_count;
    let function_indices = module
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            u32::try_from(index)
                .map(|index| (function.id, index + import_count))
                .map_err(|_| CodegenError::ModuleTooLarge {
                    functions: module.functions.len(),
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let helper_count = import_count
        + u32::try_from(module.functions.len()).map_err(|_| CodegenError::ModuleTooLarge {
            functions: module.functions.len(),
        })?;
    let mut fs_indices = BTreeMap::new();
    let mut stream_indices = BTreeMap::new();
    let mut http_index = None;
    let mut http2_index = None;
    let mut user_indices: BTreeMap<String, u32> = BTreeMap::new();
    {
        let mut next = 1_u32; // core import 0 is the transport realloc
        for import in canonical::FS_IMPORTS {
            if fs_import_used(fs, import) {
                fs_indices.insert(import.intrinsic, next);
                next += 1;
            }
        }
        for import in canonical::STREAM_IMPORTS {
            if canonical::stream_used(streams, import) {
                stream_indices.insert(import.intrinsic, next);
                next += 1;
            }
        }
        if http {
            http_index = Some(next);
            next += 1;
        }
        if http2 {
            http2_index = Some(next);
            next += 1;
        }
        for import in &user_imports {
            for function in &import.functions {
                user_indices.insert(format!("{}.{}", import.name, function.name), next);
                next += 1;
            }
        }
    }
    let mut used_helpers = std::collections::BTreeSet::new();
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Operation::Intrinsic { name, .. } = &instruction.operation {
                    for helper in helper_dependencies(name) {
                        used_helpers.insert(helper);
                    }
                }
            }
        }
    }
    let helpers = used_helpers
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            u32::try_from(index)
                .map(|index| (*name, helper_count + 3 + index))
                .map_err(|_| CodegenError::ModuleTooLarge {
                    functions: module.functions.len(),
                })
        })
        .collect::<Result<BTreeMap<&'static str, u32>, _>>()?;
    let emit = ScriptEmit {
        abi: ScriptAbi::load(),
        data_offsets,
        data_bytes,
        arena_base,
        alloc_index: helper_count,
        realloc_index: helper_count + 1,
        post_return_index: helper_count + 2,
        helpers,
        fs,
        fs_indices,
        streams,
        stream_indices,
        http,
        http_index,
        http2,
        http2_index,
        http2_error_table,
        user_imports,
        user_indices,
    };
    let variant_tags = VariantTags::new(module, Some(&emit.abi))?;
    let core = build_script_core(module, run, &emit, &function_indices, &variant_tags)?;
    let component = canonical::wrap_script_component(
        &core.bytes,
        fs,
        streams,
        http,
        http2,
        &emit.user_imports,
        arena_base,
    );
    Ok((core, component))
}

/// Collects every static aggregate literal into one deterministic data segment.
fn script_data(module: &IrModule) -> Result<ScriptData, CodegenError> {
    let mut data_offsets = BTreeMap::new();
    let mut data_bytes = Vec::new();
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let literal = match &instruction.operation {
                    Operation::ConstString(value) => value.as_bytes().to_vec(),
                    Operation::ConstBytes(value) => value.clone(),
                    _ => continue,
                };
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    data_offsets.entry(literal.clone())
                {
                    let offset = u32::try_from(data_bytes.len()).map_err(|_| {
                        unsupported(&function.name, "script data segment exceeds arena")
                    })?;
                    entry.insert(offset);
                    data_bytes.extend_from_slice(&literal);
                }
            }
        }
    }
    if data_bytes.len() as u64 >= u64::from(canonical::ARENA_LIMIT) {
        return Err(unsupported("run", "script data segment exceeds arena"));
    }
    Ok((data_offsets, data_bytes))
}

/// Emits the Script core module: verified functions, the bounded arena
/// (`alloc`/`cabi_realloc`/`cabi_post_run`), one 64 MiB memory, the static
/// data segment and the Canonical ABI exports.
#[allow(clippy::too_many_lines)]
fn build_script_core(
    module: &IrModule,
    run: &IrFunction,
    emit: &ScriptEmit,
    function_indices: &BTreeMap<FunctionId, u32>,
    variant_tags: &VariantTags,
) -> Result<CoreCompilation, CodegenError> {
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut code = CodeSection::new();
    let mut debug_functions = Vec::with_capacity(module.functions.len());
    let import_count = u32::from(
        emit.fs.any()
            || emit.streams.any()
            || emit.http
            || emit.http2
            || !emit.user_imports.is_empty(),
    ) + (emit.fs.import_count() - u32::from(emit.fs.any()))
        + emit.streams.import_count()
        + u32::from(emit.http)
        + u32::from(emit.http2)
        + emit
            .user_imports
            .iter()
            .map(|import| u32::try_from(import.functions.len()).unwrap_or(u32::MAX))
            .sum::<u32>();
    if emit.fs.any()
        || emit.streams.any()
        || emit.http
        || emit.http2
        || !emit.user_imports.is_empty()
    {
        // Import types occupy type indices 0..import_count so the
        // type-index-equals-function-index invariant keeps holding.
        types.ty().function(
            [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            [ValType::I32],
        );
        for import in canonical::FS_IMPORTS {
            if !fs_import_used(emit.fs, import) {
                continue;
            }
            types.ty().function(
                import
                    .core_params
                    .iter()
                    .map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    })
                    .collect::<Vec<_>>(),
                [],
            );
        }
        for import in canonical::STREAM_IMPORTS {
            if !canonical::stream_used(emit.streams, import) {
                continue;
            }
            let flat = |flat: &Flat| match flat {
                Flat::I32 => ValType::I32,
                Flat::I64 => ValType::I64,
            };
            types.ty().function(
                import.core_params.iter().map(flat).collect::<Vec<_>>(),
                import.core_results.iter().map(flat).collect::<Vec<_>>(),
            );
        }
        if emit.http {
            types.ty().function(
                canonical::HTTP_IMPORT
                    .core_params
                    .iter()
                    .map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    })
                    .collect::<Vec<_>>(),
                [],
            );
        }
        if emit.http2 {
            types.ty().function(
                canonical::HTTP2_IMPORT
                    .core_params
                    .iter()
                    .map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    })
                    .collect::<Vec<_>>(),
                [],
            );
        }
        // RFC-0039 §2.4 (STEP-0147): one core type per user import —
        // flattened params plus the caller-allocated return-area pointer;
        // results are always returned through the area (MAX_FLAT_RESULTS
        // follows the script profile convention).
        for import in &emit.user_imports {
            for function in &import.functions {
                let mut core_params = Vec::new();
                for parameter in &function.parameters {
                    let Some(flat) = canonical::user_param_flatten(parameter) else {
                        return Err(unsupported(
                            &function.name,
                            "user import parameter value set",
                        ));
                    };
                    core_params.extend(flat.iter().map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    }));
                }
                core_params.push(ValType::I32);
                types.ty().function(core_params, []);
            }
        }
    }
    for (index, function) in module.functions.iter().enumerate() {
        if !function.effects.is_empty() {
            return Err(unsupported(
                &function.name,
                "effectful function without a Component host adapter",
            ));
        }
        // STEP-0148 (RFC-0039 A6 repair): user functions returning a
        // checked fixed-width `Result` are legal again. The core callee
        // declares the full flat width [tag, ok, error]; `emit_return`
        // widens the packed two-slot value form on the way out and the
        // call site re-packs it on the way in, so the seam can no longer
        // produce invalid WebAssembly.
        let function_index = import_count
            + u32::try_from(index).map_err(|_| CodegenError::ModuleTooLarge {
                functions: module.functions.len(),
            })?;
        let (params, results) = if function.name == "run" {
            (
                emit.abi
                    .input
                    .flat
                    .iter()
                    .map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    })
                    .collect(),
                vec![ValType::I32],
            )
        } else {
            let params = function
                .parameters
                .iter()
                .map(|parameter| {
                    if let Some(flat) = emit.abi.flat_ir_types(&parameter.ty) {
                        Ok(flat
                            .iter()
                            .map(|flat| match flat {
                                Flat::I32 => ValType::I32,
                                Flat::I64 => ValType::I64,
                            })
                            .collect::<Vec<_>>())
                    } else {
                        lower_parameter_type(&function.name, &parameter.ty).map(|ty| vec![ty])
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
                .concat();
            let results = match emit.abi.flat_ir_types(&function.return_type) {
                Some(flat) => flat
                    .iter()
                    .map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    })
                    .collect(),
                None => lower_result_type(&function.name, &function.return_type, CoreAbi::Direct)?,
            };
            (params, results)
        };
        types.ty().function(params, results);
        functions.function(function_index);
        let compiled = compile_function(
            function,
            CoreAbi::Direct,
            function_indices,
            variant_tags,
            Some(emit),
        )?;
        code.function(&compiled.body);
        debug_functions.push(CoreDebugFunction {
            id: format!("function-{:010}", function.id.0),
            core_index: function_index,
            mappings: compiled.mappings,
        });
    }
    types.ty().function([ValType::I32], [ValType::I32]);
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function([ValType::I32], []);
    functions.function(emit.alloc_index);
    functions.function(emit.realloc_index);
    functions.function(emit.post_return_index);
    if emit.fs.any()
        || emit.streams.any()
        || emit.http
        || emit.http2
        || !emit.user_imports.is_empty()
    {
        // The transport module owns the memory and the bump allocator; the
        // local alloc/realloc forward to the imported realloc (core import 0)
        // so one allocator serves guest and host-lowered provider calls
        // alike (fs, streams, http and RFC-0039 §2.4 package imports).
        let alloc = emit_alloc_forwarded();
        let realloc = emit_realloc_forwarded();
        let post_return = emit_post_return_noop();
        code.function(&alloc);
        code.function(&realloc);
        code.function(&post_return);
        debug_functions.push(generated_debug_function(
            "generated.alloc",
            emit.alloc_index,
            &alloc,
        ));
        debug_functions.push(generated_debug_function(
            "generated.realloc",
            emit.realloc_index,
            &realloc,
        ));
        debug_functions.push(generated_debug_function(
            "generated.post-return",
            emit.post_return_index,
            &post_return,
        ));
    } else {
        let alloc = emit_alloc();
        let realloc = emit_realloc();
        let post_return = emit_post_return(emit.arena_base);
        code.function(&alloc);
        code.function(&realloc);
        code.function(&post_return);
        debug_functions.push(generated_debug_function(
            "generated.alloc",
            emit.alloc_index,
            &alloc,
        ));
        debug_functions.push(generated_debug_function(
            "generated.realloc",
            emit.realloc_index,
            &realloc,
        ));
        debug_functions.push(generated_debug_function(
            "generated.post-return",
            emit.post_return_index,
            &post_return,
        ));
    }
    for name in emit.helpers.keys() {
        let (params, results) = stdlib::helper_signature(name);
        types.ty().function(params, results);
        functions.function(emit.helpers[name]);
        let helper = stdlib::emit_helper(name, emit.alloc_index, &emit.helpers);
        code.function(&helper);
        debug_functions.push(generated_debug_function(
            &format!("generated.helper.{name}"),
            emit.helpers[name],
            &helper,
        ));
    }
    let bytes = finish_script_core(run, emit, function_indices, &types, &functions, &code)?;
    Ok(CoreCompilation {
        bytes,
        functions: debug_functions,
    })
}

fn generated_debug_function(id: &str, core_index: u32, body: &Function) -> CoreDebugFunction {
    CoreDebugFunction {
        id: id.into(),
        core_index,
        mappings: vec![LocalDebugMapping {
            start: 0,
            end: body.byte_len(),
            source: None,
            generated: true,
        }],
    }
}

/// Assembles the Script core sections after code emission.
fn finish_script_core(
    run: &IrFunction,
    emit: &ScriptEmit,
    function_indices: &BTreeMap<FunctionId, u32>,
    types: &TypeSection,
    functions: &FunctionSection,
    code: &CodeSection,
) -> Result<Vec<u8>, CodegenError> {
    let transport = emit.fs.any()
        || emit.streams.any()
        || emit.http
        || emit.http2
        || !emit.user_imports.is_empty();
    let imports = transport.then(|| script_transport_imports(emit));
    let mut memories = MemorySection::new();
    if !transport {
        memories.memory(MemoryType {
            minimum: u64::from(canonical::MEMORY_PAGES),
            maximum: Some(u64::from(canonical::MEMORY_PAGES)),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
    }
    let mut globals = GlobalSection::new();
    if !transport {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(i32::try_from(emit.arena_base).unwrap_or(i32::MAX)),
        );
    }
    let run_index = function_indices
        .get(&run.id)
        .copied()
        .ok_or_else(|| unsupported("run", "call target after verification"))?;
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("run", ExportKind::Func, run_index);
    exports.export("cabi_realloc", ExportKind::Func, emit.realloc_index);
    exports.export("cabi_post_run", ExportKind::Func, emit.post_return_index);

    let mut output = Module::new();
    output.section(types);
    if transport {
        output.section(imports.as_ref().expect("transport imports exist"));
    }
    output.section(functions);
    if !transport {
        output.section(&memories);
        output.section(&globals);
    }
    output.section(&exports);
    output.section(code);
    if !emit.data_bytes.is_empty() {
        let mut data = DataSection::new();
        data.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: &ConstExpr::i32_const(0),
            },
            data: emit.data_bytes.clone(),
        });
        output.section(&data);
    }
    Ok(output.finish())
}

fn script_transport_imports(emit: &ScriptEmit) -> ImportSection {
    let mut imports = ImportSection::new();
    // The transport memory is imported as memory 0 (the only memory), so one
    // allocator serves guest code and every host-lowered provider call.
    imports.import(
        canonical::FS_TRANSPORT_MODULE,
        "memory",
        EntityType::Memory(MemoryType {
            minimum: u64::from(canonical::MEMORY_PAGES),
            maximum: Some(u64::from(canonical::MEMORY_PAGES)),
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    imports.import(
        canonical::FS_TRANSPORT_MODULE,
        "realloc",
        EntityType::Function(0),
    );
    let mut type_index = 1_u32;
    for import in canonical::FS_IMPORTS {
        if fs_import_used(emit.fs, import) {
            imports.import(
                import.interface,
                import.function,
                EntityType::Function(type_index),
            );
            type_index += 1;
        }
    }
    for import in canonical::STREAM_IMPORTS {
        if canonical::stream_used(emit.streams, import) {
            imports.import(
                canonical::STREAMS_INTERFACE,
                import.function,
                EntityType::Function(type_index),
            );
            type_index += 1;
        }
    }
    if emit.http {
        imports.import(
            canonical::HTTP_INTERFACE,
            canonical::HTTP_IMPORT.function,
            EntityType::Function(type_index),
        );
        type_index += 1;
    }
    if emit.http2 {
        imports.import(
            canonical::HTTP2_INTERFACE,
            canonical::HTTP2_IMPORT.function,
            EntityType::Function(type_index),
        );
        type_index += 1;
    }
    for import in &emit.user_imports {
        for function in &import.functions {
            imports.import(
                &import.name,
                &function.name,
                EntityType::Function(type_index),
            );
            type_index += 1;
        }
    }
    imports
}

fn fs_import_used(usage: canonical::FsUse, import: &canonical::FsImport) -> bool {
    match import.function {
        "read" => usage.read,
        "exists" => usage.exists,
        "write" => usage.write,
        _ => false,
    }
}

/// `alloc(size)` forwarding to the imported transport realloc (core import 0).
fn emit_alloc_forwarded() -> Function {
    let mut body = Function::new(vec![]);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::I32Const(8));
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::End);
    body
}

/// `cabi_realloc` forwarding to the imported transport realloc (import 0).
fn emit_realloc_forwarded() -> Function {
    let mut body = Function::new(vec![]);
    for parameter in 0..4 {
        body.instruction(&Instruction::LocalGet(parameter));
    }
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::End);
    body
}

/// No-op `cabi_post_run`: the transport allocator outlives one call, and the
/// runner drops the whole Store (and memory) after every invocation.
fn emit_post_return_noop() -> Function {
    let mut body = Function::new(vec![]);
    body.instruction(&Instruction::End);
    body
}

/// Arena allocation with checked 32-bit arithmetic: any wrap or ceiling
/// crossing traps instead of silently corrupting memory.
fn emit_alloc() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    body.instruction(&Instruction::GlobalGet(0));
    body.instruction(&Instruction::I32Const(7));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Const(-8));
    body.instruction(&Instruction::I32And);
    body.instruction(&Instruction::LocalSet(1));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalTee(2));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32LtU);
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::I32Const(
        i32::try_from(canonical::ARENA_LIMIT).unwrap_or(i32::MAX),
    ));
    body.instruction(&Instruction::I32GtU);
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::GlobalSet(0));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::End);
    body
}

/// Canonical `cabi_realloc` over the same bounded arena.
fn emit_realloc() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // new_ptr = (heap + align - 1) & -align
    body.instruction(&Instruction::GlobalGet(0));
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Const(-1));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::I32Sub);
    body.instruction(&Instruction::I32And);
    body.instruction(&Instruction::LocalSet(4));
    // new_end = new_ptr + new_size, checked
    body.instruction(&Instruction::LocalGet(4));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalTee(5));
    body.instruction(&Instruction::LocalGet(4));
    body.instruction(&Instruction::I32LtU);
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(5));
    body.instruction(&Instruction::I32Const(
        i32::try_from(canonical::ARENA_LIMIT).unwrap_or(i32::MAX),
    ));
    body.instruction(&Instruction::I32GtU);
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(5));
    body.instruction(&Instruction::GlobalSet(0));
    // if old_ptr != 0: memory.copy(new_ptr, old_ptr, min(old_size, new_size))
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::LocalGet(4));
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32LtU);
    body.instruction(&Instruction::Select);
    body.instruction(&Instruction::MemoryCopy {
        dst_mem: 0,
        src_mem: 0,
    });
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(4));
    body.instruction(&Instruction::End);
    body
}

/// `cabi_post_run`: deterministic per-call arena cleanup. Resetting the bump
/// pointer is idempotent, so a repeated cleanup cannot corrupt state. The
/// Canonical ABI passes the core function results (the return-area pointer).
fn emit_post_return(arena_base: u32) -> Function {
    let mut body = Function::new(Vec::new());
    body.instruction(&Instruction::I32Const(
        i32::try_from(arena_base).unwrap_or(i32::MAX),
    ));
    body.instruction(&Instruction::GlobalSet(0));
    body.instruction(&Instruction::End);
    body
}

#[derive(Clone, Copy)]
struct NumericComponentTypes {
    i64_result: u32,
    u64_result: u32,
}

fn component_type(
    builder: &mut ComponentBuilder,
    numeric_types: &mut Option<NumericComponentTypes>,
    function: &str,
    ty: &Type,
) -> Result<ComponentValType, CodegenError> {
    match ty {
        Type::Bool => Ok(ComponentValType::Primitive(PrimitiveValType::Bool)),
        Type::Int | Type::I64 => Ok(ComponentValType::Primitive(PrimitiveValType::S64)),
        Type::U64 => Ok(ComponentValType::Primitive(PrimitiveValType::U64)),
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE) =>
        {
            let types = *numeric_types.get_or_insert_with(|| define_numeric_types(builder));
            Ok(ComponentValType::Type(match ok.as_ref() {
                Type::I64 => types.i64_result,
                Type::U64 => types.u64_result,
                _ => unreachable!("guarded fixed result"),
            }))
        }
        _ => Err(unsupported(function, "non-scalar Component value")),
    }
}

fn define_numeric_types(builder: &mut ComponentBuilder) -> NumericComponentTypes {
    let (error_type, error) = builder.type_defined(Some("numeric-error"));
    error.enum_type(["overflow", "underflow", "division-by-zero"]);
    let error_type = builder.export("numeric-error", ComponentExportKind::Type, error_type, None);

    let (i64_result, result) = builder.type_defined(Some("checked-i64"));
    result.result(
        Some(ComponentValType::Primitive(PrimitiveValType::S64)),
        Some(ComponentValType::Type(error_type)),
    );
    let i64_result = builder.export("checked-i64", ComponentExportKind::Type, i64_result, None);

    let (u64_result, result) = builder.type_defined(Some("checked-u64"));
    result.result(
        Some(ComponentValType::Primitive(PrimitiveValType::U64)),
        Some(ComponentValType::Type(error_type)),
    );
    let u64_result = builder.export("checked-u64", ComponentExportKind::Type, u64_result, None);
    NumericComponentTypes {
        i64_result,
        u64_result,
    }
}

fn lower_parameter_type(function: &str, ty: &Type) -> Result<ValType, CodegenError> {
    match ty {
        Type::Bool => Ok(ValType::I32),
        Type::I64 | Type::U64 => Ok(ValType::I64),
        Type::Int => Err(unsupported(function, "unbounded Int parameter")),
        Type::Task(_) => Err(async_unsupported(function, "Task")),
        Type::Future(_) => Err(async_unsupported(function, "Future")),
        Type::Stream(_) => Err(async_unsupported(function, "Stream")),
        _ => Err(unsupported(function, "non-scalar parameter")),
    }
}

fn lower_result_type(
    function: &str,
    ty: &Type,
    abi: CoreAbi,
) -> Result<Vec<ValType>, CodegenError> {
    match ty {
        Type::Unit => Ok(Vec::new()),
        Type::Bool => Ok(vec![ValType::I32]),
        Type::Int | Type::I64 | Type::U64 => Ok(vec![ValType::I64]),
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE) =>
        {
            if abi == CoreAbi::ComponentLift {
                Ok(vec![ValType::I32])
            } else {
                // STEP-0148 (A6 repair): the Direct path declares the same
                // full flat width the script profile declares — [tag, ok,
                // error] — and `emit_return`/the call site widen and re-pack
                // the packed value form at the seam.
                Ok(vec![ValType::I32, ValType::I64, ValType::I32])
            }
        }
        Type::Task(_) => Err(async_unsupported(function, "Task")),
        Type::Future(_) => Err(async_unsupported(function, "Future")),
        Type::Stream(_) => Err(async_unsupported(function, "Stream")),
        _ => Err(unsupported(function, "non-scalar result")),
    }
}

fn compile_function(
    function: &IrFunction,
    abi: CoreAbi,
    function_indices: &BTreeMap<FunctionId, u32>,
    variant_tags: &VariantTags,
    script: Option<&ScriptEmit>,
) -> Result<CompiledFunction, CodegenError> {
    let plan = LocalLayout::plan(function, script)?;
    let layout = plan.layout;
    let cell_slots = plan.cell_slots;
    let dispatcher = plan.dispatcher;
    let scratch = plan.scratch;
    let locals = plan.locals;
    let mut body = Function::new(locals);
    body.instruction(&Instruction::I32Const(
        i32::try_from(function.entry.0)
            .map_err(|_| unsupported(&function.name, "block id outside i32"))?,
    ));
    body.instruction(&Instruction::LocalSet(dispatcher));
    body.instruction(&Instruction::Block(BlockType::Empty));
    body.instruction(&Instruction::Loop(BlockType::Empty));
    let context = CompileContext {
        function,
        layout: &layout,
        abi,
        cell_slots: &cell_slots,
        dispatcher,
        function_indices,
        variant_tags,
        script,
        scratch,
    };
    let mut mappings = Vec::new();
    for current in &function.blocks {
        let block_start = body.byte_len();
        let first_block_mapping = mappings.len();
        body.instruction(&Instruction::LocalGet(dispatcher));
        body.instruction(&Instruction::I32Const(
            i32::try_from(current.id.0)
                .map_err(|_| unsupported(&function.name, "block id outside i32"))?,
        ));
        body.instruction(&Instruction::I32Eq);
        body.instruction(&Instruction::If(BlockType::Empty));
        let mut constants = BTreeMap::new();
        compile_instructions(&context, current, &mut body, &mut constants, &mut mappings)?;
        let terminator_start = body.byte_len();
        compile_terminator(&context, &current.terminator, &mut body)?;
        let terminator_end = body.byte_len();
        if terminator_end > terminator_start {
            mappings.push(LocalDebugMapping {
                start: terminator_start,
                end: terminator_end,
                source: Some(current.range),
                generated: false,
            });
        }
        body.instruction(&Instruction::End);
        let block_end = body.byte_len();
        add_generated_gaps(
            &mut mappings,
            first_block_mapping,
            block_start,
            block_end,
            Some(current.range),
        );
    }
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    Ok(CompiledFunction { body, mappings })
}

fn add_generated_gaps(
    mappings: &mut Vec<LocalDebugMapping>,
    first_mapping: usize,
    start: usize,
    end: usize,
    source: Option<sico_ir::SourceRange>,
) {
    let mut cursor = start;
    let mut gaps = Vec::new();
    for mapping in &mappings[first_mapping..] {
        if cursor < mapping.start {
            gaps.push(LocalDebugMapping {
                start: cursor,
                end: mapping.start,
                source,
                generated: true,
            });
        }
        cursor = cursor.max(mapping.end);
    }
    if cursor < end {
        gaps.push(LocalDebugMapping {
            start: cursor,
            end,
            source,
            generated: true,
        });
    }
    mappings.extend(gaps);
}

#[derive(Clone)]
struct ValueLayout {
    slots: Vec<u32>,
    types: Vec<ValType>,
    fields: BTreeMap<String, Vec<usize>>,
    variant: bool,
}

struct LocalLayout {
    values: BTreeMap<ValueId, ValueLayout>,
}

type WasmLocals = Vec<(u32, ValType)>;

/// Static aggregate literal offsets plus the concatenated data-segment bytes.
type ScriptData = (BTreeMap<Vec<u8>, u32>, Vec<u8>);

struct LocalPlan {
    locals: WasmLocals,
    layout: LocalLayout,
    /// Wasm slots per IR local cell (M14 STEP-0130); empty when the
    /// function declares no cells so existing layouts stay byte-stable.
    cell_slots: Vec<Vec<u32>>,
    dispatcher: u32,
    scratch: Option<(u32, u32)>,
}

impl LocalLayout {
    #[allow(clippy::too_many_lines)]
    fn plan(function: &IrFunction, script: Option<&ScriptEmit>) -> Result<LocalPlan, CodegenError> {
        let mut values = BTreeMap::new();
        let mut next = 0_u32;
        let mut locals = Vec::new();
        for parameter in &function.parameters {
            let aggregate = script.and_then(|emit| {
                emit.abi
                    .flat_ir_types(&parameter.ty)
                    .map(|flat| (emit, flat))
            });
            let (types, fields) = match aggregate {
                Some((emit, flat)) => {
                    let fields = match &parameter.ty {
                        Type::Named(name) => emit
                            .abi
                            .record(name)
                            .map(|record| {
                                record
                                    .fields
                                    .iter()
                                    .map(|field| {
                                        (
                                            ir_field_name(&field.name),
                                            (field.slot_start..field.slot_start + field.slot_len)
                                                .collect::<Vec<usize>>(),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        // Synthetic payload fields for `case ok(x)` matching
                        // on Result parameters.
                        Type::Result { ok, error } => {
                            let mut fields: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                            if let (Some(ok_flat), Some(error_flat)) =
                                (emit.abi.flat_ir_types(ok), emit.abi.flat_ir_types(error))
                            {
                                fields.insert("ok".to_owned(), (1..=ok_flat.len()).collect());
                                fields.insert(
                                    "error".to_owned(),
                                    (1 + ok_flat.len()..1 + ok_flat.len() + error_flat.len())
                                        .collect(),
                                );
                            }
                            fields
                        }
                        _ => BTreeMap::new(),
                    };
                    (
                        flat.iter()
                            .map(|flat| match flat {
                                Flat::I32 => ValType::I32,
                                Flat::I64 => ValType::I64,
                            })
                            .collect::<Vec<_>>(),
                        fields,
                    )
                }
                None => (
                    vec![lower_parameter_type(&function.name, &parameter.ty)?],
                    BTreeMap::new(),
                ),
            };
            let mut slots = Vec::new();
            for _ in &types {
                slots.push(next);
                next = next
                    .checked_add(1)
                    .ok_or_else(|| unsupported(&function.name, "too many Wasm locals"))?;
            }
            values.insert(
                parameter.id,
                ValueLayout {
                    slots,
                    types,
                    fields,
                    variant: matches!(&parameter.ty, Type::Result { .. } | Type::Option(_)),
                },
            );
        }
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            let mut value_layout = inferred_local_layout(
                function,
                instruction,
                &values,
                script.map(|emit| &emit.abi),
            )?;
            for ty in &value_layout.types {
                value_layout.slots.push(next);
                next = next
                    .checked_add(1)
                    .ok_or_else(|| unsupported(&function.name, "too many Wasm locals"))?;
                locals.push((1, *ty));
            }
            values.insert(instruction.result, value_layout);
        }
        let mut cell_slots = Vec::new();
        for local in &function.locals {
            let types = lower_local_types(&function.name, &local.ty, script.map(|emit| &emit.abi))?;
            let mut slots = Vec::new();
            for ty in &types {
                slots.push(next);
                next = next
                    .checked_add(1)
                    .ok_or_else(|| unsupported(&function.name, "too many Wasm locals"))?;
                locals.push((1, *ty));
            }
            cell_slots.push(slots);
        }
        let dispatcher = next;
        locals.push((1, ValType::I32));
        next = next
            .checked_add(1)
            .ok_or_else(|| unsupported(&function.name, "too many Wasm locals"))?;
        let scratch = if script.is_some() {
            let pair = (next, next + 1);
            locals.push((2, ValType::I32));
            Some(pair)
        } else {
            None
        };
        Ok(LocalPlan {
            locals,
            layout: Self { values },
            cell_slots,
            dispatcher,
            scratch,
        })
    }

    fn layout(&self, function: &str, value: ValueId) -> Result<&ValueLayout, CodegenError> {
        self.values
            .get(&value)
            .ok_or_else(|| unsupported(function, "missing local after verification"))
    }

    fn get(&self, function: &str, value: ValueId) -> Result<&[u32], CodegenError> {
        Ok(&self.layout(function, value)?.slots)
    }

    fn field(&self, function: &str, value: ValueId, field: &str) -> Result<Vec<u32>, CodegenError> {
        let layout = self.layout(function, value)?;
        let Some(indices) = layout.fields.get(field) else {
            return Err(unsupported(function, "unknown internal aggregate field"));
        };
        Ok(indices.iter().map(|index| layout.slots[*index]).collect())
    }

    fn tag(&self, function: &str, value: ValueId) -> Result<u32, CodegenError> {
        let layout = self.layout(function, value)?;
        if !layout.variant {
            return Err(unsupported(
                function,
                "variant pattern over non-variant value",
            ));
        }
        layout
            .slots
            .first()
            .copied()
            .ok_or_else(|| unsupported(function, "missing variant tag"))
    }

    fn scalar(&self, function: &str, value: ValueId) -> Result<u32, CodegenError> {
        let [slot] = self.get(function, value)? else {
            return Err(unsupported(function, "aggregate used as scalar"));
        };
        Ok(*slot)
    }
}

#[allow(clippy::too_many_lines)]
fn inferred_local_layout(
    function: &IrFunction,
    instruction: &sico_ir::Instruction,
    available: &BTreeMap<ValueId, ValueLayout>,
    script: Option<&ScriptAbi>,
) -> Result<ValueLayout, CodegenError> {
    let mut layout = ValueLayout {
        slots: Vec::new(),
        types: Vec::new(),
        fields: BTreeMap::new(),
        variant: false,
    };
    match &instruction.operation {
        Operation::Copy(source) => {
            let Some(source) = available.get(source) else {
                return Err(unsupported(
                    &function.name,
                    "copy layout after verification",
                ));
            };
            layout.types.clone_from(&source.types);
            layout.fields.clone_from(&source.fields);
            layout.variant = source.variant;
        }
        Operation::TaskScopeOpen { .. }
        | Operation::TaskScopeClose { .. }
        | Operation::WriteLocal { .. } => {
            // Region markers are compile-time structure (RFC-0036 §5.4) and
            // cell writes are Unit: no result slots.
        }
        Operation::ReadLocal { local } => {
            let Some(decl) = function.locals.get(*local as usize) else {
                return Err(unsupported(&function.name, "missing local cell"));
            };
            layout.types = lower_local_types(&function.name, &decl.ty, script)?;
            apply_result_fields(&mut layout, script, &decl.ty);
            layout.variant = matches!(&decl.ty, Type::Result { .. } | Type::Option(_));
        }
        Operation::Construct { fields, .. } if matches!(&instruction.ty, Type::List(_)) => {
            // A collect list is a (table, count) pair in the bounded arena,
            // not a concatenated record (RFC-0036 §4).
            layout.types = lower_local_types(&function.name, &instruction.ty, script)?;
        }
        Operation::Construct { fields, .. } => {
            for field in fields {
                let Some(source) = available.get(&field.value) else {
                    return Err(unsupported(
                        &function.name,
                        "construct layout after verification",
                    ));
                };
                let start = layout.types.len();
                layout.types.extend_from_slice(&source.types);
                layout
                    .fields
                    .insert(field.name.clone(), (start..layout.types.len()).collect());
            }
        }
        Operation::Project { base, field } => {
            let Some(base) = available.get(base) else {
                return Err(unsupported(
                    &function.name,
                    "project layout after verification",
                ));
            };

            let Some(indices) = base.fields.get(field) else {
                return Err(unsupported(
                    &function.name,
                    "unknown record field in local layout",
                ));
            };
            layout
                .types
                .extend(indices.iter().map(|index| base.types[*index]));
            let expected = lower_local_types(&function.name, &instruction.ty, script)?;
            if layout.types != expected {
                return Err(unsupported(
                    &function.name,
                    "projected field does not match its declared Wasm layout",
                ));
            }
            if let Some(record) = script.and_then(|abi| record_fields(abi, &instruction.ty)) {
                for field in &record.fields {
                    layout.fields.insert(
                        ir_field_name(&field.name),
                        (field.slot_start..field.slot_start + field.slot_len).collect(),
                    );
                }
            }
        }
        Operation::Variant { name, payload } => {
            layout.types.push(ValType::I32);
            layout.variant = true;
            let mut payload_types = Vec::new();
            for value in payload {
                let Some(payload) = available.get(value) else {
                    return Err(unsupported(
                        &function.name,
                        "variant layout after verification",
                    ));
                };
                payload_types.extend_from_slice(&payload.types);
            }
            // Result variants in the Script profile occupy the full flat
            // width (tag + ok payload + error payload) so values crossing
            // helper-function boundaries match the declared signature; the
            // inactive payload region is zero-filled at construction.
            let shift = if let (Some(abi), Type::Result { ok, error }) = (script, &instruction.ty) {
                match (abi.flat_ir_types(ok), abi.flat_ir_types(error)) {
                    (Some(ok_flat), Some(_error_flat)) if name == "error" => {
                        layout
                            .types
                            .extend(ok_flat.iter().copied().map(flat_local_type));
                        layout.types.extend_from_slice(&payload_types);
                        1 + ok_flat.len()
                    }
                    (Some(_), Some(error_flat)) => {
                        layout.types.extend_from_slice(&payload_types);
                        layout
                            .types
                            .extend(error_flat.iter().copied().map(flat_local_type));
                        1
                    }
                    _ => {
                        layout.types.extend_from_slice(&payload_types);
                        1
                    }
                }
            } else {
                layout.types.extend_from_slice(&payload_types);
                1
            };
            if let [single] = payload.as_slice()
                && let Some(payload) = available.get(single)
            {
                for (name, indices) in &payload.fields {
                    layout.fields.insert(
                        name.clone(),
                        indices.iter().map(|index| index + shift).collect(),
                    );
                }
            }
            // Synthetic variant-payload field for `case ok(x)` projection.
            layout
                .fields
                .insert(name.clone(), (shift..shift + payload_types.len()).collect());
        }
        _ => {
            layout.variant = matches!(&instruction.ty, Type::Result { .. } | Type::Option(_));
            layout.types = lower_local_types(&function.name, &instruction.ty, script)?;
            // Result-typed values carry both payloads' field maps with
            // shifted slot indices: ok fields after the tag, error fields
            // after the ok payload. This lets the boundary lower results
            // produced by calls, not only by direct Variant construction.
            // Synthetic `ok`/`error` fields expose the payloads to
            // `case ok(x)` projection.
            if let (Some(abi), Type::Result { ok, error }) = (script, &instruction.ty)
                && let (Some(ok_flat), Some(error_flat)) =
                    (abi.flat_ir_types(ok), abi.flat_ir_types(error))
            {
                layout
                    .fields
                    .insert("ok".to_owned(), (1..=ok_flat.len()).collect());
                // The packed fixed-width `Result[I64|U64, NumericError]`
                // local is two slots (tag + one shared payload), so the
                // error discriminant has no third slot to map: omit the
                // field and let a `Project error` fail closed with the
                // unknown-field refusal instead of panicking on an
                // out-of-range index (the A6 seam, STEP-0175).
                let error_end = 1 + ok_flat.len() + error_flat.len();
                if error_end <= layout.types.len() {
                    layout
                        .fields
                        .insert("error".to_owned(), (1 + ok_flat.len()..error_end).collect());
                }
                if let Some(ok_record) = record_fields(abi, ok) {
                    for field in &ok_record.fields {
                        layout.fields.insert(
                            ir_field_name(&field.name),
                            (1 + field.slot_start..1 + field.slot_start + field.slot_len).collect(),
                        );
                    }
                }
                if let Some(error_record) = record_fields(abi, error) {
                    let offset = 1 + ok_flat.len();
                    for field in &error_record.fields {
                        layout.fields.insert(
                            ir_field_name(&field.name),
                            (offset + field.slot_start..offset + field.slot_start + field.slot_len)
                                .collect(),
                        );
                    }
                }
            }
        }
    }
    Ok(layout)
}

fn record_fields<'a>(abi: &'a ScriptAbi, ty: &Type) -> Option<&'a canonical::RecordLayout> {
    match ty {
        Type::Named(name) => abi.record(name),
        _ => None,
    }
}

/// Adds the synthetic `ok`/`error` field maps (plus record payload fields)
/// to a Result-typed value layout (M14 STEP-0130 local-cell reads).
fn apply_result_fields(layout: &mut ValueLayout, script: Option<&ScriptAbi>, ty: &Type) {
    if let (Some(abi), Type::Result { ok, error }) = (script, ty)
        && let Some(ok_flat) = abi.flat_ir_types(ok)
    {
        layout
            .fields
            .insert("ok".to_owned(), (1..=ok_flat.len()).collect());
        // The error payload may be unflattenable (for example
        // `NumericError`, whose layout is the fixed [I32, I64] tag form);
        // its fields are then simply absent, and `case error(_)` still
        // matches by tag.
        if let Some(error_flat) = abi.flat_ir_types(error) {
            layout.fields.insert(
                "error".to_owned(),
                (1 + ok_flat.len()..1 + ok_flat.len() + error_flat.len()).collect(),
            );
            if let Some(error_record) = record_fields(abi, error) {
                let offset = 1 + ok_flat.len();
                for field in &error_record.fields {
                    layout.fields.insert(
                        ir_field_name(&field.name),
                        (offset + field.slot_start..offset + field.slot_start + field.slot_len)
                            .collect(),
                    );
                }
            }
        }
        if let Some(ok_record) = record_fields(abi, ok) {
            for field in &ok_record.fields {
                layout.fields.insert(
                    ir_field_name(&field.name),
                    (1 + field.slot_start..1 + field.slot_start + field.slot_len).collect(),
                );
            }
        }
    }
}

fn flat_local_type(flat: Flat) -> ValType {
    match flat {
        Flat::I32 => ValType::I32,
        Flat::I64 => ValType::I64,
    }
}

fn set_dispatcher(
    function: &IrFunction,
    body: &mut Function,
    dispatcher: u32,
    target: BlockId,
) -> Result<(), CodegenError> {
    body.instruction(&Instruction::I32Const(
        i32::try_from(target.0).map_err(|_| unsupported(&function.name, "block id outside i32"))?,
    ));
    body.instruction(&Instruction::LocalSet(dispatcher));
    Ok(())
}

fn compile_terminator(
    context: &CompileContext<'_>,
    terminator: &Terminator,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let dispatcher = context.dispatcher;
    let variant_tags = context.variant_tags;
    match terminator {
        Terminator::Return(value) => emit_return(context, body, *value),
        Terminator::Jump(target) => {
            set_dispatcher(function, body, dispatcher, *target)?;
            body.instruction(&Instruction::Br(1));
            Ok(())
        }
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            body.instruction(&Instruction::LocalGet(
                layout.scalar(&function.name, *condition)?,
            ));
            body.instruction(&Instruction::If(BlockType::Empty));
            set_dispatcher(function, body, dispatcher, *then_block)?;
            body.instruction(&Instruction::Else);
            set_dispatcher(function, body, dispatcher, *else_block)?;
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::Br(1));
            Ok(())
        }
        Terminator::Match { values, arms } => {
            body.instruction(&Instruction::I32Const(-1));
            body.instruction(&Instruction::LocalSet(dispatcher));
            for arm in arms {
                body.instruction(&Instruction::LocalGet(dispatcher));
                body.instruction(&Instruction::I32Const(-1));
                body.instruction(&Instruction::I32Eq);
                body.instruction(&Instruction::If(BlockType::Empty));
                emit_match_condition(function, layout, body, values, &arm.patterns, variant_tags)?;
                body.instruction(&Instruction::If(BlockType::Empty));
                set_dispatcher(function, body, dispatcher, arm.target)?;
                body.instruction(&Instruction::End);
                body.instruction(&Instruction::End);
            }
            body.instruction(&Instruction::LocalGet(dispatcher));
            body.instruction(&Instruction::I32Const(-1));
            body.instruction(&Instruction::I32Eq);
            body.instruction(&Instruction::If(BlockType::Empty));
            body.instruction(&Instruction::Unreachable);
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::Br(1));
            Ok(())
        }
        Terminator::Unreachable => {
            body.instruction(&Instruction::Unreachable);
            Ok(())
        }
    }
}

fn emit_match_condition(
    function: &IrFunction,
    layout: &LocalLayout,
    body: &mut Function,
    values: &[ValueId],
    patterns: &[Pattern],
    variant_tags: &VariantTags,
) -> Result<(), CodegenError> {
    body.instruction(&Instruction::I32Const(1));
    for (value, pattern) in values.iter().zip(patterns) {
        match pattern {
            Pattern::Wildcard => {
                body.instruction(&Instruction::I32Const(1));
            }
            Pattern::Binding(_) => {
                return Err(unsupported(&function.name, "match binding transport"));
            }
            Pattern::Bool(expected) => {
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *value)?,
                ));
                body.instruction(&Instruction::I32Const(i32::from(*expected)));
                body.instruction(&Instruction::I32Eq);
            }
            Pattern::Variant { name, payload } => {
                // Binding payloads need no inspection: the arm block reads
                // the payload through a Project instruction; only nested
                // literal/variant patterns are refused here.
                if payload
                    .iter()
                    .any(|pattern| matches!(pattern, Pattern::Bool(_) | Pattern::Variant { .. }))
                {
                    return Err(unsupported(&function.name, "match payload inspection"));
                }
                body.instruction(&Instruction::LocalGet(layout.tag(&function.name, *value)?));
                body.instruction(&Instruction::I32Const(
                    variant_tags.get(&function.name, name)?,
                ));
                body.instruction(&Instruction::I32Eq);
            }
        }
        body.instruction(&Instruction::I32And);
    }
    Ok(())
}

struct VariantTags(BTreeMap<String, i32>);

impl VariantTags {
    fn new(module: &IrModule, script: Option<&ScriptAbi>) -> Result<Self, CodegenError> {
        let mut names = std::collections::BTreeSet::new();
        for function in &module.functions {
            for current in &function.blocks {
                for instruction in &current.instructions {
                    if let Operation::Variant { name, .. } = &instruction.operation {
                        names.insert(name.clone());
                    }
                }
                if let Terminator::Match { arms, .. } = &current.terminator {
                    for arm in arms {
                        for pattern in &arm.patterns {
                            collect_variant_patterns(pattern, &mut names);
                        }
                    }
                }
            }
        }
        let mut tags = BTreeMap::new();
        let mut next = 0_i32;
        if let Some(abi) = script {
            // Canonical ABI discriminants: result ok=0/err=1, option some=0/none=1,
            // script-error-code cases in WIT declaration order, and the
            // STEP-0077 numeric-error enum order.
            for (name, tag) in [
                ("ok".to_owned(), 0),
                ("error".to_owned(), 1),
                ("some".to_owned(), 0),
                ("none".to_owned(), 1),
                ("NumericError.overflow".to_owned(), 0),
                ("NumericError.underflow".to_owned(), 1),
                ("NumericError.division-by-zero".to_owned(), 2),
            ] {
                tags.insert(name, tag);
            }
            for (case, tag) in &abi.error_tags {
                tags.insert(format!("ScriptErrorCode.{}", kebab_to_camel(case)), *tag);
            }
            next = 2;
        }
        for name in names {
            if tags.contains_key(&name) {
                continue;
            }
            if next == i32::MAX {
                return Err(CodegenError::ModuleTooLarge {
                    functions: module.functions.len(),
                });
            }
            tags.insert(name, next);
            next += 1;
        }
        Ok(Self(tags))
    }

    fn get(&self, function: &str, name: &str) -> Result<i32, CodegenError> {
        self.0
            .get(name)
            .copied()
            .ok_or_else(|| unsupported(function, "unknown variant tag"))
    }
}

fn collect_variant_patterns(pattern: &Pattern, names: &mut std::collections::BTreeSet<String>) {
    if let Pattern::Variant { name, payload } = pattern {
        names.insert(name.clone());
        for pattern in payload {
            collect_variant_patterns(pattern, names);
        }
    }
}

struct CompileContext<'a> {
    function: &'a IrFunction,
    layout: &'a LocalLayout,
    abi: CoreAbi,
    cell_slots: &'a [Vec<u32>],
    dispatcher: u32,
    function_indices: &'a BTreeMap<FunctionId, u32>,
    variant_tags: &'a VariantTags,
    script: Option<&'a ScriptEmit>,
    scratch: Option<(u32, u32)>,
}

#[allow(clippy::too_many_lines)]
fn compile_instructions(
    context: &CompileContext<'_>,
    block: &Block,
    body: &mut Function,
    constants: &mut BTreeMap<ValueId, i64>,
    mappings: &mut Vec<LocalDebugMapping>,
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let abi = context.abi;
    let dispatcher = context.dispatcher;
    let function_indices = context.function_indices;
    let variant_tags = context.variant_tags;
    for instruction in &block.instructions {
        let mapping_start = body.byte_len();
        match &instruction.operation {
            Operation::ConstInt(value) => {
                let parsed =
                    value
                        .parse::<i64>()
                        .map_err(|_| CodegenError::IntegerOutsideProvenI64 {
                            function: function.name.clone(),
                            bytes: value.len(),
                        })?;
                constants.insert(instruction.result, parsed);
                body.instruction(&Instruction::I64Const(parsed));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstI64(value) => {
                body.instruction(&Instruction::I64Const(*value));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstU64(value) => {
                body.instruction(&Instruction::I64Const(i64::from_le_bytes(
                    value.to_le_bytes(),
                )));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstBool(value) => {
                body.instruction(&Instruction::I32Const(i32::from(*value)));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstString(value) if context.script.is_some() => {
                emit_static_bytes(
                    context,
                    body,
                    value.as_bytes(),
                    instruction.result,
                    "text literal",
                )?;
            }
            Operation::ConstBytes(value) if context.script.is_some() => {
                emit_static_bytes(context, body, value, instruction.result, "bytes literal")?;
            }
            Operation::Intrinsic { name, arguments } if context.script.is_some() => {
                emit_intrinsic(context, body, instruction, name, arguments)?;
            }
            Operation::Copy(source)
                if matches!(instruction.ty, Type::Bool | Type::I64 | Type::U64) =>
            {
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *source)?,
                ));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::Copy(source) if instruction.ty == Type::Int => {
                let Some(value) = constants.get(source).copied() else {
                    return Err(unsupported(&function.name, "non-constant Int copy"));
                };
                constants.insert(instruction.result, value);
                body.instruction(&Instruction::I64Const(value));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            // sequential-v1: await is the identity over the eagerly resolved
            // value; collect is the identity over the creation-order list.
            Operation::Copy(source)
            | Operation::Await(source)
            | Operation::TaskCollect { tasks: source, .. } => {
                copy_slots(
                    &function.name,
                    body,
                    layout.get(&function.name, *source)?,
                    layout.get(&function.name, instruction.result)?,
                )?;
            }
            Operation::AddInt { left, right } => {
                let (Some(left_value), Some(right_value)) =
                    (constants.get(left), constants.get(right))
                else {
                    return Err(unsupported(
                        &function.name,
                        "non-constant arbitrary Int add",
                    ));
                };
                let value = left_value.checked_add(*right_value).ok_or_else(|| {
                    CodegenError::IntegerOutsideProvenI64 {
                        function: function.name.clone(),
                        bytes: 8,
                    }
                })?;
                constants.insert(instruction.result, value);
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *left)?,
                ));
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *right)?,
                ));
                body.instruction(&Instruction::I64Add);
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::CheckedAdd { left, right } => {
                emit_checked_fixed(function, layout, instruction, *left, *right, true, body)?;
            }
            Operation::CheckedSub { left, right } => {
                emit_checked_fixed(function, layout, instruction, *left, *right, false, body)?;
            }
            Operation::CheckedMul { left, right } => {
                emit_checked_mul(function, layout, instruction, *left, *right, body)?;
            }
            Operation::CheckedDiv { left, right } => {
                emit_checked_div(function, layout, instruction, *left, *right, body)?;
            }
            Operation::EqualFixed { left, right } => {
                emit_fixed_comparison(function, layout, instruction, *left, *right, true, body)?;
            }
            Operation::LessFixed { left, right } => {
                emit_fixed_comparison(function, layout, instruction, *left, *right, false, body)?;
            }
            // STEP-0132: bit operations are pure single-slot i64 instructions
            // in both fixed-width types. Shr is arithmetic for I64 and
            // logical for U64; wasm masks shift counts to [0, 63].
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
                let left_slot = layout.scalar(&function.name, *left)?;
                let right_slot = layout.scalar(&function.name, *right)?;
                let result = layout.scalar(&function.name, instruction.result)?;
                body.instruction(&Instruction::LocalGet(left_slot));
                body.instruction(&Instruction::LocalGet(right_slot));
                body.instruction(&match &instruction.operation {
                    Operation::BitAnd { .. } => Instruction::I64And,
                    Operation::BitOr { .. } => Instruction::I64Or,
                    Operation::BitXor { .. } => Instruction::I64Xor,
                    Operation::Shl { .. } => Instruction::I64Shl,
                    _ => {
                        if matches!(instruction.ty, Type::U64) {
                            Instruction::I64ShrU
                        } else {
                            Instruction::I64ShrS
                        }
                    }
                });
                body.instruction(&Instruction::LocalSet(result));
            }
            // sequential-v1 (RFC-0036 §5.4): spawn is the eager call itself;
            // the `Task[T]` result shares the callee's resolved-value layout.
            Operation::Call {
                function: target,
                arguments,
            }
            | Operation::Spawn {
                callee: target,
                arguments,
                ..
            } => {
                if instruction.ty == Type::Int {
                    return Err(unsupported(&function.name, "non-constant Int call result"));
                }
                for argument in arguments {
                    let slots = layout.get(&function.name, *argument)?;
                    if slots.len() > 1 && context.script.is_none() {
                        return Err(unsupported(
                            &function.name,
                            "aggregate call argument outside script profile",
                        ));
                    }
                    for slot in slots {
                        body.instruction(&Instruction::LocalGet(*slot));
                    }
                }
                let target = function_indices
                    .get(target)
                    .copied()
                    .ok_or_else(|| unsupported(&function.name, "call target after verification"))?;
                body.instruction(&Instruction::Call(target));
                let result = layout.get(&function.name, instruction.result)?;
                if abi == CoreAbi::Direct && is_checked_fixed_result(&instruction.ty) {
                    // STEP-0148: the callee returned the full flat width
                    // [tag, ok, error] (losing side zeroed); re-pack into
                    // the [tag, payload] value form, where the payload is
                    // the ok value or the zero-extended error tag.
                    let [tag, payload] = result else {
                        return Err(unsupported(&function.name, "checked call result layout"));
                    };
                    let Some((_, copy_destination)) = context.scratch else {
                        return Err(unsupported(&function.name, "checked call scratch layout"));
                    };
                    body.instruction(&Instruction::LocalSet(copy_destination));
                    body.instruction(&Instruction::LocalSet(*payload));
                    body.instruction(&Instruction::LocalSet(*tag));
                    body.instruction(&Instruction::LocalGet(*tag));
                    body.instruction(&Instruction::I32Eqz);
                    body.instruction(&Instruction::If(BlockType::Empty));
                    body.instruction(&Instruction::Else);
                    body.instruction(&Instruction::LocalGet(copy_destination));
                    body.instruction(&Instruction::I64ExtendI32U);
                    body.instruction(&Instruction::LocalSet(*payload));
                    body.instruction(&Instruction::End);
                } else if abi == CoreAbi::ComponentLift && is_checked_fixed_result(&instruction.ty)
                {
                    let [tag, payload] = result else {
                        return Err(unsupported(&function.name, "checked call result layout"));
                    };
                    body.instruction(&Instruction::LocalSet(dispatcher));
                    body.instruction(&Instruction::LocalGet(dispatcher));
                    body.instruction(&Instruction::I32Load8U(MemArg {
                        offset: 0,
                        align: 0,
                        memory_index: 0,
                    }));
                    body.instruction(&Instruction::LocalSet(*tag));
                    body.instruction(&Instruction::LocalGet(dispatcher));
                    body.instruction(&Instruction::I64Load(MemArg {
                        offset: 8,
                        align: 3,
                        memory_index: 0,
                    }));
                    body.instruction(&Instruction::LocalSet(*payload));
                } else {
                    for slot in result.iter().rev() {
                        body.instruction(&Instruction::LocalSet(*slot));
                    }
                }
            }
            Operation::ReadLocal { local } => {
                let Some(slots) = context.cell_slots.get(*local as usize) else {
                    return Err(unsupported(&function.name, "missing local cell"));
                };
                let result = layout.get(&function.name, instruction.result)?;
                if slots.len() != result.len() {
                    return Err(unsupported(&function.name, "local cell layout"));
                }
                for (cell, result) in slots.iter().zip(result) {
                    body.instruction(&Instruction::LocalGet(*cell));
                    body.instruction(&Instruction::LocalSet(*result));
                }
            }
            Operation::WriteLocal { local, value } => {
                let Some(slots) = context.cell_slots.get(*local as usize) else {
                    return Err(unsupported(&function.name, "missing local cell"));
                };
                let source = layout.get(&function.name, *value)?;
                if slots.len() != source.len() {
                    return Err(unsupported(&function.name, "local cell layout"));
                }
                for (cell, source) in slots.iter().zip(source) {
                    body.instruction(&Instruction::LocalGet(*source));
                    body.instruction(&Instruction::LocalSet(*cell));
                }
            }
            Operation::TaskScopeOpen { .. } | Operation::TaskScopeClose { .. } => {
                // Region markers have no runtime cost in sequential-v1
                // (RFC-0036 §5.4); they exist for the verifier and future
                // scheduler profiles.
            }
            Operation::Construct { fields, .. } if matches!(&instruction.ty, Type::List(_)) => {
                emit_collect_list(context, body, instruction, fields)?;
            }
            Operation::Construct { fields, .. } => {
                let mut destination = 0;
                let result = layout.get(&function.name, instruction.result)?;
                for field in fields {
                    let source = layout.get(&function.name, field.value)?;
                    let end = destination + source.len();
                    copy_slots(
                        &function.name,
                        body,
                        source,
                        result.get(destination..end).ok_or_else(|| {
                            unsupported(&function.name, "construct destination layout")
                        })?,
                    )?;
                    destination = end;
                }
            }
            Operation::Project { base, field } => {
                copy_slots(
                    &function.name,
                    body,
                    &layout.field(&function.name, *base, field)?,
                    layout.get(&function.name, instruction.result)?,
                )?;
            }
            Operation::Variant { name, payload } => {
                let value_layout = layout.layout(&function.name, instruction.result)?;
                let Some((tag, destination)) = value_layout.slots.split_first() else {
                    return Err(unsupported(&function.name, "variant destination layout"));
                };
                body.instruction(&Instruction::I32Const(
                    variant_tags.get(&function.name, name)?,
                ));
                body.instruction(&Instruction::LocalSet(*tag));
                let mut payload_len = 0;
                for value in payload {
                    payload_len += layout.get(&function.name, *value)?.len();
                }
                // Full-width Result layouts right-align the error payload and
                // left-align the ok payload; inactive regions are zero-filled
                // so every slot of the declared flat signature is defined.
                let start = if destination.len() > payload_len && name == "error" {
                    destination.len() - payload_len
                } else {
                    0
                };
                let destination_types = &value_layout.types[1..];
                for (slot, ty) in destination[..start].iter().zip(destination_types) {
                    zero_local(body, *slot, *ty);
                }
                let mut offset = start;
                for value in payload {
                    let source = layout.get(&function.name, *value)?;
                    let end = offset + source.len();
                    copy_slots(
                        &function.name,
                        body,
                        source,
                        destination
                            .get(offset..end)
                            .ok_or_else(|| unsupported(&function.name, "variant payload layout"))?,
                    )?;
                    offset = end;
                }
                for (slot, ty) in destination[offset..]
                    .iter()
                    .zip(&destination_types[offset..])
                {
                    zero_local(body, *slot, *ty);
                }
            }
            _ => return Err(unsupported(&function.name, "operation")),
        }
        let mapping_end = body.byte_len();
        if mapping_end > mapping_start {
            mappings.push(LocalDebugMapping {
                start: mapping_start,
                end: mapping_end,
                source: Some(instruction.range),
                generated: false,
            });
        }
    }
    Ok(())
}

/// Builds a collect list in the bounded arena: one 8-byte (pointer, length)
/// cell per element in creation order, result `(table, count)` (RFC-0036 §4).
fn emit_collect_list(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    fields: &[sico_ir::ConstructField],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "collect list outside the Script profile",
        ));
    };
    let [table, count] = layout.get(&function.name, instruction.result)? else {
        return Err(unsupported(&function.name, "collect list layout"));
    };
    body.instruction(&Instruction::I32Const(
        i32::try_from(fields.len())
            .map_err(|_| unsupported(&function.name, "collect list length outside i32"))?,
    ));
    body.instruction(&Instruction::LocalSet(*count));
    if fields.is_empty() {
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::LocalSet(*table));
        return Ok(());
    }
    let bytes = fields
        .len()
        .checked_mul(8)
        .and_then(|bytes| i32::try_from(bytes).ok())
        .ok_or_else(|| unsupported(&function.name, "collect list byte size outside i32"))?;
    body.instruction(&Instruction::I32Const(bytes));
    body.instruction(&Instruction::Call(emit.alloc_index));
    body.instruction(&Instruction::LocalSet(*table));
    for (index, field) in fields.iter().enumerate() {
        let [pointer, length] = layout.get(&function.name, field.value)? else {
            return Err(unsupported(&function.name, "collect list element layout"));
        };
        let offset = u64::try_from(index * 8)
            .map_err(|_| unsupported(&function.name, "collect list offset outside u64"))?;
        body.instruction(&Instruction::LocalGet(*table));
        body.instruction(&Instruction::LocalGet(*pointer));
        body.instruction(&Instruction::I32Store(MemArg {
            offset,
            align: 2,
            memory_index: 0,
        }));
        body.instruction(&Instruction::LocalGet(*table));
        body.instruction(&Instruction::LocalGet(*length));
        body.instruction(&Instruction::I32Store(MemArg {
            offset: offset + 4,
            align: 2,
            memory_index: 0,
        }));
    }
    Ok(())
}

fn zero_local(body: &mut Function, slot: u32, ty: ValType) {
    match ty {
        ValType::I32 => body.instruction(&Instruction::I32Const(0)),
        ValType::I64 => body.instruction(&Instruction::I64Const(0)),
        _ => unreachable!("script aggregates only use i32/i64 locals"),
    };
    body.instruction(&Instruction::LocalSet(slot));
}

fn copy_slots(
    function: &str,
    body: &mut Function,
    source: &[u32],
    destination: &[u32],
) -> Result<(), CodegenError> {
    if source.len() != destination.len() {
        return Err(unsupported(
            function,
            "incompatible internal aggregate layout",
        ));
    }
    for (source, destination) in source.iter().zip(destination) {
        body.instruction(&Instruction::LocalGet(*source));
        body.instruction(&Instruction::LocalSet(*destination));
    }
    Ok(())
}

fn lower_local_types(
    function: &str,
    ty: &Type,
    script: Option<&ScriptAbi>,
) -> Result<Vec<ValType>, CodegenError> {
    match ty {
        Type::Bool => Ok(vec![ValType::I32]),
        Type::Int | Type::I64 | Type::U64 => Ok(vec![ValType::I64]),
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE) =>
        {
            Ok(vec![ValType::I32, ValType::I64])
        }
        // sequential-v1 (RFC-0036 §5.4): inside the Script profile a
        // Task/Future value is representation-identical to its resolved
        // value; outside it the RFC-0013 async boundary refusal stands.
        Type::Task(inner) => lower_async_local(function, inner, script, "Task"),
        Type::Future(inner) => lower_async_local(function, inner, script, "Future"),
        Type::Stream(_) => Err(async_unsupported(function, "Stream")),
        _ => {
            if let Some(abi) = script
                && let Some(flat) = abi.flat_ir_types(ty)
            {
                return Ok(flat
                    .iter()
                    .map(|flat| match flat {
                        Flat::I32 => ValType::I32,
                        Flat::I64 => ValType::I64,
                    })
                    .collect());
            }
            Err(unsupported(function, "non-scalar local"))
        }
    }
}

/// Projects an internal `Task[T]`/`Future[T]` local to the resolved-value
/// layout in the Script profile, or keeps the RFC-0013 refusal elsewhere.
fn lower_async_local(
    function: &str,
    inner: &Type,
    script: Option<&ScriptAbi>,
    feature: &'static str,
) -> Result<Vec<ValType>, CodegenError> {
    match script.and_then(|abi| abi.flat_ir_types(inner)) {
        Some(flat) => Ok(flat.iter().map(|flat| flat_local_type(*flat)).collect()),
        None => Err(async_unsupported(function, feature)),
    }
}

/// Points an aggregate literal value at its static data-segment bytes.
fn emit_static_bytes(
    context: &CompileContext<'_>,
    body: &mut Function,
    bytes: &[u8],
    result: ValueId,
    feature: &str,
) -> Result<(), CodegenError> {
    let function = context.function;
    let Some(emit) = context.script else {
        return Err(unsupported(&function.name, feature));
    };
    let Some(offset) = emit.data_offsets.get(bytes).copied() else {
        return Err(unsupported(
            &function.name,
            "static aggregate literal layout",
        ));
    };
    let [pointer, length] = context.layout.get(&function.name, result)? else {
        return Err(unsupported(
            &function.name,
            "aggregate literal local layout",
        ));
    };
    body.instruction(&Instruction::I32Const(
        i32::try_from(offset).map_err(|_| unsupported(&function.name, feature))?,
    ));
    body.instruction(&Instruction::LocalSet(*pointer));
    body.instruction(&Instruction::I32Const(
        i32::try_from(bytes.len()).map_err(|_| unsupported(&function.name, feature))?,
    ));
    body.instruction(&Instruction::LocalSet(*length));
    Ok(())
}

fn emit_checked_fixed(
    function: &IrFunction,
    layout: &LocalLayout,
    instruction: &sico_ir::Instruction,
    left: ValueId,
    right: ValueId,
    add: bool,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let Type::Result { ok, .. } = &instruction.ty else {
        return Err(unsupported(&function.name, "checked result type"));
    };
    let [tag, payload] = layout.get(&function.name, instruction.result)? else {
        return Err(unsupported(&function.name, "checked result layout"));
    };
    let left = layout.scalar(&function.name, left)?;
    let right = layout.scalar(&function.name, right)?;

    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::LocalGet(right));
    body.instruction(if add {
        &Instruction::I64Add
    } else {
        &Instruction::I64Sub
    });
    body.instruction(&Instruction::LocalSet(*payload));

    match (ok.as_ref(), add) {
        (Type::I64, true) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::I64And);
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
        }
        (Type::I64, false) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::I64And);
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
        }
        (Type::U64, true) => {
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64LtU);
        }
        (Type::U64, false) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::I64LtU);
        }
        _ => return Err(unsupported(&function.name, "checked fixed operand")),
    }
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::LocalSet(*tag));
    match (ok.as_ref(), add) {
        (Type::I64, _) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
            body.instruction(&Instruction::I64ExtendI32U);
        }
        (Type::U64, true) => {
            body.instruction(&Instruction::I64Const(0));
        }
        (Type::U64, false) => {
            body.instruction(&Instruction::I64Const(1));
        }
        _ => return Err(unsupported(&function.name, "checked fixed error")),
    }
    body.instruction(&Instruction::LocalSet(*payload));
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(*tag));
    body.instruction(&Instruction::End);
    Ok(())
}

/// STEP-0134 checked multiply. The product is computed with wrapping
/// `i64.mul`; overflow is then detected by dividing the product back. The
/// division check is sound on two's complement: for `a != 0` the wrapped
/// product is `p = a*b - k*2^64` with `|k*2^64/a| > 1` whenever `k != 0`,
/// so `trunc(p / a)` can never equal `b` after a real overflow. The two
/// trapping divisions are pre-empted: `a == 0` short-circuits, and
/// `a == -1` overflows iff `b == i64::MIN` without dividing.
fn emit_checked_mul(
    function: &IrFunction,
    layout: &LocalLayout,
    instruction: &sico_ir::Instruction,
    left: ValueId,
    right: ValueId,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let Type::Result { ok, .. } = &instruction.ty else {
        return Err(unsupported(&function.name, "checked result type"));
    };
    let [tag, payload] = layout.get(&function.name, instruction.result)? else {
        return Err(unsupported(&function.name, "checked result layout"));
    };
    let left = layout.scalar(&function.name, left)?;
    let right = layout.scalar(&function.name, right)?;

    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::LocalGet(right));
    body.instruction(&Instruction::I64Mul);
    body.instruction(&Instruction::LocalSet(*payload));

    match ok.as_ref() {
        Type::I64 => {
            emit_i64_mul_overflow_condition(body, left, right, *payload);
        }
        Type::U64 => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64Eqz);
            body.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64DivU);
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::I64Ne);
            body.instruction(&Instruction::End);
        }
        _ => return Err(unsupported(&function.name, "checked fixed operand")),
    }
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::LocalSet(*tag));
    // error payload: the direction of the lost product — same-sign operands
    // overflow positive, mixed signs underflow.
    match ok.as_ref() {
        Type::I64 => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
            body.instruction(&Instruction::I32Xor);
            body.instruction(&Instruction::I64ExtendI32U);
        }
        Type::U64 => {
            body.instruction(&Instruction::I64Const(0));
        }
        _ => return Err(unsupported(&function.name, "checked fixed error")),
    }
    body.instruction(&Instruction::LocalSet(*payload));
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(*tag));
    body.instruction(&Instruction::End);
    Ok(())
}

/// I64 overflow condition for `emit_checked_mul`: `a == 0` never overflows;
/// `a == -1` overflows iff `b == i64::MIN`; otherwise divide back. The
/// nested result-typed If arms each leave one i32 on the stack.
fn emit_i64_mul_overflow_condition(body: &mut Function, left: u32, right: u32, payload: u32) {
    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::I64Eqz);
    body.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::I64Const(-1));
    body.instruction(&Instruction::I64Eq);
    body.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
    body.instruction(&Instruction::LocalGet(right));
    body.instruction(&Instruction::I64Const(i64::MIN));
    body.instruction(&Instruction::I64Eq);
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::LocalGet(payload));
    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::I64DivS);
    body.instruction(&Instruction::LocalGet(right));
    body.instruction(&Instruction::I64Ne);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::End);
}

/// STEP-0134 checked divide. Division by zero reports
/// `NumericError.division-by-zero`; `i64::MIN / -1` reports `overflow`
/// (the only signed-overflow case); every other division is exact and
/// trap-free.
fn emit_checked_div(
    function: &IrFunction,
    layout: &LocalLayout,
    instruction: &sico_ir::Instruction,
    left: ValueId,
    right: ValueId,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let Type::Result { ok, .. } = &instruction.ty else {
        return Err(unsupported(&function.name, "checked result type"));
    };
    let [tag, payload] = layout.get(&function.name, instruction.result)? else {
        return Err(unsupported(&function.name, "checked result layout"));
    };
    let left = layout.scalar(&function.name, left)?;
    let right = layout.scalar(&function.name, right)?;

    body.instruction(&Instruction::LocalGet(right));
    body.instruction(&Instruction::I64Eqz);
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::LocalSet(*tag));
    body.instruction(&Instruction::I64Const(2));
    body.instruction(&Instruction::LocalSet(*payload));
    body.instruction(&Instruction::Else);
    if matches!(ok.as_ref(), Type::I64) {
        body.instruction(&Instruction::LocalGet(left));
        body.instruction(&Instruction::I64Const(i64::MIN));
        body.instruction(&Instruction::I64Eq);
        body.instruction(&Instruction::LocalGet(right));
        body.instruction(&Instruction::I64Const(-1));
        body.instruction(&Instruction::I64Eq);
        body.instruction(&Instruction::I32And);
        body.instruction(&Instruction::If(BlockType::Empty));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::LocalSet(*tag));
        body.instruction(&Instruction::I64Const(0));
        body.instruction(&Instruction::LocalSet(*payload));
        body.instruction(&Instruction::Else);
    }
    if !matches!(ok.as_ref(), Type::I64 | Type::U64) {
        return Err(unsupported(&function.name, "checked fixed operand"));
    }
    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::LocalGet(right));
    body.instruction(if matches!(ok.as_ref(), Type::I64) {
        &Instruction::I64DivS
    } else {
        &Instruction::I64DivU
    });
    body.instruction(&Instruction::LocalSet(*payload));
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(*tag));
    if matches!(ok.as_ref(), Type::I64) {
        body.instruction(&Instruction::End);
    }
    body.instruction(&Instruction::End);
    Ok(())
}

fn emit_fixed_comparison(
    function: &IrFunction,
    layout: &LocalLayout,
    instruction: &sico_ir::Instruction,
    left: ValueId,
    right: ValueId,
    equal: bool,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let left_slot = layout.scalar(&function.name, left)?;
    let right_slot = layout.scalar(&function.name, right)?;
    body.instruction(&Instruction::LocalGet(left_slot));
    body.instruction(&Instruction::LocalGet(right_slot));
    if equal {
        body.instruction(&Instruction::I64Eq);
    } else {
        let ty = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find(|candidate| candidate.result == left)
            .map(|candidate| &candidate.ty)
            .or_else(|| {
                function
                    .parameters
                    .iter()
                    .find(|parameter| parameter.id == left)
                    .map(|parameter| &parameter.ty)
            })
            .ok_or_else(|| unsupported(&function.name, "comparison operand type"))?;
        body.instruction(if *ty == Type::I64 {
            &Instruction::I64LtS
        } else {
            &Instruction::I64LtU
        });
    }
    body.instruction(&Instruction::LocalSet(
        layout.scalar(&function.name, instruction.result)?,
    ));
    Ok(())
}

fn emit_return(
    context: &CompileContext<'_>,
    body: &mut Function,
    value: Option<ValueId>,
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let abi = context.abi;
    if let (Some(emit), Some(value)) = (context.script, value)
        && function.name == "run"
    {
        return emit_script_return(context, body, value, emit);
    }
    if let Some(value) = value {
        if abi == CoreAbi::Direct && is_checked_fixed_result(&function.return_type) {
            // STEP-0148: the packed value form is [tag, payload] with the
            // payload carrying the ok value or the numeric error tag; the
            // declared core width is [tag, ok, error]. Widen with the
            // losing side zeroed.
            let [tag, payload] = layout.get(&function.name, value)? else {
                return Err(unsupported(&function.name, "checked result layout"));
            };
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::Select);
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::Select);
        } else if abi == CoreAbi::ComponentLift && is_checked_fixed_result(&function.return_type) {
            let [tag, payload] = layout.get(&function.name, value)? else {
                return Err(unsupported(&function.name, "checked result layout"));
            };
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Store(MemArg {
                offset: 8,
                align: 3,
                memory_index: 0,
            }));
            body.instruction(&Instruction::I32Const(0));
        } else {
            for slot in layout.get(&function.name, value)? {
                body.instruction(&Instruction::LocalGet(*slot));
            }
        }
    }
    body.instruction(&Instruction::Return);
    Ok(())
}

/// Lowers the Script `run` result into the 32-byte Canonical ABI return area
/// and returns its pointer. Result payload bytes are copied into the bounded
/// arena so result buffers never alias caller-supplied parameter storage.
fn emit_script_return(
    context: &CompileContext<'_>,
    body: &mut Function,
    value: ValueId,
    emit: &ScriptEmit,
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some((return_area, copy_destination)) = context.scratch else {
        return Err(unsupported(&function.name, "script return scratch layout"));
    };
    let Type::Result { .. } = &function.return_type else {
        return Err(unsupported(&function.name, "script result type"));
    };
    let value_layout = layout.layout(&function.name, value)?;
    let has_ok = emit.abi.output.fields.iter().all(|field| {
        value_layout
            .fields
            .contains_key(&ir_field_name(&field.name))
    });
    let has_error = emit.abi.error.fields.iter().all(|field| {
        value_layout
            .fields
            .contains_key(&ir_field_name(&field.name))
    });
    if !has_ok && !has_error {
        return Err(unsupported(&function.name, "script result payload layout"));
    }
    let [tag, ..] = layout.get(&function.name, value)? else {
        return Err(unsupported(&function.name, "script result local layout"));
    };

    body.instruction(&Instruction::I32Const(
        i32::try_from(emit.abi.result_size)
            .map_err(|_| unsupported(&function.name, "script result area"))?,
    ));
    body.instruction(&Instruction::Call(emit.alloc_index));
    body.instruction(&Instruction::LocalSet(return_area));
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::LocalGet(*tag));
    body.instruction(&Instruction::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    if has_ok && has_error {
        // Values produced by calls carry both payload field maps; select
        // the record to lower from the runtime tag instead of statically.
        body.instruction(&Instruction::LocalGet(*tag));
        body.instruction(&Instruction::I32Eqz);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        emit_result_fields(
            context,
            body,
            emit,
            &emit.abi.output,
            value,
            return_area,
            copy_destination,
        )?;
        body.instruction(&Instruction::Else);
        emit_result_fields(
            context,
            body,
            emit,
            &emit.abi.error,
            value,
            return_area,
            copy_destination,
        )?;
        body.instruction(&Instruction::End);
    } else {
        let record = if has_ok {
            &emit.abi.output
        } else {
            &emit.abi.error
        };
        emit_result_fields(
            context,
            body,
            emit,
            record,
            value,
            return_area,
            copy_destination,
        )?;
    }
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::Return);
    Ok(())
}

/// Stores one result payload record: lists are copied into the arena and
/// scalars are stored at their Canonical ABI byte offsets. Fields are
/// resolved by name, so source construction order cannot corrupt the ABI.
fn emit_result_fields(
    context: &CompileContext<'_>,
    body: &mut Function,
    emit: &ScriptEmit,
    record: &canonical::RecordLayout,
    value: ValueId,
    return_area: u32,
    copy_destination: u32,
) -> Result<(), CodegenError> {
    let function = context.function;
    for field in &record.fields {
        let field_slots =
            context
                .layout
                .field(&function.name, value, &ir_field_name(&field.name))?;
        let offset = emit.abi.result_payload_offset + field.byte_offset;
        match field_slots.as_slice() {
            [pointer, length] => {
                body.instruction(&Instruction::LocalGet(*length));
                body.instruction(&Instruction::Call(emit.alloc_index));
                body.instruction(&Instruction::LocalSet(copy_destination));
                body.instruction(&Instruction::LocalGet(copy_destination));
                body.instruction(&Instruction::LocalGet(*pointer));
                body.instruction(&Instruction::LocalGet(*length));
                body.instruction(&Instruction::MemoryCopy {
                    dst_mem: 0,
                    src_mem: 0,
                });
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::LocalGet(copy_destination));
                body.instruction(&Instruction::I32Store(MemArg {
                    offset: u64::from(offset),
                    align: 2,
                    memory_index: 0,
                }));
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::LocalGet(*length));
                body.instruction(&Instruction::I32Store(MemArg {
                    offset: u64::from(offset + 4),
                    align: 2,
                    memory_index: 0,
                }));
            }
            [scalar] if record.flat.get(field.slot_start) == Some(&Flat::I64) => {
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::LocalGet(*scalar));
                body.instruction(&Instruction::I64Store(MemArg {
                    offset: u64::from(offset),
                    align: 3,
                    memory_index: 0,
                }));
            }
            [scalar] => {
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::LocalGet(*scalar));
                body.instruction(&Instruction::I32Store8(MemArg {
                    offset: u64::from(offset),
                    align: 0,
                    memory_index: 0,
                }));
            }
            _ => return Err(unsupported(&function.name, "script result field kind")),
        }
    }
    Ok(())
}

fn is_checked_fixed_result(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE)
    )
}

/// Maps a WIT kebab-case field name to the source-level `snake_case` name
/// used in IR Construct/Project operations (`exit-code` ↔ `exit_code`).
fn ir_field_name(wit_name: &str) -> String {
    wit_name.replace('-', "_")
}

/// Maps a WIT kebab-case enum case to the source-level CamelCase variant name
/// (`domain-error` ↔ `DomainError`).
fn kebab_to_camel(kebab: &str) -> String {
    kebab
        .split('-')
        .map(|segment| {
            let mut characters = segment.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn unsupported(function: &str, feature: &str) -> CodegenError {
    CodegenError::Unsupported {
        function: function.to_owned(),
        feature: feature.to_owned(),
    }
}

fn async_unsupported(function: &str, feature: &str) -> CodegenError {
    let contract = match feature {
        "Task" => "structured Task remains language-local; no WIT Task handle",
        "Future" => "Future lowering requires an async Component function and cancellation edge",
        "Stream" => "Stream lowering requires an explicit bound and close/cancel edge",
        _ => "unsupported asynchronous boundary",
    };
    CodegenError::AsyncUnsupported {
        function: function.to_owned(),
        feature: feature.to_owned(),
        contract,
    }
}

/// Emits one STEP-0083 stdlib intrinsic over canonical `(ptr, len)` pairs.
#[allow(clippy::too_many_lines)]
/// Lowers `sico.http.request` (RFC-0031): three (pointer, length) argument
/// pairs plus the 24-byte result area; the response record spreads into the
/// full-width `Result[HttpResponse, Text]` layout through selects.
fn emit_http_call(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "http call outside script profile",
        ));
    };
    let Some((return_area, copy_destination)) = context.scratch else {
        return Err(unsupported(&function.name, "http call scratch layout"));
    };
    let import = emit
        .http_index
        .ok_or_else(|| unsupported(&function.name, "http import layout"))?;
    let mem = |offset: u64, align: u32| MemArg {
        offset,
        align,
        memory_index: 0,
    };

    body.instruction(&Instruction::I32Const(
        i32::try_from(canonical::HTTP_RESULT_SIZE)
            .map_err(|_| unsupported(&function.name, "http result area"))?,
    ));
    body.instruction(&Instruction::Call(emit.alloc_index));
    body.instruction(&Instruction::LocalSet(return_area));
    for offset in [0_u64, 4, 8, 12, 16, 20] {
        body.instruction(&Instruction::LocalGet(return_area));
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::I32Store(mem(offset, 2)));
    }
    for argument in arguments {
        for slot in layout.get(&function.name, *argument)? {
            body.instruction(&Instruction::LocalGet(*slot));
        }
    }
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::Call(import));

    let result = layout.get(&function.name, instruction.result)?;
    let [
        tag,
        ok_status,
        ok_pointer,
        ok_length,
        err_pointer,
        err_length,
    ] = result
    else {
        return Err(unsupported(&function.name, "http result layout"));
    };
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load8U(mem(0, 0)));
    body.instruction(&Instruction::LocalSet(*tag));
    // status (s64 at 8)
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I64Load(mem(8, 3)));
    body.instruction(&Instruction::LocalSet(*ok_status));
    // response body pointer/length at 16/20
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load(mem(16, 2)));
    body.instruction(&Instruction::LocalSet(copy_destination));
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load(mem(20, 2)));
    body.instruction(&Instruction::LocalSet(*ok_length));
    // error string pointer/length at 8/12
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load(mem(8, 2)));
    body.instruction(&Instruction::LocalSet(*err_pointer));
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load(mem(12, 2)));
    body.instruction(&Instruction::LocalSet(*err_length));
    // select the live side by the tag; losing side is already zero
    for (source, destination, err_side) in [
        (copy_destination, ok_pointer, false),
        (*ok_length, ok_length, false),
        (*err_pointer, err_pointer, true),
        (*err_length, err_length, true),
    ] {
        if err_side {
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(source));
        } else {
            body.instruction(&Instruction::LocalGet(source));
            body.instruction(&Instruction::I32Const(0));
        }
        body.instruction(&Instruction::LocalGet(*tag));
        body.instruction(&Instruction::I32Eqz);
        body.instruction(&Instruction::Select);
        body.instruction(&Instruction::LocalSet(*destination));
    }
    // status is meaningful only on success
    body.instruction(&Instruction::LocalGet(*ok_status));
    body.instruction(&Instruction::I64Const(0));
    body.instruction(&Instruction::LocalGet(*tag));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::Select);
    body.instruction(&Instruction::LocalSet(*ok_status));
    Ok(())
}

/// Splits a full user import name into its instance identity and function
/// name: `sico:user/<interface>@<version>.<function>`.
fn split_user_import_name(name: &str) -> Option<(&str, &str)> {
    // The instance identity carries semver (`csv@1.0.0`) with dots; the
    // function name is the last dot-separated segment. The full instance
    // identity (prefix included) is the import name both sides agree on.
    if !name.starts_with("sico:user/") {
        return None;
    }
    let (instance, function) = name.rsplit_once('.')?;
    Some((instance, function))
}

/// Lowers one RFC-0039 §2.4 user-WIT package call (STEP-0147, amendment A7).
/// Arguments flatten directly (scalars one slot; `Text`/`Bytes`/`List[T]`
/// their `(ptr, len)` pair into the shared transport memory); the result is
/// always written by the callee into a caller-allocated return area —
/// the exact convention of the fs/stream/http imports. Result shapes lay
/// out as a discriminant byte at 0 plus the payload at 4 (both v0 cases
/// are pointer-length or byte payloads).
#[allow(clippy::too_many_lines)] // the per-shape result reads are one coherent table
fn emit_user_call(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    name: &str,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "user import call outside script profile",
        ));
    };
    let Some((return_area, copy_destination)) = context.scratch else {
        return Err(unsupported(&function.name, "user import scratch layout"));
    };
    let import_index = emit
        .user_indices
        .get(name)
        .copied()
        .ok_or_else(|| unsupported(&function.name, "user import index layout"))?;
    let signature = emit
        .user_imports
        .iter()
        .find(|import| name.starts_with(&format!("{}.", import.name)))
        .and_then(|import| {
            import
                .functions
                .iter()
                .find(|entry| format!("{}.{}", import.name, entry.name) == name)
        })
        .ok_or_else(|| unsupported(&function.name, "user import signature layout"))?;
    let mem = |offset: u64, align: u32| MemArg {
        offset,
        align,
        memory_index: 0,
    };

    // Caller-allocated return area, zeroed before the call.
    let area_size = canonical::user_result_area_size(&signature.result);
    body.instruction(&Instruction::I32Const(
        i32::try_from(area_size.max(1)).unwrap_or(1),
    ));
    body.instruction(&Instruction::Call(emit.alloc_index));
    body.instruction(&Instruction::LocalSet(return_area));
    let mut offset = 0_u64;
    while offset < u64::from(area_size) {
        body.instruction(&Instruction::LocalGet(return_area));
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::I32Store(mem(offset, 2)));
        offset += 4;
    }
    // Flattened arguments, then the area pointer.
    if arguments.len() != signature.parameters.len() {
        return Err(unsupported(&function.name, "user import arity"));
    }
    for (argument, parameter) in arguments.iter().zip(&signature.parameters) {
        let slots = layout.get(&function.name, *argument)?;
        let expected = match parameter {
            sico_ir::Type::Bool | sico_ir::Type::I64 | sico_ir::Type::U64 => 1,
            sico_ir::Type::String | sico_ir::Type::Bytes | sico_ir::Type::List(_) => 2,
            _ => {
                return Err(unsupported(
                    &function.name,
                    "user import parameter value set",
                ));
            }
        };
        if slots.len() != expected {
            return Err(unsupported(&function.name, "user import operand layout"));
        }
        for slot in slots {
            body.instruction(&Instruction::LocalGet(*slot));
        }
    }
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::Call(import_index));

    // Read the area into the result's guest slots.
    let result_slots = layout.get(&function.name, instruction.result)?;
    match &signature.result {
        sico_ir::Type::Unit => {}
        sico_ir::Type::Bool => {
            let [value] = result_slots else {
                return Err(unsupported(&function.name, "user import result layout"));
            };
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load8U(mem(0, 0)));
            body.instruction(&Instruction::LocalSet(*value));
        }
        sico_ir::Type::I64 | sico_ir::Type::U64 => {
            let [value] = result_slots else {
                return Err(unsupported(&function.name, "user import result layout"));
            };
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I64Load(mem(0, 3)));
            body.instruction(&Instruction::LocalSet(*value));
        }
        sico_ir::Type::String | sico_ir::Type::Bytes | sico_ir::Type::List(_) => {
            let [pointer, length] = result_slots else {
                return Err(unsupported(&function.name, "user import result layout"));
            };
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(0, 2)));
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(4, 2)));
            body.instruction(&Instruction::LocalSet(*length));
        }
        sico_ir::Type::Result { ok, .. } => {
            let [tag, ok_slots @ .., err_ptr, err_len] = result_slots else {
                return Err(unsupported(&function.name, "user import result layout"));
            };
            // Discriminant byte, then the joined payload slot pair at 4/8.
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load8U(mem(0, 0)));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(4, 2)));
            body.instruction(&Instruction::LocalSet(copy_destination));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(8, 2)));
            body.instruction(&Instruction::LocalSet(return_area));
            // Error side (keep the payload when tag != 0) …
            for (source, destination) in [(copy_destination, *err_ptr), (return_area, *err_len)] {
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::LocalGet(source));
                body.instruction(&Instruction::LocalGet(*tag));
                body.instruction(&Instruction::I32Eqz);
                body.instruction(&Instruction::Select);
                body.instruction(&Instruction::LocalSet(destination));
            }
            // … then the ok side (keep when tag == 0): the ok pointer slot
            // first, then the ok length slot when the ok payload is a
            // pointer-length shape (Unit/Bool carry at most one slot).
            if let Some(destination) = ok_slots.first().copied() {
                body.instruction(&Instruction::LocalGet(copy_destination));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::LocalGet(*tag));
                body.instruction(&Instruction::I32Eqz);
                body.instruction(&Instruction::Select);
                body.instruction(&Instruction::LocalSet(destination));
            }
            if let Some(destination) = ok_slots.get(1).copied() {
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::LocalGet(*tag));
                body.instruction(&Instruction::I32Eqz);
                body.instruction(&Instruction::Select);
                body.instruction(&Instruction::LocalSet(destination));
            }
            let _ = ok;
        }
        _ => return Err(unsupported(&function.name, "user import result value set")),
    }
    Ok(())
}

/// Lowers `sico.http2.request` (STEP-0136, RFC-0037): the buffered one-shot
/// over `sico:script/http@0.2.0` with Host-default options (empty headers,
/// no redirect follow, no retry, Host-default timeout). The 32-byte result
/// area carries tag@0; on ok, status s64 @8 and body ptr/len @24/28 (the
/// response headers @16/20 are lifted and deliberately narrowed away); on
/// error, the http-error discriminant i32 @8 maps to the static case-name
/// table in the data segment.
#[allow(clippy::too_many_lines)]
fn emit_http2_call(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "http2 call outside script profile",
        ));
    };
    let Some((return_area, copy_destination)) = context.scratch else {
        return Err(unsupported(&function.name, "http2 call scratch layout"));
    };
    let import = emit
        .http2_index
        .ok_or_else(|| unsupported(&function.name, "http2 import layout"))?;
    let error_table = emit
        .http2_error_table
        .ok_or_else(|| unsupported(&function.name, "http2 error table layout"))?;
    let mem = |offset: u64, align: u32| MemArg {
        offset,
        align,
        memory_index: 0,
    };

    body.instruction(&Instruction::I32Const(
        i32::try_from(canonical::HTTP2_RESULT_SIZE)
            .map_err(|_| unsupported(&function.name, "http2 result area"))?,
    ));
    body.instruction(&Instruction::Call(emit.alloc_index));
    body.instruction(&Instruction::LocalSet(return_area));
    for offset in [0_u64, 4, 8, 12, 16, 20, 24, 28] {
        body.instruction(&Instruction::LocalGet(return_area));
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::I32Store(mem(offset, 2)));
    }
    // Wire order: method pair, url pair, the flattened default options
    // (empty headers (0, 0), follow-redirects 0, retry-idempotent 0,
    // timeout-ms 0 = Host default), body pair, result-area pointer.
    let [method, url, body_argument] = arguments else {
        return Err(unsupported(&function.name, "http2 request arity"));
    };
    for argument in [method, url] {
        for slot in layout.get(&function.name, *argument)? {
            body.instruction(&Instruction::LocalGet(*slot));
        }
    }
    for instruction in [
        Instruction::I32Const(0),
        Instruction::I32Const(0),
        Instruction::I32Const(0),
        Instruction::I32Const(0),
        Instruction::I64Const(0),
    ] {
        body.instruction(&instruction);
    }
    for slot in layout.get(&function.name, *body_argument)? {
        body.instruction(&Instruction::LocalGet(*slot));
    }
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::Call(import));

    let result = layout.get(&function.name, instruction.result)?;
    let [
        tag,
        ok_status,
        ok_pointer,
        ok_length,
        err_pointer,
        err_length,
    ] = result
    else {
        return Err(unsupported(&function.name, "http2 result layout"));
    };
    for instruction in [
        Instruction::LocalGet(return_area),
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalSet(*tag),
        // status (s64 at 8)
        Instruction::LocalGet(return_area),
        Instruction::I64Load(mem(8, 3)),
        Instruction::LocalSet(*ok_status),
        // response body pointer/length at 24/28
        Instruction::LocalGet(return_area),
        Instruction::I32Load(mem(24, 2)),
        Instruction::LocalSet(copy_destination),
        Instruction::LocalGet(return_area),
        Instruction::I32Load(mem(28, 2)),
        Instruction::LocalSet(*ok_length),
        // http-error discriminant (i32 at 8) → case-name table lookup
        Instruction::LocalGet(return_area),
        Instruction::I32Load(mem(8, 2)),
        // On the ok side offset 8 holds the low status half, not a
        // discriminant — force code 0 so the bounds check never traps
        // (the table result is discarded by the select below anyway).
        Instruction::I32Const(0),
        Instruction::LocalGet(*tag),
        Instruction::Select,
        Instruction::LocalSet(return_area),
        Instruction::LocalGet(return_area),
        Instruction::I32Const(8),
        Instruction::I32GeU,
    ] {
        body.instruction(&instruction);
    }
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    let table = i32::try_from(error_table)
        .map_err(|_| unsupported(&function.name, "http2 error table layout"))?;
    for instruction in [
        Instruction::LocalGet(return_area),
        Instruction::I32Const(3),
        Instruction::I32Shl,
        Instruction::I32Const(table),
        Instruction::I32Add,
        Instruction::LocalSet(return_area),
        Instruction::LocalGet(return_area),
        Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(*err_pointer),
        Instruction::LocalGet(return_area),
        Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(*err_length),
    ] {
        body.instruction(&instruction);
    }
    // select the live side by the tag; losing side is already zero
    for (source, destination, err_side) in [
        (copy_destination, ok_pointer, false),
        (*ok_length, ok_length, false),
        (*err_pointer, err_pointer, true),
        (*err_length, err_length, true),
    ] {
        if err_side {
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(source));
        } else {
            body.instruction(&Instruction::LocalGet(source));
            body.instruction(&Instruction::I32Const(0));
        }
        body.instruction(&Instruction::LocalGet(*tag));
        body.instruction(&Instruction::I32Eqz);
        body.instruction(&Instruction::Select);
        body.instruction(&Instruction::LocalSet(*destination));
    }
    // status is meaningful only on success
    body.instruction(&Instruction::LocalGet(*ok_status));
    body.instruction(&Instruction::I64Const(0));
    body.instruction(&Instruction::LocalGet(*tag));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::Select);
    body.instruction(&Instruction::LocalSet(*ok_status));
    Ok(())
}

/// Lowers one scoped `sico.fs.*` call. Arguments cross as (pointer, length)
/// pairs into the shared transport memory; the 12-byte Canonical ABI result
/// area is zeroed before the call and fanned out into the full-width
/// `Result[ok, Text]` layout after it (select keeps the losing side zeroed).
fn emit_fs_call(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    name: &str,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "fs call outside script profile",
        ));
    };
    let Some((return_area, copy_destination)) = context.scratch else {
        return Err(unsupported(&function.name, "fs call scratch layout"));
    };
    let import = emit
        .fs_indices
        .get(name)
        .copied()
        .ok_or_else(|| unsupported(&function.name, "fs import layout"))?;
    let mem = |offset: u64, align: u32| MemArg {
        offset,
        align,
        memory_index: 0,
    };

    body.instruction(&Instruction::I32Const(
        i32::try_from(canonical::FS_RESULT_SIZE)
            .map_err(|_| unsupported(&function.name, "fs result area"))?,
    ));
    body.instruction(&Instruction::Call(emit.alloc_index));
    body.instruction(&Instruction::LocalSet(return_area));
    for offset in [0_u64, 4, 8] {
        body.instruction(&Instruction::LocalGet(return_area));
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::I32Store(mem(offset, 2)));
    }
    for argument in arguments {
        for slot in layout.get(&function.name, *argument)? {
            body.instruction(&Instruction::LocalGet(*slot));
        }
    }
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::Call(import));

    let result = layout.get(&function.name, instruction.result)?;
    let [tag, rest @ ..] = result else {
        return Err(unsupported(&function.name, "fs result layout"));
    };
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load8U(mem(0, 0)));
    body.instruction(&Instruction::LocalSet(*tag));
    // Payload halves: copy_destination = pointer, return_area = length.
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load(mem(4, 2)));
    body.instruction(&Instruction::LocalSet(copy_destination));
    body.instruction(&Instruction::LocalGet(return_area));
    body.instruction(&Instruction::I32Load(mem(8, 2)));
    body.instruction(&Instruction::LocalSet(return_area));
    let select_payload = |body: &mut Function, source: u32, destination: u32, err_side: bool| {
        // tag == 0 keeps the ok side, tag == 1 keeps the error side; the
        // losing side of the full-width layout stays zero.
        if err_side {
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(source));
        } else {
            body.instruction(&Instruction::LocalGet(source));
            body.instruction(&Instruction::I32Const(0));
        }
        body.instruction(&Instruction::LocalGet(*tag));
        body.instruction(&Instruction::I32Eqz);
        body.instruction(&Instruction::Select);
        body.instruction(&Instruction::LocalSet(destination));
    };
    if name == "sico.fs.read" {
        let [ok_pointer, ok_length, err_pointer, err_length] = rest else {
            return Err(unsupported(&function.name, "fs.read result layout"));
        };
        select_payload(body, copy_destination, *err_pointer, true);
        select_payload(body, return_area, *err_length, true);
        select_payload(body, copy_destination, *ok_pointer, false);
        select_payload(body, return_area, *ok_length, false);
    } else {
        let [ok_value, err_pointer, err_length] = rest else {
            return Err(unsupported(&function.name, "fs result layout"));
        };
        // exists/write carry a unit-ish ok: materialize `true`.
        body.instruction(&Instruction::LocalGet(*tag));
        body.instruction(&Instruction::I32Eqz);
        body.instruction(&Instruction::LocalSet(*ok_value));
        select_payload(body, copy_destination, *err_pointer, true);
        select_payload(body, return_area, *err_length, true);
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
/// Loads the error-text (pointer, length) pair for a stream-error enum held
/// in `code_local` into (`err_pointer`, `err_length`), trapping on any
/// discriminant outside the frozen WIT declaration order.
fn emit_stream_error_text(body: &mut Function, code_local: u32, err_pointer: u32, err_length: u32) {
    // bounds check: code must be < 4 (io/cancelled/closed/resource-limit)
    body.instruction(&Instruction::LocalGet(code_local));
    body.instruction(&Instruction::I32Const(4));
    body.instruction(&Instruction::I32GeU);
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    // entry = code * 8; table base is absolute 0 in the data segment
    body.instruction(&Instruction::LocalGet(code_local));
    body.instruction(&Instruction::I32Const(3));
    body.instruction(&Instruction::I32Shl);
    body.instruction(&Instruction::LocalSet(err_pointer));
    body.instruction(&Instruction::LocalGet(err_pointer));
    body.instruction(&Instruction::I32Load(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    body.instruction(&Instruction::LocalSet(err_pointer));
    body.instruction(&Instruction::LocalGet(code_local));
    body.instruction(&Instruction::I32Const(3));
    body.instruction(&Instruction::I32Shl);
    body.instruction(&Instruction::LocalSet(err_length));
    body.instruction(&Instruction::LocalGet(err_length));
    body.instruction(&Instruction::I32Load(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    body.instruction(&Instruction::LocalSet(err_length));
}

/// Lowers one `sico.stream.*` call (RFC-0030). Handles are Canonical ABI
/// resource indices; error enums map to static texts through the
/// data-segment table written next to the literals.
#[allow(clippy::too_many_lines)]
fn emit_stream_call(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    name: &str,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "stream call outside script profile",
        ));
    };
    let Some((return_area, copy_destination)) = context.scratch else {
        return Err(unsupported(&function.name, "stream call scratch layout"));
    };
    let import = emit
        .stream_indices
        .get(name)
        .copied()
        .ok_or_else(|| unsupported(&function.name, "stream import layout"))?;
    let result = layout.get(&function.name, instruction.result)?;
    let mem = |offset: u64, align: u32| MemArg {
        offset,
        align,
        memory_index: 0,
    };
    let scalar = |index: usize| layout.scalar(&function.name, arguments[index]);

    match name {
        "sico.stream.stdin" | "sico.stream.stdout" | "sico.stream.stderr" => {
            let [handle] = result else {
                return Err(unsupported(
                    &function.name,
                    "stream constructor result layout",
                ));
            };
            body.instruction(&Instruction::Call(import));
            body.instruction(&Instruction::LocalSet(*handle));
        }
        "sico.stream.close_input" | "sico.stream.close_output" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "stream close result layout"));
            };
            body.instruction(&Instruction::LocalGet(scalar(0)?));
            body.instruction(&Instruction::Call(import));
            body.instruction(&Instruction::I32Const(1));
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.stream.read" => {
            let [tag, ok_pointer, ok_length, err_pointer, err_length] = result else {
                return Err(unsupported(&function.name, "stream.read result layout"));
            };
            body.instruction(&Instruction::I32Const(
                i32::try_from(canonical::STREAM_READ_RESULT_SIZE)
                    .map_err(|_| unsupported(&function.name, "stream result area"))?,
            ));
            body.instruction(&Instruction::Call(emit.alloc_index));
            body.instruction(&Instruction::LocalSet(return_area));
            for offset in [0_u64, 4, 8] {
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Store(mem(offset, 2)));
            }
            body.instruction(&Instruction::LocalGet(scalar(0)?));
            body.instruction(&Instruction::LocalGet(scalar(1)?));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::Call(import));
            // tag
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load8U(mem(0, 0)));
            body.instruction(&Instruction::LocalSet(*tag));
            // payload pointer/length (union with the error enum)
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(4, 2)));
            body.instruction(&Instruction::LocalSet(copy_destination));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(8, 2)));
            body.instruction(&Instruction::LocalSet(return_area));
            // ok side keeps the payload when tag == 0
            body.instruction(&Instruction::LocalGet(copy_destination));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::Select);
            body.instruction(&Instruction::LocalSet(*ok_pointer));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::Select);
            body.instruction(&Instruction::LocalSet(*ok_length));
            // error side maps the enum through the static text table
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_stream_error_text(body, copy_destination, *err_pointer, *err_length);
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*err_pointer));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*err_length));
            body.instruction(&Instruction::End);
        }
        "sico.stream.pump" => {
            // result<u64, stream-error>: tag at 0, payload (u64 or enum) at 8
            let [tag, ok_value, err_pointer, err_length] = result else {
                return Err(unsupported(&function.name, "stream.pump result layout"));
            };
            body.instruction(&Instruction::I32Const(16));
            body.instruction(&Instruction::Call(emit.alloc_index));
            body.instruction(&Instruction::LocalSet(return_area));
            for offset in [0_u64, 4, 8, 12] {
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Store(mem(offset, 2)));
            }
            body.instruction(&Instruction::LocalGet(scalar(0)?));
            body.instruction(&Instruction::LocalGet(scalar(1)?));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::Call(import));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load8U(mem(0, 0)));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I64Load(mem(8, 3)));
            body.instruction(&Instruction::LocalSet(*ok_value));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(8, 2)));
            body.instruction(&Instruction::LocalSet(copy_destination));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_stream_error_text(body, copy_destination, *err_pointer, *err_length);
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*err_pointer));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*err_length));
            body.instruction(&Instruction::End);
        }
        _ => {
            // write / flush: tag at 0, error enum at 4
            let [tag, ok_value, err_pointer, err_length] = result else {
                return Err(unsupported(&function.name, "stream.write result layout"));
            };
            body.instruction(&Instruction::I32Const(
                i32::try_from(canonical::STREAM_WRITE_RESULT_SIZE)
                    .map_err(|_| unsupported(&function.name, "stream result area"))?,
            ));
            body.instruction(&Instruction::Call(emit.alloc_index));
            body.instruction(&Instruction::LocalSet(return_area));
            for offset in [0_u64, 4] {
                body.instruction(&Instruction::LocalGet(return_area));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Store(mem(offset, 2)));
            }
            body.instruction(&Instruction::LocalGet(scalar(0)?));
            if name == "sico.stream.write" {
                let [pointer, length] = layout.get(&function.name, arguments[1])? else {
                    return Err(unsupported(&function.name, "stream.write payload layout"));
                };
                body.instruction(&Instruction::LocalGet(*pointer));
                body.instruction(&Instruction::LocalGet(*length));
            }
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::Call(import));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load8U(mem(0, 0)));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::LocalSet(*ok_value));
            body.instruction(&Instruction::LocalGet(return_area));
            body.instruction(&Instruction::I32Load(mem(4, 2)));
            body.instruction(&Instruction::LocalSet(copy_destination));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_stream_error_text(body, copy_destination, *err_pointer, *err_length);
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*err_pointer));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*err_length));
            body.instruction(&Instruction::End);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn emit_intrinsic(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    name: &str,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let Some(emit) = context.script else {
        return Err(unsupported(
            &function.name,
            "stdlib intrinsic outside script profile",
        ));
    };
    let result = layout.get(&function.name, instruction.result)?;
    let pair = |index: usize| -> Result<[u32; 2], CodegenError> {
        let [pointer, length] = layout.get(&function.name, arguments[index])? else {
            return Err(unsupported(
                &function.name,
                "stdlib intrinsic operand layout",
            ));
        };
        Ok([*pointer, *length])
    };
    let scalar = |index: usize| layout.scalar(&function.name, arguments[index]);
    let helper = |helper_name: &str| -> Result<u32, CodegenError> {
        emit.helpers
            .get(helper_name)
            .copied()
            .ok_or_else(|| unsupported(&function.name, "stdlib helper layout"))
    };
    match name {
        "sico.http.request" => {
            emit_http_call(context, body, instruction, arguments)?;
        }
        "sico.http2.request" => {
            emit_http2_call(context, body, instruction, arguments)?;
        }
        _ if name.starts_with("sico:user/") => {
            emit_user_call(context, body, instruction, name, arguments)?;
        }
        _ if sico_ir::collection_intrinsic(name).is_some() => {
            emit_collection_call(context, body, instruction, name, arguments)?;
        }
        "sico.fs.read" | "sico.fs.exists" | "sico.fs.write" => {
            emit_fs_call(context, body, instruction, name, arguments)?;
        }
        "sico.stream.stdin"
        | "sico.stream.stdout"
        | "sico.stream.stderr"
        | "sico.stream.read"
        | "sico.stream.write"
        | "sico.stream.flush"
        | "sico.stream.pump"
        | "sico.stream.close_input"
        | "sico.stream.close_output" => {
            emit_stream_call(context, body, instruction, name, arguments)?;
        }
        "sico.bytes.length" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "bytes.length result layout"));
            };
            let [_, length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(length));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.text.encode" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "text.encode result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::LocalSet(*length));
        }
        "sico.text.length" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "text.length result layout"));
            };
            let [pointer, length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(pointer));
            body.instruction(&Instruction::LocalGet(length));
            body.instruction(&Instruction::Call(helper("sico.text.length")?));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.bytes.concat" | "sico.text.concat" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "concat result layout"));
            };
            let [a_pointer, a_length] = pair(0)?;
            let [b_pointer, b_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(a_pointer));
            body.instruction(&Instruction::LocalGet(a_length));
            body.instruction(&Instruction::LocalGet(b_pointer));
            body.instruction(&Instruction::LocalGet(b_length));
            body.instruction(&Instruction::Call(helper(name)?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.bytes.utf8_decode" => {
            // Passthrough: callers guard with `sico.bytes.is_utf8`; the
            // bytes stay the same canonical (ptr, len) pair.
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "utf8_decode result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::LocalSet(*length));
        }
        "sico.bytes.is_utf8" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "is_utf8 result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::Call(helper("sico.bytes.utf8_decode")?));
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.text.leading_spaces" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "leading_spaces result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            // Scan for the first non-space/tab byte; the result is the
            // count as one i64 U64 slot. No push-local priming: the loop
            // counter lives entirely in the scan.
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(4));
            body.instruction(&Instruction::Block(BlockType::Empty));
            body.instruction(&Instruction::Loop(BlockType::Empty));
            body.instruction(&Instruction::LocalGet(4));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::I32GeU);
            body.instruction(&Instruction::BrIf(1));
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(4));
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            body.instruction(&Instruction::LocalSet(5));
            body.instruction(&Instruction::LocalGet(5));
            body.instruction(&Instruction::I32Const(32));
            body.instruction(&Instruction::I32Eq);
            body.instruction(&Instruction::LocalGet(5));
            body.instruction(&Instruction::I32Const(9));
            body.instruction(&Instruction::I32Eq);
            body.instruction(&Instruction::I32Or);
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::BrIf(1));
            body.instruction(&Instruction::LocalGet(4));
            body.instruction(&Instruction::I32Const(1));
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::LocalSet(4));
            body.instruction(&Instruction::Br(0));
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::LocalGet(4));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.text.join" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "join result layout"));
            };
            let [table, count] = pair(0)?;
            let [separator_pointer, separator_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(table));
            body.instruction(&Instruction::LocalGet(count));
            body.instruction(&Instruction::LocalGet(separator_pointer));
            body.instruction(&Instruction::LocalGet(separator_length));
            body.instruction(&Instruction::Call(helper("sico.text.join")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.text.split_lines" | "sico.text.split_words" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "split result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::Call(helper(name)?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.text.trim" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "trim result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::Call(helper("sico.text.trim")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.text.contains" | "sico.text.starts_with" | "sico.text.ends_with" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "text predicate result layout"));
            };
            let [a_pointer, a_length] = pair(0)?;
            let [b_pointer, b_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(a_pointer));
            body.instruction(&Instruction::LocalGet(a_length));
            body.instruction(&Instruction::LocalGet(b_pointer));
            body.instruction(&Instruction::LocalGet(b_length));
            body.instruction(&Instruction::Call(helper(name)?));
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.json.is_valid" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "json.is_valid result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            let Some((scratch_end, scratch_position)) = context.scratch else {
                return Err(unsupported(&function.name, "json scratch layout"));
            };
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::LocalSet(scratch_end));
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(scratch_end));
            body.instruction(&Instruction::Call(helper("sico.json.ws")?));
            body.instruction(&Instruction::LocalSet(scratch_position));
            body.instruction(&Instruction::LocalGet(scratch_position));
            body.instruction(&Instruction::LocalGet(scratch_end));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::Call(helper("sico.json.scan")?));
            body.instruction(&Instruction::LocalTee(scratch_position));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::LocalGet(scratch_position));
            body.instruction(&Instruction::LocalGet(scratch_end));
            body.instruction(&Instruction::Call(helper("sico.json.ws")?));
            body.instruction(&Instruction::LocalGet(scratch_end));
            body.instruction(&Instruction::I32Eq);
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.json.get" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "json.get result layout"));
            };
            let [document_pointer, document_length] = pair(0)?;
            let [key_pointer, key_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(document_pointer));
            body.instruction(&Instruction::LocalGet(document_length));
            body.instruction(&Instruction::LocalGet(key_pointer));
            body.instruction(&Instruction::LocalGet(key_length));
            body.instruction(&Instruction::Call(helper("sico.json.find")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.json.has" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "json.has result layout"));
            };
            let [document_pointer, document_length] = pair(0)?;
            let [key_pointer, key_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(document_pointer));
            body.instruction(&Instruction::LocalGet(document_length));
            body.instruction(&Instruction::LocalGet(key_pointer));
            body.instruction(&Instruction::LocalGet(key_length));
            body.instruction(&Instruction::Call(helper("sico.json.find")?));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::I32Ne);
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.json.quote" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "json.quote result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::Call(helper("sico.json.quote")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.bytes.slice" => {
            let [tag, pointer, length, error_tag] = result else {
                return Err(unsupported(&function.name, "bytes.slice result layout"));
            };
            let [source_pointer, source_length] = pair(0)?;
            let start = scalar(1)?;
            let requested = scalar(2)?;
            let overflow = context
                .variant_tags
                .get(&function.name, "NumericError.overflow")?;
            // Reject inputs that do not fit the 32-bit arena.
            body.instruction(&Instruction::LocalGet(start));
            body.instruction(&Instruction::I64Const(0x1_0000_0000));
            body.instruction(&Instruction::I64GeU);
            body.instruction(&Instruction::LocalGet(requested));
            body.instruction(&Instruction::I64Const(0x1_0000_0000));
            body.instruction(&Instruction::I64GeU);
            body.instruction(&Instruction::I32Or);
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_numeric_error(body, result, overflow);
            body.instruction(&Instruction::Else);
            // end = start + requested (32-bit, wrap-checked)
            body.instruction(&Instruction::LocalGet(start));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::LocalGet(requested));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::LocalTee(*length));
            body.instruction(&Instruction::LocalGet(start));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::I32LtU);
            body.instruction(&Instruction::LocalGet(*length));
            body.instruction(&Instruction::LocalGet(source_length));
            body.instruction(&Instruction::I32GtU);
            body.instruction(&Instruction::I32Or);
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_numeric_error(body, result, overflow);
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::LocalGet(source_pointer));
            body.instruction(&Instruction::LocalGet(start));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::LocalGet(requested));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::End);
            let _ = error_tag;
        }
        "sico.list.length" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "list.length result layout"));
            };
            let [_, length] = pair(0)?;
            body.instruction(&Instruction::LocalGet(length));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.list.get" => {
            let [tag, pointer, length, error_tag] = result else {
                return Err(unsupported(&function.name, "list.get result layout"));
            };
            let [table, count] = pair(0)?;
            let index = scalar(1)?;
            let overflow = context
                .variant_tags
                .get(&function.name, "NumericError.overflow")?;
            // index >= count -> overflow error (count fits u32, so any
            // 64-bit index beyond it is rejected without truncation)
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::LocalGet(count));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::I64GeU);
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_numeric_error(body, result, overflow);
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::LocalGet(table));
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::I32Const(3));
            body.instruction(&Instruction::I32Shl);
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::LocalTee(*pointer));
            body.instruction(&Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalGet(*pointer));
            body.instruction(&Instruction::I32Load(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }));
            body.instruction(&Instruction::LocalSet(*pointer));
            // slots are (ptr, len): first load gave the element ptr into
            // length temporarily; swap into place
            body.instruction(&Instruction::LocalGet(*length));
            body.instruction(&Instruction::LocalGet(*pointer));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::End);
            let _ = error_tag;
        }
        "sico.list.append" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "list.append result layout"));
            };
            let [table, count] = pair(0)?;
            let [element_pointer, element_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(table));
            body.instruction(&Instruction::LocalGet(count));
            body.instruction(&Instruction::LocalGet(element_pointer));
            body.instruction(&Instruction::LocalGet(element_length));
            body.instruction(&Instruction::Call(helper("sico.list.append")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.bytes.at" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "bytes.at result layout"));
            };
            let [source, len] = pair(0)?;
            let index = scalar(1)?;
            let fallback = scalar(2)?;
            // Total function (RFC-0045 D1 amendment): the byte at `index`,
            // or the explicit fallback when out of range. Proven single-slot
            // If/Else shape.
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::LocalGet(len));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::I64LtU);
            body.instruction(&Instruction::If(BlockType::Empty));
            body.instruction(&Instruction::LocalGet(source));
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            body.instruction(&Instruction::I64ExtendI32U);
            body.instruction(&Instruction::LocalSet(*value));
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::LocalGet(fallback));
            body.instruction(&Instruction::LocalSet(*value));
            body.instruction(&Instruction::End);
        }
        "sico.bytes.equal" => {
            let [value] = result else {
                return Err(unsupported(&function.name, "bytes.equal result layout"));
            };
            let [a_pointer, a_length] = pair(0)?;
            let [b_pointer, b_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(a_pointer));
            body.instruction(&Instruction::LocalGet(a_length));
            body.instruction(&Instruction::LocalGet(b_pointer));
            body.instruction(&Instruction::LocalGet(b_length));
            body.instruction(&Instruction::Call(helper("sico.bytes.equal")?));
            body.instruction(&Instruction::LocalSet(*value));
        }
        "sico.text.compare" => {
            let [order] = result else {
                return Err(unsupported(&function.name, "text.compare result layout"));
            };
            let [a_pointer, a_length] = pair(0)?;
            let [b_pointer, b_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(a_pointer));
            body.instruction(&Instruction::LocalGet(a_length));
            body.instruction(&Instruction::LocalGet(b_pointer));
            body.instruction(&Instruction::LocalGet(b_length));
            body.instruction(&Instruction::Call(helper("sico.text.compare")?));
            body.instruction(&Instruction::LocalSet(*order));
        }
        "sico.text.char_at" => {
            let [tag, pointer, length, error_tag] = result else {
                return Err(unsupported(&function.name, "text.char_at result layout"));
            };
            let [source, len] = pair(0)?;
            let index = scalar(1)?;
            let overflow = context
                .variant_tags
                .get(&function.name, "NumericError.overflow")?;
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::I64Const(0x1_0000_0000));
            body.instruction(&Instruction::I64GeU);
            body.instruction(&Instruction::If(BlockType::Empty));
            emit_numeric_error(body, result, overflow);
            body.instruction(&Instruction::Else);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::LocalGet(source));
            body.instruction(&Instruction::LocalGet(len));
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::I32WrapI64);
            body.instruction(&Instruction::Call(helper("sico.text.char_loc")?));
            // helper returns (offset, char_len); char_len == 0 is the
            // out-of-range sentinel. The Text result pointer is the source
            // base plus the char's byte offset.
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::LocalGet(*pointer));
            body.instruction(&Instruction::LocalGet(source));
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::LocalGet(*length));
            body.instruction(&Instruction::I32Eqz);
            body.instruction(&Instruction::If(BlockType::Empty));
            body.instruction(&Instruction::I32Const(1));
            body.instruction(&Instruction::LocalSet(*tag));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalSet(*pointer));
            body.instruction(&Instruction::I32Const(overflow));
            body.instruction(&Instruction::LocalSet(*error_tag));
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::End);
        }
        "sico.text.format" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "text.format result layout"));
            };
            let [template_pointer, template_length] = pair(0)?;
            let [table, count] = pair(1)?;
            body.instruction(&Instruction::LocalGet(template_pointer));
            body.instruction(&Instruction::LocalGet(template_length));
            body.instruction(&Instruction::LocalGet(table));
            body.instruction(&Instruction::LocalGet(count));
            body.instruction(&Instruction::Call(helper("sico.text.format")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.list.sort" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "list.sort result layout"));
            };
            let [table, count] = pair(0)?;
            body.instruction(&Instruction::LocalGet(table));
            body.instruction(&Instruction::LocalGet(count));
            body.instruction(&Instruction::Call(helper("sico.list.sort")?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.list.min" | "sico.list.max" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "list extremum result layout"));
            };
            let [table, count] = pair(0)?;
            let [default_pointer, default_length] = pair(1)?;
            body.instruction(&Instruction::LocalGet(table));
            body.instruction(&Instruction::LocalGet(count));
            body.instruction(&Instruction::LocalGet(default_pointer));
            body.instruction(&Instruction::LocalGet(default_length));
            body.instruction(&Instruction::Call(helper(name)?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        "sico.u64.to_text" | "sico.i64.to_text" => {
            let [pointer, length] = result else {
                return Err(unsupported(&function.name, "to_text result layout"));
            };
            let value = scalar(0)?;
            body.instruction(&Instruction::LocalGet(value));
            body.instruction(&Instruction::Call(helper(name)?));
            body.instruction(&Instruction::LocalSet(*length));
            body.instruction(&Instruction::LocalSet(*pointer));
        }
        _ => return Err(unsupported(&function.name, "stdlib intrinsic")),
    }
    Ok(())
}

/// Lowers one `sico.map.*[K,V]`/`sico.set.*[K]` intrinsic (M14 STEP-0131):
/// push the flat operand slots in the helper's parameter order, call the
/// monomorphized helper, and store its result slots.
fn emit_collection_call(
    context: &CompileContext<'_>,
    body: &mut Function,
    instruction: &sico_ir::Instruction,
    name: &str,
    arguments: &[ValueId],
) -> Result<(), CodegenError> {
    let function = context.function;
    let layout = context.layout;
    let result = layout.get(&function.name, instruction.result)?;
    let flat = |value: ValueId| -> Result<Vec<u32>, CodegenError> {
        Ok(layout.get(&function.name, value)?.to_vec())
    };
    // Pair-typed operands (map/set values and `Text` keys/results) occupy two
    // i32 slots; scalars occupy one i64 slot. The helper signatures in
    // `collection_helper_signature` consume exactly this order.
    for argument in arguments {
        for slot in flat(*argument)? {
            body.instruction(&Instruction::LocalGet(slot));
        }
    }
    let helper = context
        .script
        .and_then(|emit| emit.helpers.get(name))
        .copied()
        .ok_or_else(|| unsupported(&function.name, "collection helper layout"))?;
    body.instruction(&Instruction::Call(helper));
    // `length` is the one operation whose IR result type (a scalar `U64`)
    // is narrower than the helper's `(table, count)` pair: drop the table
    // and widen the count. Everything else stores the pair/tag forms
    // verbatim into the result slots planned by `flat_ir_types`.
    if matches!(
        sico_ir::collection_intrinsic(name),
        Some(sico_ir::CollectionIntrinsic {
            operation: sico_ir::CollectionOperation::MapLength
                | sico_ir::CollectionOperation::SetLength
                | sico_ir::CollectionOperation::ListLength,
            ..
        })
    ) {
        // The helper returned `(table i32, count i32)`; keep only the count
        // and widen it into the scalar U64 result slot. The first scratch
        // local (`return_area`, i32) safely holds the dropped table pointer.
        let [value] = result else {
            return Err(unsupported(
                &function.name,
                "collection length result layout",
            ));
        };
        let Some((scratch, _)) = context.scratch else {
            return Err(unsupported(
                &function.name,
                "collection length scratch layout",
            ));
        };
        body.instruction(&Instruction::LocalSet(scratch));
        body.instruction(&Instruction::LocalGet(scratch));
        body.instruction(&Instruction::I64ExtendI32U);
        body.instruction(&Instruction::LocalSet(*value));
        return Ok(());
    }
    for slot in result.iter().rev() {
        body.instruction(&Instruction::LocalSet(*slot));
    }
    Ok(())
}

/// Writes an error `Result` value: tag 1 and the numeric error discriminant.
fn emit_numeric_error(body: &mut Function, result: &[u32], overflow: i32) {
    let [tag, pointer, length, error_tag] = result else {
        unreachable!("numeric error results have four slots");
    };
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::LocalSet(*tag));
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(*pointer));
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(*length));
    body.instruction(&Instruction::I32Const(overflow));
    body.instruction(&Instruction::LocalSet(*error_tag));
}
