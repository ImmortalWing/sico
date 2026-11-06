# STEP-0518 m22-w2 S5 **W2 族 T1 前端刀（F）**：`return sico.text.concat(<形参>, <形参>)` ⇒ canonical `intrinsic` IR 与 Rust 前端**逐字节等同**

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（上一片 · T1 真值测量片）、[STEP-0515](./STEP-0515-m22-w2-s5-host-call-let-binding.md)（宿主调用前端刀的方法学先例）
> - gate: **不推进任何出口门** —— W2 的**第 2 拍（前端刀 F）**；**族未闭合**（组件层仍拒 ⇒ 按计划 §3.3.5 的族级出口判据，**五项缺一不算闭合**）
> - numbering: **STEP-0518**（取号前已核对 `docs/steps/`：最大号 0517；*0518 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| owner 已授权 W2 族按「每族四拍」推进。 | ✅ 计划 §3.3.5 |
| 上一拍（真值测量片 T）已给出真值与切点。 | ✅ STEP-0517：规则表 + 子集边界 + 下一刀切点 |
| 本拍是**实现片**：改 `selfhost/parser.sico`，**纯插入、零既有行重排**。 | ✅ §4.1（自证 removed=0） |
| 出口测试有界。 | ✅ §3：3 条新钉 + 1 条邻接钉更新 + 前沿探针翻转 |

⇒ **取号合法**。

## 1. 这一片回答的问题（阶段的决策棱镜）

STEP-0517 的结论把 T1 的下一刀**压缩成了一条路径**：guest **早就实现了** `sico.text.concat` 的 return 位置下降（`straight_cell_function_ir` / `straight_environment_function_ir` 里都有 return 位置的 concat 分支），缺的只是**可达性**——路由梯把整个 `straight_cell_*` 家族门控在「函数体除 return 外还有一条语句」的条件下。

于是本片的问题**不是**「怎么实现 T1 的 return 位置」，而是：

1. **能不能只开一条可达路径，而不改动任何既有判定？** ⇒ 答案必须是**能**，且必须**纯插入**。
2. **这条路径认领哪一个形状？** ⇒ 必须**恰好一个**，其余一律 `SKIP`，**既有拒绝身份不变**。
3. **新代码本身能不能被 guest 自己逐字节降低？** ⇒ 自举形状前沿必须**不推进**。

## 2. 实施设计

### 2.1 落点

| 项 | 落点 |
|---|---|
| 新函数 | `selfhost/parser.sico` 的 `direct_text_concat_return_ir`（**+146 行**，含 2 行注释） |
| 接线 | `scalar_ir` 路由梯：插在 `direct_host_call_let_ir` 分支之后、`fixed_guard_chain_ir` 之前（**+24 行**） |
| 合计 | **+170 行 / −0 行**，**纯插入、零既有行重排** |
| 新造机器 | **零** —— 复用既有 `parameters_ir` / `intrinsic_instruction_json` / `return_expression_start` / `return_expression_end` / `binding_parameter_index` / `binding_parameter_kind` / `is_intrinsic_path` / `matching_close` / `gw_find_code_text` |

**为什么不调用 13 实参的 `straight_text_concat_packed`**：本片认领的形状里，两个操作数**都是形参** ⇒ 没有字面量、没有 cell、没有嵌套调用 ⇒ canonical IR 就是**一条 `intrinsic`**（操作数即两个形参的值 id），用既有 `intrinsic_instruction_json` 直接拼即可 —— 与 STEP-0512/0515 的 `direct_host_call_return_ir` / `direct_host_call_let_ir` **完全同族**。

### 2.2 支持面 = 恰好一个形状

```
function join(a: Text, b: Text) returns Text:
  return sico.text.concat(a, b)
end function
```

即：**2 个形参**、声明返回 `Text`、函数体**唯一语句**是 `return sico.text.concat(<Text 形参>, <Text 形参>)`。

**闸门（全部 `SKIP`，一律不静默放宽）**：

| 闸门 | 条件 |
|---|---|
| 形参数 | `parameter_count == 2` |
| 声明返回类型 | `string` |
| 函数体 | 第一条语句是 `return`，其后**直接**是 `end function`（⇒ 只有一条语句） |
| 调用路径 | `is_intrinsic_path(..., "text", "concat")` |
| 调用形状 | `(` 后**恰好两个**操作数、`,` 分隔、`)` 收尾 |
| 操作数类型 | 两个都是 **`Text` 形参**（`binding_parameter_index < parameter_count` 且 `binding_parameter_kind == "string"`） |

### 2.3 自举形状陷阱（本片踩到并绕开的一条）

新路由自己的代码**不能**出现 `return sico.text.concat(...)` —— 那正是它要开启的形状 ⇒ **循环依赖**（guest 降自己的新代码时需要自己的新路由）。**绕法**：像 `direct_host_call_let_ir` 那样，末尾用 `let` 累加 JSON 片段、`return` 的是一个**局部变量**。这个形状已被 STEP-0515 的自举前沿钉证明可降。

## 3. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 |
|---|---|---|
| 1 | `sico_compiler_lowers_the_text_concat_as_the_whole_expression_byte_exactly` | guest canonical IR **逐字节** == Rust 前端 canonical IR（**2295 B**）；intrinsic 形状 = `{"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}`；range **374..396**；terminator `{"kind":"return","data":2}` |
| 2 | `sico_compiler_lowers_the_step0518_parser_function_byte_exactly` | **自举形状前沿**：新路由闭包 **20** 个函数经 guest 自己逐字节降低 == Rust 前端 ⇒ 未推进形状边界 |
| 3 | `sico_compiler_keeps_the_unclosed_text_concat_shapes_typed_refused`（**更新**） | 三个**未认领**形状仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`：两个字面量实参 / 混合实参对 / **嵌套宿主调用** |

⇒ 固件从 **127** 增至 **129** 条。

## 4. 实施记录（2026-10-23）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/parser.sico` | 新增 `direct_text_concat_return_ir`（**+146 行**）+ `scalar_ir` 路由梯接线（**+24 行**）= **+170 / −0** |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新增 2 条钉 + 更新 1 条邻接形状钉 |
| `tools/probe-m22-frontend-frontier.ps1` | **翻转** 1 条期望值 + 新增 2 条用例（26 → **28** 条） |
| `selfhost/codegen.sico` / oracle 生产代码（`crates/*`） | **零改动** |
| `docs/*` | 同步本片结论（5 处） |

### 4.2 纯插入自证（不是口头声明）

`target/tmp/step0518/audit_pure_insertion.py`：把本片插入的**两块**（新函数块 + 路由梯块）从当前文件里**精确还原**掉，再与当前文件做行级 diff ⇒

```
before_lines=22172 after_lines=22342
inserted_lines=170
removed_lines=0
PURE_INSERTION_GREEN removed=0
```

⇒ **170 行全是新增，零既有行被删除或重排**。

### 4.3 出口达成

| 项 | 值 |
|---|---|
| canonical IR | **2295 B** == Rust 前端 **2295 B**（**逐字节相同**） |
| intrinsic | `{"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}` |
| intrinsic range | **374..396**（= 调用表达式的文本跨度） |
| terminator | `{"kind":"return","data":2}` |
| 自举前沿 | 闭包 **20** 个函数逐字节等同 ⇒ **未推进形状边界** |
| 固件 | **129 passed / 0 failed**（STEP-0517 的 127 + 本片 2 条） |
| 前沿探针 | **28/28** ⇒ `FRONTIER_UNCHANGED` |

### 4.4 没有静默放宽

STEP-0517 测到的**四个**未闭合形状里，**一个**被本片认领（`return sico.text.concat(形参, 形参)`），其余**三个**保持 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`：

| 形状 | 判读 |
|---|---|
| 两个**字面量**实参（`sico.text.concat("a", "b")`） | ❌ `ERR:E-SH-IR-EXPRESSION` |
| **混合**实参对（`sico.text.concat(a, "\t")`） | ❌ `ERR:E-SH-IR-EXPRESSION` |
| **嵌套**宿主调用（`sico.text.concat(kind, sico.text.concat(":", name))`） | ❌ `ERR:E-SH-IR-EXPRESSION` |

⇒ 前沿探针**新增 2 条用例**把前两个钉成 `refused:ERR:E-SH-IR-EXPRESSION`，第三个由固件钉覆盖。

### 4.5 自造缺陷 ①：**审计脚本的方向写反了**

第一版纯插入审计把 `current` 当作「改动前」、`reconstructed` 当作「改动后」⇒ 报告说「删了 170 行」——**恰恰是本片新增的那 170 行**。**根因**：diff 的两个操作数放反了。**修法**：`before = reconstructed`（还原后的旧文件）、`after = current`（当前文件），只有 `delete`/`replace` 才算删除 ⇒ `removed_lines=0`。**教训**：**自证脚本本身也会说谎**，必须交叉核对（第一次跑出来的「删除 170 行」与「本片是纯插入」直接矛盾 ⇒ 立刻定位到方向问题）。

### 4.6 自造缺陷 ②：**新路由若调用 13 实参的机器 ⇒ 闭包无法自证**

设计初稿曾打算直接复用 `straight_text_concat_packed`（它已经支持 N 个实参、字面量、cell 与嵌套）。**为什么放弃**：①那个机器的**调用点**要传 **13 个实参**，而 guest 的自举前沿从未证明过「13 实参的用户调用」可降（`straight_cell_function_ir` / `straight_environment_function_ir` 都不在任何闭包钉里）⇒ **自证会失败**；②本形状根本不需要它（两个操作数都是形参）。**改法**：改用既有 `intrinsic_instruction_json` 直接拼 canonical IR ⇒ 闭包与 STEP-0512 的 `direct_host_call_return_ir` **完全同族**（20 个函数，已被证明可降）。

### 4.7 自造缺陷 ③：**新路由的代码不能自己用 `return sico.text.concat(...)`**

见 §2.3。若那样写，guest 降自己的新代码时会需要自己的新路由 ⇒ **循环依赖**。**修法**：末尾 `let` 累加 + `return` 局部变量（`direct_host_call_let_ir` 同款）。

## 5. 诚实性

### 5.1 本片**只闭合前端面**，**族未闭合**

- **组件层仍然拒绝**：该 fixture 的 `--emit-component-hex` 依旧 fail-closed `unsupported codegen source shape`（`cgs_intrinsic_hex` 只认 `sico.bytes.length`，STEP-0513）⇒ 按计划 §3.3.5 的族级出口判据（canonical IR 逐字节等同 **且** 整组件逐字节等同 **且** `wasmparser::Validator` 通过 **且** 前沿探针已翻转 **且** 独立 CI 绿，**五项缺一不算闭合**），**T1 族尚未闭合**。
- **未闭合的 T1 形状**：两个字面量实参、混合实参对、嵌套宿主调用、let 绑定位置的字面量/cell 变体、**体内另有语句**的其它位置变体，以及 S 内 2,882 个调用点的绝大部分 ⇒ **语料覆盖仍为 0 条**。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- **M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。

### 5.2 本片**没有**改动的

- oracle 生产代码（`crates/*`）**零改动**；`selfhost/codegen.sico` **零改动**。
- **不改任何预算常量**、不改任何既有断言的期望值（只翻转前沿探针里**本片正好认领**的那一条）。
- **未提交、未推送**；**独立 CI 尚未运行**。

### 5.3 下一刀（W2 族 T1 的第 3 拍 / codegen 刀 C）

按 STEP-0517 §4.5 的发射真值：

1. `cgs_intrinsic_hex` 认 `sico.text.concat`，**四道闸门全 fail-closed**（名字 / 实参个数 = 2 / 实参类型 = `string` / 结果槽数 = 2）。
2. 与 STEP-0513 的 `sico.bytes.length`（**内联**发射）**不同**：`sico.text.concat` 必须**发射一个 helper 函数体**（arena 分配 + 两段 `MemoryCopy`）+ 调用点 `call <helper>` ⇒ 工作量大一档。
3. helper 里的拷贝是**线性**的 ⇒ 正好落在 STEP-0516 的「线性组装」约束内；guest 现有的**增量拼接**路径（`cg_leb_hex` / `cg_hex_byte` / `cg_section_hex`）**不得**沿用。
4. 出口判据：整组件 **1094 B** 与 oracle 生产输出**逐字节相同** + 过真实 `wasmparser::Validator`。

## 6. 验证记录（2026-10-23）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉（3 条定向） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --exact <3 条> --test-threads=1` | ✅ **3 passed / 0 failed** |
| 固件全量 | 同上，去掉 `--exact` | ✅ **129 passed / 0 failed** |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | ✅ **28/28** ⇒ `FRONTIER_UNCHANGED` |
| 纯插入自证 | `python .\target\tmp\step0518\audit_pure_insertion.py` | ✅ `PURE_INSERTION_GREEN removed=0`（**170 行新增**） |
| 注释纪律 | `python .\tools\validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| clippy / rustfmt | 见 §6.2 | ✅ |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |

### 6.1 T1 族状态速查

| 拍 | 状态 | 证据 |
|---|---|---|
| 第 1 拍（真值测量 T） | ✅ 完成 | STEP-0517：10 变体规则表 + 子集边界 + 发射真值 |
| **第 2 拍（前端刀 F）** | ✅ **本片完成** | 本片：canonical IR 逐字节等同（2295 B）+ 自举前沿未推进 |
| 第 3 拍（codegen 刀 C） | ❌ 未开始 | 组件层仍 fail-closed 拒绝；下一刀 |
| 第 4 拍（收口片 X） | ❌ 未开始 | 需在前三拍之后 |

⇒ **T1 族：2/4 拍完成，尚未闭合。**

### 6.2 电池结果（如实记录）

见 §6.4（电池在本片收尾时整轮重跑）。

### 6.3 未做的事

- **只认领一个形状**（两个 `Text` 形参、唯一语句、return 位置）⇒ T1 的其余形状与 S 内 2,882 个调用点**仍未覆盖**；
- **组件层未实现**（见 §5.1）；
- **未提交、未推送**；**独立 CI 尚未运行**；
- **不推进任何出口门**，M22 仍为 NO-GO。

### 6.4 本机验证电池（11 步）

本轮电池 **11/11 全绿**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 426 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试 ⇒ **固件 129 条在本步已跑过**） |
| `validate-0245` | 135 | 0 | 内含固件全量 + clippy（`-D warnings` 零警告） |
| `validate-0262` | 149 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 186 | 0 | 前端前沿 **28/28** ⇒ `FRONTIER_UNCHANGED`（含本片翻转的 1 条与新增的 2 条 T1 用例） |
| `probe-ir-plan-frontier` | 25 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 0 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN`（**25** 个文件，含本片新增的 STEP-0518 文档） |

⇒ `BATTERY_GREEN 11/11`。
