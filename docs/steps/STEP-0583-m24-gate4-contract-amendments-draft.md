# STEP-0583 M24 gate-4 合同修正案草案：hostcall 预算语义 + Script 组件内存下界（PROPOSED）

| 项 | 值 |
|---|---|
| 片型 | **纯合同起草片**（只读核验 + 两份 PROPOSED 修正案草案；**零实现、零行为改动、零自裁**）：`docs/rfc/RFC-0051-amendment-A2-hostcall-budget-and-script-memory-floor.md`（合并 C1/C2 两个决策点的一份修正案文件） |
| 状态 | **PROPOSED — 呈 owner 裁决**。本片不实现、不改行为；任何选项均未「已裁决」；owner 接受后**另开实现 STEP**。gate 4 维持 STEP-0578 现状（hostcall 计数运行期强制与真实 Script 组件内存路径**未达**） |
| 依据 | STEP-0578 §6（实测合同缺口 C1/C2）；只读核验见 §2（每条断言带 `文件:行` 或 STEP 章节，未凭计划原文推断实现状态，AGENTS §1.4；凡与实现现状出入取下界写） |
| 零改动 | `docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、任何源码与测试（当前有 4 个代理在改 `runner/`、`crates/`）、`docs/rfc/` 既有 RFC 正文（只读参考）——本片**只新增下列两个文件** |

## 1. 背景

STEP-0578（M24 gate-4 provenance 预算运行期强制 + 取消接线）完成并实测后，§6 登记了两处**运行期合同空白**，本片按任务书把这两处空白整理成**两份 PROPOSED 修正案草案**（合并为一份 `RFC-0051-amendment-A2` 文件，含 C1/C2 两个决策点），呈 owner 裁决。任务书明确：**不改产品代码、不实现、不自裁**。

- **C1 hostcall 预算运行期不生效 + 单位不匹配**：`max_hostcalls` 是「hostcall **计数**」上限，却被 A1.3 映射为 `hostcall_fuel`（**每次 hostcall 的 fuel 费用**），且 `invoke`/`start_debug` 用 `max(limits.hostcall_fuel, stdin*64 + 64 MiB)` 作地板 ⇒ 低于该地板的预算被覆盖。
- **C2 真实 Script 组件内存下界**：真实 Sico Script 组件声明 `SCRIPT_MEMORY_BYTES` ≈ 2.06 GiB；provenance 预算 `max_memory_bytes`（如 1 MiB）映射为 guest 线性内存上限 ⇒ install 期子集检查通过、运行期实例化 trap（STEP-0497 同源现象）。

## 2. 只读核验（逐条确认/纠正，2026-11-07 worktree 快照）

> 注：另有 4 个代理正在改 `runner/`、`crates/`；下列行号对 2026-11-07 快照成立，实现片开工前须重新核对最新快照。

### 2.1 C1 确认（全部属实，无纠正）

| 断言 | 证据 | 结论 |
|---|---|---|
| manifest 字段声明/文档为「计数」 | `crates/sico-package/src/provenance.rs:66-67`（"Host-call **count** ceiling"）；`docs/rfc/RFC-0051-model-provenance-manifest-v0.md:31-34`（§2 `max_hostcalls: U64`）、`:53`（§3 "hostcall-**count** ... brackets"） | **确认** |
| 该字段被映射为 `hostcall_fuel` | `docs/rfc/RFC-0051-amendment-A1-carriage-and-budget-mapping.md` §A1.3（`max_hostcalls → RuntimeLimits.hostcall_fuel`）；`runner/sico-runner/src/lib.rs:261`（doc 注释同一映射）；`lib.rs:278-279`（`narrowed_by`：`min(hostcall_fuel, max_hostcalls)`）；`crates/sico-package/src/provenance.rs:269`（`check_budgets_within`：`max_hostcalls > package.hostcall_fuel`） | **确认**（单位不匹配：计数 vs 每调用 fuel 费用） |
| `hostcall_fuel` 语义是「每调用费用」而非计数 | `lib.rs:238-239`（文档为 "Canonical-ABI transport budget (elements are charged per value)"——**第三种说法**）；`lib.rs:1342`（`STREAM_HOSTCALL_FUEL_PER_CALL = 64 << 20`，命名 **per call**）、`lib.rs:1346`（`set_hostcall_fuel`）；`lib.rs:2235`、`lib.rs:2517`（`store.set_hostcall_fuel(...)`，Wasmtime 燃料/每次 hostcall） | **确认**（同一字段三处描述互不一致：计数 / 元素每值 / fuel 每调用） |
| `invoke`/`start_debug` 用 `max(limits.hostcall_fuel, stdin*64 + 64 MiB)` 作地板 | `lib.rs:2229-2235`（`start_debug`：`stdin.len()*64 + (64<<20)` 再 `.max(limits.hostcall_fuel)`）、`lib.rs:2511-2517`（`invoke` 同式）；`lib.rs:248`（`RunnerLimits::default().hostcall_fuel = 128 << 20`） | **确认** |
| 冻结 fixture 预算 100000 被 64 MiB 地板覆盖 | `crates/sico-package/tests/fixtures/provenance/model-fixture.provenance.json:17`（`max_hostcalls: 100000`）；代入：`min(128<<20, 100000)=100000`，空 stdin 地板 `64<<20=67,108,864` ⇒ `set_hostcall_fuel(max(100000, 67108864)) = 67108864` | **确认**（低于 `stdin_len*64+64 MiB` 的预算零效果） |

### 2.2 C2 确认（全部属实，无纠正；一处下界声明）

| 断言 | 证据 | 结论 |
|---|---|---|
| `SCRIPT_MEMORY_BYTES` ≈ 2.06 GiB | `crates/sico-codegen-wasm/src/canonical.rs:28`（`MEMORY_PAGES = 33792`）、`:39`（`SCRIPT_MEMORY_BYTES = 33792 * 65536 = 2,214,592,512`）；`crates/sico-codegen-wasm/tests/script_abi.rs:35-36`（断言 `2_214_592_512`） | **确认** |
| Script 组件声明 `min == max == MEMORY_PAGES`（固定，非区间） | `crates/sico-codegen-wasm/src/package_builder.rs:119-120`、`crates/sico-codegen-wasm/src/lib.rs:1255-1256`、`:1337-1338`、`canonical.rs:975-976` | **确认** |
| `script_profile_limits().memory_bytes = SCRIPT_MEMORY_BYTES`；低于声明最小不可满足 | `crates/sico-package/src/lib.rs:151-157`；`lib.rs:143-149`（STEP-0497 注释：opaque "forcing trap when growing memory to … bytes"） | **确认** |
| build 期已拒绝 `limits.memory_bytes < SCRIPT_MEMORY_BYTES` | `crates/sico-package/src/lib.rs:441-446` | **确认**（证明该原则已被接受；预算路径重开同洞） |
| `check_budgets_within` 只查子集（上界），无下界 | `crates/sico-package/src/provenance.rs:255-275`（仅 `max_memory_bytes > package.memory_bytes`）；冻结 fixture `max_memory_bytes: 1,048,576`（`model-fixture.provenance.json:15`）过子集检查 | **确认** |
| `narrowed_by` 把预算映射为 `memory_ceiling` | `runner/sico-runner/src/lib.rs:275-276`（`min(package, 1,048,576)`）、`lib.rs:2155`/`lib.rs:2369`（`memory_ceiling`）、`lib.rs:1350-1360`（`memory_growing` 拒绝 > ceiling） | **确认** |
| 真实组件下 1 MiB 预算 → 运行期实例化 trap | 机制由 STEP-0497 注释（`lib.rs:143-149`）与 `min==max` 固定声明（§2.2 第 2 行）支撑；STEP-0578 §4/§5 因 C2 故意用最小 guest 而非真实组件（`STEP-0578:51`） | **下界声明**：为**机制已文档化**，非本片新实测——实现片须先以真实 Script 组件 + 1 MiB 预算复测，再宣告地板为修复 |

### 2.3 兼容性检索结果（AGENTS §9 口径）

| 对象 | 检索命令 | 结果 |
|---|---|---|
| 235 条冻结语料 | `grep -iE 'provenance\|max_hostcalls\|hostcall_fuel\|max_memory_bytes\|max_wall_ms' selfhost/`；`selfhost/corpus-v0.json` 显式 `"entry_count": 235` | **0 条命中**（唯一命中为 `corpus-parity-map-v1.json:22` 文档串 "provenance of the evidence."，非预算字段）。235 条语料**不受 C1/C2 影响**（无 provenance/budget 字段；M22 自举语料与 M24 provenance 正交） |
| 既有 `.sapp` / M7 信任链 | `crates/sico-package/tests/fixtures/provenance/` 含 `model-fixture.bin` + `model-fixture.provenance.json` + `malformed-unknown-field.provenance.json`（git 溯源 `095227b`）；`crates/sico-package/src/lib.rs:611`（`verify` → `verify_provenance_in_package`） | C2-a 采纳后 `model-fixture`（`max_memory_bytes=1 MiB`）**将新拒绝**（1 MiB < 2.06 GiB 地板）——需重基线；C1-a 不影响 `.sapp` 安装（仅语义变更，阈值需重推）；C1-b/C2-b 需给旧 `.sapp` 语义版本化 |
| 既有测试 | `crates/sico-package/tests/provenance_budget.rs:75`（断言 `1_048_576`）、`runner/sico-runner/tests/provenance_runtime.rs:183`（断言 `1_048_576`）、`:185`（断言 `hostcall_fuel==100000`）、`:321-372`（端到端依赖 1 MiB）、`provenance_install.rs`（state-1 依赖 1 MiB；state-2 limit+1 用 `hostcall_fuel+1`） | C2-a 需改 `1_048_576` 断言与 fixture；C1-a 不改断言值（映射不变）但 `hostcall_fuel` 语义文档要改；C1-b 需新字段+新断言。`crates/sico-runtime/tests/memory_ceiling.rs` 钉 `SCRIPT_MEMORY_BYTES` 下界，不受 C1 影响 |

## 3. 决策点 C1：hostcall 预算语义（二选一）

**问题陈述**：`max_hostcalls`（计数）被 A1.3 映射为 `hostcall_fuel`（每次 hostcall 的 fuel 费用）；且 `invoke`/`start_debug` 以 `stdin_len*64 + 64 MiB` 为地板，低于地板的预算被覆盖（实测：100000 预算 → 实际生效值 64 MiB）。

**当前行为**：预算对 hostcall 计数**不生效**（地板覆盖），且「计数」从未被作为计数强制。

**根因**：①A1.3 映射把「计数」当作「每调用费用」赋给 `hostcall_fuel`（单位不匹配）；②run 路径在 `set_hostcall_fuel` 前用 `max(...)` 地板，未考虑预算语义。

**选项 (a) 推荐**：把 `max_hostcalls` 口径改为「每次 hostcall 的 fuel 费用」并修订 A1.3 映射（不新增上限路径；阈值需换算）。**选项 (b)**：保留「计数」语义并引入独立计数上限（新增上限路径 ⇒ 需 RFC + 更强默认拒绝 + 每调用计数点开销评估）。

**后果与不变量影响**：

| 维度 | (a) 每次 hostcall 的 fuel 费用 | (b) 独立计数上限 |
|---|---|---|
| 新增上限路径 | **无**（保持 A1.3/STEP-0578「只收窄、无第二条上限路径」不变量；`lib.rs:1554-1561`、AGENTS §helpers narrow never widen） | **有**（新 limit kind + 运行期计数器 + 新拒绝判别式 ⇒ 违反「不新增上限路径」不变量，需独立 RFC + default-deny 缺省值） |
| 默认拒绝强度 | 不变（子集 + limit+1 fail-closed 原样） | 需为新字段定义缺省（绝不可开顶） |
| 与既有 `RuntimeLimits`/fuel/timeout 语义冲突 | 与 `limits.fuel` 交互：有效调用数 ≈ `fuel / hostcall_fuel`（调用方自身 fuel 仍封顶）；不改 timeout | 计数与 fuel/timeout 相互独立；不冲突 |
| guest 可观测差异 | 每次调用 ≥ 每调用费用 → `FuelExhausted`；大量廉价调用**不受计数约束**（仅受总 fuel）；无独立「hostcall-count」结果 | 廉价调用也计数 → 新增 typed「hostcall 预算超限」结果；即使 fuel 未耗尽也被计数截断 |
| M7 信任链 / `.sapp` | `.sapp` 安装不变（语义变更）；阈值 100000 需**重新推导**（非可复用） | `.sapp` 安装不变，但旧 `.sapp` 的预算语义静默改变，需字段/版本化 |
| 实现片 | 见 §6 | 见 §6 |

**推荐项与理由**：**(a)**。复用既有 limit kind、保持「不新增上限路径」不变量、只改文档与阈值换算。诚实代价：字段不再逐字表达「计数」，owner 须接受「此字段 = 每次 hostcall 的 fuel 费用」。若 owner 对 RFC-0051 §3 property 3 的意图是字面计数，则 **(b)** 是唯一忠实选项，且「不新增上限路径」须重裁（A1.3 记为偏好，非绝对）。

## 4. 决策点 C2：Script 组件内存下界（二选一）

**问题陈述**：真实 Sico Script 组件声明 `SCRIPT_MEMORY_BYTES = 2,214,592,512`（≈2.06 GiB，`min==max` 固定）；provenance 预算 `max_memory_bytes`（冻结 fixture 1 MiB）过 install 子集检查却把 `memory_ceiling` 压到 1 MiB ⇒ 运行期实例化 trap（STEP-0497 同源）。

**当前行为**：1 MiB 预算通过 `check_budgets_within`（仅查子集），运行期 `narrowed_by` → `memory_ceiling=1 MiB` → 真实组件实例化不透明 trap。

**根因**：预算只做子集上界检查、无下界；`max_memory_bytes` 被当作「guest 总线性内存上限」，而 Script 组件声明内存固定为 2.06 GiB，更小预算不是「更紧」而是「不可满足」。

**选项 (a) 推荐**：对 Script 类包（`Package.script.is_some()`）要求 `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES`，否则 install 期类型化拒绝（口径 = 「组件可实例化的线性内存下界」）。**选项 (b)**：把预算口径改述为「资产操作内存」并与组件声明内存解耦（`memory_bytes = SCRIPT_MEMORY_BYTES + max_memory_bytes`；须诚实向 owner 表达「此预算不约束组件线性内存」）。

**后果与不变量影响**：

| 维度 | (a) 下界 = SCRIPT_MEMORY_BYTES | (b) 资产操作内存解耦 |
|---|---|---|
| 新增上限路径 | 无（仅给既有子集检查加**下界**，仍 fail-closed） | 无（改映射公式） |
| 默认拒绝强度 | **增强**（低于下界预算由「运行期不透明 trap」提前为「install 类型化拒绝」，无 guest 执行） | **减弱**（budget 不再约束 guest 总内存：读者看 `1 MiB` 实则允许 `2.06 GiB + 1 MiB`）——须明示 |
| provenance 约束什么 | 「guest 内存 ≥ 组件声明内存」；预算仍需 ≤ package ceiling（只收窄） | 「预算只约束声明基线之上的增长」；与组件声明内存解耦 |
| 与既有 `RuntimeLimits`/fuel/timeout 冲突 | 无（仅内存预算可取区间加下界 `[SCRIPT_MEMORY_BYTES, ceiling]`；fuel/timeout 不变） | 内存上限变 `基线+增长`；fuel/timeout 不变 |
| 冻结 fixture 兼容 | **破坏**：`1 MiB < 2.06 GiB` ⇒ `model-fixture` install 新拒绝；3 个测试断言 `1_048_576` 需重基线（预算提到 ≥ `SCRIPT_MEMORY_BYTES` 会使「收窄」测试退化，需补非收窄测试或改测试 guest） | **不破坏**：1 MiB 仍是合法增长预算；3 测试不需重基线（除增长记账改动） |
| M7 信任链 / `.sapp` | 既有 Script `.sapp` 且内存预算 < 地板将新拒绝（当前唯一冻结 fixture 受影响） | 旧 `.sapp` 安装不变但预算语义静默改变，需字段重命名/版本化（属 schema 变更，A2 不声称） |
| 实现片 | 见 §6 | 见 §6 |

**推荐项与理由**：**(a)**。保持「预算约束 guest 内存」为诚实契约，把不可满足情形提前为类型化拒绝，并镜像 STEP-0497 已接受原则（`min==max` 固定 ⇒ 更小预算不可满足而非更紧）。若 owner 意图预算治理「资产相对增长」，**(b)** 才成立，且须强制明示 + 字段重命名/版本化，避免隐含「总内存上限」。

## 5. 推荐项汇总

- **C1 → (a)**：`max_hostcalls` 口径改为「每次 hostcall 的 fuel 费用」，修订 A1.3 映射与文档；不新增上限路径；阈值 100000 须重新推导。
- **C2 → (a)**：对 Script 类包要求 `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES`，否则 install 类型化拒绝；需重基线冻结 fixture 与 3 个测试。
- 两项均**不削弱 default-deny**；均保持「预算只收窄、无第二条上限路径」；均不翻 gate 4，仅关闭 STEP-0578 §7 标注的两条未达路径。

## 6. 若采纳所需实现片清单（owner 接受后另开实现 STEP，本片不开工）

**C1-a 实现片（若采纳）**：
- 文件：`docs/rfc/RFC-0051-model-provenance-manifest-v0.md`（§2/§3 语义句）、`docs/rfc/RFC-0051-amendment-A1-carriage-and-budget-mapping.md`（A1.3 映射句）、`crates/sico-package/src/provenance.rs`（`Budgets::max_hostcalls` 文档，:66-67）、`runner/sico-runner/src/lib.rs`（`RunnerLimits::hostcall_fuel` 文档，:238-239；`narrowed_by` 注释，:261）。
- 函数/测试要点：不改 `check_budgets_within`/`narrowed_by` 逻辑；改 4 处文档 + 重推 fixture 阈值语义（新阈值 `T` 使 `fuel/T` 等于意图调用数）；`provenance_runtime.rs:185` 断言值可能随阈值变更；新增「每调用费用触发 `FuelExhausted`」测试与「廉价调用不受计数约束」测试。
- 验证命令：`$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"`；`& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test provenance_runtime -- --test-threads=1`；`& $cargo test --locked --offline -p sico-package`；`git diff --check`。

**C1-b 实现片（若采纳，另需独立 RFC）**：`runner/sico-runner/src/lib.rs`（`RunnerLimits` 新字段 + `RunState` 计数 + hostcall 路径 `+1` 计数点 + 新拒绝判别式）、`crates/sico-package/src/lib.rs`（`RuntimeLimits` 新字段 + default-deny 缺省）、`crates/sico-package/src/provenance.rs`（`check_budgets_within` 计数比较）、测试（计数触发、缺省值、hostcall 热路径开销基线）。验证同上 + `cargo clippy --offline --lib -- -D warnings`。

**C2-a 实现片（若采纳）**：
- 文件：`crates/sico-package/src/provenance.rs`（`check_budgets_within` 加下界参数：需知是否 Script 包）、`crates/sico-package/src/lib.rs`（`verify`/`verify_provenance_in_package` 传 `manifest.script.is_some()` 或 `package_limits.memory_bytes == SCRIPT_MEMORY_BYTES`）、`crates/sico-package/tests/provenance_install.rs`（新增「Script 包内存预算 < 地板 → Manifest 拒绝」测试）、`crates/sico-package/tests/provenance_budget.rs` 与 `runner/sico-runner/tests/provenance_runtime.rs`（重基线 `1_048_576` 断言）。
- 函数/测试要点：`check_budgets_within(script: bool, ...)` 或新 `check_script_memory_floor`；下界拒绝判别复用 `PackageError::Manifest`（带字段名）；重基线冻结 fixture（`max_memory_bytes` 提到 `≥ SCRIPT_MEMORY_BYTES`）或改测试 guest；补「内存预算非收窄时仍过子集」与「Script 包下界触发」两条测试。
- 验证命令：同上 runner 测试 + `cargo test --locked --offline -p sico-package` + `git diff --check`。

**C2-b 实现片（若采纳）**：`runner/sico-runner/src/lib.rs`（`narrowed_by` 改 `memory_bytes = SCRIPT_MEMORY_BYTES + max_memory_bytes` 或新增增长记账）、`crates/sico-package/src/lib.rs`（`RuntimeLimits`/文档口径）、文档（RFC-0051 §2 口径改述 + 明示「不约束组件线性内存」）、测试（增长预算语义、基线不变量）。字段重命名/版本化另属 schema 变更，需单独 RFC。

## 7. 兼容性影响（若采纳，检索结果见 §2.3）

- **235 语料**：不受影响（无 provenance/budget 字段，与 M24 正交）。
- **既有测试**：C2-a 需改 `1_048_576` 断言（`provenance_budget.rs:75`、`provenance_runtime.rs:183`）与 end-to-end（`:321-372`）；C1-a 断言值不变（映射不变）但文档变更；C1-b/C2-b 需新字段与断言。
- **既有 `.sapp`**：C2-a 使 `model-fixture`（1 MiB）install 新拒绝，需重基线；C1-a 仅语义变更；C1-b/C2-b 旧 `.sapp` 语义静默变化，需字段/版本化。
- **M7 信任链**：安装路径（`verify` → `verify_provenance_in_package`，`lib.rs:611`）逻辑不变；C2-a 在其上加下界，仍是 fail-closed。
- **不确定项**：①是否存在 `model-fixture` 之外、携带 Script 身份且内存预算 < 2.06 GiB 的既有 `.sapp`（未全库检索）；②C1 阈值重推的具体数值（取决于意图调用数与 `limits.fuel`）；③`set_hostcall_fuel` 与 `limits.fuel` 的精确交互（Wasmtime 版本语义）未实测。

## 8. 未决问题（owner 需回答）

1. **C1**：取 (a) 还是 (b)？
2. **C2**：取 (a) 还是 (b)？
3. **不变量**：「不新增上限路径」（A1.3；`lib.rs:1554-1561`）是硬不变量（⇒ C1 必 (a)、C2 必 (a)）还是可重裁的偏好（⇒ C1-b 可考虑）？
4. **CLI/app 接线**：`VerifiedPackage::provenance_budgets` / `run_program_with_budgets` 接入 `sico-cli`/`sico-app-cli` 应**等本裁决**还是并行？（两者正交；但若 C1-b 新增 limit kind，接线的 limit 集会变。）
5. **fixture 重基线**：若 C2-a 采纳，由谁重基线冻结 fixture 与 3 个测试？预算提到 `≥ SCRIPT_MEMORY_BYTES` 后内存「收窄」测试退化（只测地板）——是否需要独立非收窄测试或改用更小声明内存的测试 guest？

## 9. 诚实收窄

- **本片未实现任何一方**：未改 `provenance.rs`、`lib.rs`、`canonical.rs` 或任何测试；两份草案全部标 `PROPOSED`；无选项写成「已裁决」。
- **gate 4 维持 STEP-0578 现状**：hostcall 计数运行期强制（C1）与真实 Script 组件内存语义（C2）两条路径在 owner 接受修正案并另开实现 STEP 前**未达**；不得据本片宣称运行期强制完整。
- **C2 运行期 trap 为机制文档化而非本片新实测**（§2.2 末行）：实现片须先以真实 Script 组件 + 1 MiB 预算复测。
- **证据等级**：`internal-fixture` 口径沿用 STEP-0578；无 clean-room consumer、无 external pilot、非 live-model。
- **未提交未推送**：本片只新增两个文件（见下），未 commit/push，未触碰任何被禁文件。
