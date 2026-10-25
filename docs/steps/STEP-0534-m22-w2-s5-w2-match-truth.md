# STEP-0534 m22-w2 S5 **W2 族 `match` 构造的真值测量片**（拍 1）：519 条语句的形状普查 + 三个拒绝门的归因

> - status: **本机实测绿（随本片提交并推送；独立 CI 裁决待下一批回填）**
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0533](./STEP-0533-m22-w2-s5-w2-t2-family-closure.md)（**跨族实测**：本族被定为首要杠杆）、[STEP-0527](./STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（族级真值表 + D15 的方法学先例）、[STEP-0526](./STEP-0526-m22-w2-s5-gd2-emitter-branch-support.md)（发射端分支 / 跳转 / 不可达终结符的既有实现）
> - gate: **不推进任何出口门** —— 本片是**测量片**，`selfhost/*` 与 `crates/*` **零改动** ⇒ **不包装为特性支持**
> - numbering: **STEP-0534**（取号前已核对 `docs/steps/`：最大号 0533；*0534 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-match.md)

## 0. 入口门槛

计划 §3.3.5 给每个族定义了「拍 1 真值测量片 → 前端刀 → codegen 刀 → 收口片 X」的次序。本片是 `match` 构造的**拍 1**：

| 入口条件 | 状态 |
|---|---|
| 族被识别为**整文件自举的关键路径** | ✅ STEP-0533 的跨族实测：`match` **519** 条语句 + **476** 个算子操作数，比 T 族剩余的 7 个站点大两个数量级 |
| 既有实现的路由已定位 | ✅ guest 前端有 **3 个 match 降低器**：`list_get_match_ir`（`selfhost/parser.sico:18853`）/ `checked_arithmetic_set_match_ir`（`:19471`）/ `checked_arithmetic_match_ir`（`:20362`）；通用路由还有 `gw_match_open_pack`（`:15586`） |
| 本片**不实现**、只测量 | ✅ `selfhost/*` 与 `crates/*` 零改动 |

## 1. 这一片回答的问题

**S 里的 `match` 语句，guest 到底能降多少？哪些形状不能降，为什么不能降？**

**为什么必须先答**：整文件自举的固定点要求 guest 能编译**整个**冻结源集 S。S 里有 **519** 条 `match` 语句；在本片之前，没有任何一份证据说明这些语句能不能降、或者卡在哪一道门上。

## 2. 普查：S 内 519 条 `match` 语句

（脚本 `target/tmp/step0534/census_match.py`，口径 = 文本计数）

| 口径 | 数值 |
|---|---|
| `match` 语句总数 | **519**（顶层 **478** + 嵌套在别的 case 体内 **41**） |
| 按文件 | `parser.sico` **428** / `codegen.sico` **14** / `compiler.sico` **13** / `formatter.sico` **9** + 其余零星 |
| **匹配头算子** | `U64.checked_add` **396** / `U64.checked_sub` **36** / `sico.bytes.slice` **21** / `sico.list.get` 系列 **15** / `U64.checked_mul` **7** / `U64.checked_div` **3** / `sico.text.length` **2**（T2 族） |
| **宿主函数形参数** | 1 个 **34** / 2 个 **53** / 3 个 **21** / 4–8 个 **50** / 9 个 **103** / 10 个 **16** / 11 个 **51** / **12 个 114** / 13 个 **20** / 14 个 **7** / 17 个 **9** |
| **case 体形状** | `set`+`return` **385** / `return`+`return` **30** / `set`+`set` **16** / **嵌套 `match`** **12+** / 带 `if` 的复杂体 **≤ 8** |

**两处普查口径订正**（写进登记表）：

1. STEP-0533 记的「`match` 语句 519」与本片普查的「478」不矛盾：**519 是全部 `match` 语句**（线性扫描），**478 是顶层**（41 条嵌套在别的 case 体内）。
2. 首版普查把「4 个 case」记成一种形状 —— 那是**脚本缺陷**（把嵌套 `match` 的 case 算进了父级）。**真实形状一律两个 case**（`case ok(x):` + `case error(_):`）⇒ **本族的形状空间比首版普查小得多**。

## 3. 真值测量：12 个形状（前端 + 组件两侧都测）

（脚本 `target/tmp/step0534/match_shapes.py`；oracle = Rust 前端的 canonical IR 与 Rust 后端的生产组件；guest = guest 前端与 guest 发射器）

| # | 形状 | IR | guest 前端 | guest 组件 |
|---|---|---|---|---|
| a | `U64.checked_add` · 2 形参 · `set`/`return` | 4511 | ✅ **逐字节等同** | ❌ 发射端拒 |
| b | `U64.checked_add` · 1 形参 · `set`/`return` | 4438 | ✅ **逐字节等同** | ❌ 发射端拒 |
| c | `sico.list.get(words, i)` · 2 形参 | 3615 | ✅ **逐字节等同** | ❌ 发射端拒 |
| d | `sico.list.get[U64](values, idx)` · **3 形参** | 3577 | ✅ **逐字节等同** | ❌ 发射端拒 |
| e | **嵌套 `match`** | 6342 | ✅ **逐字节等同** | ❌ 发射端拒 |
| f | `U64.checked_add` · 2 形参 · `return`/`return` | 3547 | ✅ **逐字节等同** | ❌ 发射端拒 |
| g | `U64.checked_add` · **3 形参** | 4585 | ❌ `ERR:E-SH-IR-MATCH-SHAPE` | ❌ 发射端拒 |
| h | `U64.checked_add` · **12 形参** | 5247 | ❌ `ERR:E-SH-IR-MATCH-SHAPE` | ❌ 发射端拒 |
| i | `U64.checked_sub` · 2 形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 发射端拒 |
| j | `U64.checked_div` · 2 形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 发射端拒 |
| k | `U64.checked_mul` · 2 形参 | 4511 | ❌ `ERR:E-SH-IR-CALL-TARGET` | ❌ 发射端拒 |
| l | `sico.bytes.slice` · 3 形参 | 4555 | ❌ `ERR:E-SH-IR-STATEMENT` | ❌ 发射端拒 |

⇒ **前端逐字节等同 6 行**；**前端 typed 拒绝 6 行**（**三个不同的身份**）；**组件层 typed 拒绝 12 行**。

> **嵌套形状的预算边界（实测，写进登记表）**：行 e（嵌套 `match`）在 **runner 的 800 B fuel 预算**下逐字节等同（6342 B，固件钉住），但在 **CLI 的默认 fuel 预算**（10 M）下**耗尽预算**（`resource-limit.fuel` / `FuelExhausted`）⇒ **这不是形状边界，而是预算边界** ⇒ **前沿探针量不了它**（探针走 CLI 路径，且 CLI 没有 `--fuel` 选项）⇒ 该形状**只由固件钉住**，探针里不列。更小的嵌套变体（去掉 `let` + `set`）改被拒 `ERR:E-SH-IR-UNRESOLVED` ⇒ **嵌套路由要的是 `let` + `set` 那个形状**。

## 4. 归因：**三个拒绝身份 = 三个不同的门**

| 身份 | 门 | 落点 | S 内站点 |
|---|---|---|---|
| `ERR:E-SH-IR-MATCH-SHAPE` | **形参计数门**：`checked_arithmetic_match_ir` 要求**恰好 2 个形参**（`:20364`） | `selfhost/parser.sico:20362` | **354** |
| `ERR:E-SH-IR-CALL-TARGET` | **算子白名单**：该路由只认 `U64.checked_add` | 同上 | **46** |
| `ERR:E-SH-IR-STATEMENT` | **没有任何 match 路由认领这个头** ⇒ 落到通用路由的语句形状门 | `gw_rhs_packed` / 通用路由 | **21** |

**两个「看起来矛盾、其实不是」的实测结果**（写进登记表，避免下一片误判）：

- **`U64.checked_add` 在 1 个形参的函数里能降、在 3 个形参的函数里被拒** ⇒ 存在**两条**不同的路由：带 `set` 的 case 体走 `checked_arithmetic_set_match_ir`（**无形参门**），`return`/`return` 的 case 体走 `checked_arithmetic_match_ir`（**有 2 形参门**）。
- **`sico.list.get` 在 3 个形参的函数里能降** ⇒ `list_get_match_ir` **没有形参门** ⇒ **形参计数门不是 `match` 构造的性质，是某一条路由的性质**。

## 5. 今天 guest 能降多少（把普查与真值合起来）

| 层 | 能降 | 不能降 |
|---|---|---|
| **前端** | **57 / 519（11%）**：其中 `checked_add` 且宿主 ≤2 形参 **42** + `sico.list.get` 系列 **15** | **462** |
| **组件层** | **0 / 519** | **519**（发射器没有任何 `match` 支持） |

⇒ **即便前端把三个缺口全补上，519 条 `match` 语句仍然一条都发不出组件** ⇒ **组件层的 `match` 发射路径是本族收口前的硬门槛**（缺口 **M-g4**）。

## 6. 出口测试（钉）

本片**新增 1 条钉**（固件 **170 → 171**）：`the_match_construct_truth_is_measured_and_pinned` —— 钉住 6 个逐字节等同形状的 IR 字节数（4511 / 4438 / 3615 / 3577 / 6342 / 3547 B）、6 个拒绝形状的 IR 字节数与**三个 typed 身份**、以及**组件层对三个代表性 `match` 形状**（`checked_add` 2 形参 / `sico.list.get` / **嵌套**）都拒 `unsupported codegen source shape`（**组件层的 12 行全拒是本片脚本的实测，固件只钉这三个代表性形状**）。

**前沿探针**：**65 → 71 条**（**+6**：3 条 `lowered` 期望 + 3 条 typed 拒绝期望）。

## 7. 验证记录（2026-10-24）

### 7.1 固件

- **171 passed / 0 failed**（本片新增 1 条）。

### 7.2 前沿探针

- **71 / 71 全部与期望一致** ⇒ `FRONTIER_UNCHANGED`。
- 其中本片新增的 6 条：`match-checked-add-in-a-two-parameter-function`（`lowered`）/ `match-checked-add-in-a-one-parameter-function`（`lowered`）/ `match-list-get-generic`（`lowered`）/ `match-checked-add-in-a-three-parameter-function`（`refused:ERR:E-SH-IR-MATCH-SHAPE`）/ `match-checked-sub`（`refused:ERR:E-SH-IR-CALL-TARGET`）/ `match-bytes-slice`（`refused:ERR:E-SH-IR-STATEMENT`）。
- **嵌套 `match` 不在探针里**（预算边界，见 §3 的说明）⇒ 该形状**只由固件钉住**。

### 7.3 电池

- 本机 **BATTERY_GREEN 22/22**（**22 步** = STEP-0533 的 21 步，把 `step0533-doc-check` 换成同位置的 `step0534-doc-check`，再加 **1 步新增** `step0534-match-census`；步骤表见 §7.5）。

### 7.4 独立 CI

- 上一批 **STEP-0533**（提交 `3b88469`）的 **run #42** 裁决 = **success ✅**（见 §7.6）。
- 本片推送后触发下一轮（**run #43**），**裁决由下一片回填**（仓库既有做法：不为此再推一轮）。

### 7.5 电池步骤表（本片实测）

| 步 | 秒 | exit |
|---|---|---|
| workspace-integrity | 53 | 0 |
| fmt-runner | 1 | 0 |
| ci-one-key | 418 | 0 |
| validate-0245 | 132 | 0 |
| validate-0262 | 148 | 0 |
| probe-frontend-frontier | 475 | 0 |
| probe-ir-plan-frontier | 26 | 0 |
| validate-0124 | 1 | 0 |
| validate-0131 | 1 | 0 |
| validate-0156 | 1 | 0 |
| git-diff-check | 0 | 0 |
| line-ending-audit | 0 | 0 |
| selfhost-comment-discipline | 0 | 0 |
| step0526-footprint-audit | 0 | 0 |
| step0526-census-branches | 0 | 0 |
| step0526-differential | 85 | 0 |
| step0528-census-t2 | 0 | 0 |
| step0533-cell-census | 0 | 0 |
| step0533-operator-census | 0 | 0 |
| **step0534-match-census** | 0 | 0 |
| **step0534-doc-check** | 0 | 0 |
| workspace-integrity-final | 52 | 0 |

⇒ **BATTERY_GREEN 22/22**（本机，2026-10-24；原始日志 `target/tmp/step0534/battery.log`，gitignored）。

> `ci-one-key` 一步内含固件全量：**171 passed / 0 failed**（该固件跑了两遍：120.38s 与 118.70s）。

### 7.6 独立 CI 回填（run #42 = 上一批 STEP-0533）

| 项 | 值 |
|---|---|
| run | **#42**（id `40048806433`） |
| head_sha | `3b88469`（STEP-0533） |
| 结论 | **success ✅** |
| 用时 | 2026-10-24T01:13:31Z → 01:33:09Z（**19m38s**） |
| 连续性 | **#40 / #41 / #42 三轮连续 success** ⇒ 上一批的「独立 CI 绿」判据**连续第三轮达成** |

> **观测说明**：run #42 在观测窗口内一度显示 `in_progress` 且 `updated_at` 长时间不动（疑似排队）⇒ **最终完成且为绿**。⇒ 判定「CI 长时间 in_progress」时**必须以最终态为准**，不得据此登记红灯。

## 8. 诚实性

### 8.1 本片**闭合**了什么

- **把 `match` 构造从「一个数字」变成一张可施工的表**：519 条语句、8 个匹配头算子、宿主形参数分布、case 体形状、6 个逐字节等同的前端单元格、6 个拒绝单元格（**三个门**）、组件层**零**支持。
- **两处普查口径订正**（478 vs 519 的嵌套；「4 个 case」是脚本缺陷）。
- **两个反直觉结论被实测钉住**：1 形参能降 / 3 形参被降（两条不同路由）、`sico.list.get` 无形参门 ⇒ **形参门是路由的性质，不是构造的性质**。
- **落点定位**：三个门分别落在 `checked_arithmetic_match_ir`（`:20364` 的形参门）、同一路由的算子白名单、以及通用路由的语句形状门。

### 8.2 本片**没有**闭合什么（**诚实收窄**）

| 项 | 状态 |
|---|---|
| **`match` 构造的族闭合** | ❌ **NO-GO** —— 前端只覆盖 **57 / 519（11%）**，组件层 **0 / 519** ⇒ **D10 ⇒ 不得降级、不得宣告闭合** |
| **本片性质** | ⏔ **纯测量**：`selfhost/*` 与 `crates/*` **零改动**，**不推进任何出口门**、**不包装为特性支持** |
| **语料覆盖** | ⏔ **仍 0 条** —— 519 条 `match` 一条都没进语料 |
| **整文件自举** | ⏔ 仍未关闭；本片把它的**首要阻塞点**量化到了站点级 |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；门 1 / 4 / 5 / 7 GO；门 2 PARTIAL；**M22 整体仍 NO-GO** |

### 8.3 下一刀（按实测杠杆排序）

1. **M-g4 组件层 `match` 发射路径**（**519** 站点）—— 本族**最大**的一块，也是收口前的硬门槛。**ADR 判定（已用证据收敛）**：发射形状**不是新架构**，而是**已冻结合同的实现** ——（a）生产后端 `crates/sico-codegen-wasm/src/lib.rs` 的 `compile_terminator`（`Terminator::Match` 分支，`:2295`）与 `emit_match_condition`（`:2325`）**已经**定义了逐字节的发射形状（dispatcher 局部 + 逐 arm 的 `if` 链 + `variant_tags` 表）；（b）[ADR-0015](../adr/ADR-0015-compiler-bootstrap-closure-v0.md) 的 `A == B == C` Component 字节闭合与 [ADR-0016](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) §6 的 canary 梯级（scalar IR → `formatter.sico` 组件级 → 语料组件级 → 完整 bundle 组件级）**已冻结**目标形状 ⇒ **不需要新开 ADR**，需要的是**对照字节钉**（guest 的 match 组件字节 ≡ 生产后端字节）。
2. **M-g1 形参计数门**（**354** 站点）—— 前端刀：放宽 `checked_arithmetic_match_ir` 的「恰好 2 个形参」。
3. **M-g2 算子白名单**（**46** 站点）—— 前端刀：`checked_sub` / `checked_div` / `checked_mul`。
4. **M-g3 未认领的匹配头**（**21** 站点）—— 前端刀：`sico.bytes.slice`。
5. 之后回到 **T3 族**（`sico.text.starts_with`，316 处）、**语料覆盖**、**门 3 / 6**。

> **一条方法学结论**：**「某形状被拒绝」≠「该形状没实现」**（承 STEP-0521）在本族再次成立 —— `match` 的三个拒绝身份分属三个不同的门，若不逐条归因，很容易把「形参门」误当成「`match` 构造不支持」。
