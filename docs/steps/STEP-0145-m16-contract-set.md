# STEP-0145: M16 contract set — threat model, capability WIT RFC, platform ADR

> - status: complete (documents drafted; owner acceptance pending per M16 plan §2)
> - phase: M16 contract track (M16 plan §5.1–§5.3; contract work allowed before the entry gate closes)
> - completed: 2026-09-08
> - owners: autonomous-agent
> - artifacts: [threat model](../reports/m16-threat-model.md), [`RFC-0040`](../rfc/RFC-0040-native-automation-capabilities-v0.md) (draft), [`ADR-0013`](../adr/ADR-0013-native-automation-host-platform-v0.md) (proposed)

## 1. What was done

The three pre-implementation contracts M16 plan §5 requires, drafted to
owner-review quality:

1. **Threat model** (`docs/reports/m16-threat-model.md`): ten threat
   paths (T1–T10), each mapped to a capability boundary or non-goal AND a
   testable refusal/corpus case; two open findings (F-1 sensitive-field
   policy, F-2 audit retention) recorded as owner decisions.
2. **RFC-0040** (capability WIT v0, draft): the six-interface world
   (observe/capture/pointer/keyboard/audit/stop — the last two
   deliberately guest-invisible), preview/commit token fields, revision
   semantics, authority-composition rules, the E9xxx refusal-corpus plan,
   and the v0 non-goals.
3. **ADR-0013** (platform adapter, proposed): Windows-first; Win32
   enumeration with the UIA line drawn explicitly; WGC capture with GDI
   fallback; SendInput with commit-time rectangle re-derivation; bounded
   action atomicity as the in-flight cancellation invariant; Host-side
   dry-run; smallest-isolated-crate FFI placement with process isolation
   deferred and re-evaluated alongside M17's provider ADR.

## 2. Validation

Document-only step: no code changed; the planning validator
(`validate-step-0124.ps1`) and `git diff --check` green. RFC-0040 and
ADR-0013 proceed to owner acceptance; implementation STEPs (synthetic
adapter first) start only after all three contracts are accepted
(M16 plan §2 gate).

## 3. Open decisions left to the owner

- Accept/amend the threat model's F-1 (sensitive-field policy shape) and
  F-2 (audit retention) findings.
- Accept RFC-0040's provisional names and the E9xxx code allocation.
- Accept ADR-0013's in-process-adapter-for-v0 choice (process isolation
  deferred) and the UIA-line wording.
