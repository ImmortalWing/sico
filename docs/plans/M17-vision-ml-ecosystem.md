# M17 Vision and model package ecosystem

> Status: planned; plan refined 2026-09-07 at owner request after M14 GO; no STEP numbers reserved

## 1. Objective

Provide deterministic image-processing packages and optional bounded model inference through the versioned package/Component ecosystem. Vision is not core syntax and model inference is not an implicit dependency.

The milestone succeeds when the Russian-tetris recognition chain — grid and piece recognition over captured frames — runs as deterministic packages with a fixed oracle, and when every accelerated or model-backed path is an explicitly granted, budgeted, provenance-bound consumer that falls back honestly.

## 2. Entry gate

Per-condition status, measured 2026-09-07. Contract preparation (§3.1/§3.2 sketches) may run in parallel with M16 now; implementation STEPs may not start until every condition is satisfied.

| # | Condition | Status | Evidence / open gap |
|---|---|---|---|
| 1 | M14 is GO | **satisfied** | STEP-0141 (2026-09-06) |
| 2 | M16 has at least an accepted read-only capture prototype and frozen image ownership/size limits | **OPEN** | M16 contracts are not yet drafted (M16 plan §5.1–§5.3); the image contract sketch is deliberately producer-agnostic so it can be drafted in parallel, but capture-consuming corpora wait for M16's prototype |
| 3 | M7 package identity, dependency lock, update and provider trust can carry binary implementations and model assets | **PARTIAL — contract level only** | RFC-0015/0024/0025/0026 define format, signing, update and lock; carrying large binary/model assets (size ceilings, chunked digest, memory-bounded load) is unproven and needs an explicit measured STEP before any provider/model package ships |
| 4 | Native/GPU providers require an ADR and actual runner evidence | **OPEN by design** | §3.3 defines the ADR's decision axes; no provider claim exists today |

## 3. Required workstreams

### 3.1 Portable data contract (RFC)

- Candidate types: `Image { width, height, stride, format, space, orientation }`, `Region`; owned/borrowed pixel-buffer rules; exact allocation ceilings (dimensions × bytes-per-pixel bounds checked at construction; allocation failures typed).
- Decisions the RFC must settle: packed vs planar default; orientation as metadata vs normalized layout; color-space conversion as explicit op vs implicit normalization (implicit conversion is contrary to house style — default explicit).
- Canonical serialization only where copying is explicitly affordable; no hidden zero-copy across the Component boundary.
- Alignment with M16: capture frames must convert to this contract without a second image definition (M16 plan §3 `screen.capture` already commits to owned stride/format buffers).

### 3.2 Deterministic vision packages (RFC + corpus)

Package roster (provisional until the RFC): color conversion, thresholding, resize/crop, bounded convolution, template match, grid detection, contour primitives.

- Every package ships with: a fixed input corpus, a Python/numpy fixed comparison oracle (the M14 solver-oracle pattern), an explicit numeric tolerance policy (exact-integer vs tolerance-labelled, never "approximate"), and **content-asserting e2e fixtures** — the M14 lesson (STEP-0131 forensics) is that count-only assertions let element contents break silently.
- Determinism contract: same input bytes + same package version → identical output bytes, on every supported platform; any platform-divergent fast path is refused or tolerance-labelled.

### 3.3 Optional accelerated providers (ADR)

- OpenCV or equivalent behind a versioned Component/Host provider; the ADR settles: isolation mechanism (separate process vs separate Store), budget-enforcement point (Host-side, guest-invisible), crash/OOM containment, and the result-comparison protocol against §3.2 references within frozen tolerances.
- No provider may receive broader filesystem, network, screen or input authority than its caller grants; provider grants compose only by explicit policy.
- Performance claims name exact hardware, provider version and runner; untested hardware remains unsupported.

### 3.4 Model inference packages (RFC + contract)

- Model/weight digest, provenance record and format/version identity; operator allowlist; budgets as explicit, enforceable fields (CPU/GPU/memory/time/output-size) with typed out-of-budget outcomes.
- Deterministic or tolerance-labelled results, and a non-model fallback whenever the application permits one; the fallback path is part of the acceptance corpus, not a footnote.
- No automatic download, no telemetry, no ambient credential access — restated in the RFC as testable refusals, and model assets enter only through the M7 trust/update boundary with explicit owner authority for each specific model.

### 3.5 Deterministic-first rule

Every acceptance task in §4 must first pass with deterministic packages alone. A model path may be added only when measured evidence justifies it, and the record must show both paths' results side by side. Deterministic vision is the default; "the model handles it" is never an exit argument.

## 4. Acceptance corpus

- Synthetic and captured grid boards across scale, color and compression variants; the captured class comes from M16's fixture-app frames once its prototype exists.
- Hostile dimensions/stride/metadata and decompression-bomb cases (limit+1 in every axis; typed refusals).
- Reference vs accelerated result comparison within frozen tolerances.
- Model corruption, unsupported operator, cancellation and out-of-budget cases fail closed and leave the Host reusable.
- End-to-end consumer: the tetris recognition chain as a source-level package consumer — which also exercises M15 §3.0's modules/user-WIT track if M15 has landed it by then; otherwise the Component-level consumer path is used and the source-level gap stays declared.

## 5. Non-goals

- Adding image, OpenCV, GPU or model primitives to core language syntax.
- Giving vision packages ambient screen, filesystem, network or input authority.
- Making a model mandatory when deterministic geometry/color algorithms satisfy the task.
- Claiming cross-platform acceleration without hardware-specific runner evidence.
- Absorbing M16's capture/input authority into vision packages.

## 6. Exit gates

1. The portable baseline completes the fixed corpus without model inference, oracle-matched and content-asserted.
2. Accelerated paths match reference results within frozen tolerances on named hardware/provider/version.
3. Package consumers do not modify compiler/Runtime core (a pilot needing a core patch fails here, not in M18).
4. Model and provider authority remains explicit and least-privilege; every §3.4 refusal corpus case fails closed.
5. Resource, crash and malicious-asset corpora fail closed and leave the Host reusable.
6. Performance claims identify exact hardware/provider/version with raw evidence.
7. Full M0–M16 regression is green and the exit audit is explicit (GO/NO-GO with per-gate evidence).

## 7. Sequencing rule

No implementation STEP is reserved by this plan. The entry order is:

1. First STEP: kickoff inventory + the image data-contract RFC draft (§3.1), started in parallel with M16's ADR because the contract is producer-agnostic; the owner accepts before freezing.
2. The §3.2 package RFC and its per-package corpora freeze next; deterministic packages need only §3.1 plus the §2.3 binary-carrying proof — not M16.
3. The §3.3 provider ADR and §3.4 model RFC follow; both additionally require M16's capture prototype for their captured-frame corpora.
4. Each subsequent STEP closes one measured package gap and reruns the cumulative oracle; regression stays green throughout.
