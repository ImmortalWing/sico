# STEP-0593: M24 gate 3 registry 半部 — runner 包实例 table 限额与 instances/memories 同形修正

> - status: **实现 + 验证完成；gate 3 registry 消费半部（STEP-0588 ②）证据闭合；§3 gate 3 聚合裁决仍属出口审计，不翻转**
> - phase: M24 §3 gate 3 / §8.3 确定性视觉包 roster（三包：image-match / image-grid / image-contour）与 `image-codec@1`（M24 #2）；解除 STEP-0588 §Blocker
> - completed: 2026-11-07
> - contracts: [`RFC-0043`](../rfc/RFC-0043-deterministic-vision-packages-v0.md) §Summary（[A1](../rfc/RFC-0043-amendment-A1-region-packed-bytes.md)/[A2](../rfc/RFC-0043-amendment-A2-bitmap-input-serialization.md)）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)、[`RFC-0039`](../rfc/RFC-0039-package-consumer-bridge-v0.md) §2.3/§2.4（M7 registry 路径）、[`RFC-0050`](../rfc/RFC-0050-image-codec-package-v0.md)（image-codec@1）
> - evidence: ① runner 限额语义 = **contract-verified**（wasmtime 47.0.2 源码 `bump_resource_counts`/`limit_new`）；② registry 消费 = **internal-fixture**（package/Component 级）且 **runtime-verified**（11/11 在已执行 runner 上逐字节断言）；③ 包核 table 计数 = **internal-fixture**（四份 core `.wasm` 二进制解析）；**不是**脚本级、**不是**试点级
> - pre-existing dirty worktree: `runner/sico-runner/src/lib.rs`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/**` 等由**并行代理**修改（STEP-0587 / STEP-0592 D2D 布局 / 包侧改动）；本片**只**在 `lib.rs` 限额段做精确 Edit，未触碰任何并发文件，未触碰冻结语料与 digest-pinned core `.wasm`

## Objective

解除 STEP-0588 §Blocker：runner 的 `ResourceLimiter` 对 run Store **硬编码 `fn tables() -> usize { 0 }`**（旧 `lib.rs:1428`）且 `table_growing` 恒返 `Ok(false)`（旧 `lib.rs:1412`），导致 ADR-0019 Rust 编译包核（class (b)，各带一个 wasm function table）在 `instantiate_packages` 阶段即被拒（`table count too high at 1`），registry 路径因此**只支持无 table 的包**（手编 `csv` core 能绿，roster 三包与 `image-codec@1` 全红）。本片把 `tables()`（及 `table_growing`）修正为与既有 `instances()`/`memories()` **同形**、按 `package_instances_capacity` **有界伸缩**，使 roster 三包与 `image-codec@1` 沿 M7 registry 路径可实例化、被真实 consumer 消费。**零 compiler/Runtime 语义改动**（包仍是已编译 Component，消费仍即实例化；只调限额段）。证据等级 internal-fixture + runtime-verified，不翻任何 §3 gate。

## Root cause（STEP-0588 §Blocker，本片复核确认并补强）

- **事实**：`ResourceLimiter`（`runner/sico-runner/src/lib.rs`，`impl ResourceLimiter for RunState`）三个上限方法不对称——`instances()` = `2 + package_instances_capacity`（`lib.rs:1435`）、`memories()` = `1 + package_instances_capacity`（`lib.rs:1454`），而 `tables()` = `0`（旧 `lib.rs:1428`）。
- **wasmtime 计数语义（contract-verified，wasmtime 47.0.2 源码）**：`StoreOpaque::bump_resource_counts`（`src/runtime/store.rs:1507-1531`）在**每个 core 模块实例化**时累计 `+1 instance`、`+num_defined_memories()`、`+num_defined_tables()`，并分别对 `(l.instances(), l.tables(), l.memories())`（`store.rs:938`，即 `ResourceLimiter` 的三个上限）做 `new > max ⇒ bail("resource limit exceeded: {desc} count too high at {new}")`。故 `tables()` 是**每 Store 累计 table 数上限**，不是单实例上限。
- **包核 table 计数（internal-fixture，四份 core `.wasm` 二进制解析）**：`image-{match,grid,contour}-core.wasm` 各 **1 个 defined funcref table**，`min == max == 18`；`image-codec-core.wasm` **1 个 defined funcref table**，`min == max == 71`。即每包恰好 1 个定长 table。STEP-0588 的失败报错 `table count too high at 1` 正是首个包实例化时 `0 + 1 > 0` 的运行时证据。
- **判读**：`tables() = 0` 是 **committed 状态**（非并行代理在途改动），属**先例性 latent gap**——registry 路径（STEP-0147 建立时）假定包无 table；ADR-0019 class-b 包核（STEP-0331 codec、STEP-0335/0336/0337 roster）后到、各带一个定长 table；`instances()`/`memories()` 已按包容量伸缩，`tables()` 漏了。**同一缺口同时阻断 `image-codec@1`（STEP-0331）与 roster 三包。**

## 推导 — table 预算的正确取值

**程序/传输侧 = 0 table。** 程序 core（Sico codegen 输出）与 STEP-0083 fs transport 模块都是手写/表无关 core：committed `tables() = 0` 下**所有非包 run 全绿**（M14–M21 全套、runner `--lib` 42/42、csv 消费 2/2），若任一侧定义 table，这些 run 都会以 `table count too high` 失败。故程序 + 传输侧 defined table 数为 **0**。

**每包 = 1 table。** ADR-0019 class-b 包核各 1 个 defined funcref table（上节实测）。

**合计（`bump_resource_counts` 逐模块累计）**：`0 + 1 × package_instances_capacity = package_instances_capacity`。

> **结论：`tables() = package_instances_capacity`。**
> - **上界依据**：`package_instances_capacity` 在 prepare 时被固定（`instantiate_packages` 开头 `store.data_mut().package_instances_capacity = self.packages.len()`，`lib.rs:2147`），**有界**（不是无限、不是与包容量无关的固定大值）；与 `instances()`/`memories()` 的 `+ capacity` 项**同形**（base 项分别为 2/1/0，正是程序+传输侧的实例/内存/table 数：2 个 core 实例、1 块内存、0 张 table——三者**彼此一致**）。
> - **不取 `1 + capacity`**：程序/传输侧实测 0 table，`+1` 会是一个未证实的宽松余量；且 base 项 0 与 `instances()` 的 base 2、`memories()` 的 base 1 在「程序+传输侧贡献」上自洽。取精确值，未来若程序侧出现 table 会在实例化时**清晰失败**（`table count too high`），不静默。

## `table_growing` 是否需要改 — 需要，且已改

- **wasmtime 语义（contract-verified）**：`table_growing` **不仅在运行时 `table.grow` 调用**（`src/runtime/vm/table.rs:650`），**在 table 创建时也调用**——`Table::limit_new`（`table.rs:443-487`）在实例化分配 table 前调 `table_growing(0, minimum, maximum)`（`table.rs:468`）。若返回 `Ok(false)`，创建即 `bail!("table minimum size … exceeds table limits")`。
- **旧实现为什么错**：`table_growing` 恒 `Ok(false)` + `denied = true`。包核 table 创建时 `(0, 18, 18)` 被拒 → 实例化 bail → `run_unchecked` 的错误处理看到 `denied == true` → 把 Launch 错误**改写为 `RunOutcome::MemoryLimit`**（`lib.rs:2528-2539`）。这正是 STEP-0588 7 个 happy-path 在 table 限额放开后从 `Launch` 变 `MemoryLimit` 的原因——**不是 memory 超限，是 table 创建被 `table_growing` 拒绝**。
- **修正**：`table_growing` 允许 `desired <= maximum`（`maximum` 为模块声明上限，wasmtime 在回调**之后**仍会自查该上限）；`maximum == None`（无声明上限）⇒ 拒绝，**不引入无界 table 路径**；拒绝时保持 `denied = true`（fail-closed 姿态，与 `memory_growing` 超限时置位一致）。ADR-0019 包核 table 均为定长（`min == max`），`desired = min ≤ max` 恒成立 ⇒ 创建放行；运行时 `table.grow` 语义上不可能（`min == max`），故该回调对包核只会在创建时触发一次。

## Implementation（`runner/sico-runner/src/lib.rs` 限额段，两处精确 Edit）

| 处 | 旧 | 新 | 行号（当前快照） |
|---|---|---|---|
| `tables()` | `0` | `self.package_instances_capacity`（注释：程序/传输 0 table + 每包 1 table；上界 = prepare 时固定容量；与 `instances()`/`memories()` 同形） | `lib.rs:1441-1452` |
| `table_growing` | `self.denied = true; Ok(false)` | `let allowed = maximum.is_some_and(\|max\| desired <= max); if !allowed { self.denied = true; } Ok(allowed)`（注释：创建 + 运行时 grow 均经此；定长 table 仅创建触发；模块声明上限为权威有界上界；拒绝无上限 table） | `lib.rs:1412-1433` |

**并发安全**：两处均为**精确 Edit**（非整文件 Write）；编辑前重新读取目标区（行号与并发代理 STEP-0587 中间态核对一致）；编辑前后核对并发代理产物仍在（`stream_provider.rs` 存在且 `StreamTransportFactory` 引用 10 处）；`lib.rs` 其余部分（ui.rs / layout.rs 等）未触碰。与并发改动**无冲突**（不同文件、不同段）。

## Executed validation

工具链 `1.98.0-x86_64-pc-windows-gnu`，`--offline`，`--locked`，`--test-threads=1`（runner 侧）。**编译面曾短暂被并行代理 STEP-0592 在途改动阻塞**（`crates/sico-ui-renderer-d2d/src/layout.rs` 内属性位置中间态，`lib.rs` 无关），代理修复后编译面恢复——与 STEP-0588 记录的先例性短暂阻塞同性质。

| 验证 | 命令 | 结果 |
|---|---|---|
| STEP-0588 主验证 | `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_roster_consumer -- --test-threads=1` | **11 passed; 0 failed**（7 happy-path + 4 fail-closed） |
| 逐包消费（happy-path） | 同上一并覆盖 | `image_match_template`（16B A1 `03000000020000000300000002000000`）、`image_match_error_case_name`（`unsupported-format`）、`image_grid_detect_lines`（count-prefix `0100000000000000000000000300000001000000`）、`image_grid_error_case_name`（`unsupported-format`）、`image_contour_find`（count-prefix `0100000001000000010000000300000003000000`）、`image_contour_error_case_name`（`unsupported-format`）、`roster_consumer_run_is_repeatable`（跨 Store 字节稳定）——**全部 ok**，断言字节与冻结语料一致 |
| 回归（csv 先例） | `--test package_consumer_forward` | **2 passed; 0 failed** |
| 回归（fail-closed） | `--test package_consumer_authority` | **3 passed; 0 failed** |
| 回归（包侧） | `cargo test --locked --offline -p sico-package` | **51 passed; 0 failed**（lib 18 + binary_carrying 1 + package 15 + provenance_budget 3 + provenance_fixture 6 + provenance_install 4 + security_properties 4 + doc 0） |
| 回归（runner lib） | `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --lib` | **42 passed; 0 failed** |
| 全二进制编译 | `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --no-run` | exit 0（全部测试二进制编译） |
| clippy（本片范围） | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --lib -- -D warnings` | **OK** |
| clippy（测试） | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_roster_consumer -- -D warnings` | **OK** |
| fmt（本片范围） | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml -p sico-runner -- --check` | **exit 0（净）** |
| `git diff --check` | 本片两个文件 | **净** |

**说明**：`cargo fmt --all --manifest-path runner/sico-runner/Cargo.toml -- --check`（全 workspace）**失败**——但失败完全在 `crates/sico-package/tests/provenance_budget.rs`（fmt diff 0 处提及 `lib.rs`/本片行；该文件为 root workspace 文件、**禁动**（`crates/sico-package/**`）且非本片所改）。`-p sico-runner` 包级 fmt 净（exit 0）即证明本片改动 fmt-clean。

## 对 `image-codec@1`（STEP-0331）的影响评估

- **预期受益方**：`image-codec-core.wasm` 实测 1 个 defined funcref table（`min == max == 71`），与 roster 同属 ADR-0019 class-b。本修正后它同样可沿 M7 registry 路径实例化（此前被同一 `tables() = 0` 阻塞）。
- **未在本片补断言**（如实登记，待后续片验证）：`image-codec@1` 导出 `decode-png(Bytes) -> Result[Bytes, Text]`（**1 参**）与 `encode-png(Bytes, U64, U64) -> Result[Bytes, Text]`（**3 参**），与 STEP-0588 测试辅助 `RosterCall`/`guest_module` 硬编码的 **2 参** roster 形不同，且输入为 PNG 文件（`tests/corpus/image-codec/*.png`，非 A2 packed header）——补断言需扩展测试基础设施（新参数元数 + 文件输入读取），超出「仅在必要时微调断言」的边界。**登记为「修正的预期受益方，待后续片验证」**；后续片可直接用冻结用例 `decode-ok-2x2-rgba`（期望 `payload_hex = 020000000000000002000000000000001e140aff3c3228805a504600786e6440`）作起点。
- **`denied = true` 姿态**：本修正保持拒绝时置位 `denied`（与 `memory_growing` 一致）；对定长 table（`min == max`）运行时 grow 语义上不可能，故对 ADR-0019 包核无行为差异。

## Honest residuals / 诚实收窄

- **gate 3 不翻转**：本片解除 registry 半部（STEP-0588 ②）的唯一阻塞点；gate 3 的聚合 GO/NO-GO 裁决属**出口审计**（M24 plan §3、§8.3）。corpora 半部（STEP-0335/0336/0337，3/3）与本 registry 消费半部**分开记录**，不得把任一全绿当 gate 3 整体 GO。
- **registry 消费 = internal-fixture**（package/Component 级，runtime-verified at 已执行 runner）；不是 clean-room-consumer、不是 external-pilot。`image-codec@1` 的 registry 消费未在本片单独演示（见上节）。
- **零 compile 声明边界**：消费路径不触发 Rust 侧重新编译（包是已编译 Component，`instantiate_packages` 仅实例化）；本片**未运行**任何构建工具链重建 core `.wasm`（四份 core 为提交件、digest-pinned，仅做只读二进制解析）。
- **`tables()` 精确值假设**：依赖「每 ADR-0019 class-b 包核恰 1 个 defined table」与「程序/传输侧 0 table」。前者对当前四份 core 实测成立；后者由 committed `tables()=0` 下全套非包 run 全绿佐证。若未来出现「单包多 table」或「程序侧带 table」的包/程序，会在实例化时**清晰失败**（`table count too high`）而非静默——需另行修订推导，但**不静默降级**。
- **`table_growing` 对无上限 table 拒绝**：`maximum == None` ⇒ 拒绝（不引入无界路径）；当前语料无此类 table，属 fail-closed 前摄。
- **`denied → MemoryLimit` 语义**：table 拒绝（含运行时 grow 超限）会以 `MemoryLimit` 结束 run（旧行为一致），而非 wasm 语义的 `table.grow` 返回 -1；对定长 table 无实际触发面。

## Gate 3 两半部完成度（分开记）

| 半部 | 状态 | 证据入口 |
|---|---|---|
| ① corpora：三包 deterministic 字节精确 vs 独立 oracle + RFC-0041 可达 `image-error` 闭集 | **完成 3/3** | STEP-0335/0336/0337 + `tests/image_{match,grid,contour}_package.rs` + `tests/corpus/{image-match,image-grid,image-contour}/manifest.json` + `tools/validate-package-core.ps1` |
| ② consumers 经 M7 registry 路径安装 + 消费演示 | **完成 11/11**（fail-closed 4/4 + happy-path 7/7，逐字节 vs 冻结语料）；本片解除唯一阻塞点 | `tests/package_roster_consumer.rs`（11/11 绿，本片复跑）+ 本 STEP |

**gate 3 registry 半部（②）闭合结论**：STEP-0588 的唯一具名阻塞（runner `tables()=0` + `table_growing` 拒绝 table 创建）已修正并验证；**registry 消费证据（含 fail-closed 面）完整、全绿**。gate 3 **整体**是否 GO 仍由出口审计聚合裁决（含 corpora 半部、`image-codec@1` registry 消费待补、以及 §3 其余 gate），本片不越权判定。

## Files touched

- `runner/sico-runner/src/lib.rs`（**限额段**，两处精确 Edit：`tables()` `lib.rs:1441-1452`、`table_growing` `lib.rs:1412-1433`；`instances()`/`memories()`/其余未动）
- `docs/steps/STEP-0593-m24-registry-table-budget-fix.md`（本文件）

未触碰：`runner/sico-runner/src/ui.rs`、`runner/sico-runner/tests/ui_authority.rs`、`runner/sico-runner/tests/provider_crash_isolation.rs`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/**`、`wit/**`、`selfhost/`、`guests/**` core `.wasm`（digest-pinned）、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、冻结语料与 `tests/corpus/**`。

**回归基线确认**：本片改前，`package_roster_consumer.rs` 的 7 个 happy-path 全红（`table count too high at 1` → 后表修正为 `MemoryLimit`）、4 个 fail-closed 全绿（STEP-0588）；改后 11/11 绿。csv 先例（forward 2/2、authority 3/3）改前改后均绿。
