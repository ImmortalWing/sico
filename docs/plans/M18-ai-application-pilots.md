# M18 Representative AI applications and external pilots

> Status: planned; owner-approved 2026-09-04; refined 2026-09-13 at owner directive "规划M18-M20" (entry-gate statuses measured, dependency on M19 added); no STEP numbers reserved

## 1. Objective

Validate Sico through independently reproducible applications rather than infrastructure-only demonstrations. M18 consumes stable public interfaces; a pilot that requires compiler or Runtime modification exposes an earlier milestone gap and does not pass.

## 2. Entry gate

| # | Condition | Status (measured 2026-09-13) |
|---|---|---|
| 1 | M14 is GO | **satisfied** — STEP-0141 (gate 7 live-model re-measure remains a separate owner-gated follow-up) |
| 2 | Each pilot's required platform milestone is GO | **partial, per pilot**: API agent (M12 full GO) ✓; streaming tool (M9/M11 GO) ✓; Web/UI app (M15 7/7 GO per STEP-0158+0162) ✓; native visual automation (M16 7/7 GO ✓ **and M17 GO ✗ — the block-game path waits for M17**) |
| 3 | External accounts, devices, publishing identities and credentials are supplied explicitly by their owner | remains an external gate per pilot |
| 4 | Internal clean-room fixtures and external adoption remain separate evidence classes | standing rule |
| 5 | Release bundle install/upgrade path exists | **new (2026-09-13)** — pilots must install via the M19 release bundle, not raw target/ artifacts; M19 §3.2 gates this |


## 3. Required portfolio

1. Secure API agent using M12.
2. Streaming data tool using M9/M11.
3. Web/UI application using M15.
4. Native visual automation application using M16/M17.
5. Independent package/Component consumer with no compiler/Runtime patch.

## 4. Block-game automation acceptance path

1. M14 evidence: board/rules/search/scoring implemented and executed entirely in Sico against the Python oracle corpus.
2. M16 evidence: authorized window capture, dry-run plan, preview-bound single drag, new-frame verification and safe stop.
3. M17 evidence: deterministic grid/piece recognition; optional small model used only when measured evidence justifies it.
4. M18 evidence: repeated controlled runs, injected recognition/action failures, resource/performance records and an independently reproducible setup.

The application must never treat capture permission as input permission, execute multiple unverified actions, or bypass platform/service rules.

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
the M19 release bundle. The block-game acceptance path additionally
requires M17 to reach GO (RFC-0043 implementation per STEP-0160's
completion path).

## 8. Exit gates

1. Every portfolio application builds and runs without core patches.
2. Security, fault, cancellation, update and long-run tests are reproducible.
3. AI generation/repair/comprehension measurements use the frozen protocol and report costs/limits honestly.
4. At least one external pilot exists for any external-adoption claim.
5. Product/platform claims map to exact native or browser evidence.
6. Residual risks and unsupported scenarios remain explicit.
7. Full M0–M17 regression is green and the exit audit is explicit.
