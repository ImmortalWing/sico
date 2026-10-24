# M22 支持面登记表 · `match` 构造（语言构造族，非内建算子族）

> - status: **STEP-0535 落地（`match` 构造族的组件层发射刀 M-g4a + owner 审计 R0 证据整改）—— 族未闭合**：S 内 **519** 条 `match` 语句（实测）；**guest 前端只降 57 条（11%）**；**组件层 4 个合成形状逐字节等同**；**42 个候选站点已逐站复核**（owner 审计 R0.4）：**8 个组件级全等 / 9 个 IR 全等但组件被发射端拒 / 1 个前端就拒 / 24 个闭包过大**；**语料覆盖 0 条**
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**（族 × 子集边界 / 语料覆盖 / 未覆盖形状）—— 本族是**语言构造**（`match` 语句），不是内建算子族（T1–T20）；STEP-0533 的跨族实测把它定为整文件自举的最大阻塞点
> - depends: [STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（T1 族收口：族级真值表 + D15 的方法学先例）、[STEP-0533](../steps/STEP-0533-m22-w2-s5-w2-t2-family-closure.md)（**跨族实测**：本族被定为首要杠杆）、[STEP-0526](../steps/STEP-0526-m22-w2-s5-gd2-emitter-branch-support.md)（发射端分支 / 跳转 / 不可达终结符的既有实现）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`；Rust 后端 `compile_script_program`），**不是设计值**；**真实 S 站点、合成形状、冻结语料是三种不同覆盖口径，分别报告，不得互相替代**（owner 审计 AGENTS.md §10.2）

## 0. 为什么单独立族

STEP-0533 为了给 T2 的拒绝归因，顺带量了两个族的覆盖面：

- **`match` 语句 519 条**（S 内），外加 **476 个 `U64.checked_add` / `checked_sub` / `checked_div` 操作数** —— 其中 419 个 `checked_add` **全部**在 `match` 头里；
- **`U64.less_than` 990 处**（if 条件 659 + while 条件 327）—— 发射端只认 `U64.equal`。

⇒ **`match` 是整文件自举最大的单点阻塞**（519 条语句 + 476 个算子操作数），比 T2 族剩余的 7 个站点大两个数量级 ⇒ **本族排在 T3 之前**。

## 1. 子集边界（D15 要求登记的六项）

| 项 | 值 | 证据 |
|---|---|---|
| 支持的形状 | **一个 `match` 语句**，**恰好两个 case**（`case ok(x):` + `case error(_):`） | STEP-0534 普查：S 内 **每一条** `match` 都是两个 case（先前普查出现的「4 个 case」是脚本把**嵌套** `match` 的 case 算进了父级 —— **已订正**） |
| 支持的匹配头 | **`U64.checked_add`** 与 **`sico.list.get`（含 `[U64]` / `[Text,U64]` 泛型形式）** | STEP-0534 实测：`U64.checked_sub` / `checked_div` / `checked_mul` / `sico.bytes.slice` 全部被拒（身份见 §3） |
| 宿主函数形参数 | **`U64.checked_add` 头：恰好 1 或 2 个形参**；**`sico.list.get` 头：无此限制**（实测 3 个形参也降） | STEP-0534 实测：3 个 / 12 个形参 ⇒ `ERR:E-SH-IR-MATCH-SHAPE` |
| case 体形状 | **`set` + `return`**（385 条 S 站点）或 **`return` + `return`**（30 条）；**嵌套 `match`** 也可 | STEP-0534 实测：嵌套形状 **6342 B 逐字节等同** |
| 语料覆盖 | **0 条** | S 内 519 条 `match` **一条都没进** `selfhost/corpus-v0.json` / `corpus-w1` |
| 未覆盖形状 | **4 类**（**M-g1 / M-g2 / M-g3 / M-g4**，见 §3） | §4 普查 |

## 2. 族级真值表（实测，固件钉住）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。

| # | 形状 | IR | guest 前端 | guest 组件 | 组件字节 |
|---|---|---|---|---|---|
| a | `U64.checked_add` · 2 个形参 · `set`/`return` | 4511 | ✅ **逐字节等同** | ✅ **逐字节等同** | **1215 B** |
| b | `U64.checked_add` · 1 个形参 · `set`/`return` | 4438 | ✅ **逐字节等同** | ✅ **逐字节等同** | **1214 B** |
| c | `sico.list.get(words, i)` · 2 个形参 · `return`/`return` | 3615 | ✅ **逐字节等同** | ❌ 仍拒（**M-g3'**） | — |
| d | `sico.list.get[U64](values, idx)` · **3 个形参** · `return`/`return` | 3577 | ✅ **逐字节等同** | ❌ 仍拒（**M-g3'**） | — |
| e | **嵌套 `match`**（`checked_add` 外套 `checked_sub`） | 6342 | ✅ **逐字节等同** | ✅ **逐字节等同** | **1389 B** |
| f | `U64.checked_add` · 2 个形参 · `return`/`return` | 3547 | ✅ **逐字节等同** | ✅ **逐字节等同** | **1167 B** |
| g | `U64.checked_add` · **3 个形参** | 4585 | ❌ `ERR:E-SH-IR-MATCH-SHAPE` | ❌ 仍拒 | — |
| h | `U64.checked_add` · **12 个形参**（S 里最常见） | 5247 | ❌ `ERR:E-SH-IR-MATCH-SHAPE` | ❌ 仍拒 | — |
| i | `U64.checked_sub` · 2 个形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 仍拒 | — |
| j | `U64.checked_div` · 2 个形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 仍拒 | — |
| k | `U64.checked_mul` · 2 个形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 仍拒 | — |
| l | `sico.bytes.slice` · 3 个形参 | 4555 | ❌ `ERR:E-SH-IR-STATEMENT` | ❌ 仍拒 | — |

⇒ **前端逐字节等同 6 行**（a–f）；**前端 typed 拒绝 6 行**（g–l，**三个不同的拒绝身份**）；**组件层逐字节等同 4 行**（a、b、e、f —— **STEP-0535 的 M-g4a 刀**）；**组件层仍拒 8 行**（c、d 因 **M-g3'**；g–l 因前端就拒）。

> **行 e（嵌套 `match`）的预算边界**：它在 **runner 的 800 B fuel 预算**下逐字节等同（6342 B，固件钉住），但在 **CLI 的默认 fuel 预算**（10 M）下**耗尽预算**（`resource-limit.fuel`）⇒ **预算边界，不是形状边界** ⇒ **前沿探针量不了它**（探针走 CLI 路径，CLI 无 `--fuel` 选项）⇒ 该形状**只由固件钉住**。更小的嵌套变体（去掉 `let` + `set`）被拒 `ERR:E-SH-IR-UNRESOLVED` ⇒ **嵌套路由要的是 `let` + `set` 那个形状**。

> **三个拒绝身份分属三个不同的门**（实测）：`ERR:E-SH-IR-MATCH-SHAPE` = checked-arithmetic 路由的**形参计数门**；`ERR:E-SH-IR-CALL-TARGET` = 该路由的**算子白名单**（只认 `U64.checked_add`）；`ERR:E-SH-IR-STATEMENT` = **没有任何 match 路由认领这个头** ⇒ 落到通用路由的语句形状门。

## 3. 缺口登记

| 缺口 | 症状 | 根因 | S 内站点 | 归属 |
|---|---|---|---|---|
| **M-g1** 形参计数门 | `U64.checked_add` 头在 **≥3 个形参**的函数里被拒 | `checked_arithmetic_match_ir` 要求**恰好 2 个形参** | **354**（`checked_add` 396 − 2 形参 33 − 1 形参 9） | 本族（前端） |
| **M-g2** 算子白名单 | `checked_sub` / `checked_div` / `checked_mul` 被拒 `ERR:E-SH-IR-CALL-TARGET` | match 路由的算子白名单只有 `U64.checked_add` | **46** | 本族（前端） |
| **M-g3** 未认领的匹配头 | `sico.bytes.slice` 被拒 `ERR:E-SH-IR-STATEMENT` | 没有任何 match 路由认领这个头 | **21** | 本族（前端） |
| **M-g4** 组件层零支持 | **全部** match 形状被发射端拒 | **STEP-0535 订正：这不是一个缺口，而是五个缺口叠加** —— 见下表 | **519**（全部） | 本族（发射端） |
| **R0.4** 真实站点复核 | 组件层只测过 **4 个合成 fixture** | 「42 / 519 形状面闭合」是**外推**，owner 审计（AGENTS.md §10.1）判定不成立 ⇒ 改为逐站复核 | **42**（候选） | 本族（证据） |

**真实站点复核结果（R0.4，账本 `selfhost/match-component-coverage-v0.json`）**：42 个候选站点（`U64.checked_add` 头 + 宿主函数 ≤2 形参）逐站对 oracle 跑**两条轴**（canonical IR + 组件字节，组件轴过真实 `wasmparser::Validator`）：

| verdict | 站点数 | 含义 |
|---|---|---|
| `ir-equal+component-equal` | **8** | 组件字节与生产后端**逐字节等同** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **9** | canonical IR 逐字节等同；**发射端**不认识该函数体形状（`is_ident_byte` / `type_json` / `lex_words` ×5 / `gw_instr_count` / `return_expression_start`） |
| `guest-refused:ERR:E-SH-IR-MATCH-SHAPE` | **1** | **前端就拒**：`codegen.sico:cg_next`（1 形参 + `return`/`return`）⇒ **M-g1 形参门在真实站点上复现** |
| `closure-too-large` | **24** | 同文件依赖闭包超过 64 函数 / 256 KiB（`parser.sico:scalar_ir`）⇒ IR 证据取自**整文件**差分钉，组件轴未取得 |

⇒ **组件级逐字节等到的真实站点是 8 / 42**（不是 42 / 519）。**D10 ⇒ 不得降级、不得宣告闭合**。

**M-g4 的分解（STEP-0535 实测，逐个关掉才看清）**：

| 子缺口 | 症状 | 根因（实测） | 状态 |
|---|---|---|---|
| **M-g4a** 终结符 | 发射器只发 `return` / `branch` / `jump` / `unreachable` | 没有 `match` 的发射路径 | ✅ **闭合（STEP-0535）** |
| **M-g4b** 匹配头的运算 | 运算白名单里没有 `U64.checked_add` / `checked_sub` | 匹配头的值必须先能发出来 | ✅ **闭合（STEP-0535，无符号两个变体）** |
| **M-g4c** 分支载荷投影 | arm 体读载荷的 `project` 只认**命名记录** | 没有 Result 分支（`ok` / `error`）的投影 | ✅ **闭合（STEP-0535）** |
| **M-g4d** Result 扁平布局 | `Result[U64, NumericError]` 被拒 `ERR:E-SH-CG-IR-FLAT` | 生产后端 `lower_local_types` 对该形状有专门分支（`[i32 tag, i64 ok]`） | ✅ **闭合（STEP-0535）** |
| **M-g3'** 宿主调用传输 | `sico.list.get` 头的形状仍被拒 | 发射器没有该 intrinsic 的宿主调用传输 | ❌ **仍开（15 站点）** |

## 4. S 内 `match` 语句普查（519 条，口径：文本计数）

| 口径 | 数值 |
|---|---|
| `match` 语句总数 | **519**（其中 **478** 条是顶层，**41** 条嵌套在别的 `match` 的 case 体内） |
| 按文件 | `parser.sico` **428** / `codegen.sico` **14** / `compiler.sico` **13** / `formatter.sico` **9**（+ `compiler_semantics.sico` / `compiler_lexer.sico` / `compiler_parser.sico` 少量） |
| **匹配头算子** | `U64.checked_add` **396** / `U64.checked_sub` **36** / `sico.bytes.slice` **21** / `sico.list.get` 系列 **15** / `U64.checked_mul` **7** / `U64.checked_div` **3** / `sico.text.length` **2**（T2 族） |
| **宿主函数形参数** | 1 个 **34** / 2 个 **53** / 3 个 **21** / 4–8 个 **50** / **9 个 103** / 10 个 **16** / **11 个 51** / **12 个 114** / 13 个 **20** / 14 个 **7** / 17 个 **9** |
| **case 体形状** | `set` + `return` **385** / `return` + `return` **30** / `set` + `set` **16** / **嵌套 `match`** **12+** / 其余为带 `if` 的复杂体（**≤ 8**） |

**今天 guest 前端能降多少**：

- `U64.checked_add` 且宿主函数 ≤2 个形参：**42** 条（1 形参 9 + 2 形参 33）；
- `sico.list.get` 系列：**15** 条（无形参门）；
- ⇒ **57 / 519 = 11%**；其余 **462** 条 typed 拒。

**组件层：0 / 519**（STEP-0534 口径，已过时）→ **当前口径见 §5**：4 个合成形状逐字节等同，真实站点 **8 / 42** 组件级全等。

## 5. 组件层真值

- **STEP-0535 之前**：发射端对**每一个** `match` 形状都拒 `unsupported codegen source shape`；
- **根因（STEP-0534 的口径已订正）**：不是「发射端没有 `match` 支持」这**一个**缺口，而是 **M-g4a..M-g4d 四个缺口 + M-g3' 一个缺口**叠加（§3 的分解表）；
- **STEP-0535 之后**：4 个**合成**形状的组件字节**逐字节等同**生产后端（1215 / 1214 / 1167 / 1389 B），`sico.list.get` 头仍 typed 拒；
- **STEP-0535 R0.4 之后（当前口径）**：合成形状 **4 个**；**真实站点逐站复核 42 个** ⇒ **组件级全等 8 个 / IR 全等 9 个 / 前端拒 1 个 / 闭包过大 24 个**；
- ⇒ 组件层从 **0 个形状**推进到 **4 个合成形状 + 8 个真实站点**（**真实站点口径 8 / 42**）⇒ **M-g4 尚未闭合**（M-g3' 与前端三门仍开）。
- ⇒ **旧口径「组件层 42 / 519（形状面）」作废**（owner 审计 AGENTS.md §10.1）：那是**按形状归并**的外推，不是逐站点证据 ⇒ **D10 ⇒ 不得降级、不得宣告闭合**。

## 6. 族级出口判据（六项缺一不算闭合）

| 判据 | 状态 |
|---|---|
| `canonical IR` 逐字节等同 | ⏳ **6** 个形状（a–f）—— 覆盖 S 内 **57 / 519** 条语句的形状；**真实站点**：**9 / 42** 个候选站点的 canonical IR 与 oracle 逐字节等同（`is_ident_byte` / `type_json` / `lex_words` ×5 / `gw_instr_count` / `return_expression_start`），**1 个前端就拒**，**24 个闭包过大**（IR 证据取自整文件差分钉） |
| `整组件` 逐字节等同 | ⏳ **4** 个**合成**形状（a、b、e、f）—— **STEP-0535 的 M-g4a 刀**：guest 的组件字节 ≡ 生产后端 `compile_script_program` 的组件字节；**真实站点**：**8 / 42**（`cgs_add` / `add_u64` / `add` ×4 / `next_index` / `enum_case_end`） |
| **真实站点逐站复核** | ✅ **STEP-0535 R0.4**：账本 `selfhost/match-component-coverage-v0.json`（42 条，含 path/function/head/case-shape/verdict/digests）+ 探针 `tools/probe-m22-match-coverage.ps1` + 固件 `sico_compiler_reproduces_the_frozen_match_site_ledger`（逐站 IR + 组件 + 真实 validator）⇒ **42 / 42 有当前快照证据** |
| `wasmparser::Validator` 通过 | ⏳ 前端逐字节等同的行本身两侧都过；**真实站点**：**8 / 42** 的组件两侧都过真实 validator |
| 前沿探针期望值已翻转 | ✅ STEP-0534：**65 → 71** 条（**+6**：3 条 `lowered` + 3 条 typed 拒绝期望）⇒ 实测 `FRONTIER_UNCHANGED`（71/71）；**STEP-0535 只改发射端，前沿不变** |
| 固件电池绿 | ✅ STEP-0535：**173 → 174**（**+1 条**：站点账本钉）+ **版本化电池** `tools/validate-m22-s5.ps1`（取代一次性 `target/tmp` 电池脚本） |
| **独立 CI 绿** | ⏳ 本片推送后触发下一轮，**裁决由下一片回填**；上一批 **STEP-0533** 的 **run #42 = success ✅**（run id `40048806433`，01:13:31Z → 01:33:09Z，19m38s ⇒ #40 / #41 / #42 三轮连续绿）—— 见 [STEP-0534 §7.6](./steps/STEP-0534-m22-w2-s5-w2-match-truth.md) |

⇒ **本族未闭合**：`match` 构造在 guest 里**前端只覆盖 11% 的 S 形状**，**组件层真实站点只有 8 / 42 组件级全等**；**D10 ⇒ 不得降级、不得宣告闭合**。

## 7. 下一刀（按实测杠杆）

1. **M-g1 形参计数门**（**354** 站点）—— 前端刀：让 `checked_arithmetic_*` 路由不再要求「恰好 2 个形参」。**R0.4 已在真实站点上复现**：`codegen.sico:cg_next`（1 形参 + `return`/`return`）被 `ERR:E-SH-IR-MATCH-SHAPE` 拒 ⇒ 这不是理论缺口。**落点已定位到两处**：`selfhost/parser.sico:19473-19479`（`set` 路由，只允许 1..2 形参）与 `selfhost/parser.sico:20364`（`return` 路由，只允许恰好 2）。
2. **M-g3' 组件层的 `sico.list.get` 宿主调用传输**（**15** 站点）—— **发射端刀**：让发射器能发 `sico.list.get` 这类宿主调用 ⇒ 需要宿主调用传输路径。**这是组件层剩下的唯一缺口**。**注意（实测）**：生产后端对 `sico.list.get` **不是** import 调用，而是**内联线性内存查表序列**（`table`/`count` → `i32.const 3; i32.shl; i32.add` → 读 `ptr`/`len`），所以这一步的工作量是**内联序列**而不是「加一个 import」。
3. **发射端对真实函数体形状的覆盖**（**R0.4 新登记，9 个真实站点**）—— `is_ident_byte` / `type_json` / `lex_words` ×5 / `gw_instr_count` / `return_expression_start` 的 canonical IR 已逐字节等同，但**发射端不认识这些函数体**（`unsupported codegen source shape`）⇒ 组件轴要补的是**分支 / 循环 / 多语句函数体**的发射能力，不是 `match` 本身。
4. **M-g2 算子白名单**（**46** 站点）—— 前端刀：把 `checked_sub` / `checked_div` / `checked_mul` 纳入匹配头的白名单；其中 `checked_sub` 的**发射**本片已经具备。
5. **M-g3 未认领的匹配头**（**21** 站点）—— 前端刀：`sico.bytes.slice` 头。
6. 之后回到 **T3 族** / **语料覆盖** / **门 3 / 6**。

> **ADR 判定（STEP-0535 实测）**：**不需要新开 ADR** —— 生产后端 `crates/sico-codegen-wasm/src/lib.rs` 的 `compile_terminator`（`Terminator::Match` 分支 `:2295`）、`emit_match_condition`（`:2325`）、`emit_checked_fixed`（`:3010`）、`apply_result_fields`（`:2193`）与 `lower_local_types`（`:2924`）**已经**冻结了逐字节的发射形状，[ADR-0015](../../adr/ADR-0015-compiler-bootstrap-closure-v0.md) 的 `A == B == C` 字节闭合与 [ADR-0016](../../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) §6 的 canary 梯级也已冻结目标 ⇒ 这是**已冻结合同的实现**，需要的是**对照字节钉**。
