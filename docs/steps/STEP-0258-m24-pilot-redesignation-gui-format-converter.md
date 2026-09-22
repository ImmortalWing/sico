# STEP-0258: M24 pilot re-designation — block-game pilot deleted, GUI format converter becomes the application pilot

> - status: complete / planning documentation; no gate-count, evidence-class or capability-contract change; M17/M24 NO-GO unchanged, M15 GO scope unchanged, M16 evidence stands at its M16 GO level
> - phase: M24 planning support (owner directive 2026-09-22 「方块游戏pilot目标删除，改为带GUI界面的格式转换器，第一步可以先实现图片格式互相转换，例如jpg互转png」)
> - completed: 2026-09-22
> - evidence: documentation-only; contract checks via `tools/validate-step-0258.ps1` (which re-runs `tools/validate-step-0124.ps1` and `git diff --check`)

## Objective

Record the owner's deletion of the M24 block-game automation pilot and the
re-designation of the M18 portfolio item 4 (application pilot) as a
first-party GUI format converter: pick image files, convert between JPEG
and PNG in both directions (first increment), save the results — authored
in Sico over the Sico-language native UI library registered by STEP-0257
(§8.6.1 of the M24 plan carries over unchanged). Supersedes the STEP-0257
pilot target (§8.6.2 block-game app) while preserving its §8.6.1
prerequisite workstream.

## Changes

- **M24 plan renamed and rewritten** from
  `M24-vision-closure-block-game-pilot.md` to
  [`M24-vision-closure-gui-application-pilot.md`](../plans/M24-vision-closure-gui-application-pilot.md):
  the M17 closure work (§8.1–8.3) is preserved verbatim in substance; the
  pilot work packages are replaced — §8.4 is now the format-converter
  work package (source/save dialogs over the existing file capability,
  conversion engine on a versioned package/host-provider surface frozen
  by an RFC, presentation on the Sico-language native UI library, typed
  refusals end to end), §8.5 is now the converter's fail-closed evidence
  corpus (bad magic, truncation, trailing data, limit+1 dimensions,
  unsupported profiles, mid-conversion cancellation, save failure,
  byte-stable repeat conversion, long-run sampling), and §8.6 records
  both 2026-09-22 owner directives with a deleted-content note: the
  former block-game target, its window-binding design, the former loop
  mapping and the former automation error-injection corpus are deleted;
  M16 real-loop evidence is not extended beyond its M16 GO level
  (STEP-0159).
- **诚实边界登记**: the image-vision@1 package is deterministic pixel
  processing only and provides no codec I/O — the conversion engine is
  new versioned-package/host-provider surface, contract-frozen (accepted
  JPEG/PNG profiles, frozen encoder settings, typed untrusted-image
  bounds) before implementation; no codec support is claimed today.
- **ROADMAP**: the M24 heading, status line, entry/exit conditions, the
  M18 section paragraph, the M25 entry-condition reference and the
  dependency-shape label are re-pointed at the GUI application pilot;
  the roadmap phase header records the re-designation.
- **M18 plan**: entry-gate row 2, portfolio item 4, §4 (retitled to the
  application-pilot acceptance path with the former block-game path kept
  as a deleted-content note) and §7 dependencies are re-pointed; the
  M18 portfolio item 4 no longer waits on M17 (its prerequisites are the
  M24 UI-library and codec-contract workstreams plus the M7 package
  boundary).
- **M25 plan**: entry-gate bullet renamed to the M24 application-pilot
  gate.
- **AGENT_GOAL / DIRECTION**: the M24 milestone text and the closing-pace
  paragraph record the deletion and the converter pilot.
- **Case README**: the M24 pilot-target section added by STEP-0257 is
  replaced — the case remains the M14 solver acceptance oracle (STEP-0138)
  with its Python oracle, and is no longer the M18/M24 acceptance
  vehicle; the converter pilot is not part of the case project.
- **STEP-0257**: marked superseded by this record (its §8.6.1 workstream
  carries over); its validator is re-pointed at the superseded state.
- Cross-reference: `docs/steps/README.md` rows updated (STEP-0257 marked
  superseded; STEP-0258 added).

## What this step deliberately does not do

- No §3 gate numbering change, no evidence-class change and no capability
  contract change: the converter pilot closes the same recorded M18
  portfolio item 4 NO-GO under the same `clean-room-consumer` class, and
  consumes no capture/input grants.
- No implementation exists: neither the Sico-language native UI library
  nor the codec contract nor the converter app has an RFC accepted or a
  line of code; every §8.4–8.6 statement is planned. No STEP number is
  reserved for them.
- The 俄罗斯方块 case is not deleted from the repository or retired as
  the M14 solver oracle (STEP-0138, AGENTS.md §6); only its M18/M24
  pilot role is removed. M14/M17 vision closure work is untouched.
- No M15 scope change: the Sico-language native UI library is native
  desktop surface, distinct from M15's Web-scoped UI binding.

## Executed validation

- `tools/validate-step-0258.ps1`: asserts the renamed M24 plan (converter
  sections §8.4/§8.5, deletion markers, §8.6.1 carried over), the
  re-pointed ROADMAP/M18/M25/AGENT_GOAL/DIRECTION/case-README records, the
  STEP-0257 superseded marker and the steps-index rows, then re-runs
  `tools/validate-step-0124.ps1` (M14–M25 planning contract, roadmap
  ordering, STEP-number reservation check) and `git diff --check`. All
  green; `tools/validate-step-0257.ps1` re-run green against its
  superseded-state assertions.

## Honest residuals

- The converter's codec contract RFC and the UI-library RFC/ADR remain
  unscheduled planning work; the §8.6.2 window-binding design from
  STEP-0257 is deleted with the block-game target.
- The M17 gates 2/4 and roster packages still gate the M17 flip
  independently of the pilot re-designation; the §7 kickoff inventory
  still precedes any new vision package.
- The M22 self-host mainline (S3/S4 partial, S5/S6 not entered) is
  unchanged and remains the active track.
