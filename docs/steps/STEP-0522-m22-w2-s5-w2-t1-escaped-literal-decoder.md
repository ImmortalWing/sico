# STEP-0522 m22-w2 S5 **W2 族 T1 的 G-b 刀**：转义字面量解码 + 空字面量（G-e）

> - status: **implementation complete locally**（本机验证电池 **16/16 `BATTERY_GREEN`**；随本片提交并推送；独立 CI 裁决待下一批回填）
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0521](./STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表 + D15 登记）、[STEP-0520](./STEP-0520-m22-w2-s5-w2-shared-local-cell-support.md)（cell 共享前置）、[STEP-0519](./STEP-0519-m22-w2-s5-w2-t1-codegen-knife.md)（codegen 刀）
> - gate: **不推进任何出口门** —— 本片是 **T1 族的收口前置片之一**（G-b / G-e 两刀）；**T1 族仍未闭合**（G-d 在 S 内真实存在）
> - numbering: **STEP-0522**（取号前已核对 `docs/steps/`：最大号 0521；*0522 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15，已更新）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| STEP-0521 的 D15 登记表把 **G-b** 列为 T1 族的硬阻塞（转义字面量在组件层 typed 拒绝）。 | ✅ 已登记 |
| G-b 的位置已定位到**唯一闸门**：`cgs_literal_text` 故意拒任何含 `\` 的字面量。 | ✅ STEP-0521 §3 |
| 真值可测：后端 `script_data` 收的是 `value.as_bytes()`（**已解码**的字节）。 | ✅ 见 §1 |
| 前置依赖已闭合：cell（STEP-0520）、族级真值表（STEP-0521）。 | ✅ |

## 1. 这一片回答的问题

**guest 能不能把一个带 JSON 转义的字面量编成与 Rust 后端逐字节相同的组件？**

## 2. 真值测量（改前 / 改后，均来自生产输出）

测量口径：**Rust 后端 = `sico build --profile script-v0` 的生产组件**（与固件里的 `compile_script_program` 同一函数）；**guest = `selfhost/compiler.sico --emit-component-hex`**（真燃料预算 800B，与固件同一 runner）。

fixture：`first` 带 `"tab\there"`、`second` 带 `"nl\nhere"`（entry 为 `run`；CLI 变体入口名 `main`，内部改名 `run` ⇒ 两侧模块等价）。

| 时点 | Rust 后端组件 | guest 组件 | 静态 data 段 |
|---|---|---|---|
| **改前** | 1197 B | **typed 拒绝**（`ERR:E-SH-CG-IR-EMIT` ⇒ 宿主侧 "unsupported codegen source shape"） | —— |
| **改后** | 1197 B | **1197 B，逐字节相同** | `74616209686572656e6c0a68657265`（15 B）= `tab<TAB>here` + `nl<LF>here` |

⇒ **真值**：后端的静态 data 段放的是**解码后的字节**（TAB = `09`，LF = `0a`），**不是**源码里的两个字符 `\t`（`5c 74`）。guest 的 IR 里字面量保持 JSON 形式，所以 guest 必须在**发射端**解转义。

> **口径说明**：STEP-0521 真值表里 L / O / Q 三行「组件层 typed 拒绝」就是这个闸门；本片把三行全部变成「逐字节等同」。

## 3. 落点（实施记录）

落点文件：`selfhost/codegen.sico`（**oracle 生产代码 `crates/*` 零改动**）。

### 3.1 核心改动：字面量的身份 = 它的**解码后字节**

| 改前 | 改后 |
|---|---|
| `cgs_literal_text(bytes, instr) -> Text`：返回字面量的**源码文本**，遇 `\` 直接拒绝 | `cgs_literal_hex(bytes, instr) -> Text`：返回字面量**解码后字节的 hex 字节对** |
| `cgs_data_offset(bytes, text)`：按**文本**去重、按**文本**长度累计偏移 | `cgs_data_offset(bytes, hex)`：按**解码后字节**（hex）去重、按**字节**累计偏移 |
| `cgs_data_hex`：用 `cgs_text_bytes_hex(text)` 把文本转成 hex | `cgs_data_hex`：直接用 `cgs_literal_hex` 的 hex |

**为什么必须换身份**：后端 `script_data` 用**字节**（`value.as_bytes()`）做 key 去重、并按**字节**算偏移。guest 若按源码文本做身份，则「`"\t`」与「`"\t`」（同一字节、不同拼法）」会被当成两个字面量 ⇒ 偏移错 ⇒ **静默错字节**（fail-closed 纪律唯一不允许的结果）。换成 hex 之后，guest 的身份与后端**逐字节同构**。

### 3.2 新增函数（4 个全新 + 1 个改名重塑）

| 函数 | 作用 |
|---|---|
| `cgs_literal_hex(bytes, instr)` | `cgs_literal_text` 的**改名重塑**（删除旧名）：返回字面量**解码后字节的 hex 字节对**；不可解码 ⇒ `ERR:E-SH-CG-IR-EMIT` |
| `cgj_string_hex(bytes, start, close)` | JSON 串 → **解码后字节的 hex**；逐字节扫描，遇到 `\` 走转义分支；无法解码时返回 `ERR:E-SH-CG-IR-EMIT`（typed 拒绝，不静默近似） |
| `cgj_escape_hex(escaped)` | 简单转义 → 字节：`\"`→`22`、`\\`→`5c`、`\n`→`0a`、`\r`→`0d`、`\t`→`09` |
| `cgs_is_const_string(bytes, instr)` | 「这条指令是不是 `const_string`」——把**空字面量**（hex 为空）和**不是字面量**分开（G-e 的哨兵） |
| `cgs_hex_bytes(hex)` | hex 字节对 → 字节数（按 2 步长计数） |

`\u0000`（IR 生产者把源码 `\0` 改写成的 JSON 转义）在 `cgj_string_hex` 里**内联**处理：它跨 6 个字节，必须连同 4 个 `0` 一起消费掉。

### 3.3 顺带修掉：空字面量（G-e）

普查时**新发现**的缺口：`""`（空字面量）此前也被拒（`cgs_literal_text` 的长度检查把空当成「不是字面量」）。后端对它的处理是：进 `data_offsets`（占一个 key）、**贡献 0 字节**、指针 = 当前 data 长度。

⇒ 改后：`cgs_literal_hex` 对空字面量返回**空 hex**（合法值，不是拒绝），`cgs_data_offset` 的遍历用 `cgs_is_const_string` 当闸门（不再用「长度 > 0」），因此空字面量**能被定位到**，指针与后端一致。

### 3.4 顺带删掉：`cgs_text_bytes_hex`

它唯一的作用是「文本 → hex」，在字面量身份换成 hex 之后**没有调用点了**（死代码）。删掉，不留悬空函数。

### 3.5 规模

| 项 | STEP-0521 后 | 本片后 |
|---|---|---|
| `selfhost/codegen.sico` 行数 | 4,303 | **4,413**（**+110**） |
| 函数数 | 149 | **152**（+5 新增 −2 删除：`cgs_literal_text` / `cgs_text_bytes_hex`） |

## 4. S 内普查（口径：文本调用点，`target/tmp/step0522/census_s_literals.py`）

| 口径 | 数值 |
|---|---|
| S = `selfhost/*.sico` 9 个文件 | **2,943** 个 `sico.text.concat(` 调用点（STEP-0521 记 2,941；差的 2 个是**本片新增**的两处 concat，见 §3.2） |
| 操作数形态（**按操作数**计，共 5,888 个） | 名字 3,308 / 调用或成员 1,004 / **带转义的字面量 952** / 普通字面量 619 / **空字面量 5** |
| ⇒ **G-b** | S 内 **952** 个带转义的字面量操作数 ⇒ 硬阻塞（**本片已闭合**） |
| ⇒ **G-e** | S 内 **5** 个空字面量操作数（都在 `parser.sico`）⇒ 硬阻塞（**本片已闭合**） |

> **口径说明**：STEP-0521 的表按**调用点**记形态对（形参+字面量 1329 …），本表按**操作数**记，两表口径不同、**不互相矛盾**。

## 5. 自举形状前沿：**本片把 G-b 的闸门函数拉回了前沿内**

STEP-0521 记录：`cgs_literal_text` 仍在自举形状前沿之外（guest 前端报 `ERR:E-SH-IR-CALL-ARITY`），而它的同族函数都能逐字节降低。

本片把字面量逻辑整体搬到**字节域**（hex 字节对），于是**不再需要任何 text 形状的宿主调用** ⇒ 前沿问题**自然消失**，不是靠改形状硬挤过去的。

前沿实测（`target/tmp/step0522/frontier.py`，走真燃料 runner）：

| 函数 | STEP-0521 | 本片 |
|---|---|---|
| `cgs_literal_text` / `cgs_literal_hex` | `ERR:E-SH-IR-CALL-ARITY`（**在前沿外**） | **lowered** |
| `cgs_data_offset` | lowered | lowered |
| `cgs_const_string_hex` | lowered | lowered |
| `cgs_data_hex` | lowered | lowered |
| `cgs_is_const_string` / `cgj_string_hex` / `cgj_escape_hex` / `cgs_hex_bytes` | —— | **lowered** |

⇒ 8 个新增/改动函数**全部**能被 guest 前端逐字节降低（新钉）。

> **注意**：`U64.checked_div` 这条路由**不在** guest 前端的路由梯里（实测 `ERR:E-SH-IR-CALL-TARGET`）⇒ 字节数改用**按 2 步长计数**（`cgs_hex_bytes`）得到，**不引入除法**。

## 6. 撞到并修掉的 3 条自造缺陷

1. **`\u0000` 只消费了 2 个字节**：第一版解码器把 `\u0000` 认成 NUL 后只跳过「`\` + `u`」，4 个 `0` 被当成字面量字节 ⇒ guest 的 data 段多了 `30 30 30 30`。**差分测出来**：guest 1429 B vs 后端 1429 B 但 data 段 `...6e756c00 30303030 62...` vs `...6e756c00 62...` ⇒ 改成连 4 个数字一起消费（§3.2）。
2. **形参名用了保留字 `end`**：`cgj_string_hex(bytes, start, end)` ⇒ Rust 前端拒 `unsupported atomic expression at bytes 94054..94057` ⇒ 改名 `close`。
3. **哨兵歧义**：空字面量与「解码失败」都返回 `""` ⇒ 无法区分 ⇒ 改用 `ERR:E-SH-CG-IR-EMIT` 标记解码失败（§3.2 / §3.3）。

## 7. 出口测试（钉）

| 钉 | 断言 |
|---|---|
| `sico_codegen_guest_emits_the_escaped_literal_component_byte_exactly` | 4 个 fixture（`\t`+`\n` / `\"`+`\\`+`\r` / `\0` / 空字面量）的 guest 组件与 Rust 后端**逐字节相同**，组件尺寸钉住（**1201 / 1279 / 1122 / 1190** B），两侧都过 `wasmparser::Validator` |
| `sico_codegen_guest_derives_the_decoded_literal_bytes` | 6 种转义（`\t` `\n` `\r` `\0` `\"` `\\`）的**解码后字节**（`09` `0a` `0d` `00` `22` `5c`）确实出现在静态 data 段里（后端侧与 guest 侧各验一次），组件逐字节相同 |
| `sico_codegen_guest_refuses_the_unknown_escape_with_a_typed_identity` | 未知转义（`"\q"`）**两侧前端都 typed 拒绝**（Rust lexer 拒 / guest `ERR:E-SH-IR-STRING-ESCAPE`），组件路径给通用身份 |
| `sico_compiler_lowers_the_step0522_codegen_functions_byte_exactly` | 8 个新增/改动函数能被 guest 前端**逐字节降低**（自举前沿） |

另外：**STEP-0521 的族级真值表钉已按新实测更新**（L / O / Q 三行从「组件层 typed 拒绝」变成「逐字节等同」⇒ 逐字节等同 12 → **15** 行、组件层 typed 拒绝 3 → **0** 行）；`GuestVerdict::CodegenRefused` 变体随之**不再有任何行使用** ⇒ 删除（否则 `dead_code` 会让 clippy `-D warnings` 变红）。

固件：141 → **145** 条。

## 8. 诚实性

### 8.1 T1 族**仍未闭合**

| 未闭合项 | 状态 |
|---|---|
| **G-d**（单语句 `return` 的非形参操作数） | ⏳ **仍在前端**（S 内 10 个调用点）⇒ 按 D10 **不得降级** |
| **独立 CI** | 本片推送后触发（上一批 `a5e0b65` 的 **run #28 = success** ✅） |
| 语料覆盖 | **0 条**（S 内 2,943 个调用点一个都没进语料） |
| 整文件自举 | **仍开**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`） |

⇒ 本片**不推进任何出口门**，M22 仍为 **NO-GO**。

### 8.2 本片**没有**做的事

- 没有认领 **G-d**（前端路由门）。
- 没有认领其它内建；T1 仍只认 `sico.text.concat`。
- 没有把语料覆盖从 0 变成 1。
- 没有动 `crates/*` 的任何生产代码（oracle 零改动）。
- 没有动 `selfhost/parser.sico`（前端零改动 ⇒ 前沿探针的**既有 31 条期望值全部不变**，本片只**新增** 3 条）。

### 8.3 本片解码器的**接受边界**（实测 + 读码器源码）

| 转义 | 处理 | 依据 |
|---|---|---|
| `\"` `\\` `\n` `\r` `\t` | 解码成各自那一个字节 | Rust 前端 `unquote`（`crates/sico-ir/src/lower.rs`）只认这 6 种里的 5 种 + `\0` |
| `\u0000` | 解码成 NUL | guest IR 生产者 `json_string_token`（`selfhost/parser.sico`）把源码 `\0` 改写成它 |
| 其它 `\uXXXX` / 未知转义 / 结尾孤立反斜杠 | **typed 拒绝** `ERR:E-SH-CG-IR-EMIT` | fail-closed：宁可拒绝，不静默近似 |

> **实测**：未知转义（如 `"\q"`）**两侧前端都先拒** —— Rust lexer 报 `UnknownStringEscape`，guest lexer 报 `ERR:E-SH-IR-STRING-ESCAPE` ⇒ 解码器那道 `ERR:E-SH-CG-IR-EMIT` 是**第二道防线**，前端不会让它透过去（新钉 `sico_codegen_guest_refuses_the_unknown_escape_with_a_typed_identity`）。

> **未测的边界**：源码字面量里**裸控制字符**（例如裸 TAB）在 serde 的 canonical IR 里会被写成 `\t`，而 guest 的 IR 生产者原样拷贝 ⇒ 那种源码的 **IR 逐字节等同**不成立（**本片未测，未认领**）。数据段字节仍正确（解码后的字节一致）。

### 8.4 已知风险

- `cgs_data_offset` 现在每个 `const_string` 扫一遍整个模块 ⇒ 偏移推导 **O(字面量数²)**（STEP-0521 已登记）；本片未改变这个量级。
- `cgs_data_hex` / `cgs_data_offset` 对**每个**字面量做一次 hex 扫描 ⇒ 发射期开销随字面量数线性增长。

## 9. 验证记录（2026-10-23）

### 9.1 固件

- 新钉 4/4 绿；固件全量 **145 / 145**（Windows x64，`RUST_TEST_THREADS=1`；141 → 145）。

### 9.2 前沿探针

- 31 → **34** 条（新增 `\0` 实参、空字面量实参两条；另把已能通过的形状再钉一条），全部 `lowered` ⇒ `FRONTIER_UNCHANGED`。

### 9.3 本机验证电池（16 步）

电池脚本：`target/tmp/step0522/battery.ps1`（16 个 `Run-Step`）。实测 **BATTERY_GREEN 16/16**（2026-10-23，Windows x64）：

`fmt-runner` / `ci-one-key` / `validate-0245` / `validate-0262` / `probe-frontend-frontier` / `probe-ir-plan-frontier` / `validate-0124` / `validate-0131` / `validate-0156` / `git-diff-check` / `line-ending-audit` / `selfhost-comment-discipline` / `step0522-footprint-audit` / `step0522-crosscheck-0521` / `step0522-census-s` / `step0522-differential` —— **16/16 exit=0**。

> **口径说明**：STEP-0521 的两条足迹审计（`step0521-footprint-audit` / `step0521-crosscheck`）在本片**被替换**成 `step0522-footprint-audit` / `step0522-crosscheck-0521`（前者反向撤销 STEP-0521 的改动，而本片又重塑了那些函数），因此步数是 **16** 而不是 17。替换掉的交叉见证仍然成立：STEP-0521 的审计脚本指向本片重建的文件后，仍报 STEP-0521 当时记下的数字。


### 9.4 未做的事

- 未跑跨平台矩阵（仍只有 Windows x64 原生证据）。
- 未推语料（`selfhost/corpus-v0.json` 仍 0 条本族覆盖）。

## 10. 下一刀

**G-d**（前端可达性：函数体只有一条 `return`、操作数不是两个 `Text` 形参）⇒ 之后才是 **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + 独立 CI 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。
