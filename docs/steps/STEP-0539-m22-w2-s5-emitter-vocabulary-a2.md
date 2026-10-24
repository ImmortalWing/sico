# STEP-0539 m22-w2 S5 **发射词表 A-2 实现片（C 拍）**：`sico.list.get` 与 `sico.bytes.slice` 落地，四槽 numeric Result 打通

> - 性质：**实现片**（组件 C 拍）。**`crates/*` 零改动**；**`selfhost/parser.sico` 零改动**（本片只动发射端 ⇒ 前端前沿不动）；**不推进任何出口门**；**M22 仍为 NO-GO**（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO）；**`match` 族仍未闭合**（组件级全等 **14 / 396**）。
> - 归属：[M22 计划 §3.3.5](./../plans/M22-compiler-self-host.md) 的「每族四拍（真值 T → 前端 F → 组件 C → 收口 X）」的对象 = **发射词表 A** 的**第二刀**。承 [STEP-0538](./STEP-0538-m22-w2-s5-emitter-vocabulary-a1.md)（A-1：7 项内联词表 + `sico.bytes.utf8_decode` 的 core helper）。
> - 产物：`selfhost/codegen.sico`（**+260 行**，5298 → **5743** 行）+ 固件改钉与新钉 + 站点账本 v1 刷新（1 个站点 verdict 前移）+ 发射词表工件刷新（164 → **163** 站点）+ 电池 15 步不变。
> - **上一片的独立 CI 裁决（本片回填）**：STEP-0538 的提交 `5c61771` ⇒ run **#49** = `completed` / **`success`**（id `40100457405`，`windows-gnu` job，13:58:54Z → 14:33:39Z = **34m45s**）⇒ **STEP-0538 的独立 CI 裁决为 GO**。

## 0. 入口门槛

- **缺口是真的**：STEP-0537/0538 的普查在组件被拒站点上逐站点命名了缺口；A-2 的两项（`sico.list.get` / `sico.bytes.slice`）是其中**唯一两项不带 helper 尾巴**的内建。
- **本片只做 A-2**：`sico.list.get` 是**最划算的一刀** —— 它**内联**（不带方括号的形式不需要 core helper），且**单独**就能闭合 **MS-017** `parser.sico:word_span_offset`。
- **不能按预测改钉**：STEP-0538 §2.4 的词表推算是**预期**，本片全部**实测**后才动账本与钉。

## 1. 本片方法

1. **先量后写**：用 Rust oracle 的**生产输出**（`compile_script_program`）作为唯一真值，先把两个形状的组件字节量出来，再写 guest 的发射字节。
2. **发射端落点**（`selfhost/codegen.sico`）：
   - **一处布局扩展**：`cgs_flat_hex` 的 `result` 分支新增 **`Result[string|bytes, NumericError]` = 4 槽**（`017f` + `017f017f` + `017f`）。此前该类型 **fail-close**（返回 "" ⇒ 槽数 0），是本片所有工作的前置。
   - **一个共用错误臂发射器** `cgs_numeric_error_hex`：tag=1 / pointer=0 / length=0 / error_tag=**0**（`NumericError.overflow` 在 Script profile **恒为常量 0**，读自 oracle 的 `VariantTags::new` 播种表）。
   - **两个内建发射器**：`cgs_list_get_hex`（越界 → 错误臂；否则读元素记录并**交换** `(ptr, len)`）、`cgs_bytes_slice_hex`（两条错误出口：超出 32 位 arena / 32 位回绕或越过源长）。
   - **一个字节助手** `cgs_local_tee_hex`（`local.tee` = `22` + LEB）。
   - **四道闸门全 fail-closed**：名字 / 实参个数 / 实参类型 / 结果槽数（= **4**），任一不过即 `ERR:E-SH-CG-IR-EMIT`（ADR-0016 决策 6）。
3. **改钉口径**（承 A-1 的纪律）：发射端词表变大 ⇒ 旧的「**必须拒**」的钉**升级**为「**必须逐字节等同**」的钉。

## 2. 实测结果

### 2.1 逐字节等同的形状

| 组 | 形状 | oracle 组件 | 结论 |
|---|---|---|---|
| 内建 `sico.list.get` | `match` 头（前端路径，函数名 `size`） | **1246 B** | ✅ 逐字节等同 |
| 内建 `sico.list.get` | `match` 头（注 IR 路径，函数名 `word_at`） | **1255 B** | ✅ 逐字节等同 |
| 内建 `sico.bytes.slice` | `match` 头（注 IR 路径，函数名 `slice_text`） | **1769 B** | ✅ 逐字节等同 |

- 三条形状都过**真实 `wasmparser::Validator`**；比较的是**整个组件**，不是长度。
- **两个内建的错误臂就在同一个函数体里** ⇒ 「逐字节等同」**同时**覆盖了 `ok` 臂与 `error` 臂的每一个字节（`tag=1 / ptr=0 / len=0 / error_tag=0`）。
- **闸门是活的**：把 `sico.list.get` 的下标实参去掉后，**Rust verifier 拒绝**（arity）、**guest 也拒绝**（自己的实参个数闸门）⇒ 不是「什么形状都收」。

### 2.2 改钉：两条「必须拒」升级为「必须逐字节等同」

| 钉 | 原断言 | 新断言 |
|---|---|---|
| `sico_codegen_guest_emits_the_match_component_bytes_byte_exactly` | 「`sico.list.get` 匹配头**必须留在 typed 拒绝**」 | 「组件与 oracle **逐字节等同**（**1246 B**）」 |
| `the_match_construct_truth_is_measured_and_pinned` | 同上 | 同上（字节等同） |

两条钉的**原意**（「拒绝优于近似」）仍成立，但**词表变大后**它们描述的形状已经能被逐字节复现 ⇒ 留着一个「必须拒」的钉会**把已达成的能力说成缺口** ⇒ 按 A-1 的同一口径升级（STEP-0530 的先例：保留原文 + addendum）。

### 2.3 新钉

- **`the_emitter_vocabulary_a2_covers_the_list_get_and_bytes_slice_shapes`**（新）：两个形状的整组件逐字节等同 + 一条**拒绝对照**（去掉下标实参）。
- **自举前沿钉**（`sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly`）的函数清单**新增 4 项**：`cgs_local_tee_hex` / `cgs_numeric_error_hex` / `cgs_list_get_hex` / `cgs_bytes_slice_hex` ⇒ 本片新增的发射代码**自己**能经 guest 前端逐字节降为 Rust canonical IR。

### 2.4 账本：组件轴 **13 → 14 / 396**

| 站点 | 文件 | 函数 | verdict 变化 |
|---|---|---|---|
| **MS-017** | `parser.sico` | `word_span_offset` | `component-refused` → **`component-equal`** |

- **组件被拒站点 164 → 163**；**组件级全等 13 → 14 / 396**。
- **与 STEP-0538 的词表推算一致**：STEP-0538 §2.4 判定 MS-017「只要 `sico.list.get`（无 helper）」即可闭合 ⇒ **实测闭合**。
- **MS-002 行号位移**（`codegen.sico` `cgs_add`：1567 → **1580**）：本片给 `codegen.sico` 增行 ⇒ 账本按**稳定身份**（路径+函数+序号）保留 verdict、只更新行号 ⇒ **没有**把已闭合的站点记回 `pending`。

### 2.5 普查工件：剩余缺口**只剩 8 项 helper**

`selfhost/emitter-vocabulary-v1.json` 刷新为 **163** 站点：

- **缺的算子：0**（`missing_op_frequency` 为空）。
- **缺的内建 = 缺的 helper，一一对应**：

| helper | 站点数 | 形态 |
|---|---|---|
| `sico.bytes.equal` | **156** | core helper |
| `sico.u64.to_text` | **144** | core helper |
| `sico.text.starts_with` | **136** | core helper |
| `sico.list.get[U64]` | **73** | 集合单态化 helper |
| `sico.text.split_lines` | **60** | core helper |
| `sico.list.empty[U64]` | **16** | 集合单态化 helper |
| `sico.list.append` | **11** | core helper |
| `sico.list.append[U64]` | **6** | 集合单态化 helper |

⇒ **发射词表 A 已清空**；**发射词表 B = 这 8 项 helper**（与 STEP-0537/0538 的预测逐项、逐站点数一致）。

**新的「还差几项」直方图**（现在只可能差偶数项：每个 helper 同时算 1 个内建 + 1 个 helper）：

| 还差 | 2 | 4 | 6 | 8 | 10 | 12 | 16 |
|---|---|---|---|---|---|---|---|
| 站点数 | 10 | 22 | 56 | 23 | 36 | 10 | 6 |

⇒ **上界从 18 降到 16**（`sico.bytes.slice` 那一条缺口消失）；中位仍在 **6–10** 一档。

### 2.6 本片**发现**的一个独立缺口（不属 A-2，未修）

测量期间命中一条**与 A-2 无关**的 guest 缺口：**`cgs_core_type_hex` 拒绝「非 `run` 函数返回 `result` 类型」**（`selfhost/codegen.sico` 的 return-kind 闸门）。

- 它**不是** A-2 的阻塞项：冻结 S 的真实调用点里，`sico.list.get` / `sico.bytes.slice` 都在 `match` 里，**所在函数返回 Text / List[Text] 等**，不返回 `result` ⇒ 真实站点不受影响。
- 但它是一条**真实的能力边界**：一个「直接返回 `Result[...]` 的内部函数」仍会被 guest 拒绝。
- 本片**没有**动它：放宽它会把发射端覆盖面扩大到**没有证据**的形状（ADR-0016 决策 6）。**登记为待办**，归「发射词表 B 之后的整族收口」一并处理。

## 3. 本片**没有**做的事（诚实边界）

| 项 | 状态 |
|---|---|
| **`match` 族闭合** | ❌ **没有**：组件级全等 **14 / 396**；**发射词表 B（8 项 helper）** 未做 |
| **闭包规模** | └ 仍 **219** 站点组件轴**未测**（闭包超限）⇒ 「未测」不是「通过」 |
| **语料覆盖** | └ 仍 **0 条** |
| **整文件自举** | └ 仍未关闭 |
| **M-g3' / M-g2 / M-g3** | ⏳ 仍开（**15 / 46 / 21** 站点） |
| **M22 出口门** | ❌ 门 3 / 6 仍 **NO-GO**；M22 整体仍 **NO-GO** |
| **oracle** | ✅ `crates/*` **零改动**；真值全部取自 oracle **生产输出** |
| **「返回 result 的内部函数」** | ❌ 仍被拒（§2.6），已登记 |

## 4. 下一刀（按本片实测）

1. **发射词表 B**（8 项 helper）：先 `sico.bytes.equal`（**156** 站点）、再 `sico.u64.to_text`（**144**）、`sico.text.starts_with`（**136**）… ⇒ 每项都要**新增 core 模块函数**并逐字节复现其**类型段 / 函数段 / 函数体**；**排序位会同时影响调用点的 helper 索引**（A-1 的 `sico.bytes.utf8_decode` 已经演示过一次）。
2. **M-g3'（15）/ M-g2（46）/ M-g3（21）**：前端三门仍开。
3. **闭包规模（219）**、**语料覆盖（0 条）**、**门 3 / 6**。
4. **整族收口**时一并处理 §2.6 的「返回 `result` 的内部函数」缺口。

## 5. 验证记录（2026-10-25）

**整轮版本化电池：`BATTERY_GREEN 15/15`**（单实例，2026-10-25 00:00 → 00:47:01 本机时，累计 **2821 s**）。
日志：`target/tmp/m22-s5-battery/battery.log`（版本化脚本：`tools/validate-m22-s5.ps1`）。

| # | 步骤 | 耗时 | exit | 命令 |
|---|---|---|---|---|
| 1 | `fmt (workspace)` | 1 s | **0** | `cargo fmt --all -- --check` |
| 2 | `fmt (runner)` | 2 s | **0** | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| 3 | `clippy (workspace)` | 1 s | **0** | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| 4 | `clippy (runner)` | 1 s | **0** | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| 5 | `ci-one-key` | 720 s | **0** | `powershell -File tools/run-ci.ps1` |
| 6 | `validate-0245` | 341 s | **0** | `powershell -File tools/validate-step-0245.ps1` |
| 7 | `validate-0262` | 375 s | **0** | `powershell -File tools/validate-step-0262.ps1` |
| 8 | `probe-frontend-frontier` | 518 s | **0** | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| 9 | `probe-ir-plan-frontier` | 27 s | **0** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| 10 | `comment-discipline` | 0 s | **0** | `python tools/validate-selfhost-comment-discipline.py`（GREEN files=2） |
| 11 | `match-site-census` | 0 s | **0** | `python tools/census-m22-match-sites.py --check`（LEDGER_CURRENT sites=396） |
| 12 | `match-site-coverage` | 280 s | **0** | `powershell -File tools/probe-m22-match-coverage.ps1` |
| 13 | `emitter-vocabulary-census` | 25 s | **0** | `python tools/census-m22-emitter-vocabulary.py --check`（EMITTER_VOCABULARY_CURRENT sites=163） |
| 14 | `firmware-selfhost` | 421 s | **0** | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` ⇒ **178 passed / 0 failed** |
| 15 | `git-diff-check` | 0 s | **0** | `git diff --check` |

**补充（同一快照，另行复跑）**：`validate-step-0124.ps1` / `validate-step-0131.ps1` / `validate-step-0156.ps1` 全部 exit 0。

**判定**：本片的实现、改钉、账本刷新、普查工件刷新与文档同步在**同一份快照**上通过整轮电池 ⇒ **STEP-0539 达成**。
但**不推进任何出口门**：门 3 / 6 仍 **NO-GO**，M22 整体仍 **NO-GO**；`match` 族组件级全等仍是 **14 / 396**。

**独立 CI 裁决**：本片提交推送后触发 `codex/m22-w1-ci` 的 **run #50**；其裁决**由下一片回填**（不在本片预写结论）。

---
