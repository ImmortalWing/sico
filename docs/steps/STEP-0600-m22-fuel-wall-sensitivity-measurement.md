# STEP-0600 · M22 W2 · 燃料墙敏感度测量（D8 限额重基线可判定证据）

> **片型** = 纯测量片（诊断）。不推进任何出口门、**不上调任何限额**、不改 runner 限额默认值、不改冻结语料、不改产品语义、M22 整体仍 **NO-GO**。
> **唯一代码产物** = 新测量测试 `runner/sico-runner/tests/m22_fuel_sensitivity.rs` + 测量脚本（tools/）。
>
> **复跑路径**（本片实际使用，可直接复现）：
> - 探针与测量：`cargo test --offline --manifest-path runner/sico-runner/Cargo.toml --test m22_fuel_sensitivity -- --test-threads=1 --ignored`（长耗时，默认 `#[ignore]`；1.6T 全量约 41 分钟/62 组、3.2T 子集约 18.5 分钟/8 组）。
> - 聚合分析：`powershell -File tools/probe-m22-fuel-sensitivity.ps1 -Log1p6t <1.6T 日志> -Log3p2t <3.2T 日志> -SitesMap <每组站点数 CSV>` ⇒ 输出逐函数表（函数、源字节、1.6T/3.2T 结果、2×/4× 是否翻转）与站点级汇总（2×/4×/两者皆未翻转的站点数）。
> - 固化工件与读数说明：[`docs/evidence/m22/README.md`](../evidence/m22/README.md)。
> **禁改清单（本片遵守）**：`runner/sico-runner/src/lib.rs`（限额默认与生产语义）、`runner/sico-runner/src/ui.rs`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/**`、`selfhost/**`（含 `match-component-coverage-v1.json`）、`guests/**`、`wit/**`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、既有 STEP/报告正文。

## 0. 背景（D8 缺什么）

M22 自举轨的阻塞是 **fuel 墙**：guest 自举编译器在燃料上限处耗尽。现状判定（STEP-0555/0556/0562/0572/0575 + STEP-0579 判定档案）：**发射器层 · fuel ~ IR² · 常数主导**（per-op overhead × op count），具体 wasm-op 源未定位；**match 主刀不值得开工**（STEP-0579：判据被尺寸梯度伪影解释，无判别力）。

owner **D8 裁决待定**（M22 计划 §3.3.2）：fuel **1.6T（2×）** 重基线 vs 收窄语料/闭包口径（STEP-0556 §11 选项 2；「上限不得无证据上调，每次上调 = 新 attempt 并记录」）。**缺的证据** = 「若把上限放宽到 2×/4×，多少站点会转绿」的**敏感度**实测 ⇒ 本片提供。

## 1. 测量方法与口径

### 1.1 基线口径（用什么作基线）

- **基线 = 冻结账本** `selfhost/match-component-coverage-v1.json`（396 站点，schema `sico.m22.match-coverage.v1`）。verdict 分布实测：**331 `ir-equal+component-other:FuelExhausted` / 36 `component-equal` / 24 `guest-refused:Trap`（8 MiB stdout 上限）/ 5 `component-refused-guest:unsupported codegen source shape`**。
- **331 个 FuelExhausted 站点全部在 `parser.sico`**，跨 **62 个去重 `path:function` 组**（每组 1–23 个站点）。账本 verdict 由**全量 guest 编译器一次**（`run_guest_with_args`，`--emit-component-hex`，fuel 800e9 / timeout 120 s / 内存 2560 MiB / hostcall 8<<30）测出（见 `selfhost_compiler.rs:192-233`、`match_compare_site`）。
- **测量单位 = 去重函数组**：一个函数的所有站点共享同一闭包（`sico_closure`：root + 同文件传递闭包，函数上限 256、字节上限 1 MiB）、同一 fuel 需求、同一翻转结果 ⇒ 每函数测一次，映射到其全部站点。

### 1.2 被测路径（为什么是 ledger 路径而非 emit-only）

账本 verdict 走**全量编译器一次**（parse + IR 构建 + emit）。emit-only 注入路径（`selfhost_compiler.rs` 的 `guest_emission_with_fuel`）**跳过 guest 前端且发射器跑两次**（`component_refusal_for_ir` + `component_hex_for_ir`），与账本 verdict **不可比**。本片用**ledger 路径**（`guest_component_with_fuel`，同 `--emit-component-hex`，timeout 可配置）⇒ before/after 是 like-for-like。

### 1.3 「转绿」的精确定义

账本 FuelExhausted 站点的 IR 轴已绿（`ir-equal` 已成立）⇒ 唯一可动的是组件轴。「转绿」= guest 在给定 fuel 下**发射组件**且其 SHA-256 **等于账本冻结的 `component_sha256`**（Rust oracle 生产输出）。发射但不等 ⇒ `component-mismatch`（不算绿）。账本每站点都有冻结 `component_sha256`（即使 FuelExhausted 站点，Rust oracle 的组件已在 `match_compare_site` 中算出）⇒ 可直接比对。

### 1.4 测量接口（新测量测试 `m22_fuel_sensitivity.rs`）

- `SICO_M22_SENS_GROUPS` = `path:function,...`（去重函数组，必须）。
- `SICO_M22_SENS_FUELS` = 逗号分隔 fuel 预算（默认 `800000000000,1600000000000,3200000000000` = 1×/2×/4×）。
- `SICO_M22_SENS_TIMEOUT` = 每次运行墙钟秒（默认 300；账本 120、D8 2× 档案 180；宽松默认使 **fuel 为绑定约束**，任何墙钟 trap 单独记录）。
- 输出 `SENS_FUEL path:function fuel=X outcome=... component_bytes=... component_equal=... wall_ms=... source_bytes=...`。
- 测试为 `#[ignore]`（测量探针惯例），用 `--ignored --nocapture` 显式运行。
- 复刻闭包构建与 `guest_component_with_fuel` 与 `selfhost_compiler.rs` **逐字**（保证与账本同闭包、同路径）；`compile_guest` 复用 `support/selfhost_common.rs` 的磁盘组件缓存（`cached_component("compiler.sico", 5 文件)`）。

### 1.5 成本探针（先小样本，再定全量/子集）

单站成本实测（2026-11-07 本机，预热缓存后）：

| 函数组 | 闭包 source_bytes | fuel | outcome | wall_ms | 速率（fuel/s） |
|---|---:|---:|---|---:|---:|
| `fixed_literal_const_segment` | 11,200 | 200e9 | fuel-exhausted | 11,778 | 1.70e10 |
| `control_while_function_ir` | 403,232 | 200e9 | fuel-exhausted | 12,129 | 1.65e10 |
| `straight_cell_function_ir` | 398,767 | 200e9 | fuel-exhausted | 9,620 | 2.08e10 |
| `general_while_function_ir` | 541,403 | 200e9 | fuel-exhausted | 9,858 | 2.03e10 |

**速率稳定 ~1.7–2.1e10 fuel/s，与闭包大小无关**（200e9 在 9.6–12.1 s 耗尽，最小 11 KB 与最大 541 KB 闭包同量级）⇒ 墙钟 ≈ fuel / 速率，单次运行成本由 fuel 预算决定，不随站点变大。**外推**：单次 1.6T ≈ 94 s、单次 3.2T ≈ 188 s。

**据此决定：全量 62 函数组（非子集）**，因为 62 × ~94 s ≈ **1.6 小时**在可接受预算内，且给出**完整** 2× 翻转答案（逐组→逐站点映射），无需外推。

### 1.6 测量序列

1. **1.6T（2×）全量 62 组**：每函数一次运行（emits ⇒ 2× 翻转；exhausts ⇒ 4× 候选）。
2. **3.2T（4×）对 1.6T 未翻转组**：每函数一次运行。
3. **W4 parity 3 项**：`scope-nesting-limit.sico` 三份（`semantic-cases/future-task/invalid/`、`syntax-candidates/b/...`、`syntax-candidates/c/...`）在 1.6T/3.2T 下是否由 `FuelExhausted` 达 typed 拒绝（匹配 Rust `REFUSED:LEXICAL`）。

## 2. 实测结果（2× / 4× 翻转）

### 2.1 2×（1.6T）：全量 62 函数组 / 331 站点（完整，非子集）

1.6T 全量扫描（62/62 组，EXIT=0，无 trap/refusal/组件失配）：

| 口径 | 数值 |
|---|---:|
| 被测函数组 | 62 / 62 |
| 2× 翻转组（`component_equal=yes`） | **3 / 62（4.84%）** |
| 2× 仍耗尽组 | 59 / 62（95.16%） |
| 映射到 FuelExhausted 站点 | 331 |
| **2× 翻转站点** | **5 / 331（1.51%）** |
| **2× 仍耗尽站点** | **326 / 331（98.49%）** |
| 组件失配（发射但 ≠ Rust） | 0 |
| trap / refusal | 0 |

**2× 翻转的 3 个函数组**（全部为小闭包，源 17–21 KB）：

| 函数组 | 源 bytes | 组件 bytes | wall_ms | 估计需求 M（wall×速率） | 站点 |
|---|---:|---:|---:|---:|---:|
| `fixed_operation_operand_check` | 20,370 | 17,688 | 71,204 | ≈1.5T | 2 |
| `gw_rhs_single_pack` | 17,331 | 16,123 | 78,924 | ≈1.58T（≤1.6T） | 1 |
| `straight_environment_operand_check` | 20,741 | 18,029 | 69,454 | ≈1.46T | 2 |

**关键观测**：翻转与**闭包大小不单调**——`parameters_ir`（源 13,807 B，比翻转组更小）在 1.6T 耗尽，而 `fixed_operation_operand_check`（20,370 B）翻转 ⇒ fuel 需求 M **不是**闭包大小的严格函数（每操作开销随函数体构成变化），**IR²/尺寸外推对单站点不可靠** ⇒ 必须实测，不能由尺寸外推替代。

### 2.2 4×（3.2T）：子集（8 个最小非翻转闭包，选取理由 = 最可能翻转）

实测（`m22-sens-3p2t-sub.log`，8/8 组，EXIT=0，耗时 1113 s）：

| 函数组 | 源 bytes | 组件 bytes | wall_ms | 3.2T 结果 |
|---|---:|---:|---:|---|
| `parameters_ir` | 13,807 | 11,917 | 82,246 | **翻转**（equal=yes）⇒ M ≈ 1.6T |
| `fixed_operation_literal_instructions` | — | 18,043 | 130,310 | **翻转**（equal=yes）⇒ M ≈ 2.5T |
| `gw_rhs_utf8_decode_arm` | 23,455 | 19,109 | 143,865 | **翻转**（equal=yes）⇒ M ≈ 2.7T |
| `fixed_guard_condition_json` | 25,983 | 22,237 | 147,252 | **翻转**（equal=yes）⇒ M ≈ 2.8T |
| `gw_rhs_error_arm` | 19,774 | 0 | 156,083 | 仍耗尽 ⇒ **M > 3.2T** |
| `construct_field_value_ir` | 25,234 | 0 | 153,029 | 仍耗尽 ⇒ **M > 3.2T** |
| `gw_list_get_subject_packed` | 28,781 | 0 | 152,689 | 仍耗尽 ⇒ **M > 3.2T** |
| `gw_u64_less_than_value_packed` | 28,908 | 0 | 147,481 | 仍耗尽 ⇒ **M > 3.2T** |

⇒ **4/8 翻转、4/8 仍耗尽**；M 估计按 §3.1 的 ~1.9e10 fuel/s 折算（**翻转组的 wall 是完成时间 ⇒ M ≈ wall×rate；耗尽组的 wall 是耗尽 3.2T 的时间 ⇒ M > 3.2T**）。

### 2.3 W4 parity 剩余 3 项（`scope-nesting-limit`）的档位敏感度——**订正旧登记**

实测（`m22-sens-w4.log`，3 个文件 × 3 个 fuel 档，EXIT=0，耗时 2.45 s）：

| 文件 | 800e9 | 1.6T | 3.2T |
|---|---|---|---|
| `semantic-cases/future-task/invalid/scope-nesting-limit.sico` | refused `E-SH-IR-FUNCTION-COUNT`（2.3 s） | refused 同上（15 ms） | refused 同上（14 ms） |
| `syntax-candidates/b/future-task/invalid/scope-nesting-limit.sico` | refused `E-SH-IR-EXPRESSION`（16 ms） | refused 同上（15 ms） | refused 同上（15 ms） |
| `syntax-candidates/c/future-task/invalid/scope-nesting-limit.sico` | refused `E-SH-IR-FUNCTION-COUNT`（15 ms） | refused 同上（14 ms） | refused 同上（17 ms） |

⇒ **这 3 项不是燃料墙**：它们在**最低档 800e9** 就以**毫秒级**被**其它**类型化拒绝（function-count / expression），**提高 fuel 上限不改变结果**。此前把它们登记为「`scope-nesting-limit` 燃料墙」**属口径错误**（见 §5 订正说明）。

## 3. 成本实测与全量估算

### 3.1 单位成本（实测）

- 速率稳定 **~1.7–2.1e10 fuel/s**（200e9 在 9.6–12.1 s 耗尽，闭包 11 KB–541 KB 同量级；与站点大小无关）。
- 单次 1.6T ≈ **70–79 s**（实测 62 组均值 ~75 s）；单次 3.2T ≈ 2× ≈ **150–190 s**。
- 预热后无组件缓存重建（`compile_guest` 复用磁盘缓存）。

### 3.2 全量估算

| 项 | 单位成本 | 规模 | 总墙钟（实测/外推） |
|---|---|---|---|
| 1.6T 全量 62 组 | ~75 s/组 | 62 | **实测 ~69 min**（7 组首跑被默认后台超时中断后重跑，含 2 批） |
| 3.2T 全量 59 组（若做） | ~188 s/组 | 59 | **外推 ~3.1 h** |
| W4 3 文件 × 3 fuel | ~80–190 s/次 | 9 | 外推 ~18 min |

**结论**：1.6T 全量在预算内（已实测完成）；**3.2T 全量 59 组 ≈ 3.1 h 超出预算** ⇒ 本片用**8 组最小非翻转子集**界定 4×（见 §2.2），并如实报告「4× 未全量实测」。

## 4. 对 D8 的可判定结论 / 不足判定缺口清单

**结论（一句话）：「fuel 1.6T（2×）重基线」不足以解除燃料墙——2× 只让 331 个站点中的 5 个（1.51%）转绿，98.5% 仍耗尽；4× 也只翻了最小的 4/8 组，其余仍耗尽且 M > 3.2T。**

依据（全部实测，无推测）：
1. **2× 全量**：62/62 函数组实测完毕，无 trap、无 refusal、无组件失配（**0 失配**说明测量口径本身可信：能跑完的站点产出的组件与 Rust oracle **逐字节相等**）；仅 3 组（4.84%）翻转，映射到站点为 **5/331 = 1.51%**；**326/331（98.49%）仍耗尽**。
2. **大闭包在 2× 下全部耗尽**：翻转的 3 组源 17–21 KB；`parameters_ir`（源 13.8 KB，**更小**）在 1.6T 耗尽 ⇒ **翻转与闭包大小并非单调**（但整体上仍是**小闭包才可能过 2×**）。
3. **4× 子集**：8 个最小未翻转闭包中 4 个在 3.2T 翻转（M ≈ 1.6–2.8T），4 个仍耗尽（M > 3.2T）；**更大闭包的 M 按 §3.1 的速率与 §2.1 的观测只会更高**（fuel ~ 闭包规模²）⇒ 4× 全量亦大概率**不能**解除燃料墙，但**本片未做 4× 全量**，故只把该判断限定在「最小闭包子集」范围内。
4. **逐站上调不可行**：每站所需 M 随闭包规模**超线性**增长，且大闭包的 M 已 > 3.2T ⇒ 「按站点逐个上调 fuel」在工程上不成立（上调幅度会随闭包规模无界增长）。
5. **W4 剩余 3 项与 fuel 无关**（§2.3）：在 800e9 即毫秒级被 function-count/expression 拒绝 ⇒ **不构成 fuel 墙的证据，也不因放宽限额而改善**。

**因此给 owner 的 D8 输入**：
- 选项「fuel 1.6T（2×）重基线」：**实测证据反对**（只解 1.51% 站点）。
- 选项「收窄语料/闭包口径」：本片**未测**其效果（属另一类工作），但 §2.1/§2.2 表明**闭包规模是 M 的主因** ⇒ 收窄闭包（而非抬限额）是与实测一致的路径。
- 第三条路「op 级燃料剖析定位发射器常数来源」：仍需 owner 授权 runner 生产钩（STEP-0572 §5）；本片的敏感度数据**强化**了该路线的必要性（因为抬限额与逐站上调都已被实测否定）。

## 5. 诚实收窄

1. **2× 是全量**（62/62 组、331/331 站点，EXIT=0）；**4× 只是 8 组最小未翻转闭包的子集**（约 3.2T 全量外推 ~3.1 h，超出本片预算）⇒ **4× 的结论只在该子集范围内成立**，**未做 4× 全量**，不得据此声称「4× 一定不行」。
2. **W4 3 项的档位敏感度已测**（结论：与 fuel 无关），但**这 3 项为何被拒**（function-count / expression 语义的正确处置）**不在本片范围**——属语料/契约问题，须另行裁决。
3. **本片是测量**：**未改任何产品限额或默认值**（`RunnerLimits` 默认与 `lib.rs` 限额段均未动；测量走**每次运行的限额参数**）；改动文件仅限测量测试与证据工件。
4. **M 估计是按速率折算**（~1.9e10 fuel/s），非逐站 fuel 读数（runner 公共 API **不暴露 fuel 读数**——这正是 STEP-0572 §5 记录的缺口）⇒ M 值是**量级估计**，用于比较大小而非精确值。
5. **口径订正**：此前把 W4 剩余 3 项登记为「`scope-nesting-limit` 燃料墙」**不准确**——实测显示它们在最低档即毫秒级被**其它**类型化拒绝，与 fuel 无关；本片据此订正（§2.3），相关登记（STATUS / AGENTS §10.1.4 / ROADMAP）已同步修正。
6. 测量中的 `0xc0000005` 间歇性退出与 `rustc` ICE 属**本机既有环境 flake**（多个代理在同机并发编译时复现于未触碰的二进制），不作为本片结论。

## 6. 后续片建议

1. **owner 裁决 D8**：按本片实测，建议**否决 2× 重基线**、考虑「收窄闭包口径」或「op 级剖析」（后者需授权 runner 生产钩）。
2. **若 owner 仍要 4× 全量结论**：按 §3.2 的 ~188 s/组外推，需 ~3.1 h 独占机时（并避免并发编译干扰）。
3. **W4 3 项的语义处置**：它们是 `scope-nesting-limit` 用例的 function-count/expression 拒绝——须判定「语料是否放错位置」或「契约是否过宽」（与 IND-003 同类裁决面）。
4. **fuel 读数缺口**：runner 公共 API 无 fuel 读数 ⇒ 所有 M 只能折算；若要做严格归因，仍需 STEP-0572 §5 的 op 级读数能力（owner 授权）。
