use std::time::Instant;

use serde_json::json;
use sico_package::{
    BuildInput, ResourceInput, RuntimeLimits, TrustPolicy, build_unsigned, sign_development,
    verify, verify_trusted,
};
use wasm_encoder::Component;

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .map_or(Ok(1_000_usize), |value| value.parse())
        .expect("iterations must be an integer");
    let input = || BuildInput {
        app_id: "dev.sico.performance".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: Component::new().finish(),
        resources: vec![ResourceInput {
            path: "payload.bin".to_owned(),
            bytes: vec![0x5a; 1_024],
        }],
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    };
    let unsigned = build_unsigned(input()).unwrap();
    let signed = sign_development(&unsigned, &[11_u8; 32]).unwrap();

    let build_start = Instant::now();
    let mut built_bytes = 0_usize;
    for _ in 0..iterations {
        built_bytes += build_unsigned(input()).unwrap().len();
    }
    let build = build_start.elapsed();

    let verify_start = Instant::now();
    let mut verified_bytes = 0_usize;
    for _ in 0..iterations {
        verified_bytes += verify(&unsigned).unwrap().component.len();
    }
    let verify = verify_start.elapsed();

    let signature_start = Instant::now();
    let mut trusted_bytes = 0_usize;
    for _ in 0..iterations {
        trusted_bytes += verify_trusted(&signed, &TrustPolicy::AllowUnsignedDevelopment)
            .unwrap()
            .package
            .component
            .len();
    }
    let signature = signature_start.elapsed();

    println!(
        "{}",
        serde_json::to_string(&json!({
            "iterations": iterations,
            "unsigned_package_bytes": unsigned.len(),
            "signed_package_bytes": signed.len(),
            "built_bytes": built_bytes,
            "verified_component_bytes": verified_bytes,
            "trusted_component_bytes": trusted_bytes,
            "build_ms": milliseconds(build),
            "verify_ms": milliseconds(verify),
            "signature_verify_ms": milliseconds(signature),
            "total_operations": iterations * 3
        }))
        .expect("measurement JSON serialization cannot fail")
    );
}

fn milliseconds(duration: std::time::Duration) -> f64 {
    (duration.as_secs_f64() * 1_000_000.0).round() / 1_000.0
}
