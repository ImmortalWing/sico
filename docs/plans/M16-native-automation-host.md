# M16 Native Automation Host

> Status: planned; owner-approved 2026-09-04; no STEP numbers reserved

## 1. Objective

Create a bounded Host capability layer for AI applications that observe an explicitly authorized native surface, compute a plan in Sico, preview or commit one action, then verify the resulting state.

## 2. Entry gate

- M14 is GO.
- A threat model, capability/WIT RFC and platform adapter ADR are accepted before implementation.
- M10 identity/events/cancellation, M11 task accounting and M12 Host-provider isolation are reusable without widening them.
- M15 is not a dependency; both tracks may proceed after shared Host contracts are frozen.

## 3. Capability model

- `window.observe`: discover/bind only surfaces selected under Host policy; return stable, revocable identities.
- `screen.capture`: capture a bounded authorized surface/region with size/rate limits.
- `input.pointer` and `input.touch`: scoped coordinates, duration and action budgets.
- Optional `input.keyboard`: separate grant, printable/control distinction and sensitive-field refusal policy.
- Preview/commit tokens bind surface identity, observation revision, action digest and expiration.
- Capture does not imply input; input does not imply clipboard, filesystem, credentials, process control or full-desktop access.

Exact names are provisional until RFC acceptance.

## 4. Execution contract

```text
observe -> recognize/plan -> preview -> execute one bounded action -> observe -> verify or stop
```

- A changed surface identity or stale observation invalidates the action.
- Dry-run is the default development mode.
- Repeated unchanged frames, unexpected dialogs, timeout, cancellation or focus drift stop execution.
- Every committed input emits bounded, redacted audit events.
- The Host owns emergency stop and teardown; guest code cannot suppress them.

## 5. Platform sequence

1. Contract and synthetic adapter.
2. Windows real-window capture and pointer path.
3. Windows lifecycle/security/performance audit.
4. macOS/Linux/Android only when licensed toolchains and native runners provide equivalent evidence.

## 6. Non-goals

- Full-desktop surveillance, hidden background keystrokes or ambient accessibility-tree scraping.
- Password-manager, authentication, CAPTCHA, paywall, security-warning or anti-cheat bypass.
- Arbitrary process injection, shell execution or raw OS handles.
- General CV/ML algorithms; those belong to M17.

## 7. Exit gates

1. Real Windows observe/preview/execute-one/verify loop succeeds against a controlled fixture app.
2. Surface drift, stale revision, permission revocation, duplicate commit and limit+1 all fail closed.
3. Capture/input authority mutation corpus cannot widen scope.
4. Cancellation and Host crash recovery leave no orphan input or resource.
5. Long-running dry-run and bounded action runs have recorded handle/RSS/event behavior.
6. Only native-tested platforms are advertised.
7. Full M0–M14 regression is green and the exit audit is explicit.
