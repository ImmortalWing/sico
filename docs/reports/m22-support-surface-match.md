# M22 支持面登记表 · `match` 构造（语言构造族，非内建算子族）

> - status: **STEP-0534 落地（`match` 构造族的真值测量片 = 拍 1；族未闭合）** —— S 内 **519** 条 `match` 语句（实测）：**guest 前端只降 57 条（11%）**，**组件层 0 条**（发射端没有任何 `match` 支持）；已测的 3 个前端可用单元格逐字节等同、6 个拒绝单元格各有自己的 typed 身份；**语料覆盖 0 条**
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**（族 × 子集边界 / 语料覆盖 / 未覆盖形状）—— 本族是**语言构造**（`match` 语句），不是内建算子族（T1–T20）；STEP-0533 的跨族实测把它定为整文件自举的最大阻塞点
> - depends: [STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（T1 族收口：族级真值表 + D15 的方法学先例）、[STEP-0533](../steps/STEP-0533-m22-w2-s5-w2-t2-family-closure.md)（**跨族实测**：本族被定为首要杠杆）、[STEP-0526](../steps/STEP-0526-m22-w2-s5-gd2-emitter-branch-support.md)（发射端分支 / 跳转 / 不可达终结符的既有实现）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`；Rust 后端 `compile_script_program`），**不是设计值**

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

| # | 形状 | IR | guest 前端 | guest 组件 |
|---|---|---|---|---|
| a | `U64.checked_add` · 2 个形参 · `set`/`return` | 4511 | ✅ **逐字节等同** | ❌ 发射端拒 |
| b | `U64.checked_add` · 1 个形参 · `set`/`return` | 4438 | ✅ **逐字节等同** | ❌ 发射端拒 |
| c | `sico.list.get(words, i)` · 2 个形参 · `return`/`return` | 3615 | ✅ **逐字节等同** | ❌ 发射端拒 |
| d | `sico.list.get[U64](values, idx)` · **3 个形参** · `return`/`return` | 3577 | ✅ **逐字节等同** | ❌ 发射端拒 |
| e | **嵌套 `match`**（`checked_add` 外套 `checked_sub`） | 6342 | ✅ **逐字节等同** | ❌ 发射端拒 |
| f | `U64.checked_add` · 2 个形参 · `return`/`return` | 3547 | ✅ **逐字节等同** | ❌ 发射端拒 |
| g | `U64.checked_add` · **3 个形参** | 4585 | ❌ `ERR:E-SH-IR-MATCH-SHAPE` | ❌ 发射端拒 |
| h | `U64.checked_add` · **12 个形参**（S 里最常见） | 5247 | ❌ `ERR:E-SH-IR-MATCH-SHAPE` | ❌ 发射端拒 |
| i | `U64.checked_sub` · 2 个形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 发射端拒 |
| j | `U64.checked_div` · 2 个形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 发射端拒 |
| k | `U64.checked_mul` · 2 个形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 发射端拒 |
| l | `sico.bytes.slice` · 3 个形参 | 4555 | ❌ `ERR:E-SH-IR-STATEMENT` | ❌ 发射端拒 |

⇒ **前端逐字节等同 6 行**（a–f）；**前端 typed 拒绝 6 行**（g–l，**三个不同的拒绝身份**）；**组件层 typed 拒绝 12 行**（**发射端没有任何 `match` 支持**）。

> **行 e（嵌套 `match`）的预算边界**：它在 **runner 的 800 B fuel 预算**下逐字节等同（6342 B，固件钉住），但在 **CLI 的默认 fuel 预算**（10 M）下**耗尽预算**（`resource-limit.fuel`）⇒ **预算边界，不是形状边界** ⇒ **前沿探针量不了它**（探针走 CLI 路径，CLI 无 `--fuel` 选项）⇒ 该形状**只由固件钉住**。更小的嵌套变体（去掉 `let` + `set`）被拒 `ERR:E-SH-IR-UNRESOLVED` ⇒ **嵌套路由要的是 `let` + `set` 那个形状**。

> **三个拒绝身份分属三个不同的门**（实测）：`ERR:E-SH-IR-MATCH-SHAPE` = checked-arithmetic 路由的**形参计数门**；`ERR:E-SH-IR-CALL-TARGET` = 该路由的**算子白名单**（只认 `U64.checked_add`）；`ERR:E-SH-IR-STATEMENT` = **没有任何 match 路由认领这个头** ⇒ 落到通用路由的语句形状门。

## 3. 缺口登记

| 缺口 | 症状 | 根因 | S 内站点 | 归属 |
|---|---|---|---|---|
| **M-g1** 形参计数门 | `U64.checked_add` 头在 **≥3 个形参**的函数里被拒 | `checked_arithmetic_match_ir` 要求**恰好 2 个形参** | **354**（`checked_add` 396 − 2 形参 33 − 1 形参 9） | 本族（前端） |
| **M-g2** 算子白名单 | `checked_sub` / `checked_div` / `checked_mul` 被拒 `ERR:E-SH-IR-CALL-TARGET` | match 路由的算子白名单只有 `U64.checked_add` | **46** | 本族（前端） |
| **M-g3** 未认领的匹配头 | `sico.bytes.slice` 被拒 `ERR:E-SH-IR-STATEMENT` | 没有任何 match 路由认领这个头 | **21** | 本族（前端） |
| **M-g4** 组件层零支持 | **全部** match 形状被发射端拒 | guest 发射器只有 `return` / `branch` / `jump` / `unreachable` 四种终结符，**没有 `match` 的发射路径** | **519**（全部） | 本族（发射端） |

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

**组件层：0 / 519**。

## 5. 组件层真值

- 发射端对**每一个** `match` 形状都拒 `unsupported codegen source shape`（实测 3 个形状：checked_add / list.get / 嵌套）；
- 根因 = guest 发射器只发四种终结符（`return` / `branch` / `jump` / `unreachable`），**没有把 `match` 变成分支的发射路径**；
- ⇒ 即便前端把 §3 的三个缺口全补上，**519 条 `match` 语句仍然一条都发不出组件** ⇒ **M-g4 是本族收口前必须先过的一关**。

## 6. 族级出口判据（六项缺一不算闭合）

| 判据 | 状态 |
|---|---|
| `canonical IR` 逐字节等同 | ⏳ **6** 个形状（a–f）—— 覆盖 S 内 **57 / 519** 条语句的形状 |
| `整组件` 逐字节等同 | ❌ **0** 个形状 —— 发射端没有任何 `match` 支持（**M-g4**） |
| `wasmparser::Validator` 通过 | ⏳ 前端逐字节等同的行本身两侧都过（组件层无形状可比） |
| 前沿探针期望值已翻转 | ✅ STEP-0534：**65 → 71** 条（**+6**：3 条 `lowered` + 3 条 typed 拒绝期望）⇒ 实测 `FRONTIER_UNCHANGED`（71/71） |
| 固件电池绿 | ✅ STEP-0534：固件 **171 / 171**（**+1 条**新钉）+ 本机 **BATTERY_GREEN 22/22** |
| **独立 CI 绿** | ⏳ 本片推送后触发下一轮，**裁决由下一片回填**；上一批 **STEP-0533** 的 **run #42 = success ✅**（run id `40048806433`，01:13:31Z → 01:33:09Z，19m38s ⇒ #40 / #41 / #42 三轮连续绿）—— 见 [STEP-0534 §7.6](./steps/STEP-0534-m22-w2-s5-w2-match-truth.md) |

⇒ **本族未闭合**：`match` 构造在 guest 里**只覆盖了 11% 的 S 形状**，**组件层为零**；**D10 ⇒ 不得降级、不得宣告闭合**。

## 7. 下一刀（按实测杠杆）

1. **M-g4 组件层零支持**（**519** 站点）—— **发射端刀**：`match` → 分支 / 跳转的发射路径；这是本族**收口前的硬门槛**（前端的 462 条即使全降，组件层仍为零）。**ADR 判定**：**不需要新开 ADR** —— 生产后端 `crates/sico-codegen-wasm/src/lib.rs` 的 `compile_terminator`（`Terminator::Match` 分支 `:2295`）与 `emit_match_condition`（`:2325`）**已经**冻结了逐字节的发射形状，[ADR-0015](../../adr/ADR-0015-compiler-bootstrap-closure-v0.md) 的 `A == B == C` 字节闭合与 [ADR-0016](../../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) §6 的 canary 梯级也已冻结目标 ⇒ 这是**已冻结合同的实现**，需要的是**对照字节钉**。
2. **M-g1 形参计数门**（354 站点）—— 前端刀：让 `checked_arithmetic_*` 路由不再要求「恰好 2 个形参」。**风险**：`parser.sico` 与 `codegen.sico` 是共享热点（**D9** ⇒ 不得并行开族），且这是既有路由的放宽 ⇒ 需要**对照形状**钉住边界。
3. **M-g2 算子白名单**（46 站点）—— 前端刀：把 `checked_sub` / `checked_div` / `checked_mul` 纳入匹配头的白名单。
4. **M-g3 未认领的匹配头**（21 站点）—— 前端刀：`sico.bytes.slice` 头。
5. 之后回到 **T3 族** / **语料覆盖** / **门 3 / 6**。
