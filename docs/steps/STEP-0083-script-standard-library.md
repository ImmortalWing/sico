# STEP-0083: minimal useful script standard library

> - status: complete for the executed pilot corpus; intrinsic-completeness claim corrected by STEP-0203
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-18
> - owners: autonomous-agent

## 1. Objective

Make the Script profile useful for representative AI automation without native escape hatches: `sico.text`, `sico.bytes`, `sico.list` primitives, bounded `sico.json` parse/stringify with a documented number-precision contract, and `sico.fs` scoped file access through Host-granted roots — proven by the args-echo, word-count, JSON-filter and scoped-file-transform pilots running through `sico run` without compiler/Runtime modifications per script.

## 2. Architecture

- Pure-computation primitives (`sico.text`/`sico.bytes`/`sico.list`) are IR `Intrinsic` operations from a closed, versioned registry. The verifier pins every signature; the backend emits deterministic helper core functions into the generated module, reusing the STEP-0079 bounded arena and checked arithmetic. No host authority is involved.
- `sico.json` is a bounded guest-side parser/stringifier with a documented number-precision contract (exact `I64` integers and decimal text round-trip; no floating point).
- `sico.fs` is a capability: the versioned `sico:script/fs-read@0.1.0` and `sico:script/fs-write@0.1.0` interfaces implemented by the runner against Host-granted roots, gated by the RFC-0029 `storage.read`/`storage.write` capabilities (two interfaces so the read/write split is expressible at import granularity). Default remains no filesystem.
- map/filter/fold require closures and are explicitly deferred; top-level statements, REPL and streaming stay M9.

## 3. Included

- text: length, concat, trim, contains, starts_with, split_lines, split_words;
- bytes: length, concat, slice (checked), utf8_decode (validated), text.encode;
- list: length, get (checked), append;
- `sico.u64.to_text` / `sico.i64.to_text`;
- JSON bounded parse/stringify and number-precision contract;
- scoped fs through the capability channel;
- the four pilots.

## 4. Excluded

- closures and map/filter/fold;
- arbitrary filesystem/environment/process authority;
- streaming/async (M9);
- changing Script WIT or the frozen aggregate ABI.

## 5. Exit gate

- every intrinsic has verifier-pinned signatures, deterministic emission and misuse refusals;
- the four pilots run through `sico run` and produce exact expected outputs;
- invalid UTF-8, out-of-range indices/slices and oversized JSON fail closed;
- scoped fs outside granted roots is refused;
- STEP-0077–0082 evidence and the full workspace regression remain green.

## 6. Links

- [`M8 plan`](../plans/M8-script-profile.md) (§8 standard-library minimum)
- [`STEP-0079 evidence`](../reports/script-aggregate-canonical-abi-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)

## 7. Executable-evidence correction (STEP-0203)

`sico.text.replace` has a semantics/IR signature but no backend emission
case. A real `sico build --profile script-v0` therefore returns typed
`unsupported stdlib intrinsic`. The four STEP-0083 pilots did not execute
that intrinsic, so their evidence remains valid, but the earlier statement
that every listed intrinsic had deterministic emission was too broad.
`text.replace` is now recorded as check-accepted/build-refused in the
application support matrix; it is not executable support.
