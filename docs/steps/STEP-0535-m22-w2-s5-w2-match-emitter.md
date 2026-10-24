# STEP-0535 m22-w2 S5 **W2 族 `match` 构造的组件层发射刀（M-g4a）**：4 个形状的组件字节首次端到端逐字节等同 **+ owner 审计 R0 证据整改**

> - status: **本机实测绿**（随本片提交并推送；独立 CI 裁决待下一片回填）
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0534](./STEP-0534-m22-w2-s5-w2-match-truth.md)（519 条 `match` 普查 + 三个拒绝门的归因 + **M-g4 缺口登记**）、[STEP-0533](./STEP-0533-m22-w2-s5-w2-t2-family-closure.md)（族级真值表方法学 + 「族调用点计数不是常量」）、[ADR-0015](../../adr/ADR-0015-compiler-bootstrap-closure-v0.md)（`A == B == C` 字节闭合）、[ADR-0016](../../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) §6（canary 梯级）、**AGENTS.md §10（2026-10-24 owner 审计：R0–R6 整改计划）**
> - gate: **闭合 M-g4 的第一段**（组件层 **0 → 4 个形状端到端字节全等**）+ **R0.1 / R0.2 / R0.4 三项证据整改**；**族仍未闭合** ⇒ **不推进任何出口门**、**不包装为特性支持**
> - numbering: **STEP-0535**（取号前已核对 `docs/steps/`：最大号 0534；*0535 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）。**本片的 R0 部分不另占 STEP 号**（owner 把 R0 定义为「STEP-0535 证据整改」，见 AGENTS.md §10.1.1 与 §10.4）
> - 产物: [支持面登记表](../reports/m22-support-surface-match.md)（本片更新缺口分解与组件层真值）、**站点账本** `selfhost/match-component-coverage-v0.json`（R0.4）、**版本化电池** `tools/validate-m22-s5.ps1`（R0.2）、**站点探针** `tools/probe-m22-match-coverage.ps1`（R0.4）

## 0. 入口门槛

| 入口条件 | 状态 |
|---|---|
| 缺口已被测量并登记 | ✅ STEP-0534：**组件层 0 / 519**（登记为 **M-g4**） |
| 发射形状已冻结 | ✅ 生产后端 `crates/sico-codegen-wasm/src/lib.rs`：`compile_terminator` 的 `Terminator::Match` 分支（`:2295`）、`emit_match_condition`（`:2325`）、`emit_checked_fixed`（`:3010`）、`apply_result_fields`（`:2193`）、`lower_local_types`（`:2924`）——⇒ **不需要新开 ADR**（ADR-0015 的字节闭合 + ADR-0016 §6 的梯级已冻结目标形状） |
| 有界出口测试 | ✅ 「guest 的组件字节 ≡ 生产后端的组件字节」逐字节比对 + `wasmparser::Validator` 通过 + 被拒形状保持 typed 拒绝 |

## 1. **STEP-0534 的归因要订正：组件层不止一个缺口**

STEP-0534 记的是「发射端没有任何 `match` 支持」。本片逐个关掉才发现：**组件层当时有 5 个独立缺口**，其中 4 个与 `match` 本身无关。

| 缺口 | 症状 | 根因（实测） | 本片 |
|---|---|---|---|
| **M-g4a** 终结符 | 发射器只发 `return` / `branch` / `jump` / `unreachable` 四种终结符 | 没有 `match` 的发射路径 | ✅ **闭合** |
| **M-g4b** 匹配头的运算 | 发射器的运算白名单里没有 `U64.checked_add` / `checked_sub` | 匹配头的值必须先能发出来 | ✅ **闭合（U64 的两个变体）** |
| **M-g4c** 分支载荷投影 | arm 体读载荷用的是 `project`，而发射器的 `project` 只认**命名记录** | 没有 Result 分支（`ok` / `error`）的投影 | ✅ **闭合** |
| **M-g4d** Result 的扁平布局 | `Result[U64, NumericError]` 的扁平布局被拒 `ERR:E-SH-CG-IR-FLAT` | 生产后端 `lower_local_types` 对这个形状有**专门分支**（`[i32 tag, i64 ok]`，错误载荷不占槽），guest 不认 | ✅ **闭合** |
| **M-g3'** 宿主调用传输 | `sico.list.get` 头的形状仍被拒 | 发射器没有该 intrinsic 的宿主调用传输 | ❌ **仍开**（15 站点） |

⇒ **订正后的口径**：「组件层零支持」是**五个独立缺口叠加**的结果，**不是**「`match` 构造没实现」。这一条写进登记表，避免下一片按错误口径下刀。

## 2. 实现（`selfhost/codegen.sico` **+363 行**，`crates/*` 零改动）

**新增 7 个 guest 函数**：

| 函数 | 作用 | 对照的生产后端 |
|---|---|---|
| `cgs_match_hex` | `match` 终结符的字节 | `compile_terminator` 的 `Terminator::Match` 分支 |
| `cgs_match_condition_hex` | 一个 arm 的条件（累加器 + 每个模式一项测试 + 每项后一个 `i32.and`） | `emit_match_condition` |
| `cgs_match_pattern_hex` | 一个模式项的测试（变体判别式 / 字面量 / 通配） | `emit_match_condition` 的模式分支 |
| `cgs_match_payload_ok` | 载荷里出现字面量或嵌套变体 ⇒ 拒绝 | `emit_match_condition` 的 `match payload inspection` 拒绝 |
| `cgs_i32_const_neg_one_hex` | 调度器的 `-1` 哨兵 | `Instruction::I32Const(-1)` |
| `cgs_checked_fixed_hex` | `U64.checked_add` / `checked_sub` 的发射 | `emit_checked_fixed` 的无符号分支 |
| `cgs_project_result_hex` | Result 分支（`ok` / `error`）的投影 | `apply_result_fields` + `copy_slots` |

**扩展 4 处既有代码**：`cgs_flat_hex`（认 `Result[固定宽 ok, NumericError]` ⇒ `[i32 tag, i64 ok]`）、`cgs_cell_flat_hex`（cell 的 Result 布局）、`cgs_project_hex`（Result 分支放在命名记录检查**之前**）、`cgs_op_hex`（`checked_add` / `checked_sub` 两个分派）。

**纪律**：新代码**只用 `if` / `while`**，**不使用 `match`** ⇒ 发射器不依赖它自己新增的能力（自举不产生新的循环依赖）。

**fail-closed**：checked 发射写的是「结果第一个槽 = 标签、第二个槽 = 载荷」⇒ guest 与生产后端一样，**结果布局不是恰好 2 个槽就 typed 拒**（`ERR:E-SH-CG-IR-EMIT`），不做近似；变体模式里的**绑定载荷不需要检查**（后端自己也不检查），而**载荷里出现字面量或嵌套变体则拒绝**（后端的 `match payload inspection`）。

## 3. 实测结果（组件层，逐字节）

| 形状 | 前端 IR | **组件字节** | 组件层 |
|---|---|---|---|
| `U64.checked_add` · 2 形参 · `set`/`return` | 4511 B | **1215 B** | ✅ **逐字节等同** |
| `U64.checked_add` · 1 形参 · `set`/`return` | 4438 B | **1214 B** | ✅ **逐字节等同** |
| `U64.checked_add` · 2 形参 · `return`/`return` | 3547 B | **1167 B** | ✅ **逐字节等同** |
| **嵌套 `match`**（`checked_add` 外套 `checked_sub`） | 6342 B | **1389 B** | ✅ **逐字节等同** |
| `sico.list.get(words, i)` · 2 形参 | 3615 B | — | ❌ 仍 typed 拒（**M-g3'**） |

⇒ **组件层从 0 个形状变成 4 个形状端到端逐字节等同**（guest 的组件字节 ≡ 生产后端 `compile_script_program` 的组件字节，且过真实 `wasmparser::Validator`）。

**站点口径**：这 4 个形状**按形状**覆盖了 `U64.checked_add` 头且宿主 ≤2 形参的 42 个 S 站点 ⇒ 但**逐站点复核当时未做**（那是 R0.4 补上的，见 §8）。**R0.4 的实测结论**：真实站点里组件级全等的是 **8 / 42**，不是 42 / 42 ⇒ **旧口径「组件层 42 / 519（形状面）」作废**。

## 4. 出口测试（钉）

固件 **171 → 174**（**+3**）：

1. `sico_codegen_guest_emits_the_match_component_bytes_byte_exactly` —— 4 个形状的**组件字节逐字节等同**生产后端 + 4 个尺寸钉（1215 / 1214 / 1167 / 1389 B）+ `wasmparser::Validator` 通过 + **`sico.list.get` 头保持 typed 拒绝**（`unsupported codegen source shape`）。
2. `sico_compiler_lowers_the_step0535_codegen_functions_byte_exactly` —— 7 个新 guest 函数**逐个**经 guest 前端降低、与 Rust 前端的 canonical IR **逐字节等同**（自举形状前沿）。
3. `sico_compiler_reproduces_the_frozen_match_site_ledger` —— **R0.4**：42 个**真实** S 站点逐站对 oracle 比对（canonical IR + 组件字节 + 真实 validator），并核对冻结账本的 verdict 与 digests。

**前沿探针**：**71 条不变**（本片只改发射端，前端前沿不动）⇒ 预期 `FRONTIER_UNCHANGED`。

## 5. 验证记录（2026-10-24）

### 5.1 固件

- **176 passed / 0 failed**（M-g4a 段新增 3 条 ⇒ 174；R0.3 / R0.5 段再新增 2 条 ⇒ 176，见 §7.1 / §7.2）。

### 5.2 前沿探针

- **71 / 71 全部与期望一致** ⇒ `FRONTIER_UNCHANGED`。

### 5.3 电池

- 本机 **BATTERY_GREEN 14/14**（**版本化电池** `tools/validate-m22-s5.ps1`，步骤表见 §5.5）。

### 5.4 独立 CI

- 上一批 **STEP-0534**（提交 `54b6d97`）的 **run #43** = **success ✅**（见 §5.6）。
- 本片推送后触发下一轮（**run #44**），**裁决由下一片回填**。

### 5.5 电池步骤表（本片实测）

电池脚本已**版本化**为 `tools/validate-m22-s5.ps1`（**14 步**；每步记录命令、耗时与 exit，任一步失败即整体失败）。本片实测日志：`target/tmp/m22-s5-battery/battery.log`（gitignored）。

| 步 | 秒 | exit | 命令 |
|---|---|---|---|
| fmt (workspace) | 1 | 0 | `cargo fmt --all -- --check` |
| fmt (runner) | 2 | 0 | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| clippy (workspace) | 1 | 0 | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| clippy (runner) | 1 | 0 | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| ci-one-key | 423 | 0 | `powershell -File tools/run-ci.ps1` |
| validate-0245 | 134 | 0 | `powershell -File tools/validate-step-0245.ps1` |
| validate-0262 | 151 | 0 | `powershell -File tools/validate-step-0262.ps1` |
| probe-frontend-frontier | 483 | 0 | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| probe-ir-plan-frontier | 26 | 0 | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| comment-discipline | 0 | 0 | `python tools/validate-selfhost-comment-discipline.py` |
| match-site-census | 0 | 0 | `python tools/census-m22-match-sites.py --check` |
| match-site-coverage | 6 | 0 | `powershell -File tools/probe-m22-match-coverage.ps1` |
| firmware-selfhost | 123 | 0 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| git-diff-check | 0 | 0 | `git diff --check` |

⇒ **BATTERY_GREEN 14/14**（版本化电池；固件步单独跑 = **176 passed / 0 failed**；同轮 `FRONTIER_UNCHANGED` / `PLAN_FRONTIER_UNCHANGED` / `LEDGER_CURRENT sites=42` / `MATCH_COVERAGE_OK`）。

> **电池必须单实例运行（实测踩坑）**：本片第一次重跑时，上一轮遗留的电池进程**仍在运行**（未退出），于是**两个电池实例并发**争用同一 `target/` 与同一 `battery.log` ⇒ 日志被交错、步骤顺序失真、结果不可引用。处置 = 终止陈旧进程 ⇒ **单实例重跑** ⇒ 上表即来自那次**干净单实例**运行（`battery-clean.log`）。**结论**：电池的「每步 exit」只有在**同一时刻只有一个实例**时才是证据；并发跑的电池**必须作废重跑**，不能拿它当绿灯。


### 5.6 独立 CI 回填（run #43 = 上一批 STEP-0534）

| 项 | 值 |
|---|---|
| run | **#43**（id `40053245939`） |
| head_sha | `54b6d97`（STEP-0534） |
| 结论 | **success ✅** |
| 用时 | 2026-10-24T02:13:07Z → 02:33:18Z（**20m11s**） |
| 连续性 | **#41 / #42 / #43 三轮连续 success** |

> **观测说明**：run #43 在观测窗口内一度显示 `in_progress` ⇒ **最终完成且为绿** ⇒ 「CI 长时间 in_progress」时**必须以最终态为准**（承 STEP-0534 §7.6 的同一条结论）。

### 5.7 独立 CI 回填（run #44 = 本片 STEP-0535）= **failure**

| 项 | 值 |
|---|---|
| run | **#44**（id `40067049721`） |
| head_sha | `da01829`（STEP-0535） |
| 结论 | **failure ❌** |
| 用时 | 2026-10-24T05:38:36Z → 06:04:03Z（**25m27s**） |
| GitHub 步骤 | `Set up job` ✅ / `checkout` ✅ / `Install GNU build tools` ✅ / **`Run repository CI on a fresh runner` ❌** / `Post checkout` ✅ / `Complete job` ✅ |
| 注解 | `windows-gnu — Process completed with exit code 1`（另两条 Node.js 20 / MINGW64 弃用警告，与失败无关） |

> **失败子步骤不可得**：GitHub 的 **Actions 日志需要登录会话**才能读（匿名 API 的 `jobs/{id}/logs` 返回 **403**，网页端显示「Sign in to view logs」）⇒ 与 **run #39** 同一个限制 ⇒ **必须由 owner 提供已登录会话 / `gh` / token 才能定位**。本片**不把红灯包装成绿灯**，也**不**在没有证据的情况下把它归因到某个子步骤。
>
> **本机对照（实测）**：`da01829` **本身**：整轮版本化电池 **14 / 14 绿**（含 `run-ci.ps1` 自身）、固件 **174 / 174**；**本片工作区**（多出 R0.3 / R0.5 两条新固件）：干净**单实例**电池 **14 / 14 绿**（固件 **176 / 176**，见 §5.5）、冷组件缓存下**整个 runner 套件全绿** ⇒ 本机**无法复现**该红灯 ⇒ 红灯目前只知「发生在 `run-ci.ps1` 之内」。
>
> **可诊断性改进（本片附带）**：`tools/run-ci.ps1` 现在把**失败子步骤的名字**与**该子步骤输出的末尾若干行**写成 GitHub **工作流注解**（`::error title=CI step failed::…`），并把逐步骤结果表写进 **job summary** ⇒ 这两者在**运行页面上无需登录**即可读（run #44 的页面已证明「Annotations」区块匿名可见）⇒ **下一次红灯的子步骤不再需要凭据就能定位**。

### 5.8 独立 CI 回填（run #45 = 本片 R0.3 / R0.5 提交 `f83ab7b`）= **success ✅** ⇒ **R0 exit 达成**

| 项 | 值 |
|---|---|
| run | **#45**（id `40073369883`） |
| head_sha | `f83ab7b`（STEP-0535 的 **R0.3 / R0.5** 收尾提交） |
| 结论 | **success ✅** |
| 用时 | 2026-10-24T07:14:46Z → 07:33:50Z（**19m04s**） |
| GitHub 步骤 | `Set up job` ✅ / `checkout` ✅ / `Install GNU build tools` ✅ / `Run repository CI on a fresh runner` ✅ / `Post checkout` ✅ / `Complete job` ✅ |

> **R0 exit 的判定**：owner 审计 `AGENTS.md` §10.4 的 R0 五项（R0.1 修正声明 / R0.2 版本化电池 / R0.3 畸形 IR fail-closed / R0.4 真实站点账本 / R0.5 布局与边界补测）**全部交付**，且**最后一项外部条件** —— STEP-0535 的**精确提交快照**的独立 CI 裁决 —— 由 **run #45 = success** 满足 ⇒ **R0 exit 达成**（STEP-0536 起，M22 的工作面回到 R1 的族闭合阶梯）。
>
> **与 run #44 的关系**：run #44 跑的是 `da01829`（R0.3 / R0.5 尚未提交的那棵树），结果 **failure**、子步骤不可得；**run #45** 跑的是**加了 R0.3 / R0.5 的精确提交** `f83ab7b`，结果 **success** ⇒ 两次红灯/绿灯之差**不能**归因到某个子步骤（run #44 的子步骤至今不可得），**只能**记为：**该提交快照在独立 Windows GNU runner 上全绿**。
>
> **本机对照**：`f83ab7b` 的干净**单实例**版本化电池 **14 / 14 绿**（固件 **176 / 176**，见 §5.5）⇒ 与独立 CI **同结论**。

## 6. 诚实性

### 6.1 本片**闭合**了什么

- **组件层从 0 到 4 个形状端到端逐字节等同**（首次）—— 这是 `match` 族（519 站点）收口路上第一个组件级硬证据。
- **订正了 STEP-0534 的归因**：组件层零支持是**五个缺口**叠加，不是「`match` 没实现」。
- **发射形状全部来自生产后端**（`compile_terminator` / `emit_checked_fixed` / `apply_result_fields` / `lower_local_types`）⇒ 逐字节等同是可证的，不是设计值。
- **R0.4**：把「42 / 519 形状面闭合」的外推换成 **42 个候选真实站点的逐站证据**（组件级全等 **8 / 42**），并用**版本化电池**取代一次性脚本 ⇒ 本片的声明现在只覆盖实测范围。
- **R0.3**：canonical IR 边界上**两道真实缺口**已补齐并**双方 typed 拒**（完整 flat-type 比较 + variant-layout 校验），且拒绝身份**沿发射链透传**（`component_refusal_for_ir`）⇒ 失败从「空串」变成**可归因的身份**。
- **R0.5**：六个布局/边界形状（ok/error 投影、Result cell、布尔模式、U64 add/sub 判定）与生产后端**逐字节等同**，有符号 checked 的收窄**钉成双方拒绝身份**。

### 6.2 本片**没有**闭合什么

| 项 | 状态 |
|---|---|
| **`match` 构造的族闭合** | ❌ **NO-GO** —— 组件层覆盖 **4 个合成形状**，**真实站点只有 8 / 42 组件级全等**（R0.4 实测），`sico.list.get` 系列 **15 站点**仍拒 ⇒ **D10 ⇒ 不得降级、不得宣告闭合** |
| **`sico.list.get` 头的组件层** | ❌ **M-g3' 仍开** —— 发射器没有该 intrinsic 的宿主调用传输（**实测**：生产后端对它**不是** import 调用，而是**内联线性内存查表序列**） |
| **有符号的 checked 运算** | ❌ `I64.checked_add` / `checked_sub` **仍 typed 拒**（guest 只实现了无符号的两个变体）；**R0.5 已把这条收窄钉成双方拒绝身份**（oracle 发射、guest `ERR:E-SH-CG-IR-EMIT`） |
| **bool 匹配头的槽位校验** | ✅ **R0.5 已闭合** —— guest 的 bool 模式项现在要求被匹配值是 `bool` 单槽标量（`ERR:E-SH-CG-IR-MATCH-SCALAR`），与后端 `LocalLayout::scalar` 的「aggregate used as scalar」对齐 |
| **畸形 canonical IR 的 fail-closed** | ✅ **R0.3 已闭合** —— Result 投影改比**完整 Wasm 类型序列**（`ERR:E-SH-CG-IR-PROJECT-TYPE`），variant 模式拒**非 variant 值**（`ERR:E-SH-CG-IR-MATCH-VARIANT`）；四个畸形形状**双方 typed 拒**（§7.1） |
| **溢出/下溢的运行期行为** | ⏔ R0.5 覆盖的是**发射字节**（判定指令序列逐字节等同），**不是**运行期行为证据 —— 注入的 `size` 不在 `run` 的可达路径上，本片没有执行发射出的组件 |
| **前端三个门** | ❌ **M-g1（354）/ M-g2（46）/ M-g3（21）** 一个都没动（本片只改发射端）；**M-g1 已在真实站点复现**（`codegen.sico:cg_next` ⇒ `ERR:E-SH-IR-MATCH-SHAPE`） |
| **语料覆盖** | ⏔ **仍 0 条** |
| **整文件自举** | ⏔ 仍未关闭；本片把它的首要阻塞点从「组件层 0」推进到「组件层真实站点 8 / 42」 |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；门 1 / 4 / 5 / 7 GO；门 2 PARTIAL；**M22 整体仍 NO-GO** |

### 6.3 下一刀（按实测杠杆，R0 全部交付之后）

1. **M-g1 形参计数门**（**354 站点**）—— 前端刀：放宽 `checked_arithmetic_*` 路由的形参计数门（**两个落点**：`selfhost/parser.sico:19473-19479` 的 `set` 路由只允许 1..2 形参、`:20364` 的 `return` 路由只允许恰好 2）。**R0.4 已在真实站点复现**（`codegen.sico:cg_next`）⇒ 不是理论缺口。
2. **M-g3' 组件层的 `sico.list.get` 宿主调用传输**（**15 站点**）—— 发射端刀：**实测**生产后端对它**不是** import 调用，而是**内联线性内存查表序列**（`table`/`count` → `i32.const 3; i32.shl; i32.add` → 读 `ptr`/`len`）⇒ 这一步是内联序列，不是「加一个 import」。
3. **发射端对真实函数体形状的覆盖**（**9 个真实站点**）—— `is_ident_byte` / `type_json` / `lex_words` ×5 / `gw_instr_count` / `return_expression_start` 的 IR 已逐字节等同，但**发射端不认识这些函数体** ⇒ 组件轴缺的是**分支 / 循环 / 多语句函数体**的发射能力。
4. **M-g2 算子白名单**（**46 站点**）—— 前端刀：`checked_sub` / `checked_div` / `checked_mul`；其中 `checked_sub` 的**发射**本片已经具备（**R0.5 已逐字节钉**）。
5. **M-g3 未认领的匹配头**（**21 站点**）—— 前端刀：`sico.bytes.slice` 头。
6. 之后回到 **T3 族**、**语料覆盖**、**门 3 / 6**。

> **一条方法学结论**：**「某形状被拒绝」≠「该形状没实现」**（承 STEP-0521）在本族再次成立，而且这次代价更大：组件层的一个「拒绝」背后可能是**五个**独立的缺口 ⇒ **拒绝必须逐个归因**，否则下一片会按错误口径下刀。
>
> **R0.4 追加的第二条方法学结论**：**「一个形状通过」≠「覆盖它的站点通过」** —— 4 个合成 fixture 通过**推不出** 42 个真实站点通过；owner 审计据此把「42 / 519 形状面闭合」判为外推 ⇒ **真实站点、合成形状、冻结语料是三种不同覆盖口径，必须分别报告**。

## 7. **owner 审计 R0：STEP-0535 证据整改**

> owner 于 2026-10-24 在 `AGENTS.md` §10 写入 M22 整改与收尾计划（R0–R6），把 **R0「STEP-0535 证据整改」** 定为**先于任何新能力片**的工作，并判定本片先前的「42 / 519 形状面闭合」声明**不成立**（那是按形状归并的外推，不是逐站点证据）。本片**同一提交**内交付 R0 的三项：

| R0 项 | 交付物 | 状态 |
|---|---|---|
| **R0.1 修正声明** | 登记表 / STATUS / ROADMAP / handoff / 计划一律降为「**4 个合成形状通过**；**42 个候选站点逐站复核**（组件级全等 **8 / 42**）」；旧口径「组件层 42 / 519（形状面）」与「组件层 0 / 519」标注作废 | ✅ |
| **R0.2 恢复可复现电池** | **版本化** `tools/validate-m22-s5.ps1`（**14 步**：fmt ×2 / clippy ×2 / `run-ci.ps1` / 0245 / 0262 / 两个前沿探针 / comment discipline / 站点普查 / 站点覆盖 / 固件 / `git diff --check`），**每步记录命令、耗时与 exit，任一步失败即整体失败**；**不再依赖** `target/tmp/step0526/integrity.py` 的硬编码 dirty-list | ✅ |
| **R0.4 建立真实站点账本** | `selfhost/match-component-coverage-v0.json`（42 条：path / function / head / labels / case-shape / **verdict** / **source·ir·component 三条 digests**）+ 普查脚本 `tools/census-m22-match-sites.py`（账本的唯一写者，可 `--check` / `--update`）+ 探针 `tools/probe-m22-match-coverage.ps1` + 固件钉 `sico_compiler_reproduces_the_frozen_match_site_ledger`（逐站 IR + 组件 + **真实 `wasmparser::Validator`**） | ✅ |
| **R0.3 畸形 IR 拒绝测试** | canonical-IR 注入夹具 + guest 的完整 flat-type 比较与 variant-layout 校验 | ✅（§7.1） |
| **R0.5 布局/边界补测** | ok/error 投影、Result cell、布尔模式、I64 checked 明确拒、U64 add 溢出 / sub 下溢的逐字节或双方拒绝身份 | ✅（§7.2） |

**R0.4 的实测结论（42 个真实站点）**：

| verdict | 站点 | 说明 |
|---|---|---|
| `ir-equal+component-equal` | **8** | 组件字节与生产后端**逐字节等同**，两侧都过真实 validator |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **9** | canonical IR 逐字节等同；**发射端**不认识该函数体形状 |
| `guest-refused:ERR:E-SH-IR-MATCH-SHAPE` | **1** | **前端就拒**：`codegen.sico:cg_next` ⇒ **M-g1 在真实站点上复现** |
| `closure-too-large` | **24** | 同文件依赖闭包 > 64 函数 / 256 KiB（`parser.sico:scalar_ir`）⇒ IR 证据取自**整文件**差分钉，组件轴未取得 |

**比较单元**：每个站点的比较单元是「**同文件依赖闭包** + Script-profile 载体（`run` 与 `ScriptInput`/`ScriptOutput` 记录）」⇒ 两侧看到**同一个模块**，组件轴才可比；闭包过大时**不静默跳过**，而是显式记为 `closure-too-large` 并指明整文件差分钉。

**探针与账本的纪律**：账本是**冻结的期望表** —— 站点行号、函数行号、匹配头、case 标签都在固件钉里逐条复核；任何站点移动 ⇒ **钉失败**（与前沿探针同一条纪律）。账本只能由 `tools/census-m22-match-sites.py` 写，`--check` 模式证明它仍描述冻结的 S。

> **R0 exit 的当前状态**：版本化电池 ✅、42/42 候选站点有当前快照证据 ✅、**畸形 IR 测试 ✅（§7.1）**、文档只声明实测范围 ✅ ⇒ **R0 的五项已全部交付**；**R0 exit 仍差最后一项外部条件**：STEP-0535 的**精确提交快照**的独立 CI 裁决（run #44）。

### 7.1 R0.3：畸形 canonical IR 的 fail-closed（注入夹具）

**机制**：canonical IR 是 guest 发射器的**不可信边界输入**。本片在 **runner 测试侧**建了一个注入夹具（`IR_INJECTION_SOURCE`）：它把 stdin 的 canonical IR **直接**交给 `codegen.component_hex_for_ir` / `codegen.component_refusal_for_ir`，因此**Sico 前端无法产生的 IR 也能到达生产后端看到的同一条边界**。**没有新增产品 CLI，没有改 Rust oracle**（`crates/*` 零改动）；畸形 IR 用 `serde_json::to_string` 序列化（`canonical_json` 会先跑 `verify`，无法序列化畸形模块）。

**guest 侧两处真实缺口与修法**（`selfhost/codegen.sico`）：

| 缺口 | 生产后端合同 | guest 修法 | 新身份 |
|---|---|---|---|
| Result 投影只比**槽数** | `Operation::Project` 布局比较**完整 Wasm 类型序列**（`layout.types != expected`） | 新增 `cgs_result_arm_ty`（取臂的 IR 类型），`cgs_project_result_hex` 改为比较臂的 **flat-type 序列**与声明类型的 flat-type 序列 | `ERR:E-SH-CG-IR-PROJECT-TYPE` |
| variant 模式不校验**变体布局** | `LocalLayout::tag` 拒「variant pattern over non-variant value」；`LocalLayout::scalar` 拒「aggregate used as scalar」 | `cgs_match_pattern_hex`：variant 臂要求被匹配值的类型是 `result`；bool 臂要求被匹配值的类型是 `bool`（单槽标量） | `ERR:E-SH-CG-IR-MATCH-VARIANT` / `ERR:E-SH-CG-IR-MATCH-SCALAR` |

**拒绝身份透传**：此前发射链把每个失败都塌缩成空串（`""`），无法区分是哪一道闸门拒的。本片让**身份沿链透传**（`cgs_function_body_hex` → `cgs_core_code_hex` → `cg_core_module_hex` → `cg_component_hex`），并新增导出函数 `component_refusal_for_ir`：成功返回 `""`，失败返回**第一道拒绝的身份**；`component_hex_for_ir` 的公开契约不变（失败仍是 `""` ⇒ 驱动侧仍报 `unsupported codegen source shape`）。

**实测（固件 `the_malformed_canonical_ir_is_refused_by_the_guest_and_by_the_rust_oracle`）**：

| 形状 | Rust oracle | guest 身份 |
|---|---|---|
| **阳性对照**：未畸形的 match 模块 | 发射组件（过真实 validator） | **组件字节逐字节等同**（证明注入夹具确实到达了发射端） |
| **同槽数异型投影**（`project ok` 声明成 `bool`） | 拒（verifier `TypeMismatch`） | `ERR:E-SH-CG-IR-PROJECT-TYPE` |
| **非 variant 值上的 variant 模式**（匹配值改成 `u64` 参数） | 拒（verifier `TypeMismatch`） | `ERR:E-SH-CG-IR-MATCH-VARIANT` |
| **非布尔值上的字面模式**（匹配值改成 `u64` 参数、模式改成 bool） | 拒（verifier `TypeMismatch`） | `ERR:E-SH-CG-IR-MATCH-SCALAR` |
| **非法嵌套 payload**（variant 模式里再套 variant 模式） | 拒（后端「match payload inspection」） | `ERR:E-SH-CG-IR-EMIT` |

> **归因订正**：这**四个**形状**都不能由合法 Sico 源触发**（verifier 先拒）⇒ 它们测的是**canonical-IR 边界**的 fail-closed，不是语言能力。**「被拒绝」≠「没实现」**：这里被拒的正是**应当被拒**的输入。

### 7.2 R0.5：布局与边界补测

固件 `the_result_layout_and_the_checked_arithmetic_boundaries_are_pinned`。**每个被接受的形状都与生产后端逐字节比较**（不是只比长度），并都过真实 `wasmparser::Validator`：

| 形状 | 覆盖点 | 结论 |
|---|---|---|
| `ok-arm-projection` | `project ok` 的**槽位算术 + 类型序列** | 组件字节**逐字节等同** |
| `result-cell` | `Result[u64,u64]` cell 的**局部声明 + 回读** | 组件字节**逐字节等同** |
| `error-arm-projection` | `project error` 的**臂偏移（tag + ok 载荷之后）** | 组件字节**逐字节等同** |
| `bool-pattern` | **布尔模式**的 `i32.eq` 发射 | 组件字节**逐字节等同** |
| `u64-add-overflow-check` | `checked_add` 的**无符号溢出判定**（`i64.lt_u`）与错误载荷 | 组件字节**逐字节等同** |
| `u64-sub-underflow-check` | `checked_sub` 的**无符号下溢判定**与错误载荷（与 add 的错误载荷不同） | 组件字节**逐字节等同** |
| **I64 checked** | guest 覆盖**窄于** oracle：后端发射有符号溢出判定，guest **明确拒绝** | 双方**拒绝身份**：oracle 发射；guest `ERR:E-SH-CG-IR-EMIT` |

> **诚实性边界**：本片的溢出/下溢覆盖是**发射字节级**的（溢出判定的指令序列逐字节等同生产后端），**不是**运行期行为证据 —— 注入的 `size` 函数不在 `run` 的可达路径上，本片**没有**执行发射出的组件去观测运行期溢出结果。该缺口登记在 §6.2。

---

**证据等级**：本片是**生产后端对照的实现证据**（guest 组件字节 ≡ 生产后端组件字节，逐字节）+ **真实站点逐站差分证据**（IR + 组件 + 真实 validator）；**不是**独立 CI 证据（CI 裁决由下一片回填）；**不是**整文件自举证据。
