//! RFC-0039 §2.4 (STEP-0147): user-WIT package Component builder contract,
//! validated against the real wasmtime component model. The package
//! Components must validate, instantiate, export the frozen
//! `sico:user/<interface>@<version>` instance and answer the frozen
//! semantics with exact payload content (not just result counts).

use sico_codegen_wasm::package_builder::{csv_package, table_stats_package};
use wasmtime::component::{Component, Func, Instance, Linker, Val};
use wasmtime::{Engine, Store};

/// One instantiated package Component with its owning Store.
struct PackageInstance {
    store: Store<()>,
    instance: Instance,
}

fn instantiate(bytes: &[u8]) -> PackageInstance {
    let engine = Engine::default();
    let component = Component::from_binary(&engine, bytes).expect("package component validates");
    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("package component links with no imports");
    PackageInstance { store, instance }
}

/// Resolves one function of the exported interface instance and checks its
/// frozen signature shape.
fn interface_func(pkg: &mut PackageInstance, interface: &str, function: &str) -> Func {
    let nested = pkg
        .instance
        .get_export_index(&mut pkg.store, None, interface)
        .unwrap_or_else(|| panic!("missing instance export {interface}"));
    let index = pkg
        .instance
        .get_export_index(&mut pkg.store, Some(&nested), function)
        .unwrap_or_else(|| panic!("missing function export {function}"));
    let func = pkg
        .instance
        .get_func(&mut pkg.store, index)
        .expect("resolved function");
    let ty = func.ty(&pkg.store);
    let name = format!("{interface}.{function}");
    for (param, _) in ty.params() {
        assert!(param.starts_with('p'), "{name}: unexpected param {param}");
    }
    assert_eq!(
        ty.results().len(),
        1,
        "{name}: v0 functions return exactly one value"
    );
    func
}

/// Calls an untyped function and returns its single result value.
fn call1(pkg: &mut PackageInstance, func: &Func, params: &[Val]) -> Val {
    let results = func.ty(&pkg.store).results().len();
    let mut out = vec![Val::Bool(false); results];
    func.call(&mut pkg.store, params, &mut out)
        .expect("package function call succeeds");
    assert_eq!(out.len(), 1);
    out.pop().expect("one result")
}

/// Reads a `result<list<string>, string>` payload into a Rust-friendly form.
fn expect_result_list_string(value: &Val) -> Result<Vec<String>, String> {
    let wasmtime::component::Val::Result(payload) = value else {
        panic!("expected a result value, got {value:?}");
    };
    match payload {
        Ok(Some(inner)) => match inner.as_ref() {
            wasmtime::component::Val::List(items) => Ok(items
                .iter()
                .map(|item| match item {
                    wasmtime::component::Val::String(text) => text.clone(),
                    other => panic!("expected string element, got {other:?}"),
                })
                .collect()),
            other => panic!("expected list payload, got {other:?}"),
        },
        Ok(None) => panic!("expected a present ok payload"),
        Err(Some(inner)) => match inner.as_ref() {
            wasmtime::component::Val::String(text) => Err(text.clone()),
            other => panic!("expected string error, got {other:?}"),
        },
        Err(None) => panic!("expected a present error payload"),
    }
}

fn text(value: &str) -> Val {
    Val::String(value.to_owned())
}

fn list_str(values: &[&str]) -> Val {
    Val::List(
        values
            .iter()
            .map(|value| Val::String((*value).to_owned()))
            .collect(),
    )
}

#[test]
fn csv_parse_line_content_contract() {
    let mut pkg = instantiate(&csv_package());
    let parse = interface_func(&mut pkg, "sico:user/csv@1.0.0", "parse-line");

    // Quoted field keeps its comma; plain and empty fields round-trip.
    let line = "\"a,b\",plain,\"\"";
    let value = call1(&mut pkg, &parse, &[text(line)]);
    assert_eq!(
        expect_result_list_string(&value),
        Ok(vec!["a,b".into(), "plain".into(), String::new()])
    );

    // `""` inside quotes is one literal quote character.
    let line = "\"say \"\"hi\"\"\",x";
    let value = call1(&mut pkg, &parse, &[text(line)]);
    assert_eq!(
        expect_result_list_string(&value),
        Ok(vec!["say \"hi\"".into(), "x".into()])
    );

    // Fail-closed: an unterminated quote is a typed error.
    let value = call1(&mut pkg, &parse, &[text("\"abc")]);
    assert_eq!(
        expect_result_list_string(&value),
        Err("unterminated-quote".into())
    );

    // Fail-closed: a quote inside a plain field is a typed error.
    let value = call1(&mut pkg, &parse, &[text("a\"b")]);
    assert_eq!(expect_result_list_string(&value), Err("stray-quote".into()));

    // A trailing CRLF is stripped, not embedded in the last field.
    let value = call1(&mut pkg, &parse, &[text("r\r\n")]);
    assert_eq!(expect_result_list_string(&value), Ok(vec!["r".into()]));

    // Zero-byte input is an empty list, not a one-empty-field list.
    let value = call1(&mut pkg, &parse, &[text("")]);
    assert_eq!(expect_result_list_string(&value), Ok(Vec::new()));
}

#[test]
fn table_stats_aggregate_content_contract() {
    let mut pkg = instantiate(&table_stats_package());
    let aggregate = interface_func(&mut pkg, "sico:user/table-stats@1.0.0", "aggregate");

    let call = |pkg: &mut PackageInstance, values: &[&str], op: &str| {
        let value = call1(pkg, &aggregate, &[list_str(values), text(op)]);
        expect_result_list_string(&value)
    };

    assert_eq!(call(&mut pkg, &["3", "7"], "mean"), Ok(vec!["5000".into()]));
    assert_eq!(
        call(&mut pkg, &["1", "2", "3"], "max"),
        Ok(vec!["3".into()])
    );
    assert_eq!(call(&mut pkg, &[], "min"), Err("empty-column".into()));
    assert_eq!(
        call(&mut pkg, &["9223372036854775807", "1"], "mean"),
        Err("overflow".into())
    );
    assert_eq!(call(&mut pkg, &["5", "5"], "count"), Ok(vec!["2".into()]));
    assert_eq!(call(&mut pkg, &[], "count"), Ok(vec!["0".into()]));
    assert_eq!(call(&mut pkg, &["1"], "sum-x"), Err("unknown-op".into()));

    // Decimal parsing inside the package, aggregate values as decimal text
    // (including the negative mean floor).
    assert_eq!(
        call(&mut pkg, &["4", "10", "-2"], "count"),
        Ok(vec!["3".into()])
    );
    assert_eq!(
        call(&mut pkg, &["4", "10", "-2"], "mean"),
        Ok(vec!["4000".into()])
    );
    assert_eq!(
        call(&mut pkg, &["4", "10", "-2"], "min"),
        Ok(vec!["-2".into()])
    );
    assert_eq!(
        call(&mut pkg, &["6", "-7"], "mean"),
        Ok(vec!["-500".into()])
    );
    assert_eq!(call(&mut pkg, &["-3"], "mean"), Ok(vec!["-3000".into()]));
    assert_eq!(call(&mut pkg, &["10", "-2"], "min"), Ok(vec!["-2".into()]));

    // First malformed element wins, with its zero-based index in the text.
    assert_eq!(
        call(&mut pkg, &["12abc"], "min"),
        Err("malformed-number@0".into())
    );
    assert_eq!(
        call(&mut pkg, &["5", "x", "6"], "mean"),
        Err("malformed-number@1".into())
    );
    assert_eq!(
        call(&mut pkg, &[""], "max"),
        Err("malformed-number@0".into())
    );
    assert_eq!(
        call(&mut pkg, &["+5"], "min"),
        Err("malformed-number@0".into())
    );
    assert_eq!(
        call(&mut pkg, &["9223372036854775808"], "min"),
        Err("malformed-number@0".into())
    );
    assert_eq!(
        call(&mut pkg, &["-9223372036854775809"], "min"),
        Err("malformed-number@0".into())
    );

    // count (and unknown-op) short-circuit before parsing: non-numeric
    // elements still count and never produce malformed-number.
    assert_eq!(call(&mut pkg, &["", "5"], "count"), Ok(vec!["2".into()]));
    assert_eq!(call(&mut pkg, &["abc"], "count"), Ok(vec!["1".into()]));
    assert_eq!(call(&mut pkg, &["abc"], "sum-x"), Err("unknown-op".into()));
}

#[test]
fn package_component_exports_the_frozen_instance_names() {
    // Structural check through the component binary: the only component-level
    // export of each package is the frozen instance name of kind instance.
    for (bytes, expected) in [
        (csv_package(), "sico:user/csv@1.0.0"),
        (table_stats_package(), "sico:user/table-stats@1.0.0"),
    ] {
        let mut found = Vec::new();
        for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
            let wasmparser::Payload::ComponentExportSection(exports) =
                payload.expect("component parses")
            else {
                continue;
            };
            for export in exports {
                let export = export.expect("export entry parses");
                found.push((export.name.name.to_owned(), export.kind));
            }
        }
        assert_eq!(
            found,
            vec![(
                expected.to_owned(),
                wasmparser::ComponentExternalKind::Instance
            )],
            "{expected}: unexpected component export surface"
        );
    }
}

#[test]
fn package_builds_are_deterministic() {
    assert_eq!(csv_package(), csv_package());
    assert_eq!(table_stats_package(), table_stats_package());
    assert_ne!(csv_package(), table_stats_package());
}
