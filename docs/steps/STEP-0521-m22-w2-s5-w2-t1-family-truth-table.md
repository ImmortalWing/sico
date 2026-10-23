# STEP-0521 m22-w2 S5 **W2 族 T1 族级真值表与支持面登记（D15）**：20 行真值表 + 模块级静态 data 段偏移修正

> - status: **implementation complete locally（本机全绿；已随 STEP-0506…0521 批次提交 `a5e0b65` 并推送到 `codex/m22-w1-ci`；独立 CI run #28 已触发、尚未出结论）**
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0520](./STEP-0520-m22-w2-s5-w2-shared-local-cell-support.md)（cell 共享前置）、[STEP-0519](./STEP-0519-m22-w2-s5-w2-t1-codegen-knife.md)（codegen 刀）、[STEP-0518](./STEP-0518-m22-w2-s5-w2-t1-frontend-knife.md)（前端刀）、[STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（真值测量）
> - gate: **不推进任何出口门** —— 本片是 **T1 族的收口前置**（真值表 + D15 登记 + 缺口定位 + 一处静默偏移修正）；**T1 族仍未闭合**（G-b / G-d 在 S 内真实存在，独立 CI 尚未运行）
> - numbering: **STEP-0521**（取号前已核对 `docs/steps/`：最大号 0520；*0521 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| 前三拍已完成（真值 T / 前端刀 F / codegen 刀 C）+ 共享前置（cell）。 | ✅ STEP-0517 / 0518 / 0519 / 0520 |
| 族的支持面**从未被当作一张表测量过**（每片只钉一个形状）。 | ✅ §2 的普查覆盖 20 个形状 |
| D15 要求每族闭合时登记「族 × 子集边界 × 语料覆盖 × 未覆盖形状」。 | ✅ §3 + 支持面登记表 |
| 本片**不推进出口门**，只把族级事实钉死并把缺口定位到可施工的形状。 | ✅ §4 |

⇒ **取号合法**。

## 1. 这一片回答的问题

1. **T1 族到底支持到哪一步？** ⇒ §2 的 20 行真值表（Rust 前端 / Rust 后端 / guest 前端 / guest 组件 四轴）。
2. **S 里的调用点形状，哪些还没被覆盖？** ⇒ §3 的 S 普查 + 4 个缺口（G-a / G-b / G-c / G-d）。
3. **guest 有没有在“静默发出错误字节”？** ⇒ **有，且本片已定位并修正**：G-a。

## 2. 族级真值表（20 行，实测，固件钉住）

| # | 形状 | oracle IR | oracle 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| A | let 绑定位置（形参 ×2） | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| B | let 绑定位置（cell ×2） | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| C | return 位置 + 体内另有 `let` | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| D | return 位置 + 体内**只有**一条 return | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| E | 被赋值的 `let`（cell 写） | 2630 | 1114 | 逐字节等同 | 逐字节等同 |
| F | 被赋值的 `let`（cell 写 + 读） | 2868 | 1138 | 逐字节等同 | 逐字节等同 |
| G | I64 cell | 2837 | 1116 | 逐字节等同 | 逐字节等同 |
| H | 两个字面量实参（体内只有 return） | 2372 | 1124 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| I | 形参 + 字面量（体内只有 return） | 2334 | 1113 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| J | 嵌套宿主调用（体内只有 return） | 2447 | 1112 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| K | 两个字面量实参（体内另有语句） | 2485 | 1137 | 逐字节等同 | 逐字节等同 |
| L | 形参 + `"\t"`（体内另有语句） | 2334 | 1113 | 逐字节等同 | **typed 拒绝**（组件层） |
| M | 嵌套 `concat`（体内另有语句） | 2447 | 1112 | 逐字节等同 | 逐字节等同 |
| N | 形参 + 普通字面量 `"x"`（体内另有语句） | 2333 | 1113 | 逐字节等同 | 逐字节等同 |
| O | `"a"` + `"\t"`（体内另有语句） | 2486 | 1137 | 逐字节等同 | **typed 拒绝**（组件层） |
| Q | 形参 + `"\n"`（体内另有语句） | 2334 | 1113 | 逐字节等同 | **typed 拒绝**（组件层） |
| R | 形参 + `"\t"`（体内只有 return） | 2334 | 1113 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| S | **两个函数各带一个字面量** | 3060 | 1188 | 逐字节等同 | 逐字节等同（**本片修正后**） |
| T | **同一字面量出现两次** | 2673 | 1145 | 逐字节等同 | 逐字节等同（**本片修正后**） |
| U | 字面量只出现在后一个函数 | 2721 | 1155 | 逐字节等同 | 逐字节等同 |

⇒ **逐字节等同 12 行**、**组件层 typed 拒绝 3 行**、**前端 typed 拒绝 4 行**、**修正前静默错字节 1 行（已修）**。

⇒ **「某形状被拒绝」≠「该形状没实现」**：H / I / J 被拒是**路由门**（函数体只有一条 `return`）而不是能力缺失 —— 加一条无关语句后，**同样两个操作数**（K / N / M）就逐字节降下来了。

## 3. 四个缺口（按 S 内是否出现排序）

| 缺口 | 症状 | 根因 | S 内出现？ | 处置 |
|---|---|---|---|---|
| **G-b** 含 JSON 转义的字面量（`"\t"` / `"\n"`） | 组件层 typed 拒绝 | `cgs_literal_text` **故意**拒任何含 `\` 的字面量（guest 不反转义 JSON 字符串） | **是**（S 内 1,576 个调用点的操作数是字面量） | **待修**：全族共享前置 |
| **G-d** 单语句 `return` 的非形参操作数 | 前端 typed 拒绝 | 路由梯把 `straight_cell_*` 家族门控在 `body_statement_count > 0`；专用路由只认两个 `Text` 形参 | **是**（10 个调用点） | **待修**：前端 |
| **G-a** 模块级 data 段偏移 | **同长但字节不同**（数据指针差 1）⇒ **静默错字节** | `cgs_data_offset` 只扫**当前函数**的指令，而后端的静态 data 段收**整个模块**（函数 → 块 → 指令） | **是** | **本片已修正** |
| **G-c** 重复字面量 | 组件层 typed 拒绝 | `cgs_data_offset` 把重复字面量当作不可推导 | **是** | **本片已修正** |

### 3.1 S 内 T1 调用点普查（口径：文本出现次数，`target/tmp/step0521/census_s.py`）

| 口径 | 数值 |
|---|---|
| S = `selfhost/*.sico` 9 个文件 | 2,941 个 `sico.text.concat(` 文本调用点 |
| 其中**操作数含字面量** | 1,576（⇒ **G-b** 是固定点的硬阻塞） |
| 落在**没有 `let` / `set`** 的函数体里 | **10**（⇒ **G-d** 是固定点的硬阻塞） |
| 操作数形态 | 形参+字面量 1329 / 形参+调用 752 / 形参+形参 559 / 字面量+形参 102 / 字面量+调用 101 / 调用+调用 51 / 调用+字面量 44 / 调用+形参 3 |

> **口径说明**：STEP-0520 记的 **2,857** 是 **IR 级**（Rust 前端实际降出来的调用点）计数；本表的 **2,941** 是**文本**计数（含 `if` / `while` 体内的调用点）。两个数字口径不同，**不互相矛盾**。

⇒ 按 **D10**：G-b 与 G-d 在 S 内**真实存在** ⇒ **T1 族不得降级**，必须继续闭合。

## 4. 落点（实施记录）

| 文件 | 改动 |
|---|---|
| `selfhost/codegen.sico` | **重塑 1 个函数** `cgs_data_offset`（当前函数 ⇒ 模块级遍历 + 去重 + 首次出现解析）+ **改 1 处** `cgs_const_string_hex`（拆开嵌套宿主调用）+ **改 1 段注释** `cgs_data_hex` ⇒ 文件 4278 → **4303 行**（**+25 行**），函数数 **149 → 149** |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新增 **3** 条钉（族级真值表 18 行 + 模块级 data 偏移 2 行 + 自举前沿 2 个函数）⇒ 固件 **138 → 141** |
| `tools/probe-m22-frontend-frontier.ps1` | 新增 **3** 条用例（28 → **31**），三条都判 `lowered` |
| `docs/reports/m22-support-surface-t1-text-concat.md` | **新增**：D15 支持面登记表 |
| oracle 生产代码（`crates/*`） / `selfhost/parser.sico` | **零改动** |

### 4.1 修正的到底是什么（G-a / G-c）

后端的静态 data 段（`crates/sico-codegen-wasm/src/lib.rs` 的 `script_data`）收**整个模块**的字符串字面量：函数顺序 → 块顺序 → 指令顺序，**重复的字面量保留第一次出现的字节**。

guest 的 `cgs_data_offset` 原来只扫**当前函数**的指令 ⇒ 两个后果：

1. **G-a（静默错字节）**：前面的函数也带字面量时，算出来的偏移偏小 ⇒ 发射出的组件**长度与 oracle 相同、但字节不同**（实测：字节 612 处 `00` vs `01`，即数据指针）。这是 fail-closed 纪律唯一不允许的结果 —— **不是拒绝，而是发出错误的字节**。
2. **G-c（误拒）**：重复字面量被判成「偏移不可推导」⇒ 整个程序被拒，而后端**支持**这种程序（去重）。

修正后 `cgs_data_offset` 与 `cgs_data_hex` **同构**（都是模块级遍历 + 去重），因此两者的偏移与字节必然一致；重复字面量解析到**第一次出现**的偏移。

### 4.2 自举形状前沿：**本片把两个函数拉回了前沿内**

修正前的 `cgs_data_offset` / `cgs_const_string_hex` 里有一处**嵌套宿主调用**（`sico.bytes.length(sico.text.encode(...))`），guest 前端对它报 `ERR:E-SH-IR-CALL-SHAPE` ⇒ 这两个函数当时**在自举形状前沿之外**。

按 STEP-0515 已经用过的修法（拆成临时变量），本片把嵌套调用拆开：

```
let cdo_literal_bytes = sico.text.encode(cdo_literal)
let cdo_literal_len = sico.bytes.length(cdo_literal_bytes)
```

⇒ 实测 `cgs_data_offset`（28,205 B）与 `cgs_const_string_hex`（9,963 B）**现在都能被 guest 前端逐字节降低** ⇒ 新钉 `sico_compiler_lowers_the_step0521_codegen_functions_byte_exactly`。

> **仍然在前沿之外**：`cgs_literal_text`（G-b 的闸门所在）依旧报 `ERR:E-SH-IR-CALL-ARITY` ⇒ **G-b 的实现必须同时把它拉回前沿内**（已登记进支持面登记表 §3.2）。

### 4.3 改动面自证（独立见证）

`target/tmp/step0521/audit_footprint.py` 把本片的 **3 处改动逐条反向撤销**，重建出本片之前的文件：

```
STEP-0521 reverse-applied footprint
  reconstructed_lines=4278
  current_lines=4303
  delta_lines=25
STEP0521_FOOTPRINT_GREEN edits_reversed=3
```

**交叉验证（独立见证）**：重建文件再喂给 **STEP-0520 的足迹审计脚本**（独立写的 11 处反向撤销），得到

```
  reconstructed_lines=3980
  current_lines=4278
  delta_lines=298
STEP0520_FOOTPRINT_GREEN edits_reversed=11
```

⇒ **3980 / 4278 / +298 三个数字与 STEP-0520 当时记录的值逐个相同** ⇒ 重建文件**就是**本片之前的那份，不多不少。

### 4.4 自造缺陷 ①：**「普查数字」与「钉住数字」打架 ⇒ 转写错了一行**

第一版普查（临时探针）记的「两个字面量实参 + 体内另有语句」是 **2446 B / 1126 B**；把它写进钉之后第一遍跑就红：**实测 2485 B / 1137 B**。

**根因**：普查转写进证据文件的那一行**形状描述与探针里的实际源码不一致**（探针那一版用的是别的扰动形状）。

**修法**：把「期望值」改成**在钉的同一 fixture 里实测**（临时测量测试跑同一组 20 行，读出 IR 字节数 / 组件字节数 / guest 判定），再把实测值写进钉 ⇒ 钉一次绿。

**教训**：**自证脚本与证据文件也会说谎** —— 证据文件里的数字必须与**产生它的那份源码**同源；跨脚本抄数字必须能被钉当场复核。

### 4.5 自造缺陷 ②：**电池里出现一个「假红」**

第一次跑本机电池时，`probe-frontend-frontier` 判 `MOVED`，但移动的是 `scalar-identity`，实际输出是

```
failed:sico.exe : sico: source cache failure: Io("C:\Users\64473\AppData\Local\sic...
```

⇒ **不是语义移动，是 CLI 的 source cache 一次性 I/O 失败**（环境抖动）。单独复跑该探针 ⇒ `FRONTIER_UNCHANGED`（28/28）。

**处置**：本片的电池**整轮重跑**，且把该步的判定写成「先看失败形态，再判定是语义移动还是环境抖动」—— 探针脚本本身的 `MOVED` 标签把两种失败混在一起，这是**探针脚本的已知弱点**（登记为后续改进项，不在本片修）。

## 5. 出口测试（钉）

| # | 钉 | 断言 |
|---|---|---|
| 1 | `sico_codegen_guest_pins_the_t1_family_truth_table` | **18 行**族级真值表：每行的 oracle IR 字节数 / oracle 组件字节数 / guest 前端判定 / guest 组件判定；`Exact` 行要求**逐字节等同**且两侧都过真实 `wasmparser::Validator`；拒绝行要求**typed 身份不变**（前端 `ERR:E-SH-IR-EXPRESSION` / 组件层 `unsupported codegen source shape`） |
| 2 | `sico_codegen_guest_derives_the_module_wide_data_offsets` | **G-a / G-c** 两个形状：两个函数各带一个字面量（1188 B）与同一字面量出现两次（1145 B）⇒ 组件与 oracle **逐字节等同** + 两侧 Validator |
| 3 | `sico_compiler_lowers_the_step0521_codegen_functions_byte_exactly` | **自举形状前沿**：本片重塑的 2 个函数经 guest 自己逐字节降低 == Rust 前端 ⇒ **未推进形状边界**（反而把两个函数**拉回**了边界内） |

⇒ 固件从 **138** 增至 **141** 条。

## 6. 诚实性

### 6.1 T1 族**仍未闭合**

- 族级出口判据五项：`canonical IR` ✅ / `整组件` ✅（部分形状 typed 拒绝）/ `Validator` ✅ / `前沿探针` ✅ / `固件电池` ✅ / **独立 CI ⏳（已随批次提交 `a5e0b65` 推送触发 **run #28**，head_sha `a5e0b65`，**尚未出结论**）**。
- **G-b**（转义字面量）与 **G-d**（单语句 return 的非形参操作数）在 S 内**真实存在** ⇒ 按 D10 **不得降级** ⇒ 收口片 X 必须等这两刀做完。

### 6.2 本片**没有**做的事

- **没有**认领 G-b / G-d；**没有**认领其它内建；**没有**认领 `cgs_literal_text` 的前沿缺口。
- **没有**改 oracle 生产代码（`crates/*` 零改动）、**没有**改 `selfhost/parser.sico`。
- **没有**改任何预算常量、**没有**改任何既有断言的期望值（只**新增** 3 条钉）。
- **语料覆盖仍 0 条**；**整文件自举仍未关闭**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）⇒ **门 3 未动，M22 仍为 NO-GO**。
- **未提交、未推送**；**独立 CI 尚未运行**。

### 6.3 已知风险

- `cgs_data_offset` 现在每个 `const_string` 指令都要**扫一遍整个模块** ⇒ 偏移推导是 **O(字面量数²)**。STEP-0516 已经量过「字节组装必须线性」的教训；**模块规模（W5 的 1 MB 级组件）下这一项必须重测**（登记为 W5 的前置风险，本片不处理）。
- 探针脚本把「环境抖动」和「语义移动」都标成 `MOVED`（§4.5）⇒ 需要改进，本片不处理。

## 7. 验证记录（2026-10-23）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉（3 条定向） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --exact <3 条> --test-threads=1` | ✅ **3 passed / 0 failed** |
| 固件全量 | 同上，去掉 `--exact` | ✅ **141 passed / 0 failed** |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | ✅ **31/31** ⇒ `FRONTIER_UNCHANGED` |
| 改动面审计 | `python .\target\tmp\step0521\audit_footprint.py` | ✅ `STEP0521_FOOTPRINT_GREEN edits_reversed=3`（**+25 行**） |
| 独立交叉验证 | `python .\target\tmp\step0521\audit_step0520_copy.py` | ✅ `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（**3980 / 4278 / +298**，与 STEP-0520 记录值逐个相同）⇒ `STEP0521_CROSSCHECK_GREEN` |
| S 内调用点普查 | `python .\target\tmp\step0521\census_s.py` | ✅ 2,941 个文本调用点 / 1,576 个含字面量 / 10 个无 `let`+`set` |
| 注释纪律 | `python .\tools\validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 行尾审计 | `python .\target\tmp\step0507\audit_lines.py` | ✅ `LINE_ENDING_AUDIT_GREEN`（**29** 个文件，含本片新增的 STEP-0521 文档与支持面登记表） |
| 本机验证电池 | `target/tmp/step0521/battery.ps1`（**17 步**） | 见 §7.2 |

### 7.2 本机验证电池（17 步）

本轮电池 **17/17 全绿**（`target/tmp/step0521/battery.ps1`，日志 `target/tmp/step0521/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 429 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试 ⇒ **固件 141 条在本步已跑过**） |
| `validate-0245` | 139 | 0 | 内含固件全量 + clippy（`-D warnings` 零警告） |
| `validate-0262` | 152 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 209 | 0 | 前端前沿 **31/31** ⇒ `FRONTIER_UNCHANGED`（本片**前移** 3 条新用例） |
| `probe-ir-plan-frontier` | 26 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN`（**29** 个文件） |
| `selfhost-comment-discipline` | 0 | 0 | `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| `step0520-footprint-audit` | 0 | 0 | `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（**+298 行**） |
| `step0520-crosscheck-0519` | 0 | 0 | `STEP0519_FOOTPRINT_GREEN edits_reversed=9`（**3745 / 3980 / +235**） |
| `step0521-footprint-audit` | 0 | 0 | `STEP0521_FOOTPRINT_GREEN edits_reversed=3`（**+25 行**） |
| `step0521-crosscheck-0520` | 0 | 0 | `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（**3980 / 4278 / +298**）⇒ `STEP0521_CROSSCHECK_GREEN` |
| `step0521-census-s` | 1 | 0 | S 内调用点普查（只读） |

⇒ `BATTERY_GREEN 17/17`。

> **本片之前的两次电池记录（留痕）**：①STEP-0520 自己的 16 步电池在 `probe-frontend-frontier` 上判**红**，但失败形态是 CLI 的 `source cache failure: Io(...)` **环境抖动**（不是语义移动，单独复跑 `FRONTIER_UNCHANGED`）；②本片的 17 步电池在**改动落地后整轮重跑**，全绿。

### 7.3 未做的事

- **未认领**前端未闭合的形状；**未认领** `Bytes` / `List[Text]` / 记录型 cell；**未认领**其它内建；
- **已提交并推送**（批次提交 `a5e0b65` ⇒ `codex/m22-w1-ci`）；**独立 CI run #28 已触发、尚未出结论**；
- **不推进任何出口门**，M22 仍为 NO-GO。

## 8. 下一刀

1. **G-b：转义字面量**（全族共享前置）—— guest 需要能反转义 JSON 字符串（`\\` `\"` `\n` `\r` `\t` `\b` `\f` `\u00XX`），并且 `cgs_literal_text` 必须**同时**被拉回自举形状前沿内；这是 S 内 1,576 个调用点的硬阻塞。
2. **G-d：单语句 `return` 的非形参操作数**（前端）—— S 内 10 个调用点的硬阻塞；能力已存在（`straight_text_concat_packed` / `gw_text_concat_packed`），缺的是可达性。
3. 两刀做完 ⇒ **T1 收口片 X**（真值表复跑 + 登记表定稿 + 前沿探针 + 电池 + **独立 CI**）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。
