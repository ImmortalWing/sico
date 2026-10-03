//! M24 gate-3 roster (RFC-0043 §Summary, frozen by Amendment A1 result +
//! Amendment A2 input, ADR-0019): `sico:user/image-match@1` SAD template-match
//! content contracts, asserted on the real Component path against a frozen,
//! independently-generated corpus.
//!
//! The corpus (`tests/corpus/image-match/manifest.json`, schema
//! `sico.m24.roster-fixtures.v0`) is produced by `prototypes/image-match-probe/
//! tools/make-fixtures.py` — a pure-Python re-implementation of the A2 decode,
//! the deterministic row-major integer SAD scan (ties by lowest `(y,x)`), and
//! the A1 region encoding that shares no code with the Rust guest core. The
//! test reads its expectations from the manifest, so core and oracle agreeing
//! byte-for-byte is the content proof (element contents, not counts).

use sico_codegen_wasm::package_builder::image_match_package;
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

const INTERFACE: &str = "sico:user/image-match@1.0.0";

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).unwrap()
}

/// Outcome of one `template` call: `Ok(region-bytes)` or `Err(case-name)`.
type MatchOutcome = Result<Vec<u8>, String>;

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn bytes_val(bytes: &[u8]) -> Val {
    Val::List(bytes.iter().map(|b| Val::U8(*b)).collect())
}

fn result_bytes(value: &Val) -> Vec<u8> {
    match value {
        Val::List(items) => items
            .iter()
            .map(|item| match item {
                Val::U8(b) => *b,
                other => panic!("unexpected list element {other:?}"),
            })
            .collect(),
        other => panic!("unexpected ok payload {other:?}"),
    }
}

/// Instantiates the package and calls `template(bmp, tpl)`, decoding the
/// Canonical ABI `Result[Bytes, Text]` return area.
fn call(bmp: &[u8], tpl: &[u8]) -> MatchOutcome {
    let package = image_match_package();
    let engine = engine();
    let component = Component::new(&engine, &package)
        .map_err(|error| panic!("component loads: {error:#}"))
        .expect("compiles");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    // The core is import-free (ADR-0019): an empty linker satisfies purity.
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiates against an empty linker");
    let inner = instance
        .get_export_index(&mut store, None, INTERFACE)
        .expect("interface instance exported");
    let func_index = instance
        .get_export_index(&mut store, Some(&inner), "template")
        .expect("export template present");
    let callee = instance
        .get_func(&mut store, func_index)
        .expect("function resolves");
    let args = [bytes_val(bmp), bytes_val(tpl)];
    let mut results = [Val::Result(Ok(None))];
    callee
        .call(&mut store, &args, &mut results)
        .unwrap_or_else(|error| panic!("call template traps: {error:#}"));
    match &results[0] {
        Val::Result(Ok(Some(value))) => Ok(result_bytes(value.as_ref())),
        Val::Result(Ok(None)) => panic!("empty ok payload from template"),
        Val::Result(Err(Some(value))) => match value.as_ref() {
            Val::String(case) => Err(case.clone()),
            other => panic!("unexpected error payload {other:?}"),
        },
        Val::Result(Err(None)) => panic!("empty error payload from template"),
        other => panic!("unexpected result {other:?}"),
    }
}

fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!("corpus/image-match/manifest.json")).expect("manifest parses")
}

#[test]
fn image_match_package_is_deterministic_and_pure() {
    let package = image_match_package();
    assert_eq!(image_match_package(), package, "two builds byte-identical");
    let engine = engine();
    let component = Component::new(&engine, &package).expect("compile");
    let ty = component.component_type();
    let mut exports = 0;
    for (name, item) in ty.exports(&engine) {
        assert!(name.starts_with("sico:user/image-match@"), "{name}");
        assert!(matches!(
            item.ty,
            wasmtime::component::types::ComponentItem::ComponentInstance(_)
        ));
        exports += 1;
    }
    assert_eq!(exports, 1, "exactly one interface instance exported");
}

#[test]
fn template_corpus_matches_independent_oracle() {
    let m = manifest();
    let cases = m["cases"].as_array().expect("cases array");
    assert_eq!(cases.len(), 14, "frozen 14-case template corpus");
    let mut ok_seen = 0;
    let mut err_seen = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let bmp = unhex(case["bmp_hex"].as_str().unwrap());
        let tpl = unhex(case["tpl_hex"].as_str().unwrap());
        let expect = &case["expect"];
        match expect["kind"].as_str().unwrap() {
            "ok" => {
                let want = expect["region_hex"].as_str().unwrap();
                let got = call(&bmp, &tpl).unwrap_or_else(|c| panic!("{name}: refused with {c}"));
                assert_eq!(got.len(), 16, "{name}: region is 16 bytes");
                assert_eq!(hex(&got), want, "{name}: byte-exact region");
                ok_seen += 1;
            }
            "err" => {
                let want = expect["case"].as_str().unwrap();
                let got = call(&bmp, &tpl)
                    .err()
                    .unwrap_or_else(|| panic!("{name}: expected refusal {want}, got ok"));
                assert_eq!(got, want, "{name}: typed refusal");
                err_seen += 1;
            }
            other => panic!("{name}: unknown expect kind {other}"),
        }
    }
    assert_eq!((ok_seen, err_seen), (5, 9), "5 ok + 9 typed refusals");
}

#[test]
fn refusal_corpus_covers_the_image_error_closed_set() {
    // RFC-0041 `image-error` cases must each be reachable through `template`.
    let required = [
        "unsupported-format",
        "stride-mismatch",
        "dimension-overflow",
        "allocation-limit",
        "region-out-of-bounds",
    ];
    let m = manifest();
    let cases = m["cases"].as_array().unwrap();
    let observed: Vec<String> = cases
        .iter()
        .filter_map(|case| case["expect"]["case"].as_str().map(str::to_owned))
        .collect();
    for want in required {
        assert!(
            observed.iter().any(|seen| seen == want),
            "refusal corpus missing {want}"
        );
    }
}

#[test]
fn template_is_byte_stable_and_repeats_across_stores() {
    // Determinism gate: same inputs -> identical bytes on fresh instantiation.
    let m = manifest();
    let cases = m["cases"].as_array().unwrap();
    let ok = cases
        .iter()
        .find(|case| case["expect"]["kind"] == "ok")
        .expect("at least one ok case");
    let bmp = unhex(ok["bmp_hex"].as_str().unwrap());
    let tpl = unhex(ok["tpl_hex"].as_str().unwrap());
    let first = call(&bmp, &tpl).expect("ok match");
    let second = call(&bmp, &tpl).expect("ok match on a fresh store");
    assert_eq!(first, second, "template is byte-stable across runs");
}
