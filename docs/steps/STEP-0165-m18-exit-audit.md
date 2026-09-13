# STEP-0165: M18 exit audit

> - status: complete — **audit verdict: GO（内部组合 5 项中的 4 项）；两项诚实 NO-GO 登记**
> - phase: M18 exit (M18 plan §7)
> - completed: 2026-09-13
> - owners: autonomous-agent
> - inputs: STEP-0149/0156/0162/0164; M12/M15/M16/M17 audit states

## Per-gate verdicts (M18 plan §7)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | Portfolio builds and runs without core patches | **GO（4/5）** | API agent (2/2), streaming tool (2/2), Web/UI app (Edge 152 executed), package consumer (log-analyzer 9/9); native-visual pilot blocked on M17 GO — honest register |
| 2 | Security/fault/cancellation/update/long-run reproducible | **GO（范围内）** | security/fault: per-pilot fail-closed corpora + M12/M16 carried corpora; long-run: STEP-0155/0159 budgets; update rehearsal: carried M7 corpus; cancellation: M10/M11 carried |
| 3 | AI generation/repair/comprehension measured via frozen protocol | **GO（offline 类）** | M13/M14 协议与语料已冻结且可复现；live-model 数字仍以 STEP-0128 (0.9095) 为准，M14 gate 7 重测仍 owner 凭据门控 — 不重复计数 |
| 4 | External pilot for external-adoption claims | **NO-GO（诚实）** | external-pilot 类需要独立参与者与环境 — owner 外部输入；本轮不可制造。无任何 external-adoption 声明被作出 |
| 5 | Product/platform claims map to exact evidence | **GO** | every claim links to runner/browser engine runs with versions pinned (Edge 152.0.4191.66; Windows x64) |
| 6 | Residual risks explicit | **GO** | 见下 |
| 7 | Full M0–M17 regression green + explicit audit | **GO** | workspace 372/0; runner green solo（2 个环境敏感预算断言记录在案） |

## Residual risks / deferred (explicit)

1. **Pilot 4（block-game 视觉自动化）**: blocked on M17 GO（RFC-0043 CV 包
   wasm 实现是剩余主体）。M17 完成路径已在 STEP-0160 登记。
2. **External pilot（gate 4）**: 需要独立第三方参与者——owner 外部输入；
   在此之前 M18 的一切证据都是 `internal-fixture`/`clean-room-consumer`
   类，不作 external-adoption 声明。
3. **Live-model 重测**: M14 gate 7（AI 生成/修复的 live 回归）仍等待
   owner 提供 DeepSeek 凭据；offline 协议可复现。
4. **Release-bundle 安装**: pilots 本轮经 cargo 构建产物运行；M19 §3.2
   的安装/升级 bundle 完成后需重放一次 pilot 启动作为确认。
