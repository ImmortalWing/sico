# STEP-0596: M24 #2 — `sico:user/image-codec@1` registry-path consumer 证据（STEP-0593 预期受益方结案）

> - status: **实现 + 验证完成；`image-codec@1` 经 M7 registry 路径被真实 consumer 消费的注册缺口（STEP-0593「未在本片补断言」）已闭合；§3 gate 3 聚合裁决仍属出口审计，不翻转**
> - phase: M24 §3 gate 3 / §8.4 转换引擎（`image-codec@1`，STEP-0331 的 registry 消费半部）；结案 STEP-0593 对 `image-codec@1`（STEP-0331）的「预期受益方，待后续片验证」登记
> - completed: 2026-11-07
> - contracts: [`RFC-0050`](../rfc/RFC-0050-image-codec-package-v0.md)（A1 扁平面）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)、[`RFC-0039`](../rfc/RFC-0039-package-consumer-bridge-v0.md) §2.3/§2.4（M7 registry 路径）、[`STEP-0593`](./STEP-0593-m24-registry-table-budget-fix.md)
> - evidence: **internal-fixture**（package/Component 级）且 **runtime-verified**（8/8 在已执行 runner 上逐字节断言）；**零 compile**（包为预编译、digest-pinned Component，消费仅实例化）；**不是**脚本级、**不是**试点级
> - pre-existing dirty worktree: `runner/sico-runner/src/lib.rs`、`runner/sico-runner/src/ui.rs`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/**`、`selfhost/`、`guests/**` 等由**并行代理**修改（STEP-0587 / STEP-0592 D2D / STEP-0593 限额 / M22 轨）；本片**只**新建一个测试文件与一个 STEP 文档，未触碰任何并发文件，未触碰冻结语料与 digest-pinned core `.wasm`

## Objective

闭合 STEP-0593 登记的注册缺口：`image-codec@1`（STEP-0331）当时被 runner `tables() = 0` 阻塞，STEP-0593 修正限额后**明确列为「预期受益方」**，但「**未在本片补断言**」（见 STEP-0593 §「对 `image-codec@1` 的影响评估」），并给出起点：冻结用例 `decode-ok-2x2-rgba`，期望 `payload_hex = 020000000000000002000000000000001e140aff3c3228805a504600786e6440`。本片沿既有 registry 消费先例（`package_roster_consumer.rs` 的 M7 路径，非另造）补上该断言，并覆盖 fail-closed 面与可重复性。**零 compiler/Runtime 语义改动**（包仍是已编译 Component，消费仍即实例化）。证据等级 internal-fixture + runtime-verified，不翻任何 §3 gate。

## 侦察（只读，确切签名与夹具位置）

### producer 与确切导出签名（`crates/sico-codegen-wasm/src/package_builder.rs:2069` `image_codec_package()`）

- `decode-png(data: Bytes) -> Result[Bytes, Text]` — **1 参**（`list<u8>`），ok = `Bytes`（16 字节小端头 `width u64 · height u64` + RFC-0041 BGRA8 像素），err = `Text`（RFC-0050 §3 + A1 case 名）。
- `encode-png(pixels: Bytes, width: U64, height: U64) -> Result[Bytes, Text]` — **3 参**（`list<u8>` + 两个 `u64`），ok = 编码 PNG 字节，err = `Text`。
- core = `include_bytes!("../artifacts/image-codec-core.wasm")`（**预编译、digest-pinned**，ADR-0019 class (b)；producer 只嵌入并复用与手编包同一个 `build_package_component`）。

### 冻结用例与夹具位置（`runner/sico-runner/tests/corpus/image-codec/`）

- 冻结 manifest：`tests/corpus/image-codec/manifest.json`（schema `sico.m24.codec-fixtures.v0`），`decode_cases` 9 例（1 ok + 8 类型化拒绝）。
- **起点用例**：`decode-ok-2x2-rgba` → 输入夹具 `tests/corpus/image-codec/ok-2x2-rgba.png`（83 B），期望 `payload_hex = 020000000000000002000000000000001e140aff3c3228805a504600786e6440`（32 B = 16 B 头 + 16 B BGRA8 像素，`width==height==2`）。
- 错误用例（本片取 1 例）：`decode-bad-signature` → `tests/corpus/image-codec/bad-signature.png`（83 B），期望 case 名 `bad-signature`。
- **PNG 输入字节来源**：既有冻结夹具文件（`std::fs::read` 运行期读取，与 `image_codec_package.rs` 的 `fixture()` 相同）；**期望输出字节**：从冻结 manifest 程序化读取（`decode_ok_payload()`），测试与 oracle 不漂移。

### 先例路径（镜像，未另造）

`package_roster_consumer.rs`（STEP-0588/0593）与 `package_consumer_forward.rs`/`package_consumer_authority.rs`：`Runner::prepare_program_with_packages` + `PreparedProgram::run`（`prepare_package` → `link_packages` → `instantiate_packages`）。`link_packages`（`runner/sico-runner/src/lib.rs:2027`）按**接口名 + 函数名 + arity** 匹配，报错文本：`no prepared package exports it`（`lib.rs:2074`）、`arity mismatch`（`lib.rs:2095`）、`must not import`（`lib.rs:1966`）。动态 `Val` 直转发，故同 arity 类型不符只能在**调用时**失败（无部分调用）。

## Implementation（`runner/sico-runner/tests/package_image_codec_consumer.rs`，新建）

手编 `wasm-encoder` 真实 guest 组件，形态镜像 `package_roster_consumer.rs`：import `sico:user/image-codec@1.0.0`，`lower_func` 将包导出降到共享 transport（内存 + 检查式 bump 分配器），guest core 转发静态输入并回传 payload 字节。

- **签名适配**：`CodecCall { function, params: Vec<ParamKind>, input, u64_args, expected_disc }`，`ParamKind::{Bytes, U64}` 驱动组件级实例类型（`list<u8>`@0、`result<list<u8>, string>`@1、func@2）、core 签名（`Bytes`→`(i32,i32)`、`U64`→`i64`、尾随 retptr）与 guest 调用序列。`decode-png` = 1 参 `Bytes`；`encode-png` = `[Bytes, U64, U64]` + `u64_args = [width, height]`。
- **ok/err 判别读取**：`result<list<u8>, string>` 的 return-area union——tag i32 @0、payload `(ptr,len)` @4/@8（ok 臂 `list<u8>` 与 err 臂 `string` 同布局）；guest 仅当判别符 == 期望极性时回传 payload 字节作为 stdout，否则 exit = 1000+disc（永不部分调用）。

### happy path（逐字节断言）

| 测试 | 输入 | 断言 |
|---|---|---|
| `image_codec_decode_ok_reaches_the_consumer_through_the_registry` | 冻结 `ok-2x2-rgba.png`（83 B） | `stdout ==` 冻结 `payload_hex`（32 B，逐字节） |
| `image_codec_encode_round_trips_through_the_registry` | 冻结 payload 的 16 B BGRA8 尾段，`width=height=2` | 两次 encode `stdout` 逐字节相同（byte-stable）；再 decode 回 `stdout ==` 冻结 `payload_hex` |
| `image_codec_error_case_name_reaches_the_consumer_through_the_registry` | 冻结 `bad-signature.png` | `stdout == b"bad-signature"`（err 臂 case 名文本） |
| `codec_consumer_run_is_repeatable` | 冻结 `ok-2x2-rgba.png` | 两次 run `stdout`/`exit_code` 相同（跨 Store 字节稳定） |

### fail-closed 覆盖矩阵（全部经同 registry 路径，类型化 `Launch`、无部分调用）

| 面 | 用例（按真实签名取） | 断言 |
|---|---|---|
| 未备包导入 | `decode-png`（1 参）+ `encode-png`（3 参），packages = `[]` | `Launch` 含 `no prepared package exports it` |
| arity 不符 | `decode-png` 与 `encode-png` 均声明为 **2 参** | `Launch` 含 `arity mismatch` |
| 同 arity 类型不符 | `decode-png` 声明为 `func(u64) -> result<list<u8>, string>`（arity 仍 1，参数型错） | link 期接受，调用期**非 Output**（typed 失败，无部分调用） |
| 非纯包 | 包本身有 import，guest 为空 | `Launch` 含 `must not import` |

## Executed validation

工具链 `1.98.0-x86_64-pc-windows-gnu`，`--offline`，`--locked`，`--test-threads=1`（runner 侧）。**本片零 compile**：包为预编译 digest-pinned Component（`include_bytes!` 提交 core），消费仅实例化；未运行任何构建工具链重建 core `.wasm`；未改 `guests/**`、未改 `crates/sico-package` producer 或冻结语料。

| 验证 | 命令 | 结果 |
|---|---|---|
| 本片主验证 | `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_image_codec_consumer -- --test-threads=1` | **8 passed; 0 failed**（4 happy + 4 fail-closed） |
| 逐条断言 | 同上一并覆盖 | decode-ok 逐字节 == 冻结 `payload_hex`；encode 两次逐字节相同且 round-trip == 冻结 `payload_hex`；err 臂 `bad-signature`；可重复性跨 Store 字节稳定；fail-closed 4 面全部 typed `Launch` / 非 `Output` |
| 回归（roster 先例） | `--test package_roster_consumer` | **11 passed; 0 failed** |
| 回归（csv 先例） | `--test package_consumer_forward` | **2 passed; 0 failed** |
| 回归（fail-closed 先例） | `--test package_consumer_authority` | **3 passed; 0 failed** |
| 回归（codec 内容测） | `--test image_codec_package` | **5 passed; 0 failed** |
| 回归（包侧） | `cargo test --locked --offline -p sico-package` | **51 passed; 0 failed**（lib 18 + binary_carrying 1 + package 15 + provenance_budget 3 + provenance_fixture 6 + provenance_install 4 + security_properties 4 + doc 0） |
| clippy（本片测试） | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test package_image_codec_consumer -- -D warnings` | **OK** |
| clippy（lib） | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --lib -- -D warnings` | **OK** |
| fmt | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml -p sico-runner -- --check` | **exit 0（净；diff 仅本片新文件，已格式化）** |
| `git diff --check` | 全 tracked 树 | **exit 0（净）** |

**环境性说明（如实登记）**：首次执行新构建的测试二进制时观察到一次进程退出 `STATUS_ACCESS_VIOLATION`（0xc0000005），发生在 **8/8 测试已报告 ok 之后**；复跑 3 次均 exit 0。同一现象亦在**既有** `image_codec_package` 测试二进制上偶发一次（复跑 exit 0）。判定为 **wasmtime JIT teardown 在本机 Windows GNU target 上的间歇性进程退出异常**，与测试内容无关（roster/csv 同法 0 次）；所有计数的运行均报告 `8/8`、`5/5` 且 exit 0，不构成本片断言被放宽或测试不稳定。

## 口径声明

- **零 compile**：消费路径未触发 Rust 侧重编译（包为预编译、digest-pinned Component，`instantiate_packages` 仅实例化）；本片未运行任何构建工具链重建 core `.wasm`。
- **未改冻结资产**：未改 `guests/**` 的 digest-pinned `.wasm`、未重建 core；未改 `crates/sico-package` 的 producer 或冻结语料；未改 `tests/corpus/image-codec/**`。
- **未动并发文件**：未改 `runner/sico-runner/src/**`、`ui_guest_visible.rs`、`ui_authority.rs`、`package_roster_consumer.rs`、`provider_crash_isolation.rs`、`crates/sico-ui-renderer-d2d/**`、`wit/**`、`selfhost/`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`。本片**只**新建 `runner/sico-runner/tests/package_image_codec_consumer.rs` 与 `docs/steps/STEP-0596-*.md`。
- **证据等级**：internal-fixture（package/Component 级）+ runtime-verified（已执行 runner 上逐字节断言）；不是 clean-room-consumer、不是 external-pilot、不是脚本级。

## 对 STEP-0593「预期受益方」登记的结案说明

STEP-0593 §「对 `image-codec@1`（STEP-0331）的影响评估」记载：`image-codec-core.wasm` 实测 1 个 defined funcref table（`min == max == 71`），修正后应可沿 M7 registry 路径实例化；当时**未在本片补断言**，登记为「**修正的预期受益方，待后续片验证**」，并给出起点 `decode-ok-2x2-rgba`。本片以该冻结用例为起点，新增 `package_image_codec_consumer.rs` 8/8 绿，覆盖 1 参 `decode-png` 与 3 参 `encode-png` 两形、ok/err 两臂、fail-closed 四面与可重复性——**STEP-0593 登记的「预期受益方待验证」缺口就此闭合**。这与 STEP-0593 对 roster 三包已闭合的 registry 半部（11/11）同属一条 M7 路径；**不构成 gate 3 整体 GO**（聚合裁决属出口审计，含 corpora 半部与 §3 其余 gate）。

## Honest residuals / 诚实收窄

- **gate 3 不翻转**：本片只补 `image-codec@1` 的 registry 消费证据；gate 3 的聚合 GO/NO-GO 属**出口审计**（M24 plan §3、§8.4）。roster 三包 corpora（STEP-0335/0336/0337）+ registry 消费（STEP-0588/0593）与 `image-codec@1` corpora（STEP-0331）+ registry 消费（本片）**分开记录**，不得把任一全绿当 gate 3 整体 GO。
- **registry 消费 = internal-fixture**（package/Component 级，runtime-verified at 已执行 runner）；不是 clean-room-consumer、不是 external-pilot。
- **encode 侧 byte-exact 冻结字节未单列**：`encode-png` 的 ok 字节在冻结 manifest 中无独立 hex 记录（manifest 只冻结 decode_cases），故本片沿既有 `image_codec_package.rs` 的既定断言——**两次 encode 逐字节相同** + **`decode(encode(x)) == x` 回到冻结 `payload_hex`**（frozen-anchored，不引入未冻结的硬编码字节）。如需单列 encode 冻结字节，须由独立 oracle 补入 manifest（超出本片「不改冻结语料」边界）。
- **环境性 AV flake**：本机 Windows GNU 上 wasmtime JIT teardown 的间歇性进程退出异常（偶发于测试二进制进程退出，含既有 `image_codec_package`），已如实登记；不影响本片断言计数（每次报告 8/8、5/5 且 exit 0）。

## Files touched

- `runner/sico-runner/tests/package_image_codec_consumer.rs`（**新建**，8 测试：4 happy + 4 fail-closed）
- `docs/steps/STEP-0596-m24-image-codec-registry-consumer.md`（本文件）

未触碰：`runner/sico-runner/src/**`、`runner/sico-runner/tests/{ui_guest_visible,ui_authority,package_roster_consumer,provider_crash_isolation}.rs`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/**`、`wit/**`、`selfhost/`、`guests/**` core `.wasm`（digest-pinned）、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、冻结语料与 `tests/corpus/**`。
