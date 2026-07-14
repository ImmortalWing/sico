#[test]
fn wasi_0_3_resource_async_wit_parses() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit");
    let mut resolve = wit_parser::Resolve::default();
    let (package, sources) = resolve.push_dir(&path).expect("WIT package should parse");
    assert_eq!(resolve.packages[package].name.namespace, "sico");
    assert!(sources.paths().next().is_some());
}
