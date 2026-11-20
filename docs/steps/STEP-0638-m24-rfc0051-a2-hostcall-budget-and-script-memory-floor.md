# STEP-0638 — RFC-0051-A2 implementation: hostcall budget semantics + Script memory floor + I3 wiring

> - status: implementation complete locally（**未 commit/push；本片关闭 gate 2 的 C1/C2/I3 残余 ⇒ gate 2 具备翻 GO 的执行证据**）
> - date: 2026-11-19
> - contract: [`RFC-0051-A2`](../rfc/RFC-0051-amendment-A2-hostcall-budget-and-script-memory-floor.md)（**ACCEPTED，owner 2026-11-19「授权都按你推荐的来」**：C1=(a)、C2=(a)、不变式=硬、I3 在两裁决后接、agent 重基线冻结样例+3 测试）
> - validator: `cargo test -p sico-package --lib --tests` + `cargo test --manifest-path runner/sico-runner/Cargo.toml --test provenance_runtime --test provider_crash_isolation`
> - evidence class: `internal-fixture`

## 1. 目的

落地 RFC-0051-A2 的三项裁决，关闭 M24 gate 2（M17 gate 4 model provenance）此前由 STEP-0583 起草、STEP-0578 留下的两条未达路径（C1 hostcall 预算运行期不生效 + 单位错配；C2 真实 Script 组件内存地板使小额预算不可满足），并接线 I3（`provenance_budgets`/`run_program_with_budgets` 接入 CLI/app 层）。

## 2. 改动面

- **C1-a（doc-only 口径重释）**：`max_hostcalls` 重释为 **per-hostcall fuel cost**（非计数）。A1.3 映射 `max_hostcalls → hostcall_fuel` 逐字节保留，`narrowed_by` 仍 `min(hostcall_fuel, max_hostcalls)`、`check_budgets_within` 仍只做子集上界检查 ⇒ **不新增第二条上限路径**（不变式硬）。`runner/sico-runner/src/lib.rs` 的 `narrowed_by`/`effective_memory_limits` 文档注释更新为 per-call fuel 费用口径。运行期五件套证据（`provenance_runtime` 6/6）不变。
- **C2-a（Script 包内存地板）**：`crates/sico-package/src/lib.rs` 暴露 `SCRIPT_MEMORY_BYTES = 33792×65536 = 2,214,592,512`（≈2.06 GiB）；install 期对 **Script 类包**强制 `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES`，否则 distinct typed `Manifest` 拒绝（镜像 STEP-0497 build 期拒绝原则，增强 default-deny）。非 Script 包与等于地板的预算不受影响。
- **I3（运行期限额接线）**：`crates/sico-runtime/src/lib.rs` 的 `effective_memory_limits` 按 `provenance_budgets()` 收窄（只收窄、永不放大），新增 `PackageRuntimeError::Provenance` 变体承载 provenance 来源的运行期错误。
- **重基线（owner 授权 agent 执行）**：冻结样例 `model-fixture.provenance.json` 的 `max_memory_bytes` 1,048,576 → 2,214,592,512（≥ Script 地板）；`provenance_budget.rs`/`provenance_install.rs`/`provenance_runtime.rs`/`memory_ceiling.rs` 共 3+ 测试随重基线更新。`provenance_runtime.rs` 的 `memory_budget_is_enforced_*` 改用**显式 sub-ceiling 预算（1 MiB）**测 memory narrowing——地板使真实 Script 包不可达该性质，但非 Script/sub-ceiling 预算仍成立。

## 3. 回归修复（本片发现并修复）

C2-a 重基线把冻结 fixture 预算从 1 MiB 提到 2 GiB 后，`provider_crash_isolation.rs::resource_exhaustion_is_typed_and_host_stays_reusable`（gate 2 provider-crash 隔离第 1 件）仍用冻结 fixture 预算 ⇒ `grow_past_one_mib`（最多 1024 页 = 64 MiB）不再越过预算 ⇒ 实测得 `Trap(unreachable)` 而非期望的 `MemoryLimit`（**1 FAILED / 3 passed**）。修复与 `provenance_runtime.rs` 同法：该测试改用显式 sub-ceiling 预算 `Budgets { max_memory_bytes: 1_048_576, ..fixture_budgets() }`，注释订正为 A2 C2-a 重基线口径。修复后 **4/4 绿**。

## 4. 验证（本机 MSVC 实跑）

| 命令 | 结果 |
|---|---|
| `cargo test -p sico-package --lib --tests` | 全绿（21+1+15+3+6+5+4，含 `provenance_budget`/`provenance_install`/`provenance_fixture`/`security_properties`/`memory_ceiling`） |
| `cargo test --manifest-path runner/… --test provenance_runtime` | **6/6 绿**（narrowed_by 映射+不放大 / memory（sub-ceiling 1 MiB）/ wall timeout / fuel / mid-flight cancel / 端到端 `MemoryLimit`） |
| `cargo test --manifest-path runner/… --test provider_crash_isolation` | **4/4 绿**（修复回归后：resource-exhaustion→`MemoryLimit`、provider error→`HostProviderFailure`、provider panic→typed、各态宿主存活+复用） |

## 5. 诚实收窄

- C1-a 为**口径重释 + 文档**，运行期行为（`min` 收窄）不变；`max_hostcalls` 字段名与 per-call fuel 语义的措辞错配**保留**（A2.1-a 已记录，改字段名须版本化、超出本片）。
- 单机 MSVC 快照，非独立 CI；未跑全 workspace。
- **gate 2 现具备五件套全执行证据**（digest-verified load + malicious-asset 拒绝、limit+1 typed、budget 运行期强制、mid-inference cancel、provider-crash 隔离免宿主重启），但**翻 GO 的最终裁决在 M24 出口审计刷新文件**（本片只交付实现证据）。
- 未触碰 `selfhost/*`（并发 M22 代理持有）、`image-codec-core.wasm`、WIT、包接口语义。
