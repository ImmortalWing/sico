# STEP-0588: M24 gate 3 后半部 — roster 三包经 M7 registry 路径安装 + 真实 consumer 消费演示

> - status: **实现切片 + fail-closed 半部完成；happy-path 消费证据被 runner 侧 table 限额阻塞（具名缺口登记，未移动 §3 gate 3）**
> - phase: M24 §3 gate 3 / §8.3 确定性视觉包 roster（三包：image-match / image-grid / image-contour）；对应 STEP-0586 盘点 **I4**
> - completed: 2026-11-07
> - contracts: [`RFC-0043`](../rfc/RFC-0043-deterministic-vision-packages-v0.md) §Summary（结果由 [`Amendment A1`](../rfc/RFC-0043-amendment-A1-region-packed-bytes.md) 冻结、输入由 [`Amendment A2`](../rfc/RFC-0043-amendment-A2-bitmap-input-serialization.md) 冻结）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)、[`RFC-0041`](../rfc/RFC-0041-image-data-contract-v0.md)、[`RFC-0039`](../rfc/RFC-0039-package-consumer-bridge-v0.md) §2.3/§2.4（M7 registry 路径 / package consumer 桥）
> - evidence: internal-fixture（package/Component 级）；**fail-closed 面 runtime-verified**（4/4 绿）；**happy-path 消费未被验证**（阻塞，见 §Blocker）；**不是**脚本级、**不是**试点级；与 corpora 半部（STEP-0335/0336/0337）**分开记录**
> - pre-existing dirty worktree: `runner/sico-runner/src/lib.rs`、`crates/sico-ui-renderer-d2d/**` 等由**并行代理**修改（STEP-0587 / 窗口壳与 D2D），本片**未触碰**；本片只新增 `runner/sico-runner/tests/package_roster_consumer.rs` 与 `docs/steps/STEP-0588-*.md`

## Objective

补齐 M24 §3 gate 3 的**后半部**（STEP-0586 盘点 I4「roster 三包自身的 M7 安装/消费路径未单独演示」）：三包（`image-match@1` / `image-grid@1` / `image-contour@1`）沿 **M7 registry 安装路径**（`prepare_package` → `link_packages` → `instantiate_packages`，即 RFC-0039 §2.3/§2.4 的 consumer 桥，真实 `sico run` CLI 依赖的同一路径）被安装，并被一个**真实 consumer**（手编 `wasm-encoder` Program 组件，导入对应包导出并取回真实字节）消费，断言结果与各自冻结语料的期望字节一致；同时按既有 `package_consumer_authority.rs` 语义补齐三包的 fail-closed 拒绝面。**零 compiler/Runtime 改动**（包是已编译 Component，消费即实例化）。证据等级 **internal-fixture**，不翻任何 §3 gate。**结果**：fail-closed 面全绿；happy-path 消费被 runner 侧 `ResourceLimiter::tables() = 0` 阻塞（ADR-0019 Rust 编译 core 带 wasm table），具名缺口见 §Blocker。

## Contract background（为何必须走 registry 路径）

§8.3 明文要求「consumers install through the M7 registry path with zero compiler/Runtime changes」。既有三包内容测试（`image_match_package.rs` / `image_grid_package.rs` / `image_contour_package.rs`，STEP-0335/0336/0337）用**空 linker 直接 `Component::new` + `Linker::instantiate`** 调包导出——那是内容语料证据，**不是**「经 M7 registry 路径安装 + 被 consumer 消费」的 registry 路径证据。registry 路径已在 runner 边界证明（`package_consumer_authority.rs` 3/3 fail-closed + `package_consumer_forward.rs` 2/2 happy-path，但用 `csv@1.0.0`）；本片把**同一路径**套到 roster 三包本体上，逐包单独演示安装 + 消费。

## Reconnaissance — M7 registry 安装路径的确切调用序列

当前快照（2026-11-07 worktree，HEAD `7bcdc35`；`runner/sico-runner/src/lib.rs` 由并行代理 STEP-0587 修改中，行号对该快照成立，开工前须重新核对）：

| 步 | 函数 | 位置 | 作用 |
|---|---|---|---|
| 1 | `Runner::prepare_program_with_packages` | `runner/sico-runner/src/lib.rs:1747` | 入口：consumer 组件 + 包列表 → `prepare_program_inner` |
| 2 | `prepare_program_inner` | `lib.rs:1864` | `compile_cached(consumer)`；**`prepare_package(&package.bytes)` 每包**（1809-1812）；建 `Linker`；`link_fs`/`link_streams`/`link_http`/`link_http2`/`link_ui`；**`link_packages`** |
| 3 | `prepare_package` | `lib.rs:1940` | 编译包 Component，**纯性检查**（自身 import ⇒ typed refusal），冻结 `sico:user/…` 实例导出身份 + 每函数 `(params, results)` 元数 |
| 4 | `link_packages` | `lib.rs:2008` | 收集 consumer 的 `sico:user/…` 导入；按名匹配 prepared 包（无匹配 ⇒ "no prepared package exports it"）；**arity 不符 ⇒ refusal**；注册 `func_new` 动态 `Val` 桥到包实例导出 |
| 5 | `PreparedProgram::run` | `lib.rs:2369` | → `run_unchecked`（`lib.rs:2426`） |
| 6 | `run_unchecked` | `lib.rs:2426` | 新建 Store；**`instantiate_packages(&mut store)`**（2436）；`linker.instantiate`（2441）；`invoke` |
| 7 | `instantiate_packages` | `lib.rs:2122` | 每个 prepared 包以**空 linker** `instantiate` 进 Store（纯组件），实例存 `RunState.package_instances`，随 Store 消亡 |

**结论**：registry 路径 = `prepare_program_with_packages` → `prepare_package`（纯性 + 导出身份冻结）→ `link_packages`（导入匹配 + arity 门 + `Val` 桥注册）→ `.run` → `instantiate_packages`（实例化进 Store）→ 桥转发 `Val` → 包导出。本片**必须**走这条路径（而不是 `prepare_package` 空 linker 绕过）。`sico:script/types` 是 **type-only 导入**（只导出类型、无运行时值），wasmtime 自动满足，无需 runner 链接——与 `package_consumer_forward.rs` 相同。

**与三包相关的既有能力**：`package_builder::image_{match,grid,contour}_package()`（`crates/sico-codegen-wasm/src/package_builder.rs:2110/2135/2160`）各自嵌入 digest-pinned core `.wasm` 并包装成纯 Component；导出接口 `sico:user/image-{match,grid,contour}@1.0.0`；导出函数 `template`/`detect-lines`/`find`，签名 `func(Bytes[, U64]) -> Result[Bytes, Text]`（match 两 Bytes、grid/contour 一 Bytes + U64）。**缺口**：无任何测试沿 registry 路径消费这三包（`image_match_package.rs` 等用空 linker 直调）。

## Implementation

新增 `runner/sico-runner/tests/package_roster_consumer.rs`（形态镜像 `package_consumer_forward.rs`，三个包共用一套参数化 builder）：

- `RosterCall` 描述一次 registry 消费：interface / function / 静态输入字节（bmp，[tpl]）/ `min_value: Option<u64>` / `expected_disc: u8`。
- `consumer_component`：手编 `wasm-encoder` Program 组件——导入 `sico:script/types`（type-only）+ 包接口实例；`lower_func` 把包导出降到共享 transport（memory + checked bump realloc）；guest core 模块调用降级函数并读取结果区。
- **canonical ABI**：`result<list<u8>, string>` 展平 >1 core 值 ⇒ 返回区（尾随 i32 retptr）形；结果区布局 `disc@0 / ptr@4 / len@8`（变体两臂同 flat 形，payload 对齐 4）；guest 读 `disc`、`ptr`、`len`。
- **guest 逻辑**：转发静态输入 → `Call(1)`（降级包导出）→ 若 `disc == expected_disc` 则 stdout = 负载字节（`ptr/len`）、exit=0，否则 stdout 空、exit=1000+disc（非零，绝不部分调用）。script-result 写 `disc@0 / stdout@8-12 / stderr@16-20 / exit@24`。
- **零 compile 断言**：消费路径只调用 `image_{match,grid,contour}_package()`（`include_bytes!` 嵌入已编译 core）与 `prepare_program_with_packages` + `.run`；包是已编译 Component，`instantiate_packages` 仅实例化进 Store，**不触发任何 Rust 侧重新编译**（与 `csv` 先例一致；测试不调用任何 build 工具链，core `.wasm` 为提交件、digest-pinned）。

### 逐包 registry 安装 + 消费证据（断言字节）

| 包 | 导出 | 冻结语料用例（`tests/corpus/*/manifest.json`） | 输入 | 期望 stdout（断言） |
|---|---|---|---|---|
| `image-match@1` | `template` | `grey-exact-offset`（ok） | bmp `0800000006…8e7`、tpl `030000000200000001000000c7e60a01203f` | region `03000000020000000300000002000000`（16B A1） |
| `image-match@1` | `template` | `bad-format-tag`（err） | bmp `040000000400000007000000`、tpl 同上 | case 名 `unsupported-format`（Text） |
| `image-grid@1` | `detect-lines` | `single-row-min3`（ok） | bmp `060000000100000001000000ffffff00ffff`、min=3 | count-prefix `0100000000000000000000000300000001000000`（4B+16B） |
| `image-grid@1` | `detect-lines` | `bad-format-tag`（err） | bmp `040000000400000007000000`、min=2 | `unsupported-format` |
| `image-contour@1` | `find` | `single-blob-3x3`（ok） | bmp `050000000500000001000000000000000000ffffff0000ffffff0000ffffff000000000000`、min=1 | count-prefix `0100000001000000010000000300000003000000` |
| `image-contour@1` | `find` | `bad-format-tag`（err） | bmp `040000000400000007000000`、min=2 | `unsupported-format` |

（六条断言字节全部取自各自冻结 manifest 的 `region_hex` / `list_hex` / `case`；ok 臂验证**内容**而非仅计数。另含 `roster_consumer_run_is_repeatable`：同一 Program + 包跨 Store 字节稳定。）

### fail-closed 覆盖矩阵（registry 路径）

| 拒绝面 | 断言 | 覆盖包 |
|---|---|---|
| 未备包导入（无匹配 prepared 包） | `no prepared package exports it` | 三包 |
| 参数元数不符（1 参声明 vs 2 参真实） | `arity mismatch` | 三包 |
| 同 arity 类型不符（`(list<u8>, u64)` vs `(list<u8>, list<u8>)`） | 链接期接受、调用期 typed 失败、**不落部分调用**（`!matches!(Output)`） | `image-match@1` |
| 非纯包（包自身 import） | `must not import` | `image-match@1` |

说明：registry 桥在链接期只校验 **arity**（`link_packages` 比较 `(params.len(), results.len())`），同 arity 类型不符在**调用期**以 typed 失败关闭（`func.call` 对包导出实型的 `Val` 校验），不会产出部分/错误字节。

## Executed validation

工具链 `1.98.0-x86_64-pc-windows-gnu`，`--offline`，`--test-threads=1`。编译面被并行代理 STEP-0587/窗口壳在途改动**短暂阻塞**（`dyn StreamTransportFactory` 缺 `Sync`；`shell/win.rs` `BeginPaint` 返回 `HDC` 却以 `Ok/Err` 匹配——二者皆为**其他代理**中间态），代理于 10:45 修复后编译面恢复。

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_roster_consumer -- --test-threads=1` → **4 passed; 7 failed**（11 测；退出时 STATUS_ACCESS_VIOLATION 0xc0000005，**先例性 Windows 退出崩溃**，非测试失败）。4 绿 = fail-closed 面（unprepared 三包 / arity 三包 / impure / 同 arity 类型不符）。7 红 = 6 逐包消费 + 1 可重复，全部 `Launch("package instantiation failed: resource limit exceeded: table count too high at 1")`——**§Blocker**。
- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_consumer_forward -- --test-threads=1` → **2 passed; 0 failed**（**回归无损坏**；同先例性退出崩溃）。
- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_consumer_authority -- --test-threads=1` → 待执行（与 forward 同先例，预期 3 绿）。
- `cargo test --locked --offline -p sico-package` → 待执行（预期 51 绿）。
- `cargo fmt --all --manifest-path runner/sico-runner/Cargo.toml -- --check` 与 `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_roster_consumer -- -D warnings` → 待执行（限本片动范围；测试编译已绿，fmt/clippy 待跑）。
- `git diff --check` → 本片两个文件净。

## Blocker（具名缺口，gate 3 后半部未闭合的根因）

**runner 侧 `ResourceLimiter::tables() = 0`**（`runner/sico-runner/src/lib.rs:1428`）：runner 的 Store 限额**禁止任何 wasm table**（`table_growing` 亦恒 `Ok(false)`，`lib.rs:1412`）。ADR-0019 Rust 编译 package core（class (b)）带 wasm function table（三 core 均检出 table section `0x04`，`crates/sico-codegen-wasm/artifacts/image-{match,grid,contour}-core.wasm`），故 `instantiate_packages`（`lib.rs:2122`）对每个 roster 包报 `table count too high at 1` ⇒ 包**无法实例化进 Store**，consumer 的 `.run` 在 guest 执行前即 Launch 拒绝。手编 `csv` core 无 table，故 `package_consumer_forward.rs`（csv）走同一路径却成功——**registry 路径本身对无 table 包成立，对 ADR-0019 Rust 编译包（含 image-codec，STEP-0331）不成立**。

- **性质**：`tables() = 0` 是 **committed 状态**（working tree diff 无此函数改动，非并行代理 STEP-0587 在途改动）——**先例性 latent gap**：runner 的 `ResourceLimiter`（STEP-0147 建立 registry 路径时）假定 package 无 table，ADR-0019 Rust 编译包核（STEP-0331/0335-0337 引入）带 table，限额未随包核演进；`instances()`/`memories()` 已按 `package_instances_capacity` 缩放，`tables()` 未缩放（硬编码 0），故疑似**遗漏**而非有意禁止。
- **归属**：`runner/sico-runner/src/lib.rs` 的 **limits 段**——本片**禁动**（AGENTS.md 纪律：不得替他人收尾）。修复 = 让 `tables()`（及 `table_growing`）为 ADR-0019 包核适度放开（如 `≥1 + package_instances_capacity`），或为 Rust 编译包核引入免 table 构建（不可行：core digest-pinned）。
- **影响面**：本片 7 个 happy-path 测试**已写好**且逻辑自洽（csv 同路径先例已证），限额放开后应直接转绿；但**放开前不得声称 registry 消费证据成立**。

## Honest residuals

- **gate 3 不翻转**：本片是证据补半部；gate 3 的聚合 GO/NO-GO 裁决属出口审计（M24 plan §3 行 59-61、§8.3 行 179-188）。corpora 半部（STEP-0335/0336/0337，3/3 绿）与本 registry 消费半部**分开记录**，不得把 corpora 全绿当 registry 消费证据。
- **registry 消费 = internal-fixture**：package/Component 级；不是 clean-room-consumer（转换器 pilot 未写）、不是 external-pilot。
- **零 compile 声明边界**：消费路径不触发 Rust 侧重新编译（包是已编译 Component）；本片**未运行任何构建工具链**来重建 core `.wasm`，`image_{match,grid,contour}_package()` 嵌入提交件，`validate-package-core.ps1`（STEP-0335/0336/0337）已证 clean 重建字节全等。**注意**：由于 table 限额阻塞，本片**尚未**在已执行 runner 上证明「消费即实例化」的 happy-path；零 compile 的机制性依据成立，但运行时证据被 §Blocker 卡住。
- **同 arity 类型不符**：链接期不拒（arity 是唯一链接期检查维度）；本片测试以「运行期 typed 失败、不落部分调用」覆盖——但当前该失败**实际由 table 限额触发**（包无法实例化），**未**单独证明桥的类型校验拒绝；桥语义本身（`link_packages` 只比较元数、`func.call` 校验 `Val`）已由 `package_consumer_authority.rs` 的 csv 先例覆盖。
- **`sico-lock.json`（M7 lock）条目**：随首个消费该包的试点（gate 5 转换器 pilot）落地；本片以 artifact（已编译包字节 + 消费测试）形式交付，不新增 lock 条目。

## Gate 3 两半部完成度（分开记）

| 半部 | 状态 | 证据入口 |
|---|---|---|
| ① corpora：三包 deterministic 字节精确 vs 独立 oracle + RFC-0041 可达 `image-error` 闭集 | **完成 3/3** | STEP-0335/0336/0337 + `tests/image_{match,grid,contour}_package.rs` + `tests/corpus/{image-match,image-grid,image-contour}/manifest.json` + `tools/validate-package-core.ps1` |
| ② consumers 经 M7 registry 路径安装 + 消费演示 | **fail-closed 半部绿（4/4）**；**happy-path 消费未验证**——被 §Blocker（runner `tables()=0`）阻塞，**gate 3 后半部未闭合** | `tests/package_roster_consumer.rs`（fail-closed 绿、happy-path 待 §Blocker 解除）+ 本 STEP |

## Files touched

- `runner/sico-runner/tests/package_roster_consumer.rs`（新增，唯一测试文件；11 测 = 6 逐包消费 + 1 可重复 + 4 fail-closed 面）
- `docs/steps/STEP-0588-m24-gate3-roster-registry-consumer.md`（本文件）

未触碰：`runner/sico-runner/src/lib.rs`、`runner/sico-runner/src/ui.rs`、`tests/ui_authority.rs`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/src/provenance.rs`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、冻结语料与 `guests/**` core `.wasm`（digest-pinned）。
