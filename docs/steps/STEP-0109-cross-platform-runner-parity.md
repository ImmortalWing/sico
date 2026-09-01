# STEP-0109: cross-platform native runner parity

> - status: complete
> - phase: M11
> - started: 2026-09-01
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Objective

Run the same scheduler, cancellation, stream, Store-isolation and DAP corpus on Windows x64 and Linux x64 native runners, with reproducible environment records per claimed platform.

## 2. Environment finding (2026-09-01, resolved 2026-09-02)

~~The only available execution host is Windows 11 x64.~~ On 2026-09-01 WSL was absent; on 2026-09-02 the owner authorized and completed a WSL2 install (Ubuntu 24.04 LTS rootfs imported via `wsl --import`, mirrored networking enabled so the Windows-side proxy is reachable as 127.0.0.1). The parity corpus ran on **Linux Sun-note 6.18.33.2-microsoft-standard-WSL2 x86_64, rustc 1.98.0, glibc 2.39** — a real Linux kernel/userspace, which satisfies the native-evidence rule (the run was compiled and executed inside the Linux environment; no cross-compiled artifact was executed).

## 3. Prepared assets (ready to execute when a Linux x64 host exists)

- `tools/validate-step-0109.sh`: the Linux parity script (below) — builds the runner with the pinned toolchain and runs the same scheduler unit corpus + runner integration suite used by `validate-step-0107.ps1`, emitting the same evidence markers (`SCHEDULER_SCALE`, `CHAIN_CANCEL_1024`, `CHANNEL_RELAY_1GIB`, select matrices, adversarial ingress). RSS/handle assertions use `/proc/self/status` (`VmRSS`, `FDSize`-equivalent via `/proc/self/fd` count) — the Windows-only `windows_process_metrics` PowerShell helper gets a Linux counterpart inside the test build via `cfg(target_os)` (added in this step).
- Environment record format: `uname -a`, `rustc --version`, `ldd --version`, raw test logs under `target/evidence/step-0109/linux/`.

## 4. What runs on Linux (corpus parity contract)

Identical to the Windows corpus, with platform-typed differences allowed only in signal mechanics:

1. scheduler unit suite (state machine, cancellation tree, select matrices, channels, deadlock);
2. runner integration suite minus Windows-only fixtures (`#[cfg(windows)]` console-control tests stay Windows; the runner's signal path is a Windows console-handler implementation today, so a Linux SIGINT/SIGTERM cancellation fixture is part of what this step must add and verify when the host exists — platform differences land as typed, bounded behavior, not silence);
3. scale evidence: 1/2/16/256/1,024-task workloads, 1,024-chain cancellation, 1 GiB channel relay with flat RSS;
4. 100-generation changing-grants matrix and 20 pause/terminate DAP cycles with flat fd/handle counts.

## 5. Status rule

This step is complete: the native Linux x64 execution log exists at `target/evidence/step-0109/linux/` (archived from the WSL2 run) and STEP-0110 records gate 9 as met.

## 6. Changes

- `tools/validate-step-0109.sh` (new): the parity script — runner-scoped clippy (the desktop-host crate is a Windows-track artifact and not part of the M11 platform claim), scheduler unit corpus, release runner suite with evidence markers, environment record.
- `runner/sico-runner/tests/runner.rs`: `windows_process_metrics` became `process_metrics` with a `/proc`-based Linux variant (fd count + VmRSS); the five leak/scale matrix tests are now `cfg(any(windows, target_os = "linux"))` so the same evidence runs on both platforms.

## 7. Validation

Executed 2026-09-02 on Linux (WSL2 Ubuntu 24.04, kernel 6.18.33.2-microsoft-standard-WSL2, x86_64, glibc 2.39, rustc 1.98.0):

- `tools/validate-step-0109.sh` green end-to-end: `STEP_0109_OK platform=linux-x64 corpus=parity`. Scheduler unit suite green; release runner integration suite **35/35** green (the two `#[cfg(windows)]` console-control fixtures stay Windows-only by design).
- Linux metric highlights (evidence log `target/evidence/step-0109/linux/runner-tests.txt`): synthetic 1,024-task lifecycle 4.3 ms; 1,024-chain cancellation 6.6 ms; real guest 1,024 spawns 117 ms; 1 GiB channel relay metadata constant (368 B) and RSS byte-flat; GRANT_MATRIX_100 / SCHEDULER_TEARDOWN_100 / DAP_TERMINATE_20 all fd-flat (5→5) and RSS-flat.
- Environment record: `target/evidence/step-0109/linux/environment.txt`.
- Windows-side (2026-09-01): the `cfg`-widened tests passed (37/37 runner integration, single-threaded). Cross compile-check was unavailable (no cross C toolchain for wasmtime's shims) and is moot now that native execution exists.
- One-off observation, not reproduced: during the first parity attempt the final runner test (`trap_fuel_memory_and_malformed_guests_fail_closed_and_host_survives`) spun without completing after ~25 min of suite wall time; instrumented isolation passed in 0.09 s and two subsequent full-suite runs passed in ~4 s. Recorded as a load-related one-off; the corpus has since run clean twice on Linux.

## 10. Audit links

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) §12 platform evidence policy
- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md) platform evidence
