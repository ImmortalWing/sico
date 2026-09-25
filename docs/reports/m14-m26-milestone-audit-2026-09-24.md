# M14–M26 milestone route audit (2026-09-24)

> Evidence class: planning audit. This report reconciles recorded exit audits,
> accepted contracts, current plans and the latest STEP; it does not promote a
> contract or a historical test run to new runtime evidence. Historical STEP
> records retain their original verdicts. Changes to executable support still
> require their own measured STEP and runner evidence.

## Verdict register

Gate numbers refer to each milestone plan's exit-gate section. `GO` below is a
recorded verdict at its stated scope and date, not a claim that every old test
was rerun for this planning audit.

| Milestone | Gate-level state and evidence | Evidence class / platform | Open gate or prerequisite | Next bounded action |
|---|---|---|---|---|
| M14 | STEP-0141: gates 1–6, 8 GO; gate 7 blocked-external-evidence. Application matrix and three applications executed. | internal-fixture; Windows x64 GNU and Linux x64 native at audit time | Owner-credentialed M14-profile AI re-measure. Current static inspection still finds no `open_scopes.push` and no `For` entry in `region_close_map`; runtime reproduction of the reported check/build scope gap is pending. | Reproduce the scope case before using “zero undeclared gap” as an M25 freeze premise. |
| M15 | STEP-0158 + STEP-0162: gates 1–7 GO for the declared Web v0 subset. | internal-fixture / clean-room-consumer; headless Edge 152 and Windows native path | Other browser engines, screen reader and native Sico UI binding are outside this GO. | Reuse the Web evidence only for Web claims; inventory the separate M24 native binding. |
| M16 | STEP-0159: gates 1–7 GO for the controlled Windows observe/preview/execute-one/verify loop. | internal-fixture; Windows x64 native, synthetic cases contract-verified | No broader platform or keyboard/full-desktop grant follows. | Reuse the existing capability boundary; do not make it a prerequisite for the M24 converter. |
| M17 | STEP-0160 NO-GO; STEP-0166 later closes gate 1. Gates 2/4 and the remaining roster/provider corpora remain open. | internal-fixture; deterministic package corpus, no accelerated-run claim | Gate 2 needs a real named accelerator comparison; gate 4 needs model/provider evidence. | M24 inventory names the evidence host and freezes separate gate-2, gate-4 and roster exit tests. |
| M18 | STEP-0165: four of five portfolio applications GO at their scope; external pilot gate 4 NO-GO. STEP-0259 reassigns only the unfinished item 4 to the GUI converter. | internal-fixture / clean-room-consumer; external-pilot absent | M24 converter gate 5, then M18 addendum; independent external participant remains owner-gated. | Run the converter addendum after M24 gate 5, retaining the original external-pilot verdict. |
| M19 | STEP-0168: gates 1/2/4/7 GO; install drill gate 3 and budget/documentation gates 5/6 partial. | internal-fixture / clean-room-consumer; local CI/release rehearsal | Fresh clean-host signed install/upgrade/uninstall and release-candidate budget/documentation closure. | M25 inventories each residual against its release candidate and executes the full signed drill. |
| M20 | STEP-0170 overall NO-GO: gates 1–3 NO-GO, 4 partial, 5–6 GO at audit time. Language work moved to M23/M25; platform inputs remain external. | internal-fixture for batch 1; macOS/Linux/Android native parity not claimed | Language freeze, owner-supplied runners/device, v1 tooling re-measure. | Carry the language freeze to M25 and retain platform deferrals with exact external gates. |
| M21 | STEP-0177 records gates 1–6 GO, with owner-gated live-model and independent-author work. Current static inspection confirms `opener_close` still lacks While/For; this is not a new exit audit. | internal-fixture / clean-room-consumer; local runner at audit time | Formatter and reported diagnostic/scope gaps need focused executable re-verification before M25 claims a coherent v1 language surface. | Reproduce relevant cases during M23 inventory; repair under a separately bounded implementation STEP if confirmed. |
| M22 | S1 and declared S2 subset complete; S3/S4/S5 partial, S6/S7 not entered. STEP-0278 retires legacy units; STEP-0280 implements W1 C/D with local 215 + 5 real-runner differential green. | internal-fixture; Windows GNU runner and STEP-0221/0245/0261/0262 canaries green locally, independent CI pending | W1 CI adjudication, records re-baseline, phase canary, S5 and ADR-0015 `A == B == C`. | Resolve W1 CI result, then pin W2 phase canary before counting R3. |
| M23 | Planned; Route A after M22 S7 GO or Route B after a recorded M22 R3 stop-loss. No batch-3 item has an accepted implementation RFC. | planning-only | Measured four-item inventory, accepted per-item RFC, chosen route. | Inventory the four consumers and diagnostic namespace while M22 proceeds. |
| M24 | Planned; M17 gate 2/4 and roster, native UI library and JPEG↔PNG converter have distinct verdicts. Kickoff is independent of M22/M23. | planning-only | RFC/ADR for native UI and codec; real Windows renderer and accelerator evidence where claimed. | Inventory native UI controls, codec surface and M17 evidence host; freeze contracts before implementation. |
| M25 | Planned. Audit may run with explicit upstream NO-GO, but v1.0 release GO requires M23 batch 3 and M24 native GUI converter GO plus release gates. Project §13 completion is a separate verdict. | planning-only | M23/M24 product gates, full signed bundle drill, platform/external evidence register. | Diff STEP-0170 §13 table against M21–M24 and freeze release composition. |
| M26 | Planned after M25 v1.0 release GO: versioned PDF package plus Sico-native GUI. No PDF runtime support claimed. | planning-only | PDF RFCs, bounded parser corpus, M24 native UI readiness; post-v1 UI extensions need version/compatibility evidence. | Inventory and draft RFCs early; implement only after M25 release GO. |

## Route decisions from this audit

1. The M22 R3 counter starts with W2 S3/S4 implementation STEPs after the
   records-surface canary is pinned. Every implementation STEP counts, even
   when it makes no progress; documentation and measurement-only STEPs do not.
   Four consecutive STEPs without current-phase coverage growth **and**
   without a moved typed refusal frontier require a stop-loss declaration and
   M23 Route B. Pending CI evidence leaves the STEP unadjudicated until the
   required runner run completes. Formatter coverage is the
   first canary; after 30/30, use byte-exact differential progress across the
   remaining selfhost source set. S5 has its separate byte-equal codegen corpus.
2. STEP-0266 measured formatter coverage and fuel intervals, but the runner
   CLI exposed no stack high-water. The R1 record remains explicit about that
   missing measurement; it is not silently called observed S6 evidence.
   STEP-0270's EC-4 census also limits direct list-ceremony removal to a
   2.13% line-count proxy; Option A is justified by typed invariants and
   maintenance rather than a promised dramatic source-size reduction.
3. M24 work can close before M23, with regression over all code landed at its
   audit revision. M25 reruns M0–M24 after both milestones. A reference-only
   result cannot close M17 gate 2 or flip M17 to GO.
4. M25 issues separate `v1.0 release GO/NO-GO` and `project §13 completion
   GO/NO-GO` verdicts. M23 batch 3 or the M24 native GUI converter at NO-GO
   forces v1.0 release NO-GO, although the audit itself may finish. An M22
   stop-loss is disclosed as NO-GO, never self-host GO. External gates may be
   deferred by name, without turning a missing §13 item into project complete.
5. M26 retains the final read/merge/rotate/GUI objective. Inventory and RFC
   drafting can precede M25, but implementation waits for v1.0 release GO.
   Its work is staged
   as contract → structural reader subset → extended accepted PDF forms →
   merge/rotate → native GUI. Any native UI addition after v1.0 is versioned
   and compatibility-tested; the M25 evidence snapshot is not rewritten.

## Discrepancy handling

- The M14/M21 post-audit findings in
  [`m20-m21-quality-review-2026-09-20.md`](./m20-m21-quality-review-2026-09-20.md)
  are not a new exit audit. A current static source check confirms no
  `open_scopes.push` in `sico-semantics`, no `For` branch in its
  `region_close_map`, and no While/For branch in `sico-format::opener_close`.
  This is a credible gap, but a check/build/format executable reproduction
  is still needed before changing a historical GO verdict or claiming repair.
- The M14–M21 plan headers describe plans as originally registered. Current
  verdicts come from the linked STEP audits and ROADMAP. Historical audit
  files are not rewritten to retroactively use the M24 converter target.
- STEP-0278's local source checks passed. STEP-0280 subsequently ran the
  215-case frozen differential plus five SHA-frozen W1 additions on the real
  Windows GNU runner. Local STEP-0221/0245/0261/0262 canaries passed after
  reconciling post-freeze records and current frontier; independent CI is
  pending. This audit does not close W1 or change M22's NO-GO state.
