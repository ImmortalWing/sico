# M15 Web platform and UI controls

> Status: planned; plan refined 2026-09-07 at owner request after M14 GO; no STEP numbers reserved

## 1. Objective

Provide a first-class Sico Web/UI path without introducing JavaScript semantics into the language or duplicating Runtime security rules.

The milestone succeeds when one real business component — not a demo — runs with equivalent core behavior on a declared Web Host and the native Host, and when every web-facing authority (DOM, network, storage, events, lifecycle) is an explicit, least-privilege grant composed from the existing capability primitives (M5 identity, M10 events/cancellation/redaction, M12 endpoint policy, M7 trust boundary).

## 2. Entry gate

Per-condition status, measured 2026-09-07. Implementation STEPs may not start until every condition is satisfied; contract/prototype work under §8 may proceed now.

| # | Condition | Status | Evidence / open gap |
|---|---|---|---|
| 1 | M14 application-ready language baseline is GO | **satisfied** | STEP-0141 (2026-09-06): exit gates 1–6 and 8 GO |
| 2 | M13 AI tooling covers application-profile source | **satisfied at M13's deliverable level** | STEP-0121/0122 tooling covers accepted source; the M14-profile live-model re-measure is M14 exit-gate 7 (`blocked-external-evidence`) — a recorded parallel follow-up that must appear in any M15 AI-tooling claim, not silently inherit the 0.9095 number |
| 3 | At least one verifiable application/package consumer exists outside compiler implementation fixtures | **OPEN — closure path decided (owner, 2026-09-07)** | The owner chose to satisfy the condition rather than re-scope it: the §3.0 track exit test must be delivered as a `clean-room-consumer`-class application (requirements in §3.0). M14's three acceptance applications are `internal-fixture` class and do not count; the lower support claim is kept until the consumer evidence is recorded. |

## 3. Required workstreams

### 3.0 Prerequisite track: source modules, package resolution and user WIT (M14 §3.3 carry-over)

The compiler-facing UI/WIT bindings this milestone depends on require source-level modules/imports, versioned package resolution and user-defined WIT imports/exports. These were classified outside RFC-0038's frozen profile and are **not delivered by M14** (STEP-0141 scope-honesty note). M15 must not build browser contracts on an unclaimed foundation, so this track closes the gap first, under its own RFC:

- Source modules/imports resolved through the M7 trust/lock boundary (RFC-0015/0024/0025/0026 shapes; no build scripts, no native FFI escape).
- Compiler-facing user WIT imports/exports composing RFC-0012 (Component/WIT boundary) and RFC-0013 (async/resource mapping): generated bindings preserve nominal types, Result, resources, async and capability/effect facts.
- Typed refusals with stable feature identity and source span for everything outside the frozen binding surface; the machine-readable matrix (`tests/language-matrix/`) gains a bindings profile.
- Track exit test: a source-level consumer imports a versioned package exporting a user WIT interface and calls it through a real Component Runtime; limit+1 and security mutation corpora fail closed.
- Owner decision (2026-09-07): this exit test doubles as the §2 condition-3 evidence and must therefore be delivered at `clean-room-consumer` class — a separate code path/directory outside `tests/` and `examples/`, developed against published documentation rather than fixture internals, with no compiler/Runtime patches, a bounded feature list frozen by the kickoff STEP, and raw evidence committed. The evidence record must state the same-author limitation honestly: this class proves independent code-path pressure, not independent third-party validation.

### 3.1 Web hosting ADR

Decides the execution substrate and its security consequences. Each decision axis must be settled with measured evidence, not preference:

- Substrate: browser-native Wasm Component execution (engine-native component-model support vs a JS canonical-ABI shim) vs a controlled webview Host vs staged coexistence.
- HTTP authority: how RFC-0037 endpoint/DNS policy, trust roots and opaque secrets map onto the chosen substrate (browser-managed TLS and credential attachment differ from the M12 provider; per-Store connection pooling may be unrepresentable — declare, do not approximate).
- Identity: M5/M10 Host identity vs origin/CSP; what the M10 debug triplet means in a browser.
- Store lifetime: per-load Stores vs worker/service-worker persistence; cancellation and M11 scheduler semantics across page lifecycle.
- Limits: where fuel/stack/memory limits are enforced when the Runtime is not wasmtime-in-process; typed `RunOutcome`s must not silently degrade to opaque engine traps.
- Rendering: DOM-backed controls vs canvas, and the accessibility-tree consequences of that choice.
- The ADR names the exact browsers/engines/versions every later platform claim will require.

### 3.2 UI control and event contract (RFC)

A deterministic, AI-writable control tree with stable identity — not ambient DOM ownership.

- v0 widget set (provisional): container, text, button, text input, list, image, plus layout containers; every name frozen only by the RFC.
- Layout: a deterministic v0 subset (stacked/flowed/grid with fully specified rules); CSS-parity layout is explicitly future scope until a measured need exists.
- Events: typed event structs, deterministic dispatch order, no ambient listeners; event payloads follow M10 redaction rules.
- State: explicit property assignment only in v0; no reactive/framework semantics.
- Accessibility: roles, names and focus order as first-class contract fields, mapped to ARIA when hosted in a browser; IME composition behaviour labelled per engine with no silent fallback.

### 3.3 Host authority surfaces

- DOM: scoped root grant (a bounded subtree), never ambient document ownership; hostile-content escaping contract — guest strings never become unchecked HTML, script, URL, path or command.
- Network: M12 endpoint policy re-used over the ADR-chosen substrate; endpoint-scoped grants; secrets stay opaque and Host-side.
- Storage: scoped key/value with existing storage-provider semantics and a per-origin isolation statement.
- Lifecycle: page lifecycle events, visibility change and unload cancellation composed with the M10/M11 cancellation trees.

### 3.4 Debuggability, limits and evidence infrastructure

- Source identity preserved in browser tooling or a DAP bridge; any degradation of the M10 debug triplet is declared, not hidden.
- Resource limits enforced and typed on every supported substrate.
- The "same component, same behavior" cross-host matrix (start from `tests/end-to-end/*`) becomes an executable corpus with a validator, mirroring the `validate-step-0131.ps1` pattern.
- A real-browser automation harness (engine and version pinned) produces the exit evidence; webview-prototype results are never renamed as browser support.

## 4. Non-goals

- Treating Sico as JavaScript or HTML syntax sugar.
- Ambient browser cookies, unrestricted DOM ownership or arbitrary script injection.
- Desktop screen automation; that belongs to M16.
- Inferring browser support from a webview prototype.
- CSS-parity layout, reactive state frameworks or full DOM API surface in v0.

## 5. Exit gates

1. One business component runs with equivalent core behavior in a declared Web Host and the native Host; the cross-host matrix is green on both and committed as evidence.
2. UI state/event/accessibility behavior is deterministic under the frozen contract, verified by corpus and validator.
3. Navigation, network, storage and user-input authority are explicit and least-privilege; authority mutation corpora fail closed.
4. Hostile markup/URL/event/size/rate corpora fail closed.
5. Actual-browser performance, lifecycle and crash-isolation evidence exists (named engines/versions, raw logs committed).
6. Platform claims name exact tested engines and versions; untested engines remain explicitly unsupported.
7. Full M0–M14 regression is green and the exit audit is explicit (GO/NO-GO with per-gate evidence).

## 6. Dependencies and parallelism

- §3.0 blocks all binding-dependent work; §3.1 blocks all substrate work. Corpus and hostile-input work may proceed once their RFCs freeze.
- M16 shares Host grant/permission shapes (RFC-0037 closure) and M10 identity/cancellation/redaction; shared shapes may be frozen jointly, but neither track gates the other's platform evidence.
- M14 exit-gate 7 (live-model regression on the M14 profile) is an owner-gated parallel follow-up; M15 planning does not wait for it, and no M15 AI claim may cite pre-M14 numbers.

## 7. Sequencing rule

No implementation STEP is reserved by this plan. The entry order is:

1. First STEP: an inventory that measures the open §2 conditions and drafts the §3.0 RFC (mirroring the STEP-0129/RFC-0038 pattern); the owner accepts before any code changes. The §3.0 clean-room consumer is developed in parallel with RFC review and must be recorded before the first implementation STEP starts.
2. The §3.1 ADR follows; no substrate implementation before acceptance.
3. The §3.2 RFC and the hostile-content corpus freeze before any control implementation.
4. Each subsequent STEP closes one measured cross-host gap and reruns the cumulative cross-host matrix; regression stays green throughout.

## 8. Preparation status

The pre-entry workstream sketches from the 2026-09-04 revision (ADR-inputs memo, binding sketch, corpus skeleton, dependency note) were never executed as STEPs and produced no support claim; their intent is absorbed into §3.1 (decision axes), §3.0/§3.2 (binding and control contracts) and §3.4 (corpus). Contract preparation may start immediately under §2's contract-only rule.
