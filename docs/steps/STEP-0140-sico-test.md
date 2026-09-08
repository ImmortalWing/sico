# STEP-0140: `sico test` — deterministic discovery and bounded execution

> - status: complete
> - phase: M14 (plan §3.4 application development loop)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: M14 plan §3.4 ("a real `sico test` ... with deterministic
>   discovery and bounded execution")

## 1. What was implemented

`sico test [PATH]` in the `sico` CLI:

- **Contract**: a test is a `NAME.sico` source beside a `NAME.test.json`
  manifest. Manifest fields (all optional, unknown fields rejected per
  the repository parsing rule): `stdin` (guest stdin text), `args`
  (guest arguments), `expect_exit` (default 0), `expect_stdout`
  (default empty; compared byte-exactly).
- **Deterministic discovery**: `PATH` is a `.sico` file or a directory
  scanned recursively with sorted traversal; sources without a manifest
  are not tests and are ignored. The report is a sorted, stable list of
  `PASS <path>` / `FAIL <path> (<reason>)` lines and a
  `N passed, M failed` summary; exit 0 iff every test passes.
- **Bounded execution**: every test builds through the same cache
  identity as `sico run` and executes through the real `sico-runner`
  under its default bounded limits (fuel, wall clock, memory,
  host-call budget) — a hanging or runaway guest fails the test with a
  typed runner outcome instead of hanging the suite.
- **Scope (declared)**: v0 covers the buffered profile; components
  importing `sico:script/streams@0.1.0` fail with a typed reason
  (the runner would inherit harness stdio). Provider grants (fs/net)
  are not configurable through manifests yet — such guests belong in
  runner-test suites for now.
- **Fixtures**: `tests/sico-tests/` ships two committed pairs
  (`greeting`, `echo-args` with `args`) as the executable usage example.

## 2. Evidence

- Root-workspace integration test
  `crates/sico-cli/tests/test_command.rs` (2/2 green): sorted discovery
  order, fail-first reporting, `1 passed, 1 failed` → exit 1, manifest-less
  sources ignored, `expect_exit`/`args` honored, all-pass → exit 0.
- Manual: `sico test tests/sico-tests` → `2 passed, 0 failed` (exit 0).

## 3. Validation

- `cargo fmt`/`clippy -D warnings` clean; `Cargo.lock` gained one
  additive line (`sico-cli` → serde derive).
- Full root + runner suites green (see STATUS).
