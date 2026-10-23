# STEP-0532 m22-w2 S5 **W2 族 T2 的 T2-g2 实现刀**：「实参本身是嵌套调用」的前端形状落地

> - status: **本机实测绿（随本片提交并推送；独立 CI 裁决待下一批回填）**
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0529](./STEP-0529-m22-w2-s5-w2-t2-frontend-knife.md)（**T2-g1 前端刀 F**）、[STEP-0530](./STEP-0530-m22-w2-s5-w2-t2-codegen-knife.md)（**T2-c1 codegen 刀 C**）、[STEP-0531](./STEP-0531-m22-w2-s5-w2-t2-g2-truth.md)（**T2-g2 的真值测量片**：目标 IR 规则、拒绝身份、两条路由落点）、[STEP-0525](./STEP-0525-m22-w2-s5-w2-t1-call-arg-nested-call.md)（**T1 族 G-f2 刀**：嵌套调用实参的既有先例与「落点 = 内联」的教训）
> - gate: **不推进任何出口门** —— 本片是 **T2-g2 的实现片**，**oracle 生产代码零改动** ⇒ **不包装为特性支持**
> - numbering: **STEP-0532**（取号前已核对 `docs/steps/`：最大号 0531；*0532 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表 §3.3](../reports/m22-support-surface-t2-text-length.md#33-t2-g2-的实现刀step-0532)

## 0. 入口门槛

| 入口条件 | 状态 |
|---|---|
| T2 族的**其余两条缺口已闭合**（T2-g1 = STEP-0529、T2-c1 = STEP-0530）⇒ 只剩 T2-g2 | ✅ |
| 上一刀**只登记缺口、不预支能力**：STEP-0531 §7.3 写明「下一刀 = T2-g2 刀：让 guest 前端认**实参本身是嵌套调用**的 `sico.text.length`」，并给出**两条落点** | ✅ |
| 出口测试有界 | ✅ §2：**3 条新钉 / 2 条改钉**（前端逐字节等同 + 组件层分层 + 诚实归因 + 自举形状前沿）+ 前沿探针 53 → **54** 条用例 |
| **本片只改 `selfhost/parser.sico`**（oracle 生产代码 `crates/*` **零改动**） | ✅ §4.1 |

⇒ **取号合法**。*实现已完成并本地实测绿*，见 §5。

## 1. 这一片回答的问题

STEP-0531 把 T2-g2 的**靶子**量清楚了（目标 IR 规则、5 个形状的拒绝身份、两条路由落点）。本片把那把刀落下去，并回答一个 STEP-0531 **没有**问到的问题：

**S 内那两个真实调用点，降下来了吗？**

**本片的结论（先说结果）**：

- **前端层：形状闭合** —— `sico.text.length(<嵌套调用>)` 的 **5 个形状全部逐字节等同** oracle 的 canonical IR（**2477 / 2656 / 2369 / 2783 / 2374 B**）；**另补测 STEP-0529 的 2 个 `let` 绑定形**（**2656 / 2369 B**）⇒ 前端合计 **7/7** 逐字节等同。
- **组件层：分层** —— 主测 5 个形状里 **3 个**的整组件与 Rust 后端生产输出**逐字节等同**（**1183 / 1145 / 1162 B**）；另 **2 个形状**被**发射端**拒（`unsupported codegen source shape`），根因是**内层算子** `sico.bytes.utf8_decode`（**T11 族**）发射端不认识 ⇒ **不是 T2 的缺口**；**补测的 2 个 `let` 绑定形**同样是 **1 个逐字节等同**（**1145 B**）+ **1 个发射端拒** ⇒ 组件层合计 **4/7 逐字节等同 + 3/7 发射端拒**。
- **两个真实 S 调用点：宿主函数仍不降，但归因已钉死，且归因都在 T2 之外**：
  - **S 站点 ①**（`compiler.sico:57`，宿主函数 `lex_words`）⇒ `ERR:E-SH-IR-CALL-SHAPE`，根因是它携带的 `sico.list.append(<局部>, <嵌套调用>)` 形状（**T7 族**）；
  - **S 站点 ②**（`compiler.sico:361`，宿主函数 `positive_sleb_length`）⇒ `ERR:E-SH-IR-MATCH-SHAPE`，根因是**匹配路由要求恰好 2 个形参**（该函数 1 个）；其后再叠一道**固定算子实参路径不接受 `sico.*` 实参**的闸门（对照形状：**2 个形参 + 非嵌套实参**同样被拒）⇒ **与嵌套调用形状无关**。
- ⇒ **T2 族的「形状面」闭合；T2 族仍不得宣告整体闭合**（两个真实调用点的宿主函数不降 + 语料覆盖 0 条 + 整文件自举未关闭）。

## 2. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 | 状态 |
|---|---|---|---|
| 1 | `the_t2_g2_nested_call_shapes_lower_byte_exactly`（**改钉**，承 STEP-0531 的「保持 typed 拒绝」钉） | **5 个嵌套实参形状**的 guest canonical IR **逐字节等同** oracle（IR 字节数 **2477 / 2656 / 2369 / 2783 / 2374** 钉住） | ✅ 改钉 |
| 2 | `sico_compiler_lowers_the_nested_text_length_arguments_byte_exactly`（**改钉**，承 STEP-0529 的「保持 typed 拒绝」钉） | **2 个 `let` 绑定形**（嵌套用户调用 / 嵌套宿主调用）**逐字节等同**（**2656 / 2369 B**） | ✅ 改钉 |
| 3 | `the_t2_g2_component_layer_covers_the_nested_shapes_the_emitter_knows`（**新**） | **4 个形状整组件逐字节等同**（**1183 / 1145 / 1162 / 1145 B**，两侧都过真实 `wasmparser::Validator`）+ **3 个形状被发射端拒**（`unsupported codegen source shape`）⇒ **前端拒绝与发射端拒绝分开记** | ✅ 新增 |
| 4 | `the_t2_g2_s_site_refusals_belong_to_other_families`（**新**） | **3 个对照形状**钉住 S 站点 ② 的归因（1 形参 + 嵌套 ⇒ `ERR:E-SH-IR-MATCH-SHAPE`；2 形参 + **非嵌套** ⇒ `ERR:E-SH-IR-CALL-ARGUMENT`；2 形参 + 嵌套 ⇒ `ERR:E-SH-IR-CALL-ARGUMENT`）+ **同一形状离开匹配路由后逐字节降** | ✅ 新增 |
| 5 | `sico_compiler_lowers_the_step0532_parser_functions_byte_exactly`（**新**） | **自举形状前沿**：新函数 `nested_text_call_packed`（形参打桩版 + **真实闭包版**）与两条被接线的路由（`straight_text_length_packed` / `gw_rhs_packed`）**逐字节等同** | ✅ 新增 |

⇒ 固件从 **163** 增至 **166** 条；前沿探针从 **53** 增至 **54** 条用例（**4 条翻转** + **1 条新增**）。

## 3. 实施设计

### 3.1 落点（一处新函数 + 两处接线，**纯插入**）

| 落点 | 覆盖的位置 | 手法 |
|---|---|---|
| **新函数 `nested_text_call_packed`**（`selfhost/parser.sico:9086`） | —— | 把「实参是一个调用」降低成 `<指令数>\n<结果 id>\n<种类>\n<指令 JSON>` 的 pack |
| `straight_text_length_packed`（**直线路由家族**） | `return` / `let` / `set` 语句位 | 在既有的「实参不是单个词 ⇒ `ERR:E-SH-IR-EXPRESSION`」之前**插入**嵌套调用分支；**既有的拒绝行原样保留**（只多一层缩进语义） |
| `gw_rhs_packed` 的 `sico.text.length` 分支（**通用宿主调用路由**） | 通用路由函数（`general_while_function_ir` 家族）的 `let` / `set` 右值、分支块内的语句 | 在该分支的 `return gw_intrinsic_call_packed(...)` **之前插入**嵌套调用分支；**既有行不重排** |

**没有新造机器**：嵌套调用的降低**复用**既有降低器 —— `sico.text.concat` 走 `straight_text_concat_packed`（T1 族已证明可降）、`sico.bytes.utf8_decode` 走 `gw_intrinsic_call_packed`、用户调用走 `user_call_expression_json`。

### 3.2 声明的子集边界（**不得放宽**）

| 项 | 值 |
|---|---|
| 嵌套调用的种类 | `sico.text.concat` / `sico.bytes.utf8_decode`（宿主内建）+ **用户定义的函数调用** |
| 嵌套调用的实参 | 形参或字面量（用户调用走 `user_call_expression_json`，其局部表为空） |
| 外层 `sico.text.length` 的元数 | **恰好 1**（未放宽） |
| 外层 `sico.text.length` 的实参类型 | **必须是 `Text`**（嵌套调用的返回种类 ≠ `string` ⇒ `ERR:E-SH-IR-CALL-TYPE`） |
| **尚未覆盖** | **比较实参路由**（`gw_compare_packed`）：`if` / `while` 条件里把 `sico.text.length(<嵌套调用>)` 当作比较的一侧，仍 typed 拒 `ERR:E-SH-IR-CALL-SHAPE`（S 内**没有**这种调用点） |

### 3.3 值 id 规则（承 STEP-0531 的实测，本片实测复现）

```
嵌套调用 = 1 条指令（或 2 条：实参字面量 + 调用本身）
嵌套调用的结果 id  <  外层调用的结果 id      （后序分配）
外层 sico.text.length 的唯一实参 = 嵌套调用的结果 id
```

## 4. 实施记录（2026-10-24）

### 4.1 改动面

`selfhost/parser.sico` **+197 / −0 行**（22829 → **23026** 行，函数数 **177 → 178**）⇒ **纯插入、既有行零重排**。
`runner/sico-runner/tests/selfhost_compiler.rs` **+245 / −29 行**（3 条新钉 + 2 条改钉 + 1 处固件闭包表补项 + 组件层钉补进 2 个 `let` 绑定形）。
`tools/probe-m22-frontend-frontier.ps1` **+21 / −16 行**（4 条期望翻转 + 1 条新增用例）。
**oracle 生产代码 `crates/*` 零改动** ⇒ **零自举风险**（guest 侧）、**零字节等同性风险**（oracle 侧）。

### 4.2 撞到并修掉 1 条自造缺陷：**接线落在死代码里**

第一版把 `gw_rhs_packed` 的嵌套调用分支**插在**了那个「识别到 `sico.text.length` 就 `return gw_intrinsic_call_packed(...)`」的 `if` **之后** ⇒ 该 `if` 永远先返回 ⇒ 新分支**不可达** ⇒ 通用路由仍然 typed 拒 `ERR:E-SH-IR-CALL-SHAPE`。

**症状定位**：语句位（`return` / `let` / `set`）与分支块内的语句都降了，**只有通用路由的 `let` 右值不降** ⇒ 说明接线没进那条路。
**修法**：把嵌套调用分支移进 `if` **内部**、`return` **之前** ⇒ 通用路由的 `let` 右值随即降下来（`general-route-let` 形状由 `REFUSED` 变 `LOWERED`）。

> **教训**：**插入点必须落在「已经会返回」的分支内部**，否则新代码是死代码；**位置探针**（哪些位置降、哪些不降）比「整体跑一遍」更快地暴露这类缺陷。

### 4.3 固件侧撞到 1 条连带缺陷：**手维护的闭包表漏了新函数**

STEP-0400 的那条固件测试**手维护**了一份 `parser.sico` 函数表（不是算出来的闭包）⇒ 新函数不可达 ⇒ oracle 报 `unresolved call target nested_text_call_packed`。

**修法**：把那 9 个新进入闭包的成员补进表里（`nested_text_call_packed` 及其可达的 8 个）⇒ 该测试恢复绿。
**顺带**：用**脚本**把固件里 12 条手维护表全部扫了一遍，确认**只有这一条**需要补。

> **教训**：**手维护的闭包表会随路由变化漂移**（STEP-0523 已经为此改用算出来的闭包）；本片再次撞上 ⇒ **新函数入库后必须扫一遍全部手维护表**。

### 4.4 诚实归因：两个真实 S 调用点的宿主函数**仍不降**

STEP-0531 的 5 个钉用的是**简化 fixture**，本片改用 `selfhost/compiler.sico` 的**逐字原文**重测，判词**变了**：

| 真实站点 | 宿主函数 | 改前判词 | 改后判词 | 归因（对照形状证伪） |
|---|---|---|---|---|
| ① `compiler.sico:57` | `lex_words` | `ERR:E-SH-IR-EXPRESSION` | **`ERR:E-SH-IR-CALL-SHAPE`** | 该函数另含 `sico.list.append(<局部>, <嵌套调用>)` 形状（**T7 族**）；把 T2 那一句摊平后判词**再变**（`ERR:E-SH-IR-CALL-TARGET`）⇒ T2 那一句**不再是**拦路的那一句 |
| ② `compiler.sico:361` | `positive_sleb_length` | `ERR:E-SH-IR-EXPRESSION` | **`ERR:E-SH-IR-MATCH-SHAPE`** | **匹配路由要求恰好 2 个形参**（该函数 1 个）；其后再叠**固定算子实参路径不接受 `sico.*` 实参**（对照形状：**2 个形参 + 非嵌套实参**同样 `ERR:E-SH-IR-CALL-ARGUMENT`）⇒ **与嵌套调用形状无关** |

⇒ **两个真实调用点的宿主函数仍不降，但都不再是 T2 的形状缺口** ⇒ 按 **D10**：**T2 族不得宣告闭合**；两个缺口分别登记到 **T7 族**与**匹配路由**名下（§7.2）。

### 4.5 补测：STEP-0529 的两个 `let` 绑定形在**组件层**的真值

STEP-0529 把两个 `let` 绑定形（嵌套用户调用 / 嵌套宿主调用）钉在**前端**（`ERR:E-SH-IR-EXPRESSION`）。本片把它们翻成前端逐字节等同（**2656 / 2369 B**），但**组件层**当时**没有单独测** —— 组件层的主测脚本覆盖的是 5 个 `return` 位的形状。

「没测」与「逐字节等同」不是一回事 ⇒ 本片**补测**：

| 形状 | oracle 组件 | guest 组件 | 裁决 |
|---|---|---|---|
| `let n = sico.text.length(helper(t))`（嵌套**用户**调用） | **1145 B** | **1145 B** | ✅ **逐字节等同** |
| `let n = sico.text.length(sico.bytes.utf8_decode(src))`（嵌套**宿主**调用） | **1570 B** | —— | ⏳ **被发射端拒** `unsupported codegen source shape`（内层算子 = **T11 族**） |

⇒ 两个形状**已补进固件钉**（`the_t2_g2_component_layer_covers_the_nested_shapes_the_emitter_knows`）⇒ 组件层合计 **4/7 逐字节等同 + 3/7 发射端拒**。

> **为什么值得补**：没钉的声明会漂。补钉之后，**前端与组件层的分母都写在同一张表里**，下一片不必再猜「这个形状到底测没测」。

## 5. 验证记录（2026-10-24）

| 验证 | 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`，冷构建） | ✅ **166 passed / 0 failed**（116.46s，**补钉之后**跑的） |
| oracle vs guest 的 canonical IR | `python target/tmp/step0532/compare_ir.py` | ✅ **5/5 形状逐字节等同**（2477 / 2656 / 2369 / 2783 / 2374 B） |
| oracle vs guest 的整组件 | `python target/tmp/step0532/compare_component.py` | ✅ **3/5 逐字节等同**（1183 / 1145 / 1162 B）；**2 个被发射端拒**（内层算子 = T11） |
| oracle vs guest 的整组件（**补测**：STEP-0529 的 2 个 `let` 绑定形） | `python target/tmp/step0532/compare_component_let.py` | ✅ `let-user-call`：**1145 B 逐字节等同**；`let-host-call`：**被发射端拒**（内层算子 = T11）⇒ 两个形状已补进固件钉 |
| 位置探针 | `python target/tmp/step0532/position_probe.py` | `return` / `let` / `set` / 分支块内 / 通用路由 `let` = **LOWERED**；**比较实参路由 = REFUSED**（§3.2 边界） |
| 归因探针 | `python target/tmp/step0532/attribution.py`、`attribution_lex.py` | ✅ §4.4 的判词全部复现 |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | ✅ **54 / 54 条用例全部与期望一致**（`FRONTIER_UNCHANGED`） |
| 本机电池 | `target/tmp/step0532/battery.ps1`（**19 步**） | 见 §6.2 |

### 6.1 T2 规则表更新

[T2 支持面登记表 §3.3](../reports/m22-support-surface-t2-text-length.md#33-t2-g2-的实现刀step-0532) 新增 **T2-g2 的实现记录**小节（前端 7 行逐字节等同 + 组件层 4 行逐字节等同 / 3 行发射端拒 + 位置边界 + 两个真实站点的归因）。

### 6.2 电池

本片电池 = `target/tmp/step0532/battery.ps1`（**19 步** = STEP-0531 的 19 步原样复用，把 `step0531-doc-check` 换成 `step0532-doc-check`），日志 `target/tmp/step0532/battery.log`。

| 步骤 | 耗时 | 裁决 |
|---|---|---|
| `workspace-integrity` | 55s | ✅ |
| `fmt-runner` | 1s | ✅ |
| `ci-one-key` | 443s | ✅（**CI GREEN**：fmt / clippy / build / test(workspace) / test(runner) / module boundaries / comment discipline / planning contract / profile matrix / cross-host / whitespace 全 PASS） |
| `validate-0245` | 126s | ✅ |
| `validate-0262` | 144s | ✅ |
| `probe-frontend-frontier` | 362s | ✅（**54** 条用例全部与期望一致，`FRONTIER_UNCHANGED`） |
| `probe-ir-plan-frontier` | 26s | ✅ |
| `validate-0124` | 1s | ✅ |
| `validate-0131` | 2s | ✅ |
| `validate-0156` | 1s | ✅ |
| `git-diff-check` | 0s | ✅ |
| `line-ending-audit` | 0s | ✅ |
| `selfhost-comment-discipline` | 0s | ✅ |
| `step0526-footprint-audit` | 0s | ✅ |
| `step0526-census-branches` | 0s | ✅ |
| `step0526-differential` | 85s | ✅ |
| `step0528-census-t2` | 0s | ✅ |
| `step0532-doc-check` | 0s | ✅ |
| `workspace-integrity-final` | 51s | ✅ |

⇒ **BATTERY_GREEN 19/19**。

> **电池跑了两轮，这是有意的**：第 1 轮在把 STEP-0529 的两个 `let` 绑定形**补测**并**补进固件钉**之前就已 19/19 绿；补测之后改了 `runner/sico-runner/tests/selfhost_compiler.rs` ⇒ 按纪律「改了 `runner/` 必须整轮重跑电池」⇒ **第 2 轮 19/19 绿**（上表即第 2 轮）。第 1 轮耗时：workspace-integrity 52s / fmt-runner 1s / ci-one-key 417s / validate-0245 126s / validate-0262 144s / probe-frontend-frontier 361s / probe-ir-plan-frontier 26s / validate-0124 1s / validate-0131 2s / validate-0156 1s / git-diff-check 0s / line-ending-audit 0s / selfhost-comment-discipline 0s / step0526-footprint-audit 0s / step0526-census-branches 0s / step0526-differential 85s / step0528-census-t2 0s / step0532-doc-check 0s / workspace-integrity-final 52s ⇒ **BATTERY_GREEN 19/19**。

### 6.3 独立 CI 回填（run #39 = **failure** ❌）

**本片的独立 CI 是红的。族级出口判据的「独立 CI 绿」一项因此尚未达成 ⇒ 本片不得被读成「族已闭合」。**

| 项 | 值 |
|---|---|
| run | **#39**，run id `40038444653`，head_sha `504c849df8712042b094752609fdd00979c346ad` |
| 触发 / 结束 | 2026-10-23T23:13:39Z 推送触发；job `windows-gnu`（id `120703551371`）耗时 **25m 26s** |
| 裁决 | **failure**（`status: completed` / `conclusion: failure`） |
| 失败的 workflow step | 第 4 步「Run repository CI on a fresh runner」（即 `tools/run-ci.ps1` 整体）—— workflow 把 13 个子步骤包在**同一个** step 里 ⇒ GitHub 只给出**这一层**的粒度 |
| 公开注解 | **1 error**（`Process completed with exit code 1`，指向 step 4 第 279 行）+ **2 warning**，两个 warning 都是**基础设施**的（`actions/checkout@v4` 的 Node.js 20 弃用、MSYS2 `MINGW64` 弃用）⇒ **没有任何 rustc / clippy / 测试诊断类注解** |
| 日志可得性 | **不可得**：公开 job 页显示「Sign in to view logs」；REST 日志端点匿名访问 **403**；本机**未装 `gh`**、环境内**无 token**；AGENTS.md §9 禁止未经明示使用外部凭据 ⇒ **失败的具体子步骤未能取到** |

**已做的复现尝试与结果**（全部为本机实测）：

| 尝试 | 结果 |
|---|---|
| 同一 worktree 跑 `tools/run-ci.ps1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | ✅ **绿**（417 s / 443 s，两轮电池的 `ci-one-key` 步） |
| **按 CI 的缓存配置**跑 runner 全量（`SICO_SELFHOST_COMPONENT_CACHE=1` + 全新 `TEMP` ⇒ 冷缓存） | ✅ **exit 0**；固件 **166 passed / 0 failed**（117.91 s） |
| **全新克隆** pushed SHA 后跑 `tools/run-ci.ps1` | ❌ **CI RED**，但红的是**五个 cargo 步**，根因 = **本机没有 C 编译器**（`cc-rs: failed to find tool "gcc.exe"`；仓库本地 `target/tooling/msys2-binutils` 在本机缺失）⇒ **CI runner 上 MSYS2 步是成功的**，所以**这次复现不能作为 CI 红灯的证据**，它只证明本机做不了 from-scratch 的 GNU 构建 |
| 给全新克隆预置第三方依赖产物（只重建仓库自身 crates） | ⛔ 被本机安全策略拦住（递归写），**未完成** |

**归因结论（诚实边界）**：本片**零改动 `crates/`** ⇒ from-scratch 重建的**第三方与仓库自身 crates** 与上一轮绿 run 完全同构；本片真正改动的输入只有 `selfhost/parser.sico`、固件、探针脚本与文档，而**这四者在本机所有可复现口径下全绿**（电池 19/19 ×2、固件 166/166 ×2、runner 全量 exit 0、探针 54/54、doc-check 绿、integrity 绿、clippy/fmt 绿）。⇒ **从现有证据无法把 run #39 的红归因到本片的内容**；与「runner 侧环境抖动（`cargo fetch` 约 200 个 crate 的下载、wasmtime 下载、MSYS2 安装）」这一解释**更相容**，但**无法证实** ⇒ 登记为**未归因的独立 CI 红灯**，不得写成「已查明是抖动」。

**处置**：本片的**代码与文档已经落地并本地全绿**，但**独立 CI 一项未达成** ⇒
1. **T2 族收口片 X 不得宣告族闭合**（族级出口判据「独立 CI 绿」缺一不算闭合）；
2. 本片附一次**纯文档的回填提交**，把本节写进权威文档并**触发新一轮 CI**（run #40）作为裁决；run #40 的裁决由**下一片**回填；
3. 若下一片仍拿不到 run #39 的日志，**唯一剩下的取证路径**是：owner 提供已登录的 GitHub 会话（右侧浏览器）或安装 `gh`/提供 token。

## 6'. 固件条数

固件 **163 → 166** 条（**166 passed / 0 failed**）。

## 7. 诚实性

### 7.1 本片**闭合**了什么、**没有**闭合什么

- **闭合**：T2 族的**形状面** —— 「`sico.text.length` 的实参本身是嵌套调用」在 guest 前端**逐字节降**（7 个形状），并且在**发射端认识内层算子**的 4 个形状上**整组件逐字节等同**。
- **没有闭合**：
  - **T2 族整体不得宣告闭合**（**D10**）：S 内两个真实调用点的**宿主函数**仍不降（§4.4）；
  - **组件层 3/7** 仍被**发射端**拒（内层算子 `sico.bytes.utf8_decode` = **T11 族**）；
  - **比较实参路由**（`if` / `while` 条件里的嵌套实参）仍 typed 拒（S 内无此形状）；
  - **语料覆盖仍 0 条**（S 内 322 个 `sico.text.length(` 调用点一个都没进差分语料）；
  - **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。

### 7.2 缺口登记（承自本片）

| 缺口 | 症状 | 根因 | 归属 |
|---|---|---|---|
| **T2-g2** 实参本身是嵌套调用 | ~~前端 typed 拒 `ERR:E-SH-IR-EXPRESSION`~~ | —— | ✅ **本片闭合**（前端 7/7 逐字节等同：主测 5 个 + 补测 STEP-0529 的 2 个 `let` 绑定形） |
| **T2-c2** 嵌套内层算子 `sico.bytes.utf8_decode` 的发射端缺口 | 组件层 typed 拒 `unsupported codegen source shape` | guest 发射器只认 3 个算子 | ⏳ **T11 族**（不是 T2） |
| **G-M1** 匹配路由的形参数目闸门 | S 站点 ② ⇒ `ERR:E-SH-IR-MATCH-SHAPE` | `checked_arithmetic_match_ir` 要求恰好 2 个形参 | ⏳ **匹配路由**（不是 T2） |
| **G-M2** 固定算子实参路径不接受 `sico.*` 实参 | S 站点 ② 的第二道闸门 ⇒ `ERR:E-SH-IR-CALL-ARGUMENT`（**对照形状：非嵌套实参同样被拒**） | `argument_segment_kind` 只认形参 / 字面量 / 用户调用 | ⏳ **匹配路由**（不是 T2） |
| **G-T7a** `sico.list.append(<局部>, <嵌套调用>)` | S 站点 ① ⇒ `ERR:E-SH-IR-CALL-SHAPE` | 通用调用路由的实参分支不认嵌套调用 | ⏳ **T7 族**（不是 T2） |

### 7.3 下一刀

**T2 族收口片 X**（**验证片**）：钉住 + 纠偏 + 前沿探针期望值随之前移 + 固件电池 + 独立 CI 回填 + **诚实收窄段**。
收口片必须显式写清：**T2 的形状面已闭合**，但**两个真实调用点的宿主函数仍不降**（归因见 §4.4）、**组件层 3/7 属 T11**、**语料覆盖仍 0 条** ⇒ **族级出口判据的「整组件逐字节等同」一项要按「内层算子未知即分族」的口径写**，不得把 T11 的缺口记进 T2 的账，也不得因此把 T2 读成整体闭合。

其后是 **T2 族之后的下一族**（计划 §3.3.5 的 **T3 = `sico.text.starts_with`**，S 内 316 处），以及**语料覆盖**与**整文件自举**两条线。
