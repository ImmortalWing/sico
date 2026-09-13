# M20 Cross-platform runtime, language v1 and completion audit

> Status: planned; owner session directive "规划M18-M20" (2026-09-13); refined 2026-09-13 after M17 gate-1 GO (STEP-0166) and the M18 exit audit (STEP-0165) — entry gates re-measured, language v1 candidates grounded in the measured pilot friction log; no STEP numbers reserved

## 1. Objective

Close the two remaining §13 completion dimensions the M14–M19 arc did
not cover: (a) platform breadth — the Desktop Host's macOS/Linux parity
evidence and the Android re-entry path (M6's deferred runtime evidence);
(b) language v1 — promote the application-proven profile plus the
accepted proposed surface (for-loops, closures, collection iteration)
into a versioned, spec-complete language release with migration rules;
then run the final §13 completion audit that the whole roadmap points
at.

## 2. Entry gate

| # | Condition | Status (measured 2026-09-13) |
|---|---|---|
| 1 | M19 CI + release engineering green | future — M19 plan refined and sequenced (§8); its exit gates 1–3 remain the gate |
| 2 | M18 portfolio runs without core patches | **satisfied (4/5)** — STEP-0165 exit audit; block-game pilot deferred pending M17 full GO |
| 3 | macOS/Linux runners and Android toolchain/licensed devices are owner-supplied | external inputs; contract work may proceed without them (contract-verified only) |
| 4 | Language surface changes carry an accepted RFC each | standing rule (RFC-0033 reconsideration gate) |

## 3. Required workstreams

### 3.1 Language v1 stabilization

- Promote the language surface that M14–M18 consumers ACTUALLY needed,
  measured from the pilot friction log (each through its own RFC, with
  syntax-candidates corpus, diagnostics and formatter/parser/outline
  support):
  - byte-level access (`byte-at`, bytes equality) — the tetris reader
    had to push ASCII-mask encoding into the package because v0 has no
    byte-at intrinsic (STEP-0166);
  - text index/char access — same class of gap;
  - infix equality/arithmetic in all lowering positions — the Web/UI
    pilot had to rewrite `==` conditions into `match` form (STEP-0164);
  - reserved-word collisions (`task` is reserved — hit by the API agent
    pilot) and the migration-note set for every reserved word since
    0.0.1;
  - the original proposed candidates stand: for-loops, closures,
    map/record iteration keys-values.
- Decisions also owed: whether checked-only arithmetic stays the sole
  form in v1 (plain `/` is currently a lexer-level refusal;
  `checked_div` works) — measured in the tetris pilot.
- Freeze language v1: grammar version, reserved words, profile matrix,
  refusal corpus, migration notes for every reserved-word or behavior
  change since 0.0.1 (module/use, pkg, interface version).
- Spec: SEMANTICS.md and SYNTAX.md updated to match compiler behavior
  exactly (§13: "规范与编译器行为一致"), each rule linked to its
  positive/negative corpus cases.

### 3.2 Desktop Host cross-platform parity (macOS/Linux)

- Port/replay the M5 evidence classes (install/open/run/uninstall,
  permission UI equivalence, lifecycle) on macOS and Linux with pinned
  distro/OS versions; parity corpus = the M5 corpus re-run natively.
- M15 cross-host matrix re-run per platform (the JS substrate makes
  browser parity cheap; the native runner parity is the real work).
- Honest rule: per-platform claims only on real native runners;
  contract-verified stays labelled.

### 3.3 Android re-entry (M6 unblock path)

- Resume STEP-0060 device validation the moment licensed SDK/NDK +
  device arrive: the shared-core contracts from M6 remain the spec.
- wasmtime Android target (Tier-3) risk re-assessment with a spike
  STEP; if unmet, Android stays a documented deferred platform (honest
  NO-GO, not a claim).

### 3.4 Tooling and ecosystem polish

- LSP/AI tooling surface review against §13 (`outline`, `describe`,
  `slice`, `impact`, `flow` stability), semantic-index accuracy re-run.
- Standard library boundary review: every `sico.*` intrinsic documented,
  versioned and corpus-linked.

### 3.5 Final §13 completion audit

- Requirement-by-requirement audit of AGENT_GOAL.md §13 (language,
  toolchain, runtime, platform, AI workflow, ecosystem/delivery, audit),
  each linked to evidence, each gap either closed or registered as a
  documented deferred platform/capability with its external gate named.

## 4. Non-goals

- New application domains (M18 scope) or new host capabilities.
- Floating-point/ML language surface (stays M17 scope).
- Claiming Android/macOS/Linux from contract-verified work.
- Silent language changes: every v1 change carries migration notes.

## 5. Exit gates

1. Language v1 frozen: grammar/spec/corpus/diagnostics consistent,
   migration notes complete, accepted per-feature RFCs.
2. Desktop Host parity corpus green on every declared macOS/Linux
   version (pinned); per-platform claims map to those runs.
3. Android path decided on evidence: either device-validated corpus
   green or a documented deferral with its external gate.
4. Tooling review closed: AI workflow protocols stable and measured.
5. §13 completion audit published with per-item evidence links; every
   unmet item carries a named external gate or a documented deferral.
6. Full M0–M19 regression green; the completion audit is explicit.

## 6. Dependencies and parallelism

- §3.1 blocks the v1 audit but nothing else; §3.2/§3.3 are independent
  of each other and of §3.1; §3.4 can run from the entry gate.
- External inputs (runners, devices) gate only their own evidence.

## 7. Sequencing rule

No implementation STEP is reserved by this plan. First STEP: language
v1 inventory (which proposed entries did M14–M18 consumers need —
measured, STEP-0142 pattern; the §3.1 list above is the starting
inventory), then per-feature RFCs. Platform work follows owner-supplied
runners; the §13 audit runs last against whatever is honestly closed.

## 8. Execution sequence (work items; STEP numbers allocated at kickoff)

1. Language v1 inventory STEP (measured, STEP-0142 pattern) freezing the
   candidate list and its corpus plan.
2. Per-feature RFCs + implementation + corpus (ordered by pilot need:
   byte/text access first — two pilots hit it).
3. v1 freeze STEP: grammar version, migration notes, spec sync
   (SEMANTICS.md/SYNTAX.md vs compiler behavior).
4. macOS/Linux Desktop Host parity corpus (on owner runners).
5. Android spike + re-entry decision (or documented deferral).
6. Tooling/AI-protocol review (§13 tooling items).
7. §13 completion audit STEP (requirement-by-requirement, evidence
   links, named external gates for gaps).
