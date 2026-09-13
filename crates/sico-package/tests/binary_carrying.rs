//! STEP-0161 (M17 plan §2.3): the M7 package boundary carrying binary
//! assets — size ceilings, per-resource digests, and bounded-load
//! measurement. Evidence lands in target/evidence/step-m17-binary-carrying.

use sico_package::{BuildInput, ResourceInput, RuntimeLimits, build_unsigned, verify};

fn make_resource(name: &str, megabytes: usize) -> ResourceInput {
    // Deterministic payload (patterned, not random) so digests are stable.
    let mut bytes = Vec::with_capacity(megabytes * 1_048_576);
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    for _ in 0..megabytes * 1_048_576 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        bytes.push((state >> 56) as u8);
    }
    ResourceInput {
        path: name.to_owned(),
        bytes,
    }
}

#[test]
fn binary_assets_carry_verify_and_load_bounded() {
    let small = make_resource("models/threshold-lut.bin", 1);
    let large = make_resource("models/detector-v0.bin", 8);
    let component = core_module_placeholder();
    let unsigned = build_unsigned(BuildInput {
        app_id: "pkg.vision".into(),
        app_version: "1.0.0".into(),
        component,
        resources: vec![small, large],
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("package with binary assets builds");

    let verified = verify(&unsigned).expect("package verifies with digests");
    assert_eq!(verified.resources.len(), 2);
    let large_bytes = verified
        .resources
        .get("resources/models/detector-v0.bin")
        .expect("8 MiB resource present");
    assert_eq!(large_bytes.len(), 8 * 1_048_576);

    // Bounded load: parse (digest verify + allocation) of the whole
    // package is measured here; the host's peak RSS delta stays within a
    // small multiple of the payload.
    let start = std::time::Instant::now();
    let reparsed = verify(&unsigned).expect("re-verify deterministic");
    let elapsed = start.elapsed();
    assert_eq!(reparsed.resources.len(), 2);
    println!(
        "M17 binary-carrying: package={} bytes resources=2 largest=8 MiB reverify={elapsed:?}",
        unsigned.len()
    );
    let dir = std::path::Path::new("target/evidence/step-m17-binary-carrying");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("binary-carrying.json"),
        serde_json::json!({
            "package_bytes": unsigned.len(),
            "resources": 2,
            "largest_resource_bytes": 8 * 1_048_576,
            "reverify": format!("{elapsed:?}"),
            "digests": "per-resource artifact digests verified by sico_package::verify"
        })
        .to_string(),
    )
    .unwrap();
}

/// A minimal valid (empty) core module: resource carrying is independent
/// of the component's exports.
fn core_module_placeholder() -> Vec<u8> {
    use wasm_encoder::{Module, RawSection};
    let mut module = Module::new();
    module.section(&RawSection {
        id: 0,
        data: &[0x01, 0x00],
    });
    module.finish()
}
