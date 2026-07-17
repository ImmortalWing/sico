# STEP-0079: Script aggregate Canonical ABI

> - status: planned
> - phase: M8
> - started: -
> - completed: -
> - owners: autonomous-agent

## 1. Objective

Define and implement the bounded Script v0 aggregate boundary so Text, Bytes, lists, records and general Result values can cross between generated Program Components and the versioned Script Adapter without treating STEP-0078's internal locals as a public ABI.

## 2. Required order

1. Freeze one WIT-driven layout table for string/list/record/variant/result, including alignment, discriminants, ownership and post-return cleanup.
2. Define the per-instance bounded arena and checked pointer/length arithmetic before emitting any load/store path.
3. Add independent lift/lower helpers with invalid UTF-8, overflow, misalignment, out-of-bounds and double-cleanup mutation tests.
4. Implement Text and Bytes first, then lists, records and Result; do not start adapter packaging until host/guest randomized roundtrips pass.
5. Integrate the exact `sico:script/program@0.1.0` boundary and only then hand off to STEP-0080 manifest/composition work.

## 3. Included

- strict UTF-8 Text and arbitrary Bytes;
- bounded `list<Text>` plus the record/Result shapes required by `ScriptInput`, `ScriptOutput` and `ScriptError`;
- checked allocation, alignment, pointer/length arithmetic and deterministic cleanup;
- Component validation, randomized host/guest roundtrips and malformed-memory refusals;
- exact WIT identity and deterministic artifact snapshots.

## 4. Excluded

- arbitrary nested generics or a general GC;
- cross-instance references and long-lived borrowed guest memory;
- adapter composition, manifest v1 and package closure (STEP-0080);
- in-process runner and WASI host policy (STEP-0081);
- streaming I/O, async, HTTP, watch mode and REPL (M9).

## 5. Exit gate

- every accepted aggregate shape has one documented Core and Canonical ABI representation;
- 1,024 arguments, 64 KiB per argument and 1 MiB total argument bytes obey the M8 bounds;
- invalid UTF-8, arithmetic overflow, invalid tag, misalignment and out-of-bounds memory fail closed;
- at least 10,000 seeded boundary roundtrips are byte/value exact in the host and Wasmtime;
- allocation cleanup is deterministic and repeated calls do not grow memory beyond the declared arena policy;
- STEP-0078 scalar/call/CFG behavior and the full workspace regression remain green.

## 6. Links

- [`STEP-0078`](./STEP-0078-general-executable-control-codegen.md)
- [`STEP-0078 evidence`](../reports/general-executable-codegen-v0.md)
- [`M8 plan`](../plans/M8-script-profile.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`script WIT`](../../wit/script-profile-v0)
