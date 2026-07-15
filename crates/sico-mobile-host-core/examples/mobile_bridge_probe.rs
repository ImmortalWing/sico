use std::{collections::BTreeSet, fs, time::Instant};

use sico_mobile_host_core::{BRIDGE_SCHEMA, BridgeOperation, BridgeRequest, MobileHostCore};

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let root = std::env::temp_dir().join(format!("sico-mobile-probe-{}", std::process::id()));
    let core = MobileHostCore::open(&root, BTreeSet::new()).expect("open mobile host core");
    let envelope = serde_json::to_vec(&BridgeRequest {
        schema: BRIDGE_SCHEMA.to_owned(),
        request_id: "performance".to_owned(),
        operation: BridgeOperation::Probe,
    })
    .expect("encode probe");
    let start = Instant::now();
    let mut response_bytes = 0_usize;
    for _ in 0..iterations {
        response_bytes += core.dispatch(&envelope, None).len();
    }
    let elapsed_ns = start.elapsed().as_nanos();
    println!(
        "{{\"iterations\":{iterations},\"elapsed_ns\":{elapsed_ns},\"response_bytes\":{response_bytes}}}"
    );
    fs::remove_dir_all(root).expect("remove mobile host probe store");
}
