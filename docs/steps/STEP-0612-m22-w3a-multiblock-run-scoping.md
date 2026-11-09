# STEP-0612 — M22 W3-a 前置侦察：multi-block-`run` 组件发射缺口 scoping（只读片；订正 STEP-0610 的「两侧共享」判读）

- Date: 2026-11-08
- Track: M22 compiler self-host, goal **W3-a**（`AGENTS.md` §10.7 / `docs/plans/M22-compiler-self-host.md`）
- Slice type: **read-only recon / diagnostic** — zero file modifications, zero gates flipped, nothing committed or pushed. 取号核对：`docs/steps/` 当时最高为 STEP-0610；并行的 op 级燃料剖析片（D-A）进行中、预计占 STEP-0611，本片取 **STEP-0612**；若后续出现取号冲突，按「已提交方保留」规则调和并回补登记。
- Worktree HEAD at start: `b5ae008`（`b5ae0083067997d57ff72790d65427f4e73caac1`）.

## 1. Goal

把 W3-a 的真正挡板 scoping 清楚，为下一把实现刀（multi-block-`run` 发射）产出设计决策材料：
oracle 拒绝语义、guest 需要的改动面、真实覆盖面（formatter 实形 + accepted 语料）、是否需要
RFC/ADR、切片规划。**本片最重要的产出是一个判读订正**（§2），使 b5ae008 中「两侧共享缺口」的
登记被 addendum 修正。

## 2. 核心订正：STEP-0610 的「两侧共享缺口」判读不成立，W3-a 是 guest-only 缺口

- **决定性证据（STEP-0608 已有、此前被误读）**：`target/tmp/step0608/formatter-component-measurement.log`
  记录 `oracle_bytes_A_main = 28459`、`oracle_bytes_B_run = 28459`、`oracle_size_delta_B_minus_A = 0`
  —— **Rust oracle 对整份 `formatter.sico`（`main` 入口与 `run` 入口）都成功发射了 28459 字节完整
  Component**。oracle 从未拒绝过真实 formatter 的 branched `run`。
- **`stdlib helper layout` 拒绝（`lib.rs:4408-4412`）的真实根因 = fixture 伪信号**：
  `sico.bytes.is_utf8` 发射臂（`lib.rs:4499-4508`）调用 `helper("sico.bytes.utf8_decode")`，但
  `helper_dependencies`（`lib.rs:510-556`）**无 `is_utf8` 条目**（默认臂返回 `&[]`）⇒ 只用
  `is_utf8` 而从不直接调用 `utf8_decode` 的模块缺 helper 注册。STEP-0610 的手写 fixture
  `IS_UTF8_GATE_FIXTURE_SOURCE` 恰好如此；**真实 `formatter.sico:819` 直接调用
  `sico.bytes.utf8_decode`** ⇒ helper 已注册 ⇒ oracle 顺利发射。
- **结论**：W3-a 是 **guest-only** 实现缺口；oracle 侧唯一卫生项为 1 行
  `helper_dependencies` 补条目（可选、非 W3-a 必需，但建议先行以免 G1 刀的 fixture 误判）。

## 3. Oracle 侧语义（`crates/sico-codegen-wasm/src/lib.rs`，已支持任意块数）

- `compile_function` 体发射（`:1691-1748`）：所有函数（含 `run`）统一 dispatcher 循环
  （`i32.const <entry块id>; local.set <dispatcher>; block; loop; 每块
  <dispatcher==block_id 分派头><指令><终止子>; unreachable; end; end`）。
- `compile_terminator`（`:2263-2323`）：已实现 `Return`/`Jump`/`Branch`（if/else 分派 + `br 1`）/
  `Match`/`Unreachable` 五种终止子——`Branch` 即 multi-block 核心结构。
- `emit_script_return`（`:3373-3463`）：单 return 点 result-area 发射，**每个 `Return` 终止子各发
  一份**，共用同一对 scratch 局部（`:3381-3383`），每时刻只走一条路径故无冲突。
- **契约判定**：`docs/development/script-component-emission-v0.md` §4（dispatcher 多块体）与 §6
  （result lowering）已冻结该形状，**无任何「run 必须单块」条款** ⇒ **既有契约内的实现缺口，
  不需要 RFC/ADR**。可选清洁项：§6 补一句「本序列在每个 `return` 终止子处各发一次」。

## 4. Guest 侧缺口（`selfhost/codegen.sico`，4 个具名挡点）

| # | 挡点 | 位置 | 行为 |
|---|---|---|---|
| ① | 多块硬拒 | `cgs_result_return_hex:4798-4800` | run 有第 2 块即 `ERR:E-SH-CG-IR-EMIT` |
| ② | 只读块 0 返回值 | `cgs_run_return_value:4388-4417` | 只取 `blocks[0].terminator`；多块时块 0 是 `branch` ⇒ 返回空 ⇒ ①②叠加拒 |
| ③ | run 终止子一律拒 | `cgs_block_terminator_hex:6630-6632` | `if cg_is_word(name,"run"): return ERR` |
| ④ | run 指令段无终止子 | `cgs_run_instr_hex:4121-4128` | 以 `terms=0` 调 `cgs_function_instr_hex` ⇒ 每块 `if` 无 `0b` 收口 |

**推广方向**：把「单块 run 的 result-area 推导」推广为**逐块发射**——`return` 终止子 → 内联
result-area 序列（现 `cgs_result_return_hex:4835-4893` 的 ok/error 臂，逐 return 点各发一份）；
`branch` 终止子 → 现成的 `cgs_branch_hex`（`:5478-5513`）；逐块返回值定位复用既有
`cgs_instruction_for_value`/`cgs_value_slot`/`cgs_variant_payload`/`cgs_construct_has_field`。
**规模量级**：约 **150–300 行 Sico 净增、3–6 个新/改造函数**。**主风险 = 自举形状前沿**
（新函数必须能被 guest 前端自己降低；缓解：函数保持小、避免循环内 `match`、复用
STEP-0571/0605/0609 已验证的「纯插入 + 委托」模式，逐片以 `step0513` 自举守卫复核）。

## 5. 真实覆盖面盘点

- **formatter `run` 实形**（`formatter.sico:817-827`，生产 CLI 经 `prepare_script_module` 把
  `main`→`run`）：2 层嵌套 `if`、无 loop；**3 个 return 点**（内层 `error(LEXICAL)`/内层
  `ok(ScriptOutput{...})`/外层 `error(not-UTF-8)`）；约 5 个基本块（精确值建议 G1 开工前以
  `--emit-ir-plan` 复核）；result-area 涉及 `ScriptOutput` 与 `ScriptError` 两种 record。
- **accepted 语料（W3-b）**：本片初稿曾把「37 accepted」与 `selfhost/codegen-corpus-v0.json`
  （16 条 byte-equal + 7 条 typed-refusal = **23 条**，Core Direct 探针语料）比对并判「口径漂移」
  ——**该比对混同了两个语料，登记作废（coordinator 订正）**。真实口径链：**「37 accepted /
  178 refused」是 STEP-0220 冻结时 `selfhost/script-build-corpus-v0.json` 的基线快照**（当时共
  215 条，README:256）；当前该文件实测 **235 条**（`rust_script_build_exit=0` 且
  `component_reproducible=True` 的 **42 条** / 拒绝 46+147=193 条）⇒ 语料在冻结后增长了 20 条，
  登记里的 37/178 是**冻结时点数、非当前计数**，W3-b/W4 开工前必须按当前文件重计。
  **关键新事实（coordinator 复核补测）**：42 条 accepted 中 **37 条源含控制流关键字
  （if/match/while）** ⇒ W3-b 的组件级差分几乎全部需要 multi-block-`run` 发射 ⇒
  **G1–G3 这把刀同时解锁 W3-a 与 W3-b 主体，杠杆比本片初判更大**。
- 账本中 formatter 站点（如 MS-009 `add`）已 `ir-equal+component-equal` ⇒ guest 已能发射
  formatter 的 helper 函数；**multi-block run 是 W3-a 唯一挡板**。

## 6. 切片规划建议（预计 3–4 片）

| 片 | 内容 | 验证 | 规模 |
|---|---|---|---|
| **O1**（oracle 卫生，可并入 G1） | `helper_dependencies` 补 `is_utf8→utf8_decode`（1 行）；STEP-0610 钉测试的 oracle 断言由 `is_err` 改发射成功 | `--exact sico_is_utf8_lowers --exact` | 1 行 |
| **G1**（guest 最小多块） | run 终止子发射器 + 逐 return 点 result-area；支持 `if + 两臂 return`（3 块，以 `IS_UTF8_GATE_FIXTURE_SOURCE` 形状为字节锚） | oracle vs guest 全等 fixture；`step0513` 自举守卫 | 1 片 |
| **G2**（guest 嵌套 if） | 扩到 formatter 实形（2 分支 + 3 return，约 5 块） | guest 对整份 renamed formatter 发射逐字节 == oracle 28459 B | 1 片 |
| **G3**（W3-a 收口） | 整份 formatter 全等 + `wasmparser::Validator`；词表/账本刷新；`script-component-emission-v0.md` §6 补注 | STEP-0608 `--ignored` 探针 + 出口审计项 | 1 片 |

次风险：账本行号位移重冻结（G1/G2 改 `codegen.sico` ⇒ `emitter-vocabulary` 刷新 + 逐站 digest，
属已知绑定成本）。

## 7. 诚实收窄

- 只读片：未修改任何文件、未跑 `cargo`、未跑完整前端；formatter `run` 的精确基本块数（5 vs 6）
  以 `--emit-ir-plan` 复核为准（G1 前置检查）。
- **语料口径订正（本片初稿混同，coordinator 已订正 §5）**：「37/178」= STEP-0220 冻结基线
  （`script-build-corpus-v0.json` 时点数）；当前该文件 235 条（42 accepted / 193 refused），
  W3-b/W4 开工前须按当前文件重计。`codegen-corpus-v0.json`（16+7）是 Core Direct 探针语料，
  与 W3-b 无关。
- W3-a 仍 open；**未翻任何门，M22 overall 仍 NO-GO**；本片未 commit/push。
