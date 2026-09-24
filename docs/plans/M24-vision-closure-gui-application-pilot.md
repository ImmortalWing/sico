# M24 Vision closure and GUI application pilot

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved; amended 2026-09-22 twice — (1) owner directives 「方块游戏pilot改为GUI和原生界面库（风格类似winui3）」 and 「原生界面库是sico语言原生界面库」 registered the Sico-language native UI library prerequisite and a first-party GUI pilot target (STEP-0257); (2) owner directive 「方块游戏pilot目标删除，改为带GUI界面的格式转换器，第一步可以先实现图片格式互相转换，例如jpg互转png」 deleted the block-game pilot target and re-designated the pilot as a GUI format converter (STEP-0258)

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
2. **M18 application pilot (portfolio item 4)**: a GUI format converter
   authored in Sico over the Sico-language native UI library (§8.6.1),
   closing the recorded pilot NO-GO; the first increment converts images
   between JPEG and PNG in both directions. Re-designated by owner
   directive (2026-09-22), which deleted the former block-game
   automation pilot; the former capture → vision → solver → input loop
   is deleted from this milestone (§8.6.2).

This milestone carries the M17 vision closure and the re-designated
application pilot. The 俄罗斯方块 case remains the M14 pure-Sico solver
acceptance oracle (STEP-0138 ✓) with its Python implementation as the
case oracle; it is no longer the M18/M24 acceptance vehicle (owner
directive, 2026-09-22).

## 2. Entry gate

- M16 GO ✓（STEP-0159）；M17 gate 1 GO ✓（STEP-0166）；M18 combined
  portfolio 4/5 ✓（STEP-0165）— the pilot item's recorded NO-GO is what
  this milestone closes, now against the re-designated format-converter
  target (2026-09-22).
- M7 package/trust/update boundary measured to carry versioned
  binaries ✓（M17 gate-1 evidence）; model assets extend the same
  digest discipline via the gate-4 RFC, accepted before implementation.
- Per-package/per-gate RFCs (RFC-0043 continuation, accelerator ADR,
  model RFC, format-converter codec contract) accepted before their
  implementation.
- Pilot authority: the converter runs as a local application under the
  user's authority on the evidence host; file access composes existing
  file capabilities with user-selected paths (dialogs, least-privilege);
  the pilot consumes no capture or input grants and the M16 capability
  contract is untouched.

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
5. GUI format-converter pilot (portfolio item 4, re-designated
   2026-09-22): the converter application authored in Sico over the
   Sico-language native UI library runs on the Windows evidence host;
   the first increment converts images between JPEG and PNG in both
   directions on a frozen corpus with byte-stable outputs; the §8.5
   fail-closed corpora are green; no silent fallback, no native escape,
   no core compiler/Runtime patches.
6. Evidence labels stay distinct: `internal-fixture` for M17 corpora
   and refusal fixtures, `clean-room-consumer` for the pilot
   application; any external observation stays owner-gated and is not
   implied by this milestone.
7. M0–M23 regression green + explicit exit audit for the M17 flip and
   the M18 application-pilot gate.

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
- Reviving the block-game automation pilot or the former
  capture→vision→solver→input chain inside this milestone (deleted by
  owner directive, 2026-09-22); the 俄罗斯方块 case remains the M14
  solver acceptance oracle only.

## 5. Risks

- Vision corpora stay fixture-pinned: generated fixtures pin
  resolution, theme and window state, and record drift behavior rather
  than hide it. No live application window is in the vision evidence
  path after the 2026-09-22 re-designation.
- Codec risk (pilot): JPEG decode/encode correctness and determinism
  are contract-pinned before implementation; untrusted-image bounds
  (dimension/pixel ceilings, memory/time budgets) must be typed, or the
  increment does not close.
- Accelerator availability on the evidence host: gate 2 may close
  reference-only with the ADR recording the acceleration deferral if no
  accelerator is authorized.

## 6. Evidence classes

`internal-fixture` and `clean-room-consumer` only. M17 corpora and
refusal fixtures are internal-fixture; the converter pilot is a
clean-room-consumer application on the evidence host; nothing in this
milestone constitutes external adoption or production deployment.

## 7. Kickoff requirement

First STEP is an inventory in the STEP-0129/0142 pattern: measured
probe of the current image-vision@1 surface vs the roster needs, the
M17 gate gaps, and the frozen corpus plan — before any new package
lands.

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
  corpus (grey8/threshold/occupancy/occupancy-mask) — per-pixel/absolute
  bounds plus an aggregate bound per fixture; the deterministic reference
  implementation remains the default path; acceleration is opt-in and
  declared, never a silent substitution.
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
- The former tetris recognition-chain extension is deleted with the
  block-game pilot (owner directive, 2026-09-22); the roster packages
  close §3 gate 3 on their own corpora.

### 8.4 Format-converter pilot — work package (re-designated 2026-09-22)

Supersedes the deleted block-game loop mapping (former §8.4, deleted by
owner directive). The pilot is a first-party GUI format converter; the
work package maps each concern to its contract home:

| Concern | Home | Constraint |
|---|---|---|
| source selection | file open dialog over the existing file capability | user-selected paths, least-privilege, no default-deny bypass |
| conversion engine | versioned package and/or host provider surface | frozen by RFC before implementation; image-vision@1 is deterministic pixel processing only and provides no codec I/O (honest gap) |
| presentation | Sico-language native UI library (§8.6.1) | pinned WinUI 3-like Fluent style, deterministic frames |
| persistence | file save dialog over the existing file capability | user-selected paths; typed save failures |
| refusal | typed outcomes end to end | no trap, hang, partial output or silent fallback |

- Increments: step 1 (§3 gate 5) converts JPEG ↔ PNG in both
  directions; further formats, options or batch features take their own
  RFC + STEP.
- The first increment's scope is frozen by the codec contract (RFC):
  accepted JPEG/PNG profiles, output parameters, and determinism
  guarantees (PNG outputs byte-stable for fixed input; JPEG encoder
  settings frozen by the corpus).

### 8.5 Format-converter evidence contract (fail-closed corpora, enumerated once)

Supersedes the deleted block-game error-injection corpus (former §8.5).
Enumerated once so the exit audit can check it line by line, all typed:
bad magic / wrong signature; truncated file; trailing data; oversized
dimensions or pixel count (declared limit+1); unsupported color types or
JPEG profiles pinned by the corpus; cancellation mid-conversion (typed
outcome, no orphaned partial output); save-path failure (unwritable or
refused); repeat conversion of the same input (byte-stable output);
long-run batch sampling at declared intervals. Every case produces a
typed outcome; none may trap, hang, write a partial file or fall back
silently.

### 8.6 Owner-directed pilot re-designation (2026-09-22; STEP-0257/0258)

Two same-day owner directives re-shaped the pilot:

1. 「方块游戏pilot改为GUI和原生界面库（风格类似winui3）」 clarified by
   「原生界面库是sico语言原生界面库」: the pilot GUI is built with a
   Sico-language native UI library styled like WinUI 3 (Fluent) — not a
   third-party or web toolkit (STEP-0257).
2. 「方块游戏pilot目标删除，改为带GUI界面的格式转换器，第一步可以先实现
   图片格式互相转换，例如jpg互转png」: the block-game pilot target is
   deleted; the pilot is the GUI format converter (STEP-0258).

Beyond the re-designation itself, no §3 gate numbering, no evidence
class and no capability contract changes; no STEP number is reserved.

#### 8.6.1 Sico-language native UI library (prerequisite workstream)

Today the M5 desktop host UI is a strict companion model with no
compiler-facing Sico binding (recorded in the 案例项目 README), and M15's
compiler-facing UI binding track is Web-scoped with M15 GO for that scope.
The directives therefore register a new surface, anchored on the
AGENT_GOAL desktop-host item 「最小 UI WIT/SDK」:

- Source side: a compiler-facing UI vocabulary authored in Sico source —
  window, layout panels, text, button, dropdown/selection, file open/save
  dialogs, image preview, progress indication, and pointer/click events —
  exposed through the versioned package/user-WIT surface of the RFC-0039
  modules/imports line, composed with the existing capability primitives
  (explicit, least-privilege, default-deny UI authority; no new trust
  path).
- Host side: the native desktop host renders the controls with a pinned
  WinUI 3-like Fluent style (typography, geometry, accent system, pinned
  color scheme). The renderer architecture is frozen by an ADR before
  implementation, honoring the AGENTS.md §4 boundaries: `sico` owns
  checking/compilation and never platform implementations; the desktop
  host owns the platform rendering; no web/webview path substitutes for
  the native library.
- Determinism contract (the corpus-facing property): fixed client size,
  pinned color scheme, no timers, animations or environment-dependent
  drawing in the conversion-relevant regions — every frame is a pure
  function of UI state.
- Contract-first discipline: an RFC freezes the source-level binding
  surface and typed refusals, and the ADR freezes the renderer
  architecture, before any implementation STEP (AGENTS.md §3). Scheduling
  is independent of the M17 gate order; the §7 kickoff inventory still
  precedes any new vision package.
- Honest scoping: this is new language/platform surface, not a claim that
  any UI binding exists today. Nothing in this workstream changes M15's
  GO state or Web scope, and no implementation claim is made until its
  own STEP lands with evidence.
- Dependent consumer (owner directive 2026-09-24): the M26 GUI PDF tool
  (M26 plan, STEP-0263) is authored over this library; the M26 kickoff
  inventory measures this surface against the PDF tool's control needs
  (file open/save dialogs, page list, metadata view, rotate/merge
  actions, progress), and any measured gap is closed in this workstream
  under its existing contract-first discipline before the M26 GUI gate
  starts. This cross-reference changes no M24 gate numbering, evidence
  class or capability contract, and reserves no STEP number.

#### 8.6.2 First-party GUI pilot target app

- The pilot application is the GUI format converter: pick image files,
  convert between JPEG and PNG (first increment), save the results.
  Authored in Sico over the UI library (§8.6.1); the conversion engine
  rides the §8.4 package/provider surface (RFC-first).
- Untrusted-input discipline (AGENTS.md §7): decoded sizes bounded,
  typed refusals per §8.5, budgets enforced, no partial outputs.
- Bounded exit test (its own STEP when scheduled): the app builds and
  runs on the Windows evidence host authored in Sico over the UI
  library; the Fluent-styled UI renders; the frozen conversion corpus
  produces byte-stable outputs; the §8.5 refusal corpus fails closed.
  Evidence class `clean-room-consumer` (§6).
- Deleted-content note: the former block-game target, its game-window
  binding design (exact title + owning-process identity), the former
  §8.4 loop mapping and the former §8.5 automation error-injection
  corpus are deleted (owner directive, 2026-09-22). M16 real-loop
  evidence stands at its M16 GO level (STEP-0159) and is not extended.
  The 俄罗斯方块 case remains the M14 solver acceptance oracle
  (STEP-0138) with the Python gamebot as the case oracle — not part of
  M24 evidence.
