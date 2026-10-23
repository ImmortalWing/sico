# STEP-0527 m22-w2 S5 **T1 族收口片 X**（`sico.text.concat`）：钉住 + 纠偏 + 独立 CI 回填 + 诚实收窄

> - status: **implementation complete locally**（本机电池 **BATTERY_GREEN 18/18**；固件 **153 / 153**；独立 CI 回填见 §3）
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0521](./STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表 + D15 登记）、[STEP-0523](./STEP-0523-m22-w2-s5-w2-t1-gd-single-return-route.md)（G-d1）、[STEP-0524](./STEP-0524-m22-w2-s5-w2-t1-gf-multi-argument-call-operand.md)（G-f）、[STEP-0525](./STEP-0525-m22-w2-s5-w2-t1-gf2-nested-call-argument.md)（G-f2）、[STEP-0526](./STEP-0526-m22-w2-s5-gd2-emitter-branch-support.md)（G-d2 / G-d3）
> - gate: **T1 族**按计划 §3.3.5 的**族级出口判据**闭合；**M22 仍为 NO-GO**（门 2 / 3 / 6 未过）
> - numbering: **STEP-0527**（取号前已核对 `docs/steps/`：最大号 0526；*0527 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15，已定稿）

## 0. 这一片的性质

计划 §3.3.5 给每个族定义了**收口片（X）**：**钉住 + 纠偏 + 前沿探针期望值随之前移 + 固件电池 + 独立 CI（提交后）+ 诚实收窄段**。

**本片不做新实现** —— 它是**验证片**：把族级真值表、固件钉、前沿探针、电池与独立 CI 一次性对齐，并给出**诚实的收窄结论**。

## 1. 族级出口判据（六项，逐项核对）

判据原文（计划 §3.3.5）：**`canonical IR` 逐字节等同 且 `整组件` 逐字节等同 且 `wasmparser::Validator` 通过 且 前沿探针期望值已翻转 且 固件电池绿 且 独立 CI 绿。五项缺一不算闭合。**

> **计数订正（STEP-0527）**：上面这句原文列的是**六条**判据，却写成「五项」—— 这是**计数笔误**，不是判据内容有出入。**本片把计划 §3.3.5 的「五项」订正为「六项」**（判据**内容一个字未改**），下文按**六项**核对。

| 判据 | 状态 | 证据 |
|---|---|---|
| `canonical IR` 逐字节等同 | ✅ **35** 个形状逐字节等同（真值表钉住） | `sico_codegen_guest_pins_the_t1_family_truth_table`（**36 行** = **35** 行逐字节等同 + **1** 行两侧**一致**的 typed 拒绝 BE）+ `sico_codegen_guest_derives_the_module_wide_data_offsets`（2 行） |
| `整组件` 逐字节等同 | ✅ 同上（**组件层 typed 拒绝 0 行**） | STEP-0526 起 AA 行翻转；S 内两个真实形状（1266 B / 1316 B）逐字节相同 |
| `wasmparser::Validator` 通过 | ✅ oracle 与 guest 两侧都过 | 真值表逐行验证 |
| 前沿探针期望值已翻转 | ✅ **41** 条用例**全部与期望一致**（**30** 条期望 `lowered` —— 含 T1 族 18 条形状；**11** 条期望 typed 拒绝 `ERR:E-SH-IR-*` —— **故意钉住的拒绝身份**，不是未翻转） | `tools/probe-m22-frontend-frontier.ps1`（电池步骤 `probe-frontend-frontier`） |
| 固件电池绿 | ✅ **153 passed / 0 failed** + 本机 **BATTERY_GREEN 18/18** | `target/tmp/step0527/battery.ps1`（STEP-0526 的 17 步原样复用 + 本片新增 1 步**文档自洽检查**） |
| 独立 CI 绿 | ✅ 见 §3 | GitHub Actions（`codex/m22-w1-ci`） |

⇒ **六项全过 ⇒ T1 族的支持面闭合。**

## 2. D10 复核（不得降级）

D10：**族降级后若 S 内仍有未覆盖调用点形状 ⇒ 不得降级。**

S 内 T1 族的 **10** 个调用点（`cg_post_body_hex` ×2、`cgs_local_get_hex` / `cgs_local_set_hex` / `cgs_i32_const_hex` / `cgs_i64_const_hex` 各 1、`cgs_zero_hex` ×2、`declaration_name` ×2）：

- **6 个非分支调用点** ⇒ STEP-0518 / STEP-0519 / STEP-0523（G-d1：单语句 `return` 的通用操作数）闭合；**其中** 2 个是多实参调用形状（G-f ⇒ STEP-0524 闭合）、2 个是嵌套调用实参形状（G-f2 ⇒ STEP-0525 闭合）—— **这两组是那 6 个里的子集，不另计**；
- **4 个 `if` 块里的调用点（G-d2）**（2 个在 `cgs_zero_hex`、2 个在 `declaration_name`）⇒ **STEP-0526 闭合**（两个真实形状逐字节相同：1266 B / 1316 B）。

⇒ **6 + 4 = 10 / 10 闭合** ⇒ **S 内不再有未覆盖的 T1 调用点形状** ⇒ **不需要降级** ⇒ 族可以按判据闭合。

## 3. 独立 CI 回填

| 批次 | 提交 | run | 裁决 |
|---|---|---|---|
| STEP-0524 | `a383dfe` | #31 | ✅ success（已回填） |
| STEP-0525 | `1956158` | #32 | ✅ **success**（run id `39969358043`，**本片回填**） |
| STEP-0526 | `b4364ef` | #33 | ✅ **success**（run id `39990810669`，**本片回填**，实测自 GitHub Actions API） |

⇒ 按仓库既有做法，**裁决由下一片的提交回填**，不为此再推一轮。

⇒ **本片（STEP-0527）推送后触发下一轮（run #34）**，其裁决同样由**再下一片**回填。

## 4. **诚实收窄**（本片最重要的一段）

### 4.1 T1 族闭合**不等于** M22 闭合

| 项 | 状态 |
|---|---|
| **M22 门 1 / 4 / 5 / 7** | ✅ GO（不变） |
| **M22 门 2** | PARTIAL（不变） |
| **M22 门 3 / 6** | ❌ NO-GO（不变） |
| **M22 整体** | ❌ **仍为 NO-GO** |
| **整文件自举** | ⏔ **仍未闭合**：`List[Text]` 形参 ⇒ `ERR:E-SH-IR-RECORD-TYPE`；宿主调用内建只认两个（`sico.text.concat` / `sico.bytes.length`）；嵌套 `if` 的**前端**缺口 `ERR:E-SH-IR-STATEMENT`；`match` 终结符未实现（发射端只发 `return` / `branch` / `jump` / `unreachable` 四种；guest 前端对该形状 typed 拒绝 `ERR:E-SH-IR-MATCH-SHAPE` ⇒ 当前不可达，见 STEP-0526）；`run` 多块仍 fail closed（结果区返回是单块形状） |
| **差分语料覆盖** | ⏔ **仍 0 条** —— T1 族的形状**一个都没进** `selfhost/corpus-v0.json` / `corpus-w1` ⇒ **族的行为只由族级真值表钉住，不由差分语料钉住**（这是 T1 族登记册上**仍然为 0 的那一项**） |
| **下一个族** | T2 族（`sico.text.length`，S 内 318 处） |

### 4.2 本片**没有**做的事

- **没有**新增任何实现（`selfhost/*` 与 `crates/*` 零改动）—— 本片是验证片；
- **没有**把「族闭合」写成「M22 闭合」—— 两者是不同层级的判断；
- **没有**动语料（语料是 M22 级的工作项，不是族级收口片的产物）；
- **没有**把「组件层 typed 拒绝 0 行」读成「guest 能编译整个 S」—— 后者仍被**族外**的缺口挡住。

### 4.3 一条方法学结论

**「族闭合」与「固定点达成」是两个判断。** 前者问的是「这个族声明的支持面，guest 是不是逐字节做到了」；后者问的是「guest 能不能编译整个冻结源集 S」。

STEP-0521…0526 走完的正是这条区分：**每个族都用自己的真值表判定，而 S 的固定点由**所有**族合起来判定** —— T1 闭合之后，S 的固定点仍差着**别的族**（宿主调用内建、`List[Text]` 形参、嵌套 `if` 前端形状、record 泛型、cell 的记录类型 …）。
