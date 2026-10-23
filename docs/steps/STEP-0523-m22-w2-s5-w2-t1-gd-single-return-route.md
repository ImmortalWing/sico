# STEP-0523 m22-w2 S5 **W2 族 T1 的 G-d 刀（第一刀）**：单语句 `return` 的通用操作数

> - status: **closed**（本机验证电池 **17/17 `BATTERY_GREEN`**；已提交 `957887c` 并推送到 `codex/m22-w1-ci`；**独立 CI run #30 = success ✅**）
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0522](./STEP-0522-m22-w2-s5-w2-t1-escaped-literal-decoder.md)（G-b / G-e 两刀）、[STEP-0521](./STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表 + D15 登记）
> - gate: **不推进任何出口门** —— 本片是 **T1 族的收口前置片之一**（G-d 的**第一刀**）；**T1 族仍未闭合**
> - numbering: **STEP-0523**（取号前已核对 `docs/steps/`：最大号 0522；*0523 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15，已更新）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| STEP-0522 之后 T1 族只剩 **G-d** 一个硬阻塞（前端路由门）。 | ✅ 已登记 |
| G-d 的位置已定位到**唯一闸门**：`direct_text_concat_return_ir` 只认「两个 `Text` 形参」。 | ✅ STEP-0521 §3 / STEP-0522 §10 |
| 真值可测：Rust 前端/后端都能降这些形状。 | ✅ 见 §2 |
| 前置依赖已闭合：G-b / G-e（STEP-0522）、族级真值表（STEP-0521）。 | ✅ |

## 1. 这一片回答的问题

**guest 能不能把「函数体只有一条 `return`、而 `return` 的表达式是一个 `sico.text.concat` 调用」降下来 —— 当操作数不是两个 `Text` 形参时？**

## 2. 真值测量（生产输出）

测量口径：**Rust 后端 = `sico build --profile script-v0` 的生产组件**（与固件里的 `compile_script_program` 同一函数）；**guest = `selfhost/compiler.sico --emit-component-hex`**（真燃料 800B，与固件同一 runner）。

fixture 取自 **S 的真实形状**（`selfhost/codegen.sico` / `selfhost/compiler_parser.sico`），逐字节对齐：

| 形状 | Rust 后端组件 | guest 组件（改前） | guest 组件（改后） |
|---|---|---|---|
| 字面量 × 字面量 | 1124 B | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | **1124 B，逐字节相同** |
| 形参 + `"\t"` | 1113 B | **typed 拒绝** | **1113 B，逐字节相同** |
| 嵌套 concat | 1112 B | **typed 拒绝** | **1112 B，逐字节相同** |
| 字面量 + **调用另一个函数** | 1177 B | **typed 拒绝** | **1177 B，逐字节相同** |
| 调用另一个函数 + 字面量 | 1177 B | **typed 拒绝** | **1177 B，逐字节相同** |
| 字面量 + 嵌套 concat（内含调用） | 1208 B | **typed 拒绝** | **1208 B，逐字节相同** |
| 调用链（被调函数本身也是 concat） | 1260 B | **typed 拒绝** | **1260 B，逐字节相同** |
| 调用另一个函数（**两个实参**） | 1185 B | **typed 拒绝** | **typed 拒绝**（仍未覆盖，见 §6） |
| concat 在 **if 块**里 | 1198 B | 前端 typed 拒绝 | **前端逐字节相同；组件层仍 typed 拒绝**（见 §6） |

⇒ 改前：7 个形状在前端被拒；改后：**6 个形状逐字节等同**，2 个形状的原因被**定位得更准**（一个仍是前端路由门，一个是**发射端的分支缺口**）。

## 3. 落点（实施记录）

落点文件：**`selfhost/parser.sico`**（**oracle 生产代码 `crates/*` 零改动**）。文件 **22342 → 22304 行（−38）**，函数数 **174 → 174**（重塑 1 个，无新增/删除）。

### 3.1 核心改动：路由不再自己抄一份操作数规则

改前 `direct_text_concat_return_ir` 自己判断「两个操作数都是 `Text` 形参」，然后**手写**出 concat 指令（`dtcr_operand0_text` / `dtcr_pair_body` / … 六个临时变量拼出 `[0,1]`）。

改后它**复用 `straight_text_concat_packed`** —— 也就是 `let` 绑定路由（`straight_cell_function_ir` / `straight_environment_function_ir`）一直在用的那一个操作数降低器：

| 改前 | 改后 |
|---|---|
| 只认「两个 `Text` 形参」，其它操作数 ⇒ `SKIP` | 操作数规则**全部**交给 `straight_text_concat_packed` |
| 手写 `[0,1]` 实参表 | 实参表由降低器按操作数实际形状给出 |
| 自己拼 concat 指令 | 降低器返回「指令数 / 结果 id / 内层指令 / concat 指令」的打包结果，路由只负责组装 |

**为什么这是正确的一刀**：`let` 绑定形状（K / L / M / N / O / Q）**早就**能降字面量、嵌套 concat 与调用操作数 —— 差别只在「函数体有没有别的语句」。改后单语句 `return` 形状与 `let` 形状**走同一份规则**，于是「同一批操作数，两个路由给出不同答案」这个结构性差异消失。

### 3.2 失败语义：**保持不变**（`SKIP`，不是新的 ERR 身份）

降低器对不支持的形状返回 typed 身份（`ERR:E-SH-IR-CALL-TYPE` / `ERR:E-SH-IR-CALL-ARITY` / …）。路由把它们**转成 `SKIP`** ⇒ 路由梯继续往下走 ⇒ 最终仍是**原来那个**通用身份 `ERR:E-SH-IR-EXPRESSION`。

实测（改后）：操作数类型不对（`sico.text.concat(a, a)`，`a: U64`）与三个实参 ⇒ 仍是 `ERR:E-SH-IR-EXPRESSION`，**与改前一致**。**没有偷偷换身份。**

### 3.3 自举形状前沿：**没有推进**

`direct_text_concat_return_ir` 改前**就在**前沿内（`lowered`），改后**仍在**前沿内（`lowered`）；它现在调用的 `straight_text_concat_packed` 也**在**前沿内（改前改后都是 `lowered`）。

⇒ **本片没有推进形状边界**：新形状是靠**复用既有形状**得到的，不是靠改形状硬挤过去的。（新钉 `sico_compiler_lowers_the_step0523_parser_functions_byte_exactly`。）

## 4. S 内普查（G-d 的精确形状）

普查脚本：`target/tmp/step0523/locate_gd.py`（口径：**文本调用点**）。

S = `selfhost/*.sico` 9 个文件里，「落在没有 `let` / `set` 的函数体里」的 `sico.text.concat(` 调用点 = **10** 个（与 STEP-0521 / STEP-0522 记的 10 一致）。逐个定位后：

| # | 文件:行 | 函数 | 操作数形态 | 本片后 |
|---|---|---|---|---|
| 1 | `codegen.sico:147` | `cg_post_body_hex` | 字面量 + **嵌套 concat**（内含调用） | ✅ **逐字节等同** |
| 2 | `codegen.sico:147` | `cg_post_body_hex` | 嵌套 concat 的内层：调用 + 字面量 | ✅ **逐字节等同** |
| 3 | `codegen.sico:1630` | `cgs_local_get_hex` | 字面量 + 调用 | ✅ **逐字节等同** |
| 4 | `codegen.sico:1635` | `cgs_local_set_hex` | 字面量 + 调用 | ✅ **逐字节等同** |
| 5 | `codegen.sico:1640` | `cgs_i32_const_hex` | 字面量 + 调用 | ✅ **逐字节等同** |
| 6 | `codegen.sico:1645` | `cgs_i64_const_hex` | 字面量 + 调用 | ✅ **逐字节等同** |
| 7 | `codegen.sico:2069` | `cgs_zero_hex` | 字面量 + 调用，**在 `if` 块里** | ⏳ 前端已降，**组件层仍拒** |
| 8 | `codegen.sico:2072` | `cgs_zero_hex` | 字面量 + 调用，**在 `if` 块里** | ⏳ 同上 |
| 9 | `compiler_parser.sico:172` | `declaration_name` | **两实参调用** + 嵌套 concat，**在 `if` 块里** | ⏳ 同上（且两实参调用本身也未覆盖） |
| 10 | `compiler_parser.sico:172` | `declaration_name` | 嵌套 concat 的内层：字面量 + **两实参调用**，**在 `if` 块里** | ⏳ 同上 |

⇒ **6 / 10 个调用点闭合**；剩下 4 个**全部**在 `if` 块里。

## 5. **重要发现**：G-d 其实是**两个**缺口，不是一个

STEP-0521/0522 把 G-d 登记成「前端 typed 拒绝」。本片把剩下那 4 个调用点逐个探过之后，发现它们的拒绝**不在前端**：

| 探针（一步一因子） | guest 前端 | guest 组件 |
|---|---|---|
| `if` 块里返回**一个字面量**（与 concat 无关） | ✅ lowered | ❌ **typed 拒绝** |
| `if` 块里 `return sico.text.concat("a", "b")` | ✅ lowered | ❌ **typed 拒绝** |
| `if` 块里 `return sico.text.concat("a", a)` | ✅ lowered | ❌ **typed 拒绝** |
| `if` 块里 `return sico.text.concat("a", helper(a))` | ✅ lowered | ❌ **typed 拒绝** |
| **无** `if` 块，`return sico.text.concat("a", helper(a))` | ✅ lowered | ✅ **逐字节相同** |

⇒ **只要 IR 里有分支，guest 的组件发射就拒**。原因是 guest 的发射器（`selfhost/codegen.sico`）只支持**单块直线代码**：它认识 `return` 终结符，遇到别的终结符就 `ERR:E-SH-CG-IR-TERMINATOR` / `ERR:E-SH-CG-IR-EMIT`。

⇒ **G-d 拆成两刀**：

| 缺口 | 位置 | 性质 | 状态 |
|---|---|---|---|
| **G-d1** 单语句 `return` 的非形参操作数 | **前端**（路由门） | T1 族自身的缺口 | ✅ **本片闭合** |
| **G-d2** `if` 块里的 concat | **发射端**（分支缺口） | **不是 T1 族的缺口** —— 是发射器「只支持直线代码」的子集边界；任何 `if` 都撞上它 | ⏳ 未闭合 |

**为什么这条区分重要**：G-d2 不是一个「再补一个操作数形状」就能关掉的洞 —— 它要的是**发射器支持分支**（Wasm 的 `if`/`block`/`br` 与 IR 的多块结构），那是另一个量级的工作，**而且它不专属于 T1**。把它继续挂在 T1 族的名下会让族级出口判据失真。

## 6. 仍未闭合的三个形状（逐条定位）

| 形状 | 谁拒 | 身份 | 归属 |
|---|---|---|---|
| `sico.text.concat` 的操作数是**两个实参**的调用（`at(items, U64.literal(1))`） | **前端** | `ERR:E-SH-IR-EXPRESSION` | 借用来的降低器 `straight_text_concat_packed` 的调用分支只支持**一个**实参 |
| `if` 块里的 concat | **发射端** | `unsupported codegen source shape`（`ERR:E-SH-CG-IR-EMIT`） | **G-d2**：发射器只支持单块直线代码 |
| `if` 块里的 concat + 上面那个两实参调用 | 两者都有 | —— | G-d2 ∧ 两实参调用 |

> 前两条**互相独立**：两实参调用即使不在 `if` 块里也拒（G-d1 的残留），`if` 块即使只有字面量也拒（G-d2）。

## 7. 出口测试（钉）

| 钉 | 断言 |
|---|---|
| `sico_codegen_guest_lowers_the_gd_single_return_shapes_byte_exactly` | **S 的 4 个真实形状**：2 个已闭合形状（`cg_post_body_hex` / `cgs_local_get_hex`）的 guest IR 与 Rust canonical IR **逐字节相同**、guest 组件与 Rust 后端组件**逐字节相同**（组件尺寸钉住 **1237 / 1177** B）；2 个带 `if` 的形状（`cgs_zero_hex` / `declaration_name`）的 **IR 逐字节相同**（**3753 / 4387** B）而**组件层 typed 拒绝**（尺寸钉住 **1266 / 1316** B） |
| `sico_codegen_guest_pins_the_t1_family_truth_table`（**改**） | 24 行（原 18 行）：H / I / J / R 四行由「前端 typed 拒绝」翻成「逐字节等同」；**新增 6 行**（4 个调用操作数形状 + 1 个两实参调用〔仍前端拒〕 + 1 个 `if` 块〔前端降、组件拒〕）；真值表引入**第三种判定** `GuestVerdict::CodegenRefused`（前端降、组件拒） |
| `sico_compiler_lowers_the_step0523_parser_functions_byte_exactly` | **自举形状前沿**：重塑后的 `direct_text_concat_return_ir` 与它现在调用的 `straight_text_concat_packed` **都能被 guest 前端逐字节降低**（前沿**没有**被推进） |

固件：**145 → 147** 条。

## 8. 诚实性

### 8.1 T1 族**仍未闭合**

| 未闭合项 | 状态 |
|---|---|
| **G-d2**（`if` 块里的 concat） | ⏳ **在发射端**（S 内 4 个调用点）⇒ 按 D10 **不得降级** |
| **两实参调用**作为 concat 操作数 | ⏳ **在前端**（S 内 1 个调用点：#10）⇒ 同上 —— **⚠️ 口径订正（STEP-0524）**：这「1 个」是**上面那张 10 行表**（落在**没有 `let`/`set` 的函数体**里的 concat 调用点）的**子集**计数；**全 S** 的这种形状（G-f 面）实测是 **117** 个调用点（见 [STEP-0524](./STEP-0524-m22-w2-s5-w2-t1-gf-multi-argument-call-operand.md) §5）⇒ 同上 |
| **独立 CI** | ✅ **run #30 = success**（提交 `957887c` ⇒ `codex/m22-w1-ci`，run id `39915083272`，job `windows-gnu` 4/4 步绿） |
| 语料覆盖 | **0 条** |
| 整文件自举 | **仍开**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`） |

⇒ 本片**不推进任何出口门**，M22 仍为 **NO-GO**。

### 8.2 本片**没有**做的事

- 没有认领 **G-d2**（发射端的分支缺口）。
- 没有认领**两实参调用**这个操作数形状。
- 没有认领其它内建；T1 仍只认 `sico.text.concat`。
- 没有把语料覆盖从 0 变成 1。
- 没有动 `crates/*` 的任何生产代码（oracle 零改动）。
- 没有动 `selfhost/codegen.sico`（**发射端零改动**）⇒ 本片**没有**碰分支发射。

### 8.3 已知风险

- 路由现在**借用** `straight_text_concat_packed`（378 行、13 个形参）。该降低器的调用分支只支持**一个**实参 ⇒ S 里的 `at(items, U64.literal(1))` 仍被拒（§6）。把「一实参」放宽到「N 实参」会动到 `let` 绑定路由共用的代码 ⇒ **应当单独成片**并把 T1 族级真值表整表复跑。
- `straight_text_concat_packed` 的调用分支在 `let` 路由与本片路由之间**共用** ⇒ 本片对它的任何后续改动都会同时影响 K / L / M / N / O / Q 六行。

## 9. 验证记录（2026-10-23）

### 9.1 固件

- 新钉 2/2 绿；改钉（真值表）绿；固件全量 **147 / 147**（Windows x64，`RUST_TEST_THREADS=1`；145 → 147）。

### 9.2 前沿探针

- 34 → **38** 条：两条旧期望值翻转（`text-concat-with-literal-operands`、`text-concat-with-a-mixed-operand-pair`：前端拒 → 降），新增 4 条（调用操作数 ×2、嵌套调用操作数、两实参调用〔仍拒〕、`if` 块里的 concat〔前端降〕）。

### 9.3 本机验证电池

**`BATTERY_GREEN 17/17`**（`target/tmp/step0523/battery.ps1`，**17 个 `Run-Step`**；Windows x64，`RUST_TEST_THREADS=1`）：

`workspace-integrity` 44s / `fmt-runner` 1s / `ci-one-key` 411s / `validate-0245` 127s / `validate-0262` 135s / `probe-frontend-frontier` 268s / `probe-ir-plan-frontier` 25s / `validate-0124` 1s / `validate-0131` 2s / `validate-0156` 1s / `git-diff-check` 0s / `line-ending-audit` 0s / `selfhost-comment-discipline` 0s / `step0523-footprint-audit` 0s / `step0523-census-s` 0s / `step0523-differential` 30s / `workspace-integrity-final` 44s —— **全部 exit=0**。

> **步数口径**：以电池脚本里 `Run-Step` 的**实际个数**为准（本片 **17 步**）。STEP-0522 的电池是 **16 步**（它把 STEP-0521 的两条足迹审计换成了自己的两条）⇒ **两个数字不同是正常的**，不要拿片号推步数。**本片自己一度把 17 写成 18**（写文档时数错了）⇒ 电池一绿就按脚本实际个数订正。

### 9.4 未做的事

- 未跑跨平台矩阵（仍只有 Windows x64 原生证据）。
- 未推语料（`selfhost/corpus-v0.json` 仍 0 条本族覆盖）。

### 9.5 环境事故之一：工作区文件被**就地损坏**，而 `git status` 看不见

本片第一轮电池跑到 `validate-0245` 判红，原因是 `crates/sico-runtime/src/lib.rs` 在工作区里**语法不成立**（`use sico_package::{` 没有闭合，`AtomicU64::new` 被写成 `ItomicU64::new`）。

**关键事实**：

| 项 | 实测 |
|---|---|
| HEAD 里的同一文件 | **完好**（blob `578ac5c3`） |
| 工作区里的同一文件 | **损坏**（blob `0d049544`，与 HEAD 差 **2 个单字符**） |
| 文件大小 | **完全相同**（29986 B） |
| 文件 mtime | **完全相同**（2026-10-21 08:19:22） |
| `git status` | **报不出来**（索引的 stat 缓存仍是那对「相同的大小 + 相同的 mtime」⇒ git 认为文件没变） |
| 损坏窗口 | 第一轮电池的 `ci-one-key`（14:07 通过）之后、`validate-0245`（14:11 判红）之前 |

⇒ 两处单字符替换 + 大小与 mtime 都不变 ⇒ **不是**本片任何写操作造成的（本片没有碰 `crates/`，写盘的都是 git / cargo / python 的正常路径），而是**工作区层面的就地损坏**；`git status` 的 stat 缓存把它**藏了起来**。

**处置**：

1. 全量体检：对 **1836 个被跟踪文件**逐个算 blob id 与索引比对 ⇒ **除本片 10 个预期改动外，只有这 1 处不符**（顺带确认了本片自己的改动没有别的意外）。
2. 修复：按 HEAD 的 blob **逐字节**写回（`target/tmp/step0523/repair_file.py`），并**用 blob id 而不是 `git status`** 判定修复结果（`git checkout-index -f` 在 stat 缓存命中时**不会**真的写文件 —— 这一点也踩到了）。
3. **把体检固化成电池的一步**：`target/tmp/step0523/integrity.py`（`WORKSPACE_INTEGRITY_GREEN`），**首尾各跑一次** —— 开工前确认工作区可信，提交前再确认一次。这条检查进的是电池脚本，不是固件（固件不读工作区文件）。
4. 修复后**整轮重跑**电池 ⇒ 又判红**两次**，**两次都不是代码缺陷**（见 §9.6）⇒ 处置完再整轮重跑 ⇒ **`BATTERY_GREEN 17/17`**（§9.3）。

> **本片没有做的事**：没有去猜损坏的成因（查不到写坏它的一方），只做了「体检 → 修复 → 把体检变成常规步骤」。

### 9.6 另外两次电池判红（**都不是代码缺陷**，都留痕）

| # | 判红步 | 现象 | 根因 | 处置 |
|---|---|---|---|---|
| 1 | `workspace-integrity`（**首步**） | `docs/steps/STEP-0522-…md: index 1d36ae5a != worktree 1b29738` | STEP-0522 的 **run #29 裁决回填**是在提交 `8bdd58d` **之后**落盘的：它**已改**、**本来就要随本片一起提交**，但**不在本片的改动白名单**里 ⇒ 体检按「不在声明的改动集内」判红 | 把该文件**如实列入**白名单 ⇒ 再整轮重跑 |
| 2 | `ci-one-key` | `test (workspace)` 的 `run_executes_a_script_package_end_to_end` 失败：`sico-app: refusing to overwrite existing artifact …\Temp\sico-app-cli-18552-8-echo-run.sapp` | 该用例**成功时也不清理**自己的产物（实测：单独复跑**成功后** `%TEMP%` 里仍留下 `sico-app-cli-<pid>-0-echo-run.command.wasm` 与 `…-1-echo-run.sapp`），而临时文件名里带 **pid** ⇒ Windows **回收 pid** 后，下一轮 `pack` 就撞上一个同名产物 ⇒ 按设计**拒绝覆盖** ⇒ 用例失败。本会话里**被中断**的电池留下的残留把它提前引爆了 | 清掉 8 个残留产物 ⇒ 单独复跑该用例 **绿** ⇒ 再整轮重跑 |

⇒ ①是**记账**问题（改动集没如实登记），②是**环境残留 + pid 回收**问题。两条都**不是**「改断言 / 改预算绕过」能解决的：①要么如实登记，要么真的不提交；②要么清残留，要么换 pid。
⇒ ②暴露的是**既有的测试健壮性弱点**（用例不清产物 + pid 进临时文件名）。本片**没有**改 `crates/*`（oracle 零改动）⇒ **只记录，不在本片修**。

## 10. 下一刀

1. **G-d2（发射端分支支持）** —— 这是 T1 族剩下的唯一硬阻塞，但它是**发射器的子集边界**，量级大于一个族内切片 ⇒ 建议**先把它当作独立条目登记进计划**，再决定是否单开一片。
2. **两实参调用**作为 concat 操作数（前端）—— 影响 `declaration_name`；改的是 `let` 路由共用的降低器 ⇒ 单独成片 + 整表复跑。
3. 上面两条都落地后，才是 **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + 独立 CI 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。
