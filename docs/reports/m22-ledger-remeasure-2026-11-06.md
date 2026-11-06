# M22 站点账本与发射词表重测报告（STEP-0567 · 2026-11-06）

**片性质**：M22 W2 收尾蜂群测量/复核片。**零 oracle 生产代码改动、零 `selfhost/*.sico` 改动、零 gate 文本改动**；只动两份冻结工件（`selfhost/match-component-coverage-v1.json`、`selfhost/emitter-vocabulary-v1.json`——本片是它们与两个 census 工具的唯一写者）+ 文档。
**HEAD** = `a939862`（分支 `codex/m22-w2`），开工工作树干净。
**裁决一句话**：STEP-0560 的「ledger 行号已重推导」与「emitter 词表 json 已刷新」两项声称**实测成立**；但 STEP-0560 **没有闭合任何站点**——**7 个 `general_while` 站点在 HEAD 仍是 `FuelExhausted`**，组件轴仍 **36/396**；另暴露 **4 个燃料边界站点（MS-081..084）** 需 HEAD-fresh 固件确证。**M22 保持 NO-GO。**

---

## 1. 环境与可跑性（先廉价探测，如实登记）

| 探测 | 结果 | 影响 |
|---|---|---|
| `python` / `python3` | 不在 PATH（仅 WindowsApps stub） | 改用 `C:/Users/sun-office/AppData/Local/Programs/Python/Python315/python.exe`（3.15.2） |
| `powershell.exe -NoProfile -Command "echo psok"` | `psok` | PowerShell 可用 |
| `which gcc` / `which cc` | **均空（本机无 gcc/mingw）** | 见下（runner 测试仍可跑，clippy 不可用） |
| `tools/ensure-wasmtime.ps1` | **失败**：`Wasmtime archive checksum mismatch: expected=7e807f48… actual=026804ca…` | 无法(重)配 wasmtime CLI；**但 ledger 固件走 Rust `wasmtime` crate，不读 `SICO_TEST_WASMTIME`** ⇒ 重测不受影响 |
| `cargo test … --test selfhost_compiler --no-run` | 首次 **link `Permission denied`**（运行中旧进程锁住 exe）；杀进程后 **BUILD_EXIT=0（18:09）** | 用 `rust-lld`（**不需 gcc**）⇒ 拿到 HEAD-fresh 二进制 `selfhost_compiler-4fa1b02d73be6796.exe` |
| `/tmp` 中间文件 | 被并发代理反复删除/重写（`actual.json`、`ledger.before.json` 多次消失） | 可保存中间物改放 `target/tmp/step0567/`；以固件**自身 stdout** 为准 |

## 2. 任务 1：match-sites ledger census

```
$ python tools/census-m22-match-sites.py --check
LEDGER_CURRENT sites=396        EXIT=0
```

`--check` 从 `selfhost/*.sico` **重新枚举** 396 个 `U64.checked_add` match 站点、重算 `line`/`function_line`/`parameter_count`/`head`/`case_shape`，与磁盘 ledger **全文比对**。**EXIT=0 ⇒ STEP-0560 对 `codegen.sico`(+97)/`parser.sico`(+92) 改动后的行号重推导已落地**（STATUS/handoff 该条声称**成立**）。

**`--update` 的诚实登记（结果未采纳）**：运行时 temp 有一份**当日 17:27/17:32 由并发进程写入的 `actual.json`**（provenance 不可证）。它把 4 个站点从 `component-equal` 翻成 `FuelExhausted`：

| id | function（parser.sico） | 参数 | 行号（未变） | HEAD | actual.json |
|---|---|---|---|---|---|
| MS-081 | `fixed_literal_const_segment` | 7 | 4577 | component-equal | FuelExhausted |
| MS-082 | `fixed_literal_const_segment` | 7 | 4590 | component-equal | FuelExhausted |
| MS-083 | `fixed_literal_const_segment` | 7 | 4622 | component-equal | FuelExhausted |
| MS-084 | `fixed_literal_const_segment` | 7 | 4629 | component-equal | FuelExhausted |

行号**一个没动**（与「行号已重推导」一致），动的只是 verdict。`--update` 的 merge 逻辑 `if actual is not None and args.update` 会**无条件**用 `actual.json` 覆盖 verdict（不只 pending）⇒ 工具已知行为边界。**本片未据此改冻结账本**（`git checkout --` 还原 HEAD），改由 §3 的 HEAD-fresh 固件重测裁决。

## 3. 任务 3：组件轴逐站差分（HEAD-fresh 固件重测）

**为什么必须重测**：STEP-0560 自己声明「组件轴逐站差分未跑」。直接跑 `tools/probe-m22-match-coverage.ps1` 会走 `cargo test` 重编（慢 + 本机无 gcc 风险），故本片用**已构建的 HEAD-fresh 二进制**直接跑单条 ledger 固件测试（等价于该探针的第 2 步），并设 `SICO_SELFHOST_COMPONENT_CACHE=0` 与探针同口径。

**关键澄清（防误判）**：commit `6440890`（16:56）改了 `selfhost_compiler.rs`(+45) 与 `parser.sico`(+92)，而磁盘上 `selfhost_compiler-4fa1b02d73be6796.exe` 一直是 16:02 的旧版（link 一直 `Permission denied`）。**旧二进制 + 新 `parser.sico` 源（运行时读取）+ 旧测试代码** 的组合**不能**作为 HEAD 裁决依据。本片先杀进程解锁、重建出 **18:09 的 HEAD-fresh 二进制**，再用它跑。

**已确证（HEAD-fresh 固件，实测）——HEAD 账本在 MS-081..084 上偏乐观**：

```
fixed_literal_const_segment（parser.sico，MS-081..084 同一函数组）
  = ir-equal+component-other:FuelExhausted     ← HEAD-fresh 固件实测
  ≠ ir-equal+component-equal                    ← HEAD 冻结账本所记
```

⇒ **HEAD 账本把这 4 个站点记成 `component-equal` 是错的，它们在 HEAD 实为 `FuelExhausted`** ⇒ 「组件轴 36/396」**证伪**，真实 component-equal **≤ 32/396**。§2 里那 4 个翻转**是正确的**（本片当初还原到 HEAD 只因 provenance 不可证，现由 HEAD-fresh 固件独立确证）。

**重测部分直方图（HEAD-fresh，前 42 个函数组，非全量）**：component-equal **25**、FuelExhausted **16**、refused-guest **1**。

**未跑完（诚实登记）**：全量 165 组 / 396 站点完整直方图、以及 **7 个 `general_while` 站点（MS-318..324）在 HEAD-fresh 下的 verdict**，**本片未跑完**。原因 = 本机 CPU 被并发蜂群代理争用，速率仅 ~0.8 组/分钟（25 分钟推进 42/165），跑到 `general_while`（账本序靠尾）需 ~100+ 分钟，超时间预算。**这是被环境阻塞的「未跑」，不是「已测无移动」；后台重测仍在进行，本片不臆造其结果。**

## 4. 任务 2：emitter-vocabulary census（RED 归因）

```
$ python tools/census-m22-emitter-vocabulary.py --check
EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S   EXIT=1
```

**逐层排除归因（读工件与冻结源，不推测）**：

- `guest_emitter` 段：`source_sha256 changed? False`、ops 16→16、intrinsics/helpers 增删皆空 ⇒ **STEP-0560 的 `sico.list.length[U64]` 发射词表已正确刷进工件**（STEP-0560 §2.4 声称成立）。
- `backend_helper_rule` 段（`crates/sico-codegen-wasm/src/lib.rs` 指纹 + 13 臂）不变。
- 唯一漂移 = **站点成员集**：冻结工件 **336** 站点 ⇒（若采纳那 4 个翻转）**340** 站点，新增者**恰为 MS-081..084**（跨过 census 的 `ir-equal 且非 component-equal` 纳入线）。

⇒ **RED 唯一根因 = §2 那 4 个 verdict 翻转的下游效应，不是 STEP-0560 的发射词表缺口。** §3 已用 HEAD-fresh 固件确证这 4 个翻转正确（`fixed_literal_const_segment` 实测 `FuelExhausted`）。**处置**：`emitter-vocabulary-v1.json` 站点集须跟随**完整** HEAD-fresh 账本（340 站点口径 + 其余组可能的移动），而全量重测**未跑完**（§3）⇒ 本片**不在 RED 下用不完整账本 `--update`**；工件**保持冻结原样（336 站点口径）**，登记为「待全量重测完成后按 340 站点口径 `--update` 并复验 `--check`」。**有意的诚实收窄。**

## 5. 任务 4：396 站点四分类精确计数（HEAD 账本口径）

| verdict | 站点数 |
|---|---|
| `ir-equal+component-other:FuelExhausted` | **331** |
| `ir-equal+component-equal` | **36** |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | **24** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | **5** |
| **合计** | **396** |

组件轴逐字节全等 = **36/396**（与 STATUS/handoff 一致）；未闭合 = **360**（燃料墙 331 + 8MiB trap 24 + refused-guest 5）。

## 6. 结论与残余风险

**结论**：

- **STEP-0560 的两项工件声称成立**：ledger 行号重推导（`--check` = `LEDGER_CURRENT sites=396`，EXIT=0）、emitter 词表刷新（`guest_emitter`/`backend_helper_rule` 两段逐项不变）。
- **但 STEP-0560 未闭合任何站点，且 HEAD 账本「组件轴 36/396」偏乐观**：HEAD-fresh 固件确证 MS-081..084（`fixed_literal_const_segment`）实为 `FuelExhausted`（账本记 `component-equal`）⇒ 真实 component-equal **≤ 32/396**。
- **emitter-vocabulary RED 已归因** = 上述 4 条翻转的下游（站点集 336→340），非发射词表缺口；`--update` 待全量重测后执行。
- **全量组件轴直方图与 `general_while`（MS-318..324）裁决未跑完**（CPU 争用），如实登记。
- **M22 NO-GO、七门不变、不翻门、不做完成声明。**

**残余风险 / 未跑项**：

- **MS-081..084 账本修正未落地**：HEAD 账本仍记这 4 条为 `component-equal`（本片未擅自改冻结账本，等全量重测一并 `--update`）⇒ 在此之前 STATUS/handoff 的「36/396」偏乐观，应按「≤32/396」理解。
- **全量组件轴直方图、`general_while` HEAD-fresh verdict 未跑完**：被本机 CPU 争用阻塞（并发蜂群代理）；后台重测仍在进行，本片不臆造其结果。`general_while` 是否真闭合仍未裁决。
- **emitter-vocabulary 工件仍 RED（相对不完整账本）**：待全量重测后按 340 站点口径 `--update` 并复验 `--check`。
- **本机无 gcc** ⇒ runner `cargo clippy`、需 C 工具链的构件**未跑**（非本片引入）。runner 测试**可跑**（`rust-lld` link 成功）。
- **`ensure-wasmtime.ps1` 校验和失败** ⇒ 无法刷新 wasmtime CLI（本片重测不依赖它）。
- **未跑**：`tools/probe-m22-match-coverage.ps1` 的 cargo 全路径、runner 全量 186 测试、6114s 量级固件全量（本片只跑单条 ledger 测试且未跑完）。
- **temp 争用**：并发代理反复清理 `/tmp`；本片以固件 stdout + 仓库内 `target/tmp/step0567/` 为准。
- **未提交未推送。**
