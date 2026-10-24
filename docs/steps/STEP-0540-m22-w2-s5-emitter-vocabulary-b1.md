# STEP-0540 m22-w2 S5 **发射词表 B-1 实现片（C 拍）**：`sico.bytes.equal` 落地（首个 core helper）+ 泛化 helper 排序位

> - 性质：**实现片**（组件 C 拍）。**`crates/*` 零改动**；**`selfhost/parser.sico` 零改动**（本片只动发射端 ⇒ 前端前沿不动）；**不推进任何出口门**；**M22 仍为 NO-GO**（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO）；**`match` 族仍未闭合**（组件级全等 **23 / 396**）。
> - 归属：[M22 计划 §3.3.5](./../plans/M22-compiler-self-host.md) 的「每族四拍（真值 T → 前端 F → 组件 C → 收口 X）」的对象 = **发射词表 B** 的**第一刀**。承 [STEP-0539](./STEP-0539-m22-w2-s5-emitter-vocabulary-a2.md)（A-2：`sico.list.get` / `sico.bytes.slice` + 四槽 numeric Result 打通）。
> - 产物：`selfhost/codegen.sico`（**+154 行**，5743 → **5897** 行）+ 固件改钉与新钉 + 自举前沿钉扩容（18 → **20** 个函数名）+ 站点账本 v1 刷新（**9 个站点 verdict 前移**）+ 发射词表工件刷新（163 → **154** 站点，站点口径见 §2.5）+ 电池 15 步不变。
> - **上一片的独立 CI 裁决（本片回填）**：STEP-0539 的提交 `f01e923` ⇒ run **#50** = `completed` / **`success`**（id `40113318393`，`windows-gnu` job，16:49:21Z → 17:26:10Z = **36m49s**）⇒ **STEP-0539 的独立 CI 裁决为 GO**。

## 0. 入口门槛

- **缺口是真的**：STEP-0537 的普查在组件被拒站点上逐站点命名了缺口；发射词表 B 的 **8 项 core helper** 是最后一批；`sico.bytes.equal` 是其中**覆盖站点最多**的一项（**156** 个站点）。
- **本片只做 B-1**：它是 B 里的**第一刀**，也是**排序位最靠前**的一项（`sico.bytes.equal` < `sico.bytes.utf8_decode` < `sico.text.concat` < `sico.text.length`）⇒ 它的落地**会把已有三项的 helper 索引各推后一位** ⇒ 必须先把「按名字写死的偏移」**泛化成排序位**。
- **不能按预测改钉**：STEP-0538 §2.3 点名的 9 个站点是**预测**，本片全部**实测**后才动账本与钉。

## 1. 本片方法

1. **先量后写**：用 Rust oracle 的**生产输出**（`compile_script_program`）作为唯一真值，先把两个形状的组件字节量出来，再写 guest 的发射字节。
2. **发射端落点**（`selfhost/codegen.sico`）：
   - **一个冻结的 helper 体** `cg_bytes_equal_helper_body_hex`：**62 B**（locals `01 01 7f` + **59 B** 代码），**无 `call` 指令** ⇒ 不含函数索引 ⇒ **可逐字冻结**（与 `cg_utf8_validate_helper_body_hex` 同一判据）。
   - **一个泛化的 helper 索引** `cgs_helper_index`：**helper 索引 = `模块函数数 + 3 + 名字排序位`**；排序位**由模块自己用到的支撑 helper 集合算出**，不再对每个 helper 写死偏移。
   - **一个内建发射器** `cgs_bytes_equal_hex`：把两个 `(pointer, length)` 对透传给 helper，再把 helper 的答案写进结果槽。
   - **四道闸门全 fail-closed**：名字 / 实参个数 / 实参类型（**bytes**）/ 结果槽数（= **1**），任一不过即 `ERR:E-SH-CG-IR-EMIT`（ADR-0016 决策 6）。
   - **三段接线**：类型段（`60047f7f7f7f017f`）/ 函数段（`+1`）/ code 段（helper 体）**都按 backend 字母序**插入 ⇒ `sico.bytes.equal` 排在 `sico.bytes.utf8_decode` **之前**。

## 2. 实测结果

### 2.1 逐字节等同的形状

| 组 | 形状 | oracle 组件 | 结论 |
|---|---|---|---|
| 内建 `sico.bytes.equal` | 两段 `sico.text.encode` 的相等比较（函数名 `size`） | **1130 B** | ✅ 逐字节等同 |
| 内建 `sico.bytes.equal` + `sico.text.concat` | 同一函数体里同时用两个 helper | **1274 B** | ✅ 逐字节等同 |

- 两条形状都过**真实 `wasmparser::Validator`**；比较的是**整个组件**，不是长度。
- **第二个形状是排序位的对照**：`sico.bytes.equal` 与 `sico.text.concat` 同时出现时，两个 helper 的 core 索引必须是**相邻的两个**（`函数数+3` 与 `函数数+4`）⇒ 排序位算错时组件必然不等 ⇒ 这条形状把**索引泛化**钉住了。
- **闸门是活的**：去掉第二个实参 ⇒ **Rust verifier 拒绝**（arity）、**guest 也拒绝**（自己的实参个数闸门）；把实参换成两个 **Text 形参** ⇒ **Rust verifier 拒绝**（实参类型）、**guest 也拒绝**（自己的 bytes 实参闸门）⇒ 不是「什么形状都收」。

### 2.2 helper 索引的泛化（不再是逐 helper 的写死偏移）

| helper | 排序位 | core 索引 |
|---|---|---|
| `sico.bytes.equal` | 0 | `模块函数数 + 3` |
| `sico.bytes.utf8_decode` | 1（当模块用到 `sico.bytes.equal`） | `模块函数数 + 3 + 位` |
| `sico.text.concat` | 2（当模块用到前两项） | `模块函数数 + 3 + 位` |
| `sico.text.length` | 3 | `模块函数数 + 3 + 位` |

- 排序位**由模块自己用到的支撑 helper 集合算出**；**不认识的 helper 由内建分派的 fail-closed 自动挡住**（内建名不匹配任何分派分支 ⇒ 整个组件被拒 ⇒ guest 不需要额外的「未知 helper」检测机制）。
- 旧的写死偏移（`sico.text.concat` 调点 = `count+3` + (utf8?1:0)；`sico.text.length` 调点 = `count+3` + utf8 + concat）**在语义上被泛化版覆盖** ⇒ 既有形状的字节**不变**（§2.1 的两条形状与 STEP-0539 的三条形状都仍逐字节等同）。

### 2.3 新钉与自举前沿钉

- **`the_emitter_vocabulary_b1_covers_the_bytes_equal_helper_shape`**（新）：两个形状的整组件逐字节等同 + 两条**拒绝对照**（实参个数 / 实参类型）。
- **自举前沿钉**（`sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly`）的函数清单 **18 → 20 项**：
  - **新增 3 项**（本片的发射代码）：`cg_bytes_equal_helper_body_hex` / `cgs_helper_index` / `cgs_bytes_equal_hex`；
  - **补钉 2 项**（发射端自己的类型分派）：`cgs_flat_hex` / `cgs_value_ty` —— 这两个函数**此前从未被任何前沿钉覆盖**，本片把它们纳入 ⇒ **前沿覆盖面变完整**（见 §2.3b）。
- 前沿钉的失败信息本片**增强**：拒绝时打印 guest 的**完整 typed outcome**（含拒绝身份），不再只打印「must compile through the guest frontend」⇒ 下一次同类回退可以直接读出身份。

### 2.3b 本片**捕获并回退**的一条自举前沿回退（前沿钉抓到的，不是电池抓到的）

- **经过**：本片先做了一版**发射端分配消除**（把类型分派与槽位/类型查找里的**逐步文本物化**去掉，改为就地按字节比较）。做完把它加进**自举前沿钉** ⇒ **首跑红**：`codegen function cgj_ident_is must compile through the guest frontend` ⇒ **本片新增的发射代码落在了自举前沿之外**。
- **根因（实测定位，不是猜）** = `gw_intrinsic_call_packed`（`selfhost/parser.sico:940`）这条**通用宿主调用路由**要求宿主调用的实参**必须是形参或 cell**：实参既不是形参、又不是 cell 时，路由在 `:1006` 返回 **`ERR:E-SH-IR-CALL-ARGUMENT`**。本片那版分配消除为了拿到「`i64` / `bool` / …」的字节，写成了 `sico.text.encode("i64")` ⇒ **实参是字符串字面量** ⇒ 既不是形参也不是 cell ⇒ 前端按规则拒。
- **对照实验（已做）**：把 `selfhost/codegen.sico` 换回 HEAD 版本，用**同一个前沿钉**测 `cgs_flat_hex` / `cgs_value_ty` ⇒ **绿** ⇒ 确认这条回退是本片那版分配消除引入的，**不是**既有缺口。
- **处置** = **本片回退那版分配消除**（`selfhost/codegen.sico` 恢复到「HEAD + bytes.equal + 排序位泛化」），理由见 §2.6。
- **顺带钉住的既有缺口（F2，见 §2.6）** = 前沿钉在**既有**发射端函数 `cgj_ident_text` 上同样红（它内部有 `sico.text.encode("")`）⇒ **发射端代码早就有一部分在前沿之外**，这是**整文件自举的具名前置**。

### 2.4 账本：组件轴 **14 → 23 / 396**（**9 个站点实测闭合**）

| 站点 | 文件 | 函数 | verdict 变化 |
|---|---|---|---|
| **MS-011** | `parser.sico` | `record_map_comma` | `component-refused` → **`component-equal`** |
| **MS-012** | `parser.sico` | `record_close_bracket` | 同上 |
| **MS-018** | `parser.sico` | `matching_close` | 同上 |
| **MS-023** | `parser.sico` | `fixed_operation_operand_width` | 同上 |
| **MS-038** | `parser.sico` | `fixed_operation_operand_atom_idx` | 同上 |
| **MS-063** | `parser.sico` | `argument_instruction_count` | 同上 |
| **MS-074** | `parser.sico` | `argument_region_instruction_count` | 同上 |
| **MS-219** | `parser.sico` | `construct_field_comma` | 同上 |
| **MS-363** | `parser.sico` | `fixed_operation_operand_value` | 同上 |

- **组件级全等 14 → 23 / 396**；**与 STEP-0538 §2.3 的词表推算逐项一致**（那 9 个站点正是当时点名的同一批）⇒ **预测兑现**。
- **本片同时新登记了 37 个站点的 verdict 变化**：它们从「typed 拒绝」变成 **`component-other:FuelExhausted`**（见 §2.6）——**不是**闭合，**也不是**新的能力声明，是**新披露的预算边界**。

### 2.5 普查工件：剩余缺口**只剩 7 项 helper**

`selfhost/emitter-vocabulary-v1.json` 刷新为 **154** 站点（= 163 − 本片闭合的 9）：

- **缺的算子：0**（`missing_op_frequency` 为空）。
- **缺的内建 = 缺的 helper**，**只剩 7 项**：

| helper | 站点数 | 形态 |
|---|---|---|
| `sico.u64.to_text` | **144** | core helper |
| `sico.text.starts_with` | **136** | core helper |
| `sico.list.get[U64]` | **73** | 集合单态化 helper |
| `sico.text.split_lines` | **60** | core helper |
| `sico.list.empty[U64]` | **16** | 集合单态化 helper |
| `sico.list.append` | **11** | core helper |
| `sico.list.append[U64]` | **6** | 集合单态化 helper |

⇒ **发射词表 B 还剩 7 项**（`sico.bytes.equal` 已落地）。

**本片修正的普查口径（工具侧）**：普查工具原先只收 `ir-equal+component-refused` 的站点 ⇒ 本片新披露的 **37 个 `component-other:FuelExhausted`** 站点会被**排除**在缺口清单之外 ⇒ 缺口被**低报**。本片把口径改成「**canonical IR 已逐字节等同、但组件尚未逐字节等同**」的**全部**站点 ⇒ **154** 个（= 117 拒 + 37 耗尽）。**剩余 7 项 helper 的计数因此不变**：STEP-0539 的 163 站点里被移出的 9 个正是本片闭合的站点，而它们**本就不需要**这 7 项（否则不可能被闭合）⇒ **「发射词表 B 还剩 7 项」的结论不受影响**，但**清单的覆盖面变完整了**。

### 2.6 本片**披露**的新边界：发射端的 Component 组装是**二次的**（未修，已登记）

本片让发射端第一次真正走到大闭包，于是暴露出**发射端自身的规模上限**：

- **实测**：同一个闭包（`parser.sico:nested_intrinsic_return_ir`）在 `--emit-component-hex` 路径上，**800e9 fuel 不足以完成**；把预算抬到 **2e12** 才跑完并给出 typed 拒绝（耗时 67.6 s）。另一个闭包（`parser.sico:checked_arithmetic_match_ir`）需要 **2e12 – 8e12** 之间。
- **对照**：小闭包（`parser.sico:record_map_comma`，组件 **2486 B**）在 800e9 预算内 **1.14 s** 完成 ⇒ 单位成本 ≈ **8e6 fuel / 输出字节** ⇒ 与「成本随模块规模超线性增长」一致。
- **根因（已定位，未修）**：guest 发射端把 Component 当作**十六进制文本**逐段 `sico.text.concat` 拼起来 ⇒ **组装量随输出长度二次增长**；这正是计划 §10.3「**1 MiB 级 IR/Component/hex 中间量：必须保持线性组装**」那条要求所指的对象。
- **本片的处置**：
  - **不上调限额**（AGENTS.md §10.3：「失败后不得上调限额追认」）⇒ 800e9 的 harness 预算**原样保留**；
  - **不加新的尺寸门**（那会把二次组装藏起来，而不是暴露出来）⇒ 37 个站点的 verdict 如实记为 `component-other:FuelExhausted`；
  - **代价（诚实登记）**：这 37 个站点各自要烧满 800e9 fuel（约 40 s），⇒ **组件轴的逐站差分从约 250 s 涨到约 1100 s**（整轮电池与独立 CI 同步变长）。这是**新覆盖的代价**，不是回归。
- **登记为待办**：发射端组装必须改成线性（计划 §10.3 的同一条）；它同时是**整文件自举**（`parser.sico` 全量组件 ~1 MiB 量级）的前置。

### 2.6b 本片**回退**的一条改动：发射端分配消除（**实测否定结果**）

本片先做了一版**发射端分配消除**（把类型分派与槽位/类型查找里的逐步文本物化去掉，改为就地按字节比较），并**逐字节验证**它不改变任何发射字节。它的**测量结论**是：

- **它没有消除预算边界**（实测燃料需求不变）⇒ 它想解决的问题（§2.6 的二次组装）**不是分配造成的**；
- **它把发射端推得更远离自举前沿**（§2.3b）⇒ 逐字节相等 ≠ 在自举前沿之内；实测把前沿钉从 **绿**变成**红**；
- **绕开它的唯一办法是给前端加能力**（让宿主调用接受字面量实参），那是**前端片的活**，不能在 codegen 片里偷渡。

⇒ **本片回退它**（`selfhost/codegen.sico` = HEAD + bytes.equal + 排序位泛化，**+154 行**）。**留下的只有它的测量结论**（§2.6 的边界）与**它暴露出来的两条缺口**：

- **F2（新发现的既有缺口）**：**guest 前端不能降低「实参是字符串字面量的宿主调用」** —— `gw_intrinsic_call_packed` 要求实参是形参或 cell ⇒ 发射端里凡是 `sico.text.encode("<字面量>")` 的代码（例如既有的 `cgj_ident_text`）**都在自举前沿之外** ⇒ 这是**整文件自举的一个具名前置**，登记为下一刀的前端缺口。
- **F3（覆盖面缺口）**：发射端自己的类型分派（`cgs_flat_hex` / `cgs_value_ty`）**从未被任何前沿钉覆盖** ⇒ 本片补钉（§2.3）⇒ 前沿覆盖面变完整。

## 3. 本片**没有**做的事（诚实边界）

| 项 | 状态 |
|---|---|
| **`match` 族闭合** | ❌ **没有**：组件级全等 **23 / 396**；**发射词表 B（剩余 7 项 helper）** 未做 |
| **闭包规模** | └ 仍 **219** 站点组件轴**未测**（闭包超限）⇒ 「未测」不是「通过」 |
| **语料覆盖** | └ 仍 **0 条** |
| **整文件自举** | └ 仍未关闭（且 §2.6 的二次组装与 §2.6b 的 F2 都是其前置） |
| **M-g3' / M-g2 / M-g3** | ⏳ 仍开（**15 / 46 / 21** 站点） |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；M22 整体仍 **NO-GO** |
| **oracle** | ✅ `crates/*` **零改动**；真值全部取自 oracle **生产输出** |
| **发射端的二次组装** | ❌ **未修**（§2.6），已登记为待办 |
| **发射端分配消除** | ⏳ **已回退**（§2.6b，实测否定结果）；它的测量结论与它暴露的缺口**保留** |
| **宿主调用字面量实参** | ❌ **前端不能降低**（F2，§2.6b）⇒ 既有发射端代码 `cgj_ident_text` **在前沿之外**；本片**没有**动前端 |
| **「返回 result 的内部函数」** | ❌ 仍被拒（STEP-0539 §2.6 登记的独立缺口），本片未动 |

## 4. 下一刀（按本片实测）

1. **发射词表 B 剩余 7 项 helper**：`sico.u64.to_text`（**144**）→ `sico.text.starts_with`（**136**）→ `sico.list.get[U64]`（**73**）→ `sico.text.split_lines`（**60**）→ `sico.list.empty[U64]`（**16**）→ `sico.list.append`（**11**）→ `sico.list.append[U64]`（**6**）⇒ 每项都要**新增一个 core 模块函数**并逐字节复现其**类型段 / 函数段 / 函数体**；**排序位已泛化 ⇒ 新增一项只需扩一次词表**。
2. **F2：宿主调用的字面量实参**（前端片）—— 让 guest 前端能降低 `sico.text.encode("<字面量>")` 这一形状 ⇒ **既有**发射端代码 `cgj_ident_text` 才能回到自举前沿之内 ⇒ 这是**整文件自举**的具名前置。
3. **发射端的线性组装**（§2.6）：这是**整文件自举**的前置，也是把组件轴逐站差分的耗时压回常量的前置。
4. **M-g3'（15）/ M-g2（46）/ M-g3（21）**：前端三门仍开。
5. **闭包规模（219）**、**语料覆盖（0 条）**、**门 3 / 6**。

## 5. 验证记录（2026-10-25）

**整轮版本化电池：`BATTERY_GREEN 15/15`**（单实例，本机时）。
日志：`target/tmp/m22-s5-battery/battery.log`（版本化脚本：`tools/validate-m22-s5.ps1`）。

| # | 步骤 | 耗时 | exit | 命令 |
|---|---|---|---|---|
| 1 | `fmt (workspace)` | 1s | **0** | `cargo fmt --all -- --check` |
| 2 | `fmt (runner)` | 2s | **0** | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| 3 | `clippy (workspace)` | 1s | **0** | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| 4 | `clippy (runner)` | 1s | **0** | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| 5 | `ci-one-key` | 1418s | **0** | `powershell -File tools/run-ci.ps1` |
| 6 | `validate-0245` | 1086s | **0** | `powershell -File tools/validate-step-0245.ps1` |
| 7 | `validate-0262` | 1166s | **0** | `powershell -File tools/validate-step-0262.ps1` |
| 8 | `probe-frontend-frontier` | 506s | **0** | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| 9 | `probe-ir-plan-frontier` | 26s | **0** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| 10 | `comment-discipline` | 0s | **0** | `python tools/validate-selfhost-comment-discipline.py` |
| 11 | `match-site-census` | 0s | **0** | `python tools/census-m22-match-sites.py --check` |
| 12 | `match-site-coverage` | 983s | **0** | `powershell -File tools/probe-m22-match-coverage.ps1` |
| 13 | `emitter-vocabulary-census` | 24s | **0** | `python tools/census-m22-emitter-vocabulary.py --check` |
| 14 | `firmware-selfhost` | 1063s | **0** | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| 15 | `git-diff-check` | 0s | **0** | `git diff --check` |

**判定**：本片的实现、改钉、账本刷新、普查工件刷新与文档同步在**同一份快照**上通过整轮电池 ⇒ **STEP-0540 达成**。
但**不推进任何出口门**：门 3 / 6 仍 **NO-GO**，M22 整体仍 **NO-GO**；`match` 族组件级全等仍是 **23 / 396**。

**独立 CI 裁决**：本片提交推送后触发 `codex/m22-w1-ci` 的 **run #51**；其裁决**由下一片回填**（不在本片预写结论）。

---
