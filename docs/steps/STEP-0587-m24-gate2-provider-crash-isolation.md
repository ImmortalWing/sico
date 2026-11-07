# STEP-0587 M24 gate-2 **provider-crash 隔离（免宿主重启）**：五件套最后一件 + 可注入 fault seam

| 项 | 值 |
|---|---|
| 片型 | 实现 + 测试：`runner/sico-runner/src/stream_provider.rs`（新，`StreamTransport` / `StreamTransportFactory` seam）、`runner/sico-runner/src/lib.rs`（`RunState.io` 改 `Box<dyn StreamTransport>`、`Runner::with_stream_transport` builder、`HostStreamError` 由 `pub(crate)` 抬为 `pub`、流 provider 闭包改走 seam；**不动** provenance/limits 段既有语义）、`runner/sico-runner/tests/provider_crash_isolation.rs`（新，4 测试）；`crates/sico-package/src/`、`crates/sico-package/tests/`、`crates/sico-ui-renderer-d2d/**`、`ui.rs`、`ui_authority.rs`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`docs/steps/README.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、`docs/plans/**` **零改动** |
| exit | **达成**：新测试 `provider_crash_isolation` 4/4 绿（资源耗尽 → typed `MemoryLimit` + 宿主复用；provider 返回错误 → typed `HostProviderFailure` + 宿主复用；provider 内部 panic 被拦截 → typed trap + 宿主复用；默认 runner 仍走真实 stdio 传输，作正控制）；回归 `provenance_runtime` 6/6 绿、`sico-package` 51/51 绿；runner `lib` 与 `--test provider_crash_isolation` clippy `-D warnings` 绿；本片涉及文件 `cargo fmt --check` 净；`git diff --check` 净 |
| 不翻门 | 本片不申请任何 gate GO。gate 2 五件套的**第 5 件（provider-crash 隔离）**已具备 internal-fixture 证据，但 gate 2 整体裁决（含既述 C1/C2 合同缺口、其余四件的证据口径）属出口审计与 owner；本片只报告**完成度** |
| 证据等级 | `internal-fixture`：冻结 fixture（`model-fixture.bin` 256B + `.provenance.json`）+ 前端真实编译的 `.sico` guest（`sico build --profile script-v0`）+ wasm-encoder 手编最小 guest（资源耗尽探针）；无 clean-room consumer、无 external pilot、非 live-model |

## 1. 要回答的问题

M24 plan §3 gate 2 = M17 gate 4 model provenance，五件套为：**digest-verified load**、**budget 强制（limit+1）**、**取消（中途）**、**provider-crash 隔离（免宿主重启）**、**结构/绑定校验**。前四件已登记（STEP-0334 结构/digest/绑定、STEP-0578 budget+取消），**缺第 5 件：provider-crash 隔离**。本片回答：

1. 当 **provider 路径**发生「provider 侧崩溃/资源耗尽」时，宿主进程是否**不崩溃**、Store 是否**可复用**（后续能跑别的 guest）、对外是否**类型化 outcome**（不 panic、不吞错、不伪造成功）；
2. 该属性能否被一个**测试侧可注入、显式、类型化、最小暴露**的 fault seam 确定性触发——**不在生产路径加后门开关**。

## 2. 侦察结论（只读）

- 类型化 outcome 家族**已存在**且未被本片改动：`RunOutcome::HostProviderFailure { provider_id }`（`src/lib.rs`）、`RunOutcome::MemoryLimit` / `FuelExhausted` / `Timeout` / `Cancelled` / `Trap`。`classify_error` 已把 `ProviderFault` marker 映射为 `HostProviderFailure`。
- 流 provider（`sico:script/streams@0.1.0`，RFC-0030）的 OS 交互经 `IoWorkers`（4 条有界 channel + 2 个 worker 线程），`provider_result` 已把 `HostStreamError::Io` 映射为 `ProviderFault` → `HostProviderFailure`；**但没有任何既有测试经真实 run 触发过 `HostProviderFailure`**（只有 marker 级单测 `provider_transport_failure_is_typed_not_text_classified` 与 `ObservedRun` 分类矩阵测试）。
- `Runner` 无任何 provider 注入点：`run_unchecked` / `start_debug` 直接 `spawn_io_workers()` 硬编码真实 stdio。
- 资源耗尽类（`run_program_with_budgets` 窄化后限额）已在 STEP-0578 证明为 typed `MemoryLimit`/`FuelExhausted`/`Timeout` 且 `memory_budget_is_enforced_and_host_stays_reusable` 已断言宿主复用——但用的是**同一 guest 的无预算对照**，不是「随后跑一个**正常** guest 完整完成」。
- 结论：缺口 = ①确定性可注入 seam；②经真实 run 触发 `HostProviderFailure` 并断言宿主复用；③「正常 guest 完整完成」作为宿主可复用的更强判据（不是进程苟活但 Store 已废）。

## 3. 实现

### 3.1 可注入 fault seam（`runner/sico-runner/src/stream_provider.rs`，新模块）

- `pub trait StreamTransport: Send + 'static`：`read_stdin` / `write_output` / `flush_output` / `try_flush_output`（RFC-0030 流传输的窄接口，类型化、`&self`）。
- `pub trait StreamTransportFactory: Send + Sync + 'static`：`spawn_transport(&self) -> Box<dyn StreamTransport>`——每 Store 一个传输的工厂，即注入点。
- **不是生产后门**：默认 `Runner::new()` 与所有既有构造器不变，恒走 `RealStreamTransportFactory`（真实 stdio）；故障实现完全落在**测试**（`provider_crash_isolation.rs`），生产 crate 不编译任何 fault 分支。注入经 `Runner::with_stream_transport(Arc<dyn StreamTransportFactory>)` builder（镜像既有 `with_ui_backend` 的 DI 模式）。

### 3.2 接线（`runner/sico-runner/src/lib.rs`）

- `RunState.io` 由 `IoWorkers` 改为 `Box<dyn StreamTransport>`（每 Store 一个传输，由 `self.runner.stream_factory.spawn_transport()` 构造）；`io: IoWorkers` 保留为真实实现的内部态，`impl StreamTransport for IoWorkers` 委托 `send_cancellable`/`recv_cancellable`（行为逐字节不变）。
- 流 provider 5 处闭包（read / write / flush / pump / drop-flush）改走 trait；scheduler 操作、terminal 记账、`top_up_stream_budgets`、fuel 顶部、payload 统计**原样保留**。
- `Runner` 增加 `stream_factory: Arc<dyn StreamTransportFactory>`（默认 `RealStreamTransportFactory`）；`HostStreamError` 由 `pub(crate)` 抬为 `pub`（seam 暴露 provider 的 typed fault 类型，纯加宽无行为变化）。
- `crates/sico-package` 与 `crates/sico-ui-renderer-d2d`、`ui.rs`、`ui_authority.rs` **零改动**。

## 4. 测试（`runner/sico-runner/tests/provider_crash_isolation.rs`，4 测试，单线程）

覆盖矩阵（任务书要求的两类 + 正控制）：

1. **资源耗尽**（①类，与 `run_program_with_budgets` 窄化后限额一致）：冻结 fixture 预算（1 MiB）窄化 → 手编最小 guest 越过 1 MiB 内存 → typed `MemoryLimit`；随后**同一 runner** 跑前端编译的**正常** guest（`plain_source`，默认限额）→ `Output`（宿主可复用，非「苟活但 Store 已废」）。
2. **provider 调用返回错误**（②类）：注入 `FailingWriteFactory`（`write_output` → `Err(HostStreamError::Io)`）→ 前端编译的 writer guest（`sico.stream.write`）→ typed `HostProviderFailure { provider_id: "sico.streams.stdout" }`；随后**同一 runner** 跑正常 guest → `Output`。
3. **provider 内部 panic 被拦截**（②类）：注入 `PanickingWriteFactory`（`write_output` → `panic!`）→ writer guest → 被拦截（wasmtime `catch_unwind_and_record_trap`，或 runner guest-worker `join` 回退 → `Trap("guest worker panicked")`）→ `assert_typed_failure`（成员 of typed 家族、**非** `Output`）；随后**同一 runner** 跑正常 guest → `Output`。
4. **正控制**：默认 `Runner::new()` 跑 writer guest → `Output`（seam 未改变默认真实 stdio 传输）。

`writer_source` / `plain_source` 为真实 `.sico`（复用 STEP-0578 的 Script prelude），经 `sico_cli::run(["sico","build","--profile","script-v0",...])` 编译——provider 调用是真实编译器产物，非手编伪造。`grow_past_one_mib` 复刻 STEP-0578 手编最小 guest（wasm-encoder + `wrap_script_component`）。

## 5. 结果（实测，本机 Windows x64，RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu）

- `cargo test --offline --manifest-path runner/sico-runner/Cargo.toml --test provider_crash_isolation -- --test-threads=1` → **4 passed / 0 failed**（复跑 3 次均 4/4）。
- `cargo test --offline --manifest-path runner/sico-runner/Cargo.toml --test provenance_runtime -- --test-threads=1` → **6 passed / 0 failed**（无回归）。
- `cargo test --offline -p sico-package` → **51 passed / 0 failed**（18 unit + 1 binary_carrying + 15 package + 3 provenance_budget + 6 provenance_fixture + 4 provenance_install + 4 security_properties）。
- runner `cargo clippy --offline --lib -- -D warnings` 与 `--test provider_crash_isolation -- -D warnings` 均绿；本片涉及文件 `cargo fmt --check` 净（`crates/sico-package/tests/provenance_budget.rs`/`provenance_install.rs` 的既有 fmt 差异为 STEP-0578 未收口项，**非本片文件**，未动）。
- `git diff --check` 净。

## 6. 诚实收窄 / 已知边界

- **gate 2 不翻**：本片只交付第 5 件的 internal-fixture 证据。gate 2 整体 GO 仍需出口审计：①C1/C2 合同缺口（STEP-0578 §6，hostcall 预算 64 MiB 地板与真实 Script 组件 ~2 GiB 内存下 1 MiB 预算不可满足）仍待 owner 裁决；②其余四件与五件的证据口径、证据等级（internal-fixture 不互换 clean-room/external-pilot/production）由审计统一判读。
- **只覆盖 `sico.streams.*` 传输**：seam 注入点是流 provider 的传输（RFC-0030）。其它 provider（http、fs、ui）的内部崩溃走各自的 typed 域/launch outcome；本片**不**为它们逐一建 seam（最小暴露原则），只证明「provider 崩溃 → 宿主存活 + Store 可复用 + typed outcome」的**机制**成立且与 provider 无关（经流 provider 验证）。
- **panic 拦截路径**：`PanickingWriteFactory` 的 panic 由 wasmtime `catch_unwind_and_record_trap` 拦截为 trap（typed `Trap`），或由 runner 的 `thread::scope` + `join` 回退为 `Trap("guest worker panicked")`——两者均为 typed 且宿主存活；实测该测试通过（`assert_typed_failure` 接受任一 typed 家族成员，不硬编码具体变体）。
- **宿主可复用判据**：以「同一 `Runner` 随后跑**正常** guest 完整完成（`Output`）」为准，较 STEP-0578 的「同 guest 无预算对照」更强。
- **间歇性进程退出码 0xc0000005**：runner 测试二进制**偶发**在全部测试通过后、进程 teardown 时以 `STATUS_ACCESS_VIOLATION` 退出（在**未触碰**的 `control_flow` 上也复现过一次，随后 3 次运行干净）。判定为**本机既有环境 flake，与本片无关**；本片 4 测试 + `provenance_runtime` 6 测试多次复跑均为「test result: ok」且多数运行 exit 0。已如实记录，不掩盖、不追认。

## 7. gate 2 五件套完成度（逐条）

| # | 件 | 状态 | 依据 |
|---|---|---|---|
| 1 | digest-verified load | ✓ | STEP-0334（`provenance.rs` `verify_asset`/`verify_provenance_in_package` + 冻结 fixture 逐字节） |
| 2 | budget 强制（limit+1） | ✓（C1/C2 合同缺口待 owner） | STEP-0578（`narrowed_by` 纯收窄 + `run_program_with_budgets` 运行期强制；limit+1 / over-ceiling clamp 已测；C1/C2 缺口不自行裁定） |
| 3 | 取消（中途） | ✓ | STEP-0578 property 4（`explicit_mid_flight_cancel_is_a_typed_cancellation` → `Cancelled`，M9） |
| 4 | provider-crash 隔离（免宿主重启） | ✓（本片） | STEP-0587：资源耗尽 → `MemoryLimit` + 宿主复用；provider 错误 → `HostProviderFailure` + 宿主复用；provider panic 拦截 → typed trap + 宿主复用（4/4 绿） |
| 5 | 结构/绑定校验 | ✓ | STEP-0334（`verify_binding` + 组件结构/身份校验，`security_properties` 4 测） |

**完成度声明**：五件逐条均已有 internal-fixture 证据；但这是**完成度**，不是 gate GO。gate 2 的最终裁决（含 C1/C2、各件证据等级与口径、五件间的一致性）属出口审计与 owner（AGENTS §3 证据纪律：internal-fixture ≠ clean-room/external-pilot/production，仅 claim runtime 支持于实际执行的 native/browser runner）。

## 8. 建议主线审核点

1. **seam 是 DI 不是后门**：`with_stream_transport` 只改构造参数，默认不变；生产 crate 无任何 fault 分支（fault 实现在测试）。请复核 `stream_provider.rs` 模块注释与 `Runner::with_stream_transport` doc。
2. **既有语义未变**：流 provider 闭包改走 trait 后，`IoWorkers` 委托原 `send_cancellable`/`recv_cancellable`/`try_send`——scheduler/terminal/top_up/payload 统计逐行保留；`provenance_runtime` 6/6 与 `sico-package` 51/51 无回归。
3. **`HostStreamError` 抬为 `pub`**：纯加宽，但属公开 API 面扩大；若 owner 认为过宽，可改回并给 seam 一个私有映射（不影响本片测试）。
4. **gate 2 五件套**：已按 §7 逐条标 ✓；下一步是 owner/出口审计对五件 + C1/C2 + 证据等级做整体裁决，**不应**由任何单片自行宣告 gate 2 GO。

## 9. 文件清单（本片）

- 新增：`runner/sico-runner/src/stream_provider.rs`、`runner/sico-runner/tests/provider_crash_isolation.rs`、`docs/steps/STEP-0587-m24-gate2-provider-crash-isolation.md`。
- 修改：`runner/sico-runner/src/lib.rs`（`RunState.io` 类型、`Runner` 字段 + `with_stream_transport`、`HostStreamError` 可见性、流 provider 5 处闭包、`impl StreamTransport for IoWorkers` + `RealStreamTransportFactory`）。
- 未动（复核确认）：`crates/sico-package/src/provenance.rs`、`crates/sico-package/tests/*`、`crates/sico-ui-renderer-d2d/**`、`runner/sico-runner/src/ui.rs`、`runner/sico-runner/tests/ui_authority.rs`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`docs/steps/README.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、`docs/plans/**`。
