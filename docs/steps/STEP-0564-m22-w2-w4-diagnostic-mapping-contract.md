# STEP-0564 m22-w2 **W4 诊断 ID 映射契约（guest E-SH-IR-\* ↔ Rust LEXICAL/E-code）**

| 项 | 值 |
|---|---|
| 片型 | **工具 + 工件片**：扩展 `tools/probe-m22-corpus-parity.ps1`（本片唯一写者），逐条输出 `(guest verdict+code, rust verdict+code, mapped_id, span-axis)`；新建版本化映射工件 `selfhost/corpus-parity-map-v1.json`（新 schema `sico.m22.corpus-parity-map.v1`，字段见 §3.1） |
| 入口门槛 | STEP-0561 已交付 150 项 verdict parity（147/150）+ 冻结工件 `selfhost/corpus-parity-v0.json`；guest 44 个 `E-SH-IR-*` 码与 Rust `LEXICAL`/`E-code` 命名空间不相交，精确 `(id, span)` parity 未达 ⇒ 本片建立映射契约 |
| exit | **实测 150 项：exact 147 / structural 0 / zero 3**（3 项均为 `scope-nesting-limit.sico` 燃料墙，`LOWERED`=0）；映射表 **33 个等价类**（32 refined + 1 coarse）覆盖全部 147 个 guest 拒绝项（`unmapped_rust_ids=[]`）；探针 exit 1（`zero>0`，符合约定）；**span 轴 guest 侧结构性阻塞，未达** |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 维持 —— **无变化**；M22 保持 **NO-GO**，不做完成声明 |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 构建，`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB） |

## 1. 要回答的问题

W4 要求 178 条拒绝三元组 `(verdict, id, span)` 一致。STEP-0561 出了 verdict 半边（147/150），但 id/span 精确 parity 因**命名空间不相交**未达。本片回答：

> 能否建立一个**有证据支撑**的 `E-SH-IR-* ↔ LEXICAL/E-code` 映射契约，使 `(verdict, mapped-id)` 轴可精确出账？span 轴现状如何？

## 2. 方法前提（先核实映射可行域，全部实测）

1. **Rust 有 span，冻结语料无 span。** `sico check --json` 发射完整 `code`/`key`/`range.line:col:byte`；但 `selfhost/corpus-v0.json` 的 `diagnostic_ids` 由 `tools/update-m22-corpus.ps1:192-204` 对**纯文本** check 跑 `\bE\d{4}\b`、取不到时回退 `LEXICAL`/`SYNTAX`/`UNCLASSIFIED`，**不含 span**。
2. **guest 无 span。** guest 拒绝经 runner 返回 `{"class":"domain-error","detail":"...ERR:E-SH-IR-*"}`，裸码无位置。⇒ span 比较缺一侧输入，**结构性不可达**。
3. **码对可观测。** 冻结 parity 工件的 `(rust_diagnostic_ids, guest_verdict)` 配对得 **33 个唯一 `(rust_code, guest_code)` 对**：32 refined（具体 E-code ↔ 具体 `E-SH-IR-*`）+ 1 coarse（`LEXICAL ↔ E-SH-IR-FUNCTION-COUNT`，98 项）。

## 3. 交付物

### 3.1 映射工件 `selfhost/corpus-parity-map-v1.json`

- schema `sico.m22.corpus-parity-map.v1`、`version:1`；映射字段 `rust_code`/`guest_code`/`grain(refined|coarse)`/`equivalence_class`/`evidence{corpus_count, sample_paths, source}`。
- **只登记有冻结语料证据的等价类**（`corpus_count ≥ 1`），宁缺毋滥；`.span_axis` 显式写 `blocked-guest-side` 与原因，**不虚报 span parity**。
- 探针加载时严格校验（schema 名、version==1、非空、grain 枚举、无重复对、无空码），畸形 typed 拒绝 exit 1，禁止静默近似。

### 3.2 探针 `tools/probe-m22-corpus-parity.ps1`

- parity 三级：`exact`（拒绝且映射命中）/ `structural`（拒绝但未登记）/ `zero`（`LOWERED` 声音洞 或 `FAILED` 燃料墙）。
- 报告仍写 `selfhost/corpus-parity-v0.json`（保持 schema `sico.m22.corpus-parity.v0` 供电池消费），新增 `mapped_by`/`exact_parity_count`/`structural_parity_count`/`zero_parity_count`/`mapped_id_coverage`/`span_axis`/`unmapped_rust_ids`。
- `-RustSpans`：额外采集 Rust 侧 span 作**佐证**（不比较，guest 无 span）。

## 4. 实测证据

| 命令 | 结果 |
|---|---|
| `powershell -NoProfile -File tools/probe-m22-corpus-parity.ps1 -Limit 6` | **实测** exit 0，`checked=6 exact=6 structural=0 zero=0`；映射加载校验通过 |
| `powershell ... -Limit 2 -RustSpans -Json` | **实测** exit 0，采到 live `E1013`/`SYNTAX_UNEXPECTED_TOP_LEVEL`/`5:1:102` ⇒ 证实 coarse 类是粗分类塌缩，非语义等同 |
| `powershell -NoProfile -File tools/probe-m22-corpus-parity.ps1`（全量 150） | **实测**（2026-11-06 本机，约 44 min wall）：**exit 1**（`zero=3>0`，符合约定）；回写工件字段 `checked=150 exact=147 structural=0 zero=3`、`mapped_id_coverage=33(refined=32,coarse=1)`、`unmapped_rust_ids=[]` |

**全量运行实测（2026-11-06 本机，约 44 min wall，`/tmp/parity-full.log` + 回写工件 `selfhost/corpus-parity-v0.json`）：**

- 退出码：`PROBE_EXIT=1`（**符合预期**：`zero=3 > 0` ⇒ exit 1，见探针 exit 约定）
- 汇总（回写工件字段）：`checked_count=150`、`exact_parity_count=147`、`structural_parity_count=0`、`zero_parity_count=3`
- `mapped_id_coverage`：`registered_classes=33 (refined=32 coarse=1)`；`unmapped_rust_ids=[]`（147 个 guest 拒绝项全部命中登记映射类）
- `zero` 的 3 项 = `scope-nesting-limit.sico` ×3（`future-task`、`syntax-candidates/b`、`syntax-candidates/c`），guest 均 `FAILED`（`resource-limit`/FuelExhausted），**原样登记，未上调限额**
- `span_axis`：`not-compared`（guest 无 span）

> 注（证据卫生）：全量运行期间一次并发的 `-Limit 2 -RustSpans` 验证曾把报告工件覆盖成 2 项版本；已 `git checkout -- selfhost/corpus-parity-v0.json` 复原，全量运行结束时以 150 项版本最终覆写，故工件最终态即上列数字。全量运行的逐条 stdout 行因 PowerShell 重定向缓冲未落 `/tmp/parity-full.log`（文件 0 字节），**权威记录为回写工件 `selfhost/corpus-parity-v0.json` 的字段**（`exact/structural/zero/unmapped_rust_ids`），已如实登记。

### 4.1 精确 `(verdict, mapped-id)` parity（实测）

- **exact = 147/150**：guest 拒绝且 `(rust_id → guest_code)` 映射命中。
- **structural = 0**：无「guest 拒绝但未登记映射」项（`unmapped_rust_ids=[]`）⇒ 映射表覆盖了全部 147 个 guest 拒绝项。
- **zero = 3**：全部为燃料墙（`FAILED`），**无 `LOWERED`**。

### 4.1.1 与 STEP-0561 叙述的偏差（如实登记）

STEP-0561 §3 报「146 match + 1 `LOWERED`（`missing-field`/`E2010` 声音洞）+ 3 燃料墙」。本片全量实测为 **147 exact + 3 燃料墙、`LOWERED`=0**：`syntax-candidates/b/nominal-invariants/invalid/missing-field.sico`（rust `E2010`）在本片运行中 guest 判为 `refused:ERR:E-SH-IR-CONSTRUCT-FIELD = exact`，**不是** `LOWERED` 声音洞。本片**不裁决**孰为权威（两者都是本机 debug 运行，可能来自不同 guest 快照或分类时点），**只如实登记本片实测口径**；该偏差须由评审者在 STEP-0561 与本片之间对齐，本片不改写 STEP-0561。因此「前端 check 轴存在 `missing-field` 声音洞」这一 STEP-0561 结论**在本片实测中未被复现**，不得据本片断言其存在或不存在。

## 5. 诚实收窄

- **span parity 未达**（guest 结构性无 span；冻结 Rust 基线亦无 span）⇒ 真 `(verdict, id, span)` 三元 parity 当前不可达，**不**宣称。
- **coarse 类不读作语义等同**：`LEXICAL ↔ E-SH-IR-FUNCTION-COUNT` 只是前端 LEXICAL 桶塌缩到 guest 粗粒度 IR 拒绝，`-RustSpans` 佐证其真实身份是 `E1013` 顶层语法错。
- **3 个燃料墙**（`scope-nesting-limit.sico` ×3：2×`LEXICAL` + 1×`E5105`）guest `FAILED`/FuelExhausted，无 refusal 码 ⇒ 无法映射，**原样登记，不上调限额**（本片 `zero=3` 全部来自此处）。
- **`LOWERED`=0（本片实测）**：本片全量运行**未复现** STEP-0561 所述的 `missing-field`/`E2010` `LOWERED` 声音洞（该条在本片判为 `exact`）。本片不据此断言声音洞存在或不存在，偏差交评审者对齐（见 §4.1.1）。
- **未跑项**：未跑 runner `selfhost_compiler` 测试（本片不改 guest 源码与 runner 测试，非本片范围）；未跑 `validate-m22-s5.ps1` 整轮电池（约 2.7 h，本片为工具/工件片，不改编译产物）。环境限制：本机无 `pwsh`（用 Windows PowerShell 5.1）、无 `python`（用 `node` 做工件分析）。

## 6. 结论

本片交付 W4 的 `(verdict, mapped-id)` 映射契约（33 个有证据等价类）与扩展探针，使 id 轴可精确出账；span 轴如实登记为 guest 侧结构性残差。**M22 仍 NO-GO**，不因本片改变；3 燃料墙 + 1 声音洞保留为缺口。
