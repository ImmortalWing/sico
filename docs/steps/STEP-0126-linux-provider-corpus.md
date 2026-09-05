# STEP-0126: Linux x64 provider corpus rerun

> - status: complete
> - phase: M12 (residual integration deliverable (b) from STEP-0118)
> - started: 2026-09-05
> - completed: 2026-09-05
> - owners: autonomous-agent
> - platform: Linux x86_64 native (WSL2 Ubuntu-24.04, real Linux kernel —
>   satisfies the native-evidence rule per STEP-0109 precedent)

## 1. Outcome

The complete `sico-http-provider` corpus and the guest-visible
`http@0.2.0` runner corpus rerun natively on Linux x64: **ALL GREEN**.
M12 plan §9 required exactly this before any Linux claim: "the same local
deterministic TLS fixture corpus reruns on Linux x64 before any Linux
claim".

## 2. Method

- Working tree (Windows `E:\github\sico`, M12 STEP-0125 state) synced via
  rsync into the WSL2 clone at `/root/sico`; `cargo fetch --locked` (root
  workspace + runner workspace) through the host proxy pulled the M12
  dependencies (rustls/ring/rcgen and transitive build deps) that M11 had
  never needed on Linux; afterwards every step ran `--offline --locked`.
- Validator: `tools/validate-step-0126.sh` (fmt workspace+runner, clippy
  `-D warnings` for provider and runner, provider corpus, runner corpus
  serial, STEP-0089 frozen buffered-HTTP oracle).
- Evidence: `target/evidence/step-0126/linux/validator.log` (script exit
  0; "STEP-0126 LINUX x64: ALL GREEN"). Toolchain: rustc 1.98.0
  x86_64-unknown-linux-gnu.

## 3. Results (Linux x64 native)

| Corpus | Result |
|---|---|
| fmt (workspace + runner) | ok |
| clippy provider + runner, `-D warnings` | ok |
| provider tests (authority/TLS/framing/redirect/secrets/engine/streaming/pooling/retry/upload/fixtures) | 48/48 |
| runner lib tests (scheduler/cancellation/DAP/http links) | 37/37 |
| http@0.2.0 guest-fixture tests (buffered/pooling/streaming/upload+secret/refusals) | 6/6 |
| runner integration tests (`tests/runner.rs`, serial) | 35/35 (2 tests are `#[cfg(windows)]` console-handler tests; Linux count is by design) |
| STEP-0089 frozen buffered-HTTP oracle (`sico-package`) | 13/13 |

## 4. Platform claim

With STEP-0118 (Windows x64) and this step, the secure HTTP provider and
the `http@0.2.0` runner integration hold **runtime-verified** evidence on
Windows x64 and Linux x64. No macOS, mobile or other-platform claim is
made. The `#[cfg(windows)]`-gated runner tests remain Windows-only
evidence by design.
