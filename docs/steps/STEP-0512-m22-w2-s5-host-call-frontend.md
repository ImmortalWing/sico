# STEP-0512 m22-w2 S5 宿主调用（**前端刀**）：直线型函数体里的 `sico.bytes.length` ⇒ canonical `intrinsic` IR **逐字节等同**

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0511](./STEP-0511-m22-w2-s5-host-call-truth.md)（真值测量片：组件 1025 B + 落点定界）、[STEP-0510](./STEP-0510-m22-w2-s5-call-in-return.md)（前沿测绘 + 纠偏）、[STEP-0509](./STEP-0509-m22-w2-s5-call-component.md)（codegen 刀）
> - gate: 门 3 的 S5 波次 —— **前端刀（IR 层）**；**组件层仍是下一刀**（STEP-0513）
> - numbering: M22 线顺延为 **STEP-0512**（已核对 `docs/steps/` **无 0512**；并行 M24 轨占用 STEP-0330–0337）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ STEP-0511 §1.2（整组件 **1025 B**，过真实 `wasmparser::Validator`） |
| 有**有界出口测试** | ✅ §3（3 条新钉 + 1 条 oracle 钉翻转） |
| 前端是否需要刀已判定 | ✅ **需要**，落点已定界（STEP-0511 §2） |

⇒ 取号合法。**实现已完成**，见 §4。

## 1. 本片改了什么

### 1.1 新增顶层路由 `direct_host_call_return_ir`

`selfhost/parser.sico` 新增一个**专用顶层路由**（与 `direct_bytes_equal_encode_ir`、`record_construct_function_ir`、`nested_intrinsic_return_ir` 同一族），插在 `direct_bytes_equal_encode_ir` 之后：

```
function direct_host_call_return_ir(words: List[Text], src: Bytes, name_idx: U64, body_from: U64, body_end: U64, fn_start: U64, function_ordinal: U64, return_kind: Text, parameter_count: U64) returns Text:
```

它认**一个**形状并产出**整份函数 JSON**：

| 闸门 | 条件 |
|---|---|
| 形参 | 恰好 **1** 个 |
| 声明返回类型 | `U64`（= 内建 `sico.bytes.length` 的结果 kind） |
| 函数体 | **只有一条语句**（签名行之后第一条语句就是 `return`，且 `return` 所在行之后紧跟 `end function`） |
| 路径 | `sico . bytes . length` |
| 实参 | **恰好一个**，且是该函数的 `Bytes` **形参** |

产出（复用 guest 已有的机器，**不新造机器**）：

- `parameters_ir(...)` ⇒ 形参表；
- `binding_parameter_index` / `binding_parameter_kind` ⇒ 实参的 value id 与声明类型；
- `return_expression_start` / `return_expression_end` ⇒ 指令的源码区间；
- `intrinsic_instruction_json(result, "u64", "sico.bytes.length", "[<value id>]", ...)` ⇒ **canonical `intrinsic` 算子**；
- `gw_find_code_text(src, "end function", ...)` ⇒ 函数区间。

### 1.2 接进 `scalar_ir` 的路由梯

`selfhost/parser.sico` 的 `scalar_ir`（**裸函数体**的默认路由）在 `direct_bytes_equal_encode_ir` 之后、`fixed_guard_chain_ir` 之前多了一步：

```
let direct_host_json = direct_host_call_return_ir(words, src, name_idx, body_from, body_end_word, fn_start, function_ordinal, return_type_kind, parameter_count)
```

⇒ 该形状**不再**掉进 `scalar_ir` 自己的 return 表达式链（那条链的 `I64.literal` / `U64.literal` 固定字面量档 ⇒ `ERR:E-SH-IR-EXPRESSION`，STEP-0511 §2.2 实测）。

### 1.3 为什么是「专用顶层路由」而不是改 `scalar_ir` 的表达式链

- 本仓库的前端**已经是**这个形状：`record_construct_function_ir` / `record_variant_function_ir` / `direct_bytes_equal_encode_ir` / `nested_intrinsic_return_ir` 都是「专用顶层路由 + 返回 `SKIP` 表示不认领」；
- 改动面**纯插入**（新增一个函数 + 一段接线），**不重排任何既有行** ⇒ 回归面最小；
- 路由**只认一个形状**，其余一律 `SKIP` ⇒ **既有拒绝身份一个都不变**（§5.2 实测）。

## 2. 支持面（**恰好一个形状**）

```
function size(data: Bytes) returns U64:
  return sico.bytes.length(data)
end function
```

| 维度 | 支持 | 不支持（保持 typed 拒绝） |
|---|---|---|
| 内建 | `sico.bytes.length` | `sico.text.length`、`sico.bytes.at`（三实参）等**全部其它内建** |
| 实参 | 一个 `Bytes` **形参** | 字面量、cell、宿主调用、用户调用 |
| 位置 | **return 位置**（函数体唯一语句） | `let` 绑定位置（语句位置） |
| 形参个数 | 1 | 0 或 ≥2 |

⇒ 这是**一刀一个形状**，不是「宿主调用整类」的通吃；整类的前沿推进留给后续片。

## 3. 出口测试（有界）

| # | 钉 | 口径 |
|---|---|---|
| 1 | `sico_compiler_lowers_the_host_call_in_a_straight_line_body_byte_exactly` | guest IR **逐字节** == Rust canonical IR（**2215 B == 2215 B**）+ 含 `"op":"intrinsic"` / `"name":"sico.bytes.length"` / `"arguments":[0]` |
| 2 | `sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused` | 三实参 `sico.bytes.at` 与 **let 绑定位置**仍为 `ERR:E-SH-IR-EXPRESSION`（**不静默放宽**） |
| 3 | `sico_compiler_lowers_the_step0512_parser_function_byte_exactly` | **自举形状前沿**：新 guest 函数的 21 个闭包函数，guest 自己逐字节降低 == Rust 前端 |
| 4 | `oracle_host_call_in_a_straight_line_body_truth_is_pinned`（**翻转**） | STEP-0511 承诺的翻转：guest 半边从 typed 拒绝翻成逐字节比较；组件仍 **1025 B** + 过真实校验器 |

## 4. 实施记录（2026-10-22）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/parser.sico` | 新增顶层路由 `direct_host_call_return_ir`（**+121 行**）+ `scalar_ir` 路由梯接线（**+24 行**）—— **纯插入**，零既有行重排 |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 翻转 STEP-0511 oracle 钉的 guest 半边；新增 3 条钉 + 2 个 fixture 常量 |
| `tools/probe-m22-frontend-frontier.ps1` | `host-call-in-straight-line-body` 的期望值从 `refused:ERR:E-SH-IR-EXPRESSION` **翻成** `lowered`（前沿**移动**，不是新增用例） |
| oracle 生产代码 | **零改动** |

### 4.2 自造缺陷 #1：把「函数体第一条语句」取错了位置

初版实现用「参数表右括号之后的那一个 token」当函数体第一条语句 ⇒ 拿到的是**签名里的 `returns`**（`returns U64:`），不是 `return` 语句 ⇒ 路由永远 `SKIP`，形状照旧被拒。

**怎么发现的**：临时把路由里 14 道 `SKIP` 闸门逐个换成可区分的 `ERR:E-SH-IR-DHCR-NN` 标记（**诊断标记，测完即摘**），实测落在 **闸门 5**（`dhcr_first` 不是 `return`）。

**修法**：改用**签名行末**定位——`line_end(words, body_from)` 给出签名那一行的 `<nl>`，它的下一个 token 才是函数体第一条语句。

⇒ **记在案**：STEP-0511 §2.2 把落点定界在「`scalar_ir` 的 return 表达式链」，但**没说清怎么取函数体第一条语句**；本片实测发现「参数表之后」与「签名之后」是两回事。

### 4.3 自造缺陷 #2：新 guest 代码里的宿主调用嵌套

初稿里拼实参表写成了 `sico.text.concat("[", sico.u64.to_text(dhcr_value))` —— **宿主调用以宿主调用为实参**。按 STEP-0507 实测的形状规则（宿主调用不能直接吃另一个宿主调用的结果），这个形状 guest 自己降不了 ⇒ 新代码会**越过自举形状前沿**。

**修法**（对齐同族 `direct_bytes_equal_encode_ir` 的既有写法）：先把内层调用落成临时变量再拼：

```
let __nc_direct_host_call_return_ir_1 = sico.u64.to_text(dhcr_value)
let __h1 = sico.text.concat("[", __nc_direct_host_call_return_ir_1)
let dhcr_arguments = sico.text.concat(__h1, "]")
```

### 4.4 自举形状前沿（**实测**）

新路由函数的**传递闭包 = 21 个函数**（`next_index` / `word_at` / `is_word` / `is_ident_byte` / `record_named_type` / `type_json` / `line_end` / `intrinsic_instruction_json` / `find_text_from` / `binding_parameter_index` / `matching_close` / `is_intrinsic_path` / `scalar_type_kind` / `binding_parameter_kind` / `gw_find_code_text` / `return_expression_end` / `parameter_name_start` / `parameters_ir` / `has_return_at` / `return_expression_start` + 本函数）。

⇒ 这 21 个函数拼成的闭包**经 guest 自己**降低，结果与 **Rust 前端逐字节相等**（钉 #3）⇒ **本片没有推进 guest 前端的形状边界**（新代码全部落在既有子集内）。

**本片改了 `scalar_ir`（路由梯接线）⇒ 必须单独复核它的闭包**：`scalar_ir` 的传递闭包 = **158 个函数**，**实测仍 FLIP**（guest 自己逐字节降低 == Rust 前端）⇒ **自举形状前沿没有回退**。

（口径说明：`target/tmp/census-latest.txt` 里那份「149/149 全 FLIP」的普查快照是 **STEP-0502/0503 时期**的产物（当时的 `parser.sico` 只有 149 个函数，现在 172 个），**不能**当作本片的证据；本片用的是**闭包级实测**（同 STEP-0508 §6.4 的口径）。）

### 4.5 前沿探针

`host-call-in-straight-line-body` 的期望值**显式翻转**为 `lowered` ⇒ 前沿探针从「22 条不变」变成「**1 条移动**」；其余 21 条不变。

## 5. 诚实性

### 5.1 组件层**仍然拒绝**（下一刀）

guest 的组件发射走 `selfhost/codegen.sico` 的 `cgs_op_hex`，它**不认 `intrinsic` 算子**（落到末尾的 `ERR:E-SH-CG-IR-EMIT`）⇒ 本形状在组件层仍 typed 拒绝。

**实测（STEP-0512 现场）**：对同一 fixture 跑 guest 的 `--emit-component-hex` ⇒ `invalid-input` / **`unsupported codegen source shape`**（`selfhost/compiler.sico` 把 codegen 的错误统一映射成这一个身份）；同一次跑的 `--emit-ir-split` ⇒ **exit=0**，IR 正常产出 ⇒ **前端层已通、组件层仍拒**。

- **【STEP-0513 落地（2026-10-22）】这一条已被后继片推翻**（原句保留以便追溯）：STEP-0513 的**codegen 刀**让 `cgs_op_hex` 认了 `intrinsic` 算子 ⇒ 同一 fixture 的**整组件 = 1025 B**，与 oracle `compile_script_program` 的**生产输出逐字节相同**并过真实 `wasmparser::Validator`。详见 [STEP-0513](./STEP-0513-m22-w2-s5-host-call-component.md)。

⇒ **本片只闭合 IR 层**；**整组件 1025 B 的逐字节等同是下一刀（STEP-0513，codegen 刀）的目标**。oracle 侧的 1025 B 真值仍钉在固件里（钉 #4）。

### 5.2 没有静默放宽

新路由对**不认领的形状一律返回 `SKIP`** ⇒ 落回既有路径 ⇒ **既有拒绝身份一个都不变**（§3 钉 #2 实测：三实参与 let 绑定位置仍是 `ERR:E-SH-IR-EXPRESSION`）。

### 5.3 支持面收窄

- 只覆盖**一个内建**（`sico.bytes.length`，单实参）、**一种位置**（return 位置）、**一种实参**（形参）。
- `sico.text.length`、`sico.bytes.at`、多实参内建、cell 实参、字面量实参 ⇒ 全部仍 typed 拒绝。
- 直线型函数体里的 **let 绑定位置**（STEP-0511 的 `host-c`）**仍拒**。

### 5.4 其余缺口（承自本线）

- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- oracle 生产代码零改动；**独立 CI 尚未运行**；未提交未推送。
- **M22 整体仍为 NO-GO**（门 3 未动）。

## 6. 验证记录（2026-10-22）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉 ×4 | §3 的四条钉 | 见 §6.1 |
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | 见 §6.1 |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | 见 §6.1 |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | 见 §6.1 |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | 见 §6.1 |

### 6.1 固件与前沿结果

| 项 | 结果 |
|---|---|
| 固件全量 | **117 passed / 0 failed**（STEP-0511 的 114 + 本片 3 条新钉；oracle 钉由「拒绝」翻成「逐字节」，数量不变） |
| fixture 的 guest IR | **2215 B == 2215 B**，逐字节等同 Rust canonical IR |
| 自举前沿闭包 | 新路由 21 个函数 **FLIP**；**被改动的 `scalar_ir` 闭包 158 个函数仍 FLIP**（前沿未回退） |
| 前沿探针 | **22/22** `FRONTIER_UNCHANGED`（`host-call-in-straight-line-body` 的期望值已从 `refused:ERR:E-SH-IR-EXPRESSION` **翻成** `lowered`） |
| IR 计划层前沿探针 | **8/8** `PLAN_FRONTIER_UNCHANGED` |
| 组件层（guest `--emit-component-hex`） | **typed 拒绝** `unsupported codegen source shape`（前端层已通 ⇒ 拒在 codegen 层：§5.1） |
| IR 层（guest `--emit-ir-split`） | **exit=0**，IR 正常产出 |
| 注释纪律 | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2`（新代码在函数体内**零注释**） |
| 行尾审计 | ✅ `LINE_ENDING_AUDIT_GREEN 19 files`（含本片新文档） |

### 6.2 电池结果（如实记录）

本机验证电池（`target/tmp/step0507/battery.ps1`，11 步，日志 `target/tmp/step0507/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 387 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN` |
| `validate-0245` | 70 | 0 | 内含固件全量 **117 passed / 0 failed** + clippy |
| `validate-0262` | 176 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED`（固件 **117 passed**） |
| `probe-frontend-frontier` | 198 | 0 | 前端前沿 **22/22** ⇒ `FRONTIER_UNCHANGED` |
| `probe-ir-plan-frontier` | 39 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 5 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN 19 files` |

⇒ `BATTERY_GREEN 11/11`。

### 6.3 未做的事

- **组件层仍拒**：`cgs_op_hex` 不认 `intrinsic` ⇒ 整组件逐字节等同留给 STEP-0513（§5.1）。
- 只覆盖**一个内建 / 一个位置 / 一种实参**（§5.3）。
- **未提交、未推送**；**独立 CI 尚未运行**。
