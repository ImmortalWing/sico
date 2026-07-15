#[test]
fn wasi_0_3_resource_async_wit_parses() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wit/async-flow-v0");
    let mut resolve = wit_parser::Resolve::default();
    let (package, sources) = resolve.push_dir(&path).expect("WIT package should parse");
    assert_eq!(resolve.packages[package].name.namespace, "sico");
    assert_eq!(resolve.packages[package].name.name, "async-flow-probe");
    assert!(sources.paths().next().is_some());
}
