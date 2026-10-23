# STEP-0510 m22-w2 S5 调用位置的调用（**收口片**）：**return 表达式位置的用户调用 ⇒ IR 与整组件逐字节等同**

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0509](./STEP-0509-m22-w2-s5-call-component.md)（codegen 刀）、[STEP-0508](./STEP-0508-m22-w2-s5-call-in-expression.md)（前端刀）、[STEP-0507](./STEP-0507-m22-w2-s5-multi-function-component.md)（多函数组件刀）
> - gate: 门 3 的 S5 波次 —— **本片不推进门 3**；它把「调用位置的调用」这条线的**两个位置**都用钉固定，并**纠正一条与证据冲突的文档记录**
> - numbering: M22 线顺延为 **STEP-0510**（已核对 `docs/steps/` 共 405 个 STEP 文件**无 0510**；并行 M24 轨占用 STEP-0330–0337）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ 见 §1（canonical IR **2742 B**、整组件 **1075 B**，全部读自生产输出） |
| 有**有界出口测试** | ✅ 见 §3（IR 钉 + **整组件**钉 + 真实 `wasmparser::Validator`） |
| 前端/后端是否需要刀已判定 | ✅ **不需要刀** —— 实测 guest **早已**逐字节降这个形状 ⇒ 本片是**钉住 + 纠偏**，不是实现 |

⇒ 取号合法。**实现已完成**，见 §4。

## 1. fixture 与真值

fixture（`CALL_IN_RETURN_FIXTURE_SOURCE`，记录块与 STEP-0508/0509 同形，省略）：

```
function helper(value: I64) returns I64:
  return value
end function

function helper2(value: I64) returns I64:
  return helper(I64.literal(3))          <-- 调用在 return 表达式位置
end function

function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: helper2(I64.literal(7))))
end function
```

### 1.1 IR 级真值（Rust 前端 canonical IR）

| 量 | 值 |
|---|---|
| Rust 前端 canonical IR | **2742 B** |
| guest 前端 IR | **2742 B**，**逐字节相同** |

IR 结构（实测）：`helper2` 的函数体 = `const_i64 3` → **`call {"function":1,"arguments":[1]}`** → terminator **`return 2`**（返回的是**调用的结果 id**，不是实参）。

### 1.2 组件级真值（backend 生产输出）

| 量 | 值 |
|---|---|
| oracle `compile_script_program` 生产输出 | **1075 B** |
| guest `--emit-component-hex` | **1075 B**，**逐字节相同** |
| 真实 `wasmparser::Validator` | ✅ 通过 |

对照变体（`helper2` **不被** `run` 调用，只有 `helper2` 的 return 位置带调用）：IR **2613 B**、组件 **1067 B**，同样**逐字节相同** ⇒ 结论**不依赖**调用嵌套。

### 1.3 逐位置实测的前沿（12 个形状，全部实测，不是推导）

| 形状 | Rust 前端 | guest 前端 |
|---|---|---|
| `return helper(I64.literal(3))`（return 位置，实参是字面量） | lowered | **lowered** ✅ |
| `return helper(seed)`（实参是形参） | lowered | **lowered** ✅ |
| `return helper()`（零实参） | lowered | **lowered** ✅ |
| `return helper(a, b)`（两个实参） | lowered | **lowered** ✅ |
| `return helper(helper(...))`（实参是调用） | lowered | **lowered** ✅ |
| `let x = helper(...)` + `return x`（let 绑定） | lowered | **lowered** ✅ |
| `helper(...)` 作为**语句** | **refused**（`non-Unit expression statement`） | lowered（guest **更宽松**，见 §5.2） |
| `return helper("abc")`（被调者返回 `Text`，双槽） | lowered | **lowered** ✅ |
| construct 字段值位置上的**双实参**调用 | lowered | **lowered** ✅ |
| `run` 的 construct 字段值位置调用 `helper2` | lowered | **lowered** ✅ |
| `return sico.text.length("abc")`（**宿主**调用） | lowered | **refused** `ERR:E-SH-IR-EXPRESSION` ❌ |
| `helper(sico.text.length("abc"))`（宿主调用当实参） | lowered | **refused** `ERR:E-SH-IR-CALL-ARGUMENT` ❌ |

## 2. 过程中撞到的事实：**「return 位置仍 typed 拒绝」这条记录是错的**

### 2.1 记录说了什么

下列记录都写着「**表达式位置**（return）…的调用**前端**仍不降 ⇒ 仍是 typed 拒绝」：

| 位置 | 原句（节选） |
|---|---|
| [STEP-0509](./STEP-0509-m22-w2-s5-call-component.md) §5.1 | 「**表达式位置**（return）与**语句位置**的调用：**前端**仍不降（guest 前端只补了 construct 字段值这一个位置）⇒ 仍是 typed 拒绝。」 |
| `docs/handoff-m22.md` STEP-0509 条目 ⑤ | 「②**表达式位置**（return）与**语句位置**的调用**前端**仍不降 ⇒ typed 拒绝；」 |
| `docs/steps/README.md` STEP-0509 行 | 「the component layer still covers one fixture only (a call in a return or statement position is still refused by the frontend)」 |
| `docs/plans/M22-compiler-self-host.md` S5 行 | 同上（英文同句） |
| `docs/ROADMAP.md` STEP-0508 行 | 「**表达式位置**（return）与**语句位置**的调用路径返回类型仍写死且未钉」 |
| `docs/STATUS.md` STEP-0508 行 | 「①**本片目的** = …「**表达式位置的调用**仍未实现」；本片先把**最窄的一种**做掉」 |

### 2.2 实测结论：**不成立**

guest **早已**能把这个形状降成与 Rust 前端**逐字节相同**的 IR，并发射出与 backend 生产输出**逐字节相同**的整组件（§1）。三条独立证据：

1. **本片的实测**（§1.3 的 12 个形状 + 6 个函数体形状变体）：`return <user call>` 在**每一种**函数体形状下都 **lowered**。
2. **guest 自己的代码里就有一处这个形状，而且它被钉住了**：`selfhost/codegen.sico` 的
   ```
   function cgs_alloc_index(bytes: Bytes) returns U64:
     return cgs_function_count(bytes)
   end function
   ```
   它是 **STEP-0507 固件钉** `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly` 的 21 个被钉函数**之一**，今天**仍然绿** ⇒ 「用户调用直接做 return 表达式」这个形状**从来没有被 guest 前端拒绝过**。
3. **STEP-0507 §7.2 的表格自己与它的固件钉矛盾**：该表第一行把 `return <user call>` 标成 `ERR:E-SH-IR-CALL-ARGUMENT`，而同一节的改写表只把 `cgs_local_hex` / `cgs_run_instr_hex` 改成「先过 cell」——`cgs_alloc_index` **没有**被改写，却**钉着且绿**。

### 2.3 根因（可追溯）

这条「拒绝」的说法从 STEP-0507 §7.2 的表格第一行开始，沿 STEP-0508 → STEP-0509 一路抄进 5 处同步文档，**中间没有任何一次重新测量**。STEP-0508 §7.3 ⑤ 写的是另一件事（「返回类型仍写死 `{"kind":"u64"}`」），但同样被后续记录当成「前端拒绝」的依据继承下去。

⇒ 这是**记录过时/过窄**，不是 guest 的能力边界。按仓库规范（文档与可执行证据冲突时以证据为准，并在同一次改动里修正权威文档），本片：

1. 用两个新钉**首次**把这个形状固定成**逐字节等同**的事实（§3）；
2. 把 §2.1 表中的 6 处记录改正为实测事实，并**保留**「原记录说了什么」以便追溯；
3. 把真正还没闭合的那一块——**宿主调用（`sico.*`）在直线型函数体里的表达式位置**——作为**下一刀**重新登记（§4.6），而不是继续挂在「用户调用」名下。


## 3. 出口测试（有界）

1. **IR 钉**：`sico_compiler_lowers_the_call_in_a_return_position_byte_exactly` —— guest 的 IR == Rust 前端 canonical IR（2742 B == 2742 B）。
2. **组件钉**：`sico_codegen_guest_emits_the_call_in_a_return_component_byte_exactly` —— guest 发射的**整组件** == oracle 生产输出（1075 B == 1075 B），且过真实 `wasmparser::Validator`。
3. **前沿探针登记**：`tools/probe-m22-frontend-frontier.ps1` 新增 **3 条**用例（`user-call-in-return-position` = `lowered`、`user-call-in-let-binding` = `lowered`、`host-call-in-return-position` = `refused:ERR:E-SH-IR-EXPRESSION`）⇒ 前沿从 18 条变 **21 条**，全部 `FRONTIER_UNCHANGED`。
4. **回归**：STEP-0504/0506/0507/0508/0509 的组件钉与全部既有 lowering 钉**仍绿** ⇒ 固件全量 **113 passed / 0 failed**。
5. **fail-closed**：宿主调用（`sico.*`）在**直线型**函数体里的表达式位置仍 typed 拒绝（`ERR:E-SH-IR-EXPRESSION` / `ERR:E-SH-IR-CALL-ARGUMENT`），并已登记进前沿探针 ⇒ 拒绝是**有身份**的，不是静默回退。

## 4. 实施记录（2026-10-22）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新 fixture `CALL_IN_RETURN_FIXTURE_SOURCE` + 新钉 `sico_compiler_lowers_the_call_in_a_return_position_byte_exactly` + 新钉 `sico_codegen_guest_emits_the_call_in_a_return_component_byte_exactly` |
| `tools/probe-m22-frontend-frontier.ps1` | 前沿表新增 3 条用例（2 条 `lowered` + 1 条 typed 拒绝） |
| `selfhost/*.sico` | **零改动** —— 本片不需要刀（§0） |

### 4.2 为什么本片**不需要**改 guest 源码

动手前先做的是**测量**，不是实现：把 fixture 喂给 guest 前端与 guest 的组件发射，与 Rust 前端 / backend 生产输出逐字节对比。结论是**两边早就相同** ⇒ 缺口不在 guest 的能力，而在**钉与文档**。

这条纪律（先测真值再决定要不要动代码）是本线一直遵守的，也是本片**没有**新增 `selfhost/*.sico` 改动的原因。

### 4.3 自造缺陷 #1：把测试文件截断到**过期的基准长度**，截掉了文件尾部

**现象**：追加临时探针时，我用「本回合开始时的文件长度」当作截断基准；但本回合已经往文件里加了两个新钉 ⇒ 基准**过期** ⇒ 截断把**文件尾部**（`sico_compiler_lowers_checked_arithmetic_match_ir_closure_byte_exactly` 及其后）一起切掉了 ⇒ 编译报 `unclosed delimiter`。

**根因**：截断基准应当取「**上一次写完之后**的文件长度」，而不是「本回合开始时的长度」。

**修法（可复现）**：用 `git show HEAD:<file>` 取出基线，再用 `difflib.SequenceMatcher` 的 opcodes 把**未提交的增量**与**截断**分开：最后一个 `replace` op 就是截断点（HEAD 104 行 vs 截断后 1 行）⇒ 用 **HEAD 的完整尾部**替换它，其余 opcodes（`insert` / 中间的 `replace`）照搬工作区 ⇒ 重建出 243348 B / 5381 行的文件；两处 `argument_values_at`（STEP-0508 加进两个闭包钉的函数清单）落在正确位置；**固件全量 113 passed / 0 failed** ⇒ 重建正确。

**记在案的理由**：同族于 STEP-0497 的「本地绿灯可能是空跑」与 STEP-0504 的「验证基础设施缺陷」。**验证基础设施自身的缺陷**必须写进记录，否则下一片会重犯。

### 4.4 自造缺陷 #2：新钉没过 rustfmt（电池第一步就抓到）

第一次跑电池时，**第 1 步 `fmt-runner` 就红了**（exit=1）：本片新钉里那句一行写不完的 `.expect("the Rust backend emits the Script-profile Component for the call-in-a-return fixture")` 不符合 rustfmt 的折行规则。

⇒ 一条 `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all` 修好，`--check` 复跑为空 ⇒ 重跑整轮电池。

⇒ **顺带记一条正面证据**：电池把「新增代码没过格式化」挡在**第一步**，没让它混进后面 10 步的结论里 —— 这正是「先跑最窄的门」的价值。

### 4.5 自造缺陷 #3：临时探针混进了电池的固件计数

**现象**：电池跑完后我核固件数，发现 `validate-0245` 里报的是 **114 passed**，而我在**清理过的**树上直接跑是 **113 passed**。

**根因**：我在追加最后一个临时探针（形状对照表）之后**没有再把它摘掉**，电池就在**带着探针**的树上跑完了。探针本身自带 1 个 `#[test]` ⇒ 计数多 1。

**处置**：把测试文件按**探针标记**裁回（而不是按「本回合开始时的长度」——这正是缺陷 #1 的同一个错法），核到 **113 个 `#[test]`、0 个探针标记、2 处 `argument_values_at`、无 CRLF / BOM**，然后**在清理过的树上重跑整轮电池**。

⇒ **两条教训**（同族于缺陷 #1）：
- 裁剪基准要用**内容标记**（探针标记），不要用**长度**；
- **电池结果必须与清理后的树一一对应**：固件计数与单独复跑不一致时，先怀疑「树脏了」，别急着记进文档。

### 4.6 宿主调用缺口的**精确定位**（下一刀的范围）

宿主调用不是「一律不支持」，而是**按函数体路径**分叉（实测）：

| 形状 | guest |
|---|---|
| 直线型函数体：`let n = sico.text.length(name)` + `return n` | **refused** `ERR:E-SH-IR-EXPRESSION` |
| 同上但函数体里有 `while` | **lowered** ✅ |
| `while` 之后 `return sico.text.length(name)` | **lowered** ✅ |
| `if` 体里 `return sico.text.length(name)` | **lowered** ✅ |
| 宿主调用当**实参** | **refused** `ERR:E-SH-IR-CALL-ARGUMENT` |

⇒ **下一刀 = 直线型函数体路径的宿主调用支持**（前端），以及随之而来的 `Operation::Intrinsic` 的组件发射（codegen）。backend 侧已具备内建发射能力（`emit_intrinsic` 覆盖 `sico.bytes.length` 等），所以真值侧不缺。

## 5. 诚实性

### 5.1 本片**没有推进 guest 的能力**

- `selfhost/*.sico` **零改动**。本片做的是：**钉住一个早已存在但从未被钉过的能力** + **纠正一条与证据冲突的文档记录**。
- 只覆盖**一个 fixture**（一个用户调用在 return 位置、实参是 `I64.literal(...)`、被调者返回标量）；§1.3 的表格是**形状级实测**，不是逐个字节钉。

### 5.2 guest 比参考实现更宽松的一处（**不是**支持声明）

语句位置的裸调用（`helper(I64.literal(3))` 单独成句）：Rust 前端**拒绝**（`Unsupported { feature: "non-Unit expression statement" }`），guest 却能降。

⇒ 参考实现拒绝的形状**不进支持面**；这里只如实登记为**分歧**，不作任何支持主张。

### 5.3 其余缺口（承自本线）

- **宿主调用**（`sico.*`）在**直线型**函数体的表达式位置仍 typed 拒绝（§4.6）⇒ **下一刀**。
- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- oracle 生产代码零改动；**独立 CI 尚未运行**；未提交未推送。
- **M22 整体仍为 NO-GO**（门 3 未动）。

## 6. 验证记录（2026-10-22）

| 验证 | 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | **113 passed / 0 failed**（STEP-0509 的 111 + 本片 2 个新钉） |
| IR 钉 | `sico_compiler_lowers_the_call_in_a_return_position_byte_exactly` | ✅ 2742 B == 2742 B |
| 组件钉 | `sico_codegen_guest_emits_the_call_in_a_return_component_byte_exactly` | ✅ 1075 B == oracle 生产输出，且过 `wasmparser::Validator` |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | ✅ **21/21** `FRONTIER_UNCHANGED`（新增 3 条） |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.1 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | ✅ `LINE_ENDING_AUDIT_GREEN` |

### 6.1 电池结果（如实记录）

本机验证电池 **11/11 全绿**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`；**跑在清理掉临时探针的树上**）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 2 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 234 | 0 | 一键 CI **13 步全 PASS** ⇒ `CI GREEN` |
| `validate-0245` | 48 | 0 | 内含固件全量 **113 passed / 0 failed** + clippy |
| `validate-0262` | 128 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 137 | 0 | 前端前沿 **21/21** ⇒ `FRONTIER_UNCHANGED`（本片新增 3 条） |
| `probe-ir-plan-frontier` | 25 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 0 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN 17 files` |

⇒ `BATTERY_GREEN 11/11`。

**这一轮电池之前还红过两次，都记在案**（§4.4 / §4.5）：第一次是**第 1 步 `fmt-runner`** 抓到新钉没过 rustfmt；第二次是电池跑完才发现**树上还留着临时探针**（固件 114 ≠ 清理后的 113）⇒ 清理后**整轮重跑**，本表是**清理后那一轮**的结果。

### 6.2 未做的事

- **未提交、未推送**；**独立 CI 尚未运行**。
- **guest 源码零改动**：宿主调用（`sico.*`）在直线型函数体里仍 typed 拒绝 ⇒ 留给下一刀。
- 组件层**只**闭合了 §1.3 表格里的形状，不是通用组件发射器。
