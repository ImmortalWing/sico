# STEP-0160: M17 session audit — image contract, carrying proof, CV contract, honest gap register

> - status: complete — **audit verdict: NO-GO（整体 M17 未完成，按 gate 逐项诚实登记）；两块 GO 子项**
> - phase: M17 (M17 plan §2/§3/§6 partial)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - inputs: RFC-0041 (accepted D1–D4); STEP-0161 binary-carrying; RFC-0043 draft

## What landed this window

1. **RFC-0041 accepted** (D1 packed, D2 top-left normalized, D3 explicit
   conversion, D4 2^28-byte ceiling) — the portable image contract is
   frozen.
2. **M7 binary-carrying measured proof** (M17 plan §2.3 gap, now
   measured): `crates/sico-package/tests/binary_carrying.rs` builds a
   package carrying 1 MiB + 8 MiB deterministic binary resources,
   verifies per-resource digests, re-verifies deterministically, and
   records the evidence (`docs/evidence/m17/binary-carrying.json`) —
   the M7 boundary demonstrably carries model-scale assets with
   bounded load.
3. **RFC-0043 draft**: deterministic vision package roster
   (threshold/color/match/grid), integer-only determinism contract,
   oracle + content-assertion corpus rules, purity requirements.

## Per-gate verdicts (M17 plan §6)

| # | Gate | Verdict |
|---|---|---|
| 1 | Deterministic baseline completes fixed corpus, oracle-matched | **NO-GO** — CV packages not implemented (RFC-0043 draft only) |
| 2 | Accelerated paths match reference on named hardware | **NO-GO** — §3.3 provider ADR not drafted |
| 3 | Package consumers don't modify compiler/Runtime | **GO（前置已证）** — STEP-0147/0149: consumer ran via lock + user WIT with zero core patches; the tetris chain will reuse exactly this path |
| 4 | Model/provider authority explicit and least-privilege | **NO-GO** — §3.4 model RFC not drafted |
| 5 | Resource/crash/malicious-asset corpora fail closed | **GO（资源半区）** — binary-carrying digest-verify + bounded load measured; crash/malicious-model corpora follow the provider work |
| 6 | Performance claims name hardware/provider/version | **GO（无声明）** — no acceleration claims exist to check |
| 7 | Full M0–M16 regression + explicit audit | **NO-GO 依赖** — M16 audit STEP-0159 GO，工作区回归绿；M17 自身 gate 1/2/4 未达 |

## Path to completion (next window, in order)

1. Implement RFC-0043 packages as pure Sico-source-built components
   (STEP-0147 builder path), numpy oracle per op, tetris grid/piece
   recognition chain consuming `sico:image@0.1.0` bitmaps from
   STEP-0155 capture frames.
2. §3.3 provider ADR + §3.4 model RFC (draft, owner acceptance).
3. Re-run this audit; gates 1/2/4 flip with corpus evidence.
