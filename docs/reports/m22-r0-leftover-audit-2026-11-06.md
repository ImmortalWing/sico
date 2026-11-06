# M22 R0 遗留项审计（只读审计片，STEP-0568）

- 日期：2026-11-06
- 分支：`codex/m22-w2`，HEAD `a939862`（审计开始时工作树除 `selfhost/match-component-coverage-v1.json` 一处**既有**改动外干净；该改动非本片产生，全程保留）
- 性质：**只读审计片**。未改 `selfhost_compiler.rs`、未改任何既有文件、未改 gate 文本、未翻任何门。M22 保持 **NO-GO**。
- 本片只新建本报告与 [`docs/steps/STEP-0568-m22-w2-r0-leftover-audit.md`](../steps/STEP-0568-m22-w2-r0-leftover-audit.md)。

对账对象：AGENTS.md §10.11 A（STEP-0535 立即整改五项）与 §10.11 A2，逐项给出文件/行号证据、状态（开/闭/部分）、如开则最小下一步。

---

## 结论速览

| # | 对账项 | 状态 | 一句话结论 |
|---|---|---|---|
| a | `tools/validate-m22-s5.ps1` 版本化 + fast 子集 | **部分** | 主电池已版本化入库；`validate-m22-s5-dev.ps1` / `m22-s5-fingerprint.ps1` 在 HEAD `a939862` **尚未 versioned**（仅作 untracked 文件存在于盘上，由并发片 STEP-0565 在本片运行期间创建）；fast 子集在**已提交树**上全绿，仅在**脏工作树**（未提交账本）下 emitter-vocabulary census 红 |
| b | 畸形 IR fail-closed 测试（R0.3） | **闭** | 4 个畸形形状双方 typed 拒，测试名可列出，**实测绿** |
| c | R0.5 布局/边界测试 | **闭** | 6 个形状逐字节等同 + I64 checked 明确拒绝，**实测绿** |
| d | 文档卫生（R0.1 旧声明订正） | **闭**（含 1 处**反向陈旧**） | 42/519 已作废、旧审计链接均存在、STEP-0320 仅存历史条目；但 handoff/plan 仍称「三个 scratch 文件原样保留在工作区」而它们**已不在** |
| e | 三个 scratch 遗留文件 | **闭**（卫生护栏=部分） | 三文件均不在工作树、全分支**从未被 track**；但 `.gitignore`/候选卫生**无按名护栏**，仅靠「不存在」保护 |

---

## a) `tools/validate-m22-s5.ps1` 版本化与 fast 子集

**版本化：已入库。**
- `git ls-files tools/ | grep validate-m22` → `tools/validate-m22-s5.ps1`（148 行 / 6581 B，含 fmt×2、clippy×2、`ci-one-key`、0245、0262、前端前沿、IR 计划前沿、comment-discipline、match-site-census、match-site-coverage、emitter-vocabulary-census、firmware-selfhost、`git diff --check` 共 15 步，`-SkipCi` 可跳过 `ci-one-key`）。

**§10.11 A2 与现状不符（新发现，登记为文档/实现漂移）。**
- A2 把「分层策略落地」标为 `[x]`，声称 `tools/validate-m22-s5-dev.ps1`（`-Set fast|probes|firmware|all`）与 `tools/m22-s5-fingerprint.ps1` 已入库。**在已提交 HEAD `a939862` 上两者尚未 versioned**：`git ls-files tools/ | grep -iE "m22-s5-dev|fingerprint"` → 空（exit 1）。
- 但两者**当前在盘上存在**（mtime `validate-m22-s5-dev.ps1` 17:51 / `m22-s5-fingerprint.ps1` 17:38），是**未 track 的 untracked 文件**，且由**并发片 STEP-0565（`docs/steps/STEP-0565-m22-w2-battery-dev-layer.md`）在本片运行期间创建**。即 A2 的工作正在由另一片落地，只是**尚未提交**。
- 结论口径：**「已入库/versioned」在 HEAD 上不成立**（untracked ≠ versioned）；「分层策略在盘上已落地」成立。A2 的 `[x]` 措辞（"入库"）比 HEAD 现状超前一步，待并发片提交后即对齐。
- 附带：`validate-m22-s5.ps1` 本身**没有 `-Set` 参数**，唯一能缩短的是 `-SkipCi`，但仍要跑 clippy + 0245 + 0262 + 两个前沿探针 + 整套固件（分钟到小时级），不是 fast 子集。真正的 fast 分层入口就是上述 `validate-m22-s5-dev.ps1 -Set fast`（尚未 versioned）。

**fast 可跑子集实测（本机实际执行，非推断）。** 本片把「能在秒级跑完、不 spin cargo/selfhost 全量」的子集视为 fast 可跑子集：

| 子步骤 | 命令 | 结果 | exit |
|---|---|---|---|
| 工作树卫生 | `git diff --check` | 无输出 | **0** |
| match 站点账本 | `python tools/census-m22-match-sites.py --check` | `LEDGER_CURRENT sites=396` | **0** |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` | **0** |
| R0.3 畸形 IR 钉 | 见 b) | `1 passed; 0 failed` | **0** |
| R0.5 布局/边界钉 | 见 c) | `1 passed; 0 failed` | **0** |
| **发射词表普查（脏工作树）** | `python tools/census-m22-emitter-vocabulary.py --check` | **`EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S`** | **1** |
| **发射词表普查（已提交账本）** | 同上，脏账本 `git stash` 后复跑 | **`EMITTER_VOCABULARY_CURRENT sites=336`** | **0** |

**emitter-vocabulary census 红的归因（实测）。**
- 两侧源码 sha **都匹配**，故红**不是** §10.11 A2 记录的「stale sha」旧因：
  - `guest_emitter.source_sha256 = 013f2d7e894994b3734646c6c9996c14e2e0d999a7fd37a37ef3ac5dc0854d0f` == 当前 `selfhost/codegen.sico` 的 sha256（逐字相等）。
  - `backend_helper_rule.source_sha256 = 220b8e47accf1afc0a01dd989697850eb238e65d44f4af90dc9fa9ec65fa116e` == 当前 `crates/sico-codegen-wasm/src/lib.rs` 的 sha256（逐字相等）。
- RED 判定在 `census-m22-emitter-vocabulary.py:474-477`：`frozen != artifact`（冻结工件 vs 由当前源码+账本重导的工件）。
- 与红直接相关的改动：工作树 `selfhost/match-component-coverage-v1.json`（**既有、非本片**）把 **4 个站点**（`fli_digits_offset` / `fli_u64_digits_offset` / `fli_sign_offset` / `fli_negative_digits_offset`，均为 B2 STEP-0551 的回绿站点）的 verdict 由 `ir-equal+component-equal` 改回 `ir-equal+component-other:FuelExhausted`。普查读这份账本 ⇒ 重导工件 ≠ 冻结工件 ⇒ 红。**这 4 个站点的回退与 handoff/STATUS 记录的「组件轴 36→32」方向一致。**
- **归因确认（本片实测，非推断）**：把脏账本 `git stash push -- selfhost/match-component-coverage-v1.json` 后对**已提交账本**复跑 census ⇒ **`EMITTER_VOCABULARY_CURRENT sites=336`，exit 0** ⇒ 红完全由**未提交的账本 verdict 回退**引起；已提交树上该步是绿的。复跑后 `git stash pop` 已把脏账本**原样恢复**（本片未留下 stash；`git stash list` 仅剩一条**既有** `stash@{0}: On codex/m22-w1-ci: Qoder 检出分支临时保存`，非本片创建）。

**未跑（超出 fast 子集，本机时间预算内未执行，如实登记）。** fmt(root/runner)、clippy(root/runner)、`ci-one-key`、`validate-step-0245`、`validate-step-0262`、`probe-m22-frontend-frontier`、`probe-m22-ir-plan-frontier`、`probe-m22-match-coverage`、`firmware-selfhost` 整套。理由：这些步骤各自 spin `cargo`/selfhost，单步分钟级、固件整套约 6000 s 量级，与「fast」定义不符；本片只取秒级子集。**未跑 ≠ 通过。**

**最小下一步（如开）。** ①`validate-m22-s5-dev.ps1` / `m22-s5-fingerprint.ps1` 已在盘上（并发片 STEP-0565 交付），**待其提交后 A2 的「入库」即成立**；本片无需另做，只登记 HEAD 上尚未 versioned。②emitter-vocabulary census 在脏工作树会红；账本 verdict 定稿后由 reviewer `--update` 刷新冻结工件（本片不改），或在跑 fast 子集前先提交/暂存账本。

---

## b) 畸形 IR fail-closed 测试（R0.3）—— 闭

**位置**：`runner/sico-runner/tests/selfhost_compiler.rs:10509`（`#[test]`）/:10510（`fn the_malformed_canonical_ir_is_refused_by_the_guest_and_by_the_rust_oracle`）。

**三个点名形状全部在册，且多一个**（`:10538-10559`），每个都同时断言 **Rust oracle 拒**（`:10561` `compile_script_program(&module).is_err()`）与 **guest typed 拒且身份逐字相等**（`:10564-10572`）：

| label | 夹具（行号） | 要求的 guest 拒绝身份 |
|---|---|---|
| `same-slot-count-wrong-type-projection` | `project_type_mismatch_module` `:10431` | `ERR:E-SH-CG-IR-PROJECT-TYPE` |
| `variant-pattern-over-a-non-variant-value` | `variant_over_scalar_module` `:10447` | `ERR:E-SH-CG-IR-MATCH-VARIANT` |
| `literal-pattern-over-a-non-boolean-value` | `bool_over_scalar_module` `:10465` | `ERR:E-SH-CG-IR-MATCH-SCALAR` |
| `nested-payload-pattern` | `nested_payload_module` `:10484` | `ERR:E-SH-CG-IR-EMIT` |

- **正对照**（`:10521-10536`）：同一夹具、未畸形的 `match_shape_module`（`:10212`）必须仍能发射生产组件并过真实 `wasmparser::Validator` ⇒ 下面的拒绝是「畸形本身」而非「夹具坏了」。
- 注入夹具：`guest_emission` `:10112`、`injected_ir` `:10150`、`decode_hex` `:239`。

**测试名可列出（实测）**：
- `selfhost_compiler-4fa1b02d73be6796.exe --list | grep malformed_canonical_ir` → `the_malformed_canonical_ir_is_refused_by_the_guest_and_by_the_rust_oracle: test`，exit 0。

**实测绿**：
- `... selfhost_compiler-4fa1b02d73be6796.exe the_malformed_canonical_ir --nocapture` → `test the_malformed_canonical_ir_is_refused_by_the_guest_and_by_the_rust_oracle ... ok` / `1 passed; 0 failed; ... 191 filtered out; finished in 2.57s`，exit **0**。

**状态：闭。** 三个点名形状（+1 额外）双方 typed 拒、身份可归因、正对照在位、实测绿。

---

## c) R0.5 布局/边界测试 —— 闭

**位置**：`runner/sico-runner/tests/selfhost_compiler.rs:10576`（`#[test]`）/:10577（`fn the_result_layout_and_the_checked_arithmetic_boundaries_are_pinned`）。

**六个布局/边界形状逐字节等同**（`:10582-10607`），每个都 `decode_hex(hex) == expected`（`:10598`，**不是只比长度**）且过真实 `wasmparser::Validator`（`:10592`）：

| label | 夹具（行号） |
|---|---|
| `ok-arm-projection` | `match_shape_module` `:10212` |
| `result-cell` | `result_cell_module("ok")` `:10222` |
| `error-arm-projection` | `result_cell_module("error")` `:10222` |
| `bool-pattern` | `bool_pattern_module` `:10336` |
| `u64-add-overflow-check` | `match_shape_module` `:10212`（`checked_add` 形状） |
| `u64-sub-underflow-check` | `checked_sub_module` `:10412` |

**I64 checked 的明确拒绝**（`:10612-10626`）：oracle **能发**（`i64_checked_module` `:10369`，`signed_oracle.is_ok()`），guest **typed 拒** `ERR:E-SH-CG-IR-EMIT` ⇒ 正是 R0.5 要求的「I64 checked 的明确拒绝」而不是静默近似。

**实测绿**：
- `... selfhost_compiler-4fa1b02d73be6796.exe the_result_layout_and_the_checked --nocapture` → `test the_result_layout_and_the_checked_arithmetic_boundaries_are_pinned ... ok` / `1 passed; 0 failed; ... 191 filtered out; finished in 3.16s`，exit **0**。

**测试二进制新鲜度（保证绿反映当前树）**：`selfhost_compiler-4fa1b02d73be6796.exe` mtime `2026-11-06 17:35:40`，晚于其全部输入 —— `selfhost_compiler.rs` `15:20:23`、`selfhost/codegen.sico` `15:52:29`、`selfhost/parser.sico` `15:59:47`、`target/debug/sico.exe` `11-04 18:27`。

**状态：闭。** R0.5 点名的 ok/error 投影、Result cell、布尔模式、I64 checked 拒绝、U64 add 溢出 / sub 下溢全部在册、逐字节断言、实测绿。

---

## d) 文档卫生（R0.1 旧声明订正）—— 闭（含 1 处反向陈旧）

本片只读核查，**未编辑**任何 STATUS/plan/handoff（reviewer 所有）。

**① 「42 / 519 形状面」旧口径：已作废。**
- `docs/STATUS.md:39`（STEP-0535 条目 ⑤）：「**旧口径「组件层 42 / 519（形状面）」作废**（那是按形状归并的外推，不是逐站点证据）」，并降为「组件级全等 **8 / 42**」。
- `docs/handoff-m22.md:34`：同句「旧口径…作废」。
- `docs/plans/M22-compiler-self-host.md:355`：已改用逐站口径（组件级全等 8/42 ⇒ 后续 9/396 ⇒ 36/396）。
- 现存「519」出现处均指**合法普查口径**——`match` **语句总数 519**（`docs/STATUS.md:40` ④「519 是全部（顶层 478 + 嵌套 41）」、plan:355「前端 57/519」），不是被作废的形状面 claim。`grep -rnE "42 */ *519|42/519"` 在三份当前文档里**只命中作废句本身**，无现存断言。**状态：闭。**

**② 旧审计链接：均有效，旧报告原文保留 + addendum。**
- 引用的报告文件全部存在：`docs/reports/m22-exit-audit-2026-09-28.md`、`m22-exit-audit-2026-10-20.md`、`m22-exit-audit-2026-10-25.md`、`m14-m26-milestone-audit-2026-09-24.md`、`m22-review-2026-10-26.md`（逐个 `[ -f ]` ⇒ OK）。
- 口径纪律在册：`docs/STATUS.md:16` / `:230` / `:231`、`docs/handoff-m22.md:29` 均写明「旧报告原文保留 + addendum 指向新报告」。**状态：闭。**

**③ STEP-0320：仅存历史条目，非现存 claim。**
- `docs/STATUS.md:222` / `:224` 是「previous update (2026-09-30 …)」历史条目；`docs/plans/M22-compiler-self-host.md:205` 是历史叙述。当前 `current step` 指针链（`STATUS.md:231-244`，最旧到 STEP-0535）与文件头部（STEP-0560）均不指向 STEP-0320。**无陈旧「当前」引用。状态：闭。**

**④ 反向陈旧（新发现，需 reviewer 订正，本片不改）。**
- `docs/handoff-m22.md:23` / `:34` / `:35`（及 plan 相应处）仍称「**三个他人遗留文件未纳入提交、原样保留在工作区**：`cl_argument_instruction_count.sico`、`crates/sico-codegen-wasm/tests/scratch_dump.rs`、`tools/fix-selfhost-comment-words.py`」。**实测这三文件已不在工作树**（见 e)）。即文档**低估**了候选卫生现状——与 R0.1 要修的「高估」方向相反，但同属口径漂移。**最小下一步**：reviewer 把这三处改为「三文件已清理/已不在工作树」。

**⑤ 观察项（非违规，不改）。** `docs/handoff-m22.md:23` / `:34` 记录 STEP-0533/0535 推送到旧分支 `codex/m22-w1-ci`；当前分支为 `codex/m22-w2`。属那两片的**历史记录**，非现存错误 claim。

---

## e) 三个 scratch 遗留文件 —— 闭（卫生护栏 = 部分）

| 文件 | 工作树存在？ | git track（全分支）？ | 证据 |
|---|---|---|---|
| `cl_argument_instruction_count.sico` | **否**（`find . -name …` 全树无命中，含 target 外全路径） | **从未** | `git log --all --oneline -- <path>`（含 `selfhost/` 前缀变体）→ 空 |
| `crates/sico-codegen-wasm/tests/scratch_dump.rs` | **否**（`ls` → No such file） | **从未** | `git log --all …` → 空 |
| `tools/fix-selfhost-comment-words.py` | **否**（`ls` → No such file） | **从未** | `git log --all …` → 空 |

- `git status --porcelain --untracked-files=all` 在**本片审计时刻**对这三个名字**零命中**（`grep -iE "scratch|cl_argument|fix-selfhost"` exit 1）⇒ **无任何 untracked scratch 文件**。审计开始时工作树只有一处**既有**改动 ` M selfhost/match-component-coverage-v1.json`；本片运行期间**并发片**另加了 ` M .github/workflows/m22-w1-ci.yml`、` M tools/probe-m22-corpus-parity.ps1` 及若干 untracked 文档/工具（含 STEP-0564/0565/0566/0569/0570 等），但这些都与本片三项 scratch 无关，且本片未触碰。
- **是否被 .gitignore / 候选卫生覆盖？**
  - `.gitignore` 无任何匹配 `scratch` / `fix-selfhost` / `cl_argument` 的条目（grep 空）。
  - `git check-ignore -v` 对三个路径（含 `selfhost/` 前缀变体）返回空、exit 1 ⇒ **均未被 ignore**。
  - 候选卫生校验器 `tools/validate-selfhost-comment-discipline.py` 只查**注释内容**（`FORBIDDEN` 结构词），**不查散落文件**（grep `scratch|leftover|hygiene|untracked|git status|candidate` 空）。
- **状态：**「进入候选提交」风险 = **闭**（三文件不存在 + 全分支从未 track ⇒ 无法进 commit）；「卫生护栏」= **部分**——**没有按名的 `.gitignore`/候选卫生规则**覆盖它们，当前仅靠「文件不存在」保护；若将来重建同名文件，会以 untracked 形式出现且**无自动护栏**拦截。
- **最小下一步（如需护栏）。** 由 reviewer 决定是否在 `.gitignore` 加这三名的显式条目，或在候选卫生校验里加「禁止这三个路径入库」的断言；否则维持「靠不存在保护」并在文档里如实记为「无护栏」。

---

## 实测命令与退出码汇总（全部本机实际执行）

| 命令 | 关键输出 | exit |
|---|---|---|
| `git rev-parse --abbrev-ref HEAD` / `--short HEAD` | `codex/m22-w2` / `a939862` | 0 |
| `git ls-files tools/ \| grep validate-m22` | `tools/validate-m22-s5.ps1`（dev/fingerprint 在 HEAD **未 versioned**，盘上 untracked 存在） | 0 |
| `git ls-files tools/ \| grep -iE "m22-s5-dev\|fingerprint"` | 空（exit 1）⇒ HEAD 上未入库 | 1 |
| `git diff --check` | 无输出 | 0 |
| `python tools/census-m22-match-sites.py --check` | `LEDGER_CURRENT sites=396` | 0 |
| `python tools/validate-selfhost-comment-discipline.py` | `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` | 0 |
| `python tools/census-m22-emitter-vocabulary.py --check`（脏账本） | `EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S` | **1** |
| 同上（`git stash` 脏账本后，对已提交账本） | `EMITTER_VOCABULARY_CURRENT sites=336`；`git stash pop` 已原样恢复 | **0** |
| `…selfhost_compiler-4fa1b02d73be6796.exe --list \| grep -iE "malformed_canonical_ir\|result_layout"` | 两个测试名均列出 | 0 |
| `…exe the_malformed_canonical_ir --nocapture` | `1 passed; 0 failed`（2.57s） | 0 |
| `…exe the_result_layout_and_the_checked --nocapture` | `1 passed; 0 failed`（3.16s） | 0 |
| sha256 对账（codegen.sico / lib.rs vs 冻结工件） | 两侧 sha 逐字相等 | 0 |
| `git diff selfhost/match-component-coverage-v1.json` | 4 站点 verdict `component-equal`→`FuelExhausted` | — |
| `git log --all --oneline -- <三个 scratch 文件>` | 三者均空 | — |

## 残余风险 / 未跑项

1. **emitter-vocabulary census 在脏工作树红 / 已提交树绿**：归因已实测确认——红完全由工作树里**未提交的账本 verdict 回退**（4 站点 `component-equal`→`FuelExhausted`）引起；对已提交账本复跑 = `EMITTER_VOCABULARY_CURRENT sites=336` exit 0。**本片不 `--update` 刷新冻结工件、不翻门**；账本 verdict 定稿后刷新工件归 reviewer。
2. **fast 子集未全覆盖**：fmt / clippy / ci-one-key / 0245 / 0262 / 两个前沿探针 / match-site-coverage / 固件整套 **未跑**（时间预算），**未跑 ≠ 通过**。
3. **A2「入库」措辞在 HEAD 上超前**：`validate-m22-s5-dev.ps1` / `m22-s5-fingerprint.ps1` 已在盘上（并发片 STEP-0565 交付）但**尚未 versioned**（untracked）。待并发片提交即对齐；本片不改 AGENTS.md。
4. **handoff/plan 的 scratch 三文件陈述已陈旧**（称仍保留在工作区，实测已清理），需 reviewer 订正。本片不改这三份文档。
5. **并发片干扰**：本片运行期间工作树被并发 agent 改动（新增 `.github/workflows/m22-w1-ci.yml`、`tools/probe-m22-corpus-parity.ps1` 的 M，及多个 untracked 文档/工具）。本片所有结论均带时刻戳，且未触碰这些文件。对已提交账本的 census 复跑通过 `git stash push/pop` 完成，**未留下 stash**（`git stash list` 仅剩一条既有 w1-ci stash）。
6. 本片全程只读既有文件；新建仅本报告与 STEP-0568 文档。未 commit、未 push、未碰远端。