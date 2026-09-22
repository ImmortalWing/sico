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

## 8. Gate-by-gate work breakdown (refined 2026-09-22; no STEP numbers reserved)

Working labels for planning only; each gate keeps its §3 exit test as the
authoritative bar. Nothing here reserves a STEP identifier or widens a
capability contract.

### 8.1 M17 gate 2 — accelerator provider ADR

- ADR content: provider class and isolation home (smallest isolated
  adapter crate per the AGENTS.md §4 FFI rule; compiler and shared
  security cores stay free of platform-specific unsafe code);
  declare-only, least-privilege capability surface; versioned, default-deny
  activation.
- Tolerance contract: defined per output on the frozen image-vision@1
  corpus (grey8/threshold/occupancy/occupancy-mask) and the tetris
  recognition chain — per-pixel/absolute bounds plus an aggregate bound
  per fixture; the deterministic reference implementation remains the
  default path; acceleration is opt-in and declared, never a silent
  substitution.
- Evidence set: reference-vs-accelerated agreement table on the frozen
  corpus; crash and resource-exhaustion injection against the provider
  (typed limit+1 refusals, fail-closed, host survives without restart).
- Recorded deferral clause: if no accelerator is authorized on the
  evidence host, gate 2 closes reference-only with the ADR recording the
  deferral (§5 risk 3).

### 8.2 M17 gate 4 — model/package provenance RFC

- Asset manifest schema: pinned digest algorithm, provenance fields,
  CPU/GPU/memory/time budget fields, versioned-package integration that
  extends the existing M7 digest discipline (no parallel trust path).
- Evidence set (a fixture model satisfies the gate): digest-verified load;
  typed refusals for bad digest, truncated and oversized assets
  (limit+1); budget enforcement refusals; cancellation mid-inference with
  a typed outcome and no orphaned work; malicious-asset refusal;
  provider-crash isolation without host restart.
- Model inference stays an explicit, budgeted, provenance-bound path;
  it never becomes a default dependency of deterministic vision (§4).

### 8.3 Roster packages — template match, grid, contour

- Per package: WIT surface under the versioned image-vision line;
  deterministic reference implementation; content-asserting corpus
  (byte-exact outputs on frozen inputs, including degenerate and limit+1
  inputs); consumers install through the M7 registry path with zero
  compiler/Runtime changes.
- The existing tetris recognition chain extends as each package lands,
  giving the pilot (§8.4) its recognition vocabulary incrementally.

### 8.4 Block-game pilot loop — capability mapping

Each phase maps to an already-GO capability; no new trust surface is
invented inside the pilot:

| Loop phase | Capability | Constraint |
|---|---|---|
| observe | M16 window-scoped capture | designated fixture window only; resolution, theme and window state pinned and recorded (§5 risk 1) |
| recognize | image-vision@1 deterministic chain (+ roster packages as they land) | reference path only in the pilot unless gate 2 explicitly authorizes acceleration |
| plan | M14 pure-Sico solver (STEP-0138) | frozen Python oracle stays the comparison oracle until the corresponding gate passes |
| preview | dry-run emission of the single next action + expected post-state | consumes no input grant |
| execute-one | exactly one M16 input action | scoped, revocable grant; never full-desktop |
| verify | re-capture + deterministic recognition compare against the expected post-state | mismatch stops the loop; no auto-retry in v0 |
| stop | emergency stop + cancel token | permission revocation takes effect before the next action |

Budget record: end-to-end capture→recognize→plan→execute latency is
sampled and recorded (no SLA claim; §5 risk 2). One loop iteration
executes at most one input action — the loop shape itself is the M16
safety contract.

### 8.5 Error-injection corpus (all outcomes fail closed)

Enumerated once so the exit audit can check it line by line:
mis-recognition (corrupted/ambiguous board frame), window drift (moved or
resized window), duplicate frames (no state change between observations),
solver timeout (budget exceeded mid-plan), cancellation mid-action,
permission revoked mid-loop, provider crash (if gate 2/4 paths are in the
loop), emergency-stop latency measured, and long-run stability sampled at
declared intervals. Every case produces a typed outcome; none may trap,
hang or fall back silently.
