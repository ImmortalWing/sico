# STEP-0605 · M22-W2 · STEP-0571 §3 嵌套泛型 `list.get[U64]` 修复配方落地：两个出口阻塞红测转绿（逐字节等同）+ 账本重编号 + MS-081 既有燃料墙阻塞登记

| 项 | 值 |
|---|---|
| 片型 | **实现落地（配方生效）+ 落地状态登记**：把 [STEP-0571](./STEP-0571-m22-w2-nested-generic-list-get-fix.md) §3 的 4 处纯插入式修复落地到 `selfhost/parser.sico`，**实测把 STEP-0602/0604 钉定的 5 个出口阻塞红测中的 2 个（step0519/step0520）转绿且 guest/Rust 逐字节等同**；并随配方同步重编号冻结账本。本片**只登记落地结果**（配方编辑 + 账本重编号已在工作树中，见 §6 溯源），不改语言面、不改账本 schema、不翻任何门 |
| 状态 | **配方落地 GO（本片内）**。`selfhost/parser.sico` 工作树含 §3.1–§3.4 全部 4 处编辑，sha256 `b67ec566…`（23168 行，相对基线 `b6cb6ab0…` 为 +52/−3）；`selfhost/match-component-coverage-v1.json` 随配方重编号（+257/−257）；`selfhost/codegen.sico` 未触碰（clean，sha256 `013f2d7e…` = STEP-0604 §3.1 记录的 committed-clean 值） |
| 门 | **不翻任何门**。M22 overall **NO-GO** 不变；本片未改 gate 文本/证据等级/契约/语料 |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 构建，runner 独立 workspace，`--exact` 复跑，toolchain 1.98.0-x86_64-pc-windows-gnu |

## 0. 结论摘要（一句话）

**STEP-0571 §3 配方已落地并生效**——嵌套泛型 `sico.list.get[U64]` 前端 token/类型泛化的 4 处纯插入式编辑已在 `selfhost/parser.sico` 工作树中，**两个目标红测 `step0519`/`step0520` 由 `ERR:E-SH-IR-CALL-SHAPE` 转为逐字节等同绿**；两个独立红测（`step0507` EXPRESSION 层、`step0526` 非 Output 嵌套 `while`）**不变**，证明配方未越界；冻结账本随配方**重编号**（两个插入锚点之下 parser.sico 站点行号位移）。**账本 firmware 测试仍红于 MS-081，但这是既有燃料墙乐观口径、非本配方回归**（见 §4），且其修正被**本机无可用 Python**阻塞 ⇒ 顺延燃料墙/D8 轨。**未翻任何门，M22 overall 仍 NO-GO，未提交未推送。**

## 1. 背景：配方为何落地

[STEP-0571](./STEP-0571-m22-w2-nested-generic-list-get-fix.md) 已验证 §3 的 4 处修复配方能把 `step0519`/`step0520` 转绿，但当时因**冻结账本爆炸半径**（配方在两个函数插入行 ⇒ 130 个 parser.sico 站点行号位移 ⇒ `LEDGER_STALE`）按纪律**回退保基线**，只交付配方正文。

[STEP-0602](./STEP-0602-m22-selfhost-reds-exit-blocker-analysis.md) 与 [STEP-0604](./STEP-0604-m22-exit-blocking-red-tests-diagnostic-reclosure.md) 进一步钉定：M22 的 5 个 `selfhost_compiler` 出口阻塞红测中，`step0519`+`step0520` 是**唯一已验证修复路径**（STEP-0571 §3 配方，转绿 + 逐字节等同），且**与燃料墙正交**（快速 typed refusal）；账本漂移则是「**配方落地的绑定代价**」（须与配方同片/同步重冻结）。

⇒ 本片即执行该「**配方落地 + 账本同步重冻结**」协调变更，并取证其结果。

## 2. 配方落地内容（§3.1–§3.4，逐字节照搬既有形状）

全部在 `selfhost/parser.sico`，纯插入式 + 既有拒绝行原样保留（4 处）：

| # | 位置 | 内容 |
|---|---|---|
| §3.1 | `gw_list_get_subject_packed`（~:13958）入口 | 泛型 token 扫描：`[` / `U64`·`Text` / `]` 判别，置 `glg_elem_json`/`glg_elem_kind`/`glg_generic`，非法泛型仍 `return "ERR:E-SH-IR-CALL-TYPE"` |
| §3.2 | 同函数 list 参数 kind 门 | 由只认 `{"kind":"list","data":{"kind":"string"}}` 放宽为 string/u64 二者皆放行（否则 `CALL-TYPE` 原样保留） |
| §3.3 | 同函数结果类型 + op 名 | `glg_ty` 由硬编码 string 改为按 `glg_elem_json` 拼接；op 名由硬编码 `"sico.list.get"` 改为按 `glg_generic` 追加 `[`+kind+`]` ⇒ `sico.list.get[U64]` |
| §3.4 | `gw_match_open_pack`（~:15703）match 结果类型 | `gmo_ok_ty` 由硬编码 string 改为按主语 `sico.list.get[U64]` 的 `[` token 判 `U64`/`Text`，分别置 `"u64"`/`"string"` |

配方正文完整固化在 [STEP-0571 §3](./STEP-0571-m22-w2-nested-generic-list-get-fix.md)，此处不重复。全部 typed-refusal 臂原样保留 ⇒ 非法泛型实参仍 `CALL-TYPE`/`CALL-SHAPE`。

## 3. 落地取证（coordinator 亲跑 `--exact`）

### 3.1 两个目标红测转绿（逐字节等同）

| 测试 | 落地前 | 落地后 |
|---|---|---|
| `sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly`（`cgs_value_ty`） | FAILED，`ERR:E-SH-IR-CALL-SHAPE` | **ok，1 passed，4.21 s** |
| `sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly`（`cgs_value_slot`） | FAILED，同 ② 形状 | **ok，1 passed，3.52 s** |

两者均为 `*_byte_exactly` 测试 ⇒ **guest 组件与 Rust 生产输出逐字节等同**确认（非仅长度/形状）。

### 3.2 隔离守卫：两个独立红测不变（证明配方未越界）

| 测试 | 结果 | 根因归属 |
|---|---|---|
| `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`（`cgs_result_return_hex`） | **仍 FAILED**，`ERR:E-SH-IR-EXPRESSION` | EXPRESSION 层**独立根因**，与泛型 `list.get` 无关 |
| `sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly`（`cgs_function_instr_hex`） | **仍 FAILED** | 非 Output **嵌套 `while`** 独立根因，与泛型 `list.get` 无关 |

⇒ 配方只闭合 call-shape 层的两个站点，**未扩大改动面**、未掩盖两个独立根因。这两个仍是出口阻塞项，各需另片收口（STEP-0602 §6 已估 1–2 片/项，未验证）。

### 3.3 落地后单文件指纹

- `selfhost/parser.sico`：sha256 `b67ec56636d0211ce437b88dbfeeaceb9bad52ec45cff390922bf89d9eafd4aa`，23168 行（基线 `b6cb6ab0…`，+52/−3）。
- `selfhost/codegen.sico`：未触碰，clean。
- `selfhost/match-component-coverage-v1.json`：+257/−257（见 §4）。

## 4. 账本随配方重编号 + MS-081 既有燃料墙阻塞

### 4.1 账本重编号（配方的绑定代价，如 STEP-0604 §3.3 预告）

`selfhost/match-component-coverage-v1.json` +257/−257：配方在 `gw_list_get_subject_packed`（13958）与 `gw_match_open_pack`（15703）插入行 ⇒ 两个锚点之下的 parser.sico 站点**行号全部位移**（例：MS-267 14102→14139、MS-268 14113→14151）。这与 STEP-0571 §5 当初回退的根因是同一现象；本片按 STEP-0604 §5 的「同片/同步重冻结」要求**与配方一并交付**，不再回退。

### 4.2 账本 firmware 测试仍红于 MS-081 —— 既有问题，非本配方回归

`sico_compiler_reproduces_the_frozen_match_site_ledger`（`--exact`，3259 s ≈54 min）失败于 **MS-081**：账本冻结 verdict `"ir-equal+component-equal"` vs 实测 `"ir-equal+component-other:FuelExhausted"`。

**证明其为既有、与配方无关**（coordinator 取证）：

- **(a) HEAD 的 MS-081 具完全相同的冻结 verdict** `"ir-equal+component-equal"` 与相同的 line 4577 ⇒ 配方未改写 MS-081 的期望条目。
- **(b) MS-081..084 位于配方插入锚点之前**：它们是 `fixed_literal_const_segment`（checked_add）站点，行 4577/4590/4622/4629，全部 **< 13958/15703**；而配方只改泛型 `list.get`（§3.1–§3.4）⇒ **不可能影响 checked_add 站点的 component verdict**。

⇒ 这正是 AGENTS §10.1.3 已记录的**乐观口径**：账本标 36 个 `component-equal` 站点（含 MS-081..084），而实测这批实为 `FuelExhausted`（账本共 **331 个 FuelExhausted 站点**）——旧「36 component-equal」把燃料耗尽误记为组件全等。MS-081 是该乐观口径的一个具体暴露点。

### 4.3 MS-081 修正的阻塞项：本机无可用 Python

正确的 FuelExhausted 态 `component_sha256` 只能由账本重算工具 `tools/census-m22-match-sites.py` 产出。但本机**无可用 Python**：仅有 WindowsApps stub，`SICO_PYTHON` 未设，`Python312` 目录只有 `Lib/`、`Scripts/`、无 `python.exe` ⇒ **无法在本机重算 MS-081 的 component_sha256**。

⇒ **MS-081 修正顺延到燃料墙/D8 轨**（与 STEP-0579 §4 / STEP-0572 §5 的 op 级燃料剖析与限额裁决同轨），本片不擅自上调限额、不臆造 digest。这与 AGENTS §10.1.3「在 owner 裁决前，M22 线只允许测量片/诊断片」一致——MS-081 的 digest 修正属燃料墙面，不在本落地片范围。

## 5. 与 STEP-0602/0604 的叙事衔接（不重复）

- **STEP-0602** = 5 个出口阻塞红测的 `--exact` 精确化（只读诊断），结论「M22 存在两个正交面」。
- **STEP-0604** = 对 STEP-0602 的只读独立复核 + 三处口径订正（账本当前不红 / emitter-vocab 前提关闭 / wall-clock 上修），并明确「配方仍被 130 站账本重冻结 + owner 授权两项阻塞」。
- **STEP-0605（本片）** = 配方**落地** + 账本同步重编号 + 两个目标红测转绿取证 + MS-081 既有阻塞登记。它把 STEP-0604 §5 所称「配方落地」从「建议下一刀」推进为「已执行并取证」。

**两个正交面的口径维持不变**：本组（前端形状缺口 + 配方落地账本绑定代价）与燃料墙本体（账本里 331 个 FuelExhausted 站点）依旧分离；本片只解决前者的一半（step0519/step0520），**未触碰燃料墙本体**。

## 6. 诚实溯源（不过度声称）

`parser.sico` 的配方编辑与 `match-component-coverage-v1.json` 的重编号在落地 sub-agent 运行时**已在工作树中存在**（HEAD 未变，`af6dcb7`）；该 sub-agent **未施加任何编辑**。coordinator 独立核验：**diff 恰为 STEP-0571 §3 配方**，且两个目标测试 `--exact` **逐字节转绿**、两个隔离红测不变 ⇒ **以已验证状态采纳**。

**但精确施加时间/agent 不确定**（可能为本会话早前一次 sub-agent 运行）。此处如实记为「**已验证状态采纳 + 溯源不确定**」——只声称「工作树状态经独立复验与 §3 配方一致、目标测试转绿」，**不声称**具体的施加者、施加时刻或完整套件结果（本片未跑全量 `selfhost_compiler` 套件，见 §7）。

## 7. 诚实收窄

1. **未跑全量 `selfhost_compiler` 套件**：本片只 `--exact` 复跑 4 个目标测试（2 转绿 + 2 隔离红）与 1 个账本 firmware 测试。落地后套件的完整通过/失败计数**未测**（全量套件含 95 组慢速差分，超本片预算）。⇒ 除 §3.1/§3.2/§4.2 列名者外，其余测试状态未取证。
2. **MS-081 未修正**：账本 firmware 测试仍红；本片**未**改写 MS-081 的冻结 verdict/digest（属燃料墙/D8 轨），也未上调任何限额追认。
3. **账本慢速逐组重算未跑**：本片未对 396 站点逐组重算差分（STEP-0604 §6.3 同样未跑）；账本与被测源「同步」仅由**配方+重编号同片交付**这一事实锚定，未由完整逐组重算独立复证。
4. **`step0507`/`step0526` 未修**：两者的精确 Domain-vs-Trap 码仍未测（STEP-0604 §6.1/6.2 已登记），本片未触碰。
5. **未判定任何门**：M22 七门、ADR-0015 的 `A == B == C`、证据等级、冻结契约均未动；**M22 overall 保持 NO-GO**。
6. **未提交未推送**：HEAD 仍 `af6dcb7`，全部改动留在工作树。
7. **工作树上下文**：存在与本片无关的既有 dirty 文档（`docs/plans/M24-vision-closure-gui-application-pilot.md`、`docs/rfc/RFC-0052-amendment-A3-ui-selection-model.md`、`docs/rfc/RFC-0052-native-ui-source-binding-v0.md`、`docs/user-guide/NATIVE-UI.md`），本片**未触碰**。

## 8. 文件清单

- **配方落地（工作树已有，本片登记其状态）**：`selfhost/parser.sico`（§3.1–§3.4，sha256 `b67ec566…`）、`selfhost/match-component-coverage-v1.json`（+257/−257 重编号）。
- **未动**：`selfhost/codegen.sico`（clean）、任何 crate、任何测试、`docs/ROADMAP.md`、`docs/handoff-m22.md`、上述 4 个既有 dirty 文档。
- **新增文档**：`docs/steps/STEP-0605-m22-w2-step0571-recipe-landing.md`（本文件）。
- **日期化文档登记（不改写既有段落正文）**：
  - `docs/steps/STEP-0571-m22-w2-nested-generic-list-get-fix.md`：追加 2026-11-07 Addendum（配方已落地 + 两目标转绿 + MS-081 既有阻塞）。
  - `AGENTS.md` §10.1.4：在 STEP-0604 段后追加一条 2026-11-07 addendum（落地 + MS-081 既有阻塞）。
  - `docs/STATUS.md`：顶部日期化日志区新增一条 2026-11-07 条目（配方落地摘要）。
  - `docs/steps/README.md`：索引表在 STEP-0604 行后新增 STEP-0605 一行。