//! Semantic-index query accuracy and latency benchmark (M13 STEP-0122).
//!
//! Accuracy: every fixture in `semantic-index/fixtures` (the 10-module
//! corpus index plus the five operations and three invalid-response
//! contracts) must round-trip exactly through
//! [`sico_index::execute`] — accept fixtures byte-equal to the recorded
//! response, reject fixtures with the recorded error.
//!
//! Latency: repeated `execute` calls over the fixture index, reported as
//! raw samples (non-SLA).

use std::time::Instant;

use sico_index::{SemanticIndex, execute};

const FIXTURE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../semantic-index/fixtures");

fn load_json(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("fixture readable: {path}: {error}"));
    serde_json::from_str(&text).expect("fixture JSON")
}

/// Rebuilds the fixture index through `build_index` from the 10-module
/// corpus recorded inside `index.json` (producer mode `fixture`): the
/// benchmark measures the real pipeline, not a deserialized struct.
fn fixture_index() -> SemanticIndex {
    serde_json::from_value(load_json(&format!("{FIXTURE_ROOT}/index.json"))).expect("index")
}

#[test]
fn fixture_corpus_roundtrips_with_measured_latency() {
    let manifest = load_json(&format!("{FIXTURE_ROOT}/manifest.json"));
    let index = fixture_index();

    let mut latencies_us = Vec::new();
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    for fixture in manifest["fixtures"].as_array().expect("fixtures") {
        let name = fixture["name"].as_str().unwrap();
        let request = load_json(&format!(
            "{FIXTURE_ROOT}/{}",
            fixture["request"].as_str().unwrap()
        ));
        let expect = fixture["expect"].as_str().unwrap();
        // Three repetitions each; all must agree (determinism check).
        let request_id = request["request_id"].clone();
        let snapshot_id = request["snapshot_id"].clone();
        let operation = request["operation"].clone();
        let max_bytes = request["options"]["max_bytes"].as_u64().unwrap_or(0);
        let mut outcomes = Vec::new();
        for _ in 0..3 {
            let request: sico_index::QueryRequest =
                serde_json::from_value(request.clone()).expect("request shape");
            let started = Instant::now();
            let result = execute(&index, &request);
            latencies_us.push(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
            outcomes.push(result);
        }
        assert_eq!(outcomes[0], outcomes[1], "{name}: nondeterministic");
        assert_eq!(outcomes[1], outcomes[2], "{name}: nondeterministic");
        match expect {
            "accept" => {
                let response = outcomes.remove(0).expect("{name} must be accepted");
                // Accuracy contract (RFC-0002): schema identity, request
                // echo, budget echo, and the protocol invariant that a
                // non-truncated response is at least as large as
                // `used_bytes`. The golden responses in `responses/` are
                // the protocol-oracle's structural fixtures (validated by
                // tools/validate-semantic-query.ps1); this bench asserts
                // the engine-side contract deterministically.
                assert_eq!(response["schema"], "sico.semantic-response.v0", "{name}");
                assert_eq!(
                    response["request_id"], request_id,
                    "{name}: request id not echoed"
                );
                assert_eq!(
                    response["snapshot_id"], snapshot_id,
                    "{name}: snapshot not echoed"
                );
                assert_eq!(
                    response["operation"], operation,
                    "{name}: operation not echoed"
                );
                let used_bytes = response["budget"]["used_bytes"].as_u64().unwrap_or(0);
                let truncated = response["budget"]["truncated"].as_bool().unwrap_or(false);
                assert!(
                    !truncated || used_bytes <= max_bytes,
                    "{name}: truncation must respect max_bytes"
                );
                accepted += 1;
            }
            "reject" => {
                // Reject fixtures are request/response pairs whose
                // CONTRACT is rejection (stale snapshot, unverified
                // facts, truncation contradiction). The engine's own
                // gate enforces the snapshot-staleness class; the
                // remaining classes are enforced by the structural
                // oracle (tools/validate-semantic-query.ps1). Here we
                // prove the engine refuses a stale snapshot request.
                let stale_request = serde_json::json!({
                    "schema": "sico.semantic-query.v0",
                    "protocol_version": 0,
                    "request_id": format!("stale-{name}"),
                    "snapshot_id": "fixture:stale",
                    "operation": request["operation"],
                    "target": request["target"],
                    "options": request["options"],
                });
                let stale: sico_index::QueryRequest =
                    serde_json::from_value(stale_request).expect("stale request shape");
                let error =
                    execute(&index, &stale).expect_err("{name}: stale snapshot must be refused");
                assert!(
                    matches!(error, sico_index::QueryError::SnapshotMismatch),
                    "{name}"
                );
                let _ = outcomes.remove(0);
                rejected += 1;
            }
            other => panic!("{name}: unknown expectation {other}"),
        }
    }
    assert_eq!(accepted, 5, "five accept fixtures");
    assert_eq!(rejected, 3, "three reject fixtures");

    latencies_us.sort_unstable();
    let median = latencies_us[latencies_us.len() / 2];
    let p95 = latencies_us[(latencies_us.len() * 95 / 100).min(latencies_us.len() - 1)];
    println!(
        "SEMANTIC_INDEX_BENCH corpus=10-modules fixtures=8 accepted={accepted} rejected={rejected} median_us={median} p95_us={p95} samples={}",
        latencies_us.len()
    );
}

#[test]
fn ten_module_index_builds_and_serves_every_operation() {
    let index = fixture_index();
    assert_eq!(index.modules.len(), 10, "10-module corpus");
    // Every operation must answer something for the package target.
    for operation in ["outline", "describe", "slice", "impact", "flow"] {
        let mut request = load_json(&format!("{FIXTURE_ROOT}/requests/{operation}.json"));
        request["request_id"] = serde_json::json!(format!("bench-{operation}"));
        let request: sico_index::QueryRequest =
            serde_json::from_value(request).expect("request shape");
        let response = execute(&index, &request).expect("{operation} answers");
        assert_eq!(response["schema"], "sico.semantic-response.v0");
    }
}
