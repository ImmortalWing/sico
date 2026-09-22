# STEP-0257: M24 amendment — Sico-language native UI library and the first-party GUI pilot target

> - status: superseded by STEP-0258 (owner directive 2026-09-22 deleted
>   the block-game pilot target; the §8.6.1 UI-library prerequisite
>   workstream carries over unchanged) / planning documentation; no gate
>   or support change; M17/M24 NO-GO unchanged, M15 GO scope unchanged
> - phase: M24 planning support (owner directives 2026-09-22 「方块游戏pilot改为GUI和原生界面库（风格类似winui3）」 and 「原生界面库是sico语言原生界面库」)
> - completed: 2026-09-22
> - evidence: documentation-only; contract checks via `tools/validate-step-0257.ps1` (which asserts the amended sections, re-runs `tools/validate-step-0124.ps1` and `git diff --check`)

## Objective

Record the owner's re-designation of the M24 block-game pilot target: the
pilot application is the 俄罗斯方块消除 game as a first-party GUI
application authored in Sico over a **Sico-language native UI library**
styled like WinUI 3 (Fluent) — not an external or third-party game window,
and not a third-party Rust/web UI toolkit. Register the Sico-language
native UI library as an M24 prerequisite workstream, keeping every §3 exit
gate text, evidence class and capability contract unchanged.

> **Superseded the same day (2026-09-22) by STEP-0258**: the owner then
> deleted the block-game pilot target entirely —「方块游戏pilot目标删除，改为
> 带GUI界面的格式转换器，第一步可以先实现图片格式互相转换，例如jpg互转png」.
> The §8.6.1 Sico-language native UI library prerequisite workstream
> carries over unchanged into the re-designated GUI format-converter
> pilot; the §8.6.2 block-game target app, its window-binding design and
> its loop-mapping are deleted from the M24 plan. This record is retained
> as the directive history.

## Changes

- **M24 plan** (`docs/plans/M24-vision-closure-block-game-pilot.md`):
  header status records the amendment; §1 objective 2 notes the amended
  target designation with the gate text unchanged; §8.4's `observe` row
  now names the first-party Sico GUI game window; new §8.6 registers —
  (8.6.1) the Sico-language native UI library prerequisite workstream:
  source-side compiler-facing UI vocabulary (window, layout, text, button,
  grid/canvas board, click events) through the RFC-0039 versioned
  package/user-WIT surface with capability composition, host-side pinned
  WinUI 3-like Fluent rendering in the native desktop host with an ADR,
  a determinism contract (fixed client size, pinned color scheme, no
  timers/animations in board and tray regions — frames are a pure function
  of UI state, and the pinned palette doubles as the M17 vision color/
  threshold anchor), RFC+ADR before implementation, and honest scoping
  against the M5 companion-model status quo; (8.6.2) the first-party GUI
  pilot target app: game rules mirrored from the 案例项目 model (8×8, 22
  fixed-orientation pieces, full row/column clearing) so the M14 solver
  state space is unchanged, a two-click interaction contract (select tray
  slot, click board origin; at most one M16 input action per loop
  iteration), window binding by exact title plus owning-process identity
  with the M16 class-bound binding reserved to the counter fixture, a
  bounded exit test, and the external WeChat/ADB flow kept as the frozen
  Python oracle only.
- **ROADMAP**: the M24 status line records the 2026-09-22 owner directives
  and points to plan §8.6 (STEP-0257) with gates/evidence classes
  unchanged.
- **STATUS**: `updated`, `current support step` and `roadmap decision`
  record this amendment.
- **案例项目 README**: a new section states the M24 pilot target, the
  honest "the Sico native UI library does not exist yet" claim, and that
  the Python `gamebot` oracle is not retired.
- Cross-reference: `docs/steps/README.md` gains this record's row.

## What this step deliberately does not do

- No §3 exit gate text, entry gate, non-goal, evidence class or capability
  contract is added, removed or weakened in M24 or any other milestone;
  the first-party game window satisfies the existing "real Windows window"
  gate wording.
- No implementation exists for the Sico-language native UI library: M5 UI
  remains a strict companion model with no compiler-facing Sico binding,
  and no RFC/ADR has been drafted yet — all §8.6 statements are planned.
- No third-party UI framework (Rust toolkit, web/webview, or direct
  Windows App SDK/XAML) is adopted as the pilot UI path; "WinUI 3-like" is
  a pinned style requirement on the Sico-owned library, per the owner's
  clarification.
- No STEP number is reserved for the RFC, the ADR, the library
  implementation or the pilot app; each takes its own STEP when its entry
  gate is satisfied. The M24 §7 kickoff inventory still precedes any new
  vision package.
- No support claim changes anywhere: M17/M24 stay NO-GO, M15's GO state
  and Web scope are untouched, M22 remains the active mainline.

## Executed validation

- `tools/validate-step-0257.ps1`: asserts the amended M24 plan sections
  (§8.4 observe row, §8.6 with its two sub-blocks and the no-gate-change
  statement), the STEP-0257 pointers in ROADMAP/STATUS/steps index, the
  案例项目 README target section, then re-runs `tools/validate-step-0124.ps1`
  (M14–M25 planning contract, ADR-0015 closure record, ROADMAP ordering
  and step-number reservation check) and `git diff --check`. All green.

## Honest residuals

- The Sico-language native UI library does not exist; its binding surface
  (RFC) and renderer architecture (ADR) are unscheduled planning work, and
  the §8.6.2 window-binding design (title + process identity) is a plan
  note until the implementation RFC fixes it.
- Scheduling of the UI library workstream relative to the M17 gates is
  deliberately open; only the §7 kickoff-inventory precedence over new
  vision packages is fixed.
- The M22 self-host mainline (S3/S4 partial, S5/S6 not entered) is
  unchanged and remains the active track.
