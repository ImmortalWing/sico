# M25 Release v1.0 — platform breadth, completion audit and product exit

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved

## 1. Objective

Deliver two explicit verdicts over the M14–M24 evidence chain: whether the
versioned v1.0 release is ready, and whether the whole project satisfies
AGENT_GOAL §13. A completed audit is not itself either GO verdict:

1. **Language v1 freeze**: batches 1–3 (RFC-0044/0046 + M23) incorporated
   into a versioned language release — spec, corpus, diagnostics and
   formatter mutually consistent, with migration notes and the frozen
   application/Script profiles declared against it.
2. **§13 completion audit refresh**: the M20 per-item table re-issued
   with M21–M24 evidence links, every item GO or explicitly deferred
   with its external gate named (owner credentials, runners, public
   identity, devices).
3. **M19 residual closure where inputs allow**: full-scope install →
   run → upgrade → uninstall signed drill; registry deployment
   rehearsal repeated against the v1.0 bundle; public rollout remains
   owner-gated and is NOT claimed by this milestone.
4. **Platform breadth per available runners**: macOS/Linux parity
   corpora executed where the owner supplies runners (contract-verified
   where not); Android per the M20 two-path exit (device validation or
   explicit deferral with the external gate named).
5. **v1.0 release bundle**: reproducible composition through the M19
   pipeline (two independent builds byte-identical), signed, with
   release notes whose every claim traces to executed evidence.

## 2. Entry gate

- M23 exit audit explicit (GO or NO-GO with residuals recorded); M23 GO is
  required for **v1.0 release GO** because batch 3 is part of its freeze.
- M24 native UI library and GUI format-converter pilot gate explicit (GO or
  NO-GO with residuals recorded); converter GO on a real Windows native
  renderer is required for **v1.0 release GO**. M17 acceleration and
  external-adoption verdicts remain separately named.
- M22 dual-implementation row explicit: S7 GO or an R3 stop-loss NO-GO
  with Route-B re-baseline registered. The latter may enter this audit but
  must never be described as self-host GO.
- M19 CI/release engineering green and current (run-ci green on the
  release candidate).
- Owner inputs for production/external items either provided or
  explicitly declared deferred — the audit may not convert missing
  inputs into implied success.

## 3. Exit gates

1. §13 per-item audit published: every item linked to evidence and given
   GO, NO-GO or named external deferral. Publish **separate** `v1.0 release
   GO/NO-GO` and `project §13 completion GO/NO-GO` verdicts. Project
   completion is GO only when every §13 condition is actually met; a
   deferral is never renamed completion.
2. For v1.0 release GO, M23 batch 3 is GO and the language v1 freeze has a
   versioned spec/corpus/diagnostics snapshot,
   formatter idempotence, matrix rows consistent, migration notes for
   everything batch 1–3 changed; the freeze re-runs the full frozen
   corpus byte-exact.
3. For v1.0 release GO, the M24 native GUI converter is GO and the v1.0
   bundle reproducibility gate passes: two independent clean builds
   byte-identical; bundle installs and runs on a clean Windows host;
   upgrade/uninstall drill signed and recorded.
4. Platform matrix honest: declared platforms carry native or
   browser evidence; everything else is contract-verified or deferred
   with the gate named — no label laundering.
5. Dual-implementation register current (Rust oracle + self-host
   status), AI workflow protocol status recorded (live-model claims
   only from credentialed runs).
6. M0–M24 regression green from one command; documentation audit
   (docs vs executable behavior) passed; STATUS/ROADMAP consistent
   with the verdict.
7. Both verdicts issued against their respective gates, with the residual
   list and M22 self-host state explicit. The audit may finish while the
   v1.0 release is NO-GO; external platform or adoption deferrals may be
   listed, but cannot satisfy the project §13 completion gate.

## 4. Non-goals

- No new language surface in v1.0 (the freeze is the point).
- No public rollout, hosted registry or production identity claims
  without owner-provided inputs.
- No mobile/Harmony claims; M6 stays externally gated.
- No live-model claims without credentialed runs.
- No retirement of the Rust oracle or runner/Host native architecture
  (M22 non-goals carry forward).

## 5. Risks

- The §13 refresh can surface evidence gaps late. M23/M24 audits are
  explicit first. Internal M23 batch-3 or M24 native-converter failures
  block v1.0 release GO; owner-gated externals are named separately and
  block project completion where §13 requires them.
- Freeze discipline vs late fixes: any post-freeze language change
  re-runs the frozen corpus and gets a versioned amendment, not a
  silent patch.

## 6. Evidence classes

All release claims trace to `internal-fixture`/`clean-room-consumer`
evidence or name their external gate. `production` claims require
owner-supplied production inputs and are out of scope for the internal
verdict.

## 7. Kickoff requirement

First STEP is an inventory: diff the current §13 table (STEP-0170)
against the M21–M24 evidence chain, list every item's evidence link or
missing gate, and freeze the v1.0 bundle composition manifest — before
any release engineering lands.
