# STEP-0541 · M22 W2 S5 · 发射词表 B-2 / B-3（`sico.u64.to_text` + `sico.text.starts_with`）

> **本片性质** = 计划 §3.3.5「每族四拍」的**组件 C 拍**，对象 = **发射词表 B** 的**第二刀与第三刀**（两个 core helper）。
> **不推进任何出口门**；**`crates/*` 零改动**；**`selfhost/parser.sico` 零改动**（只动发射端 ⇒ 前端前沿不动）；**M22 仍为 NO-GO**；**`match` 族仍未闭合** ⇒ **D10 不得降级**。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| STEP-0540 已提交并推送 | ✅ | 提交 `0d49ec7`，推送到 `github/codex/m22-w1-ci`（run #51 触发） |
| STEP-0540 整轮版本化电池绿 | ✅ | `BATTERY_GREEN 15/15`（日志 `target/tmp/m22-s5-battery/battery.log`） |
| oracle 生产代码零改动 | ✅ | `git diff --stat` 只含 `selfhost/`、`runner/`、`tools/`、`docs/` |
| 本片对象在**冻结**的缺口清单里 | ✅ | `selfhost/emitter-vocabulary-v1.json`：`sico.u64.to_text` **144** 站点 / `sico.text.starts_with` **136** 站点 |

## 1. 本片方法

1. **先读冻结 oracle 源码**（不复刻、不猜）：`crates/sico-codegen-wasm/src/stdlib.rs` 的 `emit_u64_to_text` / `emit_starts_with`、
   `crates/sico-codegen-wasm/src/lib.rs` 的 `helper_dependencies` / `INTRINSIC_HELPERS` / `emit_intrinsic`、
   `crates/sico-codegen-wasm/src/canonical.rs` 的 `flat_ir_types`（**`Text` = `(ptr, len)` 两槽**）。
2. **helper 体机械推导 + 自检**：用一份 Python 编码器把 oracle 的指令列表重新编码成 wasm 字节，
   先**重推两个已冻结的体**（`sico.bytes.equal` / `sico.text.concat`）与发射端里的冻结值**逐字节比对** ⇒ 编码器可信 ⇒ 再推新体。
   （脚本：`target/tmp/step0541/derive-helper-bodies.py`、`derive-starts-with.py`；**自检 = MATCH / MATCH**。）
3. **按既有形状落地**：helper 体函数 → `cgs_helper_index` 的**名字排序位** → **类型段 / 函数段 / code 段**三处接线 →
   独立的发射函数（`cgs_*_hex`）→ `cgs_intrinsic_hex` 的**一行分派**（与 STEP-0540 的 `cgs_bytes_equal_hex` 同形）。
4. **四道闸门全 fail-closed**：实参个数 / 实参类型 / 结果槽数 / helper 索引；任一不过 ⇒ `ERR:E-SH-CG-IR-EMIT`。
5. **钉**：两个形状**整组件逐字节等同** + 两条**拒绝对照**（两侧都拒）；**自举前沿钉**加 4 个函数名。

## 2. 实测结果

### 2.1 两个 helper 的合同（全部读自冻结源码）

| 项 | `sico.u64.to_text` | `sico.text.starts_with` |
|---|---|---|
| IR 签名 | `(U64) -> String`（`sico-ir/src/lib.rs:1481`） | `(Text, Text) -> Bool` |
| helper 签名 | `(i64) -> (i32, i32)` | `(i32,i32,i32,i32) -> (i32)` |
| helper 体 | `emit_u64_to_text(alloc)`：**110 B 冻结体 + 2 处 `call alloc`** ⇒ **必须参数化** | `emit_starts_with()`：**62 B，零 `call`** ⇒ **可逐字冻结** |
| helper 类型（类型段条目） | `60017e027f7f` | `60047f7f7f7f017f` |
| 名字排序位 | **最后**（`sico.text.*` < `sico.u64.*`） | 在 `sico.text.length` 之后、`sico.u64.to_text` 之前 |
| 调用点字节 | `local.get value` / `call helper` / `local.set len` / `local.set ptr` | `local.get a_ptr` / `local.get a_len` / `local.get b_ptr` / `local.get b_len` / `call helper` / `local.set result` |

**helper 索引链（`cgs_helper_index`，泛化后的排序位）**：
`sico.bytes.equal` < `sico.bytes.utf8_decode` < `sico.text.concat` < `sico.text.length` < **`sico.text.starts_with`** < **`sico.u64.to_text`**
⇒ 本片新增的两项排在**已有四项之后** ⇒ **已有四项的 core 索引一个字节都没动**（这是可验证的，不是推断）。

### 2.2 组件层：两个 helper 的整组件与 oracle 生产输出**逐字节等同**

| 钉 | 形状 | oracle 组件 | 结论 |
|---|---|---|---|
| `the_emitter_vocabulary_b2_covers_the_u64_to_text_helper_shape` | `sico.u64.to_text(n)` | **1154 B** | ✅ **逐字节等同**，两侧都过真实 `wasmparser::Validator` |
| 同上 | `sico.u64.to_text(n)` + `sico.text.concat`（**排序位对照**） | **1245 B** | ✅ **逐字节等同** |
| `the_emitter_vocabulary_b3_covers_the_text_starts_with_helper_shape` | `sico.text.starts_with(a, b)` | **1106 B** | ✅ **逐字节等同**，两侧都过真实 `wasmparser::Validator` |
| 同上 | `sico.text.starts_with(a, b)` + `sico.text.concat`（**排序位对照**） | **1252 B** | ✅ **逐字节等同** |

**排序位对照的作用**：两个 helper 的 core 索引必须**相邻且有序** ⇒ 排序位算错时组件**必然不等** ⇒ 这两个形状是**排序位**的钉，不是「长度相同」的钉。

### 2.3 闸门是活的（两条拒绝对照，两侧都拒）

| 对照 | Rust oracle | guest |
|---|---|---|
| B-2 **实参个数**：去掉 `sico.u64.to_text` 的实参 | verifier 拒 | `ERR:E-SH-CG-IR-EMIT` |
| B-2 **实参类型**：实参换成 **Text** 形参 | verifier 拒 | `ERR:E-SH-CG-IR-EMIT` |
| B-3 **实参个数**：去掉 `sico.text.starts_with` 的第二个实参 | verifier 拒 | `ERR:E-SH-CG-IR-EMIT` |
| B-3 **实参类型**：两个形参都换成 **Bytes** | verifier 拒 | `ERR:E-SH-CG-IR-EMIT` |

### 2.4 自举前沿：本片新增的发射代码**自己**能经 guest 前端逐字节降

**自举前沿钉的函数清单 20 → 24**（新增 `cg_starts_with_helper_body_hex` / `cg_u64_to_text_helper_body_hex` /
`cgs_starts_with_hex` / `cgs_u64_to_text_hex`）⇒ **绿**。

> 这条钉是 STEP-0540 教训的直接延续：**逐字节相等 ≠ 在自举前沿之内**。
> 本片新增的代码若只做到「组件逐字节相等」而没进前沿钉，仍可能落在前沿之外（见 §2.6 的第三个缺陷）。

### 2.5 账本与普查工件刷新

**账本**（`selfhost/match-component-coverage-v1.json`，396 站点；`census-m22-match-sites.py --check` ⇒ `LEDGER_CURRENT sites=396`）：

| verdict | 站点数 | 相对 STEP-0540 |
|---|---|---|
| `ir-equal+component-equal` | **30** | **+7**（23 → 30） |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 63 | **−47** |
| `ir-equal+component-other:FuelExhausted` | **84** | **+47** |
| `closure-too-large:*`（21 个不同规模桶） | 219 | 不变 |

**54 个站点的 verdict 发生移动**，分两类：

| 移动 | 站点数 | 含义 |
|---|---|---|
| `component-refused-guest` → **`component-equal`** | **7** | **真正闭合**：`MS-014`（`parser.sico:type_json`）、`MS-021` / `MS-022`（`fixed_literal_operand_json`）、`MS-081`..`MS-084`（`fixed_literal_const_segment`） |
| `component-refused-guest` → **`component-other:FuelExhausted`** | **47** | **不是闭合**：guest 现在**能发**这段源码，但**发射端的二次组装把燃料耗尽** ⇒ 前进到下一个瓶颈 |

> 探针 `tools/probe-m22-match-coverage.ps1` 因此以 `MATCH_COVERAGE_MOVED` **exit 1** —— 这是**预期的闭合信号**，不是失败。账本按纪律**只在实测之后**用 `census --update` 合并（`LEDGER_WRITTEN sites=396 pending=0`），再以 `--check` 复验（`LEDGER_CURRENT sites=396`）。

**词表工件**（`selfhost/emitter-vocabulary-v1.json`；`census-m22-emitter-vocabulary.py --check` ⇒ `EMITTER_VOCABULARY_CURRENT sites=147`）：

| 项 | 实测 |
|---|---|
| 站点数 | **147**（= 「IR 已全等但组件未全等」的**全部**站点，含耗尽） |
| guest 已有 helper | **6**：`sico.bytes.equal` / `sico.bytes.utf8_decode` / `sico.text.concat` / `sico.text.length` / **`sico.text.starts_with`** / **`sico.u64.to_text`** |
| `missing_op_frequency` | **{}**（空 ⇒ **没有内联操作码缺口**） |
| `missing_helper_frequency` | **5 项**：`sico.list.get[U64]` **73** / `sico.text.split_lines` **60** / `sico.list.empty[U64]` **16** / `sico.list.append` **11** / `sico.list.append[U64]` **6** |
| 只缺 **1** 项 helper 的站点 | `sico.list.get[U64]` **21** + `sico.text.split_lines` **3** |
| 词表缺口数分布（缺 0 / 1 / 2 / 3 / 5 项 helper） | **66** / 24 / 41 / 10 / 6（工件 `additions_needed_histogram` 的键是 `additions_needed` = **0 / 2 / 4 / 6 / 10**，即**每缺 1 项 helper 计 2 项新增**：缺 1 个内建名 + 1 个 helper 体 ⇒ 与缺口数一一对应） |

⇒ **66 / 147** 个站点**已无词表缺口**（只剩燃料/闭包）；**发射词表 B 还剩 5 项 helper**。
⇒ 在 147 站点口径下，本片两个 helper 的**需求量**分别是 `sico.u64.to_text` **138**、`sico.text.starts_with` **135**（工件 `required_helper_frequency`）。

### 2.6 本片**捕获并修掉**的四个自造缺陷（诚实登记）

| # | 缺陷 | 怎么发现的 | 修法 |
|---|---|---|---|
| 1 | `sico.text.starts_with` 的闸门按 **4 个实参**写（把 `(ptr,len)` 拆成了两个槽） | 形状**被 guest 拒**（`ERR:E-SH-CG-IR-EMIT`）⇒ 实参是**源操作数**（两个 Text），`(ptr,len)` 的拆分发生在后端的**槽布局**里 | 闸门改成「实参 0 存在 / 实参 1 存在 / 实参 2 **不得**存在」——与 `sico.bytes.equal` 分支同形 |
| 2 | `starts_with` 冻结体的**第一个比较**写成 `I32GeU`（0x4F），oracle 是 `I32GtU`（0x4B） | **逐字节钉**在第 **962** 字节报不等（长度相同、字节不同）⇒ 正是 helper 体的位置 | 冻结体改成 `…20032001**4b**044041000f0b…`（并复核 `wasm-encoder` 的 opcode 表） |
| 3 | 落地的两个新函数**整体少了一级缩进**（发射端的约定是每级 **2 空格**） | **自举前沿钉**（它经 guest 前端真实降低发射端源码）⇒ 缩进错了前端就降不下 | 统一重排缩进；前沿钉转绿 |
| 4 | 两个新函数的 `end function` 写成**缩进版**（`  end function`），且 `cgs_bytes_equal_hex` 的**末两行被多加了 2 空格**（改到了既有代码） | **注释纪律校验器**（`tools/validate-selfhost-comment-discipline.py`）红灯：它按「**精确等于** `end function`」切函数体 ⇒ 缩进的 `end function` **没被识别** ⇒ 三个函数被并成一段 ⇒ 本在函数**之间**的注释被误判为「函数体内注释」 | 两行 `end function` 去缩进；`cgs_bytes_equal_hex` 末两行恢复 2 空格 ⇒ 校验器 **GREEN**；并用 `git diff -w` 复核：**+243 / −0** ⇒ 本片对既有发射端代码**零逻辑改动**（纯追加） |

> 第 4 条是本片最值得记的一条：**自举前沿钉与逐字节组件钉都抓不到它**——
> 因为缩进不影响 guest 的**语义降低**；抓它的是**仓库自己的注释纪律校验器**。
> 也就是说：**「钉绿」不等于「仓库规矩绿」**，两套检查必须都跑。
>
> 第 2 条同样值得记：**组件长度相同**不代表字节相同；**逐字节钉**是唯一能抓住「opcode 选错」的手段。

### 2.7 本片的**电池整改记录**（两轮红灯，均如实登记）

| 轮次 | 结果 | 处置 |
|---|---|---|
| 第 1 轮 | **RED**（第 2 步 `fmt (runner)` exit 1） | 本片新增的测试代码**未过 rustfmt** ⇒ `cargo fmt` 修好 ⇒ 改动仍是**纯追加**（`+269 / −0`），未动任何既有测试行 ⇒ 重跑 |
| 第 2 轮 | **中止**（第 5 步 `ci-one-key` 进行中） | 手工跑**注释纪律校验器**发现**第 4 个缺陷**（见 §2.6）⇒ 该轮必然在第 10 步红灯 ⇒ 立即停机（不浪费后续 ~1.5 h）⇒ 修缺陷 ⇒ 重跑 |
| 第 3 轮 | 见 §5.1 | — |

> 这两轮都是**本片自造**的问题，不是回归；两处都**先修再报**，不包装。


## 3. 本片**没有**做的事（诚实边界）

| 项 | 状态 |
|---|---|
| **`crates/*` 改动** | ❌ **零改动**（oracle、verifier、后端一字未动） |
| **`selfhost/parser.sico` 改动** | ❌ **零改动** ⇒ 前端前沿不动 |
| **F2（宿主调用的字面量实参）** | ❌ **仍未修** ⇒ 前端仍降不下 `sico.text.encode("<字面量>")` ⇒ 既有 `cgj_ident_text` **仍在前沿之外** ⇒ **整文件自举**的前置未解除 |
| **发射端的二次组装** | ❌ **仍未修** ⇒ **84** 个站点的 `component-other:FuelExhausted` 依旧（其中 **47** 个是本片从「拒」推进到「耗尽」的）（**不上调限额、不加尺寸门**） |
| **发射词表 B 的其余 5 项 helper** | ❌ 未做（`sico.list.get[U64]` / `sico.text.split_lines` / `sico.list.empty[U64]` / `sico.list.append` / `sico.list.append[U64]`）**＋ 第 6 项 `sico.text.join`**（线性组装刀的前置，见 §4） |
| **M-g3'（15）/ M-g2（46）/ M-g3（21）** | ❌ 仍开 |
| **闭包规模（219 站点）组件轴** | ❌ **未测**（不是「通过」） |
| **语料覆盖** | ❌ 仍 **0 条** |
| **门 3 / 门 6** | ❌ 仍 **NO-GO** |

## 4. 下一刀（按本片实测）

1. **发射词表 B 的其余 5 项 helper**（`sico.list.get[U64]` **73** / `sico.text.split_lines` **60** / `sico.list.empty[U64]` **16** / `sico.list.append` **11** / `sico.list.append[U64]` **6**）⇒ 前两项是**集合类** helper（名字带 `[U64]` 的**单态化**泄漏名），后三项体小。
   **＋ 第 6 项 `sico.text.join`**（本片读 oracle 源码核实，因果链已闭合）：线性组装要让**发射端源码**调用 `sico.text.join`，
   而**冻结的 `codegen.sico` 就是这份发射端源码** ⇒ guest 编译它时必须能**逐字节发出**该 helper
   ⇒ **词表缺口由 5 项变 6 项**，且**线性组装刀依赖它**（不是它依赖线性组装）⇒ helper 刀必须先行。
   （oracle 签名 4×i32→2×i32；冻结体 `emit_join` **178 B + 2 处 `call alloc`**；类型段与 `sico.list.append` **相同且不去重**。）
2. **F2：宿主调用的字面量实参**（前端片）—— 让 guest 前端能降低 `sico.text.encode("<字面量>")` ⇒ **既有**发射端代码 `cgj_ident_text` 才能回到前沿之内 ⇒ **整文件自举**的具名前置。
3. **发射端的 fuel 归因测量片（T 拍）**：计划原文把 fuel 墙归因为「hex 逐段拼接」，但本片读源码发现**两处不符** ——
   ① 第二个二次候选：`cgj_array_item`（`selfhost/codegen.sico:754-780`）**从头扫描到第 `index` 个**（O(index)），而 **56 处**调用点的下标是**递增遍历计数器** ⇒ 每次数组遍历 **O(n²)**；`cgs_value_slot` 自身也递增遍历 ⇒ 仅槽位查找即 **O(n³)**；
   ② **计量口径**：`FuelExhausted` 读的是**主 fuel**（`store.get_fuel()==0`，`runner/sico-runner/src/lib.rs:3552`），主 fuel 只按 **guest 执行的 Wasm 指令**计费；
   而 hex 拼接是**组件内 core module 的 Wasm 函数**（`emit_concat` = `alloc` + **2× `memory.copy`**，`crates/sico-codegen-wasm/src/stdlib.rs:147-160`）⇒ guest 对它是**组件内调用**（`call <core 索引>`）⇒ 记**主 fuel**；但**每次拼接只有 ~13 条指令**（`memory.copy` 是单条指令）⇒ A 路对主 fuel 的贡献是 **O(调用次数)**，随字节数增长的是**拷贝字节量**（累计 **O(L²)**）⇒ **A 路不可能是 fuel 墙**，它解的是**时间墙**与**整文件自举**；`hostcall_fuel` 只是 stdin/stdout 的 **Canonical-ABI 传输预算**（`:2128-2134`），组件内核心调用**不经过**它；B 路的 `cgj_value_end`（`:573-598`）是**纯 guest 逐字节扫描** ⇒ 代价 O(字节数) × **guest 指令**。
   ⇒ **推演指向 B 路**（仍须实测确认：词表工件的 `ops` 是「算子种类集合」而非指令数，**无判别力**）⇒ **先测量再分片**（**不上调 fuel 限额、不加尺寸门**）。
   ⇒ **规模侧佐证（账本实测）**：`closure-too-large` 的备注给出闭包规模 **64–65 个函数 / 193,141–282,294 B** ⇒ IR 的 n 达**数千** ⇒ `cgs_value_slot` 的 **O(n²)** 被调 **O(n)** 次 ⇒ **O(n³)** ⇒ 这是能吃满 **8e11** 主 fuel 的量级（**量级推演，仍须归因片实测确认**）。
4. **B 路修复（条件：归因实测确认主 fuel 由 B 路耗尽）**：把 guest 的 **IR 读取**从「按序号重扫」改成「沿游标推进」——
   **56 处**遍历计数器的调用形态改写（`cgj_array_item(bytes, arr, i)` → 游标式推进）+ `cgs_value_slot` 改查**前缀和表**
   （canonical IR 的值 id 是 `ValueId(pub u32)`，`crates/sico-ir/src/lib.rs:36` ⇒ JSON 里是**顺序数字** ⇒ 可按值号 O(1) 查槽；
   表可用字节串 + 已支持的 `sico.bytes.at` 承载 ⇒ **不必新增 helper**）⇒ **84** 个耗尽站点的解锁手段（**机械刀**，与 A 路的 463 处架构刀不同量级）。
5. **A 路：发射端的线性组装**（解**时间墙**与**整文件自举**，**不是**那 84 个站点）：把「顺序拼接累加器」换成「`List[Text]` 累加 + 一次 `sico.text.join`」
   （`sico.list.append` 是**摊还 O(1)**，`sico.text.join` 是**两遍线性**）⇒ 逐站差分耗时（983 s → 1492 s）与**整文件自举**（~1 MiB 组件）。
   **该刀不依赖 F2**：这两个原语走的是**双参**路由（`gw_text_list_binary_packed`），字面量实参走通用 RHS 降低 ⇒ 前端能降；
   但**依赖第 1 项的 `sico.text.join`**（见上）。**A 与 B 不可互相替代**（见上文第 3 项的计量口径）。
6. **F2：宿主调用的字面量实参**（前端片）—— 让 guest 前端能降低 `sico.text.encode("<字面量>")` ⇒ **既有**发射端代码 `cgj_ident_text` 才能回到前沿之内 ⇒ **整文件自举**的具名前置。
7. **M-g3'（15）/ M-g2（46）/ M-g3（21）**、**闭包规模（219）**、**语料覆盖**、**门 3 / 6**。

## 5. 验证记录（2026-10-25）

**整轮版本化电池：`BATTERY_GREEN 15/15`**（单实例，本机执行）。
日志：`target/tmp/m22-s5-battery/battery.log`（版本化脚本：`tools/validate-m22-s5.ps1`）。

### 5.1 电池表

| # | 步骤 | 耗时 | exit | 命令 |
|---|---|---|---|---|
| 1 | `fmt (workspace)` | 1s | **0** | `cargo fmt --all -- --check` |
| 2 | `fmt (runner)` | 2s | **0** | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| 3 | `clippy (workspace)` | 1s | **0** | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| 4 | `clippy (runner)` | 2s | **0** | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| 5 | `ci-one-key` | 1945s | **0** | `powershell -File tools/run-ci.ps1` |
| 6 | `validate-0245` | 1591s | **0** | `powershell -File tools/validate-step-0245.ps1` |
| 7 | `validate-0262` | 1605s | **0** | `powershell -File tools/validate-step-0262.ps1` |
| 8 | `probe-frontend-frontier` | 503s | **0** | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| 9 | `probe-ir-plan-frontier` | 28s | **0** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| 10 | `comment-discipline` | 0s | **0** | `python tools/validate-selfhost-comment-discipline.py` |
| 11 | `match-site-census` | 0s | **0** | `python tools/census-m22-match-sites.py --check` |
| 12 | `match-site-coverage` | 1441s | **0** | `powershell -File tools/probe-m22-match-coverage.ps1` |
| 13 | `emitter-vocabulary-census` | 24s | **0** | `python tools/census-m22-emitter-vocabulary.py --check` |
| 14 | `firmware-selfhost` | 1552s | **0** | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| 15 | `git-diff-check` | 0s | **0** | `git diff --check` |

**判定**：本片的实现、改钉、账本刷新、普查工件刷新与文档同步在**同一份快照**上通过整轮电池 ⇒ **STEP-0541 达成**。
但**不推进任何出口门**：门 3 / 6 仍 **NO-GO**，M22 整体仍 **NO-GO**；`match` 族组件级全等仍是 **30 / 396**。

**固件**：`selfhost_compiler` = **181 passed / 0 failed**（STEP-0540 为 179；本片新增 2 条钉）。

**独立 CI 裁决**：本片提交推送后触发 `codex/m22-w1-ci` 的 **run #52**；其裁决**由下一片回填**（不在本片预写结论）。
