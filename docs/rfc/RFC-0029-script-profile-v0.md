# RFC-0029: Script Profile v0

> - status: proposed
> - date: 2026-07-17
> - authors: autonomous-agent
> - target language/platform version: M8 draft
> - supersedes: RFC-0014/RFC-0019 source-run surface only after acceptance
> - superseded-by: -

## Summary

Define a statically typed, bounded batch Script Profile that maps one Sico entry to a versioned Component interface carrying arguments, binary stdin, binary stdout/stderr, exit status and structured errors. The profile preserves `.sapp` verification and least-authority Runtime execution while presenting `sico run` and `sico eval` as the normal local workflow.

## Problem

`sico-app dev SOURCE.sico` proves one-command source execution, but the current executable subset is synchronous scalar `main()` with no application arguments or guest stdin. The frontend and IR describe strings, records, `Result`, calls, effects, resources and async constructs that the current backend still refuses. A source command alone therefore cannot support representative AI automation such as stdin transforms, JSON filtering or scoped file operations.

The Script profile must not solve convenience by granting ambient filesystem/network/environment authority, weakening `Int` semantics, ignoring unsupported values, parsing Runtime stderr as a stable protocol or allowing unbounded input/output capture.

## Goals and non-goals

Goals:

- exact args/stdin/stdout/stderr and exit behavior;
- strict Text versus Bytes semantics;
- a small versioned WIT boundary;
- deterministic Program Component, adapter and package identities;
- explicit limits and stable failure classes;
- source and machine-code caches that do not weaken trust;
- one normal `sico run` command while preserving module ownership;
- a path to text/bytes/list/JSON/scoped-file standard APIs.

Non-goals for v0:

- streaming or interactive terminal I/O;
- Task/Future/Stream source codegen;
- HTTP or child processes;
- stateful REPL or top-level statements;
- implicit host environment/current-directory access;
- general arbitrary-depth generic Component ABI;
- claiming arbitrary-precision `Int` Runtime support without implementing it.

## Semantics

### Entry

The source entry is exactly one exported function equivalent to:

```sico
function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  // body
end function
```

`ScriptInput.arguments` preserves argument order. Every argument must convert losslessly to WIT `string`; a non-Unicode platform argument is rejected before guest execution. `ScriptInput.stdin` is arbitrary bytes.

`ScriptOutput.stdout` and `stderr` are arbitrary bytes. A guest exit value is an `s64` at the WIT boundary but must be in `0..=119`; values outside that range are rejected rather than truncated. The user-facing `sico run` exit mapping is:

| Exit | Meaning |
|---:|---|
| 0–119 | exact guest `ScriptOutput.exit-code` |
| 120 | source syntax/semantic/lowering/codegen diagnostic |
| 121 | CLI, I/O, package, trust, capability or Host setup failure |
| 122 | returned `ScriptError` / typed domain failure |
| 123 | cancellation |
| 124 | timeout |
| 125 | resource limit or guest trap |
| 126 | runner launch failure |
| 127 | required compiler, adapter or runner not found/incompatible |

This mapping is specific to the new M8 `sico run` surface. Existing `sico-app run` v0 exit codes remain compatible until an explicit migration RFC changes them.

### Source and guest stdin

- `sico run FILE.sico` gives process stdin to the guest.
- `sico run -` consumes process stdin as source and gives empty guest stdin.
- A later explicit file/descriptor option may supply guest input with source stdin, but one stream is never ambiguously consumed twice.

### Bounds

Script v0 bounds are:

- at most 1,024 arguments;
- at most 64 KiB UTF-8 bytes per argument;
- at most 1 MiB UTF-8 bytes across all arguments;
- at most 8 MiB each for stdin, stdout and stderr;
- at most 64 KiB UTF-8 bytes in `ScriptError.message`;
- at most 64 MiB guest linear memory unless a stricter package/Host limit applies.

The effective limit is the minimum of language/profile, package and Host limits. Exceeding a bound produces exit 125 and no silent truncation.

### Numbers

Sico `Int` retains its specified arbitrary-precision direction. Script v0 adds explicit fixed-width `I64/U64` values for sizes, indices, exit values and dynamic machine arithmetic. Codegen may not silently reinterpret `Int` as `i64`.

The STEP-0077 source surface is `I64.literal(Int literal)` / `U64.literal(Int literal)`, `checked_add`, `checked_sub`, `equal` and `less_than`. Literal construction is explicit and range checked; runtime conversion from an arbitrary `Int` remains a later implementation item. Checked arithmetic returns `Result[I64|U64, NumericError]`, with `NumericError.overflow` and `NumericError.underflow`. Ordinary `+` on fixed-width values is rejected; wrapping and trap-only failure are not Script v0 arithmetic semantics.

### Capabilities

Args and the current invocation's bounded stdio are visible capabilities named `script.args` and `script.stdio`; an explicit local `sico run` auto-grants only these two. They remain in the verified import/manifest closure even though they do not require a permission prompt. Environment access uses coarse request `environment.read` plus an exact Host-side name allowlist. Files use separate `storage.read` and `storage.write` requests plus canonical scoped roots. Network and process execution are absent from M8. Unknown imports, requests or scopes fail closed.

The trusted adapter is the only code allowed to import the broad WASI CLI environment/stdio interfaces. Its exact digest is verified, and its v0 implementation obtains arguments but passes no environment variables or current working directory.

STEP-0083 amendment (implemented): the scoped file channel is two versioned interfaces so the read/write split exists at import granularity — `sico:script/fs-read@0.1.0` (`read: func(path: string) -> result<list<u8>, string>`, `exists: func(path: string) -> result<bool, string>`) maps to `storage.read`, and `sico:script/fs-write@0.1.0` (`write: func(path: string, content: list<u8>) -> result<_, string>`) maps to `storage.write`. Both are declared as imports of the `sico:script/program@0.1.0` world; a program only imports what it calls. The Host grants canonical roots (`sico run --fs-read-root` / `--fs-write-root`); guest paths are relative (no parent components, drive prefixes, colons or backslashes), canonical containment is verified per call, path text is bounded to 4 KiB and file payloads to the 8 MiB channel budget. Without a matching grant every call fails closed with a typed `result` error; the error text never embeds a host path.

### Cache identity

The source cache key is SHA-256 over domain `SICO-SCRIPT-SOURCE-CACHE-V0\0` followed by a fixed-order sequence of fields. Each variable-length field is encoded as unsigned 64-bit little-endian byte length followed by exact bytes. Fixed digests are raw 32-byte SHA-256 values. Fields are:

1. compiler executable digest;
2. compiler build ID;
3. language semantics ID;
4. Script WIT package/version;
5. adapter digest;
6. exact source bytes;
7. app ID and app version;
8. profile ID `script-v0`;
9. manifest schema ID;
10. canonical codegen-options bytes.

Cache hits are strictly reverified. A missing/corrupt/stale entry is never executed or silently treated as trusted. Wasmtime machine-code caching is a separate platform/engine-keyed performance layer.

## Syntax candidates

### Candidate A: explicit typed main

```sico
function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: input.stdin, stderr: Bytes.empty(), exit_code: 0))
end function
```

Accepted as the v0 baseline because it preserves normal module/function parsing and exposes the boundary to static analysis.

### Candidate B: unrestricted top-level statements

```sico
stdout.write(stdin.read_all())
```

Deferred to M9. It changes parser recovery, formatter, HIR, module initialization, capability visibility and LSP behavior before the Script ABI is proven.

### Candidate C: labeled `script:` block

```sico
script:
  return input.stdin
end script
```

Deferred with Candidate B for an evidence-based M9 syntax decision.

`sico eval EXPR` may synthesize Candidate A in memory; this does not add a new source grammar.

## Positive and negative cases

Positive cases include empty input, Unicode arguments, binary stdin containing NUL/non-UTF-8, separated output channels, explicit guest failure, cache hit and scoped file access.

Negative cases include non-Unicode arguments, duplicate/missing entry, wrong entry signature, malformed Text, pointer/length overflow, input/output/memory limit, stale compiler/adapter cache identity, corrupt package/cache, unknown import, ungranted file/network/env access and Runtime timeout/trap.

Every negative case must identify whether failure occurred before compile, before execution, during host setup or in the guest. Tool diagnostics never contaminate guest stdout.

## AST and IR

M8 requires explicit IR types/operations for fixed-width integers, Text, Bytes, lists, nominal records and `Result` variants. The verifier owns type/layout preconditions before codegen. General dynamic calls and multi-block control flow must be emitted without compile-time constant assumptions.

Canonical ABI layout is implemented in one compiler module. Pointer/length/alignment arithmetic is checked. Script v0 may use one bounded arena per instance and reclaim it by dropping the Store; it does not claim GC or cross-instance references.

## Component/WIT mapping

The normative draft is [`wit/script-profile-v0/world.wit`](../../wit/script-profile-v0/world.wit). A Program Component exports `sico:script/program@0.1.0.run`.

The preferred packaging path composes a versioned adapter that converts WASI CLI args/stdin/stdout/stderr to/from the Program interface and exports a WASI command world. The adapter digest and WIT identity are package compatibility inputs. If composition fails its prototype gate, an in-process runner may call the same Program export directly; this does not change source semantics.

Manifest v0 continues to identify scalar `main()`. Script packages require a new strict manifest schema containing exact entry kind/world, language semantics, Script WIT, adapter and final Component identities.

## AI evaluation

The M8 corpus must measure whether a model can generate, explain and repair at least args echo, binary/text transform, word count, JSON filter and scoped file transform programs. Compiler diagnostics and Runtime faults remain separately identified. Offline fixtures may verify protocols but are not live-model quality evidence.

## Compatibility

- Existing scalar sources and manifest v0 packages remain valid under their original contract.
- Script WIT changes require a new semantic version and package compatibility value.
- Unknown newer manifests or WIT versions are rejected.
- Cached artifacts include compiler, semantics, WIT, adapter, source, profile and package identities.
- Buffered Script v0 remains a compatibility profile after M9 adds streaming.

## Security and privacy

Default Script execution has no environment, filesystem, network or process authority. Host paths are never embedded in guest-visible diagnostics unless explicitly requested for local debugging. Output and errors are bounded. Package/import/capability closure is checked after composition and before execution. Cache content is untrusted and reverified. Secrets are not included in cache names, build IDs, stdout, default stderr or evaluation logs.

## Alternatives

- Keep `sico-app dev` scalar-only: rejected because it does not satisfy scripting use cases.
- Embed compiler, package and Runtime in one crate: rejected because it violates accepted ownership boundaries and enlarges the trusted dependency graph.
- Directly expose all WASI resources to generated Sico code: deferred because it makes the first Script ABI depend on resource/async lowering.
- Use JSON strings as the only boundary: rejected as the normative interface because it loses binary stdin and moves basic type errors to Runtime.
- Implement top-level syntax first: rejected because it improves appearance without solving executable data and host boundaries.

## Validation and acceptance criteria

Acceptance requires the STEP-0076 vertical prototype, exact WIT parser/Component validation, randomized host/guest aggregate roundtrips, strict package/import closure, corrupted cache refusals, malicious guest limits, representative scripts and recorded cold/warm/RSS/artifact baselines. STEP-0075 froze the candidate bounds, exit mapping, capability names and cache encoding; STEP-0076 selected composition. The RFC remains proposed while the later compiler/package/runner/security gates are incomplete.

### STEP-0076 prototype result

The architecture gate returned `composition-go` on 2026-07-17. A deterministic 915-byte `sico:script/adapter@0.1.0` candidate canonical-lowers the Program function through private Adapter memory, invokes it through a core trampoline and canonical-lifts the Adapter export. Direct and composed paths each passed the same six cases in 20 independent runs on Wasmtime 46.0.1. Composed cold P95 was 15.8033 ms and the worst recorded per-run warm P95 was 0.2038 ms; the Adapter digest remained stable.

M8 therefore retains versioned Program/Adapter composition as the preferred path and direct Program invocation as a tested fallback. This RFC remains `proposed`: STEP-0076 did not verify compiler-generated Programs, randomized aggregate ABI, WASI CLI adaptation, manifest/package closure, caches, malicious-guest limits or representative standard-library scripts. Those existing acceptance conditions are not weakened by the architecture result.

### STEP-0077 fixed-width result

The compiler now preserves the numeric contract through semantic analysis, verified IR, dynamic Core Wasm and Component exports. Deterministic Node execution compared 2,048 generated operand pairs across eight signed/unsigned arithmetic and comparison paths; exact Wasmtime 46.0.1 execution passed 11 Component boundary cases. Overflow and underflow lift as real Component `result` errors through a private Canonical ABI return area, not as traps. General calls/control, aggregate ABI, Script packaging and runner integration remain later gates, so this result does not accept the RFC.

### STEP-0079 aggregate ABI result

The frozen layout table in `docs/development/script-canonical-abi-layout-v0.md` is derived from the normative WIT by the compiler itself. Generated Program Components now cross the exact `sico:script/program@0.1.0` boundary with Text, Bytes, `list<string>`, the Script records and the boundary `result` through one bounded per-instance arena with checked pointer/length arithmetic and deterministic `cabi_post_run` cleanup. Evidence: 10,000 seeded Wasmtime 46.0.1 roundtrips byte/value exact on one instance, a 512 × 1 MiB + 8 × 4 MiB repeated-call cleanup oracle, the 1,024-argument/8 MiB boundary case, and six malicious-memory fixtures failing closed (invalid discriminants, invalid UTF-8, out-of-bounds/overflowing pointers, misaligned result area); see `docs/reports/script-aggregate-canonical-abi-v0.md`. Compiler profile integration, adapter packaging, manifest v1 and runner integration remain STEP-0080–0082 gates, so this result does not accept the RFC.

### STEP-0080 compiler profile, adapter and manifest result

`sico build --profile script-v0` turns one source file with explicit ABI-validated Script declarations into a deterministic Program Component. The versioned `sico:script/adapter@0.1.0` (SHA-256 `185ebf1d393cb2b93c1d023f89d19d36c323c64f858a2fef1092f6a0283502c7`) composes it into a WASI `0.2.12` command whose echo/reject behavior was executed through the Wasmtime 46.0.1 CLI with real arguments and binary stdin. Strict manifest v1 (`sico.sapp.manifest.v1`) records the exact world, semantics `sico.ir.v0`, WIT digest and adapter digest, and the composed import closure maps exactly to `script.args`/`script.stdio` with unknown values failing closed; see `docs/reports/compiler-script-profile-adapter-manifest-v0.md`. WASI 0.2 exit semantics collapse guest exit values to 0/1 on the composed path; exact guest codes, the in-process runner, unified `sico run`/caches and the standard library remain STEP-0081–0083 gates, so this result does not accept the RFC.

### STEP-0081 runner result

The in-process `sico-runner` (`runner/sico-runner`, built with the MSVC toolchain because the workspace GNU toolchain cannot link the Wasmtime crate on the evidence host) invokes the Program export directly with one Store per call and no WASI context: no environment, filesystem or network is reachable by construction. Every RFC-0029 exit class is produced by a typed path — exact guest exits 0..=119 (out-of-range rejected), domain errors at 122, cancellation at 123, epoch timeout at 124, fuel/memory/trap at 125, launch at 126, incompatible artifacts at 127 — and malicious guests (trap, non-termination, memory ceiling, malformed results) leave the host healthy for subsequent calls; see `docs/reports/in-process-script-runner-v0.md`. Unified `sico run`/`eval`, caches and the standard library remain STEP-0082–0083 gates, so this result does not accept the RFC.

### STEP-0082 unified run, eval and cache result

`sico run FILE.sico -- ARGS` and `sico eval "EXPR"` now work end to end: compile through the Script profile, cache the Program Component under the frozen `SICO-SCRIPT-SOURCE-CACHE-V0` identity, and execute through `sico-runner` with exact channel and exit behavior. Repeat runs reuse byte-identical entries without recompilation, changed sources compile fresh, corrupt or conflicting entries fail closed, `run -` never feeds source stdin to the guest, `eval` accepts compile-time-constant `Int` expressions only, and diagnostics are JSON on stderr with stdout reserved for guest output; see `docs/reports/unified-run-eval-caches-v0.md`. The Wasmtime engine machine-code cache is deferred because its crate is unavailable in the offline dependency cache of the evidence host; the standard library and the M8 audit remain STEP-0083–0084 gates, so this result does not accept the RFC.

## Links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`M9 plan`](../plans/M9-streaming-async-interactive.md)
- [`STEP-0075`](../steps/STEP-0075-script-profile-contract.md)
- [`STEP-0076`](../steps/STEP-0076-script-profile-vertical-prototype.md)
- [`STEP-0077`](../steps/STEP-0077-fixed-width-dynamic-scalars.md)
- [`fixed-width evidence`](../reports/fixed-width-dynamic-scalars-v0.md)
- [`composition evidence`](../reports/script-profile-composition-v0.md)
- [`ADR-0007`](../adr/ADR-0007-openjdk-style-modular-monorepo.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
- [`RFC-0012`](./RFC-0012-component-wit-boundary-v0.md)
- [`RFC-0019`](./RFC-0019-package-cli-cache-stdio-v0.md)
