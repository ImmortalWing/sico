# STEP-0568 · M22 W2 · R0 遗留项审计（只读审计片）

- 日期：2026-11-06
- 分支：`codex/m22-w2`，HEAD `a939862`
- 报告：[`docs/reports/m22-r0-leftover-audit-2026-11-06.md`](../reports/m22-r0-leftover-audit-2026-11-06.md)
- 性质：**只读审计片**。零实现、零 gate 移动、M22 保持 **NO-GO**、不做完成声明。
- 热文件纪律：**未改** `selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`；**未改**任何 reviewer 所有文档（STATUS / handoff / plans / steps README / ROADMAP / rfc / adr）。本片**只新建**报告与本文档。

## 1. 片性质与入口门槛

对账 AGENTS.md §10.11 A（STEP-0535 立即整改五项）与 §10.11 A2 在当前树的现状，逐项给文件/行号证据、状态、最小下一步。这是**审计片**：不推进任何出口门，只登记证据与残余。

入口门槛（本片自设，审计片的合理门槛）：
- 只读，不改既有文件；
- 每项结论必须来自实际执行过的命令（命令 + exit + 关键输出）；
- 测不了/未跑的如实登记「未跑/被什么阻塞」，不包装。

## 2. 五项对账结论（详证据见报告）

| # | 项 | 状态 | 关键证据 |
|---|---|---|---|
| a | `validate-m22-s5.ps1` 版本化 + fast 子集 | **部分** | 主电池已 `git ls-files` 确认入库；`validate-m22-s5-dev.ps1` / `m22-s5-fingerprint.ps1` 在 HEAD **未 versioned**（盘上 untracked 存在，由并发片 STEP-0565 创建）；fast 子集在**已提交树**全绿，仅在**脏工作树**下 census 红 |
| b | 畸形 IR fail-closed（R0.3） | **闭** | `selfhost_compiler.rs:10510` 测试在册，4 个畸形形状双方 typed 拒，`--list` 可列名，**实测 1 passed exit 0** |
| c | 布局/边界（R0.5） | **闭** | `selfhost_compiler.rs:10577` 测试在册，6 形状逐字节等同 + I64 checked 明确拒绝，**实测 1 passed exit 0** |
| d | 文档卫生（R0.1） | **闭** + 1 处**反向陈旧** | 42/519 已作废（STATUS:39 / handoff:34 / plan:355）；旧审计报告 5 份全存在；STEP-0320 仅历史条目；但 handoff:23/34/35 仍称「三个 scratch 文件保留在工作区」而实测**已不在** |
| e | 三个 scratch 遗留文件 | **闭**（护栏=部分） | 三文件不在工作树、全分支从未 track；`.gitignore`/候选卫生**无按名护栏**（`git check-ignore` exit 1，comment-discipline 只查注释内容） |

## 3. 实测证据（命令 + exit + 关键输出）

环境：`export RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；python 经 `tools/ensure-python.ps1` 解析为 `target/tooling/python-3.12.10/python.exe`。

| 命令 | 关键输出 | exit |
|---|---|---|
| `git rev-parse --abbrev-ref HEAD` / `--short HEAD` | `codex/m22-w2` / `a939862` | 0 |
| `git ls-files tools/ \| grep validate-m22` | `tools/validate-m22-s5.ps1`（dev/fingerprint 在 HEAD 未 versioned） | 0 |
| `git ls-files tools/ \| grep -iE "m22-s5-dev\|fingerprint"` | 空 ⇒ HEAD 上未入库；但盘上 untracked 存在（17:51 / 17:38，并发片 STEP-0565 交付） | 1 |
| `git diff --check` | 无输出 | 0 |
| `python tools/census-m22-match-sites.py --check` | `LEDGER_CURRENT sites=396` | 0 |
| `python tools/validate-selfhost-comment-discipline.py` | `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` | 0 |
| `python tools/census-m22-emitter-vocabulary.py --check`（脏工作树） | `EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S` | **1** |
| 同上（`git stash push` 脏账本后，对已提交账本） | `EMITTER_VOCABULARY_CURRENT sites=336`；pop 后脏账本原样恢复 | **0** |
| sha256：`guest_emitter.source_sha256` vs `codegen.sico` | `013f2d7e…` == `013f2d7e…`（逐字相等） | 0 |
| sha256：`backend_helper_rule.source_sha256` vs `lib.rs` | `220b8e47…` == `220b8e47…`（逐字相等） | 0 |
| `git diff selfhost/match-component-coverage-v1.json` | 4 站点 `ir-equal+component-equal` → `ir-equal+component-other:FuelExhausted` | — |
| `…selfhost_compiler-4fa1b02d73be6796.exe --list \| grep …` | 两个 R0.3/R0.5 测试名均列出 | 0 |
| `…exe the_malformed_canonical_ir --nocapture` | `1 passed; 0 failed; 191 filtered out; 2.57s` | 0 |
| `…exe the_result_layout_and_the_checked --nocapture` | `1 passed; 0 failed; 191 filtered out; 3.16s` | 0 |
| `git log --all --oneline -- <三个 scratch 文件各自>` | 三者均空（从未 track） | — |
| `git check-ignore -v <三个 scratch 路径>` | 空、exit 1（未被 ignore） | 1 |
| 测试二进制 mtime vs 输入 | 二进制 `17:35:40` 晚于 `.rs 15:20` / `codegen.sico 15:52` / `parser.sico 15:59` / `sico.exe 11-04` | — |

### emitter-vocabulary census 红的归因

- 两侧源码 sha **都匹配** ⇒ 红**不是** §10.11 A2 记的旧因（stale sha）。红判定在 `census-m22-emitter-vocabulary.py:474-477`（`frozen != artifact`）。
- 与红直接相关的改动（**既有、非本片**）`selfhost/match-component-coverage-v1.json` 把 4 个 B2 回绿站点（`fli_digits_offset` / `fli_u64_digits_offset` / `fli_sign_offset` / `fli_negative_digits_offset`）verdict 由 `component-equal` 改回 `FuelExhausted`；普查读这份账本 ⇒ 重导工件 ≠ 冻结工件 ⇒ 红。方向与 STATUS/handoff 记录的「组件轴 36→32」一致。
- **归因确认（已实测）**：`git stash push -- selfhost/match-component-coverage-v1.json` 后对**已提交账本**复跑 census ⇒ **`EMITTER_VOCABULARY_CURRENT sites=336`，exit 0** ⇒ 红完全由未提交的账本 verdict 回退引起，已提交树该步绿。pop 后脏账本原样恢复。

## 4. 诚实收窄

- **未翻任何门、未改任何 gate 文本、M22 仍 NO-GO。** R0.3 / R0.5 的「闭」只指**这两类测试在当前树存在且实测绿**，不代表 R0 整体闭合，更不代表 M22 GO。
- b/c 的「实测绿」基于**预构建测试二进制** `selfhost_compiler-4fa1b02d73be6796.exe`（mtime 晚于全部输入，故反映当前树），**未从干净命令重编译 runner**（本机可能缺 gcc，未尝试重编译，见 §5）。
- emitter-vocabulary census 的红**未在本片处置**：不 `--update` 刷新冻结工件、不改账本（两者都归 reviewer；账本改动是既有 dirty，本片保留原样）。
- 文档卫生只做**只读核查**；发现的 1 处反向陈旧（scratch 三文件陈述）与 A2 措辞超前（dev/fingerprint 在 HEAD **未 versioned**、仅盘上 untracked，却标「已入库 `[x]`」）**只登记不修**（这些文件 reviewer 所有）。
- 本片运行期间**并发 agent** 在同一工作树加了 ` M .github/workflows/m22-w1-ci.yml`、` M tools/probe-m22-corpus-parity.ps1` 及多个 untracked 文档/工具（STEP-0564/0565/0566/0569/0570 等）。本片**未触碰**这些文件；所有结论带时刻戳，scratch 三文件的「不在工作树」结论在并发改动后**已复验仍然成立**。

## 5. 实测 / 未跑（诚实登记）

**已实测（本机执行，见 §3 表）。**

**未跑 / 被阻塞：**
1. **fast 子集未全覆盖**：fmt(root/runner)、clippy(root/runner)、`ci-one-key`、`validate-step-0245`、`validate-step-0262`、`probe-m22-frontend-frontier`、`probe-m22-ir-plan-frontier`、`probe-m22-match-coverage`、`firmware-selfhost` 整套 —— **未跑**。阻塞：这些步骤各自 spin `cargo`/selfhost，单步分钟级、固件整套约 6000 s 量级，超出「fast」定义与本片时间预算。**未跑 ≠ 通过。**
2. **runner 未从干净命令重编译** —— **未跑**。阻塞：本机可能缺 gcc，重编译可能失败；本片改用预构建二进制（新鲜度已验证）。因此 b/c 的绿是「预构建二进制上的绿」，不是「干净重编译上的绿」。
3. ~~对已提交账本的 census 确认复跑~~ —— **已实测**（census 单次约 500+ s），结果见下。

### 归因确认结果（实测，非推断）

- 对**已提交账本**（`git stash push -- selfhost/match-component-coverage-v1.json` 后）复跑 `census-m22-emitter-vocabulary.py --check`：**`EMITTER_VOCABULARY_CURRENT sites=336`，exit 0** ⇒ **确认**：census 红完全由工作树里那份**未提交的账本 verdict 回退**（4 站点 `component-equal`→`FuelExhausted`）引起，已提交树上该步是绿的。
- 复跑后 `git stash pop` 已把脏账本**原样恢复**（`git status` 仍含 ` M selfhost/match-component-coverage-v1.json`，非本片改动）。本片**未留下 stash**：`git stash list` 仅剩一条**既有** `stash@{0}: On codex/m22-w1-ci: Qoder 检出分支临时保存`（非本片创建，本片 push/pop 配对干净）。

## 6. 残余风险

1. emitter-vocabulary census 在**当前工作树**是红的（脏账本所致），已提交树是绿的；账本 verdict 定稿后需 reviewer `--update` 刷新冻结工件，否则 fast 子集会一直带红。本片不处置。
2. A2 的 dev/fingerprint 脚本已在盘上（并发片 STEP-0565 交付）但**在 HEAD `a939862` 上尚未 versioned**（untracked）；待并发片提交后 A2 的「入库」即成立。本片只登记 HEAD 现状，不代为提交。
3. handoff/plan 关于 scratch 三文件的陈述已陈旧（称仍保留在工作区，实测已清理），需 reviewer 订正；否则文档低估卫生现状。
4. b/c 绿依赖预构建二进制，未做干净重编译验证。
5. **并发片干扰**：本片运行期间工作树被并发 agent 改动（新增两个 M 文件 + 多个 untracked 文档/工具）。本片全程只读既有文件、未触碰并发片产物；scratch 三文件结论已复验。
6. 本片未 commit / 未 push / 未碰远端。