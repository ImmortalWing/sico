# M22 W4 拒绝轴对账现状（进行中）

> 状态：**进行中对账，不翻任何门**。本报告只做只读登记与现状盘点；M22 overall 仍 **NO-GO**，七门不变。
>
> 登记日期：2026-11-19。执行依据：[AGENTS.md §10.11-C](../../AGENTS.md)、[M22 计划 §3.3.7](../../plans/M22-compiler-self-host.md)。
> 工具：`tools/reconcile-m22-w4-refusals.py`（新建，只读默认）。

## 1. 语料在哪

W4 的「178 条拒绝语料」在 **`selfhost/script-build-corpus-v0.json`**（schema `sico.m22.script-build-corpus.v0`，
`source_manifest_sha256 = 7ff05ff4…8ad4`）。该文件是 **build 阶段**语料：每条含
`path / source_sha256 / rust_script_build(verdict) / rust_script_build_exit / build_diagnostic_ids /
build_diagnostic_sha256 / component_sha256 / component_reproducible`。

同仓库另有一套 **check 阶段**语料 **`selfhost/corpus-v0.json`**（schema `sico.m22.corpus.v0`），
每条含 `rust_check(verdict) / rust_check_exit / diagnostic_ids / …`。两套语料的 `path` 集合**完全一致**
（各 235 条）。

既有 parity 探针 `tools/probe-m22-corpus-parity.ps1` 与既有冻结工件
`selfhost/corpus-parity-v0.json` / `selfhost/corpus-parity-map-v1.json` 跑的都是 **check 语料**的
`rust_check=refused` 子集（STEP-0561 / STEP-0564）。

### 1.1 两套语料的 W4 相关子集

| 语料 | W4 目标子集 | 当前条数 | 构成 |
|---|---|---|---|
| `script-build-corpus-v0.json`（build） | `rust_script_build = refused` | **193** | LEXICAL 116 / SCRIPT_ABI 31 / E-code 46 |
| `corpus-v0.json`（check） | `rust_check = refused` | **162** | LEXICAL 116 / E-code 46（**无 SCRIPT_ABI**） |

两套语料 162 条 check-refused 全部是 build-refused 的子集；多出的 31 条（SCRIPT_ABI）全部是
**check=accepted / build=refused**。

## 2. 178 条构成是否对上 —— **对不上（语料漂移）**

计划 §3.3.7 与语料行冻结的是 **STEP-0220 时点**：`37 accepted / 178 refused`，
诊断分布 **LEXICAL 116 / SCRIPT_ABI 28 / 散码 34**。当前 `script-build-corpus-v0.json` 已漂移至：

| 类别 | 计划冻结 | 当前 | 差 |
|---|---|---|---|
| LEXICAL | 116 | 116 | **0（对上）** |
| SCRIPT_ABI | 28 | 31 | **+3** |
| 散码（E-code） | 34 | 46 | **+12** |
| 合计 refused | **178** | **193** | **+15** |

**结论：`178 条 / 116 / 28 / 34` 是过期冻结点，当前语料是 `193 / 116 / 31 / 46`。** 逐条对账必须按
当前语料 193 条进行；计划口径 178 与当前文件不一致，如实登记为**语料漂移 gap（G1）**，不臆造、
不回写冻结点。

## 3. 对账脚本

新建 `tools/reconcile-m22-w4-refusals.py`（Python 3，风格参考 `tools/census-m22-match-sites.py`）。

用途：遍历拒绝语料，逐条跑 guest 前端（`target/debug/sico.exe run selfhost/compiler.sico`，
源走 stdin，`SICO_RUNNER` 指向 `runner/sico-runner/target/debug/sico-runner.exe`，与
`probe-m22-corpus-parity.ps1` 完全同构），产出每条的
`path / 期望(verdict, diag_id, span) / 实际(verdict, diag_id, span) / parity / diff_class`。

- 期望侧：`expected_verdict` 取自语料冻结的 `rust_script_build`（或 `rust_check`）；
  `expected_diagnostic_ids` 取自冻结的 `build_diagnostic_ids`（或 `diagnostic_ids`）；`expected_span` = **不可用**
  （见 §4 G2）。
- 实际侧：guest 运行结果 → `REFUSED:<ERR:E-SH-IR-*>` | `LOWERED` | `FAILED:<…>` | `MISSING-SOURCE`。
- 判定（按 `selfhost/corpus-parity-map-v1.json` 的映射契约）：
  - `exact`：guest 拒绝且 `(rust id → guest code)` 对已登记；
  - `structural`：guest 拒绝但该对未登记（仅 verdict 级一致）；
  - `zero`：guest 未拒（`LOWERED`=声音洞，`FAILED`=燃料墙/其他运行失败）；
  - `not-guest-reachable`：SCRIPT_ABI 排除（build 语料专用，见 §5 G3）。

用法：

```text
python tools/reconcile-m22-w4-refusals.py                       # build 语料全量，stdout
python tools/reconcile-m22-w4-refusals.py --corpus check --limit 40
python tools/reconcile-m22-w4-refusals.py --corpus check --out <csv路径> --json-out <json路径>
python tools/reconcile-m22-w4-refusals.py --update --freeze-out <新文件路径>   # fail-closed：必须显式给路径，拒绝覆盖已有文件/selfhost/
```

纪律落实：默认**只读**，仅当显式给 `--out`/`--json-out` 才写文件（本报告跑批全部写 `target/tmp/`）；
`--update` 是**失败关闭**的冻结动作——要求显式 `--freeze-out`，且**拒绝覆盖已存在文件**、**拒绝写入
`selfhost/`**（冻结语料空间）。本会话**未执行 `--update`**。

## 4. span 轴：结构性不可达（G2）

真实 `(verdict, diagnostic id, span)` **三元组**逐条一致**在当前证据下不可达**：

- Rust oracle `check --json` 能产出完整 span（`range.line:col` / `byte`），但冻结语料的
  `diagnostic_ids` 是粗分类（对 check 文本正则 `E[0-9]{4}`，否则 `LEXICAL`），**不带 span**；
- guest 前端只发一个裸 `ERR:E-SH-IR-*` 码，**无任何 span 输出**。

`selfhost/corpus-parity-map-v1.json` 的 `.span_axis` 已把此登记为
`status=blocked-guest-side`，`consequence` 明确「三元组 parity 不可达，只登记
`(rust_code ↔ guest_code)` 等价类；span parity 记为残余 gap，不断言」。本对账脚本沿用该口径：
`expected_span`/`actual_span` 均记为 `not-comparable`，**不臆造期望 span、不绕过**。

## 5. 发现的结构性 gap（如实登记，不修）

| # | gap | 证据 | 处置 |
|---|---|---|---|
| G1 | 语料漂移：`178 → 193`（SCRIPT_ABI +3、散码 +12），计划 §3.3.7 的 178/116/28/34 是 STEP-0220 过期冻结点 | §2 表 | 按当前 193 条对账；计划口径与现状差异留 owner 裁决，不回写冻结点 |
| G2 | span 轴结构性不可达：guest 无 span、冻结语料无 span | `corpus-parity-map-v1.json .span_axis` | 只做 verdict+id 级对账，span 记为残余 gap |
| G3 | 31 条 SCRIPT_ABI **guest 前端不可达**：全部 check=accepted / build=refused，guest 前端（`sico run compiler.sico`）够不到 ABI 检查 | §1.1 表 + STEP-0561 排除登记 | 这些条目记为 `not-guest-reachable`，**不跑 guest**（跑了会误判成 LOWERED 声音洞）；ABI 轴对账依赖 Rust 侧 `script_build` 冻结期望，无 guest 侧证据 |
| G4 | 期望侧无独立冻结 `(verdict, id, span)` 三元组清单：期望来自语料内嵌的 `diagnostic_ids`（coarse 分类），非独立清单 | 语料字段结构 | 期望侧以语料冻结 `diagnostic_ids` 为准；若 owner 要求独立冻结清单，为后续工作项 |
| G5 | parity map 只覆盖部分 `rust_code → guest_code` 对（**33 个映射对 = 30 个命名等价类**，32 refined + 1 coarse；coarse = `LEXICAL ↔ ERR:E-SH-IR-FUNCTION-COUNT`（98 项语料）；共覆盖 29 个不同 rust 码）；存在 `unmapped`（structural）残余 | `corpus-parity-map-v1.json` + 既有 parity 工件 `structural=7` | 逐条对账时 `structural` 单独计数，不并入 exact |

## 6. 既有 parity 工件（参考基线）

`selfhost/corpus-parity-v0.json`（schema `sico.m22.corpus-parity.v0`，STEP-0561 冻结 / STEP-0564 扩展）
当前登记：`checked=162 / exact=151 / structural=7 / zero=4`（3×`scope-nesting-limit.sico` 燃料墙
+ 1×`syntax-candidates/b/nominal-invariants/invalid/missing-field.sico` 真实声音洞）。

本报告的 `tools/reconcile-m22-w4-refusals.py` 是**独立重跑对账**，不覆盖该冻结工件；两者应互相印证。

## 7. 首批评测（check 语料前 40 条，已验证复现）

2026-11-19 实跑：`python tools/reconcile-m22-w4-refusals.py --corpus check --limit 40`
（输出 `target/tmp/w4-reconcile-check-40.{log,csv}`，仅写入 scratch，未写冻结文件）。

**结果：`checked=40 exact=39 structural=0 zero=1`** —— 与先前会话抽查
（`checked=40 exact=39 structural=0 zero=1`）**逐项一致，复现成立**。

- 39 条 exact：guest 均拒绝且 `(LEXICAL → ERR:E-SH-IR-*)` 对已登记。其中
  `FUNCTION-COUNT`（coarse 主桶，98 项类）为主，另有 `RECORD-END` / `RECORD-TYPE`
  （refined 类）。
- 1 条 zero：`semantic-cases/future-task/invalid/scope-nesting-limit.sico` →
  `FAILED:{"class":"resource-limit.fuel","detail":"FuelExhausted","exit":125,…}`，
  分类 `zero-fuel`（**燃料墙，非声音洞**，与 verdict 正交）。不属本次对账的可修缺口，
  原样登记、不上调限额。
- structural = 0：前 40 条无未映射对。

**扩大批次**：全量 check 语料 162 条对账的**独立复跑**已启动
（`target/tmp/w4-reconcile-check-full.*`），本报告登记时**未跑完即收尾**（避免长时间等待；
后台任务已停止，无部分产物）。162 条的**参考基线**以既有冻结 parity 工件
`selfhost/corpus-parity-v0.json`（STEP-0561/0564 冻结）为准：**`checked=162 / exact=151 /
structural=7 / zero=4`**（3×`scope-nesting-limit.sico` 燃料墙 + 1×
`syntax-candidates/b/nominal-invariants/invalid/missing-field.sico` 真实声音洞）。本脚本的前
40 条独立结果与该冻结工件对应条目**逐项一致**，作为一致性旁证。全量独立复跑为后续工作项。

## 8. 结论

- W4 仍**未闭合**：语料已漂移（G1）、span 轴不可达（G2）、SCRIPT_ABI 轴无 guest 侧证据（G3）。
- 本报告与对账脚本**不翻任何门**，M22 overall 保持 NO-GO。
- 下一步：完成全量 193 条（build）/162 条（check）对账，将 `structural`/`zero` 逐条归因。
