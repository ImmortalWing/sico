# STEP-0536 m22-w2 S5 **W2 族 `match` 构造的形参计数门（M-g1）**：354 个真实站点第一次逐站有证据 + 账本扩到全部 396 个 `checked_add` 匹配头

> - 性质：**实现片**（前端刀）。`selfhost/parser.sico` **拆掉两处形参 guard**；**`crates/*` 零改动**；**不推进任何出口门**；**M22 仍为 NO-GO**（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO）；**本族仍未闭合**。
> - 归属：[M22 计划 §3.3.5](./../plans/M22-compiler-self-host.md) 与 owner 审计 `AGENTS.md` §10.5 **R1.2**；承 [STEP-0534](./STEP-0534-m22-w2-s5-w2-match-truth.md)（三门归因）与 [STEP-0535](./STEP-0535-m22-w2-s5-w2-match-emitter.md)（组件层 M-g4a + owner 审计 R0 证据整改）。
> - 产物：**站点账本 v1** `selfhost/match-component-coverage-v1.json`（396 站点，逐站记录真实形参数）+ 普查脚本 `tools/census-m22-match-sites.py`（唯一写者）+ 探针 `tools/probe-m22-match-coverage.ps1` + 前沿探针 `tools/probe-m22-frontend-frontier.ps1` + [支持面登记表](../reports/m22-support-surface-match.md)。

## 0. 入口门槛

- **缺口是真的，不是理论**：STEP-0534 在真实站点 `codegen.sico:cg_next`（1 形参 + `return`/`return`）上复现了 `ERR:E-SH-IR-MATCH-SHAPE`，并测出该门挡住 **354** 个站点。
- **落点已定位到两处**：`selfhost/parser.sico` 的 `checked_arithmetic_set_match_ir`（只允许 1..2 形参）与 `checked_arithmetic_match_ir`（只允许恰好 2）。
- **本片先写失败测试**：固件 `sico_compiler_lowers_the_checked_arithmetic_match_at_any_parameter_count_byte_exactly` 在实现前**先失败**（3 形参形状被前端拒），实现后**逐字节等同**。

## 1. 为什么这道门是「死门」

两处 guard 的结果变量（`casm_arity_one` / `casm_arity_two` / `arity_ok`）**从未被读取**：它们的值被赋值后没有任何一处引用。路由本身早已通过 **binding parameter index** 解析操作数，并且**逐个**把操作数下标与 `parameter_count` 比较（`selfhost/parser.sico` 的 `U64.less_than(<操作数下标>, parameter_count)` 四处）⇒ **形参数从来不是形状约束，只是一道没有依据的门**。

| 门 | 生产后端有没有对应约束 | 结论 |
|---|---|---|
| `checked_arithmetic_set_match_ir` 的 1..2 形参门 | **没有** —— 后端按形参列表建局部布局，任意元数合法 | 死门 |
| `checked_arithmetic_match_ir` 的「恰好 2」形参门 | **没有** —— 同上 | 死门 |

⇒ 本片**只拆门，不改机器**：没有新增函数、没有改既有行的语义、没有动 oracle。

## 2. 实现（`selfhost/parser.sico`，**−14 / +15 行**，`crates/*` 零改动）

- `checked_arithmetic_set_match_ir`：删掉 1..2 形参门（原 `parser.sico:19473-19479`）。
- `checked_arithmetic_match_ir`：删掉「恰好 2」形参门（原 `parser.sico:20364-20368`）。
- 两处各补一段**函数间**注释（guest 的注释纪律：函数体内禁注释；函数间注释不得含 17 个结构扫描词 —— `tools/validate-selfhost-comment-discipline.py` 逐行校验）。

## 3. 实测（固件，逐字节）

`the_result_layout_and_the_checked_arithmetic_boundaries_are_pinned` 之外新增 **`sico_compiler_lowers_the_checked_arithmetic_match_at_any_parameter_count_byte_exactly`**：

| 形参数 | 形状 | 结论 |
|---|---|---|
| 1 | `U64.checked_add(a, a)` | canonical IR **逐字节等同** oracle（**4438 B**） |
| 2 | `U64.checked_add(a, b)` | **4511 B** 逐字节等同 |
| 3 | `U64.checked_add(a, b)` + 第 3 形参未用 | **4585 B** 逐字节等同（**实现前被前端拒**） |
| 4 | 同上 | 逐字节等同 |
| 9 | 同上 | 逐字节等同 |
| 12 | 同上 | **5247 B** 逐字节等同（**实现前被前端拒**） |
| 17 | 同上 | 逐字节等同 |

⇒ **7 个形参数全部逐字节等同**；**值 id、SSA 顺序与 range 一律未变**（逐字节相等即证明）。

**边界仍然 typed 拒（不得静默近似）**：`checked_div` 的 1 形参形状仍被**算子白名单**拒 `ERR:E-SH-IR-CALL-TARGET`（白名单是**另一道**门；实现前它被形参门遮蔽，报的是 `ERR:E-SH-IR-MATCH-SHAPE`）。

## 4. 两处既有拒绝身份变化（改钉 + 写明原因）

拆门后，两处**邻居形状**的报告身份从「形参门」变成「更深的那道门」——**拒绝本身没变，变的是身份更准确了**：

| 钉 | 原身份 | 新身份 | 原因 |
|---|---|---|---|
| `the_t2_g2_s_site_refusals_belong_to_other_families` 的 `one-parameter-nested-match` | `ERR:E-SH-IR-MATCH-SHAPE` | `ERR:E-SH-IR-CALL-ARGUMENT` | 形参门原来**先**拒，遮住了固定算子实参路径的身份；现在与两个 2 形参对照形状**同一个身份** |
| `the_t2_match_head_sites_keep_their_measured_refusal_identities` 的两个真实 S 站点（`hex_text_bytes` / `positive_sleb_length`） | `ERR:E-SH-IR-MATCH-SHAPE` | `ERR:E-SH-IR-CALL-ARGUMENT` | 同上；这两个站点的真实归因**本来就是** T2-g3（算子实参路径），不是匹配路由 |

> **STEP-0533 的预判被本片证实**：当时的钉注释写的是「真实函数只有**一个**形参 ⇒ 形参门先拒；门满足后（两个形参）同一形状被算子实参路径拒（`ERR:E-SH-IR-CALL-ARGUMENT`），那才是 T2-g3 缺口 —— 不是匹配路由」。本片拆门后，两个真实站点**直接**报告那个更深的身份 ⇒ 归因**更准**，且 T2-g3 的缺口**没有**因此闭合。

## 5. 站点账本升到 v1（42 → 396，逐站带真实形参数）

**为什么必须扩**：v0 账本的候选规则是「`U64.checked_add` 头 **且** 宿主函数 ≤2 形参」，规则文字写着「the sites the guest frontend lowers today」—— 拆门之后这句话**不再成立** ⇒ 留在 v0 就是口径漂移。

- **候选规则**改为「`U64.checked_add` 匹配头，**任意形参数**」⇒ **396** 个站点（= 42 + 354，与 STEP-0534 测出的 354 完全对上）。
- **逐站新增 `parameter_count` 字段** ⇒ 账本直接写明每个站点是在**几形参**下测的（owner 计划 R1.2 的「按真实形参计数复核」）；固件逐站复核这个字段与冻结源里的真实签名**一致**，漂移即失败。
- **schema 升到 `sico.m22.match-coverage.v1`**，账本文件升到 `selfhost/match-component-coverage-v1.json`（v0 删除）。

**v1 账本的实测分布（396 站点，95 个不同函数）**：

| verdict | 站点数 | 含义 |
|---|---|---|
| `ir-equal+component-equal` | **9** | 组件字节与生产后端**逐字节等同**（v0 是 8，**多出的 1 个就是 `codegen.sico:cg_next`** ⇒ **M-g1 在真实站点上兑现**） |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **168** | canonical IR 逐字节等同；**发射端**不认识该函数体形状 |
| `closure-too-large` | **219** | 同文件依赖闭包超过 64 函数 / 256 KiB ⇒ IR 证据取自**整文件**差分钉，组件轴未取得 |
| **前端就拒** | **0** | **形参门已拆 ⇒ 396 / 396 站点的 canonical IR 全部与 oracle 逐字节等同**（v0 是 41 / 42） |

⇒ **前端轴**从 41 / 42 推进到 **396 / 396**；**组件轴**从 8 / 42 推进到 **9 / 396**。

## 6. 前沿探针（2 条期望值翻转）

| 探针 | 原期望 | 新期望 | 原因 |
|---|---|---|---|
| `match-checked-add-in-a-three-parameter-function` | `refused:ERR:E-SH-IR-MATCH-SHAPE` | `lowered` | 形参门已拆 |
| `text-length-as-a-match-head-operand-in-a-one-parameter-function` | `refused:ERR:E-SH-IR-MATCH-SHAPE` | `refused:ERR:E-SH-IR-CALL-ARGUMENT` | 形参门不再遮蔽更深的身份 |

## 7. 验证记录（2026-10-24）

### 7.1 固件

- **177 passed / 0 failed**（本片新增 1 条：形参计数门的多形参数逐字节等同）。

### 7.2 前沿探针

- **71 / 71** ⇒ `FRONTIER_UNCHANGED`（2 条期望值翻转，见 §6）。

### 7.3 站点账本探针

- `LEDGER_CURRENT sites=396` + `MATCH_COVERAGE_OK`（pending = 0）。

### 7.4 电池（**两次：第一次 RED，第二次 GREEN**）

版本化电池 `tools/validate-m22-s5.ps1`（**14 步**；每步记录命令、耗时与 exit，任一步失败即整体失败）。日志：`target/tmp/m22-s5-battery/battery.log`（gitignored）。

**第一次（RED）**：

| 步 | 秒 | exit | 命令 |
|---|---|---|---|
| fmt (workspace) | 1 | 0 | `cargo fmt --all -- --check` |
| fmt (runner) | 2 | 0 | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| clippy (workspace) | 1 | 0 | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| clippy (runner) | 1 | 0 | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| **ci-one-key** | **283** | **1** | `powershell -File tools/run-ci.ps1` |
| validate-0245 | 257 | 0 | `powershell -File tools/validate-step-0245.ps1` |
| validate-0262 | 271 | 0 | `powershell -File tools/validate-step-0262.ps1` |
| probe-frontend-frontier | 485 | 0 | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| probe-ir-plan-frontier | 26 | 0 | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| comment-discipline | 0 | 0 | `python tools/validate-selfhost-comment-discipline.py` |
| match-site-census | 0 | 0 | `python tools/census-m22-match-sites.py --check` |
| match-site-coverage | 125 | 0 | `powershell -File tools/probe-m22-match-coverage.ps1` |
| firmware-selfhost | 241 | 0 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| git-diff-check | 0 | 0 | `git diff --check` |

⇒ **BATTERY_RED 1/14** —— 唯一红灯 = `ci-one-key`。**失败子步骤** = `test (runner)`；**失败断言** = `prepared_program_reruns_are_isolated_and_survive_a_trap` 的 **20 ms 预算**（实测 warm median **20.5339 ms**，`runner/sico-runner/tests/runner.rs:1471`）⇒ 那是该断言**自己的注释里就登记过**的环境敏感预算断言（M19 计划 §3.1：「重载下失败、单独跑通过」；参考主机实测 17.4 ms）。

**第二次（GREEN，单实例、无并发）**：

| 步 | 秒 | exit | 命令 |
|---|---|---|---|
| fmt (workspace) | 1 | 0 | `cargo fmt --all -- --check` |
| fmt (runner) | 2 | 0 | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| clippy (workspace) | 1 | 0 | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| clippy (runner) | 1 | 0 | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| ci-one-key | 547 | 0 | `powershell -File tools/run-ci.ps1` |
| validate-0245 | 256 | 0 | `powershell -File tools/validate-step-0245.ps1` |
| validate-0262 | 270 | 0 | `powershell -File tools/validate-step-0262.ps1` |
| probe-frontend-frontier | 483 | 0 | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| probe-ir-plan-frontier | 26 | 0 | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| comment-discipline | 0 | 0 | `python tools/validate-selfhost-comment-discipline.py` |
| match-site-census | 0 | 0 | `python tools/census-m22-match-sites.py --check` |
| match-site-coverage | 125 | 0 | `powershell -File tools/probe-m22-match-coverage.ps1` |
| firmware-selfhost | 241 | 0 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| git-diff-check | 0 | 0 | `git diff --check` |

⇒ **BATTERY_GREEN 14/14**。

**归因（诚实）**：该断言**单独复跑通过**（`1 passed; 0 failed`），整轮**单实例**重跑也通过 ⇒ 归因为**主机负载**，**不是**实现回归（本片改的是 guest 前端的一条路由与测试固件，不触碰 runner 的持久化热重跑路径）。**但必须记下这条因果链**：本片把账本从 42 站点扩到 **396** 站点 ⇒ runner 套件的固件步从 **122 s** 拉长到 **241 s** ⇒ `test (runner)` 之前的机器负载更高 ⇒ 那个 20 ms 预算断言**更容易**在这台主机上翻红。**预算常数不得上调**（M19 计划）⇒ 处置只能是把负载降下来（单实例、不并发跑别的重活）。

> **附带成果（重要）**：第一次红灯的失败子步骤**就是靠 STEP-0535 引入的 GitHub 注解机制定位的** —— `::error title=CI step failed::test (runner)` 与输出末尾若干行，**无需登录凭据**即可读到 ⇒ 这正是 **run #44** 当时缺的能力（本片之前的两次红灯都只能记「子步骤不可得」）。

### 7.5 独立 CI 回填（上一批 `f83ab7b`）= **success ✅** ⇒ R0 exit

| 项 | 值 |
|---|---|
| run | **#45**（id `40073369883`） |
| head_sha | `f83ab7b`（STEP-0535 的 R0.3 / R0.5 收尾提交） |
| 结论 | **success ✅** |
| 用时 | 2026-10-24T07:14:46Z → 07:33:50Z（**19m04s**） |
| GitHub 步骤 | `Set up job` ✅ / `checkout` ✅ / `Install GNU build tools` ✅ / `Run repository CI on a fresh runner` ✅ / `Post checkout` ✅ / `Complete job` ✅ |

⇒ owner 审计 `AGENTS.md` §10.4 的 **R0 五项全部闭合 ⇒ R0 exit 达成**。本片自己的提交触发下一轮独立 CI，**裁决由下一片回填**。

## 8. 诚实性

| 项 | 状态 |
|---|---|
| **`match` 族的闭合** | ❌ **NO-GO** —— 组件级全等 **9 / 396**（合成形状另计 4 个）⇒ **D10 ⇒ 不得降级、不得宣告闭合** |
| **前端三门** | ⏳ **M-g1 已闭合**（本片）；**M-g2（46）/ M-g3（21）** 仍开 |
| **组件层** | ❌ **168** 个站点 canonical IR 已等同但**发射端**拒（分支 / 循环 / 多语句函数体）+ **M-g3'（`sico.list.get`，15 站点）** 仍开 |
| **闭包规模** | ⏔ **219** 个站点的组件轴**没有证据**（依赖闭包 > 64 函数 / 256 KiB）⇒ 这不是「通过」，是**未测** |
| **语料覆盖** | ⏔ **仍 0 条** |
| **整文件自举** | ⏔ 仍未关闭 |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；M22 整体仍 **NO-GO** |

## 9. 下一刀（按实测杠杆）

1. **组件层发射**：**168** 个站点 canonical IR 已逐字节等同，组件轴缺的是**分支 / 循环 / 多语句函数体**的发射能力（M-g4b/M-g4c 之外的第三类）。
2. **M-g3' `sico.list.get` 宿主调用传输**（**15** 站点）：生产后端对它**不是** import 调用，而是**内联线性内存查表序列**。
3. **M-g2 算子白名单**（**46** 站点）：`checked_sub` / `checked_div` / `checked_mul`；`checked_sub` 的发射已具备。
4. **M-g3 未认领的匹配头**（**21** 站点）：`sico.bytes.slice` 头。
5. **闭包规模**（**219** 站点）：要么给比较单元一个更小的切法，要么把闭包上限当成一条待登记的边界。
6. 之后回到 **T3 族** / **语料覆盖** / **门 3 / 6**。

---

**证据等级**：本片是**前端轴**的生产后端对照证据（guest canonical IR ≡ Rust 前端 canonical IR，逐字节，396 / 396 站点）+ **组件轴**的逐站点差分证据（9 / 396 组件字节逐字节等同 + 真实 validator）；**不是**独立 CI 证据（裁决由下一片回填）；**不是**整文件自举证据；**不是**冻结语料证据。
