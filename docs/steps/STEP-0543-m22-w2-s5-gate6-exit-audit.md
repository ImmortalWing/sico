# STEP-0543 · M22 W2 S5 · 门 6 出口审计片（`docs/reports/m22-exit-audit-2026-10-25.md`）

> **本片性质** = **文档片**（`crates/` / `runner/` / `selfhost/` **零改动**）⇒ **不重跑整轮电池**，只跑相关校验器 + `git diff --check`。
> **只推进一个门**：**门 6 由 NO-GO 升为 GO**（锚点 = 独立 CI **run #52**，提交 `9f0b7a3`，`completed / success`）。
> **M22 整体仍为 NO-GO**：门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4 与门 7 不得沿用 GO。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| 有**干净提交 SHA** 上的完整 M0–M21 回归 + 独立 CI | ✅ | **run #52** = `head_sha 9f0b7a3`、run id `40162446092`、job id `121076643414`、`windows-gnu`、04:35:39Z → 05:33:13Z = **57m34s**、**全部步骤 success** |
| 一键 CI 含**完整 M0–M21 回归** | ✅ | `tools/run-ci.ps1` 的 `test (workspace)` = `cargo test --locked --offline --workspace --all-targets --all-features` |
| 旧报告原文保留 | ✅ | 本片只给 `m22-exit-audit-2026-10-20.md` **加 addendum**，**不改历史结论** |
| 不改 `crates/` / `runner/` / `selfhost/` | ✅ | 本片只动 `docs/` |

## 1. 本片方法

1. **先读门定义**：M22 计划 §4「Exit gates」（7 条）+ `AGENTS.md` §10.1（owner 审计，2026-10-24 的**上限纪律**）。
2. **再取可执行证据**：匿名 GitHub API 逐 run 查 `status` / `conclusion` / `head_sha`；
   逐门对照**已执行**的 runner 证据，不引用未跑的命令。
3. **按上限纪律裁决**：文档与可执行证据冲突时**取更低的支持声明**（`AGENTS.md` §1 第 4 条）。
4. **保留失败**：CI 的五次失败（#17 / #18 / #39 / #44 / #46）**如实留在证据矩阵里**，不折叠、不改写。

## 2. 实测结果

### 2.1 逐门裁决（1–7）

| 门 | 裁决 | 依据 |
|---|---|---|
| 1 L1 formatter/checker | **GO（限声明子集）** | formatter canary 30/30 + `full_source_exit: 0` + `FORMATTER-CLOSED`；checker 语料电池 `0245` exit 0 |
| 2 L2 冻结语料 / 组件级字节全等 | **PARTIAL** | **前端轴 396 / 396（前端就拒 0 条）**：其中 **177** 站点有逐站 IR sha（32 组件全等 + 140 燃料耗尽 + 5 拒绝对照），**219** 站点以**整源差分钉**（`sico_guest_arena_accommodates_full_parser_selfhost_input`）为 IR 见证（闭包过大 ⇒ 组件轴未测）；组件级全等 **32 / 396**；**语料覆盖仍 0 条**；37/178 语料与 bundle 未做 |
| 3 ADR-0015 `A == B == C` + M7 `.sapp` | **NO-GO** | guest 尚未产出组件 B；三阶段字节证据、`.sapp` 链、hostile / limit+1 全部未演示 |
| 4 自编译预算（`B(S)`） | **不得沿用 GO** | 只有 **IR 层**观测（25.93 s / fuel 上限 800B / arena 2 GiB + treadmill / hostcall 8 GiB / timeout 120 s）；**`B(S)` 未实测** ⇒ cold/warm wall、fuel 读数、峰值内存、hostcall 计数、I/O、timeout/cancellation、M19 对照**全缺** |
| 5 双实现登记 | **GO** | Rust 后端仍是唯一 byte oracle；Sico 为第二实现；两实现均未被删除或降格 |
| 6 M0–M21 回归 + 独立 CI | **GO** | **run #52**（`9f0b7a3`）= `completed / success`，全部步骤 success；#47 → #52 **连续 6 次 success**，每次绑定精确 SHA |
| 7 证据与支持级别 | **不得给完成信用** | `AGENTS.md` §10.11 的未闭合项仍在（语料覆盖 0、整文件自举未关闭、`M-g3′`/`M-g2`/`M-g3` 未闭合、闭包规模 219 未测、组件轴净增被回退吃掉） |

**总裁决：M22 仍为 NO-GO。**

### 2.2 与上一份审计（STEP-0485 / `m22-exit-audit-2026-10-20.md`）的差异

| 门 | 10-20 报告 | 本片 | 原因 |
|---|---|---|---|
| 1 | GO | GO | 不变 |
| 2 | PARTIAL | PARTIAL | 不变 |
| 3 | NO-GO | NO-GO | 不变 |
| **4** | **GO** | **不得沿用 GO** | `AGENTS.md` §10.1（2026-10-24）晚于该报告（2026-10-20）⇒ **上限纪律收紧** |
| 5 | GO | GO | 不变 |
| **6** | **NO-GO**（未在提交快照上复跑） | **GO** | **run #52** 在干净提交 `9f0b7a3` 上 completed/success ⇒ 该限定**已过时** |
| **7** | **GO** | **不得给完成信用** | 同 4：`AGENTS.md` §10.1 |
| 总裁决 | NO-GO | **NO-GO** | 不变 |

> **口径纪律**：本片**只推进门 6**，**不**借机放宽门 4 / 门 7。
> 文档与可执行证据冲突时**取更低的支持声明**（`AGENTS.md` §1 第 4 条）。

### 2.3 CI 证据链（`codex/m22-w1-ci` 分支）

| run | head_sha | 结论 | run | head_sha | 结论 |
|---|---|---|---|---|---|
| #13–#16 | — | success | #46 | `9c2657b` | **failure** |
| #17 | `7e5ad3c` | **failure** | #47 | `2f3c069` | success |
| #18 | `6808569` | **failure** | #48 | `7e602e2` | success |
| #19–#22 | — | success | #49 | `5c61771` | success |
| #23–#38 | — | success | #50 | `f01e923` | success |
| #39 | `504c849` | **failure** | #51 | `0d49ec7` | success |
| #40 | `9e5dfa8` | success | **#52** | **`9f0b7a3`** | **success ✅ ← 门 6 锚点** |
| #41 | `eec5456` | success | | | |
| #42 | `3b88469` | success | | | |
| #43 | `54b6d97` | success | | | |
| #44 | `da01829` | **failure** | | | |
| #45 | `f83ab7b` | success | | | |

**五次失败如实在册**；本机**未取到** #39 / #44 的完整 Actions 日志（`gh` 未安装、匿名 API 不返回 job 日志正文）
⇒ 本片**不**对它们做归因断言，**只**登记「失败」这一事实。

## 3. 交付物（机械清单）

1. **新建** `docs/reports/m22-exit-audit-2026-10-25.md`（逐门裁决 1–7 + 证据矩阵 + 剩余缺口）。
2. **旧报告** `docs/reports/m22-exit-audit-2026-10-20.md` **加 addendum**（指向新报告，**不改历史结论**）。
3. **更新引用**（5 处）：
   - `docs/STATUS.md` 的 `> - updated:` **新增**本片条目；
   - `docs/STATUS.md` 的 `phase status:` 行里「current exit review」链接 → 新报告；
   - `docs/handoff-m22.md` **新增**本片条目；
   - `docs/plans/M22-compiler-self-host.md` 顶部 `Status:` 行 → 新门状态 + 新报告链接；
   - `docs/plans/M22-compiler-self-host.md` §量化基线表的 `当前门状态` 行 → 新门状态 + 新报告链接。
4. `docs/steps/README.md` **新增**本片条目。
5. **报告索引** `docs/reports/README.md` 补两行：本片的新报告 + 此前漏登记的 `m22-exit-audit-2026-10-20`。
6. **顺带修掉 7 处失效 ADR 链接**：`docs/steps/*.md` 里写成 `../../adr/...`，而 ADR 实际在 `docs/adr/` ⇒ 正确写法是 `../adr/...`；
   分布 = **STEP-0486 ×1 / STEP-0514 ×4 / STEP-0534 ×1 / STEP-0535 ×1**（历史文档，**只改链接路径，不改文字**）；
   本片新建的两份文档的 ADR 链接本来就写对（`../adr/...`）⇒ **修完后 `docs/steps` 内失效 ADR 链接 = 0**（已复验）。

## 4. 验证

本片是**文档片** ⇒ 按 `AGENTS.md` §8「documentation-only roadmap changes」的最低要求：

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\validate-step-0124.ps1
git diff --check
```

（**不**重跑整轮版本化电池：本片无 `crates/` / `runner/` / `selfhost/` 改动。）

## 5. 诚实收窄

- 本片**只**推进门 6；门 6 的 GO **只**覆盖「干净提交 SHA 上的完整 M0–M21 回归 + 独立 CI 绿」，
  **不**覆盖自举闭环（门 3），**不**覆盖语料覆盖。
- 门 4 与门 7 **被收紧**（相对 10-20 报告），这是 `AGENTS.md` §10.1 的要求，**不是**本片的新发现。
- 140 个 `FuelExhausted` 站点**不是**闭合；219 个 `closure-too-large` 站点是**未测**，不是**通过**。
- 证据等级仍是 `internal-fixture` ⇒ **不是** `clean-room-consumer` / `external-pilot` / `production`。
- **M22 整体仍为 NO-GO**；门 3 是主欠。
