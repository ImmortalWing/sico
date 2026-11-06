# STEP-0524 m22-w2 S5 **W2 族 T1 的 G-f 刀**：调用型操作数的**多个实参**

> - status: **implementation complete locally**（本机电池 **BATTERY_GREEN 17/17**；独立 CI 裁决待下一批回填）
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0523](./STEP-0523-m22-w2-s5-w2-t1-gd-single-return-route.md)（G-d1）、[STEP-0521](./STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表 + D15 登记）
> - gate: **不推进任何出口门** —— 本片是 **T1 族的前端收口前置片**（G-f 闭合）；**T1 族仍未闭合**（G-d2 发射端分支缺口 + **G-f2** 嵌套调用实参缺口仍开）
> - numbering: **STEP-0524**（取号前已核对 `docs/steps/`：最大号 0523；*0524 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15，已更新）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| STEP-0523 之后 T1 族的前端只差 **G-f**（调用型操作数带多个实参） | ✅ 已登记 |
| G-f 的位置已定位到**唯一闸门**：`straight_text_concat_packed` 的调用操作数分支只接受**一个**实参 | ✅ STEP-0523 §6 |
| 真值可测：Rust 前端/后端都能降这些形状 | ✅ 见 §2 |
| 前置依赖已闭合：G-d1（STEP-0523）、族级真值表（STEP-0521） | ✅ |

## 1. 这一片回答的问题

**guest 能不能把「`sico.text.concat` 的某个操作数是一个**带多个实参的调用**」降下来？**

## 2. 真值测量（生产输出）

测量口径：**Rust 后端 = `sico build --profile script-v0` 的生产组件**（与固件里的 `compile_script_program` 同一函数）；**Rust 前端 = `lower_core` + `canonical_json`**；**guest = `selfhost/compiler.sico --emit-component-hex`**（真燃料 800B，与固件同一 runner）。

fixture 取自 **S 的真实形状**（`selfhost/compiler_parser.sico:172`）与它的最小变体：

| 形状 | Rust IR | Rust 组件 | guest（改前） | guest（改后） |
|---|---|---|---|---|
| **S 真实形状**：`sico.text.concat(at(items, U64.literal(1)), sico.text.concat(".", at(items, U64.literal(2))))` | 3513 | 1229 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | **1229 B，逐字节相同** |
| 两个形参实参：`helper(a, b)` | 3036 | 1185 | **typed 拒绝** | **1185 B，逐字节相同** |
| 三个实参：`helper3(a, b, c)` | 3189 | 1193 | **typed 拒绝** | **1193 B，逐字节相同** |
| 两个字符串字面量实参：`helper2("a", "b")` | 3120 | 1211 | **typed 拒绝** | **1211 B，逐字节相同** |
| 实参是 `I64.literal(-1)` | 3070 | 1190 | **typed 拒绝** | **仍 typed 拒绝**（未认领的子形状，见 §6.1） |
| **实参本身是嵌套调用**：`helper(inner(a), "b")` | **3635** | **1267** | **typed 拒绝** | **仍 typed 拒绝**（**新登记 G-f2**，见 §5） |

⇒ 改前 6 个形状在前端全被拒；改后 **4 个逐字节等同**，2 个**按设计仍拒**。

**Rust 前端怎么降 `U64.literal(1)`**：它**不是**一次内建调用，而是 **`const_u64`** 操作（`{"op":"const_u64","data":1}`），且它的 **range 只覆盖那个数字**（不是整个 `U64.literal(1)`）—— 这一条是从 oracle 的 canonical IR 里读出来的，不是设计。

## 3. 落点（实施记录）

落点文件：**`selfhost/parser.sico`**（**oracle 生产代码 `crates/*` 零改动**）。文件 **22304 → 22458 行（+154）**，函数数 **174 → 175**（新增 1 个，改动 1 个）。

### 3.1 **纯插入**：单实参路径**原样保留**

`straight_text_concat_packed` 的调用操作数分支原本长这样：

```
if U64.equal(next_index(stc_a0, …), stc_call_close):     # 「恰好一个实参」的门
    … 单实参的全部逻辑（原样保留，一个字节都没动）…
else:
    return "ERR:E-SH-IR-CALL-ARITY"                        # ← 原来这里直接拒
end if
```

⇒ 本片只把那个 `else` 分支**换成一次分派**（+17 行）：实参个数 ≠ 1 时交给新函数
   `stc_call_operand_packed`（**新增 138 行**）。**单实参路径一个字节都没改** ⇒
   `let` 绑定路由（K / L / M / N / O / Q 六行）与 1 实参调用行**逐字节不变**（真值表实测）。

**为什么这么切**：这个降低器**被 `let` 路由共用**，是全族热点。**只新增路径、不改旧路径** ⇒
最坏情况下新路径不工作，退化回「1 个实参照旧、多个实参照旧被拒」，**不会连累既有形状**。

### 3.2 新函数 `stc_call_operand_packed` 做了什么

1. **切实参**：在调用括号内**按顶层逗号**切分实参（遇到 `(` 就用 `matching_close` 跳过整段 ⇒ 嵌套调用里的逗号不会被误切）。
2. **逐个实参分类**（与单实参路径**同一套规则**）：形参 / 局部（cell 读或普通读）/ 字符串字面量 / **`U64.literal(n)`**。
3. **拼调用指令**：`arguments` 数组 = 实参值 id 列表（`[0,1]`），调用自己的结果 id = 最后一个实参消耗之后的那个 id。
4. **打包返回**：`<消耗的 id 数>\n<调用结果 id>\n<指令文本>`（与 guest 其它打包机器同一约定）。

### 3.3 失败语义：**保持不变**

新路径的任何 typed 身份都被调用方**原样透传**（`ERR:` 前缀 ⇒ 路由转成 `SKIP`）⇒ 最终仍是
`ERR:E-SH-IR-EXPRESSION`。**实参个数为 0 时仍返回 `ERR:E-SH-IR-CALL-ARITY`**（与改前一致，**不静默放宽**）。

## 4. 撞到并修掉的 2 条自造缺陷

| # | 症状 | 根因 | 修法 |
|---|---|---|---|
| 1 | S 的真实形状仍被拒（`ERR:E-SH-IR-EXPRESSION`），而「两个形参」形状已经能降 | 实参区间是**左闭右开**（`[起, 止)`），我却拿字面量 `)` 的下标**直接**和区间末端比较 | 改成「`)` 的**下一个**词 == 区间末端」⇒ 实参必须**正好**是 `U64.literal(n)` |
| 2 | guest 前端降不下新函数（`ERR:E-SH-IR-RETURN-COUNT`），**整文件 parser.sico 也跟着降不下** | 新函数尾部是「`match` 的臂里再套一个 `match`」⇒ guest 的**通用 CFG 路由不支持臂内嵌套 match**（实测最小复现：`ERR:E-SH-IR-STATEMENT`） | 尾部改成「**计数器 + 纯 `return`**」：用 `scco_consumed` 计数代替 `checked_sub` ⇒ 去掉臂内嵌套 match。**自举前沿因此没有推进**（新函数与共用降低器都仍在前沿内） |

⇒ 第 2 条同时暴露了一条**既有事实**：`sico_compiler_lowers_straight_text_concat_packed_closure_byte_exactly`
那份**手工维护**的闭包清单在本次改动后失效（Rust oracle 报 `unresolved call target stc_call_operand_packed`）
⇒ **改成从文件自动计算**（`parser_closure_source`），与 STEP-0523 对姊妹清单做的同一件事。

## 5. **S 内 G-f 普查**（本片新增证据；口径写在脚本里）

普查脚本：`target/tmp/step0524/census_gf.py`（只读，输出随电池一起跑）。口径 = **文本计数**：
`selfhost/*.sico` 里 `sico.text.concat(` 的**每一个操作数**，当且仅当它是
**用户定义函数的调用、且该函数声明返回 `Text`、且实参 ≥ 2 个**时，算作一个 **G-f 调用点**
（这正是降低器里那道**元数闸门**过去会拒的形状）。

| 指标 | 数值 |
|---|---|
| **G-f 面（S 内）** | **117** 个调用点 |
| 其中**全部实参**都落在新函数认领的集合内（形参 / 局部 / 字符串字面量 / `U64.literal(n)`） | **95** |
| 其中带**嵌套调用实参** ⇒ 新路径仍 typed 拒绝 | **22** |
| 实参形态分布（按**实参出现次数**） | 局部 **166** / 形参 **69** / `U64.literal` **47** / **嵌套调用 22** / 字符串字面量 **10** |
| 元数分布 | 2 个实参 **90** / 3 个 **2** / 4 个 **3** / 5 个 **16** / 6 个 **6** |
| 路由分布 | `let`/`set` 路由 **115** / 单语句 `return` 路由 **2**（`compiler_parser.sico:172`，即 S 的真实形状） |
| 按文件 | `parser.sico` **80** / `codegen.sico` **34** / `compiler_parser.sico` **2** / `formatter.sico` **1** |
| （不属于 G-f 面）操作数是**宿主调用**（`sico.*`） | **787** —— 属另一个闸门（`ERR:E-SH-IR-CALL-ARGUMENT`） |

### 5.1 **订正一条口径**（STEP-0523 / STEP-0524 之前记的「S 内 1 个调用点」）

STEP-0523 §8.1 与本片初稿写的「G-f 在 S 内 **1** 个调用点」，是 **G-d 子集**口径
（§3.2 那张 10 行表 = 「落在**没有 `let`/`set` 的函数体**里的 `sico.text.concat(` 调用点」= **10** 个，
其中带两实参调用操作数的只有 `compiler_parser.sico:172` 这 **1** 处）。
**全 S 的 G-f 面是 117**（上表）⇒ 该口径必须订正，否则族级出口判据会失真。

### 5.2 **新登记 G-f2** = 调用型操作数的**实参本身是嵌套调用**

| 项 | 值 |
|---|---|
| 形状 | `sico.text.concat(f(g(x), y), z)` —— 被调函数的**实参**是另一次调用 |
| 症状 | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（新路径在实参分类处落到 `ERR:E-SH-IR-CALL-ARGUMENT` ⇒ 路由转 `SKIP`） |
| S 内是否出现 | **是 —— 22 个调用点**（`codegen.sico` 10 个 + `parser.sico` 12 个） |
| 实测真值 | Rust canonical IR **3635** B、Rust 生产组件 **1267** B —— **Rust 两个 oracle 都能降**，guest 不能 ⇒ 是**真缺口**，不是「本来就不支持」 |
| 归属 | **T1 族的前端缺口**（不是发射端）⇒ 按 **D10 不得降级** |

⇒ 这一条**推翻**了本片初稿里「嵌套调用实参在 S 内不存在」的说法（那句话只对 `I64.literal(-n)` 成立）。

## 6. 诚实性

### 6.1 **未闭合 / 未认领**

| 项 | 状态 |
|---|---|
| **G-d2**（`if` 块里的 concat） | ⏳ **仍在发射端**（S 内 4 个调用点）⇒ 按 D10 **不得降级** |
| **G-f2**（实参本身是嵌套调用） | ⏳ **在前端**（S 内 **22** 个调用点）⇒ 按 D10 **不得降级**（本片**新登记**） |
| 实参是 `I64.literal(-n)` | ⏳ **未认领**（前端 typed 拒绝）—— S 内**不存在**该形状（普查：实参形态分布里没有 `I64.literal`），故不影响族级判定 |
| 实参是**宿主调用** | ⏳ 属另一个闸门（`ERR:E-SH-IR-CALL-ARGUMENT`），**不在 G-f 面内**（普查：787 个） |
| 独立 CI | 本片推送后触发（上一批 `957887c` 的 **run #30 = success ✅** 已回填进 STEP-0523） |
| 语料覆盖 | **0 条** |
| 整文件自举 | **仍开**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`） |

⇒ 本片**不推进任何出口门**，M22 仍为 **NO-GO**，**T1 族仍未闭合**（G-d2 + G-f2 仍开）。

### 6.2 本片**没有**做的事

- 没有动 `selfhost/codegen.sico`（**发射端零改动**）⇒ 本片**没有**碰分支发射。
- 没有动 `crates/*` 的任何生产代码（oracle 零改动）。
- 没有改**单实参**路径（一个字节都没动）。
- 没有认领 `I64.literal(-n)` 实参、**嵌套调用实参**（G-f2）。
- 没有把语料覆盖从 0 变成 1。
- 没有**降级** T1 族（G-d2 + G-f2 仍开 ⇒ D10 生效）。

## 7. 出口测试（钉）

| 钉 | 断言 |
|---|---|
| `sico_codegen_guest_lowers_the_gf_multi_argument_call_operand_byte_exactly`（**新**） | **4 个 G-f 形状**：S 真实形状 / 两个形参 / 三个实参 / 两个字面量实参 —— guest IR 与 Rust canonical IR **逐字节相同**，guest 组件与 Rust 后端组件**逐字节相同**（尺寸钉住 **1229 / 1185 / 1193 / 1211** B） |
| `sico_codegen_guest_pins_the_t1_family_truth_table`（**改**） | 24 → **29 行**：Z 行（两个实参）由「前端 typed 拒绝」翻成「逐字节等同」；新增 **BB / BC / BD**（逐字节等同）、**BE**（`I64.literal(-n)` 实参 ⇒ 仍前端 typed 拒绝）与 **BF**（**实参本身是嵌套调用** ⇒ 仍前端 typed 拒绝，IR **3635** / 组件 **1267**） |
| `sico_compiler_lowers_the_step0524_parser_functions_byte_exactly`（**新**） | **自举形状前沿**：新增的 `stc_call_operand_packed` 与它所属的 `straight_text_concat_packed` **都能被 guest 前端逐字节降低**（前沿**没有**被推进） |
| `sico_compiler_keeps_the_unclosed_text_concat_shapes_typed_refused`（**改**） | 已被认领的「两个实参」形状移出；仍钉住 **`I64.literal(-n)` 实参**与「操作数类型不对」，身份仍是 `ERR:E-SH-IR-EXPRESSION` |
| `sico_compiler_lowers_straight_text_concat_packed_closure_byte_exactly`（**改**） | 手工闭包清单 ⇒ **自动计算**（`parser_closure_source`） |

固件：**147 → 149** 条（真值表行数 28 → 29，**不新增测试函数**）。

## 8. 验证记录（2026-10-23）

### 8.1 固件

- 全量 **149 / 149**（Windows x64，`RUST_TEST_THREADS=1`；含**整文件 parser.sico** 与**整模块 IR 双通道**两条回归）。

### 8.2 前沿探针

- 38 → **39** 条：`text-concat-with-a-two-argument-user-call-operand` 的期望值翻转（前端拒 → 降）；新增 `text-concat-with-a-u64-literal-argument-call-operand`（S 真实形状 ⇒ 降）。

### 8.3 差分（Rust 后端组件 vs guest 组件，逐字节）

- 4 个形状 `equal = True`（1229 / 1185 / 1193 / 1211 B）；`I64.literal(-n)` 实参形状仍 **REFUSED**。

### 8.4 G-f 普查

- 见 §5（117 / 95 / 22 …），普查脚本随电池一起跑。

### 8.5 本机验证电池

**`BATTERY_GREEN 17/17`**（`target/tmp/step0524/battery.ps1`，**17 个 `Run-Step`**；Windows x64，`RUST_TEST_THREADS=1`）：

`workspace-integrity` 54s / `fmt-runner` 3s / `ci-one-key` 446s / `validate-0245` 119s / `validate-0262` 135s / `probe-frontend-frontier` 258s / `probe-ir-plan-frontier` 26s / `validate-0124` 1s / `validate-0131` 2s / `validate-0156` 1s / `git-diff-check` 0s / `line-ending-audit` 0s / `selfhost-comment-discipline` 0s / `step0524-footprint-audit` 0s / `step0524-census-gf` 1s / `step0524-differential` 24s / `workspace-integrity-final` 52s —— **全部 exit=0**。

> **步数口径**：以电池脚本里 `Run-Step` 的**实际个数**为准（本片 **17 步**，与 STEP-0523 同为 17 步，但两片的步名不同 ⇒ 不要拿片号推步数）。

### 8.6 第 1 轮电池的红：一条**自造的**判红（已修，整轮重跑）

第 1 轮电池是 **16/17**（红的是 `step0524-differential`，日志留在 `target/tmp/step0524/battery-run1-red.log`）：

| 事实 | 值 |
|---|---|
| 4 个逐字节等同的形状 | **全部 `equal = True`**（1229 / 1185 / 1193 / 1211 B） |
| 第 5 个 fixture（`I64.literal(-n)` 实参） | guest **按设计 typed 拒绝**（`ERR:E-SH-IR-EXPRESSION`），Rust 后端**能**降（1190 B） |
| 红的原因 | **不是产品缺陷，是电池脚本的口径错**：差分脚本把「guest 拒绝」一律当成失败 ⇒ **按设计被拒的形状被判红** |
| 修法 | 差分脚本改为**逐 fixture 记录期望**（`exact` / `refused`）：`exact` 要求与 Rust 后端**逐字节相同**，`refused` 要求 **Rust 能降而 guest typed 拒绝**（这正是 BE / BF 两行的含义）⇒ 脚本只在**实测与记录的真值不符**时才红 |
| 处置 | 按纪律**整轮重跑**（第 2 轮 `BATTERY_GREEN 17/17`），电池脚本的 17 个步名与参数**一个字节都没改** |

⇒ 这条与 STEP-0523 §9.5 记的那条是**同一类事故的两种形态**：那次是**工作区文件被就地损坏而 `git status` 看不见**，这次是**判据写错而把事实判红**。两次都靠「红了就查、查完整轮重跑」收口。

### 8.7 未做的事

- 未跑跨平台矩阵（仍只有 Windows x64 原生证据）。
- 未推语料（`selfhost/corpus-v0.json` 仍 0 条本族覆盖）。
- 未把 G-f2 当作本片的目标去实现（它在本片只被**登记与测量**）。

## 9. 下一刀

1. **G-f2（嵌套调用实参）** —— 新登记的前端形状缺口（S 内 22 个调用点）。它要的是**操作数降低器能递归地降一次调用作为实参**（不是新闸门，而是把现有能力递归下去）。
2. **G-d2（发射端分支支持）** —— 已作为**独立条目**登记进计划（入口门槛 + 出口判据写全），**不是 T1 的一拍** ⇒ 决定是否单开一片。
3. **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + **独立 CI** 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。
