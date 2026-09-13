# STEP-0159: M16 exit audit

> - status: complete — **audit verdict: GO（核心循环 + 真实语料）；两处范围声明**
> - phase: M16 exit (M16 plan §7)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - inputs: STEP-0145/0150/0155; RFC-0040 (accepted, A1); ADR-0013 (accepted, A1); threat model F-1/F-2 ruled

## Per-gate verdicts (M16 plan §7)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | Real Windows observe/preview/execute-one/verify vs controlled fixture, raw logs | **GO** | STEP-0155: `docs/evidence/m16/real-loop-commit-{1,2}.jsonl` — chained digests `5360715f→b3be5a2c→f7ec8d7b`, fixture app pinned (own code, no hardening) |
| 2 | Drift/stale/revocation/duplicate/limit+1 fail closed with typed outcomes | **GO** | synthetic 15/15 (STEP-0150) + real-adapter corpus: duplicate→TokenInvalid, stale (real geometry move)→StaleRevision, capture limit+1→Limit, surface death→SurfaceInvalid + token drain (`corpus.jsonl`) |
| 3 | Authority mutation corpus cannot widen; every §5.1 threat has a test | **GO（合成层）** | STEP-0150 corpus rows T1–T8 (T9/T10 are non-goal assertions + carried M7 corpus); real-adapter authority refusals in the corpus above |
| 4 | Cancellation + Host crash recovery leave no orphan input/resource | **GO（点状）** | fixture process kill mid-session: Host driver survives, no orphan input (one atomic SendInput completes before teardown), token drain asserted; full DAP-style cancellation matrix stays M10 evidence |
| 5 | Long-run dry-run/bounded action budgets recorded | **GO** | 10-action burst: handles 112→112 (stable), RSS 8.09→8.61 MB (bounded, small multiple of session data) (`corpus.jsonl` budgets record) |
| 6 | Only native-tested platforms advertised; synthetic labelled | **GO** | Windows x64 only; STEP-0150 explicitly `contract-verified` |
| 7 | Full M0–M14 regression + explicit audit | **GO** | workspace 371/0; runner green solo（同 M15 gate 7 的环境敏感备注） |

## Overall: GO

M16 的能力模型（capture ≠ input ≠ keyboard）、preview/commit 单次动作、
dry-run 宿主强制、审计流、紧急停止全部成立，合成与真实两层语料齐备。
E9xxx 宿主侧拒绝身份已由 E8/E9 分族预留（guest 编译面为后续 RFC）。
