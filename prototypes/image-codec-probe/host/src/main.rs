//! M24 codec feasibility probe — host side (measurement only).
//!
//! Wraps a Rust-compiled `wasm32-unknown-unknown` core module with the
//! repository's own user-WIT package Component builder and exercises it
//! through wasmtime's dynamic component API, exactly the way
//! `runner/sico-runner/tests/vision_package.rs` exercises `image-vision@1`.
//! Nothing here claims a capability: the output is a probe report that
//! decides whether the RFC-0050 implementation home can stay a pure
//! package (zero compiler/Runtime change) or must move to a host provider.
//!
//! Usage: `codec-probe <core.wasm> <fixtures-dir> <report.json>`

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sico_codegen_wasm::package_builder::{build_package_component, PackageFunctionSpec};
use sico_ir::Type;
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

const INTERFACE: &str = "image-codec-probe";
const IDENTITY: &str = "sico:user/image-codec-probe@1.0.0";
/// Declared pixel bound from RFC-0050 §3, used only to build limit+1 cases.
const MAX_PIXELS: u64 = 1 << 28;

#[derive(Deserialize)]
struct Manifest {
    decode_cases: Vec<DecodeCase>,
}

#[derive(Deserialize)]
struct DecodeCase {
    name: String,
    file: String,
    expect: DecodeExpectation,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum DecodeExpectation {
    #[serde(rename = "ok")]
    Ok { payload_hex: String },
    #[serde(rename = "err")]
    Err { case: String },
}

#[derive(Serialize)]
struct CaseResult {
    name: String,
    expect: String,
    got: String,
    pass: bool,
}

#[derive(Serialize)]
struct EncodeResult {
    chunk_kinds: Vec<String>,
    chunk_set_frozen: bool,
    byte_stable: bool,
    sha256: String,
    roundtrip_payload_match: bool,
    refusals: Vec<CaseResult>,
}

#[derive(Serialize)]
struct Report {
    schema: String,
    core_module: DigestRecord,
    component: ComponentRecord,
    decode_cases: Vec<CaseResult>,
    encode: EncodeResult,
    verdict: String,
}

#[derive(Serialize)]
struct DigestRecord {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
struct ComponentRecord {
    identity: String,
    bytes: u64,
    sha256: String,
    rebuild_byte_identical: bool,
    exported_instances: Vec<String>,
    exported_functions: Vec<String>,
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn unhex(text: &str) -> Vec<u8> {
    let bytes: Vec<u8> = (0..text.len())
        .step_by(2)
        .filter_map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect();
    bytes
}

fn specs() -> Vec<PackageFunctionSpec> {
    vec![
        PackageFunctionSpec {
            name: "decode-png".into(),
            parameters: vec![Type::Bytes],
            result: Type::Result {
                ok: Box::new(Type::Bytes),
                error: Box::new(Type::String),
            },
        },
        PackageFunctionSpec {
            name: "encode-png".into(),
            parameters: vec![Type::Bytes, Type::U64, Type::U64],
            result: Type::Result {
                ok: Box::new(Type::Bytes),
                error: Box::new(Type::String),
            },
        },
    ]
}

fn engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Ok(Engine::new(&config)?)
}

/// One dynamic call against the wrapped package Component.
fn call(
    component: &[u8],
    function: &str,
    args: &[Val],
) -> Result<Result<Vec<u8>, String>, String> {
    let engine = engine().map_err(|error| format!("engine: {error:#}"))?;
    let compiled =
        Component::new(&engine, component).map_err(|error| format!("component: {error:#}"))?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &compiled)
        .map_err(|error| format!("instantiate: {error:#}"))?;
    let inner = instance
        .get_export_index(&mut store, None, IDENTITY)
        .ok_or_else(|| format!("missing instance export {IDENTITY}"))?;
    let index = instance
        .get_export_index(&mut store, Some(&inner), function)
        .ok_or_else(|| format!("missing export {function}"))?;
    let func = instance
        .get_func(&mut store, index)
        .ok_or_else(|| format!("unresolved function {function}"))?;
    let mut results = [Val::Result(Ok(None))];
    func.call(&mut store, args, &mut results)
        .map_err(|error| format!("trap: {error:#}"))?;
    Ok(match &results[0] {
        Val::Result(Ok(Some(value))) => match value.as_ref() {
            Val::List(items) => Ok(items
                .iter()
                .map(|item| match item {
                    Val::U8(byte) => *byte,
                    other => panic!("unexpected payload element {other:?}"),
                })
                .collect()),
            other => Err(format!("unexpected ok payload {other:?}")),
        },
        Val::Result(Ok(None)) => Err("empty ok payload".to_owned()),
        Val::Result(Err(Some(value))) => match value.as_ref() {
            Val::String(case) => Err(case.clone()),
            other => Err(format!("unexpected error payload {other:?}")),
        },
        Val::Result(Err(None)) => Err("empty error payload".to_owned()),
        other => Err(format!("unexpected result {other:?}")),
    })
}

fn bytes_val(bytes: &[u8]) -> Val {
    Val::List(bytes.iter().map(|byte| Val::U8(*byte)).collect())
}

/// PNG chunk kinds in stream order (independent of the `png` crate).
fn chunk_kinds(bytes: &[u8]) -> Vec<String> {
    let mut kinds = Vec::new();
    let mut offset = 8usize;
    while offset + 12 <= bytes.len() {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        kinds.push(String::from_utf8_lossy(&bytes[offset + 4..offset + 8]).into_owned());
        offset += 12 + length;
    }
    kinds
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: codec-probe <core.wasm> <fixtures-dir> <report.json>".into());
    }
    let core_path = Path::new(&args[1]);
    let fixtures = Path::new(&args[2]);
    let report_path = Path::new(&args[3]);

    let core = fs::read(core_path).map_err(|error| format!("read core: {error}"))?;
    let specs = specs();
    let component = build_package_component(INTERFACE, 1, &specs, &core);
    let rebuild = build_package_component(INTERFACE, 1, &specs, &core);

    let engine = engine().map_err(|error| format!("engine: {error:#}"))?;
    let compiled =
        Component::new(&engine, &component).map_err(|error| format!("component: {error:#}"))?;
    let ty = compiled.component_type();
    let mut instances = Vec::new();
    let mut functions = Vec::new();
    for (name, item) in ty.exports(&engine) {
        instances.push(name.to_owned());
        if let wasmtime::component::types::ComponentItem::ComponentInstance(instance) = item.ty {
            for (function_name, _) in instance.exports(&engine) {
                functions.push(function_name.to_owned());
            }
        }
    }

    let manifest_text =
        fs::read_to_string(fixtures.join("manifest.json")).map_err(|e| format!("manifest: {e}"))?;
    let manifest: Manifest =
        serde_json::from_str(&manifest_text).map_err(|e| format!("manifest parse: {e}"))?;

    let mut decode_results = Vec::new();
    let mut decoded_ok_payload: Option<Vec<u8>> = None;
    for case in &manifest.decode_cases {
        let input =
            fs::read(fixtures.join(&case.file)).map_err(|e| format!("fixture {}: {e}", case.file))?;
        let outcome = call(&component, "decode-png", &[bytes_val(&input)]);
        let (expect, got, pass) = match (&case.expect, outcome) {
            (DecodeExpectation::Ok { payload_hex }, Ok(Ok(payload))) => {
                let expected = unhex(payload_hex);
                let pass = payload == expected;
                if pass {
                    decoded_ok_payload = Some(payload.clone());
                }
                (
                    format!("ok {} bytes", expected.len()),
                    format!("ok {} bytes, match={pass}", payload.len()),
                    pass,
                )
            }
            (DecodeExpectation::Err { case: name }, Ok(Err(got))) => (
                format!("err {name}"),
                format!("err {got}"),
                got == *name,
            ),
            (DecodeExpectation::Ok { .. }, Ok(Err(got))) => {
                ("ok".to_owned(), format!("err {got}"), false)
            }
            (DecodeExpectation::Err { case: name }, Ok(Ok(payload))) => (
                format!("err {name}"),
                format!("ok {} bytes", payload.len()),
                false,
            ),
            (expect, Err(failure)) => {
                let expected = match expect {
                    DecodeExpectation::Ok { .. } => "ok",
                    DecodeExpectation::Err { .. } => "err",
                };
                (expected.to_owned(), failure, false)
            }
        };
        decode_results.push(CaseResult {
            name: case.name.clone(),
            expect,
            got,
            pass,
        });
    }

    // Encode side: frozen chunk set, byte stability, round trip, refusals.
    let payload = decoded_ok_payload.ok_or("no decoded ok payload to re-encode")?;
    let width = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let height = u64::from_le_bytes(payload[8..16].try_into().unwrap_or([0; 8]));
    let pixels = &payload[16..];
    let encode_args = [bytes_val(pixels), Val::U64(width), Val::U64(height)];
    let first = call(&component, "encode-png", &encode_args)?;
    let second = call(&component, "encode-png", &encode_args)?;
    let encoded = match first {
        Ok(bytes) => bytes,
        Err(case) => return Err(format!("encode-png refused a valid image: {case}")),
    };
    let byte_stable = second.as_ref().map(|b| b == &encoded).unwrap_or(false);
    let kinds = chunk_kinds(&encoded);
    let frozen = kinds
        .iter()
        .all(|kind| matches!(kind.as_str(), "IHDR" | "IDAT" | "IEND"))
        && kinds.first().map(String::as_str) == Some("IHDR")
        && kinds.last().map(String::as_str) == Some("IEND");

    // Round trip: decode(encode(pixels)) must reproduce the same payload.
    let roundtrip = call(&component, "decode-png", &[bytes_val(&encoded)])?;
    let roundtrip_match = matches!(&roundtrip, Ok(decoded) if decoded == &payload);

    let mut refusals = Vec::new();
    for (name, args, expected) in [
        (
            "encode-zero-width",
            vec![bytes_val(&[]), Val::U64(0), Val::U64(2)],
            "oversized",
        ),
        (
            "encode-limit-plus-one",
            vec![
                bytes_val(&[0u8; 4]),
                Val::U64(MAX_PIXELS + 1),
                Val::U64(1),
            ],
            "oversized",
        ),
        (
            "encode-buffer-mismatch",
            vec![bytes_val(&[0u8; 3]), Val::U64(2), Val::U64(2)],
            "corrupt-stream",
        ),
    ] {
        let outcome = call(&component, "encode-png", &args);
        let got = match outcome {
            Ok(Err(case)) => format!("err {case}"),
            Ok(Ok(bytes)) => format!("ok {} bytes", bytes.len()),
            Err(failure) => failure,
        };
        let expect = format!("err {expected}");
        let pass = got == expect;
        refusals.push(CaseResult {
            name: name.to_owned(),
            expect,
            got,
            pass,
        });
    }

    let decode_pass = decode_results.iter().all(|case| case.pass);
    let refusal_pass = refusals.iter().all(|case| case.pass);
    let verdict = if decode_pass
        && refusal_pass
        && byte_stable
        && frozen
        && roundtrip_match
        && component == rebuild
    {
        "package-route-feasible"
    } else {
        "package-route-not-demonstrated"
    };

    let report = Report {
        schema: "sico.m24.codec-probe.v0".into(),
        core_module: DigestRecord {
            path: core_path.display().to_string(),
            bytes: core.len() as u64,
            sha256: sha256_hex(&core),
        },
        component: ComponentRecord {
            identity: IDENTITY.into(),
            bytes: component.len() as u64,
            sha256: sha256_hex(&component),
            rebuild_byte_identical: component == rebuild,
            exported_instances: instances,
            exported_functions: functions,
        },
        decode_cases: decode_results,
        encode: EncodeResult {
            chunk_kinds: kinds,
            chunk_set_frozen: frozen,
            byte_stable,
            sha256: sha256_hex(&encoded),
            roundtrip_payload_match: roundtrip_match,
            refusals,
        },
        verdict: verdict.into(),
    };
    let text = serde_json::to_string_pretty(&report).map_err(|e| format!("report: {e}"))?;
    fs::write(report_path, text.clone()).map_err(|e| format!("write report: {e}"))?;
    println!("{text}");
    Ok(())
}
