# M24 Vision closure and block-game pilot

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved

## 1. Objective

Close the two mutually blocking NO-GOs recorded in the roadmap, in
dependency order:

1. **M17 → GO**: land the remaining M17 exit gates on top of the GO
   gate 1 (image-vision@1, STEP-0166): gate 2 (optional native
   acceleration provider ADR with reference-vs-accelerated tolerance
   evidence) and gate 4 (model/package provenance RFC: digest-bound
   weights, CPU/GPU/memory/time budgets, fail-isolated provider), plus
   the RFC-0043 roster packages (template match, grid, contour) as
   versioned packages with deterministic reference corpora.
2. **M18 block-game gate**: the 俄罗斯方块消除 case driven end-to-end on
   a real Windows window — the M14 pure-Sico solver consuming board
   state obtained through M16 scoped capture and M17 deterministic
   vision, executing through the M16 observe → plan → preview →
   execute-one → verify/stop loop with dry-run, single-step commit,
   post-action verification and emergency stop corpora.

This milestone is the acceptance chain the 案例项目 README freezes:
M14 纯 Sico 离线求解器 ✓（STEP-0138）→ M16 capture/input ✓（M16 GO）→
M17 vision（本里程碑）→ M18 完整应用试点（block-game gate，本里程碑）.

## 2. Entry gate

- M16 GO ✓（STEP-0159）；M17 gate 1 GO ✓（STEP-0166）；M18 combined
  portfolio 4/5 ✓（STEP-0165）— the block-game item is the recorded
  NO-GO this milestone closes.
- M7 package/trust/update boundary measured to carry versioned
  binaries ✓（M17 gate-1 evidence）; model assets extend the same
  digest discipline via the gate-4 RFC, accepted before implementation.
- Per-package/per-gate RFCs (RFC-0043 continuation, accelerator ADR,
  model RFC) accepted before their implementation.
- Pilot execution authority: the owner explicitly designates the target
  window/fixture for evidence runs; capture and input grants follow the
  M16 capability contract, scoped and revocable, never full-desktop.

## 3. Exit gates

1. M17 gate 2: accelerator provider ADR accepted; on at least one
   Windows host, accelerated and reference implementations agree within
   the declared tolerance on the frozen corpus; provider crash/resource
   exhaustion isolated and fail-closed.
2. M17 gate 4: model RFC accepted; a model-consuming path (may be a
   fixture model) demonstrates digest-verified load, budget enforcement
   (limit+1 typed), cancellation mid-inference, malicious-asset
   refusal, and provider-crash isolation without host restart.
3. Roster packages (template match/grid/contour) deterministic corpora
   byte-exact vs reference; consumers install them through the M7
   registry path without compiler/Runtime changes.
4. M17 flipped to GO by an explicit audit (or honest residual register).
5. Block-game pilot on a real Windows window: board recognition →
   solver plan → preview → single execute → verification corpora green;
   error injection (mis-recognition, window drift, duplicate frames,
   timeout, cancel mid-action, permission revocation) all fail closed;
   emergency stop measured; long-run stability sampled and recorded.
6. Evidence labels stay distinct: `internal-fixture` for fixture
   windows, `clean-room-consumer` for the pilot harness; any external
   observation stays owner-gated and is not implied by this milestone.
7. M0–M23 regression green + explicit exit audit for the M17 flip and
   the M18 block-game gate.

## 4. Non-goals

- Model inference as a default dependency of deterministic vision.
- CAPTCHA, authentication, anti-cheat evasion or covert monitoring of
  any kind.
- Full-desktop capture, background keyboard, clipboard, credentials or
  arbitrary process control (M16 contract unchanged).
- macOS/Linux/Android capture/input claims without native evidence.
- External pilot recruitment or third-party adoption claims (owner-gated,
  remains outside this milestone).
- Retiring the Python oracle in 案例项目 before the corresponding Sico
  layer passes its own gate (AGENTS.md §6).

## 5. Risks

- Real-window vision is environment-sensitive: corpora must pin
  resolution, theme and window state, and record drift behavior rather
  than hide it.
- Solver latency through capture/recognize/plan/execute must be
  measured (budget record, no SLA claim) — game-clock constraints may
  force explicit scope notes.
- Accelerator availability on the evidence host: gate 2 may close
  reference-only with the ADR recording the acceleration deferral if no
  accelerator is authorized.

## 6. Evidence classes

`internal-fixture` and `clean-room-consumer` only. The pilot runs on a
designated fixture window on the evidence host; nothing in this
milestone constitutes external adoption or production deployment.

## 7. Kickoff requirement

First STEP is an inventory in the STEP-0129/0142 pattern: measured
probe of the current image-vision@1 surface vs the roster needs, the
M17 gate gaps, the M16 capability surface for the pilot loop, and the
frozen corpus plan — before any new package lands.
