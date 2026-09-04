# M14 Application-ready language baseline

> Status: planned; owner-approved 2026-09-04; no STEP numbers reserved

## 1. Objective

Close the gap between accepted Sico source semantics and code that can actually build and run. “Application-ready” is a measurable compatibility baseline, not a claim that language evolution ends.

The milestone succeeds when ordinary application algorithms can stay in Sico and Host providers are needed only for explicit external capabilities.

## 2. Entry gate

- M12 has a full GO, including guest-visible provider integration and required platform parity.
- M13 has issued its AI-workflow closure verdict.
- The M0–M13 regression chain is reproducible.
- Every currently accepted source feature is classified as executable, intentionally profile-restricted, or proposed; no implicit support claim is allowed.

Missing production identity, public deployment and mobile runners remain external tracks and do not gain evidence from M14.

## 3. Required workstreams

### 3.1 Source/backend parity contract

- Publish a machine-readable check/build/run support matrix by language profile.
- Give every backend refusal a stable feature identity and source span.
- Prevent user documentation from describing check-only constructs as executable.
- Decide, through RFCs, which accepted constructs must enter the application baseline and which remain outside it.

### 3.2 General algorithm execution

- Structured loops and iteration with explicit termination/cancellation points.
- Runtime-bounded recursion or an evidence-backed rejection with an equivalent iteration model.
- Dynamic List/Map/Set traversal and construction with deterministic order rules.
- Fixed-width arithmetic, comparisons, checked conversions and bit operations suitable for bitboards.
- User records/enums/Result/Option across normal function and collection boundaries.
- Predictable allocation failure, stack/fuel/time limits and source-level faults.

### 3.3 Modules, packages and interfaces

- Source modules/imports and versioned package resolution use the M7 trust/lock boundary.
- Compiler-facing user-defined WIT/Component imports and exports.
- Generated bindings preserve nominal types, Result, resources, async and capability/effect facts.
- No native FFI or package build script may bypass Component/Host authority.

### 3.4 Application development loop

- A real `sico test` or an accepted equivalent with deterministic discovery and bounded execution.
- Fixtures, property tests, benchmark inputs and failure snapshots.
- LSP, DAP, semantic index and AI tools understand the newly executable constructs.
- Build/run/watch/cache identities remain consistent across modules and packages.

## 4. Representative acceptance applications

1. **Offline block-game solver:** 8×8 board, dynamic three-piece tray, legal placement enumeration, simultaneous row/column clearing, deterministic search and board scoring. All model and solver logic must be Sico; JSON or typed Component input/output is allowed.
2. **Streaming transform:** bounded memory independently of total input size, with cancellation and typed partial failure.
3. **Capability-backed state machine:** explicit external effect, revision guard and deterministic recovery without native business logic.

The existing Python game solver is an oracle and migration fixture, not acceptable M14 implementation evidence.

## 5. Non-goals

- DOM or browser hosting.
- Desktop/window capture or input injection.
- OpenCV, GPU, model runtimes or model weights.
- General native FFI, shell or ambient process authority.
- Declaring every future language feature complete.

## 6. Exit gates

1. No undeclared check/build/run mismatch in the application profile.
2. All three acceptance applications run from Sico source through a real Component Runtime.
3. The offline solver matches the fixed Python oracle corpus and completes within recorded, non-invented budgets.
4. Limit+1 inputs fail with typed outcomes and leave the Host reusable.
5. Module/package/WIT security mutation corpora fail closed.
6. Windows and Linux native runner evidence exists for the portable core; other platforms remain honestly labelled.
7. AI generation, repair and comprehension regression is measured under the M13 protocol.
8. Full M0–M13 regression is green and an exit audit issues an explicit GO/NO-GO.

## 7. Sequencing rule

No implementation STEP is reserved by this plan. The first future STEP must be an inventory/RFC that freezes the application profile and exit corpus before code changes. Each subsequent STEP must close one measured source/backend gap and rerun the cumulative oracle.
