# ADR-0009: Versioned Script adapter and isolated in-process runner

> - status: proposed
> - date: 2026-07-17
> - owners: autonomous-agent
> - supersedes: external Wasmtime CLI as the eventual Script production runner only after acceptance
> - superseded-by: -

## Context

The current Runtime writes an authorized Component to a temporary file and launches Wasmtime CLI with `--invoke main()` and `cache=n`. Fault classification partly interprets Runtime stderr. This is adequate evidence for the current scalar boundary but cannot robustly provide bounded binary stdin/stdout, typed faults, persistent engine/cache reuse or future streaming cancellation.

The accepted modular architecture forbids the compiler CLI from depending on package, Runtime or Host crates and forbids `sico-app` from normally depending on compiler crates. Script convenience must preserve those ownership boundaries.

## Decision drivers

- typed and stable fault handling;
- portable Component/WIT contracts;
- default-deny host authority;
- compiler/Runtime modularity;
- cold and warm script latency;
- deterministic packaging and cache invalidation;
- a compatible path from buffered M8 to streaming/async M9;
- independent conformance tests.

## Considered options

### External Wasmtime CLI for all Script execution

This preserves the existing small Rust dependency graph and is useful for a vertical prototype. It retains process startup, temporary files, CLI-text fault coupling and weaker control over streaming and Store lifecycle. It remains a diagnostic/fallback surface, not the preferred final M8 runner.

### Compiler CLI embeds Wasmtime

This minimizes process hops but violates command/module ownership, substantially enlarges the compiler binary and mixes source diagnostics with package trust and Host policy. Rejected.

### `sico-app` directly depends on compiler crates and Wasmtime

This creates a convenient monolith but contradicts ADR-0007 and makes compiler/runtime changes harder to audit independently. Rejected.

### Versioned adapter plus separate `sico-runner`

The compiler emits a Program Component against a small WIT. `sico-app` composes/packages/verifies it with an exact adapter. A separate Runtime-owned runner embeds Wasmtime/WASI, uses structured APIs and receives only an already verified/authorized execution request. Accepted as the proposed target, subject to STEP-0076 evidence.

## Decision

M8 will prototype and, if accepted, implement:

```text
sico compiler process
  -> Program Component
sico-app process
  -> adapter composition, manifest/package, trust/capability
sico-runner process or library boundary
  -> Wasmtime Engine/Component/Linker/WASI/Store
```

`sico run` is a user-facing dispatcher and may invoke sibling tools, but `sico-cli` gains no normal dependency on package, Runtime or Host crates. `sico-app` continues to invoke the compiler through a process boundary. `sico-runner` belongs to the Runtime module and does not parse source.

The adapter is versioned and content-addressed. Package compatibility records its WIT and digest. The final runner creates a fresh Store, resource table, WASI context and capability set per execution. Engine, Linker and compiled Component artifacts may be reused only where identity and isolation are proven.

The external Wasmtime 46.0.1 CLI remains the STEP-0076 prototype oracle and optional diagnostic fallback. It is not removed until the in-process runner has equivalent limit, fault and host-survival evidence.

## Consequences

Positive:

- Runtime failures become typed Wasmtime/Host outcomes instead of stderr string patterns;
- stdin/stdout and future async cancellation can be supervised directly;
- machine-code cache and Engine/Linker reuse become controllable;
- compiler ownership and a portable Program WIT remain intact;
- adapter changes are explicit compatibility events.

Costs:

- a new Runtime crate/binary and large exact Wasmtime dependencies;
- SDK binary size and build time increase;
- package manifest and architecture validator require updates;
- adapter composition adds one compatibility and security surface;
- external CLI and embedded runner require temporary parity coverage.

## Validation

STEP-0076 must first demonstrate the Program/Adapter contract under the selected Wasmtime CLI. Before this ADR can be accepted, the embedded runner must prove typed timeout/fuel/memory/trap/capability results, bounded stdio, no default env/fs/network, cache mutation refusal, fresh-Store isolation and host survival after malicious guests. Binary size, cold/warm latency and build-time effects are recorded rather than assumed.

### STEP-0076 architecture gate result

STEP-0076 returned `composition-go` on 2026-07-17 using the Wasmtime 46.0.1 Component API. The Adapter avoids unsupported imported-function re-export by canonical-lowering the Program function into private Adapter memory, invoking a core trampoline and canonical-lifting its own `run` export. An outer Component explicitly instantiates and wires Program + Adapter.

Direct and composed paths each passed 20 × 6/6; composed cold P95 was 15.8033 ms, the worst recorded per-run warm P95 was 0.2038 ms, and the 915-byte Adapter had one stable digest. The preferred architecture is therefore retained and direct invocation remains the fallback. This ADR stays `proposed` until the embedded runner and security evidence listed above are complete; the prototype does not yet include WASI CLI adaptation, package closure or production Runtime limits.

## Revisit conditions

- Component composition cannot satisfy the WIT or performance gate;
- embedded Wasmtime adds unacceptable SDK/build cost relative to typed-control benefits;
- a stable Wasmtime CLI machine-readable protocol replaces the need for embedding;
- M9 streaming/async requirements cannot reuse the proposed Store/adapter model;
- platform evidence shows materially different behavior that the adapter cannot isolate.

The fallback is direct runner invocation of the same Program WIT, not a compiler/runtime monolith.

## Links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`M9 plan`](../plans/M9-streaming-async-interactive.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`STEP-0075`](../steps/STEP-0075-script-profile-contract.md)
- [`STEP-0076`](../steps/STEP-0076-script-profile-vertical-prototype.md)
- [`composition evidence`](../reports/script-profile-composition-v0.md)
- [`ADR-0007`](./ADR-0007-openjdk-style-modular-monorepo.md)
