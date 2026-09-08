# M16 Native Automation Host

> Status: planned; plan refined 2026-09-07 at owner request after M14 GO; no STEP numbers reserved

## 1. Objective

Create a bounded Host capability layer for AI applications that observe an explicitly authorized native surface, compute a plan in Sico, preview or commit one action, then verify the resulting state.

The milestone succeeds when the full loop closes on a real Windows window of a controlled fixture app with every failure mode failing closed — and when nothing in the implementation can widen capture into input, or input into clipboard, credentials, process control or full-desktop access.

## 2. Entry gate

Per-condition status, measured 2026-09-07. Contract STEPs (threat model → WIT RFC → platform ADR) are themselves the first workstreams; platform implementation STEPs may not start until they are accepted.

| # | Condition | Status | Evidence / open gap |
|---|---|---|---|
| 1 | M14 is GO | **satisfied** | STEP-0141 (2026-09-06) |
| 2 | A threat model, capability/WIT RFC and platform adapter ADR are accepted | **OPEN by design** | none of the three documents exists yet; §5.1–§5.3 define their required content and exit tests |
| 3 | M10 identity/events/cancellation, M11 task accounting and M12 Host-provider isolation are reusable without widening them | **satisfied (evidence-carried)** | STEP-0102/0110 delivered typed cancellation, bounded events and scheduler accounting; STEP-0118/0125/0127 delivered provider isolation and grant shapes; M14's additive surface left M7/M12 security corpora untouched (STEP-0141 gate 5) |
| 4 | M15 is not a dependency | **satisfied** | shared Host contracts may be co-drafted but neither track gates the other's evidence |

## 3. Capability model

Four separately grantable capabilities; exact names provisional until the WIT RFC:

- `window.observe`: discover/bind only surfaces selected under Host policy; return stable, revocable identities. Identity must survive repaint/focus changes and invalidate on close/minimize/display change; revocation must be observable to an in-flight guest call.
- `screen.capture`: capture a bounded authorized surface/region with size, rate and total-frame ceilings; frames are owned guest-visible buffers with explicit stride/format, so the M17 image contract can consume them without re-definition.
- `input.pointer` / `input.touch`: scoped coordinates resolved against the *bound surface's current geometry* (never raw screen space), with duration and per-window action budgets.
- `input.keyboard` (optional, separate grant): printable/control distinction, per-window keystroke budget, and a sensitive-field refusal policy that fails closed.
- Preview/commit tokens bind surface identity, observation revision, action digest and expiration; single-use; Host-verified.
- Capture does not imply input; neither implies clipboard, filesystem, credentials, process control or full-desktop access. Grants compose only by explicit policy, never by inference.

## 4. Execution contract

```text
observe -> recognize/plan -> preview -> execute one bounded action -> observe -> verify or stop
```

- Verification semantics: the post-action observation must match the plan's expected state within a declared tolerance (region digest or declared feature check); mismatch is a typed outcome, never a retry by default.
- A changed surface identity or stale observation revision invalidates any outstanding token.
- Repeated unchanged frames, unexpected dialogs, timeout, cancellation or focus drift stop execution with typed outcomes.
- Dry-run is the default development mode and is enforced Host-side (guest code cannot distinguish or disable it).
- Every committed input emits a bounded, redacted audit event (action class, surface identity, digest, timestamps — never pixel or keystroke content by default).
- The Host owns emergency stop and teardown; guest code cannot suppress, defer or observe-around them.

## 5. Required workstreams

### 5.1 Threat model (document, pre-RFC)

Enumerate adversaries and abuse paths; each must map to a §3 capability boundary or a §6 non-goal **and to a testable refusal or corpus case**. Minimum coverage: hidden automation of the user's own session; credential/OTP harvesting via capture+input chains; clipboard/IME snooping via keyboard grants; review-fatigue abuse of preview dialogs (auto-accept patterns); telemetry or exfiltration of captured frames; anti-cheat/evasion pressure from fixture-app hardening; audit-log tampering from the guest side.

### 5.2 Capability WIT RFC

One interface per §3 capability, plus preview/commit token types, observation-revision handles, the audit-event stream and emergency-stop as a Host-owned resource. Composes RFC-0017 (capability closure) and RFC-0037 (grant shapes); reuses M10 session/cancellation identity. The RFC freezes the fail-closed mutation corpus: every authority-widening mutation attempt must be a typed refusal.

### 5.3 Platform ADR (Windows-first)

Must settle, with measured evidence: Win32 surface enumeration vs UIA tree — accessibility-tree *reading* is a §6 non-goal, and the ADR must draw that line explicitly; capture API choice (GDI vs DXGI) and its rate/size ceilings; input injection mechanism (SendInput) and its auditability; how M11 task cancellation reaches in-flight OS input calls; dry-run enforcement point; and process isolation of the adapter (native FFI stays in the smallest isolated adapter/provider crate per the architecture boundary — no platform-specific unsafe in compiler or shared security cores).

### 5.4 Fixture app and synthetic adapter

- Fixture app contract: a small, openly licensed Windows app with deterministic UI, scriptable state, and no anti-automation hardening; selected and recorded at kickoff.
- Synthetic adapter: the full capability contract implemented against a simulated surface, so every RFC-level guarantee is testable without OS access. Evidence class is `contract-verified` and is never advertised as runtime support.

### 5.5 Windows real path

Observe → capture → preview → execute-one → verify on the fixture app; drift/stale-revision/revocation/duplicate-commit/limit+1 corpora; cancellation and Host-crash recovery with orphan-input and orphan-resource checks; long-running dry-run and bounded action runs with recorded handle/RSS/event behaviour.

## 6. Non-goals

- Full-desktop surveillance, hidden background keystrokes or ambient accessibility-tree scraping.
- Password-manager, authentication, CAPTCHA, paywall, security-warning or anti-cheat bypass.
- Arbitrary process injection, shell execution or raw OS handles.
- General CV/ML algorithms; those belong to M17.
- Web/page automation; that belongs to M15's event and authority model.

## 7. Exit gates

1. Real Windows observe/preview/execute-one/verify loop succeeds against the controlled fixture app (raw logs committed, app version pinned).
2. Surface drift, stale revision, permission revocation, duplicate commit and limit+1 all fail closed with typed outcomes.
3. The capture/input authority mutation corpus cannot widen scope; every §5.1 threat has a corresponding test.
4. Cancellation and Host crash recovery leave no orphan input or resource.
5. Long-running dry-run and bounded action runs have recorded handle/RSS/event behavior within declared budgets.
6. Only native-tested platforms are advertised; `contract-verified` synthetic results are labelled as such.
7. Full M0–M14 regression is green and the exit audit is explicit (GO/NO-GO with per-gate evidence).

## 8. Platform sequence

1. Contract and synthetic adapter (after §5.1–§5.3 are accepted).
2. Windows real-window capture and pointer path (§5.5).
3. Windows lifecycle/security/performance audit.
4. macOS/Linux/Android only when licensed toolchains and native runners provide equivalent evidence.

## 9. Sequencing rule

No implementation STEP is reserved by this plan. The entry order is:

1. First STEP: kickoff inventory + threat-model draft (§5.1), mirroring the STEP-0129/RFC-0038 pattern; the owner accepts before contract freezing.
2. The capability WIT RFC (§5.2) and platform ADR (§5.3) follow; shared Host grant shapes may be co-drafted with M15 but freeze independently.
3. The synthetic adapter implements the frozen contract before any OS-facing code.
4. Each subsequent STEP closes one measured loop gap and reruns the cumulative fail-closed corpora; regression stays green throughout.
