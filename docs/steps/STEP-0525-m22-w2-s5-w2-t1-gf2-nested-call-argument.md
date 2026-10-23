# STEP-0525 m22-w2 S5 **W2 族 T1 的 G-f2 刀**：调用型操作数的**实参本身是嵌套调用**

> - status: **implementation complete locally**（本机电池 **BATTERY_GREEN 17/17**；固件 **150 / 150**；独立 CI 裁决待下一批回填）
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0524](./STEP-0524-m22-w2-s5-w2-t1-gf-multi-argument-call-operand.md)（G-f 闭合 + **G-f2 登记与普查**）、[STEP-0521](./STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表 + D15 登记）
> - gate: **不推进任何出口门** —— 本片是 **T1 族的前端收口前置片**（G-f2 闭合）；**T1 族仍未闭合**（**G-d2 发射端分支缺口**仍开）
> - numbering: **STEP-0525**（取号前已核对 `docs/steps/`：最大号 0524；*0525 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15，已更新）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| STEP-0524 的普查把 **G-f2**（调用型操作数的实参本身是嵌套调用）登记为**前端形状缺口**，S 内 **22** 个调用点 | ✅ STEP-0524 §5.2 |
| 真值可测：Rust 前端/后端都能降这些形状（Rust canonical IR **3635** B、Rust 生产组件 **1267** B） | ✅ 见 §2 |
| 缺口归属已判定：**前端**（不是发射端）⇒ 不适用 G-d2 的「发射器新特性」论证 | ✅ D15 §3 |
| 前置依赖已闭合：G-f（STEP-0524）、G-d1（STEP-0523）、族级真值表（STEP-0521） | ✅ |

## 1. 这一片回答的问题

**guest 能不能把「`sico.text.concat` 的某个操作数是一个调用，而**那个调用的实参又是一次调用**」降下来？**

## 2. 真值测量（生产输出）

测量口径：**Rust 后端 = `sico build --profile script-v0` 的生产组件**（与固件里的 `compile_script_program` 同一函数）；**Rust 前端 = `lower_core` + `canonical_json`**；**guest = `selfhost/compiler.sico`**（真燃料 800B，与固件同一 runner）。

fixture 取自 **S 的真实形状**（`codegen.sico:228` 等 22 个站点）与它们的最小变体：

| 形状 | Rust IR | Rust 组件 | guest（改前） | guest（改后） |
|---|---|---|---|---|
| **S 真实形状**：`helper(inner(a), "b")`（内层调用有 1 个实参） | **3635** | **1267** | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | **1267 B，逐字节相同** |
| **内层调用没有实参**：`helper(zero(), "b")`（S 内 9 个站点是这个形状） | **3559** | **1261** | **typed 拒绝** | **1261 B，逐字节相同** |
| **内层调用有 3 个实参**：`helper(inner3(a, "b", U64.literal(2)), "c")` | **4005** | **1296** | **typed 拒绝** | **1296 B，逐字节相同** |
| **内层调用返回 `U64`**：`helper(count(a), "b")`（S 内 7 个站点） | **3618** | **1250** | **typed 拒绝** | **1250 B，逐字节相同** |
| **两层嵌套**：`helper(inner(inner(a)), "b")` | **3767** | **1281** | **typed 拒绝** | **1281 B，逐字节相同** |
| （G-f 既有形状，回归）S 真实形状 / 两个形参 / 三个实参 / 两个字面量实参 | 3513 / 3036 / 3189 / 3120 | 1229 / 1185 / 1193 / 1211 | 逐字节等同 | **逐字节等同（一个字节都没变）** |
| （G-f 既有形状，回归）实参是 `I64.literal(-n)` | 3070 | 1190 | **按设计 typed 拒绝** | **仍按设计 typed 拒绝** |

⇒ 改前 5 个 G-f2 形状在前端全被拒；改后 **5 个全部逐字节等同**，既有 4 个 G-f 形状**逐字节不变**，`I64.literal(-n)` **仍按设计拒**（不静默放宽）。

**值槽规则（从 oracle 的 canonical IR 读出，不是设计）**：

1. 调用相关的值 id 是**后序分配**的 —— **先**给实参分配，**再**给调用自身的结果 id；
2. 调用指令的 range = **[被调名偏移, 闭括号偏移 + 1)**；
3. `FunctionId` 是 **1 基**（guest 的 `function_declaration_ordinal` 返回 `count + 1`）。

## 3. 落点（实施记录）

落点文件：**`selfhost/parser.sico`**（**oracle 生产代码 `crates/*` 零改动**）。文件 **22458 → 22510 行（+52）**，函数数 **175 → 175**（**不新增函数**）。diff：**+56 / −4**。

### 3.1 为什么不新增函数（**本片最关键的一次返工**）

本片**第一版**是「新增一个 helper `stc_call_operand_nested_packed`（13 参），由 `stc_call_operand_packed` 在实参分类链的最后一档分派过去」。

**它把 5 个形状都降对了**（Rust oracle 与 guest 的组件逐字节相同），**但 guest 前端降不下这个新函数** ⇒ 整文件 `parser.sico` 也跟着降不下 ⇒ **前沿反而被拉回**。

用**独立源 + stub 探针**（`target/tmp/step0525/probe_nested.py`）逐构造二分，定位到**两条独立的越界形状**：

| # | 症状 | 根因 | 证据 |
|---|---|---|---|
| A | guest 前端报 `ERR:E-SH-IR-CALL-TARGET` | 通用调用路由（`gw_call_chain_packed`）的**内建白名单**里**没有** `sico.text.concat` ⇒ 一旦某个 `sico.text.concat` 的**实参本身是一次调用**（本例 `sico.u64.to_text(U64.literal(1))`），就掉进 `CALL-TARGET` | 二分表：把 `sico.u64.to_text(U64.literal(1))` 换成局部变量 ⇒ 错误立刻变成 `ERR:E-SH-IR-RETURN-COUNT`（`bisect_nested3.py`） |
| B | 修掉 A 之后报 `ERR:E-SH-IR-RETURN-COUNT` | 通用 CFG 路由要求**函数体的最后一块是终结块**；本 helper 的最后一条语句是 `if/else`（两个分支各自 `return`）⇒ 合并块**不终结** ⇒ 路由拒 | 二分表：删掉那个 `if` 分支 ⇒ 错误从 `RETURN-COUNT` 变成别的身份（`bisect_nested2.py`） |

⇒ **修法**：把嵌套调用的逻辑**内联进 `stc_call_operand_packed`** —— 它**本来就**被 guest 逐字节降低（STEP-0524 的钉），它**有 `while`** ⇒ 走通用路由；新分支落在**已被证明可降**的位置（实参分类链的最后一档），**函数数不变**、**既有路径不动**。

内联后实测（`target/tmp/step0525/shape_probe.py`）：guest 前端 **exit 0**，guest 的 canonical IR 与 Rust oracle **逐字节相同（86,035 B）** ⇒ 「guest 与 Rust 对同一份源码给出同一个 IR」这条判据成立。

### 3.2 内联分支做了什么

1. **认nested 形状**：`scco_arg` 的下一个词是 `(` ⇒ 这是一个调用型实参；`matching_close` 定位它的闭括号（**内层的逗号不会被误切**）。
2. **找被调函数**：`function_declaration_index` 找不到 ⇒ `ERR:E-SH-IR-CALL-ARGUMENT`（**与改前同一身份**，不静默放宽）。
3. **取返回种类**：`declared_return_kind` 为空 ⇒ `ERR:E-SH-IR-CALL-TYPE`（**既有身份**，`parser.sico` 里已用它 136 处；但这条路径以前**到不了**）。
4. **0 实参的内层调用**（S 内 9 个站点）：它**只占自己的结果 id**（1 个 id）⇒ 实参值 = 当前值槽，实参占用 = 1，指令 = 一条 `call`。
5. **≥1 实参的内层调用**：**递归调用 `stc_call_operand_packed` 自己**（参数表不变，17 个实参）⇒ 内层先占自己实参的 id、再占自己的结果 id ⇒ 外层的实参值 = 内层结果 id、外层的实参占用 = 内层占用的 id 数 ⇒ **后序分配自然复合**。

### 3.3 顺带修掉的一条计数缺陷

`stc_call_operand_packed` 的 `scco_consumed` 原本统计的是「**消耗了 id 的实参个数**」，而**返回给调用方的**那个数必须是「**这个调用占用的 id 总数**」（含调用自身的结果 id）。嵌套调用出现后两者不再相等 ⇒ 改成**直接累加 id 数**（`U64.checked_add`）。

⇒ 对既有形状（实参的 advance ∈ {0, 1}）**行为完全不变** ⇒ 33 行真值表里**既有的 29 行一个字节都没变**（实测）。

## 4. 撞到并修掉的 3 条自造缺陷

| # | 症状 | 根因 | 修法 |
|---|---|---|---|
| 1 | guest 前端降不下新 helper（`ERR:E-SH-IR-CALL-TARGET`） | 通用调用路由的**内建白名单**没有 `sico.text.concat` ⇒ **内建调用的实参本身是调用**就撞墙 | 放弃「新函数」方案，把逻辑内联进**已被证明可降**的 `stc_call_operand_packed`（§3.1）⇒ 实测 guest 与 Rust **IR 逐字节相同** |
| 2 | 修掉 1 之后仍红（`ERR:E-SH-IR-RETURN-COUNT`） | 通用 CFG 路由要求**函数体最后一块终结**；helper 的尾部是 `if/else` | 同上（内联后尾部落在既有 `while` 体内） |
| 3 | 嵌套调用的 id 计数错（外层结果 id 偏移） | `scco_consumed` 统计的是**实参个数**而不是**占用的 id 数** | 改成累加 **id 数**（§3.3） |

⇒ 第 1、2 条是**同一次返工的两个面**：它们都指向同一个结论 —— **guest 前端能降的形状集合比「新写一个规整的函数」窄**，**新形状必须落在既有可降形状的延长线上**。

⇒ 这条与 STEP-0524 §4 第 2 条（臂内嵌套 `match` 不被支持）是**同一类事故**：**不是能力缺失，是形状边界**。

## 5. **S 内 G-f2 普查**（本片证据；口径写在脚本里）

普查脚本：`target/tmp/step0525/census_gf2.py`（只读，输出随电池一起跑）。口径 = **文本计数**：`selfhost/*.sico` 里 `sico.text.concat(` 的操作数，当且仅当它是**用户定义函数的调用**、且该调用的**某个实参本身又是一次用户函数调用**时，算作一个 **G-f2 调用点**。

| 指标 | 数值 |
|---|---|
| **G-f2 面（S 内）** | **22** 个调用点（`codegen.sico` **10** + `parser.sico` **12**） |
| 内层调用的**实参个数** | 0 个 **9** / 1 个 **1** / 2 个 **8** / 3 个 **4** |
| 内层被调函数的**返回种类** | `text` **15** / `u64` **7** |
| 内层实参的形态 | 形参 **13** / 局部 **12** / `U64.literal` **4** |
| **全部内层实参都落在已认领集合内**（形参 / 局部 / `U64.literal(n)`）的站点 | **22 / 22** |

⇒ **一层嵌套即可覆盖 S 内全部 22 个 G-f2 站点**（不需要两层）；两层形状（`helper(inner(inner(a)), "b")`）作为**递归边界**钉进真值表（3767 / 1281 B）。

⇒ 与 G-f 的 **117** 个调用点合并看：**G-f 面的 117 个站点里，95 个的实参全部落在认领集合内、22 个带嵌套调用实参** ⇒ **G-f + G-f2 之后，G-f 面在 S 内不再有未认领的形状**。

## 6. 诚实性

### 6.1 **未闭合 / 未认领**

| 项 | 状态 |
|---|---|
| **G-d2**（`if` 块里的 concat） | ⏳ **仍在发射端**（S 内 **4** 个调用点）⇒ 按 D10 **不得降级**；它是**独立条目**（计划 §3.3.5），排在 T1 收口片 X 之前 |
| 实参是 `I64.literal(-n)` | ⏳ **未认领**（前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`）—— S 内**不存在**该形状（普查：实参形态分布里没有 `I64.literal`），故不影响族级判定 |
| 实参是**宿主调用**（`sico.*`） | ⏳ 属另一个闸门（`ERR:E-SH-IR-CALL-ARGUMENT`），**不在 G-f / G-f2 面内**（普查：787 个） |
| **自举形状前沿** | ⏔ **没有推进**：改动后的 `stc_call_operand_packed` 与它所属的 `straight_text_concat_packed` 改前改后**都**在前沿内（新钉 `sico_compiler_lowers_the_step0525_parser_functions_byte_exactly`） |
| **guest 前端的形状边界**（本片撞到的那条） | ⏳ **未登记为独立缺口**：它是「**新函数 + 内建调用带调用实参**」的组合，属于**函数体形状**边界，**不是**某个族的操作数能力缺口；本片用「内联进既有可降形状」绕开，**没有**把它登记成族级缺口（诚实起见：它**限制**了后续能新增的函数形状） |
| 独立 CI | 上一批（STEP-0524）的 **run #31 = success ✅**（run id `39961144112`）已回填；本片推送后触发下一轮，回填在收口片 X |
| 语料覆盖 | **0 条** |
| 整文件自举 | **仍开**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`） |

⇒ 本片**不推进任何出口门**，M22 仍为 **NO-GO**，**T1 族仍未闭合**（G-d2 仍开）。

### 6.2 本片**没有**做的事

- 没有动 `selfhost/codegen.sico`（**发射端零改动**）⇒ 本片**没有**碰分支发射。
- 没有动 `crates/*` 的任何生产代码（oracle 零改动）。
- 没有**新增函数**（函数数 175 → 175）⇒ 没有扩大 guest 前端的函数体形状面。
- 没有改**单实参路径**与**多实参路径**的既有分支（只把最后一档 `else` 从「直接拒」换成「先认 nested 形状、不是 nested 就照旧拒」）。
- 没有认领 `I64.literal(-n)` 实参、**宿主调用**实参。
- 没有把语料覆盖从 0 变成 1。
- 没有**降级** T1 族（G-d2 仍开 ⇒ D10 生效）。
- 没有**删除** STEP-0524 的 G-f 能力（新分支只在最后一档生效，既有形状逐字节不变，真值表实测）。

## 7. 出口测试（钉）

| 钉 | 断言 |
|---|---|
| `sico_codegen_guest_pins_the_t1_family_truth_table`（**改**） | 29 → **33 行**：**BF 行**（实参本身是嵌套调用）由「前端 typed 拒绝」翻成「逐字节等同」（IR **3635** / 组件 **1267**）；新增 **BG**（内层 0 实参，3559 / 1261）、**BH**（内层 3 实参，4005 / 1296）、**BI**（内层返回 `U64`，3618 / 1250）、**BJ**（两层嵌套，3767 / 1281）—— **四行都是逐字节等同** |
| `sico_compiler_lowers_the_step0525_parser_functions_byte_exactly`（**新**） | **自举形状前沿**：改动后的 `stc_call_operand_packed` 与它所属的 `straight_text_concat_packed` **都能被 guest 前端逐字节降低**（前沿**没有**被推进） |
| `sico_codegen_guest_lowers_the_gf_multi_argument_call_operand_byte_exactly`（**回归**） | 4 个 G-f 形状**逐字节不变**（1229 / 1185 / 1193 / 1211 B） |

固件：**149 → 150** 条（真值表行数 29 → 33，**新增 1 个测试函数**）。

前沿探针：**39 → 41** 条（新增「实参本身是嵌套调用」与「内层调用没有实参」两条 G-f2 形状，期望 `lowered`）。

## 8. 验证记录（2026-10-23）

### 8.1 固件

- 全量 **150 / 150**（Windows x64，`RUST_TEST_THREADS=1`；含**整文件 parser.sico** 与**整模块 IR 双通道**两条回归）。

### 8.2 差分（Rust 后端组件 vs guest 组件，逐字节）

- 5 个 G-f2 形状 `equal = True`（1267 / 1261 / 1296 / 1250 / 1281 B）；4 个 G-f 形状 `equal = True`（1229 / 1185 / 1193 / 1211 B）；`I64.literal(-n)` 实参形状仍 **REFUSED**（按设计）。

### 8.3 G-f2 普查

- 见 §5（22 个站点 / 内层元数 0×9、1×1、2×8、3×4 / 内层返回种类 text×15、u64×7 / 22/22 全部可认领），普查脚本随电池一起跑。

### 8.4 本机验证电池

**`BATTERY_GREEN 17/17`**（`target/tmp/step0525/battery.ps1`，**17 个 `Run-Step`**；Windows x64，`RUST_TEST_THREADS=1`）：

`workspace-integrity` 53s / `fmt-runner` 3s / `ci-one-key` 494s / `validate-0245` 117s / `validate-0262` 133s / `probe-frontend-frontier` 272s / `probe-ir-plan-frontier` 26s / `validate-0124` 1s / `validate-0131` 2s / `validate-0156` 1s / `git-diff-check` 0s / `line-ending-audit` 0s / `selfhost-comment-discipline` 0s / `step0525-footprint-audit` 0s / `step0525-census-gf2` 1s / `step0525-differential` 46s / `workspace-integrity-final` 53s —— **全部 exit=0**。

> **步数口径**：以电池脚本里 `Run-Step` 的**实际个数**为准（本片 **17 步**，与 STEP-0523 / STEP-0524 同为 17 步，但步名不同 ⇒ 不要拿片号推步数）。

### 8.5 电池体检抓到并修掉的一处**既有**就地损坏（**不是**本片造成的）

电池第一步的**工作区完整性体检**（逐个被跟踪文件算 blob id 与索引比对）报出 **1 处不符**：

| 项 | 实测 |
|---|---|
| 文件 | `pilots/log-analyzer/t07-oversize-input.test.json`（**5,400,089 B**） |
| 差异 | 工作区里有 **2 行**被改成 `alp\xe8a,1` 与 `alqha,1`（应为 `alpha,1`）⇒ **同长、mtime 未变**（2026-09-30 22:18:06） |
| `git status` / `git diff` | **报不出来**（索引的 stat 缓存仍是「相同的大小 + 相同的 mtime」） |
| 与本片的关系 | **无** —— 本片没有碰 `pilots/`，也没有碰过这个文件；损坏**早于**本片 |

⇒ 处置按 **STEP-0523 §9.5 的既有做法**：**不猜成因**，只做「体检 → 按 HEAD 的 blob **逐字节**写回 → 用 blob id 判定 → 整轮重跑」。修复用 `target/tmp/step0525/repair_file.py`（注意 `git checkout-index -f` 在 stat 缓存命中时**不会**真的写文件，这一步也踩到了）。

⇒ 修复后 `WORKSPACE_INTEGRITY_GREEN`（1838 个被跟踪文件，0 处不符，除本片 11 个预期改动）⇒ **本轮电池 17/17 全绿**（电池日志：`target/tmp/step0525/battery.log`）。

### 8.6 未做的事

- 未跑跨平台矩阵（仍只有 Windows x64 原生证据）。
- 未推语料（`selfhost/corpus-v0.json` 仍 0 条本族覆盖）。
- 未处理 **G-d2**（发射端分支支持，独立条目）。
- 未处理 guest 前端的「**内建调用带调用实参**」这条形状边界（只绕开，未登记成族级缺口）。

## 9. 下一刀

1. **G-d2（发射端分支支持）** —— 已作为**独立条目**登记进计划（入口门槛 + 出口判据写全），**不是 T1 的一拍** ⇒ 决定是否单开一片。
2. **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + **独立 CI** 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。
