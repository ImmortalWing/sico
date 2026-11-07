# STEP-0602 · M22 · 5 个出口阻塞红测的确定性诊断（只读片，不改任何代码/语料/文档正文）

> **片型** = 纯诊断片（只读）。**不改任何源码、测试、语料、账本、文档正文**；**唯一产物** = 本文件。M22 整体仍 **NO-GO**，不翻任何门。
>
> **复现路径**（主线已复跑，本片再次逐项复跑确认）：
> `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1 --skip prepared_program_reruns_are_isolated_and_survive_a_trap`
> ⇒ **182 passed / 5 failed / 5 ignored（2778 s）**。5 个失败逐项复跑（`--exact`）确认，panic 行号与本文件一致。
>
> **工具链**：`$USERPROFILE\.cargo\bin\cargo.exe`、`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；Python 经 `tools/ensure-python.ps1` 引导（`target/tooling/python-3.12.10`）。
>
> **禁改清单（本片遵守）**：`selfhost/**`、`crates/**`、`runner/**`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`docs/steps/README.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、任何冻结语料/账本。仅新建本文件。

## 0. 结论摘要（一句话）

**5 个红测 = 4 个 guest 前端「按设计拒绝」（typed refusal 形状缺口，均毫秒级快速拒绝，与燃料墙无关）+ 1 个冻结账本漂移（verdict 失配，非燃料墙）。没有一个是燃料耗尽，也没有一个是「组件字节失配」；这 5 个都不该等 op 级燃料剖析。**

- 第 1–4 个（step0507/0519/0520/0526）：guest 前端（`selfhost/parser.sico`）在 **IR 构建 / call-shape / while-形状** 三个层面对特定形状返回 `ERR:E-SH-IR-*` 或非 Output 拒绝，而 Rust oracle 能降低同一形状 ⇒ **语义缺口（typed refusal）**。全部在 **2.3–2.9 s** 内拒绝，远低于 800e9 燃料上限 ⇒ **与燃料墙无关**。
- 第 5 个（match-site 账本）：`assert_eq!(site.verdict, outcome.verdict)`（`selfhost_compiler.rs:9927`）失配 ⇒ **冻结期望与当前实测不一致（账本漂移）**。**非燃料墙**（燃料墙是账本里 331 个 `FuelExhausted` 站点本身，不是这个测试的失败方式）。

## 1. 背景（这 5 个红测是什么）

`runner/sico-runner/tests/selfhost_compiler.rs` 是 M22 自举轨的差分测试：把 `selfhost/*.sico` 的函数逐个经 **guest 前端**（5 文件 bundle：`compiler.sico` + `compiler_lexer.sico` + `compiler_parser.sico` + `parser.sico` + `codegen.sico`）降低为 canonical IR，再与 **Rust oracle**（`lower_core` + `canonical_json`）逐字节比较。4 个 `*_codegen_functions_byte_exactly` 测试各钉一批 `codegen.sico` 函数（隔离降低：每个函数带其 callee 的 stub）；1 个 `*_match_site_ledger` 测试复现 396 站点的冻结 match-site 账本。

已实测事实（主线复跑 2778 s）：5 失败 = step0507/0519/0520/0526 + 冻结账本。

## 2. 方法

1. 读测试定位 5 个失败的断言与函数清单（`selfhost_compiler.rs:3469/6376/6701/3515/9805`）。
2. 逐项 `cargo test ... -- <test> --exact --nocapture` 复跑，取得**确定的**首个失败函数与诊断（本机 Windows x64，debug）。
3. 读冻结账本 `selfhost/match-component-coverage-v1.json`（396 站点 / 95 去重函数组）核对 verdict 分布。
4. 核对「冻结账本冻结点（6440890）→ HEAD（7bcdc35）」之间 selfhost bundle、oracle 生产代码、测量参数是否变化（`git diff`），以判定账本漂移根因。
5. 生成 66 个隔离函数源（复刻 `codegen_function_source` 到 `target/tmp/step0602/funcs/`，只读侦察），并按需小样本验证。

> 说明：对 4 个 codegen 测试，测试代码在**首个**失败处 `panic!`，故**确定失败集 = 首个失败函数**；列表内其后函数未被执行（候选未验证）。见 §5 诚实收窄。

## 3. 逐失败清单（函数名 / 失败层 / 根因类别 / 证据）

### 3.1 `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`（`selfhost_compiler.rs:3469`）

- **断言**：`:3507` `panic!("codegen function {name} must compile through the guest frontend: {diag:?}")`。
- **确定失败函数**：`cgs_result_return_hex`。
- **失败层**：guest 前端 **EXPRESSION 层**。
- **诊断（实测）**：`Domain { code: "invalid-input", message: "ERR:E-SH-IR-EXPRESSION" }`。
- **根因类别**：**typed refusal**（guest 前端按设计拒绝一个 Rust oracle 能降低的形状 = 前端语义缺口）。
- **证据**：本片复跑 `test result: FAILED ... finished in 2.75s`，panic 输出精确为 `ERR:E-SH-IR-EXPRESSION`。与 STEP-0571 §2 基线记录一致（`cgs_result_return_hex` = `ERR:E-SH-IR-EXPRESSION`，EXPRESSION 层，**独立根因**）。
- **不属于**：非燃料耗尽（2.75 s，远低于 800e9）；非组件字节失配（失败发生在组件发射**之前**的 IR 构建）。

### 3.2 `sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly`（`selfhost_compiler.rs:6376`）

- **断言**：`:6448` `panic!("codegen function {name} must compile through the guest frontend: {outcome:?}")`。
- **确定失败函数**：`cgs_value_ty`。
- **失败层**：guest 前端 **call-shape 校验层**。
- **诊断（实测）**：`Domain { code: "invalid-input", message: "ERR:E-SH-IR-CALL-SHAPE" }`。
- **根因类别**：**typed refusal**（嵌套泛型 `sico.list.get[U64]` 形状；STEP-0571 §3 已定位根因为 `gw_list_get_subject_packed` 不消费 `[U64]` 泛型 token）。
- **证据**：本片复跑 `finished in 2.89s`，panic 输出 `ERR:E-SH-IR-CALL-SHAPE`。与 STEP-0571 §2 基线一致。
- **修复候选**：STEP-0571 §3 的 4 处 `parser.sico` 配方（已验证把本测试转绿且逐字节等同，见 §4.2）。

### 3.3 `sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly`（`selfhost_compiler.rs:6701`）

- **断言**：`:6729` `panic!("codegen function {name} must compile through the guest frontend")`（**本 panic 串不含 outcome**）。
- **确定失败函数**：`cgs_value_slot`。
- **失败层**：guest 前端 **call-shape 校验层**（与 `cgs_value_ty` 同形状，STEP-0571 §2）。
- **根因类别**：**typed refusal**（同 `cgs_value_ty` 的 `sico.list.get[U64]` 泛型形状；STEP-0571 §2 基线直接记录 `cgs_value_slot` = 同 `cgs_value_ty` 形状 = 本片根因）。
- **证据**：本片复跑 `finished in 2.53s`，panic 首个失败函数 = `cgs_value_slot`。**诚实边界**：当前 panic 串省略 outcome，精确诊断取自 STEP-0571 §2 基线记录（`CALL-SHAPE`），非本片重测；本片已用独立函数源 + 冻结账本交叉核对形状族（见 §5）。
- **修复候选**：同 STEP-0571 §3 配方（同 `cgs_value_ty` 根因，同一配方修复两个测试）。

### 3.4 `sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly`（`selfhost_compiler.rs:3515`）

- **断言**：`:3546` `panic!("codegen function {name} must compile through the guest frontend")`（**本 panic 串不含 outcome**）。
- **确定失败函数**：`cgs_function_instr_hex`。
- **失败层**：guest 前端，**非 Output 结果（嵌套 `while` 形状）**（STEP-0571 §2）。
- **根因类别**：**typed refusal**（前端拒绝嵌套 `while` 形状；STEP-0571 §2 记为「非 Output 结果（嵌套 `while`，独立根因）」）。
- **证据**：本片复跑 `finished in 2.34s`，首个失败函数 = `cgs_function_instr_hex`。**诚实边界**：当前 panic 串省略 outcome，STEP-0571 §2 只记为「非 Output（嵌套 while）」，未给精确诊断码；本片不能从 panic 串读到精确码（`else` 分支丢弃 outcome）。**非燃料耗尽**（2.34 s）可确定；是否为 `Domain`（typed）还是 `Trap` 本片未重测（见 §5）。
- **修复候选**：独立前端修复（非 Output/嵌套 while 形状），未验证；或按 STEP-0571 §2「独立根因」另片收口。

### 3.5 `sico_compiler_reproduces_the_frozen_match_site_ledger`（`selfhost_compiler.rs:9805`）

- **断言**：`:9927` `assert_eq!(site.verdict, outcome.verdict, "{}: the verdict of {path}:{function} moved", site.id)`。
- **失败对象**：396 站点冻结账本（`match-component-coverage-v1.json`）中的**某个站点**，其冻结 `verdict` 与本次实测 `verdict` 失配。
- **失败层**：账本复现比较（IR + Component 双轴后的 verdict 比较），非编译失败。
- **根因类别**：**账本漂移**（冻结期望与当前实测不一致）。
- **证据**：
  - 冻结账本 verdict 分布（本片从 `match-component-coverage-v1.json` 实测）：396 站点 = 331 `FuelExhausted` / 36 `component-equal` / 24 `Trap`（8 MiB stdout 上限）/ 5 `component-refused-guest:unsupported codegen source shape`；去重 `(path,function)` 95 组 = 62 `FuelExhausted` / 30 `component-equal` / 2 `component-refused-guest` / 1 `Trap`；`parser.sico` 387 站、`codegen.sico` 2 站、`compiler.sico` 3 站、`compiler_lexer.sico` 1 站、`compiler_parser.sico` 1 站、`compiler_semantics.sico` 1 站、`formatter.sico` 1 站。
  - **部分重测**（本片两次后台复跑，各在 600 s 前台超时中断前实测）：两次各测 **35 / 36 组**（同一快速子集），**均 0 失配** ⇒ 漂移落在**未测的 ~59 组**（慢速组：大闭包 `FuelExhausted`（parser.sico）+ 全部 `Trap`(8 MiB) 组；快组 = component-equal 24 + 小 FuelExhausted + refused-guest，全部与冻结一致）。**精确漂移站点本片未取得**（需要完整 95 组重测 ≈ 20–27 min，超出本片 10 min 长跑预算；见 §5）。
  - **冻结点核对**（`git diff 6440890..HEAD`）：selfhost 5 文件 bundle（`compiler/compiler_lexer/compiler_parser/parser/codegen.sico`）**无任何提交差异**；oracle 生产代码（`sico-hir/parser/semantics/lexer/format`）**无生产差异**（工作树改动全部落在 `mod tests` 断言计数更新，见 §5）；测量参数（`MATCH_CLOSURE_*`、`run_guest` fuel 800e9 / timeout 120 s / mem 2560 MiB / hostcall 8<<30）**无提交差异** ⇒ 冻结点与当前被测对象在提交层**一致**。
  - ⇒ **判定**：漂移不是「selfhost/oracle/参数变了」的确定性语义漂移；最可能是**冻结时的测量环境与当前不同（燃料边界非确定性 / 环境 flake）**，或**账本冻结早于 6440890 且未在同一环境重冻结**。**精确根因未闭**（见 §5 诚实收窄）。
- **不属于**：非燃料耗尽（该测试的失败是 `assert_eq!` 失配，不是 `FuelExhausted`）；非组件字节失配（那是 `component-mismatch` verdict，属账本记录的一类，非本测试的失败方式）。

## 4. 修复候选与耦合度量化

### 4.1 逐项修复候选

| # | 红测 | 失败函数 | 根因类别 | 修复候选 | 燃料墙耦合 | 被「收窄语料/闭包口径」顺带解决？ |
|---|---|---|---|---|---|---|
| 1 | step0507 | `cgs_result_return_hex` | typed refusal（EXPRESSION） | 前端扩展以降低该形状，或重塑 `cgs_result_return_hex` 避开「host call 直入 return 表达式」；未验证 | **无**（2.75 s 拒绝） | **否**（形状缺口，非燃料/闭包问题） |
| 2 | step0519 | `cgs_value_ty` | typed refusal（CALL-SHAPE） | **STEP-0571 §3 配方（已验证转绿）** | **无**（2.89 s） | **否** |
| 3 | step0520 | `cgs_value_slot` | typed refusal（CALL-SHAPE） | **STEP-0571 §3 配方（同根因，已验证转绿）** | **无**（2.53 s） | **否** |
| 4 | step0526 | `cgs_function_instr_hex` | typed refusal（非 Output，嵌套 while） | 前端支持嵌套 `while` 形状，或重塑该函数；未验证 | **无**（2.34 s） | **否** |
| 5 | 冻结账本 | （未定位站点） | 账本漂移（verdict 失配） | 重冻结账本（`census-m22-match-sites.py` 重建 + 受影响站点重测）；**若为 flake 则稳定测量** | **时间耦合**（重测 FuelExhausted 组 ≈17 s/组，非依赖耦合） | 视根因（若为燃料边界 flake，收窄闭包会改变 FuelExhausted verdict ⇒ 账本仍须重冻结） |

### 4.2 关键耦合：STEP-0571 配方 ↔ 账本重冻结

STEP-0571 §3 配方在 `gw_list_get_subject_packed`（:13958）与 `gw_match_open_pack`（:15703）插入行 ⇒ **130 个 `parser.sico` 站点行号位移**，`census-m22-match-sites.py --check` 报 `LEDGER_STALE`；STEP-0571 §5 实测配方落地后 `MS-267`（冻结站点行不再含 match head）。⇒ **step0519/0520 的修复与账本重冻结是绑定变更**：不能只落配方不重冻结（会破坏冻结契约基线），也不能只重冻结不落配方（账本无意义位移）。STEP-0571 因该爆炸半径已回退保基线，配方正文完整固化在 STEP-0571 §3（可原样重放），工作副本 `target/tmp/parser.sico.step0571.fixed`。

### 4.3 燃料墙耦合结论（小样本验证依据）

- 4 个 codegen 测试的失败全部是**前端快速拒绝**（2.34–2.89 s，远低于 800e9）⇒ **与燃料墙无耦合**；`tools/probe-m22-fuel-sensitivity.ps1` / `m22_fuel_sensitivity` 的 2×/4× 结论（STEP-0600：2× 只让 5/331 站点转绿）**不适用**于这 4 个测试（它们根本不走到燃料边界）。
- 账本测试与燃料墙**时间耦合**（重测 FuelExhausted 组慢），但**非依赖耦合**：账本重冻结只需记录「当前 verdict」，不需要燃料墙被解决；且 STEP-0600 §2.1 实测 2× 只转绿 5/331 ⇒ **即便把燃料上限上调，账本也须重冻结**（因为 5 个站点的 verdict 会从 FuelExhausted 变 component-equal），燃料墙与账本重冻结是**必然同步关系**，不是「先修燃料墙再修账本」的先后依赖。

### 4.4 修复成本量级（片数估计）

| 红测 | 修复片数估计 | 说明 |
|---|---|---|
| step0519 + step0520 | 1–2 片 | STEP-0571 配方已验证（1 片落地）+ 验证；不含账本 |
| 账本重冻结 | 1–2 片 | `census-m22-match-sites.py` 重建 + 受影响站点重测 + 双轴 firmware 校验（STEP-0571 §7 记 >8 min 单测） |
| step0507 | 1–2 片 | 前端 EXPRESSION 形状扩展（未验证，需先红后绿） |
| step0526 | 1–2 片 | 前端嵌套 while 形状扩展（未验证，需先红后绿） |

## 5. 诚实收窄

1. **确定失败集 = 每测试首个失败函数**（`panic!` 在首个失败处终止；列表内其后函数未执行）。**未枚举**「首个之后是否还有失败函数」。本片生成 66 个隔离函数源（`target/tmp/step0602/funcs/`）但**未做逐函数枚举**：CLI `sico run` 复现被 CLI 默认燃料上限拦截（对 `cgs_result_return_hex` 得 `FuelExhausted` 而非 `ERR:E-SH-IR-EXPRESSION`，实测），而测试内的 `run_guest`（fuel 800e9）为私有 helper，无法从外部忠实调用 ⇒ **枚举需要测试内逻辑或专用探针**，超出本片只读边界。故「函数名清单」= 确定的**首个失败** + 测试内固化的**候选列表**（未验证），不作全枚举推测。
2. **step0520 / step0526 的精确诊断码未重测**：两者 panic 串省略 outcome；step0520 的 `CALL-SHAPE` 与 step0526 的「非 Output（嵌套 while）」取自 STEP-0571 §2 基线记录（同代码状态，`parser.sico` sha256 一致），非本片重测。step0526 是否 `Domain`（typed）还是 `Trap` 未区分。
3. **账本漂移的精确站点未定位**：完整 95 组重测约 20–27 min，超出本片「≤10 min 长跑」预算；两次部分重测（35 组 / 36 组，同一快速子集，均 0 失配）仅能定位「漂移在未测 ~59 组（慢速 FuelExhausted 大闭包 + Trap 组）」。**精确根因（燃料边界非确定性 vs 冻结环境差异）未闭**，需一次完整重测或对账本冻结环境与当前环境做对照；**漂移在慢速燃料敏感组 ⇒ 与燃料墙（331 FuelExhausted 站点）强相关，是账本重冻结与燃料墙必然同步的另一证据**。
4. **`census-m22-match-sites.py` 未运行**（其 `--check` 会与账本/源码比对，但运行它属于账本维护工具，本片未授权改动，且其全量重测超出预算）。
5. **未判定任何门**：本片是诊断，M22 七门、`A == B == C`、证据等级、冻结契约均未动；M22 overall 保持 **NO-GO**。
6. **工作树上下文**：`crates/sico-{hir,parser,semantics,lexer,format}` 有**已暂存**改动，但 `git diff --cached` 显示全部落在 `mod tests` 断言计数更新（58→66、2→4、4→7 等，因 235 条语料重冻结），**无生产代码差异** ⇒ oracle 生产行为 = HEAD，不构成账本漂移根因。
7. **`git status --short` 中与 M24 相关的未提交文件**（`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/tests/*`、`runner/sico-runner/tests/ui_*` 等）与本片无关，本片未触碰。

## 6. 下一刀建议（按解锁价值 / 成本 / 燃料墙耦合排序）

### 建议：下一刀 = **STEP-0571 配方落地（step0519 + step0520）+ 账本重冻结**（绑定变更），**不等 op 级燃料剖析**。

**理由（全部有实测支撑）：**

1. **最高解锁价值/成本比**：STEP-0571 §4 已验证配方把 step0519、step0520 两个红测转绿且 guest/Rust 逐字节等同（唯一被验证过的修复路径）；配方的验证矩阵在 STEP-0571 §4 完整记录。成本 = 1–2 片落地 + 1–2 片账本重冻结，**解锁 2 个 codegen 红测**（5 个中的 2 个）。
2. **账本重冻结是绑定变更**：配方落地必然位移 130 行 ⇒ 账本 `LEDGER_STALE`（STEP-0571 §5 实测 MS-267），故**必须与配方同片/同步**做，不能拆开（拆开 = 破坏冻结契约基线或空跑）。账本重冻结同时**修复第 5 个红测**（若其漂移非 flake，重冻结即重钉期望）。⇒ **一次协调变更最多解锁 3/5 红测**。
3. **与燃料墙无依赖耦合**：4 个 codegen 红测全部是前端快速拒绝（2.3–2.9 s），**不走到燃料边界**；账本重冻结只需记录当前 verdict（FuelExhausted 照记），不需要燃料墙被解决。STEP-0600 §2.1 已实测「2× 燃料只转绿 5/331」，**燃料墙是独立且更大的问题**，不应阻塞这 5 个红测。
4. **op 级燃料剖析不应是这 5 个的前置**：那 5 个是**前端形状缺口 + 账本漂移**，不是燃料耗尽；op 级剖析（STEP-0572 §5，需 owner 授权）解决的是 **331 个 `FuelExhausted` match 站点**（燃料墙本体），与本组红测**正交**。先修这 5 个（前端/账本），再单独排燃料墙轨。

**次优顺序（若 owner 先做小）：** ① 账本重冻结单独做（1–2 片，解锁账本红测，若为 flake 则先稳定测量）；② step0507（`cgs_result_return_hex` EXPRESSION，1–2 片，未验证需先红后绿）；③ step0526（`cgs_function_instr_hex` 嵌套 while，1–2 片，未验证）。

**不建议**：把 5 个红测都等 op 级燃料剖析 —— 实测表明它们不是燃料问题（见 §4.3/§6 理由 3–4）；op 级剖析只对 331 个 FuelExhausted 站点必要。

**仍属 owner 待裁（本片不代裁）**：STEP-0571 配方落地是否需要「账本重冻结 + 双轴 firmware 校验」的完整时间预算（STEP-0571 §7 记 >8 min）；若账本漂移确为燃料边界 flake，是否「稳定测量」优先于「重冻结」；step0526 的精确诊断（Domain vs Trap）是否值得一次专用探针。

## 7. 文件清单

- **新增**：`docs/steps/STEP-0602-m22-selfhost-reds-exit-blocker-analysis.md`（本文件）。
- **只读侦察产物**（`target/tmp/`，非仓库文件）：`target/tmp/step0602/failures.log`（5 测试复跑输出）、`target/tmp/step0602/funcs/*.sico`（66 个隔离函数源）、`target/tmp/step0602/gen.py`（复刻 `codegen_function_source` 的只读脚本）。
- **未动**：`selfhost/**`、`crates/**`、`runner/**`、`docs/{STATUS,ROADMAP}.md`、`docs/handoff-m22.md`、`docs/steps/README.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、`selfhost/match-component-coverage-v1.json`、所有冻结语料。
