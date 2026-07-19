# STEP-0081: structured in-process sico-runner

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - evidence: [`in-process-script-runner-v0`](../reports/in-process-script-runner-v0.md), `tools/validate-step-0081.ps1`

## 1. Objective

Deliver an in-process Script runner that invokes the `sico:script/program@0.1.0` export directly (the STEP-0076-tested fallback path), mapping Runtime outcomes to the RFC-0029 exit classes with typed structure instead of stderr text parsing, enforcing WASI/memory/fuel/timeout limits with no ambient environment, filesystem or network, and surviving malicious guests.

## 2. Toolchain constraint and placement

The workspace verification gate uses the GNU toolchain, which on this host cannot link the Wasmtime crate (its `dlltool` needs a standalone GNU `as`; see `prototypes/script-profile/README.md`). The runner therefore lives in `runner/sico-runner` as a product crate excluded from the workspace (same mechanism as prototypes, but with a stability contract), built with the pinned MSVC toolchain, and does not weaken the workspace gate. `sico run` integration (STEP-0082) may delegate across this executable boundary without adding a compiler-to-Runtime crate dependency.

## 3. Required order

1. Runner library: one Store per call, direct Program `run` invocation, M8 input bounds enforced before execution, per-call hostcall-fuel budget, fuel for non-termination, epoch deadline for wall-clock timeout, `StoreLimits` memory ceiling.
2. Typed outcome model: Output(stdout/stderr/exit 0..=119), domain ScriptError, cancellation, timeout, resource limit/trap, launch failure, incompatible component; guest exit values outside `0..=119` rejected, never truncated.
3. `sico-runner` CLI with the RFC-0029 exit mapping 0–119/122/123/124/125/126/127 and machine-readable JSON diagnostics on stderr, never on stdout.
4. Malicious-guest evidence: trap, non-termination, memory bomb and malformed-result guests leave the host healthy for subsequent calls.
5. Deterministic behavior and workspace regression before handoff to STEP-0082.

## 4. Included

- direct Program export invocation with exact guest exit codes;
- typed fault classes and exit mapping;
- fuel/timeout/memory/WASI-context limits with no ambient authority;
- malicious-guest host-survival evidence.

## 5. Excluded

- unified `sico run`/`eval`, source and machine caches (STEP-0082);
- standard library (STEP-0083);
- composed Adapter packaging inside the runner (the adapter remains the `sico-app compose` path);
- streaming/async (M9);
- changing the workspace GNU verification gate.

## 6. Exit gate

- every RFC-0029 exit class is produced by a typed path and asserted in tests;
- exact guest exit codes 0..=119 pass through; out-of-range values are rejected;
- malicious guests (trap, fuel, memory, malformed result) fail closed and the runner stays usable;
- no env/cwd/fs/network is granted to the guest by default;
- STEP-0077–0080 evidence and the full workspace regression remain green.

## 7. Links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`STEP-0080 evidence`](../reports/compiler-script-profile-adapter-manifest-v0.md)
- [`STEP-0076 composition evidence`](../reports/script-profile-composition-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
