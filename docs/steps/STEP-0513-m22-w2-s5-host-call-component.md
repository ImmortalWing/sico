# STEP-0513 m22-w2 S5 宿主调用（**codegen 刀**）：`intrinsic` 算子 ⇒ 整组件 **1025 B** 与 backend 生产输出**逐字节等同**

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0512](./STEP-0512-m22-w2-s5-host-call-frontend.md)（前端刀：IR 层逐字节等同）、[STEP-0511](./STEP-0511-m22-w2-s5-host-call-truth.md)（真值测量片：组件 1025 B 真值钉）、[STEP-0509](./STEP-0509-m22-w2-s5-call-component.md)（codegen 刀：同族写法）
> - gate: 门 3 的 S5 波次 —— **codegen 刀（组件层）**；IR 层已由 STEP-0512 闭合
> - numbering: M22 线顺延为 **STEP-0513**（已核对 `docs/steps/` **无 0513**、`docs/handoff-m22.md` **无 0513**；并行 M24 轨占用 STEP-0330–0337）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ STEP-0511 §1.2（整组件 **1025 B**，过真实 `wasmparser::Validator`） |
| 有**有界出口测试** | ✅ §3（2 条新钉 + STEP-0509 的 `cgs_op_hex` 钉覆盖被改写的分派） |
| IR 层是否已就绪 | ✅ STEP-0512 已闭合（IR **2215 B == 2215 B**）⇒ codegen 层拿得到 `intrinsic` 输入 |

⇒ 取号合法。**实现已完成**，见 §4。

## 1. 真值（backend **生产输出**，逐段解剖）

把 oracle 组件的 core module **code 段**解析出来（`target/tmp/step0513/dump_code.py`），5 个函数体里：

| 模块函数 | 类型 | 体长 | 说明 |
|---|---|---|---|
| fn 0 | type 0 = `(i32, i32) -> i64` | **36 B** | `size`（IR 函数 id 1） |
| fn 1 | type 1 = `(i32,i32,i32,i32) -> i32` | 223 B | `run`（IR 函数 id 2） |
| fn 2/3/4 | — | 84 / 72 / 18 B | 三个冻结的 arena helper |

**fn 0（`size`）的 36 字节**逐段：

```
03 017e 017f 027f          locals：3 组 = 1×i64（指令结果）/ 1×i32（dispatcher）/ 2×i32（scratch）
4100 2103 0240 0340 2003 4100 4604 40    dispatcher 外壳
2001 ad 2102               <-- 宿主调用的指令字节
2002 0f 0b 00 0b 0b 00 0b  直接返回（逐 slot local.get + return）
```

⇒ **`sico.bytes.length` 的真值 = `20 01` / `ad` / `21 02`**，即：

1. `local.get 1` —— **实参的长度槽**（`Bytes` 形参的 (pointer, length) 对里的**第二个**槽；pointer 是第一个槽 0）；
2. `ad` = `i64.extend_i32_u`；
3. `local.set 2` —— **指令自己的结果槽**（参数对占 2 个槽 ⇒ 结果落在槽 2）。

⇒ 槽位规则与 guest **既有的**槽位派生（`cgs_value_slot`：参数在前、指令结果按 IR 顺序）**一致** ⇒ 本片**不需要**新的槽位规则，只需要把这条操作数写出来。

## 2. 本片改了什么

`selfhost/codegen.sico`：

| 改动 | 规模 |
|---|---|
| 新增 `cgs_intrinsic_hex(bytes, run, instr, data, result0)` | **+63 行**（含 2 行文件间注释） |
| `cgs_op_hex` 增加 `intrinsic` 分派 | **+3 行** |

**纯插入**，零既有行重排。

### 2.1 `cgs_intrinsic_hex` 的闸门（全部 fail-closed）

| 闸门 | 条件 |
|---|---|
| 名字 | 必须是 `sico.bytes.length`（**其它内建一律拒**） |
| 实参 | **恰好一个**；类型必须是 `bytes`（**string / list / named 一律拒**） |
| 结果 | 平面槽数必须为 **1**（`u64`） |

任一不满足 ⇒ `ERR:E-SH-CG-IR-EMIT`（**typed 拒绝，不静默取近似**）。

### 2.2 为什么 IR **计划层**没动

`selfhost/codegen.sico` 的 IR 文本层**早就认 `intrinsic`**（`cgpo_intrinsic_text`：`intrinsic <name> args=[...]`）⇒ 计划层与计划层前沿探针（8/8）**零改动**；缺的只是**指令字节**这一层。

## 3. 出口测试（有界）

| # | 钉 | 口径 |
|---|---|---|
| 1 | `sico_codegen_guest_emits_the_host_call_component_byte_exactly` | guest 发射的**整个组件 == oracle `compile_script_program` 的生产输出**（**1025 B**）且过真实 `wasmparser::Validator` |
| 2 | `sico_compiler_lowers_the_step0513_codegen_function_byte_exactly` | **自举形状前沿**：新 codegen 函数（连同其被调桩）经 guest 自己逐字节降低 == Rust 前端 |
| 3 | （既有）`sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly` | 被改写的 `cgs_op_hex` 仍在前沿内（该钉本就含 `cgs_op_hex`） |

## 4. 实施记录（2026-10-22）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/codegen.sico` | 新增 `cgs_intrinsic_hex`（+63 行）+ `cgs_op_hex` 的 `intrinsic` 分支（+3 行）—— **纯插入** |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新增 2 条钉 |
| oracle 生产代码 | **零改动** |
| `tools/probe-m22-frontend-frontier.ps1` | **零改动**（该探针量的是 **IR 层**，本片不动 IR 层） |
| `tools/probe-m22-ir-plan-frontier.ps1` | **零改动**（计划层早就认 `intrinsic`） |

### 4.2 动手前的核对（结论：缺的只是操作数字节）

- **槽位**：guest 既有的 `cgs_value_slot`（参数在前、指令结果按 IR 顺序）自动给出长度槽 1 与结果槽 2，与 backend **一致**；
- **局部声明**：guest 既有的 `cgs_function_local_hex` 自动给出 `017e 017f 027f`（3 组），与 backend **一致**；
- **dispatcher 外壳与直接返回**：guest 既有的 `cgs_function_instr_hex` / `cgs_function_return_hex` 已在 STEP-0507 逐字节等同过。

⇒ 本片**全部**改动 = 「`intrinsic` 这条操作数怎么写成 Wasm 字节」。

### 4.3 自举形状前沿（**实测**）

`cgs_intrinsic_hex` 连同它的被调桩（`cgj_member` / `cgj_ident_text` / `cg_is_word` / `cgj_array_item` / `cgj_value_text` / `cgs_value_slot` / `cgs_value_ty` / `cgs_type_slots` / `cgs_local_get_hex` / `cgs_local_set_hex`）经 guest 自己降低，结果与 **Rust 前端逐字节相等**（钉 #2）⇒ **本片没有推进 guest 前端的形状边界**。

新代码里**没有**「宿主调用以宿主调用为实参」的嵌套：两个被调助手（`cgs_local_get_hex` / `cgs_local_set_hex`）先落成临时变量再拼接（STEP-0507 实测的形状规则）。

### 4.4 本机验证电池**首轮红过一次**：宿主**提交限额**耗尽（**环境敏感项，不是代码回退**）

首轮电池（STEP-0513 的树上）有两个红点：

| 步 | exit | 现象 |
|---|---|---|
| `ci-one-key` | 1 | `test (runner)` 的 `exact_dap_session_drives_real_component_stack_scopes_and_terminal` 断言 `exitCode` = **125**（`trap`），实测 **123**（`cancelled`） |
| `validate-0245` | 1 | 一批 selfhost 钉报 `Launch("页面文件太小，无法完成操作。 (os error 1455)")` |

⇒ **根因 = 宿主 Windows 提交限额被耗尽**：`os error 1455` 的字面含义就是「页面文件太小，无法完成操作」。电池把一键 CI 与 0245 **背靠背**跑，两者各自并发拉起多个带 2.5 GB 内存预算的 guest 进程 ⇒ 瞬时提交量超限 ⇒ 一批 guest **启动失败**（不是判定失败）；DAP 那条则是 trap 与 `terminate` 请求的**竞态**在压力下翻转（`terminate` 先到 ⇒ 退出码从 `125` 变成 `123`）。

⇒ **判为环境敏感项的三条依据**（都可核验）：

1. 两个失败测试的**源码与 `HEAD` 逐字节相同**（`git diff --quiet -- runner/sico-runner/tests/runner.rs` 与 `-- runner/sico-runner/src` 均 exit=0）⇒ 本片**没有**碰过它们；
2. 失败用例的 artifact 是**手工构造的 Rust `Module`**（`trap_debug_artifact()`），**不经过** `selfhost/*.sico` ⇒ 与本片的改动**没有因果路径**；
3. **只降并发**（`RUST_TEST_THREADS=1`）复跑 ⇒ DAP 那条 **ok**、`validate-0245` **exit=0**（内含固件全量 **119 passed / 0 failed**）。

⇒ 处置与 STEP-0502 / STEP-0508 同规：**只降并发，不改断言、不改预算常量**，并整轮重跑电池（§6.2 的表是重跑那一轮的结果）。

## 5. 诚实性

### 5.1 支持面收窄

- **只覆盖一个内建**（`sico.bytes.length`）、**一个实参**（`Bytes` 形参）、**一个结果槽数**（1）。
- `sico.text.length`、多实参内建、`string` / `list` 实参、cell 实参 ⇒ 组件层仍 typed 拒绝 `ERR:E-SH-CG-IR-EMIT`。
- **IR 层**的支持面是 STEP-0512 的那一个形状；本片**没有**扩大它。
- **诚实补充**：`cgs_intrinsic_hex` 的三道闸门里，「实参类型必须是 `bytes`」与「结果槽数必须是 1」这两道**目前从前端走不到**（前端只认 `sico.bytes.length(<Bytes 形参>)` 这一种形状）⇒ 它们是**为后续放宽准备的前置闸门**，本片**没有**为它们产出实测证据。

### 5.2 其余缺口（承自本线）

- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- oracle 生产代码零改动；**独立 CI 尚未运行**；未提交未推送。
- **M22 整体仍为 NO-GO**（门 3 未动）。

## 6. 验证记录（2026-10-22）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉 ×2 + 既有 `cgs_op_hex` 钉 | §3 | ✅ 3 passed / 0 failed |
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | 见 §6.1 |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | 见 §6.1 |
| IR 计划层前沿探针 | `tools/probe-m22-ir-plan-frontier.ps1` | 见 §6.1 |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | 见 §6.1 |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | 见 §6.1 |

### 6.1 固件与前沿结果

| 项 | 结果 |
|---|---|
| 固件全量 | **119 passed / 0 failed**（STEP-0512 的 117 + 本片 2 条新钉） |
| 整组件 | guest 发射 **1025 B** == oracle 生产输出**逐字节相同** + 过真实 `wasmparser::Validator` |
| 自举前沿 | 新 codegen 函数逐字节等同（钉 #2）；被改写的 `cgs_op_hex` 仍在前沿内 |
| 前端前沿探针 | **22/22** `FRONTIER_UNCHANGED`（本片**不动 IR 层** ⇒ 期望值不变） |
| IR 计划层前沿探针 | **8/8** `PLAN_FRONTIER_UNCHANGED`（计划层早就认 `intrinsic`） |
| 注释纪律 | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2`（新代码在函数体内**零注释**） |
| 行尾审计 | ✅ `LINE_ENDING_AUDIT_GREEN` |

### 6.2 电池结果（如实记录）

本机验证电池（`target/tmp/step0507/battery.ps1`，11 步，日志 `target/tmp/step0507/battery.log`）。**本轮在 `RUST_TEST_THREADS=1`（只降并发）下跑**，原因见 §4.4（首轮红点是宿主提交限额耗尽）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 448 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN` |
| `validate-0245` | 138 | 0 | 内含固件全量 **119 passed / 0 failed** + clippy |
| `validate-0262` | 172 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED`（固件 **119 passed**） |
| `probe-frontend-frontier` | 179 | 0 | 前端前沿 **22/22** ⇒ `FRONTIER_UNCHANGED` |
| `probe-ir-plan-frontier` | 32 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 4 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN 20 files` |

⇒ `BATTERY_GREEN 11/11`（**整轮重跑**后的结果；首轮的两个红点与归因见 §4.4）。

### 6.3 未做的事

- **组件层只覆盖一个内建**（§5.1）；`sico.text.length` 等**仍** typed 拒绝。
- **未提交、未推送**；**独立 CI 尚未运行**。
