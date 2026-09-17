# STEP-0196: M19 support — fresh-host CI repair (binutils PATH, Python fixture prerequisite, runner build order)

> - status: complete
> - phase: M19 residual engineering (support track, mirrors the STEP-0074 precedent); enabled by owner directive 「继续完成sico，M22-M25」 (2026-09-17)
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `tools/ensure-python.ps1`, `tools/run-ci.ps1`, this STEP record

## 1. Objective

This host was rebuilt from scratch (the previous Windows profile owning
`E:/github/sico` no longer exists). Bringing the dev baseline green
surfaced three gaps that break `run-ci.ps1`'s M19 contract — "every
claim re-runnable from a fresh clone" — none of which were visible on
the previous machine because its environment pre-dated the contract:

1. **GNU dlltool**: compiling `windows-sys` for
   `x86_64-pc-windows-gnu` fails with `error calling dlltool
   'dlltool.exe': program not found` unless the repo-local MSYS2
   binutils (`target/tooling/msys2-binutils/mingw64/bin`) is on `PATH`.
   `tools/validate-step-0034.ps1` already resolves this candidate;
   `run-ci.ps1` did not.
2. **Python fixture prerequisite**: the Windows console-control
   fixtures (`runner_cli_observes_real_windows_console_control`,
   `runner_watch_observes_console_control_while_idle`) execute a
   stdlib-only Python script via `Command::new("python")`. A host
   without Python fails both tests. A clean-room finding: these tests
   had been attributed to "real Windows console" sensitivity; the
   executable evidence on this host is that **the missing `python`
   dependency was the cause** — with Python 3.12.10 available both
   tests pass in the same non-interactive harness (2/2 ok, 0.87s).
3. **Runner build order**: `crates/sico-cli/tests/packages_resolve.rs`
   falls back to `runner/sico-runner/target/debug/sico-runner.exe`
   when `SICO_RUNNER` is unset. `run-ci.ps1` ran `test (workspace)`
   before anything built the runner workspace, so on a fresh clone the
   two package-consumer tests fail (`cannot read component`, exit 121)
   — observed, then green after the runner suite had built the exe.

## 2. Changes

- `tools/ensure-python.ps1` (new): mirrors `ensure-wasmtime.ps1` —
  reuse an existing importable `python.exe` (PATH or
  `target/tooling/python-3.12.10`), otherwise download the pinned
  CPython 3.12.10 embeddable amd64 archive with SHA256 verification
  (`4acbed6d…25a3c3`). Prints the resolved `python.exe` path.
- `tools/run-ci.ps1`: prepend the MSYS2 binutils directory to `PATH`
  when present; resolve `$env:SICO_PYTHON` via `ensure-python.ps1` and
  prepend its directory to `PATH`; add a `build (runner debug)` step
  before `test (workspace)` so the packages_resolve fallback path
  exists on fresh clones. No test, limit or contract constant changed.

## 3. Registered correction to STEP-0195 §5

STEP-0195 recorded the console-control failures as "require a real
Windows console and fail in a non-TTY harness shell". The corrected
root cause is the missing `python` dependency; the non-TTY theory was
not confirmed by experiment (no console-only repro was run) and both
tests pass in this harness once Python is available. The environment-
sensitivity recorded by earlier audits (M14 audit note) remains
plausible history but is no longer the operative explanation on this
host.

## 4. Validation

- `ensure-python.ps1`: resolved the already-extracted
  `target/tooling/python-3.12.10/python.exe`; import probe
  (`os, signal, subprocess, sys`) passes.
- Console-control pair with Python on PATH: 2/2 ok (`--test-threads=1`).
- Full `run-ci.ps1` (non-Fast: clippy included) run on this host after
  the change — result recorded in §5.
- Scope note: this is a **fresh-host rehearsal** (empty cargo caches,
  fresh toolchain), not a wiped-`target/` fresh-clone simulation; the
  wasmtime/binutils/python artifacts under `target/tooling` persist.

## 5. Result

- `run-ci.ps1` (full, non-Fast) on this host: **CI GREEN, 11/11 steps
  PASS** — fmt, clippy (workspace, first run on this host, `-D
  warnings`), clippy (runner), build (runner debug), test (workspace),
  test (runner, serial), module boundaries, planning contract
  (M14–M25), application-profile matrix, cross-host matrix + UI
  corpus, git diff --check. Windows x64, toolchain 1.98.0-gnu,
  wasmtime 46.0.1, CPython 3.12.10 (embeddable).
- Repair-loop register (honest): the FIRST full-CI attempt after adding
  `ensure-python.ps1` died at script start with exit 1 and zero steps
  executed. Root cause: `Get-Command python.exe` resolved the
  WindowsApps app-execution alias stub, and under
  `$ErrorActionPreference = 'Stop'` the stub's stderr became a
  terminating NativeCommandError inside the usability probe. Fixed by
  excluding `*\Microsoft\WindowsApps\*` candidates and running the
  probe under `ErrorActionPreference = 'Continue'` inside try/catch.
  The green 11/11 run above is post-fix. The separate earlier
  background attempt that produced no output was killed before
  cargo work started (no compiler processes were alive); the final
  evidence is the foreground post-fix run.

## 6. Residuals

- `run-ci.ps1` still assumes the pinned GNU toolchain is installed
  (rustup bootstrap is documented in AGENTS.md §8, not automated).
- The MSYS2 binutils under `target/tooling` have no ensure-script; on
  a truly empty host they must be provisioned once (the .zst packages
  and their provenance pre-date this repository's CI contract).
- The embeddable CPython ships stdlib only — sufficient for the
  console-control fixtures; any future fixture needing pip must extend
  the ensure-script deliberately.
