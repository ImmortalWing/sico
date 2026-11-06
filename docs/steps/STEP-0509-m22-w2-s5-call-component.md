# STEP-0509 m22-w2 S5 调用位置的调用：**codegen 刀**（**已实现，本机全绿**）

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0508](./STEP-0508-m22-w2-s5-call-in-expression.md)（调用位置的调用 = **前端刀**）、[STEP-0507](./STEP-0507-m22-w2-s5-multi-function-component.md)（多函数模块的组件级派生）、[STEP-0504](./STEP-0504-m22-w2-s5-derived-component.md)（派生组件刀）
> - gate: 门 3 的 S5 波次 = guest IR 摄取 + run 体发射 —— **本片是 codegen 刀**：让 guest 的组件发射认识 `call` 操作数
> - numbering: M22 线顺延为 **STEP-0509**（本线已占用 STEP-0491..0499、STEP-0501..0508；已核对 `docs/steps/` 共 404 个 STEP 文件**无 0509**，并行 M24 轨占用 STEP-0330–0337）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ 见 §1（整组件 **1025 B**，且**逐段解剖**出 `call` 的字节） |
| 有**有界出口测试** | ✅ 见 §3（**整组件**与 oracle 生产输出逐字节相同 + 真实 Wasm 校验器） |
| 前端是否需要刀已判定 | ✅ 前端已由 STEP-0508 闭合（IR 逐字节等同），本片**只**缺 codegen 侧 |

⇒ 取号合法。**实现已完成**，见 §4。

## 1. fixture 与真值

fixture = STEP-0508 的 §1.1（`ScriptOutput(exit_code: helper(I64.literal(7)))`），此处不再重复。

### 1.1 组件级真值

oracle `compile_script_program` 对同一 fixture 的生产输出 = **1025 B**（无调用的同形 fixture 是 **1017 B** ⇒ 差 **8 B**）。

### 1.2 逐段解剖（与无调用版逐段对比；全部读自 backend 生产输出）

**差异只落在 `run` 函数的 body 上**（组件的 type / function / export / global 段、data 段、其余四个函数体**逐字节相同**）：

| 段 | 无调用（1017 B） | 调用（1025 B） |
|---|---|---|
| `run` 的局部声明组数 | **21** 组 | **22** 组 |
| `run` 的局部声明 | `7f×4, 7e, 7f×4, 7e, 7f×5, 7e, 7f×5` | `7f×4, 7e, 7e, 7f×4, 7e, 7f×5, 7e, 7f×5` |
| `run` 的 code | … `42 07 21 08` `20 04 21 09` … | … `42 07 21 08` **`20 08 10 00 21 09`** `20 04 21 0a` … |

⇒ **多出来的 8 字节 = 2 字节局部声明组（`01 7e`）+ 6 字节代码**（`20 08` = 推实参槽、`10 00` = 调用、`21 09` = 存结果槽）。

### 1.3 逐指令读出的规则（全部实测，不是推导）

| 规则 | 实测值 |
|---|---|
| 实参的发射 | 实参指令**先**发射（`42 07` = `i64.const 7`，`21 08` = 存进槽 8） |
| 实参的入栈 | `20 08` = `local.get 8`（实参**值的槽**，不是字面量） |
| 调用 | `10 00` = `call 0` ⇒ **Wasm 函数索引 = 0** |
| 结果的落位 | `21 09` = `local.set 9`（调用的结果槽） |
| 目标索引的来源 | IR 的 `call.data.function` = 被调者的**声明 id**（1-based，实测 `1` = `helper`）⇒ **Wasm 索引 = 该函数在模块函数表里的 0-based 位置**（Script profile 的 import 数为 0） |
| 局部声明 | 遵循 backend 自己的规则：参数不声明、每条指令结果的每个 slot 单独一组、dispatcher 与 scratch 对随后 ⇒ 两条新指令结果各多一组 `01 7e` |

## 2. 前端事实（决定本片要哪一刀）

STEP-0508 之后，guest 前端已能把该 fixture 降成与 Rust 前端**逐字节相同**的 IR（`sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly`），但 **codegen 侧不认 `call` 操作数** ⇒ `--emit-component-hex` 仍是 `unsupported codegen source shape`。本片补的正是这一处。

## 3. 出口测试（有界）

1. **组件钉**：guest 对该 fixture 发射的**整个组件**与 oracle `compile_script_program` 的**生产输出逐字节相同**（**1025 B**），并通过真实 `wasmparser::Validator` ⇒ `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly`。
2. **自举形状钉**：本片新增/改写的 3 个 codegen 函数（`cgs_function_position` / `cgs_call_hex` / `cgs_op_hex`）逐个通过 guest 前端降低，与 Rust 前端的 canonical IR **逐字节相同** ⇒ `sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly`。
3. **回归**：STEP-0504/0506/0507 的组件钉与全部既有 lowering 钉**仍绿** ⇒ 固件全量绿。
4. **fail-closed**：**前端**对未声明的被调者已经 typed 拒绝（STEP-0508 §7.2，既有钉覆盖）；**codegen** 侧对「目标 id 在模块函数表里不存在」另有 `ERR:E-SH-CG-IR-EMIT` 的拒绝路径——这是**防御性**的：前端从不产生悬空目标（被调者未声明 ⇒ 前端先拒），因此该路径**未被单独钉**。

## 4. 实施记录（2026-10-22）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/codegen.sico` | 新增 `cgs_function_position`（IR 声明 id ⇒ 模块函数表里的 1-based 位置）、`cgs_call_hex`（`call` 操作数的发射）；`cgs_op_hex` 增加 `call` 分支 |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新钉 `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly` + `sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly` |

### 4.2 关键判断：**既有的派生规则已经够用，缺的只有操作数发射**

动手前先核对了 guest 已有派生规则是否会自动覆盖新 IR（结论：**会**）：

- **槽位**（`cgs_value_slot`）：参数在前、指令结果按 IR 顺序 ⇒ 实参 `const_i64 7` 落在槽 8、`call` 的结果落在槽 9 ⇒ 与 backend 实测的 `20 08` / `21 09` **一致**，**不需要改**。
- **局部声明**（`cgs_function_local_hex`）：逐条指令结果的 flat 类型各成一组 + dispatcher + scratch 对 ⇒ 21 组 → 22 组（多出的正是两条新指令结果的 `01 7e`）⇒ 与实测**一致**，**不需要改**。
- **构造成员**（`cgs_construct_hex`）：字段值按 value id 查槽 ⇒ `exit_code` 字段自动指向调用的结果槽 9 ⇒ **不需要改**。

⇒ 本片的**全部**改动就是「`call` 这条操作数怎么写成 Wasm 字节」。

### 4.3 发射顺序（对齐 backend 的 `Operation::Call` 分支）

1. 逐个实参：按其 flat 槽数，**升序** `local.get`；
2. `call <Wasm 函数索引>`；
3. 结果的 flat 槽数个 `local.set`，**从最高槽往下**（backend 的 `result.iter().rev()`）。

### 4.4 自举形状前沿

本片新增的 codegen 代码必须能被 **guest 自己**逐字节降低。三个函数（`cgs_function_position` / `cgs_call_hex` / `cgs_op_hex`）**一次通过**新钉（形状落在 STEP-0507 已登记的边界内：`let` 在循环体/分支内可用、调用实参可以是调用、宿主调用可带用户调用实参——`cgs_run_index` / `cgs_flat_valtypes` / `cgs_function_return_hex` 已是先例）。

## 5. 诚实性

### 5.1 本片闭合的是**这一个 fixture**的组件层

- 只覆盖**一个 fixture**：调用在 construct 的字段值位置、实参是 `I64.literal(...)`、被调者返回**标量**。
- **表达式位置**（return）与**语句位置**的调用：**前端**仍不降（guest 前端只补了 construct 字段值这一个位置）⇒ 仍是 typed 拒绝。
  > **【STEP-0510 实测纠正（2026-10-22）】这一行是错的。** guest 前端**早已**能逐字节降低「用户调用直接做 return 表达式」这个形状（实测 12 个形状 + 6 个函数体形状变体全部 lowered；且 `selfhost/codegen.sico` 的 `cgs_alloc_index` 就是这个形状，STEP-0507 固件钉**至今仍绿**）⇒ **不是** typed 拒绝。真正还没闭合的是**宿主调用（`sico.*`）在直线型函数体里的表达式位置**（`ERR:E-SH-IR-EXPRESSION` / `ERR:E-SH-IR-CALL-ARGUMENT`）。详见 [STEP-0510](./STEP-0510-m22-w2-s5-call-in-return.md) §1.3 / §2。
- 被调者返回 named / result / option ⇒ 该路径的 ABI（packed checked-result）**未实现** ⇒ typed 拒绝。
- 目标索引的派生假定 **Script profile 的 import 数为 0**（backend 的 `alloc_index`/`function_indices` 都是这个前提）；带 import 的模块不在本片范围。

### 5.2 其余缺口（承自本线）

- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- oracle 生产代码零改动；独立 CI 尚未运行；未提交未推送。
- **M22 整体仍为 NO-GO**（门 3 未动）。

## 6. 验证记录（2026-10-22）

| 验证 | 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | 见 §6.1 |
| 组件钉 | `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly` | ✅ 整组件 **1025 B** == oracle 生产输出，且过 `wasmparser::Validator` |
| 形状钉 | `sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly` | ✅ 3/3 逐字节等同 |
| 回归 | STEP-0504/0506/0507 的组件钉 + 全部既有 lowering 钉 | ✅ 仍绿 |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | ✅ `LINE_ENDING_AUDIT_GREEN` |

### 6.1 固件结果

**111 passed / 0 failed**（STEP-0508 的 109 + 本片 2 个新钉）。

### 6.2 电池结果（如实记录）

本机验证电池 **11 步全部 exit=0**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 2 | 0 | runner workspace 的 `cargo fmt --all` |
| `ci-one-key` | 308 | 0 | 一键 CI **13 步全 PASS** ⇒ `CI GREEN` |
| `validate-0245` | 52 | 0 | 内含固件全量 **111 passed / 0 failed** + clippy |
| `validate-0262` | 137 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 134 | 0 | 前端前沿 **18/18** ⇒ `FRONTIER_UNCHANGED` |
| `probe-ir-plan-frontier` | 28 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 0 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN 15 files` |

**与 STEP-0508 的差别**：STEP-0508 的首轮电池是 **10/11**，唯一红步 `validate-0245` 撞上了宿主 Windows 提交限额（`os error 1455`），靠**只降并发**（`RUST_TEST_THREADS=1`）复跑取证。**本片的首轮电池 11/11 直接全绿** —— 宿主此前的 5.45 GB python 常驻已退出，空闲物理内存约 8.36 GB，所以那个环境敏感项**本轮没有复现**，也就不需要降并发复跑。

### 6.3 未做的事

- **未提交、未推送**；**独立 CI 尚未运行**。
- 组件层**只**闭合了这一个 fixture 的形状；其余调用位置仍 typed 拒绝。
