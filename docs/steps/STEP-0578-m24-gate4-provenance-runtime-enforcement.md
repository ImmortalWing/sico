# STEP-0578 M24 gate-4 provenance **预算运行期强制 + 取消接线**：property 3 运行期半部与 property 4 接真实 runner

| 项 | 值 |
|---|---|
| 片型 | 实现 + 测试：`crates/sico-package/src/lib.rs`（`VerifiedPackage::provenance_budgets` 挂载点）、`crates/sico-package/tests/provenance_budget.rs`（新）、`runner/sico-runner/Cargo.toml`（`sico-package` 由 dev-dep 提为常规依赖，修复既有 src 断编）、`runner/sico-runner/src/lib.rs`（`Runner::run_program_with_budgets` 接线 + 既有 `RunnerLimits::narrowed_by` 落地为可调用）、`runner/sico-runner/tests/provenance_runtime.rs`（新）；`selfhost/`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/steps/README.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md` 等 **零改动** |
| exit | **达成（无争议半部）**：sico-package 新增 3 测试绿（预算挂载点 3 态）；runner 新增 6 测试绿（narrowing 纯性 + memory/wall/fuel/取消 4 态 + 包→预算→runner 端到端）；既有 `narrowed_by` 从死代码变为运行期实际被调用；`git diff --check` 净。**见 §6 合同缺口** —— hostcall 预算与真实 Script 组件内存语义不完整，不翻任何 gate |
| 不翻门 | 门 4 维持现状：本片只完成运行期强制与取消的 **runner 半部**，`hostcall` 运行期强制与「真实 Script 组件下 1 MiB 预算不可满足」属合同缺口，留 owner 裁决；不申请任何 gate GO |
| 证据等级 | `internal-fixture`：冻结 fixture（`model-fixture.bin` 256B + `.provenance.json`）+ wasm-encoder 手编最小 guest，无 clean-room consumer、无 external pilot、非 live-model |

## 1. 要回答的问题

RFC-0051（accepted）+ A1 amendment（accepted）把 fixture-model 资产预算（`max_memory_bytes` / `max_wall_ms` / `max_hostcalls`）带进 `.sapp` 包。加载半部（属性 1/2/5 与 A1.2/A1.3 的 install-time 子集检查）已落地（`provenance.rs` + `verify`）。**缺口**是：

1. **property 3 运行期半部**：预算是否真的收窄 run 的 `RunnerLimits` 并被执行路径强制；
2. **property 4**：timeout / fuel 触发时是否给出 typed cancellation outcome（对齐 M9）。

## 2. 侦察结论（只读）

- `RunnerLimits::narrowed_by(&Budgets)`（`runner/sico-runner/src/lib.rs`）**已存在但零调用点** —— 是死代码，预算从未流入 run 路径。
- `Runner::run_program` 只接收显式 `&RunnerLimits`，无任何接受 provenance 预算的入口；`prepare_package`/`link_packages`（RFC-0039 §2.4）处理的是**纯组件包**（`sico:user/...`），不是 `.sapp` 资产包，与 provenance 无关。
- `VerifiedPackage` 无公开途径取回 manifest 声明的 `Budgets`（`verify_provenance_in_package` 只校验不返回）。
- 运行期强制本身**已存在**：`run_unchecked` 设 `memory_ceiling`（`RunState::memory_growing` → `denied` → `MemoryLimit`）、`set_fuel`（→ `FuelExhausted`）、epoch 截止（→ `Timeout`），`CancelToken` 在 watchdog 每 tick 检查（→ `Cancelled`）。
- **合同层面缺口（不自行裁定，见 §6）**：
  - `invoke`/`start_debug` 计算 `needed = max(limits.hostcall_fuel, stdin_len*64 + 64 MiB)` —— hostcall 预算实际是 **64 MiB 地板**，预算把 `max_hostcalls`（计数）映射为每调用 fuel 费用（单位不匹配），低于地板时**不生效**；
  - 预算 `max_memory_bytes` 映射为 guest 线性内存上限，而真实 Sico Script 组件声明 ~2.06 GiB（`SCRIPT_MEMORY_BYTES`），冻结 fixture 预算 1 MiB 通过 install 子集检查却在运行期**不可满足**（不透明实例化 trap）。

## 3. 实现

### 3.1 `crates/sico-package`：预算挂载点

`VerifiedPackage::provenance_budgets() -> Result<Option<Budgets>, PackageError>`：扫描 `resources` 中的 `.provenance.json`，返回其声明的 `Budgets`。包至多携带一个 provenance manifest；第二个是**歧义运行预算**，fail-closed `Manifest` 拒绝（绝不静默选取）；无 manifest 返回 `None`（A1.1 opt-in）。这是「budgets 从 manifest 到 RuntimeLimits」的挂载点：调用方（app 层或 runner 测试）据此获得预算并交给 runner 收窄。

### 3.2 `runner/sico-runner`：运行期接线

- `Runner::run_program_with_budgets(component, input, base_limits, budgets, cancel, fs)`：`base_limits.narrowed_by(budgets)` 后委托既有 `run_program`。预算只能**收窄**（`narrowed_by` 逐字段取 min），不新增第二条上限路径，不改任何限额。
- 修复既有断编：`src/lib.rs` 的 `use sico_package::provenance::Budgets` 在 `sico-package` 仅为 dev-dep 时无法编译；把 `sico-package` 移入 `[dependencies]`。

## 4. 测试

`crates/sico-package/tests/provenance_budget.rs`（3 测试）：冻结预算挂载点返回 1 MiB / 5000 ms / 100000；无 manifest → `None`；双 manifest → 歧义 `Manifest` 拒绝。

`runner/sico-runner/tests/provenance_runtime.rs`（6 测试，单线程）：

1. `narrowed_by` 映射冻结预算且**绝不加宽**（over-ceiling 预算 clamp 到上限，不新增第二条路径）；
2. 手编最小 guest 增长内存越过 1 MiB 预算 → typed `MemoryLimit`，且 Host 复用后无预算同 guest → `Trap`（证明是预算所致，非默认值）；
3. 自旋 guest + 收紧 wall 预算（30 ms）→ typed `Timeout`（冻结 fixture 的 5000 ms 在纯性测试与 install 子集测试钉住）；
4. 自旋 guest + 小 fuel（预算无 fuel 字段，fuel 是调用方自身上限）→ typed `FuelExhausted`；
5. 自旋 guest + 显式 `CancelToken` 中途取消 → typed `Cancelled`（M9）；
6. 端到端：`build_script_v1`（冻结 fixture + 手编 guest 组件）→ `verify` → `provenance_budgets` → `run_program_with_budgets` → `MemoryLimit`。

最小 guest 用 wasm-encoder 手编（1 页内存声明 + `cabi_realloc`/`cabi_post_run`/`run` 导出，`wrap_script_component` 包装），结构复刻 `runner.rs::malformed_component`（已验证可实例化可调用）。真实 Sico Script 组件声明 ~2 GiB 内存，1 MiB 预算对其不可满足 —— 这是 §6 合同缺口，故运行期内存测试用最小 guest（任务书明确「wasm-encoder 手编最小 guest」）。

## 5. 结果

- sico-package 全套（`cargo test --offline -p sico-package`）：**51 passed / 0 failed**（18 unit + 1 binary_carrying + 15 package + 3 provenance_budget + 6 provenance_fixture + 4 provenance_install + 4 security_properties）—— 无回归；其中本片新增 `provenance_budget` 3 测试。
- runner：`cargo test --offline --test provenance_runtime -- --test-threads=1` → **6 passed / 0 failed**。
- runner `cargo fmt -- --check` 净；`cargo clippy --offline --lib -- -D warnings` 与 `cargo clippy --offline --test provenance_runtime -- -D warnings` 均绿。
- `git diff --check` 净。

## 6. 合同缺口（不自行裁定，呈 owner）

| # | 缺口 | 证据 | 最小修正案草案 |
|---|---|---|---|
| C1 | hostcall 预算运行期不生效：`invoke`/`start_debug` 用 `max(limits.hostcall_fuel, stdin*64 + 64 MiB)`，预算 `max_hostcalls` 低于 ~64 MiB 地板时被覆盖；且把「计数」映射为「每调用 fuel 费用」（单位不匹配） | `runner/sico-runner/src/lib.rs`（`invoke`、`start_debug`） | 二选一：(a) 明确 `max_hostcalls` 是**每次 hostcall 的 fuel 费用**而非总预算，`narrowed_by` 文档与 A1.3 映射同步改口径；(b) 若意图是「hostcall 计数上限」，需在运行期增加独立的 hostcall 计数边界（新字段/新上限路径，违反「不新增第二条上限路径」，需 RFC）。**倾向 (a)** —— 保持「不新增上限路径」 |
| C2 | 预算 `max_memory_bytes` 映射为 guest 线性内存上限，但真实 Sico Script 组件声明 `SCRIPT_MEMORY_BYTES` ≈ 2.06 GiB；冻结 fixture 预算 1 MiB 通过 install 子集检查却在运行期不可满足（不透明实例化 trap，STEP-0497） | `crates/sico-package/src/lib.rs`（`script_profile_limits`）、`crates/sico-codegen-wasm/src/canonical.rs`（`MEMORY_PAGES`） | A1.3 增补：对 Script 包（`manifest.script.is_some()`）要求 `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES`，否则 install 拒绝；或明确 `max_memory_bytes` 约束的是**资产操作内存**而非 guest 线性内存，并相应修订 `narrowed_by` 映射。**倾向前者**（install 拒绝早于运行期不透明 trap） |
| C3 | 预算无 fuel 字段；`narrowed_by` 有意不动 `limits.fuel`（non-termination 界保持调用方上限） | `runner/sico-runner/src/lib.rs`（`narrowed_by`） | 无需修正案：如实记录 —— 「fuel 预算被强制」指调用方自身 `limits.fuel` 的确定性非终止界（本片已测），非预算属性 |

## 7. 残余缺口与风险

- 不翻任何 gate；门 4 的 hostcall 运行期强制（C1）与真实 Script 组件内存语义（C2）留 owner 裁决后另立切片。
- `VerifiedPackage::provenance_budgets` 与 `run_program_with_budgets` 尚未接入 CLI/app 层（`crates/sico-cli`、`crates/sico-app-cli` 不在本片范围）—— 是后续接线点，非本片遗漏。
- 运行期内存测试用最小 guest 而非真实 Sico Script 组件（C2 所致）；真实组件路径在 C2 裁决前不得宣称运行期强制。
- 无 clean-room consumer / external pilot；证据等级保持 `internal-fixture`。
