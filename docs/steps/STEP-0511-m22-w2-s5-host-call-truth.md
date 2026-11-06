# STEP-0511 m22-w2 S5 宿主调用（**真值测量片**）：**直线型函数体里的 `sico.*` 调用 ⇒ 组件 1025 B 真值钉 + 补丁落点定界**

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0510](./STEP-0510-m22-w2-s5-call-in-return.md)（前沿测绘 + 纠偏）、[STEP-0509](./STEP-0509-m22-w2-s5-call-component.md)（codegen 刀）、[STEP-0508](./STEP-0508-m22-w2-s5-call-in-expression.md)（前端刀）
> - gate: 门 3 的 S5 波次 —— **本片只测量、不实现**（与 STEP-0501 / STEP-0505 同规）；**本片不推进门 3**
> - numbering: M22 线顺延为 **STEP-0511**（已核对 `docs/steps/` 共 406 个 STEP 文件**无 0511**；并行 M24 轨占用 STEP-0330–0337）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ 见 §1.2（整组件 **1025 B**，过真实 `wasmparser::Validator`） |
| 有**有界出口测试** | ✅ 见 §3（一个真值钉：组件长度 + 校验器 + canonical IR 形状 + guest 的 **typed** 拒绝） |
| 前端是否需要刀已判定 | ✅ **需要**，且**补丁落点已定界**（§2）——这一片把「下一刀改哪里」从猜测变成实测 |

⇒ 取号合法。**测量已完成**，见 §4。

## 1. fixture 与真值

### 1.1 fixture（`HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE`）

```
function size(data: Bytes) returns U64:
  return sico.bytes.length(data)          <-- 直线型函数体里的宿主调用
end function

function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))
end function
```

（记录块与 Script profile 同形，省略。）`size` 是**直线型**函数体：**没有** `let` / `set` / `if` / `while` / `match` —— 这正是 STEP-0510 §4.6 实测出的那条分界线。

### 1.2 组件级真值（backend 生产输出）

| 量 | 值 |
|---|---|
| oracle `compile_script_program` 生产输出 | **1025 B** |
| 真实 `wasmparser::Validator` | ✅ 通过 |
| guest `--emit-component-hex` | **typed 拒绝** `unsupported codegen source shape`（前端先拒 ⇒ 组件层根本没走到） |

⇒ **真值存在**：backend 真的为这个形状发射组件 ⇒ 下一刀有可对齐的字节目标。

### 1.3 IR 级真值（Rust 前端 canonical IR）

`size` 的函数体（实测）：

```json
{"result":1,"ty":{"kind":"u64"},"operation":{"op":"intrinsic","data":{"name":"sico.bytes.length","arguments":[0]}},"range":{"start":368,"end":391}}
```

⇒ 宿主调用的 canonical 形状 = **一条 `intrinsic` 算子**：`data.name` = 点分内建名、`data.arguments` = **实参的 value id**（不是字面量）、`ty` = 内建的结果 kind（`sico.bytes.length` ⇒ `u64`）。

### 1.4 guest 当前的位置（fail-closed，有身份）

| 阶段 | 结果 |
|---|---|
| guest 前端 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` |
| guest 组件发射 | 未走到（前端已拒 ⇒ `unsupported codegen source shape`） |

⇒ 拒绝**有身份**，不是静默回退（ADR-0016 决策 6）。

## 2. 补丁落点（已定界，下一刀不必再摸索）

### 2.1 缺口**不是**「没有机器」

guest 里**已经有**产出 canonical `intrinsic` 算子的机器：`selfhost/parser.sico` 的 `intrinsic_instruction_json(result_id, kind, name, arguments, range_start, range_end)`，以及把 `sico.<ns>.<fn>(<实参>)` 整个降下来的 `gw_intrinsic_call_packed`（**单实参**版本：实参可以是形参或 cell，产出 `{"op":"intrinsic","data":{"name":...,"arguments":[实参 id]}}`）。

⇒ **缺的不是能力，是接线**：直线型函数体这条路径没有把宿主调用交给那台机器。

### 2.2 接线的确切位置（实测路由）

`selfhost/parser.sico` 的 `scalar_ir`（19878 行起）是**裸函数体**的默认路由：先试 `fixed_guard_chain_ir`（对宿主调用**返回 SKIP 而不是 ERR** —— 否则 dispatcher 会直接把那个 ERR 抛出来，而 guest 实际报的是 `ERR:E-SH-IR-EXPRESSION`）→ 再试 `general_while_function_ir` / `control_if_function_ir` / `straight_cell_function_ir`（都要求 `while` / `if` / `set`）⇒ 都落空 ⇒ 进 `scalar_ir` 自己的 return 表达式处理。

在 `scalar_ir` 的表达式链里：

- **单原子**分支（`20461`）只认字面量 / `true` / `false` / 形参 ⇒ 别的 ⇒ `20410` 的 `ERR:E-SH-IR-EXPRESSION`；
- **多词**分支（`20478`）先问「`name(` 吗」⇒ 是 ⇒ **用户调用**路径（`20479`–`20497`，`call_arguments_json` / `call_target_idx`）；否 ⇒ 落进 `20498` 的 else ⇒ `fixed_op`（`I64` / `U64` 接收者）⇒ 再落进 `20533` 的 else ⇒ `I64.literal` / `U64.literal` 的**固定字面量**形状。

⇒ **`sico.bytes.length(data)` 正好掉进最后这一档**：接收者既不是 `I64` 也不是 `U64`，也不是 `literal` ⇒ 没有任何分支认领 ⇒ typed 拒绝。

⇒ **下一刀的前端落点** = 在 `scalar_ir` 的表达式链里补一个**宿主调用分支**（识别 `sico . ns . fn ( 实参 )` ⇒ 交给 `gw_intrinsic_call_packed`），并让 `cgs_op_hex` 认 `intrinsic` 算子（第二片）。

### 2.3 为什么这次值得先测

- 真值（**1025 B** + canonical IR 形状）已钉进固件 ⇒ 实现片一旦落地，**逐字节**对齐有 oracle；
- 落点已定界 ⇒ 实现片不必再做「哪种路径接它」的摸索；
- 现状是**typed 拒绝** ⇒ 实现片落地前，前沿探针的期望值不会漂移。

## 3. 出口测试（有界）

1. **真值钉**：`oracle_host_call_in_a_straight_line_body_truth_is_pinned` —— 组件 **1025 B**（过真实 `wasmparser::Validator`）+ canonical IR 含 `{"op":"intrinsic"` / `"name":"sico.bytes.length"` / `"arguments":[0]` + guest 当前以 `ERR:E-SH-IR-EXPRESSION` **typed** 拒绝。
2. **前沿探针登记**：`tools/probe-m22-frontend-frontier.ps1` 新增 **1 条**用例（`host-call-in-straight-line-body` = `refused:ERR:E-SH-IR-EXPRESSION`）⇒ 前沿从 21 条变 **22 条**。
3. **回归**：STEP-0504/0506/0507/0508/0509/0510 的组件钉与全部既有 lowering 钉**仍绿** ⇒ 固件全量 **114 passed / 0 failed**。
4. **fail-closed**：guest 的拒绝**有身份**（`ERR:E-SH-IR-EXPRESSION`），且已登记进前沿探针 ⇒ 下一刀落地时这条期望值会**显式**从 `refused` 翻成 `lowered`，不会被静默放过。

## 4. 实施记录（2026-10-22）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新 fixture `HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE` + 新钉 `oracle_host_call_in_a_straight_line_body_truth_is_pinned` |
| `tools/probe-m22-frontend-frontier.ps1` | 前沿表新增 1 条用例（typed 拒绝） |
| `selfhost/*.sico` | **零改动** —— 本片只测量（§0） |

### 4.2 测量手法（3 个变体，一次量完）

| 变体 | 形状 | oracle 组件 | guest |
|---|---|---|---|
| `host-a` | `return sico.bytes.length(data)`（单实参，return 位置） | **1025 B** | `ERR:E-SH-IR-EXPRESSION` |
| `host-b` | `return sico.bytes.at(data, index, I64.literal(0))`（**三实参**） | **1053 B** | `ERR:E-SH-IR-EXPRESSION` |
| `host-c` | `let n = sico.bytes.length(data)` + `return n`（**let 绑定**位置） | **1025 B** | `ERR:E-SH-IR-EXPRESSION` |

⇒ 三个变体在 backend 侧**都真的发射**，在 guest 侧**都** typed 拒绝 ⇒ 缺口是**整类**的，不是某一个内建的个例。

`host-b` 的 IR 还量出了**多实参**的形状：`const_i64 0` 先占一个 id ⇒ `intrinsic` 的 `arguments` = `[0, 1, 2]`（三个实参 id）⇒ backend 侧三实参内建**有真值**；但 guest 的 `gw_intrinsic_call_packed` 只处理**单实参** ⇒ 多实参不在本片范围。

### 4.3 本片**没有**动 guest 源码的理由

真值已测得、落点已定界、现状是 typed 拒绝 ⇒ 实现片（前端刀 + codegen 刀）的前置条件**齐了**；而实现本身要动 `selfhost/parser.sico` 的 `scalar_ir` 与 `selfhost/codegen.sico` 的 `cgs_op_hex`，且新代码必须能被 guest 自己逐字节降低（自举形状前沿）⇒ 那是**两片**的工作量。按本线纪律（STEP-0501 / STEP-0505 先测后做），本片只把真值与落点**钉住**。

### 4.4 自造缺陷 #1（**同一个陷阱，第二次**）：新钉没过 rustfmt

电池**第 1 步 `fmt-runner`** 又红了（exit=1）：本片新钉里那个 `let RunOutcome::Domain { code, message } = run_guest(...)` 的折行不符合 rustfmt 规则。

⇒ 一条 `cargo fmt` 修好、`--check` 复跑为空 ⇒ 重跑整轮电池。

⇒ **记在案**：STEP-0510 §4.4 已经记过同一个坑（那次是新钉里的 `.expect(...)` 一行写不完）。**两次都是「新钉的长表达式没折行」** ⇒ 下一片写新钉时**先跑一次 `cargo fmt --check`**，别指望电池替你兜底（虽然它确实兜住了，但代价是重跑一轮 12 分钟的电池）。

## 5. 诚实性

### 5.1 本片**只测量，不实现**

- guest 源码**零改动**；组件层与 IR 层**都仍** typed 拒绝这个形状。
- 本片的产出是**真值 + 落点 + 一个 oracle 钉**，不是能力。

### 5.2 真值的覆盖范围

- 只覆盖**一个 fixture**、**一个内建**（`sico.bytes.length`，**单实参**）。
- `sico.bytes.at`（三实参）backend 也发（1053 B），但 guest 的机器只支持单实参 ⇒ 不在本片范围。
- 直线型函数体里的 **let 绑定位置**同样被拒（`host-c`）⇒ 下一刀若只做 return 位置，`host-c` 仍会拒 ⇒ **下一刀的范围要显式写清楚**。

### 5.3 其余缺口（承自本线）

- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- oracle 生产代码零改动；**独立 CI 尚未运行**；未提交未推送。
- **M22 整体仍为 NO-GO**（门 3 未动）。

## 6. 验证记录（2026-10-22）

| 验证 | 口径 | 结果 |
|---|---|---|
| 真值钉 | `oracle_host_call_in_a_straight_line_body_truth_is_pinned` | ✅ 组件 **1025 B** + 真实校验器 + canonical IR 形状 + guest typed 拒绝 |
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | 见 §6.1 |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | 见 §6.1 |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | ✅ `LINE_ENDING_AUDIT_GREEN` |

### 6.1 固件与前沿结果

| 项 | 结果 |
|---|---|
| 固件全量 | **114 passed / 0 failed**（STEP-0510 的 113 + 本片 1 个真值钉） |
| 前沿探针 | **22/22** `FRONTIER_UNCHANGED`（本片新增 `host-call-in-straight-line-body` = `refused:ERR:E-SH-IR-EXPRESSION`） |
| IR 计划层前沿探针 | **8/8** `PLAN_FRONTIER_UNCHANGED` |

### 6.2 电池结果（如实记录）

本机验证电池 **11/11 全绿**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 5 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 355 | 0 | 一键 CI **13 步全 PASS** ⇒ `CI GREEN` |
| `validate-0245` | 57 | 0 | 内含固件全量 **114 passed / 0 failed** + clippy |
| `validate-0262` | 145 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED`（固件 **114 passed**） |
| `probe-frontend-frontier` | 158 | 0 | 前端前沿 **22/22** ⇒ `FRONTIER_UNCHANGED` |
| `probe-ir-plan-frontier` | 27 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 0 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN 18 files` |

⇒ `BATTERY_GREEN 11/11`。

**这一轮电池之前红过一次**（§4.4）：第 1 步 `fmt-runner` 抓到新钉没过 rustfmt ⇒ 修好后**整轮重跑**，本表是重跑那一轮的结果。

### 6.3 未做的事

- **guest 源码零改动**：直线型函数体里的宿主调用**仍** typed 拒绝 ⇒ 留给下一刀。
  - **【STEP-0512 落地（2026-10-22）】这一条已被后继片推翻**（原句保留以便追溯）：STEP-0512 的**前端刀**让这个形状降成了 canonical `intrinsic` IR（与 Rust 前端 **2215 B == 2215 B** 逐字节等同），本钉的 guest 半边也按本文件 §3 的**承诺**从 typed 拒绝**翻成逐字节比较**；组件层**仍拒**（`cgs_op_hex` 不认 `intrinsic` 算子）⇒ 整组件 1025 B 的逐字节等同留给 **STEP-0513（codegen 刀）**。详见 [STEP-0512](./STEP-0512-m22-w2-s5-host-call-frontend.md)。
- 只覆盖**一个 fixture / 一个单实参内建**（§5.2）；三实参与 let 绑定位置的 backend 真值已测得，但**不在本片范围**。
  - **【STEP-0512 实测】**这两条**仍** typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（新钉 `sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused`）⇒ 本片的支持面**没有**被悄悄放宽。
- **未提交、未推送**；**独立 CI 尚未运行**。
