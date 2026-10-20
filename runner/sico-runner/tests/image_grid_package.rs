//! M24 gate-3 roster 2/3 (RFC-0043 §Summary `image-grid@1`, frozen by Amendment
//! A2 input + Amendment A1 list result, ADR-0019): `sico:user/image-grid@1`
//! `detect-lines` content contracts, asserted on the real Component path
//! against a frozen, independently-generated corpus.
//!
//! The corpus (`tests/corpus/image-grid/manifest.json`, schema
//! `sico.m24.roster-grid-fixtures.v0`) is produced by
//! `prototypes/image-grid-probe/tools/make-fixtures.py` — a pure-Python
//! re-implementation of the A2 decode, the grey8 horizontal-maximal-run
//! detection (foreground `>= 128`, row-major) and the A1 `list<region>`
//! encoding, sharing no code with the Rust guest core. The test reads its
//! expectations from the manifest, so core and oracle agreeing byte-for-byte is
//! the content proof.

use sico_codegen_wasm::package_builder::image_grid_package;
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

const INTERFACE: &str = "sico:user/image-grid@1.0.0";

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).unwrap()
}

type GridOutcome = Result<Vec<u8>, String>;

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

fn call(bmp: &[u8], min_run: u64) -> GridOutcome {
    let package = image_grid_package();
    let engine = engine();
    let component = Component::new(&engine, &package).expect("compiles");
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
        .get_export_index(&mut store, Some(&inner), "detect-lines")
        .expect("export detect-lines present");
    let callee = instance
        .get_func(&mut store, func_index)
        .expect("function resolves");
    let args = [bytes_val(bmp), Val::U64(min_run)];
    let mut results = [Val::Result(Ok(None))];
    callee
        .call(&mut store, &args, &mut results)
        .unwrap_or_else(|error| panic!("call detect-lines traps: {error:#}"));
    match &results[0] {
        Val::Result(Ok(Some(value))) => Ok(result_bytes(value.as_ref())),
        Val::Result(Ok(None)) => panic!("empty ok payload from detect-lines"),
        Val::Result(Err(Some(value))) => match value.as_ref() {
            Val::String(case) => Err(case.clone()),
            other => panic!("unexpected error payload {other:?}"),
        },
        Val::Result(Err(None)) => panic!("empty error payload from detect-lines"),
        other => panic!("unexpected result {other:?}"),
    }
}

fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!("corpus/image-grid/manifest.json")).expect("manifest parses")
}

#[test]
fn image_grid_package_is_deterministic_and_pure() {
    let package = image_grid_package();
    assert_eq!(image_grid_package(), package, "two builds byte-identical");
    let engine = engine();
    let component = Component::new(&engine, &package).expect("compile");
    let ty = component.component_type();
    let mut exports = 0;
    for (name, item) in ty.exports(&engine) {
        assert!(name.starts_with("sico:user/image-grid@"), "{name}");
        assert!(matches!(
            item.ty,
            wasmtime::component::types::ComponentItem::ComponentInstance(_)
        ));
        exports += 1;
    }
    assert_eq!(exports, 1, "exactly one interface instance exported");
}

#[test]
fn detect_lines_corpus_matches_independent_oracle() {
    let m = manifest();
    let cases = m["cases"].as_array().expect("cases array");
    assert_eq!(cases.len(), 13, "frozen 13-case detect-lines corpus");
    let mut ok_seen = 0;
    let mut err_seen = 0;
    let mut empty_ok = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let bmp = unhex(case["bmp_hex"].as_str().unwrap());
        let min_run = case["min_run"].as_u64().unwrap();
        let expect = &case["expect"];
        match expect["kind"].as_str().unwrap() {
            "ok" => {
                let want = expect["list_hex"].as_str().unwrap();
                let got = call(&bmp, min_run).unwrap_or_else(|c| panic!("{name}: refused {c}"));
                assert!(
                    got.len() >= 4 && (got.len() - 4).is_multiple_of(16),
                    "{name}: count(4B) + 16B-per-region framing"
                );
                assert_eq!(hex(&got), want, "{name}: byte-exact list blob");
                if got == [0u8; 4] {
                    empty_ok += 1;
                }
                ok_seen += 1;
            }
            "err" => {
                let want = expect["case"].as_str().unwrap();
                let got = call(&bmp, min_run)
                    .err()
                    .unwrap_or_else(|| panic!("{name}: expected refusal {want}, got ok"));
                assert_eq!(got, want, "{name}: typed refusal");
                err_seen += 1;
            }
            other => panic!("{name}: unknown expect kind {other}"),
        }
    }
    assert_eq!((ok_seen, err_seen), (5, 8), "5 ok + 8 typed refusals");
    assert_eq!(empty_ok, 1, "exactly one count-0 ok (empty result)");
}

#[test]
fn refusal_corpus_covers_the_reachable_image_error_set() {
    // detect-lines reaches four of the five RFC-0041 `image-error` cases;
    // `region-out-of-bounds` is inapplicable (regions derive from within-image
    // runs and can never exceed the raster).
    let m = manifest();
    let cases = m["cases"].as_array().unwrap();
    let observed: Vec<String> = cases
        .iter()
        .filter_map(|case| case["expect"]["case"].as_str().map(str::to_owned))
        .collect();
    for want in [
        "unsupported-format",
        "stride-mismatch",
        "dimension-overflow",
        "allocation-limit",
    ] {
        assert!(
            observed.iter().any(|seen| seen == want),
            "refusal corpus missing {want}"
        );
    }
    assert!(
        !observed.iter().any(|seen| seen == "region-out-of-bounds"),
        "region-out-of-bounds is not reachable via detect-lines"
    );
}

#[test]
fn detect_lines_is_byte_stable_and_repeats_across_stores() {
    let m = manifest();
    let cases = m["cases"].as_array().unwrap();
    let ok = cases
        .iter()
        .find(|case| case["expect"]["kind"] == "ok" && case["expect"]["list_hex"] != "00000000")
        .expect("a non-empty ok case");
    let bmp = unhex(ok["bmp_hex"].as_str().unwrap());
    let min_run = ok["min_run"].as_u64().unwrap();
    let first = call(&bmp, min_run).expect("ok");
    let second = call(&bmp, min_run).expect("ok on a fresh store");
    assert_eq!(first, second, "detect-lines is byte-stable across runs");
}
