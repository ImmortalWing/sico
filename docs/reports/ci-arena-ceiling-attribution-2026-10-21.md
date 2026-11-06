# CI arena-ceiling attribution — `run_executes_a_script_package_end_to_end`

Date: 2026-10-21
Scope: the two GitHub Actions runs triggered on `codex/m22-w1-ci` after
STEP-0489 / STEP-0490, and the attribution of their single red point.
Evidence levels: `external-pilot` (real GitHub Actions runner) and
`internal-fixture` (local reproduction with the repo-pinned wasmtime).
Neither is a production claim.

## 1. Run inventory

| Run | SHA | Commit | Conclusion | Red point |
|---|---|---|---|---|
| 39454624507 | `253142794c19` | STEP-0489 | **failure** | `test (workspace)` → `sico-app-cli::run_executes_a_script_package_end_to_end` |
| 39457030463 | `6808569a9333` | STEP-0490 | **failure** | identical test, identical message |

Both runs are byte-identical at the failure point:

```
test run_executes_a_script_package_end_to_end ... FAILED
thread 'run_executes_a_script_package_end_to_end' panicked at crates\sico-app-cli\tests\cli.rs:264:5:
    forcing trap when growing memory to 2214592512 bytes
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Every other CI step is green in both runs:

```
PASS: fmt, clippy (workspace), clippy (runner), build (runner debug),
      test (runner), module boundaries, planning contract (M14-M25),
      application-profile matrix, cross-host matrix + UI corpus,
      whitespace (git diff --check)
```

The last green run before these two is 36559522966 (SHA `b9f652b`, 2026-09-29).
Between that SHA and `2531427` the branch gained the M22 self-host track
(not the M23 diagnostics work).

## 2. Root cause

Two constants in the same tree disagree by 1,024 Wasm pages (64 MiB):

| Constant | Value | Bytes | Location |
|---|---|---|---|
| declared guest memory `minimum`/`maximum` | 33,792 pages | 2,214,592,512 | `crates/sico-codegen-wasm/src/canonical.rs:21` (`MEMORY_PAGES`), emitted at `canonical.rs:956-961`, `package_builder.rs:118-121`, `lib.rs:1252-1256`, `lib.rs:1334-1337` |
| enforced host ceiling (`-W max-memory-size`) | 2,048 MiB | 2,147,483,648 | `crates/sico-runtime/src/lib.rs:49` (`HostLimits::default().memory_bytes`) |

`crates/sico-runtime/src/lib.rs:382-413` spawns the runtime with

```
-W max-memory-size=2147483648
-W trap-on-grow-failure=y
```

The module's declared **minimum** (2,214,592,512 B) is larger than the
enforced ceiling (2,147,483,648 B), so wasmtime cannot satisfy the minimum
at instantiation; with `trap-on-grow-failure=y` the refusal surfaces as a
trap, and the reported byte count is exactly the requested minimum
(33,792 × 64 KiB = 2,214,592,512 = `2214592512`).

**Why only this one test is red.** The in-process runner path enforces a
*larger* ceiling — `runner/sico-runner/src/lib.rs:245`
(`RunnerLimits::default().memory_bytes = 2560 * 1024 * 1024` =
2,684,354,560 B ≥ 2,214,592,512 B) — so every runner-hosted test, including
the whole self-host suite, instantiates fine. Only the `sico-app` →
`sico-runtime` → external-wasmtime path applies the 2 GiB ceiling.

**Introducing commit.** `c50b3fd` *"feat(codegen): arena treadmill intrinsics
+ 2 GiB guest ceiling per RFC-0029 amendments (STEP-0402/0421/0454/0484)"*
changed `ARENA_LIMIT` 128 MiB → 2048 MiB **and** `MEMORY_PAGES` 2048 →
33,792 in one commit. Before it, the pairing was self-consistent
(min = max = 2,048 pages = 128 MiB ≤ 2 GiB ceiling), which is why
`b9f652b` was green.

## 3. Local reproduction (deterministic — not a runner-memory problem)

Using the repo-pinned runtime (`tools/ensure-wasmtime.ps1` → wasmtime
v46.0.1) and a minimal module `(module (memory N N) (func (export "run")))`:

| Case | declared min | `-W max-memory-size` | result |
|---|---|---|---|
| A | 33,792 pages | 2,147,483,648 | **trap: `forcing trap when growing memory to 2214592512 bytes`** |
| B | 33,792 pages | 2,214,592,512 | exit 0 |
| C | 33,792 pages | 2,684,354,560 | exit 0 |
| D | 2,048 pages (pre-`c50b3fd` shape) | 2,147,483,648 | exit 0 |
| E | 32,768 pages (2 GiB) | 2,147,483,648 | exit 0 |

Case A reproduces the CI message byte-for-byte on a workstation with 15.6 GB
RAM; case D shows the pre-`c50b3fd` shape passing under the same ceiling.
**The failure does not depend on host memory budget** — the earlier
"CI runner ran out of RAM" reading is withdrawn: it is a constant mismatch
between the codegen and the runtime, and it fails on every machine.

## 4. Why the local workspace test looked green

`crates/sico-app-cli/tests/cli.rs:213-217`:

```rust
let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") else {
    // run-ci sets SICO_TEST_WASMTIME; without a runtime binary the
    // launch seam cannot be exercised.
    return;
};
```

Without `SICO_TEST_WASMTIME` the test returns immediately, so a local
`cargo test -p sico-app-cli` reports `ok` **without launching a guest**.
`tools/run-ci.ps1:15` sets the variable, so only CI exercises the
launch seam. Any future local check of this test must set the variable:

```powershell
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
```

## 5. Candidate resolutions (M22 track decision — not taken here)

Invariant to restore: **the declared guest memory minimum must be ≤ the
enforced host ceiling on every execution path** (currently only the
`sico-app` path is below the declared minimum).

- **(A) Keep the "2 GiB guest ceiling" literal.** Set `MEMORY_PAGES = 32768`
  and move the 64 MiB headroom *inside* the arena ceiling
  (`ARENA_LIMIT = 2 GiB − 64 MiB`). Declared min then equals the host
  ceiling (case E). The RFC-0029 amendment must then state whether it
  bounds the *linear memory* or the *arena*.
- **(B) Keep `MEMORY_PAGES = 33792`, raise the app-layer ceiling** to
  ≥ 2,214,592,512 B (e.g. 2,560 MiB, matching `RunnerLimits::default()`).
  The arena ceiling stays 2 GiB; the *linear memory* ceiling becomes
  2.06 GiB, so the same RFC-0029 wording clarification is needed.
- **(C) Declare a small minimum and let the guest grow.**
  `minimum = 2,048` pages, `maximum = 32,768` pages. This also stops every
  script instantiation from pre-committing 2.06 GiB, but a guest growth past
  2 GiB then traps as a wasmtime trap rather than the registered
  arena-exhaustion trap, so the RFC-0029 trap contract must be re-checked.

All three are M22-track contract changes (RFC-0029 amendments, STEP-0484 /
STEP-0487) and touch `crates/sico-codegen-wasm` and `crates/sico-runtime`.
This audit changes no product code.

## 6. Reproduction commands

```powershell
# 1. the repo-pinned runtime
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1

# 2. the failing workspace test with the launch seam actually armed
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p sico-app-cli --test cli `
  run_executes_a_script_package_end_to_end -- --nocapture

# 3. the isolated constant-mismatch experiment (no repository change)
$wt = $env:SICO_TEST_WASMTIME
Set-Content target\tmp\min33792.wat "(module (memory 33792 33792) (func (export `"run`") (result i32) (i32.const 0)))"
& $wt run --codegen cache=n -W fuel=10000000 -W timeout=10000ms `
  -W max-memory-size=2147483648 -W trap-on-grow-failure=y target\tmp\min33792.wat
```

## 7. What this does *not* claim

- It does not claim the M23 work (STEP-0489 / STEP-0490) is implicated. The
  code footprint of those two commits is `crates/sico-semantics/src/lib.rs`
  (diagnostic message text and arguments) plus `tools/`; neither touches
  codegen, the runtime, or the memory constants.
- It does not claim the M22 track is fixed. Both CI runs are red; the M23
  independent-CI gate is therefore still open, with its red point now
  attributed to an M22-owned constant mismatch.
- It does not claim a production or external-adoption result.

## 8. M22-track resolution (STEP-0497, 2026-10-21)

This section is appended by the M22 track; §1–§7 above are the M23
attribution as committed and are left unchanged.

### 8.1 One correction to the attribution

§2 identifies the enforced host ceiling (2 GiB) as the counterpart that
disagrees with the declared minimum. That is a real disagreement, but it is
**not the binding constraint**: the effective ceiling is
`min(package manifest limit, host limit)`
(`crates/sico-runtime/src/lib.rs`, `effective_limits`), and the package
manifest limit for Script packages was **64 MiB** —
`sico-app-cli::script_limits()` inherited
`sico_package::RuntimeLimits::default()` (`memory_bytes = 64 * 1024 * 1024`).
The effective ceiling was therefore `min(64 MiB, 2 GiB)` = **64 MiB**, i.e.
1/32 of the declared minimum.

This was measured, not inferred: with only the host grant raised to
2,560 MiB (option B of §5), the test **still failed** with the identical
message. Both ends of the `min` were below the declared minimum, so both
had to be repaired.

§2's "Why only this one test is red" reasoning is unaffected and remains
correct: the in-process runner path enforces 2,560 MiB and so instantiates
fine.

### 8.2 What was changed

| Layer | Before | After |
|---|---|---|
| declared guest memory | 33,792 pages, min == max (2,214,592,512 B) | **unchanged** (no snapshot, no emitted byte changed) |
| Script package manifest limit | 64 MiB | **`SCRIPT_MEMORY_BYTES`** = 2,214,592,512 B (`sico_package::script_profile_limits()`) |
| runtime `HostLimits::default().memory_bytes` | 2,048 MiB | **2,560 MiB** (same value as `RunnerLimits::default()`) |
| ceiling below the declared minimum | opaque wasmtime instantiation trap | **typed refusal**: `build_script_v1` rejects the package; `run_authorized_package` returns `PackageRuntimeError::MemoryCeilingBelowDeclared` |

Option (B) of §5 was taken, with the added requirement that the *package*
limit is aligned as well, and with (A) and (C) deliberately not taken:
(A) would change every emitted component byte, and (C) would turn the
2 GiB arena ceiling into a wasmtime growth trap instead of the registered
arena-exhaustion trap.

### 8.3 Result and guards

- The red point is green: `cargo test -p sico-app-cli --test cli
  run_executes_a_script_package_end_to_end` with `SICO_TEST_WASMTIME` set
  → **ok** (reproduced red with the identical message before the change).
- Four invariant guards now make the pairing executable instead of
  conventional, and **none of them needs a runtime binary** (so they cannot
  be skipped the way the launch-seam test is skipped without
  `SICO_TEST_WASMTIME`): the emitted memory section is checked at byte
  level (`min == max == 33,792` pages), and the package, runtime and runner
  layers each assert that their own ceiling covers the declared bound.
- RFC-0029 carries the matching STEP-0497 amendment: the "2 GiB" of the
  STEP-0454 amendment is the **arena** ceiling (`ARENA_LIMIT`), the
  **declared linear memory** is 33,792 pages = 2,214,592,512 B with
  `min == max`, and every execution path's effective ceiling must be at
  least that declared bound. The same amendment registers the resulting
  design debt: with `min == max`, the original "unless a stricter
  package/Host limit applies" clause is **unimplementable** for Script
  components, because any limit below the declaration is unsatisfiable
  rather than tighter.

Record: [STEP-0497](../steps/STEP-0497-m22-w2-ci-arena-ceiling-repair.md).

### 8.4 Independent CI verdict (2026-10-21)

The owner's authorization was granted, STEP-0496 and STEP-0497 were committed
together as `7e5ad3c` (35 files, +4868/−99) and pushed to
`github/codex/m22-w1-ci` (fast-forward `6808569..7e5ad3c`; the remote ref was
read back and equals `7e5ad3c21197e40848beb403c53074e3458feaf8`). That
triggered the independent runner:

| Item | Value |
|---|---|
| run id | `39524849306` |
| workflow | `M22 W1 independent CI` (`.github/workflows/m22-w1-ci.yml`) |
| head_sha | `7e5ad3c21197e40848beb403c53074e3458feaf8` |
| runner | `windows-2025` (GitHub Actions, independent of this workstation) |
| started / completed | 2026-10-21T01:01:29Z → 2026-10-21T01:28:08Z |
| **verdict** | **`status: completed` / `conclusion: success`** |
| job | `windows-gnu`: every step success, including `Run repository CI on a fresh runner` |

The red point is therefore green **on an independent runner**, not only on this
workstation: the fix is not a local-only green. The two historical red runs
(39454624507, 39457030463) stay red in the inventory — they are history, not
retractions.

### 8.5 What this section does *not* claim

- It does not claim M22 gate 3 progress: run-body emission, the
  37-corpus / formatter / full-source canary ladder and the ADR-0015
  `A == B == C` + `.sapp` chain are untouched.
- It does not claim a production or external-adoption result.
