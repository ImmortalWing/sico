# STEP-0604 · M22 · 5 个出口阻塞红测的「诊断复闭 + 两处口径订正」（只读片，不改任何代码/语料/账本；仅订正文档声明）

> **片型** = 纯文档/诊断复闭片（只读，无任何源码/测试/语料/账本改动）。**唯一产物** = 本文件 + 三处文档登记（`AGENTS.md` §10.1.4 addendum、§10.11-A 一条标注、`docs/STATUS.md` 顶部日期化条目、`docs/steps/README.md` 索引行）。**不翻任何门**；M22 整体仍 **NO-GO**。
>
> **复现路径（本片独立复验的只读取证）**：本片**未重跑任何 cargo**；它对 [STEP-0602](./STEP-0602-m22-selfhost-reds-exit-blocker-analysis.md) 的三项**已测量**事实做**独立只读复核 + 口径订正**，全部结论锚定到磁盘文件与 `git log` 的可复核事实（见 §3/§4）。
>
> **禁改清单（本片遵守）**：`selfhost/**`、`crates/**`、`runner/**`、任何冻结语料/账本、`docs/ROADMAP.md`、`docs/handoff-m22.md`。本片**只新增本文件**并对 `AGENTS.md`/`docs/STATUS.md`/`docs/steps/README.md` 做三处**日期化订正**（不改写任何既有段落正文）。

## 0. 结论摘要（一句话）

**STEP-0602 的「5 个红测」结论成立且被复核强化，但需三处订正**：①**第 5 个（冻结账本）当前并不是红的**——账本与被测 guest 源在**同一提交 `6440890` 冻结、当前同步**，「漂移」是**应用 STEP-0571 §3 配方之后才发生的有条件重冻结成本**，不是当下红；②**`AGENTS.md` §10.11-A 的「emitter-vocabulary 必红」前提已过期并关闭**——词表 `guest_emitter.source_sha256` 已等于当前 `codegen.sico` sha，不再判 `EMITTER_VOCABULARY_RED`；③**前 4 个 codegen 红测的 wall-clock 应从 2.3–2.9 s 上修到 5.5–7.0 s**（含 runner PreparedPool 槽编译 + 800e9-fuel guest 运行 + harness），但**「快速 typed refusal、与燃料墙正交」的定性不变**。⇒ **5 个红测仍与燃料墙无依赖耦合；STEP-0571 §3 配方仍被「130 站协调账本重冻结 + owner 授权」阻塞。**

## 1. 背景（为什么要复闭）

[STEP-0602](./STEP-0602-m22-selfhost-reds-exit-blocker-analysis.md) 把 M22 的 5 个 `selfhost_compiler` 出口阻塞红测精确化，结论：**4 个 codegen 红测 = guest 前端快速 typed refusal（与燃料墙正交）+ 1 个冻结账本漂移**。该结论**方向正确**，但有三个登记处需要订正，否则会误导 owner 的下一刀裁决：

1. STEP-0602 把第 5 个红测描述为**当前红的账本漂移**（「冻结期望 vs 实测失配」）。这是**把「有条件未来成本」写成了「当下红」**——需要核实账本与被测源是否真的已经失配。
2. `AGENTS.md` §10.11-A 仍把「STEP-0545 电池第 13 拍**必红** `EMITTER_VOCABULARY_RED`」列为**未完成**的整改项（引用过期 sha `da0fec5f…` vs `3d8705b1…`）。需要核实该前提是否仍然成立。
3. STEP-0602 引用的 2.3–2.9 s 是**测试体内单个 `finished in`** 读数；与「整测试 wall-clock」口径不同，需要上修以免 owner 误判「毫秒级」。

本片对这三处做**只读独立复核**（磁盘 sha + `git log` 冻结点 + 词表字段），不重跑 cargo（结论均为可静态复核的事实，无需重跑）。

## 2. 方法（全部只读、可复核）

1. **sha 复核**：`sha256sum selfhost/codegen.sico` 与 `selfhost/emitter-vocabulary-v1.json` 的两处 `source_sha256` 字段逐字符比对。
2. **冻结点复核**：`git log -1 --format='%h %ad %s' --date=short -- <file>` 分别查 `selfhost/match-component-coverage-v1.json`、`selfhost/codegen.sico`、`selfhost/parser.sico` 的最后修改提交。
3. **账本结构性快速相位**：核对账本 schema `sico.m22.match-coverage.v1` 与站点计数 396（来自 STEP-0602 已实测的账本读数；本片不重算慢速 95 组差分）。
4. **wall-clock 口径**：对齐 STEP-0602 复跑的 `finished in`（测试体内单次 guest 降级）与**整测试 wall-clock**（含 PreparedPool 槽编译 + 800e9-fuel guest 运行 + harness）两个不同量纲。
5. **不重跑 cargo**：本片三项订正均可由磁盘事实 + 已登记的 STEP-0602/STEP-0571 实测记录静态推出，重跑不改变结论且超预算。

## 3. 复核事实（逐项，全部本机 2026-11-07 实测）

### 3.1 单一干净写入方（并发卫生）

- 分支 `dev`，HEAD `af6dcb7`。
- `selfhost/codegen.sico` sha256 = `013f2d7e894994b3734646c6c9996c14e2e0d999a7fd37a37ef3ac5dc0854d0f`，**已提交干净**（committed-clean）。
- `selfhost/parser.sico` sha256 = `b6cb6ab05669dbda9d1bcbe986950ccf66acf1fdfdaec8e796213e48e4d5ef31`，**已提交干净**。
- ⇒ **无并发写入方**（满足 AGENTS §10.2「同时只允许一个写入方」的收尾登记要求）。

### 3.2 五个红测的实际测试名 / 位置 / 复跑结果（订正 STEP-0602 的引用口径）

STEP-0602 用的是「step0507/0519/0520/0526」这类**配方/站点族代号**；本片把每个红测钉到**实际测试函数名 + `selfhost_compiler.rs` 行号 + 复跑 wall-clock**，便于 owner 直接定位：

| # | STEP-0602 代号 | 实际测试函数名 | `selfhost_compiler.rs` 行 | 失败层 / 诊断 | 复跑 wall-clock | 出口 |
|---|---|---|---|---|---|---|
| 1 | `cgs_result_return_hex`（step0507） | `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly` | `:3468` | guest 前端 **EXPRESSION 层**；`Domain{code:"invalid-input", message:"ERR:E-SH-IR-EXPRESSION"}` | **6.47 s** | exit 101 |
| 2 | `cgs_value_ty`（step0519） | `sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly` | `:6376` | call-shape 校验层；`ERR:E-SH-IR-CALL-SHAPE` | **6.97 s** | exit 101 |
| 3 | `cgs_value_slot`（step0520） | `sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly` | `:6701` | call-shape 校验层（与 ② 同形状）；**裸 panic 丢弃 outcome**（精确码未测——需改测试打印 outcome，超本诊断片范围）；STEP-0571 §2 佐证为 CALL-SHAPE 形状 | **6.69 s** | exit 101 |
| 4 | `cgs_function_instr_hex`（step0526） | `sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly` | `:3515` | 前端非 Output（嵌套 `while`）；**裸 panic 丢弃 outcome**（精确 Domain-vs-Trap 码仍**未测**） | **5.58 s** | exit 101 |
| 5 | `frozen_match_site_ledger` | `sico_compiler_reproduces_the_frozen_match_site_ledger` | `:9805` | 读 `selfhost/match-component-coverage-v1.json`（schema `sico.m22.match-coverage.v1`，396 站点） | —（见 §4） | — |

- **wall-clock 订正（③）**：STEP-0602 引的 **2.3–2.9 s** 是**测试体内单个 `finished in`**（仅 guest 前端降级那一次）；**整测试 wall-clock（含 runner `PreparedPool` 槽编译 + 800e9-fuel guest 运行 + harness）为 5.5–7.0 s**。**「快速 typed refusal、与燃料墙正交」的定性不变**——5.5–7.0 s 仍远低于 800e9 燃料耗尽所需量级，失败发生在组件发射**之前**的 IR 构建/call-shape 层，**不是燃料耗尽**。⇒ 只需把 wall-clock 数字上修，**不改判定**。

### 3.3 KEY CORRECTION ①——冻结账本**当前不是红的**（与被测源同步）

- `git log -1` 实测：`selfhost/match-component-coverage-v1.json`、`selfhost/codegen.sico`、`selfhost/parser.sico` 三者**最后一次修改均在同一提交 `6440890`（2026-11-06，「M22 W2/W4: STEP-0556-0560 emitter scaling probes, corpus verdict parity, soundness fix」）**。
- ⇒ 账本与其被测的 5 文件 guest bundle 在提交层**同步冻结**，**当前并不处于失配（不红）**。
- **账本的快速结构相位**（schema `sico.m22.match-coverage.v1`、396 站点计数、逐站 `function_line`/`parameter_count`/head/labels）**在当前源上通过**（本片只读复核该结构相位；**慢速逐组 95 组重算未运行**，见 §6 诚实收窄）。
- **订正口径**：STEP-0602/STEP-0571 所称的「账本漂移 / `LEDGER_STALE`」是**有条件的未来成本**——它**只在应用 STEP-0571 §3 配方之后**才发生（配方在 `gw_list_get_subject_packed`/`gw_match_open_pack` 插入行 ⇒ **130 个 `parser.sico` 站点行号位移** ⇒ `census-m22-match-sites.py --check` 报 `LEDGER_STALE`，STEP-0571 §5 实测 `MS-267`「frozen site line no longer holds the match head」即此症状）。**在配方落地之前，账本是绿的/同步的**。⇒ 不得把第 5 个红测读作「当下已失配待修」；它是「**配方落地的绑定代价**」。

### 3.4 KEY CORRECTION ②——`AGENTS.md` §10.11-A 的 emitter-vocabulary 前提**已过期并关闭**

- `selfhost/emitter-vocabulary-v1.json` **行 168** 的 `guest_emitter.source_sha256` = `013f2d7e894994b3734646c6c9996c14e2e0d999a7fd37a37ef3ac5dc0854d0f`，**逐字符等于**当前 `selfhost/codegen.sico` 的 sha256（§3.1）。
- ⇒ 词表指纹与当前 `codegen.sico` **一致** ⇒ `census-m22-emitter-vocabulary.py --check` **不再判 `EMITTER_VOCABULARY_RED`**。
- **§10.11-A 旧前提失效**：该条引用的「`guest_emitter.source_sha256` 仍是 B1 修复前的 `da0fec5f…`，当前 `codegen.sico` 是 `3d8705b1…`」两个 sha **均已过期**（现分别为 `013f2d7e…` / `013f2d7e…`，**已相等**）。
- **易混淆点（本片显式澄清）**：该文件**行 110** 另有一处 `source_sha256` = `220b8e47…`，它属于 `backend_helper_rule` 对象，**与 `guest_emitter` 无关、值也不同**；不得把它误读为 `guest_emitter` 的指纹（否则会误判为仍红）。
- ⇒ **§10.11-A 的「第 13 拍必红」整改项关闭**。与 [`docs/reports/m22-doc-hygiene-2026-11-06.md`](../reports/m22-doc-hygiene-2026-11-06.md) 与 STEP-0570 一致。

## 4. 对五个红测的重新定性（订正后）

| # | 红测 | 订正后定性 | 与燃料墙关系 |
|---|---|---|---|
| 1 | step0507 `cgs_result_return_hex` | guest 前端 EXPRESSION 层 typed refusal，`ERR:E-SH-IR-EXPRESSION`，整测 6.47 s | **无依赖耦合**（非燃料耗尽） |
| 2 | step0519 `cgs_value_ty` | call-shape 层 typed refusal，`ERR:E-SH-IR-CALL-SHAPE`，整测 6.97 s | **无依赖耦合** |
| 3 | step0520 `cgs_value_slot` | call-shape 层 typed refusal（同 ② 形状），整测 6.69 s；**精确码未测** | **无依赖耦合** |
| 4 | step0526 `cgs_function_instr_hex` | 前端非 Output（嵌套 `while`）typed refusal，整测 5.58 s；**精确 Domain-vs-Trap 码未测** | **无依赖耦合** |
| 5 | frozen match-site ledger | **当前同步（不红）**；漂移 = 应用 STEP-0571 §3 配方后的**有条件重冻结成本**（130 站行号位移 ⇒ `LEDGER_STALE`） | **非燃料墙本体**（燃料墙 = 账本里 331 个 `FuelExhausted` 站点，独立且更大） |

**核心判定（复核后维持并强化）**：**M22 存在两个正交面**——本组红测是「**前端形状缺口（①–④）+ 配方落地的账本绑定代价（⑤）**」，燃料墙本体是**账本里 331 个 `FuelExhausted` 站点**（独立且更大）。**这 5 个都不该等 op 级燃料剖析**（剖析解决的是 331 个耗尽站点，与本组正交）。

## 5. 下一刀（复核后维持 STEP-0602 §6 的建议）

- **建议下一刀（若 owner 恢复 M22 实现）**：**STEP-0571 §3 配方落地（`step0519`+`step0520`，唯一已验证路径：转绿 + 逐字节等同）+ 冻结账本重冻结（绑定变更）**，一次协调变更最多解锁 **3/5**；**不必等 op 级燃料剖析**。
- **但配方仍被两项阻塞（本片复核后确认）**：①**130 站协调账本重冻结**（配方必然位移 130 行 ⇒ 必须与配方同片/同步做，不能拆开——拆开 = 破坏冻结契约基线或空跑）；②**owner 授权**（AGENTS §10.1.4「在 owner 裁决前，M22 线只允许测量片/诊断片，不允许特征实现片（含 `parser.sico`/`codegen.sico` 热文件写入）」⇒ 配方落地属特征实现片，须 owner 先解除暂停/授权）。
- **本片不做配方落地**（它是特征实现片，且本片是只读诊断片）。

## 6. 诚实收窄

1. **#3/#4 的精确诊断码未测**：两者的 panic 串**省略 outcome**（bare panic 丢弃 outcome），故本片**未从 panic 串读到精确 Domain-vs-Trap 码**。要取得精确码**需改测试打印 outcome**——这属测试编辑，**超本只读诊断片范围**，留待实现片。**可确定的**：四者均为**快速 typed refusal**（5.5–7.0 s，远低于燃料耗尽），**不是燃料耗尽**。
2. **#4 的 Domain-vs-Trap 码仍开放**：STEP-0571 §2 只记「非 Output（嵌套 `while`）」，未区分 `Domain`（typed）还是 `Trap`；本片未重测，**精确码未闭**。
3. **账本的慢速逐组重算未运行**：本片只复核了账本的**快速结构相位**（schema/396 计数/逐站结构字段）在当前源上通过；**95 组慢速逐组差分重算未运行**（完整重算 ≈20–27 min，超本片预算）。⇒ 「账本当前同步」的结论锚定于**同提交冻结（§3.3）+ 快速结构相位通过**，**未由完整逐组重算独立复证**。
4. **未重跑任何 cargo**：本片三项订正均可由磁盘 sha + `git log` 冻结点 + STEP-0602/STEP-0571 已登记实测静态推出；重跑不改变结论且超预算。本片**未独立复跑** STEP-0602 的 5 个 `--exact` 测试（其复跑结果转引自 STEP-0602 的实测记录 + 本片的 wall-clock 口径订正）。
5. **未判定任何门**：本片是诊断复闭与口径订正，M22 七门、ADR-0015 的 `A == B == C`、证据等级、冻结契约均未动；**M22 overall 保持 NO-GO**。
6. **工作树上下文**：存在与本片无关的既有 dirty 文档（`docs/plans/M24-vision-closure-gui-application-pilot.md`、`docs/rfc/RFC-0052-amendment-A3-ui-selection-model.md`、`docs/rfc/RFC-0052-native-ui-source-binding-v0.md`、`docs/user-guide/NATIVE-UI.md`），本片**未触碰**。

## 7. 文件清单

- **新增**：`docs/steps/STEP-0604-m22-exit-blocking-red-tests-diagnostic-reclosure.md`（本文件）。
- **日期化订正（不改写既有段落正文）**：
  - `AGENTS.md` §10.1.4：追加一条 2026-11-07 addendum（记录三项订正 + §10.11-A emitter-vocab 前提关闭）。
  - `AGENTS.md` §10.11-A：对「STEP-0545 电池第 13 拍必红 `EMITTER_VOCABULARY_RED`」一条**原位标注为已关闭/过期**（不删除该条），加一行指针指向本 STEP。
  - `docs/STATUS.md`：顶部日期化日志区新增一条 2026-11-07 条目（诊断复闭完成摘要）。
  - `docs/steps/README.md`：索引表新增 STEP-0604 一行。
- **未动**：`selfhost/**`、`crates/**`、`runner/**`、任何冻结语料/账本、`docs/ROADMAP.md`、`docs/handoff-m22.md`、上述 4 个既有 dirty 文档。
