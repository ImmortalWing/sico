# M18 Representative AI applications and external pilots

> Status: historical STEP-0165 audit GO for four of five portfolio applications; item 4 re-designated as the M24 native GUI format converter by STEP-0258, addendum pending per STEP-0259; external pilot remains NO-GO; no STEP numbers reserved

## 1. Objective

Validate Sico through independently reproducible applications rather than infrastructure-only demonstrations. M18 consumes stable public interfaces; a pilot that requires compiler or Runtime modification exposes an earlier milestone gap and does not pass.

## 2. Entry gate

| # | Condition | Status (measured 2026-09-13) |
|---|---|---|
| 1 | M14 is GO | **satisfied** — STEP-0141 (gate 7 live-model re-measure remains a separate owner-gated follow-up) |
| 2 | Each pilot's required platform milestone is GO | **partial, per pilot**: API agent (M12 full GO) ✓; streaming tool (M9/M11 GO) ✓; Web/UI app (M15 7/7 GO per STEP-0158+0162) ✓; application pilot (re-designated 2026-09-22: GUI format converter; its prerequisites are the M24 UI-library and codec-contract workstreams plus M7 package boundary, not M17 — former M17-dependent block-game path deleted by owner directive) |
| 3 | External accounts, devices, publishing identities and credentials are supplied explicitly by their owner | remains an external gate per pilot |
| 4 | Internal clean-room fixtures and external adoption remain separate evidence classes | standing rule |
| 5 | Release bundle install/upgrade path exists | **new (2026-09-13)** — pilots must install via the M19 release bundle, not raw target/ artifacts; M19 §3.2 gates this |


## 3. Required portfolio

1. Secure API agent using M12.
2. Streaming data tool using M9/M11.
3. Web/UI application using M15.
4. Application pilot: GUI format converter authored in Sico over the
   Sico-language native UI library (re-designated by owner directive
   2026-09-22, STEP-0258; first increment JPEG↔PNG). The former
   native-visual-automation pilot (block-game) is deleted from this
   portfolio; the M16 real-loop evidence stands at its M16 GO level.
5. Independent package/Component consumer with no compiler/Runtime patch.

## 4. Application pilot acceptance path (re-designated 2026-09-22)

1. Contract evidence: the Sico-language native UI library RFC + renderer
   ADR, and the format-converter codec contract RFC (accepted JPEG/PNG
   profiles, frozen encoder settings, typed refusal corpus), accepted
   before implementation.
2. Application evidence: the converter authored in Sico runs on the
   Windows evidence host; frozen corpus conversions are byte-stable;
   §8.5 of the M24 plan's refusal corpus fails closed with typed
   outcomes; no core compiler/Runtime patches.
3. Long-run evidence: repeat/batch sampling at declared intervals with
   resource records.

The application must never widen file authority beyond user-selected
paths, write partial outputs on failure, or fall back to a native escape.

> Former block-game acceptance path (deleted by owner directive
> 2026-09-22, STEP-0258): the 俄罗斯方块 case remains the M14 pure-Sico
> solver acceptance oracle (STEP-0138) with its Python case oracle; it is
> no longer the M18/M24 portfolio item.

## 5. Evidence classes

- `internal-fixture`: maintained in this repository.
- `clean-room-consumer`: separate code path/team but controlled locally.
- `external-pilot`: independent participant and environment.
- `production`: real deployment identity, operations and support evidence.

One class cannot be renamed as another. Reports must name the class, platform, runner, version and raw evidence location.

## 6. Non-goals

- Using pilots to introduce unreviewed compiler, Runtime or Host features.
- Treating internal fixtures as external adoption or local tests as production evidence.
- Automating accounts, devices or third-party services without explicit owner authority.
- Hiding unsupported platforms, manual steps, safety stops or residual risks.

## 7. Dependencies (added 2026-09-13)

M19 production engineering interleaves: pilots install/upgrade through
the M19 release bundle. The application pilot (GUI format converter) is
scheduled in M24 with its UI-library and codec-contract prerequisites
(STEP-0257/0258); the former M17-dependent block-game path was deleted by
owner directive and no longer gates this portfolio.

Addendum registration (owner-confirmed 2026-09-22, STEP-0259): the
completed four-pilot evidence and the STEP-0165 exit audit remain valid —
the 2026-09-22 re-designation touches only the never-completed portfolio
item 4 target, so no M18 rework is required. When the M24 format-converter
pilot passes its §3 gate 5, an **M18 exit-audit addendum** is executed
against the §4 acceptance path of this plan (build/run without core
patches, M19-bundle install, frozen-corpus byte-stability, §8.5-style
fail-closed refusals): it flips portfolio item 4 from NO-GO to GO, re-runs
the standing regression (§8 gate 7 at addendum time), updates the
ROADMAP/STATUS M18 lines, and is recorded as its own STEP. The external
pilot NO-GO is untouched by the addendum (owner-gated external gate).

## 8. Exit gates

1. Every portfolio application builds and runs without core patches.
2. Security, fault, cancellation, update and long-run tests are reproducible.
3. AI generation/repair/comprehension measurements use the frozen protocol and report costs/limits honestly.
4. At least one external pilot exists for any external-adoption claim.
5. Product/platform claims map to exact native or browser evidence.
6. Residual risks and unsupported scenarios remain explicit.
7. Full M0–M17 regression is green and the exit audit is explicit.
