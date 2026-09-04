# STEP-0122: measurement completion

> - status: complete
> - phase: M13 (parallel support track)
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Result

Both M13 measurement gaps closed:

1. **Semantic-index accuracy/latency benchmark** (`crates/sico-index/tests/semantic_bench.rs`, new): the 10-module fixture corpus index is loaded through the real `sico_index::execute` engine; all 8 protocol fixtures (outline/describe/slice/impact/flow accepts + 3 reject contracts) run 3 repetitions each with determinism asserted. Accuracy contract per RFC-0002: schema identity, request-id/snapshot/operation echo, budget echo, and truncation-respects-`max_bytes`; reject fixtures additionally prove the engine refuses stale snapshots with `SnapshotMismatch` before any result construction. Golden responses in `responses/` remain the structural protocol oracle's fixtures (`tools/validate-semantic-query.ps1`), which reruns green.
   Measured (Windows x64, Rust 1.98.0, 2026-09-02, 24 samples): `SEMANTIC_INDEX_BENCH corpus=10-modules fixtures=8 accepted=5 rejected=3 median_us=776 p95_us=1927` (non-SLA).
2. **`observed_ai_frequency`** in `ai-eval/error-taxonomy.json` replaced with the measured block from the STEP-0119 v2 run: 180 failed attempts / 2,880, failing-task distribution by candidate (A0 10 / B 8 / C 7), dominant failure `canonical-source-mismatch`, provenance-recorded (`subagent-run-v2-2026-08-04`, engineering feedback only), refresh rule for the live-model run. `tools/validate-error-taxonomy.ps1` now accepts either `not-measured` or a provenance-bearing measured block and still refuses ranking claims.

## 2. Notes

- Enabling the benchmark required `Deserialize` on the index structs and converting their `&'static str` schema/quality fields to `String` — a data-model change with no protocol-visible effect (serialized JSON is byte-identical; the validate-semantic-query oracle reruns green).
- All numbers are reproducible from committed fixtures and recorded commands: `cargo test -p sico-index --test semantic_bench -- --nocapture` and `tools/validate-error-taxonomy.ps1`.

## 3. Audit links

- [`M13 plan`](../plans/M13-ai-tooling-closure.md) STEP-0122
- [`STEP-0119`](./STEP-0119-ai-generation-quality-baseline.md)
