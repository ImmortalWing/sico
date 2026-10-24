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

> **三个拒绝身份分属三个不同的门**（实测）：`ERR:E-SH-IR-MATCH-SHAPE` = checked-arithmetic 路由的**形参计数门**（**STEP-0536 已拆** ⇒ 该身份不再出现在 `checked_add` 头上；拆门后被它遮蔽的形状改报 `ERR:E-SH-IR-CALL-ARGUMENT`）；`ERR:E-SH-IR-CALL-TARGET` = 该路由的**算子白名单**（只认 `U64.checked_add`）；`ERR:E-SH-IR-STATEMENT` = **没有任何 match 路由认领这个头** ⇒ 落到通用路由的语句形状门。

## 3. 缺口登记

| 缺口 | 症状 | 根因 | S 内站点 | 归属 |
|---|---|---|---|---|
| **M-g1** 形参计数门 | `U64.checked_add` 头在 **≥3 个形参**的函数里被拒 | `checked_arithmetic_match_ir` 要求**恰好 2 个形参** | **354**（`checked_add` 396 − 2 形参 33 − 1 形参 9） | 本族（前端） | ✅ **闭合（STEP-0536）** —— 两处 guard 的结果变量**从未被读取** ⇒ 死门 ⇒ 拆门后 **396 / 396** 站点 canonical IR 逐字节等同 |
| **M-g2** 算子白名单 | `checked_sub` / `checked_div` / `checked_mul` 被拒 `ERR:E-SH-IR-CALL-TARGET` | match 路由的算子白名单只有 `U64.checked_add` | **46** | 本族（前端） |
| **M-g3** 未认领的匹配头 | `sico.bytes.slice` 被拒 `ERR:E-SH-IR-STATEMENT` | 没有任何 match 路由认领这个头 | **21** | 本族（前端） |
| **M-g4** 组件层零支持 | **全部** match 形状被发射端拒 | **STEP-0535 订正：这不是一个缺口，而是五个缺口叠加** —— 见下表 | **519**（全部） | 本族（发射端） |
| **R0.4** 真实站点复核 | 组件层只测过 **4 个合成 fixture** | 「42 / 519 形状面闭合」是**外推**，owner 审计（AGENTS.md §10.1）判定不成立 ⇒ 改为逐站复核 | **42**（候选） | 本族（证据） |
| **R0.3** canonical-IR 边界的 fail-closed | Result 投影只比**槽数**、variant 模式不校验**变体布局** | guest 的发射链把每个失败塌缩成空串，且两处闸门缺失 | **0**（不可由合法源触发） | 本族（发射端） |
| **R0.5** 布局与边界覆盖 | ok/error 投影、Result cell、布尔模式、I64 checked、U64 溢出/下溢未逐字节钉 | 覆盖缺口，不是行为缺口 | **0**（合成形状） | 本族（证据） |

**真实站点复核结果（R0.4，账本 `selfhost/match-component-coverage-v0.json`；STEP-0536 已由 `match-component-coverage-v1.json` 取代，见下）**：42 个候选站点（`U64.checked_add` 头 + 宿主函数 ≤2 形参）逐站对 oracle 跑**两条轴**（canonical IR + 组件字节，组件轴过真实 `wasmparser::Validator`）：

| verdict | 站点数 | 含义 |
|---|---|---|
| `ir-equal+component-equal` | **8** | 组件字节与生产后端**逐字节等同** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **9** | canonical IR 逐字节等同；**发射端**不认识该函数体形状（`is_ident_byte` / `type_json` / `lex_words` ×5 / `gw_instr_count` / `return_expression_start`） |
| `guest-refused:ERR:E-SH-IR-MATCH-SHAPE` | **1** | **前端就拒**：`codegen.sico:cg_next`（1 形参 + `return`/`return`）⇒ **M-g1 形参门在真实站点上复现** |
| `closure-too-large` | **24** | 同文件依赖闭包超过 64 函数 / 256 KiB（`parser.sico:scalar_ir`）⇒ IR 证据取自**整文件**差分钉，组件轴未取得 |

⇒ **组件级逐字节等到的真实站点是 8 / 42**（不是 42 / 519）。**D10 ⇒ 不得降级、不得宣告闭合**。

**STEP-0536（M-g1）之后：账本扩到全部 396 个 `U64.checked_add` 匹配头**（账本 `selfhost/match-component-coverage-v1.json`，取代 v0）。候选规则从「宿主函数 ≤2 形参」改为「任意形参数」，并**逐站记录真实形参数** ⇒ 形参门后面的 **354** 个站点第一次有了逐站证据：

| verdict | 站点数 | 含义 |
|---|---|---|
| `ir-equal+component-equal` | **9** | 组件字节与生产后端**逐字节等同**（比 v0 多 1 个 = `codegen.sico:cg_next`，**M-g1 在真实站点上兑现**） |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **168** | canonical IR 逐字节等同；**发射端**不认识该函数体形状 |
| `closure-too-large` | **219** | 同文件依赖闭包超过 64 函数 / 256 KiB ⇒ IR 证据取自**整文件**差分钉，组件轴未取得 |
| **前端就拒** | **0** | **形参门已拆 ⇒ 396 / 396 站点的 canonical IR 全部与 oracle 逐字节等同** |

⇒ **前端轴**：396 / 396 逐字节等同（v0 时是 41 / 42）；**组件轴**：9 / 396（v0 时是 8 / 42）⇒ **D10 ⇒ 族仍未闭合、不得宣告闭合**；下一杠杆 = **组件层发射**（168 站点）与 **闭包规模**（219 站点），**不是**前端。

**M-g4 的分解（STEP-0535 实测，逐个关掉才看清）**：

| 子缺口 | 症状 | 根因（实测） | 状态 |
|---|---|---|---|
| **M-g4a** 终结符 | 发射器只发 `return` / `branch` / `jump` / `unreachable` | 没有 `match` 的发射路径 | ✅ **闭合（STEP-0535）** |
| **M-g4b** 匹配头的运算 | 运算白名单里没有 `U64.checked_add` / `checked_sub` | 匹配头的值必须先能发出来 | ✅ **闭合（STEP-0535，无符号两个变体）** |
| **M-g4c** 分支载荷投影 | arm 体读载荷的 `project` 只认**命名记录** | 没有 Result 分支（`ok` / `error`）的投影 | ✅ **闭合（STEP-0535）** |
| **M-g4d** Result 扁平布局 | `Result[U64, NumericError]` 被拒 `ERR:E-SH-CG-IR-FLAT` | 生产后端 `lower_local_types` 对该形状有专门分支（`[i32 tag, i64 ok]`） | ✅ **闭合（STEP-0535）** |
| **M-g3'** 宿主调用传输 | `sico.list.get` 头的形状仍被拒 | 发射器没有该 intrinsic 的宿主调用传输 | ❌ **仍开（15 站点）** |

### 3.1 R0.3 / R0.5：canonical-IR 边界的 fail-closed 与布局覆盖

**注入夹具**（runner 测试侧，**不新增产品 CLI、不改 Rust oracle**）：把 stdin 的 canonical IR **直接**交给 `codegen.component_hex_for_ir` / `codegen.component_refusal_for_ir` ⇒ **Sico 前端无法产生的 IR 也能到达生产后端看到的同一条边界**；畸形 IR 用 `serde_json::to_string` 序列化（`canonical_json` 会先跑 `verify`）。

**guest 侧补齐的两道闸门**（`selfhost/codegen.sico`）：

| 闸门 | 生产后端合同 | guest 现状 → 现在 | 身份 |
|---|---|---|---|
| Result 投影的**完整 Wasm 类型序列** | `Operation::Project` 布局比较 `layout.types != expected` | 只比槽数 → **比完整 flat-type 序列**（新增 `cgs_result_arm_ty`） | `ERR:E-SH-CG-IR-PROJECT-TYPE` |
| variant 模式的**变体布局** | `LocalLayout::tag` 拒非变体值；`LocalLayout::scalar` 拒聚合值 | 直接读首槽 → **要求 `result` 值 / `bool` 单槽标量** | `ERR:E-SH-CG-IR-MATCH-VARIANT` / `ERR:E-SH-CG-IR-MATCH-SCALAR` |

**拒绝身份透传**：发射链此前把失败塌缩成空串 ⇒ 现改为**身份沿链透传**，并新增导出函数 `component_refusal_for_ir`（成功 `""`，失败返回第一道拒绝的身份）；`component_hex_for_ir` 的公开契约不变。

**实测（R0.3，固件 `the_malformed_canonical_ir_is_refused_by_the_guest_and_by_the_rust_oracle`）**：

| 形状 | Rust oracle | guest 身份 |
|---|---|---|
| 阳性对照（未畸形） | 发射组件 | **组件字节逐字节等同** |
| 同槽数异型投影 | 拒（verifier） | `ERR:E-SH-CG-IR-PROJECT-TYPE` |
| 非 variant 值上的 variant 模式 | 拒（verifier） | `ERR:E-SH-CG-IR-MATCH-VARIANT` |
| 非布尔值上的字面模式 | 拒（verifier） | `ERR:E-SH-CG-IR-MATCH-SCALAR` |
| 非法嵌套 payload | 拒（后端 payload inspection） | `ERR:E-SH-CG-IR-EMIT` |

**实测（R0.5，固件 `the_result_layout_and_the_checked_arithmetic_boundaries_are_pinned`）**：六个形状与生产后端**逐字节等同**（都过真实 validator）—— `ok` 投影 / `Result[u64,u64]` cell / `error` 投影 / 布尔模式 / `U64` add 溢出判定 / `U64` sub 下溢判定；**I64 checked 是 guest 的显式收窄**：oracle 发射、guest `ERR:E-SH-CG-IR-EMIT`。

> **口径**：这些覆盖是**合成形状**口径（可由前端产生的 IR + 不可由前端产生的畸形 IR），**不是**真实站点口径，也**不是**冻结语料口径 ⇒ 三种口径分别报告，不合并计数。
>
> **残留**：溢出/下溢的覆盖是**发射字节级**的，**不是**运行期行为证据（注入的 `size` 不在 `run` 的可达路径上）。

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
- **STEP-0535 R0.4 之后**：合成形状 **4 个**；**真实站点逐站复核 42 个** ⇒ **组件级全等 8 个 / IR 全等 9 个 / 前端拒 1 个 / 闭包过大 24 个**；
- **STEP-0536（M-g1）之后（当前口径）**：合成形状 **4 个**；**真实站点逐站复核 396 个**（账本 v1，候选规则 = 全部 `U64.checked_add` 匹配头，逐站记录真实形参数）⇒ **组件级全等 9 个 / IR 全等但组件被拒 168 个 / 前端拒 0 个 / 闭包过大 219 个** ⇒ **前端轴 396 / 396 逐字节等同**（形参门已拆）；
- ⇒ 组件层从 **0 个形状**推进到 **4 个合成形状 + 9 个真实站点**（**真实站点口径 9 / 396**）⇒ **M-g4 尚未闭合**（M-g3' 与组件层的分支/循环/多语句函数体发射仍开）。
- ⇒ **旧口径「组件层 42 / 519（形状面）」作废**（owner 审计 AGENTS.md §10.1）：那是**按形状归并**的外推，不是逐站点证据 ⇒ **D10 ⇒ 不得降级、不得宣告闭合**。

## 6. 族级出口判据（六项缺一不算闭合）

| 判据 | 状态 |
|---|---|
| `canonical IR` 逐字节等同 | ✅ **STEP-0536（M-g1）**：拆掉形参门后，**396 / 396** 个 `U64.checked_add` 匹配头的 canonical IR 与 oracle **逐字节等同**（**前端就拒 0 条**）；合成形状口径另计 **6** 个（a–f） |
| `整组件` 逐字节等同 | ⏳ **4** 个**合成**形状（a、b、e、f）—— **STEP-0535 的 M-g4a 刀**；**真实站点**：**9 / 396**（v0 口径是 8 / 42；新增的 1 个 = `codegen.sico:cg_next`）⇒ **168** 个站点 canonical IR 已等同但发射端拒、**219** 个站点闭包过大未测 |
| **真实站点逐站复核** | ✅ **STEP-0536**：账本 `selfhost/match-component-coverage-v1.json`（**396** 条，含 path/function/head/case-shape/**parameter_count**/verdict/digests）+ 探针 `tools/probe-m22-match-coverage.ps1` + 固件 `sico_compiler_reproduces_the_frozen_match_site_ledger`（逐站 IR + 组件 + 真实 validator）⇒ **396 / 396 有当前快照证据**（取代 v0 的 42 / 42） |
| `wasmparser::Validator` 通过 | ⏳ 前端逐字节等同的行本身两侧都过；**真实站点**：**9 / 396** 的组件两侧都过真实 validator |
| 前沿探针期望值已翻转 | ✅ STEP-0534：**65 → 71** 条；**STEP-0536（M-g1）**：**2 条翻转**（1 条 `lowered` + 1 条拒绝身份 `MATCH-SHAPE` → `CALL-ARGUMENT`）⇒ 期望表 **71 条** |
| 固件电池绿 | ✅ STEP-0535：**173 → 174**（**+1 条**：站点账本钉）+ **版本化电池** `tools/validate-m22-s5.ps1`（取代一次性 `target/tmp` 电池脚本）；**R0.3 / R0.5 再 +2 条**（畸形 IR 拒绝 + 布局/边界）⇒ **176**；**STEP-0536（M-g1）再 +1 条**（形参计数门的多形参数逐字节等同）⇒ **177** |
| **canonical-IR 边界的 fail-closed** | ✅ **R0.3**：四个畸形形状**双方 typed 拒**（Rust oracle 拒 + guest 带**可归因身份**拒），阳性对照证明夹具确实到达发射端（§3.1） |
| **独立 CI 绿** | ⏳ 本片推送后触发下一轮，**裁决由下一片回填**；上一批 **STEP-0533** 的 **run #42 = success ✅**（run id `40048806433`，01:13:31Z → 01:33:09Z，19m38s ⇒ #40 / #41 / #42 三轮连续绿）—— 见 [STEP-0534 §7.6](./steps/STEP-0534-m22-w2-s5-w2-match-truth.md) |

⇒ **本族未闭合**：`match` 构造在 guest 里**前端只覆盖 11% 的 S 形状**，**组件层真实站点只有 8 / 42 组件级全等**；**D10 ⇒ 不得降级、不得宣告闭合**。

## 7. 下一刀（按实测杠杆）

1. **组件层发射**（**168** 个站点）—— 发射端刀：这些站点的 canonical IR **已经**与 oracle 逐字节等同，但**发射端不认识它们的函数体形状**（分支 / 循环 / 多语句函数体）⇒ 组件轴缺的不是 `match` 本身，而是**真实函数体**的发射能力。**这是当前最大的杠杆**（168 > 15 > 46 > 21）。
2. **M-g3' 组件层的 `sico.list.get` 宿主调用传输**（**15** 站点）—— **发射端刀**：让发射器能发 `sico.list.get` 这类宿主调用 ⇒ 需要宿主调用传输路径。**注意（实测）**：生产后端对 `sico.list.get` **不是** import 调用，而是**内联线性内存查表序列**（`table`/`count` → `i32.const 3; i32.shl; i32.add` → 读 `ptr`/`len`），所以这一步的工作量是**内联序列**而不是「加一个 import」。
3. **M-g2 算子白名单**（**46** 站点）—— 前端刀：把 `checked_sub` / `checked_div` / `checked_mul` 纳入匹配头的白名单；其中 `checked_sub` 的**发射**本片已经具备。
4. **M-g3 未认领的匹配头**（**21** 站点）—— 前端刀：`sico.bytes.slice` 头。
5. **闭包规模**（**219** 站点）—— 组件轴**没有证据**（依赖闭包 > 64 函数 / 256 KiB）⇒ 这是「**未测**」不是「通过」；要么给比较单元一个更小的切法，要么把闭包上限登记成一条边界。
6. 之后回到 **T3 族** / **语料覆盖** / **门 3 / 6**。

> **ADR 判定（STEP-0535 实测）**：**不需要新开 ADR** —— 生产后端 `crates/sico-codegen-wasm/src/lib.rs` 的 `compile_terminator`（`Terminator::Match` 分支 `:2295`）、`emit_match_condition`（`:2325`）、`emit_checked_fixed`（`:3010`）、`apply_result_fields`（`:2193`）与 `lower_local_types`（`:2924`）**已经**冻结了逐字节的发射形状，[ADR-0015](../../adr/ADR-0015-compiler-bootstrap-closure-v0.md) 的 `A == B == C` 字节闭合与 [ADR-0016](../../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) §6 的 canary 梯级也已冻结目标 ⇒ 这是**已冻结合同的实现**，需要的是**对照字节钉**。
