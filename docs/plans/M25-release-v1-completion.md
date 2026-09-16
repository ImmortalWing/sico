# M25 Release v1.0 — platform breadth, completion audit and product exit

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved

## 1. Objective

Deliver the versioned v1.0 product verdict that AGENT_GOAL §13 defines,
on top of the full M14–M24 evidence chain instead of partial audits:

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

- M23 exit audit explicit (GO or NO-GO with deferrals recorded).
- M24 block-game gate explicit (the M18 acceptance chain closed or
  honestly deferred with its external gate named).
- M19 CI/release engineering green and current (run-ci green on the
  release candidate).
- Owner inputs for production/external items either provided or
  explicitly declared deferred — the audit may not convert missing
  inputs into implied success.

## 3. Exit gates

1. §13 per-item audit published: every item linked to evidence, verdict
   GO / deferred-with-gate; the overall product verdict explicit
   (`complete` or `complete-with-deferrals`), never implied.
2. Language v1 freeze: versioned spec/corpus/diagnostics snapshot,
   formatter idempotence, matrix rows consistent, migration notes for
   everything batch 1–3 changed; the freeze re-runs the full frozen
   corpus byte-exact.
3. v1.0 bundle reproducibility: two independent clean builds
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
7. Product exit verdict issued against AGENT_GOAL §13, including the
   residual list, without promoting any deferred item.

## 4. Non-goals

- No new language surface in v1.0 (the freeze is the point).
- No public rollout, hosted registry or production identity claims
  without owner-provided inputs.
- No mobile/Harmony claims; M6 stays externally gated.
- No live-model claims without credentialed runs.
- No retirement of the Rust oracle or runner/Host native architecture
  (M22 non-goals carry forward).

## 5. Risks

- The §13 refresh can surface evidence gaps late; mitigation is the
  entry-gate requirement that M23/M24 audits are explicit first, and
  the audit publishes deferrals rather than blocking the release
  verdict on owner-gated externals.
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
