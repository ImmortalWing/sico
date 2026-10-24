# STEP-0537 m22-w2 S5 **组件层发射词表真值测量片（T 拍）**：把「发射端拒」变成 168 个站点的具名缺口清单

> - 性质：**测量片**（T 拍）。**`crates/*` / `runner/*` / `selfhost/*.sico` 零改动**；**不推进任何出口门**；**M22 仍为 NO-GO**（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO）；**`match` 族仍未闭合**（组件级全等 9 / 396）。
> - 归属：[M22 计划 §3.3.5](./../plans/M22-compiler-self-host.md) 的「每族四拍（真值 T → 前端 F → 组件 C → 收口 X）」，以及 owner 审计 `AGENTS.md` §10.5 **R1.5**（共享算子）与 §10.6（登记表要写「固定名字、元数、实参类型、结果槽数、真实调用点、冻结语料覆盖和未覆盖形状」）。承 [STEP-0534](./STEP-0534-m22-w2-s5-w2-match-truth.md)（`match` 族的 T 拍）、[STEP-0535](./STEP-0535-m22-w2-s5-w2-match-emitter.md)（M-g4a + R0 证据整改）、[STEP-0536](./STEP-0536-m22-w2-s5-w2-match-arity-gate.md)（M-g1 形参门 + 账本 v1）。
> - 产物：**逐站点发射词表普查** `tools/census-m22-emitter-vocabulary.py`（版本化，`--update` / `--check` / `--json`）+ **冻结工件** `selfhost/emitter-vocabulary-v0.json`（schema `sico.m22.emitter-vocabulary.v0`，168 站点）+ 电池新步 `emitter-vocabulary-census` + [支持面登记表](../reports/m22-support-surface-match.md) 订正。

## 0. 入口门槛

- **缺口是真的**：STEP-0536 的账本 v1 里，**168** 个站点的 canonical IR 已与 oracle **逐字节等同**，但**组件轴**仍被 guest 发射端拒。
- **但账本答不了「缺什么」**：这 168 条的 verdict 全是同一句 `ir-equal+component-refused-guest:unsupported codegen source shape` ⇒ **无法据此排实施顺序**，也无法据此判断「补一项能换来几个站点」。
- **先前的措辞是未验证的推断**：STEP-0535 / STEP-0536 / 计划 / STATUS / ROADMAP / handoff / 支持面报告都写着「组件层缺的是**分支 / 循环 / 多语句函数体**的发射能力」⇒ 那是把「发射端拒」直接当成「因为函数体形状」的**猜测**，本片**实测推翻**（§2.3）。

## 1. 本片方法

`tools/census-m22-emitter-vocabulary.py`（版本化、只读、确定性）：

1. **重放冻结的闭包规则**：与固件 `match_sico_functions` / `match_sico_callees` / `match_sico_closure`（`runner/sico-runner/tests/selfhost_compiler.rs:9222-9322`）**同规则**重算每个站点的**同文件依赖闭包**（上限 64 函数 / 262 144 B）。
   - **自检**：该规则必须**逐条复现**账本里 **219** 条 `closure-too-large` 的 verdict（**连 reason 里的函数数与字节数一起复现**）⇒ 实测 **OK**（`closure rule self-check: OK`）。规则若漂移，普查直接 exit 1。
2. **问 oracle 要 IR**：用**已提交的**诊断 `crates/sico-ir/tests/dump_oracle_ir.rs`（`ORACLE_SOURCE` / `ORACLE_OUT` / `ORACLE_FN`）拿 **Rust 前端生产输出**的 canonical IR，逐站点统计**算子名 / 内建名 / 终结符名**。
3. **读 guest 发射端的词表**：`selfhost/codegen.sico` 里 `cg_is_word(coph_name, "…")` 与 `cg_is_word(cih_name, "…")` 的**全部**命中 ⇒ 词表**从代码本身读出**，不可能与实现漂移。
4. **差集 = 具名缺口**：逐站点给出 `missing_ops` / `missing_intrinsics`，并写入冻结工件 `selfhost/emitter-vocabulary-v0.json`；`--check` 在工件与实测不一致时 **exit 1**。

> **不新增产品 CLI、不改 oracle**：IR 来自已提交的 Rust 前端生产输出，经已提交的诊断读取；guest 侧一行未改。

## 2. 实测结果

### 2.1 缺口清单（168 站点，**全部**组件被拒站点）

**缺的算子（3 项）**

| 算子 | 站点数 | 后端发射形态（`crates/sico-codegen-wasm/src/lib.rs`） |
|---|---|---|
| `less_fixed` | **167** | `emit_fixed_comparison(..., equal=false)`（`:2598`）⇒ `local.get left; local.get right; i64.lt_s(0x53) / i64.lt_u(0x54); local.set result` |
| `const_bool` | **154** | `I32Const(i32::from(value))` + `LocalSet`（`:2503`） |
| `checked_mul` | **54** | `emit_checked_mul`（`:3105`）⇒ `i64.mul` + 溢出条件 |

**缺的内建（14 个名字）**

| 内建 | 站点数 | 形态 |
|---|---|---|
| `sico.bytes.at` | 161 | **内联**（`:4800`） |
| `sico.text.encode` | 159 | **内联**（`:4452`） |
| `sico.list.get` | 157 | **内联**（`:4737`） |
| `sico.bytes.equal` | 156 | **需要 core helper**（`:4831` → `helper("sico.bytes.equal")`） |
| `sico.list.length` | 153 | **内联**（`:4728`） |
| `sico.u64.to_text` | 144 | **需要 core helper**（`:4939`） |
| `sico.text.starts_with` | 136 | **需要 core helper**（`:4590`） |
| `sico.bytes.slice` | 89 | **内联**（`:4676`） |
| `sico.bytes.utf8_decode` | 89 | **内联**（`:4487`） |
| `sico.list.get[U64]` | 73 | **需要 core helper**（集合单态化，`collection_intrinsic`） |
| `sico.text.split_lines` | 60 | **需要 core helper**（`:4568`） |
| `sico.list.empty[U64]` | 16 | **需要 core helper**（集合单态化） |
| `sico.list.append` | 11 | **需要 core helper**（`:4786`） |
| `sico.list.append[U64]` | 6 | **需要 core helper**（集合单态化） |

> **内联 / helper 的判据**：生产后端的 `helper_dependencies`（`crates/sico-codegen-wasm/src/lib.rs:510`）与 `INTRINSIC_HELPERS`（`:560`）决定了哪些内建要**新增一个 core 模块函数**（helper），哪些是调用点上的**内联指令序列**。这是**读冻结的 oracle**，不是设计判断。

### 2.2 终结符：**全部已支持**（推翻了先前的措辞）

| 终结符 | 站点数 | guest 发射端 |
|---|---|---|
| `branch` | 168 | ✅ `cgs_block_terminator_hex` → `cgs_branch_hex` |
| `jump` | 168 | ✅ → `cgs_jump_hex` |
| `match` | 168 | ✅ → `cgs_match_hex` |
| `return` | 168 | ✅ → `cgs_flat_return_hex` |
| `unreachable` | 132 | ✅ → `0x00` |

⇒ **「组件层缺的是分支 / 循环 / 多语句函数体的发射能力」是错的**：这五种终结符**早就**在发射端（`selfhost/codegen.sico:5053-5080`）。真实缺口是**算子与内建的词表**。

### 2.3 实施顺序的可量化形状

**每个站点需要几项新增**（工件 `aggregate.additions_needed_histogram`）：

| 需要项数 | 1 | 2 | 3 | 4 | 5 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 17 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 站点数 | 1 | 1 | 4 | 3 | 2 | 10 | 10 | 38 | 19 | 24 | 3 | 3 | 35 | 9 | 6 |

⇒ **中位数 9–10 项，最少 1 项、最多 17 项**；**没有任何站点只缺 6 项**（6 这一档为 0）。

**累计闭合曲线**（按「缺该项的站点数」降序逐项加入）：加到第 6 项只闭合 **8** 个站点，第 9 项 → **57**，第 11 项 → **87**，第 14 项 → **147**，**第 17 项（全部）→ 168**。

⇒ **组件轴在词表上是「全有或全无」**：任何子集都换不来成规模的站点闭合 ⇒ **必须按「先把 9 项内联项做完、再做 8 项 helper 项」推进**，不能指望逐项带来可见增量。

### 2.4 「9 项内联项」能换来什么（**预测，不是实测**）

9 项内联项 = 3 个算子（`less_fixed` / `const_bool` / `checked_mul`）+ 6 个内联内建（`sico.bytes.at` / `sico.bytes.slice` / `sico.bytes.utf8_decode` / `sico.list.get` / `sico.list.length` / `sico.text.encode`）。

按词表覆盖推算，这 9 项全实现后，**5 个站点的词表缺口清零**：

| 站点 | 文件 | 函数 | 它缺的项 |
|---|---|---|---|
| MS-013 | `parser.sico` | `is_ident_byte` | `const_bool`, `less_fixed` |
| MS-016 | `parser.sico` | `find_text_from` | `less_fixed`, `sico.bytes.at`, `sico.text.encode` |
| MS-017 | `parser.sico` | `word_span_offset` | `less_fixed`, `sico.bytes.at`, `sico.list.get`, `sico.list.length`, `sico.text.encode` |
| MS-020 | `parser.sico` | `gw_instr_count` | `less_fixed`, `sico.bytes.at`, `sico.text.encode` |
| MS-158 | `parser.sico` | `return_expression_start` | `const_bool`, `less_fixed`, `sico.bytes.at` |

⇒ **组件级全等预期从 9 → 14**（**这是按词表推算的预期，不是已测结果**；是否真的**逐字节等同**（含 core 模块的 helper 段、类型段、局部布局）必须等实现后**实测**，**本片不得据此改写账本 verdict**）。

**其余 163 个站点都至少缺 1 项 helper 内建**，其中：

| helper 内建 | 站点数 |
|---|---|
| `sico.bytes.equal` | **156** |
| `sico.u64.to_text` | **144** |
| `sico.text.starts_with` | **136** |
| `sico.list.get[U64]` | 73 |
| `sico.text.split_lines` | 60 |
| `sico.list.empty[U64]` | 16 |
| `sico.list.append` | 11 |
| `sico.list.append[U64]` | 6 |

⇒ **helper 项 = 组件层真正的主战场**（guest 要**新增 core 模块函数**并复现其**函数段 / 类型段 / 函数体字节**；guest 已有先例：`cg_concat_helper_body_hex` / `cg_text_length_helper_body_hex`）。

## 3. 本片**没有**做的事（诚实边界）

| 项 | 状态 |
|---|---|
| **组件轴闭合** | ❌ **没有**：本片只**测量**，**未改一行发射端代码** ⇒ 组件级全等仍是 **9 / 396** |
| **账本 verdict** | ❌ **未改**：账本 v1 仍是 `ir-equal+component-refused-guest:…`；**不按预测改钉** |
| **M-g3' / M-g2 / M-g3** | ⏳ 仍开（15 / 46 / 21 站点） |
| **闭包规模** | └ 仍 **219** 站点组件轴**未测**（闭包超限）⇒ 「未测」不是「通过」 |
| **语料覆盖** | └ 仍 **0 条** |
| **整文件自举** | └ 仍未关闭 |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；M22 整体仍 **NO-GO** |

## 4. 下一刀（按本片实测）

1. **发射词表 A（9 项内联）**：`less_fixed` / `const_bool` / `checked_mul` + `sico.text.encode` / `sico.bytes.utf8_decode` / `sico.list.length` / `sico.bytes.at` / `sico.list.get` / `sico.bytes.slice` ⇒ **预期**闭合 5 个站点（**待实测**）。
2. **发射词表 B（8 项 helper）**：先 `sico.bytes.equal`（**156** 站点），再 `sico.u64.to_text`（**144**）、`sico.text.starts_with`（**136**）… ⇒ 每项都要**新增 core 模块函数**并逐字节复现其函数体。
3. **M-g3'（15）/ M-g2（46）/ M-g3（21）**：前端三门仍开。
4. **闭包规模（219）**、**语料覆盖（0 条）**、**门 3 / 6**。

## 5. 验证记录（2026-10-24）

- `python tools/census-m22-emitter-vocabulary.py --update` ⇒ `EMITTER_VOCABULARY_WRITTEN sites=168`（exit 0）；
- `python tools/census-m22-emitter-vocabulary.py --check` ⇒ `EMITTER_VOCABULARY_CURRENT sites=168`（exit 0）；
- 闭包规则自检 ⇒ **OK**（逐条复现账本 219 条 `closure-too-large` verdict）；
- 电池新增步 `emitter-vocabulary-census`（`tools/validate-m22-s5.ps1`）⇒ 见 §5.1；
- 规划合同校验器 0124 / 0131 / 0156 ⇒ exit 0；`git diff --check` ⇒ exit 0。
- **独立 CI**：本片推送后触发下一轮，**裁决由下一片回填**。

### 5.1 本机电池（第 4 轮，**干净单实例**）

版本化电池 `tools/validate-m22-s5.ps1`（**15 步**；每步记录命令、耗时与 exit，任一步失败即整体失败）。日志：`target/tmp/m22-s5-battery/battery-step0537.log`（gitignored）。

| 步 | 秒 | exit | 命令 |
|---|---|---|---|
| fmt (workspace) | 1 | 0 | `cargo fmt --all -- --check` |
| fmt (runner) | 2 | 0 | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| clippy (workspace) | 1 | 0 | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| clippy (runner) | 1 | 0 | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| ci-one-key | 558 | 0 | `powershell -File tools/run-ci.ps1` |
| validate-0245 | 257 | 0 | `powershell -File tools/validate-step-0245.ps1` |
| validate-0262 | 276 | 0 | `powershell -File tools/validate-step-0262.ps1` |
| probe-frontend-frontier | 486 | 0 | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| probe-ir-plan-frontier | 27 | 0 | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| comment-discipline | 0 | 0 | `python tools/validate-selfhost-comment-discipline.py` |
| match-site-census | 0 | 0 | `python tools/census-m22-match-sites.py --check` |
| match-site-coverage | 130 | 0 | `powershell -File tools/probe-m22-match-coverage.ps1` |
| **emitter-vocabulary-census** | **25** | **0** | `python tools/census-m22-emitter-vocabulary.py --check` |
| firmware-selfhost | 244 | 0 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| git-diff-check | 0 | 0 | `git diff --check` |

⇒ **BATTERY_GREEN 15/15**（步数 **14 → 15**，新增 `emitter-vocabulary-census`）；固件 **177 / 177**（本片**零实现**）。

## 6. 附录（后续 STEP 的订正，**原文不改**）

### 6.1 STEP-0538：工件升 v1，并补上 **helper 轴**

- **工件改名**：`selfhost/emitter-vocabulary-v0.json` → **`selfhost/emitter-vocabulary-v1.json`**（schema `sico.m22.emitter-vocabulary.v0` → **`sico.m22.emitter-vocabulary.v1`**；v0 **已删除**，与站点账本 v0 → v1 同一口径）。逐站新增 `required_helpers` / `missing_helpers` 两列，聚合新增 `required_helper_frequency` / `missing_helper_frequency`。
- **为什么必须加这条轴**：本片只测「**调用点词表**」；但有些内建**除了调用点还要一个 core helper 函数**。`sico.bytes.utf8_decode` 就是这样一个 —— 它在 oracle 的 `INTRINSIC_HELPERS`（`crates/sico-codegen-wasm/src/lib.rs:565`）里 ⇒ 后端**除了调用点上的 (ptr,len) 透传**还要**额外发一个 458 B 的 core helper**（UTF-8 校验体）。本片把 `sico.bytes.utf8_decode` 列在「**内联**」一栏（`:4487` 的调用点确实是内联）⇒ **该分类只覆盖调用点，不覆盖 helper** ⇒ 本片的清单在 helper 这一维上是**不完整的**。STEP-0538 实测发现这一点后，给普查补上了 helper 轴（helper 规则**直接读冻结 oracle 的源码**，后端 sha256 冻进工件）。
- **本片的预测因此被交叉验证**：STEP-0538 用 helper 轴实测出的「**发射词表 B = 8 项 helper**」（`sico.bytes.equal` **156** / `sico.u64.to_text` **144** / `sico.text.starts_with` **136** / `sico.list.get[U64]` **73** / `sico.text.split_lines` **60** / `sico.list.empty[U64]` **16** / `sico.list.append` **11** / `sico.list.append[U64]` **6**）与本片 §2.4 的预测**逐项、逐站点数对上** ⇒ **预测成立，但工具化的分类口径在 helper 这一维需要补齐**。
- **§2.4 的「9 项内联项预期闭合 5 个站点」**：STEP-0538 实测 **7 项**（3 算子 + 4 内联内建）闭合 **4** 个站点（`MS-013` / `MS-016` / `MS-020` / `MS-158`）；第 5 个 `MS-017` `word_span_offset` 还缺 `sico.list.get` ⇒ **与本片的表一致**（该表里 `MS-017` 本就多列了 `sico.list.get`）⇒ **预测成立**。
- **本节原文保留**（它记录的是 2026-10-24 当时的事实）；当前口径见 [STEP-0538](./STEP-0538-m22-w2-s5-emitter-vocabulary-a1.md)。

---
