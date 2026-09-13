# STEP-0172: `sico-app run --script` seam closure and release-residual registration

> - status: complete
> - phase: M19 pre-gate hygiene (the run seam the M19 execution audit left unmeasured; owner-directed "M19 前 STEP")
> - completed: 2026-09-13
> - owners: autonomous-agent
> - artifacts: `crates/sico-runtime/src/lib.rs`, `crates/sico-app-cli/src/lib.rs`, `crates/sico-app-cli/tests/cli.rs`, `tests/end-to-end/README.md`

## 1. What was done

### 1.1 The seam

`sico-app pack/authorize` accepted RFC-0029 script packages, but `sico-app
run` could never execute one: `sico_runtime::run_authorized_package`
unconditionally invoked `wasmtime run … --invoke main()`, while a script
package's composed component is a `wasi:cli` command exporting `run` (no
`main`), and `-S cli=n` withheld the WASI CLI world the composed command
imports. No test ever ran a script package through `sico-app run`.

### 1.2 Fix

- `run_authorized_package` dispatches on the v1 manifest's script
  identity: script entries execute the command component directly (no
  `--invoke`) with inherited stdin (stdin is the ScriptInput channel);
  scalar entries keep `--invoke main()` byte-identically.
- `wasi_arguments` enables `-S cli=y` when the `script.args`/`script.stdio`
  capabilities are granted (the composed command imports
  wasi:cli/environment, stdin/stdout/stderr, exit and wasi:io).
- `sico-app run` propagates a script entry's process exit code verbatim
  instead of reclassifying a nonzero guest exit as `Trap` →
  `EXIT_RUNTIME_FAULT`. The adapter contract stands unchanged: ok
  collapses to 0, any nonzero `exit_code` field collapses to 1, exact
  `0..=119` codes remain a direct-runner (`sico run`) contract.

### 1.3 Evidence

`crates/sico-app-cli/tests/cli.rs::run_executes_a_script_package_end_to_end`
drives the real `sico-app` binary against a real wasmtime
(`SICO_TEST_WASMTIME`, skip-when-unset): pack `--script` → `run --grant
script.args --grant script.stdio` with piped stdin `ping` → stdout `ping`,
exit 0; a second package whose `exit_code` field is 7 → stdout `ping`,
process exit 1 (adapter collapse, propagated verbatim). 4/4 green with
`SICO_TEST_WASMTIME` set; `sico-runtime` suite 12/12 green.

### 1.4 Fixture-class registration (`tests/end-to-end/README.md`)

`tests/end-to-end/answer.sico` (`return 40 + 2`) was reported as stale.
Measured: `sico build` accepts it deterministically (its consumers are the
build-determinism and debug-triplet tests, plus the script-v0 refusal
case that asserts it is *refused* there); `sico run` refuses it by design
(script profile requires `record ScriptInput`). It is therefore not a
broken fixture but the declared scalar-profile build class. A README now
states the two fixture classes and why `answer.sico` keeps its scalar
shape; nothing was rewritten.

### 1.5 Release-residual registration (owner message 2026-09-13)

- **IExpress self-extracting installer**: skipped this cycle — local
  file-lock/bootstrapper race during self-validation. The supported
  install paths are the compiler zip + SDK zip with
  `tools/install-windows.ps1`; both zips exist under `dist/v0.0.2-dev/`.
- **Quality gates on packaging**: the release zips were produced with
  `-AllowDirty -SkipQualityGates` because the worktree carried this
  session's uncommitted changes. This step re-ran `cargo fmt` (both
  workspaces), `cargo clippy -D warnings` (both workspaces) and the
  targeted suites green; the next packaged refresh should run
  `tools/release-windows.ps1` without those skips on a clean tree.
- **Transient sico-ir E0308** (fingerprint race, first retry): disappeared
  on re-run, not reproduced since; recorded as an environment flake
  observation, no code change.

### 1.6 clippy/fmt debt swept in the same scope

`cargo clippy -D warnings` (root + runner workspaces) had accumulated
warnings from this cycle's uncommitted work: needless borrows and muts,
unused imports/variables, `bind_instead_of_map`/`?`/`type_complexity` in
the runner package-linking path (new `PackageExports` alias), an
`#[allow(clippy::too_many_arguments)]` on the packages runner entry, and
the `map_set.rs`/`stream_*` test debris from the STEP-0144 check-time
migration. `answer.sico`-era dead code (`chunk_of`) removed. No
`lower.rs` unreachable-pattern warning exists on the current tree
(re-checked; the RFC-0044 dead branch was already removed by the STEP-0169
fallback rework).

### 1.7 CI runner-step serialization (measurement fix, not budget change)

Two consecutive `run-ci.ps1` rounds failed only the runner step's two
process-budget tests (`dap_hundred_sequential…`,
`debug_pause_terminate…`) with *byte-identical* measurements
(rss 36589568 → 119500800, budget +64 MiB) while passing solo. The
asserts measure the test binary's process-wide RSS/handles; under the
default threaded harness, sibling tests' allocations land inside the
measurement — deterministic pollution, not runtime regression or noise.
`run-ci.ps1` now runs the runner suite serially (`--test-threads=1`;
the whole suite is seconds). The budget constants and the runtime
behavior they guard are unchanged; both tests also pass solo.

## 2. Validation

- `cargo fmt --all --check` root + runner: green (after applying fmt to
  `sico-automation-fixture` and the new test).
- `cargo clippy --locked --offline --workspace --all-targets
  --all-features -- -D warnings`: green, root + runner.
- `cargo test --manifest-path crates/sico-app-cli/Cargo.toml --test cli`
  with `SICO_TEST_WASMTIME`: 4/4.
- `cargo test --manifest-path crates/sico-runtime/Cargo.toml` with
  `SICO_TEST_WASMTIME`: 12/12.
- Full `tools/run-ci.ps1` (10 steps: fmt, clippy ×2, test ×2, module
  boundaries, planning contract, application-profile matrix, cross-host
  matrix + UI corpus, whitespace): **CI GREEN** on the third round —
  rounds 1–2 red only on fmt/clippy (this step's own new test code) and
  the runner budget asserts resolved by §1.7.

## 3. Residuals

- `sico-app run` still does not forward WASI arguments to a script
  entry (`ScriptInput.arguments` = the runtime's argv); the scalar
  no-app-arguments decision stands for v0. A declared argument-forwarding
  contract needs its own slice.
- Packages are refreshed from a clean tree at the next release cycle;
  the existing `dist/v0.0.2-dev/` zips predate this step.
