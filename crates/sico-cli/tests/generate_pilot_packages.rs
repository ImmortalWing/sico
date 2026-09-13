//! STEP-0147/0149 fixture generator: builds the two signed internal
//! packages consumed by `pilots/log-analyzer` and refreshes the pilot's
//! `sico-lock.json` digest entries. Run explicitly:
//!
//! ```text
//! cargo test -p sico-cli --offline --test generate_pilot_packages -- --ignored --nocapture
//! ```

use std::{fs, path::PathBuf};

use sha2::{Digest, Sha256};
use sico_codegen_wasm::package_builder::{csv_package, table_stats_package};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sign_development};

#[test]
#[ignore = "explicit fixture generator: refreshes pilots/log-analyzer artifacts"]
fn generate_pilot_packages() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let packages_dir = repository.join("pilots/log-analyzer/pkgs");
    fs::create_dir_all(&packages_dir).unwrap();

    let mut lock_entries = Vec::new();
    for (name, app_id, component) in [
        ("csv", "pkg.csv", csv_package()),
        ("table_stats", "pkg.table-stats", table_stats_package()),
    ] {
        let unsigned = build_unsigned(BuildInput {
            app_id: app_id.to_owned(),
            app_version: "1.0.0".to_owned(),
            component,
            resources: Vec::new(),
            source_effects: Vec::new(),
            capabilities: Vec::new(),
            limits: RuntimeLimits::default(),
        })
        .expect("package builds");
        let signed = sign_development(&unsigned, &[7_u8; 32]).expect("signs");
        let digest = format!("{:x}", Sha256::digest(signed.as_slice()));
        fs::write(packages_dir.join(format!("{name}.sapp")), &signed).unwrap();
        lock_entries.push(format!(
            "{{\"name\":\"{name}\",\"version\":1,\"path\":\"pkgs/{name}.sapp\",\"sha256\":\"{digest}\"}}"
        ));
        println!("{name}: {} bytes sha256={digest}", signed.len());
    }

    let lock = format!(
        "{{\n  \"schema\": \"sico:source-lock:v0\",\n  \"packages\": [\n    {}\n  ]\n}}\n",
        lock_entries.join(",\n    ")
    );
    fs::write(repository.join("pilots/log-analyzer/sico-lock.json"), lock).unwrap();
}

#[test]
#[ignore = "explicit fixture generator: refreshes pilots/tetris-vision artifacts"]
fn generate_tetris_vision_package() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let packages_dir = repository.join("pilots/tetris-vision/pkgs");
    fs::create_dir_all(&packages_dir).unwrap();
    let component = sico_codegen_wasm::package_builder::image_vision_package();
    let unsigned = build_unsigned(BuildInput {
        app_id: "pkg.image-vision".to_owned(),
        app_version: "1.0.0".to_owned(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("vision package builds");
    let signed = sign_development(&unsigned, &[7_u8; 32]).expect("signs");
    let digest = format!("{:x}", Sha256::digest(signed.as_slice()));
    fs::write(packages_dir.join("image_vision.sapp"), &signed).unwrap();
    let lock = format!(
        "{{\n  \"schema\": \"sico:source-lock:v0\",\n  \"packages\": [\n    {{\"name\":\"image_vision\",\"version\":1,\"path\":\"pkgs/image_vision.sapp\",\"sha256\":\"{digest}\"}}\n  ]\n}}\n"
    );
    fs::write(repository.join("pilots/tetris-vision/sico-lock.json"), lock).unwrap();
    println!("image_vision: {} bytes sha256={digest}", signed.len());
}
