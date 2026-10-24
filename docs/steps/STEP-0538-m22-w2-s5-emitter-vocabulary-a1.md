# STEP-0538 m22-w2 S5 **发射词表 A-1 实现片（C 拍）**：7 项内联词表落地，并补上 `sico.bytes.utf8_decode` 的 core helper

> - 性质：**实现片**（组件 C 拍）。**`crates/*` 零改动**；**`selfhost/parser.sico` 零改动**（本片只动发射端 ⇒ 前端前沿不动）；**不推进任何出口门**；**M22 仍为 NO-GO**（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO）；**`match` 族仍未闭合**（组件级全等 **13 / 396**）。
> - 归属：[M22 计划 §3.3.5](./../plans/M22-compiler-self-host.md) 的「每族四拍（真值 T → 前端 F → 组件 C → 收口 X）」的对象 = **发射词表 A**。承 [STEP-0537](./STEP-0537-m22-w2-s5-emitter-vocabulary-census.md)（组件层 T 拍：168 个站点的具名缺口清单）。
> - 产物：`selfhost/codegen.sico`（**+481 / −181**，5001 → **5298** 行）+ 固件改钉 + 站点账本 v1 刷新（4 个站点 verdict 前移）+ 普查工具加 **helper 轴**（工件升 `selfhost/emitter-vocabulary-v1.json`，schema `sico.m22.emitter-vocabulary.v1`）+ 电池 15 步不变。

## 0. 入口门槛

- **缺口是真的**：STEP-0537 的普查在 **168** 个组件被拒站点上逐站点命名了缺口 ⇒ 实施顺序不再是猜测。
- **本片只做 A-1**：词表 A 共 **9 项内联**（3 算子 + 6 内联内建）；本片做其中 **7 项**（3 算子 + 4 内联内建：`sico.text.encode` / `sico.bytes.utf8_decode` / `sico.list.length` / `sico.bytes.at`），留下 `sico.list.get` / `sico.bytes.slice` 归 A-2。
- **不能按预测改钉**：STEP-0537 §2.4 给的是**词表推算的预期**，本片必须**实测**后才动账本。

## 1. 本片方法

1. **先写会失败的差分测试**：把 5 个此前「组件层 typed 拒绝」的形状改成「与 oracle 生产输出**逐字节等同**」的钉（`assert_bytes_equal` + 双方都过真实 `wasmparser::Validator`）。
2. **发射端落点**（`selfhost/codegen.sico`）：
   - **3 个算子**：`less_fixed`（`i64.lt_s` = `0x53` / `i64.lt_u` = `0x54`，按左操作数类型分流）、`const_bool`（`i32.const` + `local.set`）、`checked_mul`（`i64.mul` + 溢出条件 + `if`）。
   - **4 个内建**：`sico.text.encode` / `sico.bytes.utf8_decode`（两者都是 **(pointer, length) 对**的**透传**）、`sico.list.length`（取 count + `i64.extend_i32_u`）、`sico.bytes.at`（内联 `if/else` + `i32.load8_u`）。
   - **4 道闸门全 fail-closed**：名字 / 实参个数 / 实参类型 / 结果槽数，任一不过即 `ERR:E-SH-CG-IR-EMIT`，**不静默取近似**（ADR-0016 决策 6）。
3. **结构化扫描泛化**（本片附带）：原先 **6 个**按名字写死的扫描函数（concat 家族 3 个 + length 家族 3 个）合并为 **4 个**通用扫描（`cgs_instruction_is_intrinsic` / `cgs_block_has_intrinsic` / `cgs_function_has_intrinsic` / `cgs_module_has_intrinsic`，**名字作为参数**）⇒ 新内建不再需要复制一份扫描。

## 2. 实测结果

### 2.1 关键发现：`sico.bytes.utf8_decode` **不是纯内联**

第一轮实现只做了**调用点上的 (ptr, len) 透传**，三个 utf8_decode 形状的组件仍然不等：

| | 类型段 | 函数段 | code 段 | 组件总长 |
|---|---|---|---|---|
| **oracle** | 44 B / **7** 条 | 8 B / **7** 条 | **971 B** | **1570 B** |
| **guest（修前）** | 38 B / **6** 条 | 7 B / **6** 条 | **511 B** | **1103 B** |

⇒ guest **少发一个 core 模块函数**。根因读自**冻结的 oracle 源码**：`sico.bytes.utf8_decode` 在 `INTRINSIC_HELPERS`（`crates/sico-codegen-wasm/src/lib.rs:565`）里 ⇒ 后端**除了调用点的内联透传**（`:4487`），还要**额外发一个 core helper 函数**（UTF-8 校验体，`emit_utf8_validate`，`crates/sico-codegen-wasm/src/stdlib.rs:249`）。

- **helper 体 = 458 B**（locals 声明 `01 05 7f` = 1 组 5 个 i32 + **455 B** 代码），**无 `call` 指令** ⇒ 不含函数索引 ⇒ 可**逐字冻结**。
- **helper 类型 = `(i32,i32)->i32`** = `60027f7f017f`（与 `sico.text.length` helper **同型**）。
- **helper 索引规则 = `模块函数数 + 3 + 名字排序位`**（后端 `used_helpers` 是**按名字排序**的集合 ⇒ `sico.bytes.utf8_decode` < `sico.text.concat` < `sico.text.length`）⇒ `sico.text.length` 的调用索引由 `函数数+3` 变成 `函数数+4`。
- **修法**：新增冻结体 `cg_utf8_validate_helper_body_hex`（**458 B**），并在**类型段 / 函数段 / code 段**三处按排序位插入 + 两个调用点的 helper 索引**各偏移一位** ⇒ 三段与 oracle 逐字节等同。

> **这条发现的方法学含义**：**「内建 = 调用点内联」是错的**——某些内建**同时**需要一个 core helper。STEP-0537 的普查只测「调用点词表」，会把这类缺口**漏报**；本片因此给普查加了 **helper 轴**（§2.3）。

### 2.2 7 项词表：逐字节等同的形状

| 组 | 形状 | oracle 组件 | 结论 |
|---|---|---|---|
| 算子 `less_fixed` | `if` 条件里 `U64.less_than(0, sico.text.length(x))` | **1211 B** | ✅ 逐字节等同 |
| 算子 `less_fixed` | 对照：`U64.less_than(0, U64.literal(5))`（无 length 调用） | **1132 B** | ✅ 逐字节等同 |
| 算子 `less_fixed` | 三个 `while` 循环形状（无 length / 带 length / 条件带 length） | **1142 / 1209 / 1187 B** | ✅ 逐字节等同（**钉从「必须拒」升级为「必须逐字节等同」**） |
| 内建 `sico.bytes.utf8_decode` | `return` 位嵌套调用 | **1570 B** | ✅ 逐字节等同 |
| 内建 `sico.bytes.utf8_decode` | `let` 绑定位（两处） | **1570 B** | ✅ 逐字节等同 |

> **改钉口径**：STEP-0530 的钉 `sico_codegen_guest_refuses_the_loop_shape_outside_the_text_length_subset` **改名**为 `sico_codegen_guest_emits_the_loop_shapes_with_a_less_than_condition_byte_exactly`（**更强的断言**：从「必须拒」变成「必须逐字节等同」）；STEP-0530 **保留原文**并加 addendum。同批改钉的另两条钉**保留原名**（`the_t2_cell_matrix_component_layer_covers_what_the_emitter_knows` / `the_t2_g2_component_layer_covers_the_nested_shapes_the_emitter_knows`），因为「covers what the emitter knows」仍然贴切。

### 2.3 普查加 **helper 轴**（工件升 v1）

普查工具现在除「调用点词表」外，还报告**每个站点需要的 core helper**：

- **helper 规则不重写**：直接**读冻结 oracle 的源码**（`helper_dependencies` 的显式分支 + `INTRINSIC_HELPERS`），并把该源码的 **sha256 冻进工件** ⇒ 后端一改，工件即失效（普查红灯）。
- **guest 的 helper 覆盖**由普查工具核对：guest 源码里出现任何**未登记**的 `*_helper_body_hex` 函数 ⇒ 普查**红灯**，不会静默漂移。
- **工件改名**：`selfhost/emitter-vocabulary-v0.json` → **`selfhost/emitter-vocabulary-v1.json`**（schema `sico.m22.emitter-vocabulary.v1`），旧工件删除（与账本 v0 → v1 同一口径）。

**实测：发射词表 B = 8 项 helper**（当前 **164** 个组件被拒站点里仍缺的）：

| helper | 站点数 | 形态 |
|---|---|---|
| `sico.bytes.equal` | **156** | core helper（`lib.rs:4831`） |
| `sico.u64.to_text` | **144** | core helper（`lib.rs:4939`） |
| `sico.text.starts_with` | **136** | core helper（`lib.rs:4590`） |
| `sico.list.get[U64]` | **73** | 集合单态化 helper |
| `sico.text.split_lines` | **60** | core helper（`lib.rs:4568`） |
| `sico.list.empty[U64]` | **16** | 集合单态化 helper |
| `sico.list.append` | **11** | core helper（`lib.rs:4786`） |
| `sico.list.append[U64]` | **6** | 集合单态化 helper |

⇒ **与 STEP-0537 §2.4 预测的 8 项 helper 完全一致**（逐项、逐站点数），**这是第一次由工具交叉验证出来的**。

**新的「还差几项」直方图**（`additions_needed` 现在把 helper 也计入）：

| 还差 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 14 | 18 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 站点数 | 1 | 1 | 9 | 1 | 15 | 6 | 32 | 24 | 19 | 4 | 2 | 34 | 10 | 6 |

⇒ 中位仍在 **9–12** 一档，最大 **18**（一个站点**全缺**）⇒ **「全有或全无」的形状不变**，只是上界从 17 变成 18（helper 轴新增的一项）。

**最便宜的三个真实站点**（`additions_needed ≤ 3`）：

| 站点 | 函数 | 还差什么 |
|---|---|---|
| **MS-017** | `parser.sico:word_span_offset` | 只要 **`sico.list.get`**（**无 helper** —— 不带方括号的 `sico.list.get` 是**内联**发射）⇒ **A-2 的这一项单独就能闭合它** |
| MS-014 | `parser.sico:type_json` | 只要 **`sico.text.starts_with`**（**需要 core helper**）⇒ 属发射词表 B |
| MS-011 / MS-012 / MS-018 / MS-023 / MS-038 / MS-063 / MS-074 / MS-219 / MS-363 | `parser.sico` 九个函数 | `sico.bytes.equal` + `sico.list.get`（**前者需要 helper**）⇒ **A-2 + B 各出一项**才能闭合 |

⇒ **A-2 的 `sico.list.get` 是当前最划算的一刀**：它**内联**（不拖 helper 尾巴），且**单独**就能闭合 MS-017、再为另外 **9** 个站点各减一项。

### 2.4 账本：组件轴 **9 → 13 / 396**

| 站点 | 文件 | 函数 | verdict 变化 |
|---|---|---|---|
| MS-013 | `parser.sico` | `is_ident_byte` | `component-refused` → **`component-equal`** |
| MS-016 | `parser.sico` | `find_text_from` | `component-refused` → **`component-equal`** |
| MS-020 | `parser.sico` | `gw_instr_count` | `component-refused` → **`component-equal`** |
| MS-158 | `parser.sico` | `return_expression_start` | `component-refused` → **`component-equal`** |

- **实测 4 个站点**，与 STEP-0537 的**词表推算一致**：STEP-0537 §2.4 的表里，`MS-017`（`word_span_offset`）额外缺 `sico.list.get` ⇒ **不在 A-1 的 7 项内** ⇒ 留给 A-2。
- **组件被拒站点 168 → 164**；**组件级全等 9 → 13 / 396**。
- **账本工具的键改成与行号无关的稳定身份**（`路径 + 函数 + 序号`）：`selfhost/codegen.sico` 本片增删函数 ⇒ 站点行号位移 ⇒ 旧键（`路径:行号:函数`）会把 verdict 全部记成 `pending`。改键后位移不再丢 verdict。

## 3. 本片**没有**做的事（诚实边界）

| 项 | 状态 |
|---|---|
| **`match` 族闭合** | ❌ **没有**：组件级全等 **13 / 396**；A-2（`sico.list.get` / `sico.bytes.slice`）与 **发射词表 B（8 项 helper）** 未做 |
| **闭包规模** | └ 仍 **219** 站点组件轴**未测**（闭包超限）⇒ 「未测」不是「通过」 |
| **语料覆盖** | └ 仍 **0 条** |
| **整文件自举** | └ 仍未关闭 |
| **M-g3' / M-g2 / M-g3** | ⏳ 仍开（**15 / 46 / 21** 站点） |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；M22 整体仍 **NO-GO** |
| **oracle** | ✅ `crates/*` **零改动**；真值全部取自 oracle **生产输出** |

## 4. 下一刀（按本片实测）

1. **发射词表 A-2**（2 项内联内建）：**先做 `sico.list.get`** —— 它是**内联**发射（**不带方括号的形式不需要 helper**），且**单独**就能闭合 **MS-017** `word_span_offset`（当前只差这一项）、再为另外 **9** 个站点各减一项；再 `sico.bytes.slice`（**89** 站点）。⇒ **预期**闭合 **1** 个站点，**待实测**。
2. **发射词表 B**（8 项 helper）：先 `sico.bytes.equal`（**156** 站点）、再 `sico.u64.to_text`（**144**）、`sico.text.starts_with`（**136**）… ⇒ 每项都要**新增 core 模块函数**并逐字节复现其**类型段 / 函数段 / 函数体**；**排序位会同时影响调用点的 helper 索引**（A-1 的 `sico.bytes.utf8_decode` 已经演示过一次）。
3. **M-g3'（15）/ M-g2（46）/ M-g3（21）**：前端三门仍开。
4. **闭包规模（219）**、**语料覆盖（0 条）**、**门 3 / 6**。

## 5. 验证记录（2026-10-24）

**整轮版本化电池：`BATTERY_GREEN 15/15`**（单实例，2026-10-24 21:18 → 21:57:26 本机时，累计 **2323 s**）。
日志：`target/tmp/m22-s5-battery/battery.log`（版本化脚本：`tools/validate-m22-s5.ps1`）。

| # | 步骤 | 耗时 | exit | 命令 |
|---|---|---|---|---|
| 1 | `fmt (workspace)` | 4 s | **0** | `cargo fmt --all -- --check` |
| 2 | `fmt (runner)` | 2 s | **0** | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| 3 | `clippy (workspace)` | 2 s | **0** | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| 4 | `clippy (runner)` | 1 s | **0** | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| 5 | `ci-one-key` | 742 s | **0** | `powershell -File tools/run-ci.ps1` |
| 6 | `validate-0245` | 289 s | **0** | `powershell -File tools/validate-step-0245.ps1` |
| 7 | `validate-0262` | 312 s | **0** | `powershell -File tools/validate-step-0262.ps1` |
| 8 | `probe-frontend-frontier` | 498 s | **0** | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| 9 | `probe-ir-plan-frontier` | 27 s | **0** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| 10 | `comment-discipline` | 0 s | **0** | `python tools/validate-selfhost-comment-discipline.py`（GREEN files=2） |
| 11 | `match-site-census` | 0 s | **0** | `python tools/census-m22-match-sites.py --check`（LEDGER_CURRENT sites=396） |
| 12 | `match-site-coverage` | 157 s | **0** | `powershell -File tools/probe-m22-match-coverage.ps1` |
| 13 | `emitter-vocabulary-census` | 25 s | **0** | `python tools/census-m22-emitter-vocabulary.py --check`（EMITTER_VOCABULARY_CURRENT sites=164） |
| 14 | `firmware-selfhost` | 264 s | **0** | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` ⇒ **177 passed / 0 failed** |
| 15 | `git-diff-check` | 0 s | **0** | `git diff --check` |

**补充（同一快照，另行复跑）**：`validate-step-0124.ps1` / `validate-step-0131.ps1` / `validate-step-0156.ps1` 全部 exit 0。

**判定**：本片的实现、账本刷新、普查工件升版与文档同步在**同一份快照**上通过整轮电池 ⇒ **STEP-0538 达成**。
但**不推进任何出口门**：门 3 / 6 仍 **NO-GO**，M22 整体仍 **NO-GO**；`match` 族组件级全等仍是 **13 / 396**。

**独立 CI 裁决**：本片提交推送后触发 `codex/m22-w1-ci` 的 **run #49**；其裁决**由下一片回填**（不在本片预写结论）。

## 6.1 addendum（2026-10-25，由 STEP-0539 回填）

> - **独立 CI 裁决已出**：本片提交 `5c61771` ⇒ run **#49** = `completed` / **`success`**（id `40100457405`，`windows-gnu` job，13:58:54Z → 14:33:39Z = **34m45s**）⇒ **STEP-0538 的独立 CI 裁决为 GO**。
> - **§2.3 / §2.4 的预测被 STEP-0539 实测确认**：本片 §2.3 记的「发射词表 B = 8 项 helper」清单，在 STEP-0539 刷新普查后**逐项、逐站点数**仍对上（`sico.bytes.equal` 156 / `sico.u64.to_text` 144 / `sico.text.starts_with` 136 / `sico.list.get[U64]` 73 / `sico.text.split_lines` 60 / `sico.list.empty[U64]` 16 / `sico.list.append` 11 / `sico.list.append[U64]` 6）。
> - **§2.4 的 A-2 预测已兑现**：本片 §4 写的「A-2 只做 `sico.list.get` ⇒ **单独**就能闭合 **MS-017**」被 STEP-0539 实测确认（MS-017 `parser.sico:word_span_offset` 由 `component-refused` 变为 `component-equal`；组件级全等 **13 → 14 / 396**）。
> - **本片的「组件级全等 13 / 396」「组件被拒 164」是本片快照的实测值**；当前口径以 [`docs/STATUS.md`](../STATUS.md) 与 [STEP-0539](./STEP-0539-m22-w2-s5-emitter-vocabulary-a2.md) 为准（**14 / 396**、组件被拒 **163**）。

---
