# STEP-0109: cross-platform native runner parity

> - status: blocked-external-evidence
> - phase: M11
> - started: 2026-09-01
> - completed: -
> - owners: autonomous-agent

## 1. Objective

Run the same scheduler, cancellation, stream, Store-isolation and DAP corpus on Windows x64 and Linux x64 native runners, with reproducible environment records per claimed platform.

## 2. Environment finding (2026-09-01)

The only available execution host is Windows 11 x64 (10.0.26200). Checked on 2026-09-01:

- `wsl --status` / `wsl -l -v`: WSL is **not installed** ("未安装用于 Linux 的 Windows 子系统"); installing it requires administrator rights and a machine reboot — an external environment input the agent will not fabricate or force;
- no other Linux x64 host, VM, or CI runner is registered to this repository.

Per the M11 plan §12 and ADR-0010: cross-compilation, schema fixtures and protocol fixtures never count as native runner evidence, so no Linux claim is made. **M11 cannot be GO until this step's corpus runs on a native Linux x64 host** (WSL2 with a real Linux kernel qualifies as native Linux userspace execution; cross-mingw binaries do not).

## 3. Prepared assets (ready to execute when a Linux x64 host exists)

- `tools/validate-step-0109.sh`: the Linux parity script (below) — builds the runner with the pinned toolchain and runs the same scheduler unit corpus + runner integration suite used by `validate-step-0107.ps1`, emitting the same evidence markers (`SCHEDULER_SCALE`, `CHAIN_CANCEL_1024`, `CHANNEL_RELAY_1GIB`, select matrices, adversarial ingress). RSS/handle assertions use `/proc/self/status` (`VmRSS`, `FDSize`-equivalent via `/proc/self/fd` count) — the Windows-only `windows_process_metrics` PowerShell helper gets a Linux counterpart inside the test build via `cfg(target_os)` (added in this step).
- Environment record format: `uname -a`, `rustc --version`, `ldd --version`, raw test logs under `target/evidence/step-0109/linux/`.

## 4. What runs on Linux (corpus parity contract)

Identical to the Windows corpus, with platform-typed differences allowed only in signal mechanics:

1. scheduler unit suite (state machine, cancellation tree, select matrices, channels, deadlock);
2. runner integration suite minus Windows-only fixtures (`#[cfg(windows)]` console-control tests stay Windows; their Linux counterpart is the SIGINT/SIGTERM path in the same tests behind `cfg(unix)`);
3. scale evidence: 1/2/16/256/1,024-task workloads, 1,024-chain cancellation, 1 GiB channel relay with flat RSS;
4. 100-generation changing-grants matrix and 20 pause/terminate DAP cycles with flat fd/handle counts.

## 5. Status rule

This step stays `blocked-external-evidence` until a native Linux x64 execution log exists in the repository evidence trail. STEP-0110 must record M11 gate 9 (platform parity) as unmet while this step is blocked.

## 6. Changes

- `tools/validate-step-0109.sh` (new, unexecuted — pending Linux host): the parity script described in §3/§4.
- `runner/sico-runner/tests/runner.rs`: `process_metrics` helper gained a `cfg(target_os = "linux")` counterpart reading `/proc/self`; the three `#[cfg(windows)]` leak-matrix tests from STEP-0105/0108 are now `#[cfg(any(windows, target_os = "linux"))]` so the same evidence runs on both platforms.

## 7. Validation

Windows-side: the `cfg`-widened tests still pass (STEP-0108 validator rerun). Linux-side: blocked (§2).

## 10. Audit links

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) §12 platform evidence policy
- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md) platform evidence
