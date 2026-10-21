# STEP-0503 m22-w2 S5 run 体结果区返回：guest 发射的整段 run 体与 Rust backend 实际发射逐字节相同

> - status: **complete（独立 CI 已裁决：run 39572167889 = `success`，12/12 内层步全 PASS）**
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0502](./STEP-0502-m22-w2-s5-run-instruction-segment.md)（指令段发射）、[STEP-0501](./STEP-0501-m22-w2-s5-run-body-truth.md)（run 体真值与规则表）、[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位/局部声明计划）、[STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md)（IR 摄取）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是发射侧的第三刀（结果区返回 + dispatcher 尾声）**
> - numbering: M22 线顺延为 **STEP-0503**（本线已占用 STEP-0491..0499、STEP-0501、STEP-0502；并行 M23 会话占用 STEP-0489/0490/0498/0500；取号时已核对 `docs/steps/` 与本机其它工作树**均无 0503**）

## 1. 本片目标与结论

**目标**：让 guest 从**摄取的 IR** 推出 `run` 体的 **Script 结果区返回 + dispatcher 尾声**（STEP-0501 §3.3 的 178 hex / 89 B），并与 Rust backend **实际发射**的同一段 **逐字节相同**。

**结论（实测）**：已闭合，且**顺带把整段 `run` 体打通了**。

- guest 新增 **24 个函数**（`selfhost/codegen.sico`）+ 新驱动模式 **`--emit-run-body-hex`**（`selfhost/compiler.sico`）；
- guest 推出的**整段 `run` 体 = 360 hex（180 B）**，与 Rust backend 实际发射的 `run` 体 **逐字节相同**（新固件钉 `sico_compiler_emits_the_run_body_byte_exactly`，对照读的是 oracle 的**生产输出**）；
- 其中本片新增的**结果区返回 + dispatcher 尾声 = 178 hex（89 B）**单独取出来也对拍通过（`oracle_run_result_segment_hex`）；
- **fail-closed**：子集外的形状一律 typed 拒绝（新固件钉 `sico_compiler_refuses_the_run_body_outside_the_subset`）；
- **自举形状前沿未破**：全源 census 仍绿，guest 能降低自己新增的源码。

**门 3 仍为 NO-GO**：本片只覆盖**参考形态**的 `run` 体；37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。整组件逐字节等同是 **STEP-0504** 的事（用派生 run 体替换 STEP-0487 的冻结 run 体 hex）。

## 2. 为什么这一刀能独立成刀

STEP-0501 §4.4 把 run 体尾段的依赖拆成了两类，本片正好各取一半：

| 类别 | 内容 | 本片的处理 |
|---|---|---|
| **组件级冻结事实**（不由 IR 派生） | 分配器函数索引 `1`、结果区大小 `32`、载荷起始偏移 `8`、`ScriptOutput` 的字段字节偏移、`ScriptError` 的字段字节偏移 | guest **携带**（像 STEP-0487 携带冻结的核心模块段一样） |
| **IR 派生事实** | 判别值在哪个槽、载荷各字段在哪个槽、结果区与拷贝目的地在哪个槽 | guest **从 IR 重新算**，并与冻结事实**交叉校验** |

⇒ 尾段可以在 STEP-0504（整组件）之前独立完成与对拍。

## 3. 逐字节规则：89 字节尾段

以下是把 89 字节按 wasm 操作码逐条解出来的结果（每条都对着 oracle 源码核对过）：

| 偏移 | 字节 | 指令 | 规则来源 |
|---|---|---|---|
| 0 | `41 20` | `i32.const 32` | 冻结的**结果区大小** |
| 2 | `10 01` | `call 1` | 冻结的**分配器函数索引**（核心模块函数序 0=run / 1=alloc / 2=realloc / 3=post） |
| 4 | `21 18` | `local.set 24` | `return_area` = dispatcher 槽 + 1（scratch 对的第一格） |
| 6 | `20 18` | `local.get 24` | |
| 8 | `20 0e` | `local.get 14` | **判别值槽** = 变体值的首槽（IR 派生） |
| 10 | `3a 00 00` | `i32.store8 align=0 offset=0` | 判别值写到结果区偏移 0 |
| 13 | `20 10` | `local.get 16` | `stdout` 的**长度**槽（IR 派生） |
| 15 | `10 01` | `call 1` | 向分配器要一份拷贝缓冲 |
| 17 | `21 19` | `local.set 25` | `copy_destination` = dispatcher 槽 + 2 |
| 19–24 | `20 19 20 0f 20 10` | `local.get 25/15/16` | 拷贝目的地、`stdout` 的**指针**槽、长度槽 |
| 25–28 | `fc 0a 00 00` | `memory.copy` | 有界拷贝（源与目的同在 0 号内存） |
| 28–35 | `20 18 20 19 36 02 08` | `local.get 24/25` + `i32.store align=2 offset=8` | `stdout` 指针 → 结果区 **8** |
| 36–42 | `20 18 20 10 36 02 0c` | `local.get 24/16` + `i32.store align=2 offset=12` | `stdout` 长度 → 结果区 **12** |
| 43–54 | `20 12 10 01 21 19 20 19 20 11 20 12` | 取 `stderr` 长度 → 分配 → 拷贝目的地 / 指针 / 长度 | 同上（`stderr`） |
| 55–58 | `fc 0a 00 00` | `memory.copy` | |
| 59–65 | `20 18 20 19 36 02 10` | `i32.store align=2 offset=16` | `stderr` 指针 → 结果区 **16** |
| 66–72 | `20 18 20 12 36 02 14` | `i32.store align=2 offset=20` | `stderr` 长度 → 结果区 **20** |
| 73–79 | `20 18 20 13 37 03 18` | `i64.store align=3 offset=24` | `exit_code` → 结果区 **24** |
| 80–82 | `20 18 0f` | `local.get 24` + `return` | 返回结果区指针 |
| 83–88 | `0b 00 0b 0b 00 0b` | `end`(if) `unreachable` `end`(loop) `end`(block) `unreachable` `end`(fn) | **dispatcher 尾声**（6 字节） |

> **注意 memarg 的长度**：`i32.store8` 的 memarg 是 **align + offset 两个 LEB**，所以「写判别值」是 `3a 00 00`（3 字节），不是 4 字节。STEP-0501 §4.3 的原表在这一点上写得含糊，本片解码时纠正为 3 字节（见 §6）。

### 3.1 冻结事实表（guest 携带）

| 事实 | 值 | 在 guest 里的位置 |
|---|---|---|
| 结果区大小 | `32` | `cgs_frozen_result_area_size()` |
| 分配器函数索引 | `1` | `cgs_frozen_alloc_index()` / `cgs_frozen_alloc_call_hex()` |
| `ScriptOutput` 字段名与顺序 | `stdout` / `stderr` / `exit_code` | `cgs_abi_field_name(index)` |
| `ScriptOutput` 字段的 flat 种类 | `list` / `list` / `i64` | `cgs_abi_field_kind(index)` |
| `ScriptOutput` 字段的结果区偏移 | `8` / `16` / `24` | `cgs_abi_field_area(index)` |
| `ScriptOutput` 列表字段的长度偏移 | `12` / `20` | `cgs_abi_field_area_len(index)` |
| `ScriptOutput` 字段在 flat 记录里的槽偏移 | `0` / `2` / `4` | `cgs_abi_field_slot_offset(index)` |

这些偏移是**冻结的 Script WIT 布局**（`record_layout` 从 WIT 类型算出）**叠加**载荷起始偏移 `8` 之后的**结果区**偏移；它们**不随模块源码变化**，因此 guest 可以携带（与 STEP-0501 §4.4「冻结」的含义一致）。

### 3.2 IR 派生的部分（guest 重新算）

| 量 | 怎么算 | 参考形态实测 |
|---|---|---|
| 判别值槽 | 变体值的首槽 = `cgs_value_slot(返回值) - 1` | 14 |
| 载荷基槽 | 判别值槽 + 1 | 15 |
| `stdout` 指针/长度槽 | 载荷基槽 + 0 / + 1 | 15 / 16 |
| `stderr` 指针/长度槽 | 载荷基槽 + 2 / + 3 | 17 / 18 |
| `exit_code` 槽 | 载荷基槽 + 4 | 19 |
| `return_area` / `copy_destination` | dispatcher 槽 + 1 / + 2 | 24 / 25 |

**交叉校验（fail-closed）**：guest 用「**构造点的字段顺序与 flat 种类**」算槽位，oracle 用「**推断布局的字段表**」算槽位。二者在参考形态上相等；本片在每个字段上**显式校验**（字段名必须与冻结顺序逐位一致、flat 种类必须与冻结种类一致、槽数必须相等），任一不符即 typed 拒绝，**不静默取一侧**。

## 4. guest 侧实现（`selfhost/codegen.sico`，24 个新函数）

| 组 | 函数 | 职责 |
|---|---|---|
| 冻结事实 | `cgs_abi_field_name` / `cgs_abi_field_kind` / `cgs_abi_field_area` / `cgs_abi_field_area_len` / `cgs_abi_field_slot_offset` / `cgs_frozen_result_area_size` / `cgs_frozen_alloc_index` / `cgs_frozen_alloc_call_hex` | 携带 §3.1 的冻结事实 |
| IR 导航 | `cgs_run_return_value` / `cgs_instruction_for_value` / `cgs_op_name` / `cgs_op_data` / `cgs_variant_arm` / `cgs_variant_payload` / `cgs_construct_field_name` / `cgs_construct_field_value` / `cgs_construct_has_field` | 从摄取的 IR 里找出「返回的值 → 变体 → 臂 → 载荷 → 构造点 → 字段」这条链 |
| 交叉校验 | `cgs_abi_field_matches` | 字段的 flat 种类/槽数必须与冻结事实一致 |
| 发射 | `cgs_result_list_hex` / `cgs_result_scalar_hex` / `cgs_result_field_hex` / `cgs_result_fields_hex` / `cgs_result_return_hex` / `cgs_run_body_hex` | 结果区返回与整段 run 体 |

**驱动模式**：`--emit-run-body-hex`（`selfhost/compiler.sico`），与 `--emit-run-instr-hex` 同规：读 canonical IR ⇒ 调 `codegen.cgs_run_body_hex` ⇒ 以 hex 文本输出；typed 身份以 `ScriptError(InvalidInput, "<identity>")` 抛出。

**两个 typed 身份**（沿用 STEP-0502 的身份集，不新增）：

| 身份 | 触发 |
|---|---|
| `ERR:E-SH-CG-IR-PLAN` | IR 里没有 `run` |
| `ERR:E-SH-CG-IR-EMIT` | 返回值不是 `ok` 臂的 `ScriptOutput` 构造、多于一个块、字段名/种类/槽数与冻结事实不符等 |

## 5. 踩到并修正的**一条自造缺陷**

本片只撞到**一条**约束（此前 STEP-0496/0499/0502 已经把 guest 前端的前沿摸清了），但撞到**一条自造缺陷**，且它被字节对拍当场抓出：

| # | 现象 | 根因 | 修法 |
|---|---|---|---|
| 1 | guest 输出的尾段比真值**每处少 1 个字节**（`4120` 后面直接是 `01`，少了 `call` 的操作码 `10`） | 我把 `call <index>` 写成了「**立即数的 LEB**」而不是「**操作码 + 立即数**」：`cg_leb_hex(1)` = `"01"`，而 `call 1` 必须是 `"10" + "01"` | 抽出 `cgs_frozen_alloc_call_hex()` 统一生成 `"10" + cg_leb_hex(index)`；`local.get`/`local.set`/`i32.const` 本来就自带操作码前缀（`cgs_local_get_hex` 等），只有 `call` 没有对应的助手函数 |

**留下的纪律**：凡是「操作码 + 立即数」形式的指令，**立即数助手函数不能单独使用**；`cg_leb_hex` 是**裸立即数**编码器，不是指令编码器。（`cgs_local_get_hex` = `"20" + cg_leb_hex(slot)`、`cgs_i32_const_hex` = `"41" + cgs_sleb_hex(v)` 都是对的样子；`call` 之前缺一个同形的助手。）

## 6. 顺带纠正的**一处文档偏差**

STEP-0501 §4.3 的逐字节表把 dispatcher 尾声写成了 7 项（`end`/`unreachable`/`end`/`unreachable`/`end`/`unreachable`/`end`），实测只有 **6 字节**（`0b 00 0b 0b 00 0b`）。同一张表里「写判别值」写的是 `3a 00 00`（3 字节）——这与 memarg 的真实编码一致，但**没有点明 memarg 是 align + offset 两个 LEB**，正是这一点让我第一次把 `3a 00 00` 读成了 4 字节。两处已在 STEP-0501 的记录里改正（**真值 hex 本身未变**，它一直是对的）。

## 7. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 整段 run 体逐字节等同（生产输出对拍） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_emits_the_run_body_byte_exactly` | **ok** |
| 尾段 178 hex 单独对拍 | 同上（`oracle_run_result_segment_hex` 读 oracle 生产输出） | **ok**（178 hex 固化断言） |
| 子集外形状 fail-closed | 同上，`sico_compiler_refuses_the_run_body_outside_the_subset` | **ok**（`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-EMIT`） |
| guest 直跑取证（不经测试框架） | `sico build --profile script-v0` 出组件 → `sico-runner --fuel 800000000000 <component> -- --emit-run-body-hex`（stdin 投 Script-profile echo fixture） | exit 0，stdout **360 hex**，与 oracle 整段 run 体逐字节相同 |
| 固件套件 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **9 + 101 + 1 + 2 = 113 passed / 0 failed**（selfhost_compiler 由 99 增至 101，+2 即本片的两枚新钉） |
| M22 回归 / checker | `tools/validate-step-0245.ps1` / `tools/validate-step-0262.ps1` | exit 0 / `STEP_0262_OK` |
| 前端前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | **18/18** `FRONTIER_UNCHANGED` |
| IR 计划层前沿探针 | `tools/probe-m22-ir-plan-frontier.ps1` | **8/8** `PLAN_FRONTIER_UNCHANGED` |
| 规划合同 | `tools/validate-step-0124.ps1` | `STEP_0124_OK` |
| 一键 CI | `tools/run-ci.ps1` | **12/12** 全 PASS（CI GREEN） |
| 行尾审计 | 9 个改动文件逐字节扫描 | **纯 LF、零 CRLF、零 BOM**；`git diff --check` exit 0 |

**本机验证电池**（`target/tmp/step0503-battery.ps1`，一键 CI 放在最前、机器最空闲时跑）：`fmt (runner)` 00:02 / `one-key CI` 04:28 / `M22 regression (0245)` 00:58 / `M22 checker (0262)` 02:50 / `frontend frontier probe` 02:15 / `ir-plan frontier probe` 00:27 / `planning contract (0124)` 00:00 / `whitespace (git diff --check)` 00:00 ⇒ **8/8 PASS，BATTERY_GREEN**。

## 7.1 独立 CI 裁决：**GO**（run 39572167889 = `success`）

本片已**提交并推送**（owner 授权）：提交 `7cf2ece`，推送 `github/codex/m22-w1-ci`（fast-forward `ee71719..7cf2ece`），远端 head 确认 = `7cf2ece982d511f2a39362384eb831be3b394584`。同一次提交还带上了 **STEP-0502 的独立 CI 裁决回填**（owner 偏好一次提交推送、不拆两轮独立 CI）。

| 证据 | 值 |
|---|---|
| 独立 CI run | **39572167889**（run #25，`event: push`，head_branch `codex/m22-w1-ci`） |
| head_sha | `7cf2ece982d511f2a39362384eb831be3b394584`（= 本片提交） |
| 结论 | `status: completed` / **`conclusion: success`** |
| 时间 | 08:49:22Z → 09:11:57Z（job `windows-gnu`） |
| job 内步骤 | `Set up job` / `checkout` / `Install GNU build tools` / **`Run repository CI on a fresh runner`** / `Post Run checkout` / `Complete job` —— **6/6 全部 success** |
| 内层步数 | **12/12** 全 PASS（`fmt` / `fmt (runner)` / `clippy (workspace)` / `clippy (runner)` / `build (runner debug)` / `test (workspace)` / `test (runner)` / `module boundaries` / `planning contract (M14-M25)` / `application-profile matrix` / `cross-host matrix + UI corpus` / `whitespace (git diff --check)`） |

**本片最终状态**：**complete（独立 CI GO）**。

**门 3 仍为 NO-GO**：独立 CI 证明的是**本片**在独立 runner 上成立，不是门 3 成立。整组件逐字节等同、37 语料阶梯、ADR-0015 `A == B == C` + `.sapp` 链都还没做。

## 8. 诚实性

- **本片只覆盖参考形态**。oracle 的 `compile_script_program` 只接受 Script v0 入口签名，其它形态根本不发射组件 ⇒ 真值只覆盖这一种形态。
- **支持子集**（超出即 typed 拒绝 `ERR:E-SH-CG-IR-EMIT`）：
  - `run` 必须是**单块**、终止符必须是 `return` 一个值；
  - 该值必须由一个 **`variant ok`** 产生，且**只带一个**载荷值；
  - 该载荷必须由一个 **`construct`** 产生，字段**恰好 3 个**、顺序与名称必须与冻结的 `ScriptOutput` 记录逐位一致（`stdout` / `stderr` / `exit_code`），每个字段的 flat 种类与槽数必须与冻结事实一致；
  - **`error` 臂一律拒绝**：参考形态不触发它 ⇒ `ScriptError` 的结果区偏移（`code`@8 / `message`@12、16）**没有实测真值**，本片不拿源码推导冒充支持。
- **oracle 的 `has_ok && has_error` 双载荷形态（`i32.eqz` / `if` / `else`）也一律拒绝**：那是被调用方产生的值，参考形态不触发 ⇒ 没有真值。
- **两套布局必须分清**（承自 STEP-0501）：局部变量用 **8 字节 flat 槽**，结果区用 **4 字节 WIT 布局 + 载荷偏移 8**。本片的交叉校验正是把两者绑在一起的地方。
- **guest 携带的冻结事实不是"从 IR 派生的"**：它们随冻结的 Script WIT 与冻结的组件骨架而来，与 STEP-0487 携带冻结核心模块段是同一性质。本片**没有**扩展这些事实的覆盖面。
- 本片**没有**触碰 37 语料阶梯、`.sapp` 链或 ADR-0015。**门 3 仍为 NO-GO**。

## 9. 下一片

1. **STEP-0504 = 用派生 run 体替换 STEP-0487 的冻结 run 体 hex**。切点已经找准：`selfhost/codegen.sico` 的 `cg_run_body_hex_minimal()` 现在返回的是**一条冻结常量** = `15`（局部声明组数 21 的 LEB128）+ **STEP-0499 的局部声明段** + **STEP-0503 的 360 hex run 体**；两段**现在都已经能 IR 派生**（局部声明段由 `cgs_local_hex` 给、run 体由 `cgs_run_body_hex` 给），所以本刀 = 把这条冻结常量换成 `cg_leb_hex(组数) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`。替换后 `cg_core_module_hex()` 输出的**整个核心模块**都变成 IR 派生的 ⇒ 参考形态的**整个组件**逐字节等同（参考形态无 data 段 ⇒ run 体是唯一的 IR 相关部分），门 3 的 `A == B` 在参考形态上即可达。
2. 然后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。
3. （**可选**）给 `error` 臂取真值：写一个返回 `error(...)` 的 Script fixture，让 oracle 真正发射，再把 `ScriptError` 的结果区偏移从「冻结推导」升级为「实测」。
4. （**可选**）仿 `tools/probe-m22-ir-plan-frontier.ps1` 加一个 `--emit-run-body-hex` 的**前沿探针**，把「子集内可发射 / 子集外 typed 拒绝」的边界整张表钉下来。

> 提醒（承接 [handoff-m22.md](../handoff-m22.md) §4/§5）：guest 前端的**形状前沿**与**语法/语义纪律**共同构成「新代码必须能被 guest 自己降低」的全部已知约束；本片新增的 24 个函数刻意保持**顶层浅嵌套**（`while` 体 → `if` → 早返回），未触碰既有前沿。
