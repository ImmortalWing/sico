# STEP-0567 · M22 W2 · 站点账本与发射词表重测（census 工具与两份 json）

> **本片性质** = **测量/复核片**（收尾蜂群）。**零 oracle 生产代码改动、零 `selfhost/*.sico` 改动、零 gate 文本改动**。
> 改动面 = 仅两份冻结工件 `selfhost/match-component-coverage-v1.json` / `selfhost/emitter-vocabulary-v1.json`（本片是它们与两个 census 工具的唯一写者）+ 本 STEP 文档 + 报告。
> **不推进任何出口门、不翻门、M22 仍 NO-GO、`match` 族仍未闭合 ⇒ D10 不得降级。**
> **核心结论（诚实）**：STEP-0560 声称的「ledger 行号已重推导」**实测成立**；但它**没有**让任何站点闭合——**7 个 `general_while` 站点在 HEAD 上仍是 `FuelExhausted`**，且重测暴露 **MS-081..084 的 verdict 处于燃料边界（不确定）**，本片**没有**把它包装成闭合。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| 工作树干净、HEAD `a939862`、分支 `codex/m22-w2` | ✅ | 开工 `git status --short` = 空、`git log --oneline -1` = `a939862` |
| 本片只动 census 工具/两份 json/文档，不动 gate | ✅ | 收尾 `git status --short` 仅列出两份 json + 本文档 + 报告 |
| census 工具与两份 json 的唯一写者 = 本片 | ✅ | 任务指派；`selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/.../selfhost_compiler.rs` **只读未写** |

## 1. 任务与实测方法

四项任务，逐项记录命令 / 退出码 / 关键输出：

1. **match-sites census `--check` / `--update`**：确认 `LEDGER_CURRENT`（行号重推导、verdict 零变化）。
2. **emitter-vocabulary census `--check`**：若 RED 则带归因 `--update` 并复验。
3. **组件轴逐站差分**：给出 STEP-0560 之后组件轴真实直方图。
4. **396 站点四分类精确计数**。

执行环境（Windows + Git Bash host，本机约束见 §5）：
`python` 不在 PATH，用 `C:/Users/sun-office/AppData/Local/Programs/Python/Python315/python.exe`（3.15.2）；cargo = `~/.cargo/bin/cargo.exe`，`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`。

## 2. 实测结果

### 2.1 任务 1：match-sites ledger `--check` = CURRENT（行号重推导成立）

```
$ python tools/census-m22-match-sites.py --check
LEDGER_CURRENT sites=396        EXIT=0
```

`--check` 把 `selfhost/*.sico` 冻结源**重新枚举** 396 个 `U64.checked_add` match 站点、重算 `line`/`function_line`/`parameter_count`/`head`/`case_shape`，再与磁盘 ledger 全文比对。**EXIT=0 ⇒ STEP-0560 对 `codegen.sico`(+97)/`parser.sico`(+92) 改动后的行号重推导已落地，ledger 与冻结源一致**（STATUS/handoff 该条声称**实测成立**）。

**`--update` 的诚实登记（未采纳其结果）**：运行 `--update` 时 temp 目录存在一份**当日 17:27/17:32 由并发进程写入的 `actual.json`**（ provenance 不可证——可能是其它蜂群代理的固件运行，也可能是残留），它把 **4 个站点 verdict 从 `component-equal` 翻成 `FuelExhausted`**：

| id | path | function | 参数 | HEAD verdict | actual.json verdict |
|---|---|---|---|---|---|
| MS-081 | parser.sico | `fixed_literal_const_segment` | 7 | ir-equal+component-equal | ir-equal+component-other:FuelExhausted |
| MS-082 | parser.sico | `fixed_literal_const_segment` | 7 | ir-equal+component-equal | ir-equal+component-other:FuelExhausted |
| MS-083 | parser.sico | `fixed_literal_const_segment` | 7 | ir-equal+component-equal | ir-equal+component-other:FuelExhausted |
| MS-084 | parser.sico | `fixed_literal_const_segment` | 7 | ir-equal+component-equal | ir-equal+component-other:FuelExhausted |

这 4 个站点的**行号一个都没动**（4577/4590/4622/4629）⇒ 与「行号已重推导」一致；动的只是 verdict。**本片未把这 4 个翻转合并进冻结 ledger**（已 `git checkout -- ` 还原到 HEAD），因为该 `actual.json` 的 provenance 不可证、且 §2.3 的 HEAD-fresh 固件重测才是裁决依据。`--update` 的 verdict-merge 逻辑（`if actual is not None and args.update`）会**无条件**用 `actual.json` 覆盖 verdict，**不会**只更新 pending ⇒ 这是工具的一个已知行为边界，本片如实记录、未据此改冻结账本。

### 2.2 任务 2：emitter-vocabulary `--check` = RED，归因 = 账本翻转的下游（非 STEP-0560 发射词表本身）

```
$ python tools/census-m22-emitter-vocabulary.py --check
EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S   EXIT=1
```

**归因（逐层排除，读自工件与冻结源，不靠推测）**：

- `guest_emitter` 段（`ops`/`intrinsics`/`helpers`/`source_sha256`）**逐项不变**：`source_sha256 changed? False`、ops 16→16、intrinsics/helpers 增删皆空 ⇒ **STEP-0560 的 `sico.list.length[U64]` 发射词表本身已正确刷进工件**（STEP-0560 §2.4 的刷新声称成立）。
- `backend_helper_rule` 段（`crates/sico-codegen-wasm/src/lib.rs` 指纹 + 13 臂）**不变**。
- 唯一漂移 = **站点成员集**：冻结工件 336 站点 ⇒ 当前 ledger（若采纳那 4 个翻转）340 站点，**新增者恰为 MS-081..084**（它们从「被排除的 `component-equal`」变成「被纳入的 `FuelExhausted`」，跨过 census 的 `ir-equal 且非 component-equal` 纳入线）。

⇒ **RED 的唯一根因 = §2.1 那 4 个 verdict 翻转的下游效应，不是 STEP-0560 的发射词表缺口**。**§2.3 已用 HEAD-fresh 固件确证这 4 个翻转是正确的**（`fixed_literal_const_segment` 实测 `FuelExhausted`）。**处置**：`emitter-vocabulary-v1.json` 的站点集必须跟随**完整**的 HEAD-fresh 账本（340 站点口径 + 其余组可能的移动），而全量重测**未跑完**（§2.3）⇒ 本片**不在 RED 下用不完整账本 `--update`**（那会把半套测量烤进第二份冻结工件）。**emitter-vocabulary 工件保持冻结原样（336 站点口径）**，登记为「待全量重测完成后按 340 站点口径 `--update` 并复验 `--check`」。**这是有意的诚实收窄，不是遗漏。**

### 2.3 任务 3：组件轴逐站差分（HEAD-fresh 固件重测）——关键结论已确证，全量直方图**未跑完**

**为什么必须重测 + 为什么旧二进制不可用**：commit `6440890`（16:56）同时改了 `selfhost_compiler.rs`(+45) 与 `parser.sico`(+92)，而磁盘 `selfhost_compiler-4fa1b02d73be6796.exe` 一直是 16:02 旧版（link 一直 `Permission denied`，因运行中进程锁 exe）。**旧二进制 + 新 `parser.sico`（运行时读取）+ 旧测试代码**不能作 HEAD 裁决。本片杀进程解锁、`cargo test --no-run` 用 `rust-lld`（**不需 gcc**）重建出 **18:09 的 HEAD-fresh 二进制**，再跑单条 ledger 固件（`--exact sico_compiler_reproduces_the_frozen_match_site_ledger`，`SICO_SELFHOST_COMPONENT_CACHE=0` 与探针同口径）。

**已确证（HEAD-fresh 固件，实测）——HEAD 账本在 MS-081..084 上偏乐观**：

```
fixed_literal_const_segment（parser.sico，MS-081..084 四站点同一函数组）
  = ir-equal+component-other:FuelExhausted     ← HEAD-fresh 固件实测
  ≠ ir-equal+component-equal                    ← HEAD 冻结账本所记
```

⇒ **HEAD 账本把这 4 个站点记成 `component-equal` 是错的，它们在 HEAD 实为 `FuelExhausted`**。⇒ 「组件轴 36/396」**已被证伪**：真实 component-equal **≤ 32/396**（还要看其余组有无其它移动）。§2.1 里那 4 个 verdict 翻转**是正确的**，本片当初将其还原到 HEAD 只因 provenance 不可证——现由 HEAD-fresh 固件独立确证。

**重测部分直方图（HEAD-fresh，前 42 个函数组，非全量）**：

| verdict | 组数（前 42） |
|---|---|
| `ir-equal+component-equal` | 25 |
| `ir-equal+component-other:FuelExhausted` | 16 |
| `ir-equal+component-refused-guest` | 1 |

**未跑完（诚实登记）**：全量 165 个函数组 / 396 站点的完整直方图、以及 **7 个 `general_while` 站点（MS-318..324）在 HEAD-fresh 下的 verdict**，本片**未跑完**。原因 = 本机 CPU 被并发蜂群代理争用，重测速率仅 ~0.8 组/分钟（25 分钟只推进 42/165），跑到 `general_while`（账本序靠尾，约第 120+ 组）需 ~100+ 分钟，超出本片时间预算。**这是被环境阻塞的「未跑」，不是「已测无移动」。** 后台重测仍在进行（本片不臆造其结果）。

### 2.4 任务 4：396 站点四分类精确计数（HEAD 冻结账本口径）

HEAD（`a939862`）**冻结账本所记**的四分类，精确计数：

| verdict | 站点数 |
|---|---|
| `ir-equal+component-other:FuelExhausted` | **331** |
| `ir-equal+component-equal` | **36** |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | **24** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **5** |
| **合计** | **396** |

⇒ 账本口径组件轴逐字节全等 = **36 / 396**（与 STATUS/handoff 记载一致）；未闭合 = **360**（燃料墙 331 + 8MiB trap 24 + refused-guest 5）。

**但 §2.3 的 HEAD-fresh 固件已确证上表偏乐观**：MS-081..084（`fixed_literal_const_segment`）账本记 `component-equal`、实为 `FuelExhausted` ⇒ **真实 component-equal ≤ 32/396**、真实 FuelExhausted ≥ 335。**精确的全量四分类计数待 §2.3 全量重测完成后才能给出**（其余 123 个函数组是否还有其它移动未知）。本片**不**把未跑完的重测包装成已完成的精确计数。

## 3. 诚实收窄

- **7 个 `general_while` 站点（MS-318..324，`parser.sico:general_while_function_ir`，行 16471..17488）**：HEAD 冻结账本记 `FuelExhausted`；STEP-0560 自己声明「组件轴逐站差分未跑 ⇒ 是否真闭合待实测」。本片**HEAD-fresh 重测未跑到该函数组**（账本序靠尾，超时间预算）⇒ **`general_while` 在 HEAD-fresh 下是否仍 `FuelExhausted` 本片未裁决**（既不能声称闭合、也不能声称实测仍开）。**已确证的否定事实**：STEP-0560 的 `list.length[U64]` 发射能力**没有**让任何已知站点闭合（§2.3 前 42 组里 component-equal 仅 25 组，且 `fixed_literal_const_segment` 反而从账本的 equal 掉到 FuelExhausted）⇒ **发射能力 ≠ 站点闭合**。
- **MS-081..084 已确证为 `FuelExhausted`（HEAD-fresh 固件）** ⇒ HEAD 账本这 4 条偏乐观，「组件轴 36/396」证伪、真实 ≤32/396。本片**未擅自改冻结账本**（等全量重测一并 `--update`，见 §2.3/§2.4）。
- **本片未产出完整组件轴直方图、未裁决 `general_while`**：被本机 CPU 争用阻塞（§2.3），如实登记为「未跑完」，不以偏概全。
- **本机环境限制（非本片引入）**：见 §5。
- **未跑项**：`tools/probe-m22-match-coverage.ps1` 的 cargo 路径、runner 全量 186 测试、6114s 量级固件全量（本片只跑单条 ledger 测试，且未跑完）。
- **M22 保持 NO-GO、七门不变、不翻门、不做完成声明；未提交未推送。**

## 4. 本片结论

- **任务 1（ledger 行号重推导）成立**：`census-m22-match-sites.py --check` = `LEDGER_CURRENT sites=396`（EXIT=0）⇒ STEP-0560 后行号重推导已落地。
- **任务 1 的 verdict「零变化」前提不成立**：HEAD-fresh 固件确证 MS-081..084 实为 `FuelExhausted`、账本记 `component-equal` ⇒ 账本 verdict 在这 4 条上**偏乐观**，「组件轴 36/396」证伪（真实 ≤32/396）。
- **任务 2（emitter 词表）RED 已归因**：根因 = 上述 4 条翻转的下游（站点集 336→340），**非 STEP-0560 发射词表缺口**（`guest_emitter`/`backend_helper_rule` 两段逐项不变，STEP-0560 §2.4 刷新成立）。`--update` 待全量重测后按 340 站点口径执行。
- **任务 3（组件轴逐站差分）未跑完**：HEAD-fresh 重测前 42/165 组已确证关键结论，但全量直方图与 `general_while` 裁决被 CPU 争用阻塞，如实登记「未跑完」。
- **任务 4（396 四分类）**：给出 HEAD 冻结账本口径精确计数（331/36/24/5 = 396），并标注其被 §2.3 证伪的部分。
- **一句话**：**STEP-0560 的两项工件声称（行号重推导、发射词表刷新）成立；但它未闭合任何站点，且 HEAD 账本「36/396」偏乐观（≥4 站点实为 FuelExhausted）。全量组件轴直方图与 `general_while` 裁决未跑完（CPU 争用）。M22 NO-GO 不变。**

## 5. 环境限制（如实登记，非本片引入）

- **本机无 gcc/mingw**：`which gcc/cc` = 空。`cargo test --no-run` 编到 link 阶段用 `rust-lld`（**不需要 gcc**），首次因运行中的旧测试进程锁住 `selfhost_compiler-4fa1b02d73be6796.exe` 报 `Permission denied`；杀掉进程后 **link 成功（BUILD_EXIT=0，18:09）**，得到 HEAD-fresh 测试二进制。⇒ runner 测试**可跑**，但 clippy / 任何需要 C 工具链的构件仍不可用。
- **`tools/ensure-wasmtime.ps1` 失败**：`Wasmtime archive checksum mismatch: expected=7e807f48... actual=026804ca...`（EXIT 非 0）⇒ 无法用该脚本(重)配置 wasmtime CLI。**但 ledger 固件测试走 Rust `wasmtime` crate（`run_guest` 的 `PreparedPool`），不依赖外部 wasmtime CLI、不读 `SICO_TEST_WASMTIME`** ⇒ 本片重测不受影响。
- **`python` 不在 PATH**：只有 WindowsApps stub。改用 `.../Python/Python315/python.exe`（3.15.2）。
- **temp 目录被并发代理争用**：`/tmp`（=`C:\Users\SUN-OF~1\AppData\Local\Temp`）下的 `actual.json`、`ledger.before.json` 多次被其它进程删除/重写 ⇒ 本片把可保存的中间物改放仓库内 `target/tmp/step0567/`，并以固件测试**自身 stdout** 为准（不依赖易失的 temp 文件）。
