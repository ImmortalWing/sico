# STEP-0504 m22-w2 S5 派生组件：用 IR 派生的 run 体替换冻结常量 ⇒ 参考形态整组件逐字节等同

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0503](./STEP-0503-m22-w2-s5-run-body-result-return.md)（整段 run 体）、[STEP-0502](./STEP-0502-m22-w2-s5-run-instruction-segment.md)（指令段）、[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位/局部声明计划）、[STEP-0487](./STEP-0487-m22-w2-s5-codegen-skeleton.md)（首个组件发射，冻结骨架）、[STEP-0501](./STEP-0501-m22-w2-s5-run-body-truth.md)（run 体真值）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是发射侧的第四刀（整组件）**
> - numbering: M22 线顺延为 **STEP-0504**（本线已占用 STEP-0491..0499、STEP-0501..0503；并行 M23 会话占用 STEP-0489/0490/0498/0500；取号时已核对 `docs/steps/`、全部分支与 `E:\github` 递归，**均无 0504**；`E:\github\sico-m24` 工作树不存在）

## 1. 本片目标与结论

**目标**：把 STEP-0487 留在 guest 里的**冻结 run 体常量**换成**从摄取的 IR 现算**的 run 体，使 guest 发射出的**整个组件**与 Rust backend 的生产输出**逐字节相同**。

**结论（实测）**：已闭合。

- guest 源码里**再也不存在**那条冻结 run 体常量（`4100211702400340…` 在 `selfhost/codegen.sico` 中**已搜不到**；仅存的同名 hex 在 `tests/wasm/artifacts.hex` 与测试文件里，那是 **Rust backend 的真值**，不是 guest 的存货）；
- guest 对**两个**参考形态发射的组件**都**与 oracle `compile_script_program` 的**生产输出逐字节相同**，并通过 `wasmparser::Validator` 的真实验证；
- 两个组件**必须不同**（否则就是回放冻结常量）：实测**只差 1 个字节**，且那 1 个字节正是 `exit_code` 常量的 `i64.const` 立即数；
- **fail-closed**：子集外的形状仍是 typed 拒绝 `unsupported codegen source shape`（既有契约未放宽）。

**门 3 仍为 NO-GO**：本片只做到**参考形态的组件发射**这一层；ADR-0015 的 `A == B == C` 需要**三段 runner 执行**（A = 进程内编译、B = 执行 A、C = 执行 B），本片只提供了「guest 发射的组件 == backend 组件」这一条**内容腿**；`.sapp` 链、37 语料 / formatter / 全源 canary 阶梯全部未做。

## 2. 切点：STEP-0503 §9 预切的位置

STEP-0487 的 `--emit-component-hex` 走的是**冻结骨架**路线：guest 内部有一条**写死的** run 体常量

```
15  <局部声明段 42 B>  <run 体 code 180 B>
```

其中 `15` 是局部声明**组数 21** 的 LEB128。三段**此刻都已经能 IR 派生**了：

| 段 | 派生者 | 来自 |
|---|---|---|
| 组数 | `sico.text.length(局部声明 hex) shr 2` | STEP-0499 |
| 局部声明段 | `cgs_local_hex(ir)` | STEP-0499 |
| run 体 code | `cgs_run_body_hex(ir)` | STEP-0502 + STEP-0503 |

⇒ 本刀 = 把这条常量换成 `cg_leb_hex(组数) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`，**顺序不变、字节不变**，但**来源从「写死」变成「现算」**。

## 3. 组件的字节解剖（实测，从 guest 实际产出的组件解析）

guest 发出的组件 = **981 B**（1962 hex）。按 Component 二进制格式逐段拆开：

| 段 | id | 长度 | IR 相关？ |
|---|---|---|---|
| 头部 `0061736d0d000100` | — | 8 B | 冻结 |
| Type | 7 | 268 B | 冻结 |
| Import | 10 | 28 B | 冻结 |
| Alias | 6 | 34 B | 冻结 |
| Type | 7 | 12 B | 冻结 |
| **CoreModule** | 1 | **530 B** | **部分是** |
| CoreInstance | 2 | 4 B | 冻结 |
| Alias | 6 | 55 B | 冻结 |
| CanonicalFunction | 8 | 13 B | 冻结 |
| Export | 11 | 9 B | 冻结 |

核心模块（530 B）再拆：

| 段 | id | 长度 |
|---|---|---|
| Type | 1 | 26 B |
| Function | 3 | 5 B |
| Memory | 5 | 8 B |
| Global | 6 | 20 B |
| Export | 7 | 47 B |
| **Code** | 10 | **403 B（4 个函数）** |
| ~~Data~~ | 11 | **不存在** |

Code section 的 4 个函数：

| 函数 | 大小 | 来源 |
|---|---|---|
| `run`（索引 0） | **223 B** | **IR 派生**（STEP-0504） |
| `alloc`（索引 1） | 84 B | 冻结（`cg_alloc_body_hex`） |
| `realloc`（索引 2） | 72 B | 冻结（`cg_realloc_body_hex`） |
| `post`（索引 3） | 18 B | 冻结（`cg_post_body_hex`） |

`run` 的 223 B = **1 B 组数 LEB**（`15` = 21 组）+ **42 B 局部声明** + **180 B code**。

**与 STEP-0501 的测量互证**：参考形态**无 data 段**（段号集合 {1,3,5,6,7,10}，与 STEP-0501 §3.2 记录的一致）⇒ **run 体是组件里唯一的 IR 相关部分**。这正是 STEP-0501 §3.2 「只要 run 体能 IR 派生且逐字节等同，整个组件就能逐字节等同」那句话成立的前提，本片把它兑现了。

**IR 相关字节占比**：223 B / 981 B ≈ **22.7%**。其余 758 B 仍是 guest 携带的**冻结组件骨架**（见 §8 诚实收窄）。

## 4. 证明「发射跟着 IR 走，而不是回放常量」

**方法**：造第二个参考形态 `ECHO_COMPONENT_FIXTURE_SOURCE_EXIT7` —— 与参考形态**只差一个常量**（`I64.literal(0)` → `I64.literal(7)`）。backend 对它会发射**不同的**组件；一个仍在回放冻结常量的 guest 必然发出**完全相同**的字节。

**实测**：

| 变体 | 组件长度 | 与 oracle 生产输出 |
|---|---|---|
| exit 0 | 1962 hex = 981 B | **逐字节相同** |
| exit 7 | 1962 hex = 981 B | **逐字节相同** |

两者**只差 1 个字节**：

| 位置 | exit 0 | exit 7 | 含义 |
|---|---|---|---|
| 组件字节 **567**（= 核心模块字节 205 = run 体偏移 75 = run 体 code 偏移 **32**） | `00` | `07` | `42`（`i64.const` 操作码）后的 **SLEB 立即数**，即 `exit_code` 常量 |

上下文（run 体 code 偏移 24..32）：`02 21 06 20 03 21 07 42 00` ⇒ 差的那一位紧跟在 `i64.const` 的操作码 `42` 之后，**正是源码常量落地的位置**。

⇒ 「组件发射随 IR 变化」不是断言，是**字节级的差分实测**。

## 5. guest 侧实现

### 5.1 `selfhost/codegen.sico`（2624 → 2623 行）

| 改动 | 前 | 后 |
|---|---|---|
| run 体来源 | `cg_run_body_hex_minimal()` 返回**冻结常量** | **删除** |
| run 体派生 | — | `cg_run_body_hex_from_ir(ir)` = `cg_leb_hex(组数) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`；任一段返回 `ERR:` ⇒ 返回 `""`（fail-closed） |
| 核心模块 | `cg_core_module_hex()` | `cg_core_module_hex(ir)`，**新增「IR 只能有一个函数」守卫** |
| 组件 | `cg_component_hex()` | `cg_component_hex(ir)`（核心模块为空 ⇒ 组件为空） |
| 入口 | `component_hex_for_minimal(source)`（**逐字节比对**冻结 fixture 源码） | `component_hex_for_ir(ir)` |
| 死代码 | `cg_minimal_fixture_source()` | **删除**（其唯一用途是那条逐字节比对，随之失效） |

**「IR 只能有一个函数」守卫的由来**：guest 携带的**函数段是冻结的**（`run` / `alloc` / `realloc` / `post` 四个）。backend 遇到第二个源函数时会**多发一个核心函数**（函数段与 code section 都要变），而 guest 的冻结函数段**装不下**它 ⇒ 若不拒绝，guest 会**静默丢掉**那个函数，产出一个「看起来合法、语义错误」的组件。⇒ 守卫读 `cgj_array_item(ir, functions, 1)`，非 0 即返回 `""`（见 §6）。

### 5.2 `selfhost/compiler.sico`（977 → 982 行）

`--emit-component-hex` 模式从「拿源码逐字节比对冻结 fixture」改为「**词法 → IR → 发射**」：

```
parser.lex_words(input.stdin)  →  parser.scalar_ir(...)  →  codegen.component_hex_for_ir(ir)
```

- IR 解析失败 ⇒ typed 拒绝 `unsupported codegen source shape`；
- 发射返回空串 ⇒ 同样 typed 拒绝（**既有 typed 契约未放宽**，与 STEP-0487 相同的消息与身份）。

### 5.3 踩到并修正的**两条自造缺陷**

| # | 现象 | 根因 | 修法 |
|---|---|---|---|
| 1 | `selfhost/codegen.sico` 的**工作副本整文件是 CRLF**（2623 CRLF / 0 LF），而仓库其余文件（含 HEAD 里的本文件）都是纯 LF | 上一轮改写该文件时用了会写 CRLF 的写入方式。`.gitattributes` 的 `* text=auto eol=lf` 会让 git 在**入库时**归一化成 LF ⇒ **提交内容不受影响**，但工作副本与提交树不一致 ⇒ 本机证据跑的字节 ≠ CI 检出的字节 | 字节级 CRLF→LF 转换（内容零改动，105,860 → 103,237 B），并**重跑整轮验证电池**，使证据与提交树一致 |
| 2 | 电池脚本的**行尾审计步骤**实测检出上述问题并返回 1，但汇总却打印 `BATTERY_GREEN` | `Run-Step` 用 `$LASTEXITCODE` 取退出码，而该步骤的**最后一个表达式**是 PowerShell 变量 `$bad`（值为 1）——`$LASTEXITCODE` 仍是**上一个原生命令**的 0 ⇒ 失败被吞掉 | 把行尾审计拆成独立脚本 `audit-lines.ps1`，用 `powershell -File` 调用 ⇒ 其退出码正确进入 `$LASTEXITCODE` |

第 2 条与 STEP-0497 记录的「本地绿灯可能是空跑」同族：**验证器自身的失败必须能被验证器之外的机制看见**。本片把它当作验证基础设施缺陷修掉，而不是只修被它掩盖的那一个症状。

## 6. fail-closed 的三条边界（ADR-0016 决策 6）

| 边界 | 触发 | 结果 |
|---|---|---|
| IR 里有第二个函数 | `cgj_array_item(ir, functions, 1) != 0` | typed 拒绝（**不让 backend 多出的函数被静默丢弃**） |
| IR 里没有 `run` / `run` 的形状超出发射子集 | `cgs_run_body_hex(ir)` 返回 `ERR:` | typed 拒绝 |
| 源码根本不是 Script-profile 入口 | IR 解析/计划失败 | typed 拒绝 |

新固件钉 `sico_codegen_guest_refuses_the_component_emission_outside_the_subset` 对前两类各取一个实例（`IDENTITY_SOURCE`；echo fixture + 一个未被调用的额外函数）。

## 7. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 派生组件逐字节等同（两个变体） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly` | **ok** |
| 组件通过真实验证器 | 同上（`wasmparser::Validator::validate_all`） | **ok** |
| 两个变体必须不同 | 同上（`assert_ne!`） | **ok** |
| 子集外 typed 拒绝 | 同上，`sico_codegen_guest_refuses_the_component_emission_outside_the_subset` | **ok** |
| STEP-0487 的既有组件钉仍绿 | `sico_codegen_guest_emits_component_bytes_byte_exactly` | **ok** |
| 冻结常量已从 guest 源码消失 | 全仓搜索 `4100211702400340` / `cg_run_body_hex_minimal` | `selfhost/codegen.sico` **零命中**（仅剩 `tests/wasm/artifacts.hex` 与测试文件里的 **backend 真值**） |
| 固件套件 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds -- --test-threads=1` | **9 + 103 + 1 + 2 = 115 passed / 0 failed**（selfhost_compiler 由 101 增至 103，+2 即本片两枚新钉）；**在 LF 修正后的树上复测同值** |

> **验证电池**（`target/tmp/step0504-battery.ps1`，一键 CI 放在最前、机器最空闲时跑）见 §7.1。

## 7.1 本机验证电池

`target/tmp/step0504-battery.ps1`（一键 CI 放在最前、机器最空闲时跑）：

| 步骤 | 耗时 | 结果 |
|---|---|---|
| `fmt (runner)` | 00:00:04 | PASS |
| one-key CI（`tools/run-ci.ps1`，**12 步**） | 00:05:08 | **CI GREEN** |
| M22 regression（0245） | 00:00:56 | PASS |
| M22 checker（0262） | 00:02:17 | PASS（`STEP_0262_OK`） |
| 前端前沿探针 | 00:02:08 | **18/18** `FRONTIER_UNCHANGED` |
| IR 计划层前沿探针 | 00:00:25 | **8/8** `PLAN_FRONTIER_UNCHANGED` |
| 规划合同（0124） | 00:00:00 | PASS（`STEP_0124_OK`） |
| 文档校验器（0131） | 00:00:01 | PASS |
| 文档校验器（0156） | 00:00:00 | PASS |
| 空白检查（`git diff --check`） | 00:00:00 | PASS |
| 行尾审计（纯 LF、零 CRLF、零 BOM，10 文件） | 00:00:01 | PASS（`LINE_ENDING_AUDIT_GREEN`） |

⇒ **11/11 BATTERY_GREEN**。

> **本轮是「修掉 CRLF + 修掉退出码管道」之后重跑的**。第一遍电池的**每个步骤都打印 PASS**，但行尾审计**实测检出**了 `selfhost/codegen.sico` 的 CRLF，而失败被退出码管道吞掉 ⇒ 第一遍那个 `BATTERY_GREEN` **不成立**。修复后整轮重跑，本机证据与**提交树（LF 形式）**一致。

## 7.2 独立 CI 裁决：**GO**（run 39585258425 = `success`）

本片已**提交并推送**（owner 授权）：提交 `579645b`，推送 `github/codex/m22-w1-ci`（fast-forward `7cf2ece..579645b`），远端 head 确认 = `579645b6d2096a1d31b0514d441aff380054da62`。

| 证据 | 值 |
|---|---|
| 独立 CI run | **39585258425**（run #26，`event: push`，head_branch `codex/m22-w1-ci`） |
| head_sha | `579645b6d2096a1d31b0514d441aff380054da62`（= 本片提交） |
| 结论 | `status: completed` / **`conclusion: success`** |
| 时间 | 10:31:54Z → 10:59:49Z（job `windows-gnu`） |
| job 内步骤 | `Set up job` / `checkout` / `Install GNU build tools` / **`Run repository CI on a fresh runner`** / `Post Run checkout` / `Complete job` —— **6/6 全部 success** |
| 内层步数 | **12/12** 全 PASS（`fmt` / `fmt (runner)` / `clippy (workspace)` / `clippy (runner)` / `build (runner debug)` / `test (workspace)` / `test (runner)` / `module boundaries` / `planning contract (M14-M25)` / `application-profile matrix` / `cross-host matrix + UI corpus` / `whitespace (git diff --check)`） |

**本片最终状态**：**complete（独立 CI GO）**。

**门 3 仍为 NO-GO**：独立 CI 证明的是**本片**在独立 runner 上成立，不是门 3 成立。ADR-0015 的三段 runner 执行、37 语料阶梯、formatter 与全源 canary、`.sapp` 链都还没做。

## 8. 诚实性

- **本片只覆盖参考形态**。oracle 的 `compile_script_program` 只接受 Script v0 入口签名，其它形态根本不发射组件 ⇒ 真值只覆盖这一种形态。
- **组件里仍有 758 B 是冻结骨架**（§3）。guest 现在是「**冻结组件骨架 + IR 派生的 run 体**」，**不是**通用组件发射器。类型段、导入段、别名段、导出段、`alloc`/`realloc`/`post` 三个 helper 体都还是写死的。
- **支持子集**（超出即 typed 拒绝）：IR 里**只有一个函数**；该函数是 Script-profile `run`；`run` 单块、终止符是 `return` 一个值；该值由 `variant ok` 产生；载荷由 `construct` 产生；字段恰好 3 个且与冻结的 `ScriptOutput` 记录逐位一致。
- **`error` 臂仍无实测真值**（承自 STEP-0503）：参考形态不触发它 ⇒ `ScriptError` 的结果区偏移没有真值，guest 仍然拒绝该形态，**不拿源码推导冒充支持**。
- **ADR-0015 的 `A == B == C` 没有被本片闭合**。ADR-0015 定义的是**三段 runner 执行**（A = 进程内编译、B = 执行 A、C = 执行 B）且要求三者**逐字节等同**并各自通过 Component 验证。本片提供的是「guest 现算出的组件 == backend 组件」这一条**内容腿**，只对参考形态成立；三段执行、证据清单、`.sapp` 链、预算复测全部未做。
- **组件级冻结事实与 IR 派生事实的分界不变**（承自 STEP-0501/0503）：本片把 **run 体**整体从「冻结」挪到「派生」，**没有**扩展冻结事实的覆盖面，也没有新增任何「由源码推导但无实测」的偏移。
- 本片**没有**触碰 37 语料阶梯、formatter、全源 canary 或 `.sapp` 链。**门 3 仍为 NO-GO**。

## 9. 下一片

**owner 已拍板（2026-10-21）：STEP-0505 = 把组件发射扩到非参考形态，先取 `error` 臂的真值。**

1. **`error` 臂取真值（STEP-0505 的第一刀）**：写一个返回 `error(...)` 的 Script-profile fixture，让 oracle **真正发射**，把 `ScriptError` 的结果区偏移从「冻结推导」升级为「**实测**」。这是三条候选里**唯一「只要写一个 fixture 就有真值」**的——因为 Script profile 的 `run` 返回 `Result[ScriptOutput, ScriptError]`，一个返回 `error(...)` 的 `run` **是合法入口**，backend 会照发。
2. **多函数 / data 段模块**：需要先把 guest 的**函数段**（函数个数）与 **data 段**做成派生的，比第 1 条难，放到 `error` 臂之后。
3. 然后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。
4. （**可选**）仿 `tools/probe-m22-ir-plan-frontier.ps1` 加一个 `--emit-component-hex` 的**前沿探针**，把「组件可发射 / typed 拒绝」的边界整张表钉下来。

> 提醒（承接 [handoff-m22.md](../handoff-m22.md) §4/§5）：guest 前端的**形状前沿**与**语法/语义纪律**共同构成「新代码必须能被 guest 自己降低」的全部已知约束；本片新增的代码刻意保持**顶层浅嵌套**，未触碰既有前沿（全源 census 仍绿，见验证电池）。
