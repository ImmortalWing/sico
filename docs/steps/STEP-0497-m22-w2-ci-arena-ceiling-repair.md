# STEP-0497 m22-w2 修复 M23 会话交接的 CI 红点：脚本组件 guest 内存上限与 effective ceiling 断裂

> - status: **complete（独立 CI 已裁决：run 39524849306 = `success`）**
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md)（上一片）、[RFC-0029](../rfc/RFC-0029-script-profile-v0.md)（Script profile v0 + STEP-0454/0484 修正案）
> - gate: **不推进门 3**；本片是 M23 会话交接的 CI 红点修复（owner 裁决归 M22 轨修，M23 不动 `crates/sico-codegen-wasm` / `crates/sico-runtime`）
> - numbering: M22 线顺延为 **STEP-0497**（本线已占用 STEP-0491..0496；并行 M23 会话占用 STEP-0489/0490；已核对 `docs/steps/` 无冲突）

## 1. 目标与结论

**目标**：让 `sico-app-cli::run_executes_a_script_package_end_to_end` 由红转绿，并把这个红点暴露出的**真实契约断裂**（声明下界 > effective ceiling）从"事后归因"变成"可执行的不变量"。

**结论（实测）**：红点已闭合，且断裂面比 M23 的归因报告**更完整**——真正绑定的是**包 manifest 的 64 MiB**，不是宿主上限。

- 修复前**逐字复现**：`SICO_TEST_WASMTIME` 已设时，`cargo test -p sico-app-cli --test cli run_executes_a_script_package_end_to_end` 红，错误逐字为 `forcing trap when growing memory to 2214592512 bytes`，与 CI（run 39454624507 / 39457030463）逐字一致；
- 修复后同一用例 **ok**（`1 passed / 0 failed`）；
- **不变量守卫四条**全部落地（codegen 字节级 / package 层 / runtime 层 / runner 层），全部为常量或字节比较，**不依赖 wasmtime**，每台机器都跑 ⇒ 修掉"本地绿灯是空跑"的假阴性；
- RFC-0029 增补 **STEP-0497 修正案**：写清 2 GiB 是 **arena 上限**、声明线性内存是 **33,792 页 = 2,214,592,512 B**（min == max），并写明"任何执行路径的 effective ceiling 必须 ≥ 声明下界"。

## 2. 归因链（实测，与 M23 报告的差异）

### 2.1 声明下界从哪来

`crates/sico-codegen-wasm/src/canonical.rs`：

- `ARENA_LIMIT = 2048 * 1024 * 1024`（2 GiB）= guest **arena** 上限（arena 活在 guest 线性内存**里面**）；
- `MEMORY_PAGES = 33792` 页 = 2,214,592,512 B = **声明的线性内存**（arena 2 GiB + 64 MiB 余量）；
- 声明点共**四处**，全部以 **minimum == maximum** 发出（⇒ guest 内存固定、从不增长）：
  `canonical.rs:956-961`（fs transport 模块）、`package_builder.rs:118-121`（包 core 模块）、`lib.rs:1252-1256`（script core 模块，非 transport）、`lib.rs:1334-1337`（script core 模块，transport 形态的 memory import）。

### 2.2 effective ceiling 从哪来

`crates/sico-runtime/src/lib.rs`：

- `effective_limits(package, host) = min(包 manifest 请求, 宿主)`（`lib.rs:344-360`）；
- 该值作为 wasmtime 的 `-W max-memory-size=` 下发（`lib.rs:403`）；
- wasmtime 在**初始内存分配**时检查 `bytes > max` ⇒ 超限即 `forcing trap when growing memory to N bytes`，N = 声明的 2,214,592,512 B ⇒ **报错里的数字就是声明下界**。

### 2.3 真正绑定的是包 manifest

| 层 | 修复前的值 | 是否 < 声明下界 2,214,592,512 B |
|---|---|---|
| 包 manifest `limits.memory_bytes` | **64 MiB**（`sico-app-cli::script_limits()` 继承 `RuntimeLimits::default()`） | **是**（64 MiB） |
| 宿主 `HostLimits::default().memory_bytes` | 2 GiB | **是**（2 GiB） |
| runner `RunnerLimits::default().memory_bytes` | 2.5 GiB | 否 ✓ |

因为 effective = **min(包, 宿主)**，所以**先只抬宿主上限到 2.5 GiB 时用例仍然红**（实测）——这正是 M23 归因报告把"宿主 2 GiB"当成绑定约束时漏掉的一环。

### 2.4 历史断点

`b9f652b`（2026-09-29，最后一次绿 CI）时 `MEMORY_PAGES = 1024` = 64 MiB，**与包 manifest 的 64 MiB 恰好相等** ⇒ 配对成立。`c50b3fd`（2026-10-20，STEP-0402/0421/0454/0484）把 `MEMORY_PAGES` 2048 → 33792、`ARENA_LIMIT` 128 MiB → 2 GiB、宿主授权 128 MiB → 2 GiB **一次改完，没有同步包 manifest 的 64 MiB** ⇒ 配对断裂，且断裂后的第一个 CI 运行即红。

## 3. 修法（取"不改变发射字节"的一侧）

| 决策 | 取值 | 理由 |
|---|---|---|
| `MEMORY_PAGES` | **不动**（33,792） | 改它会重生成全部快照（`tests/wasm/artifacts.hex`）、guest 冻结常量与发射字节，爆炸半径大；且 min < max 的按需增长会把"上限"变成 wasmtime trap 而非已登记的 arena 耗尽 trap，语义更差 |
| 包 manifest 限值 | **抬到 `SCRIPT_MEMORY_BYTES`**（2,214,592,512 B） | 恢复"声明 ↔ 授予"的精确配对；这是**真正的绑定约束** |
| 宿主上限 | 2 GiB → **2.5 GiB** | 与 runner 路径 `RunnerLimits::default()` 同值，两条执行路径不再对上限各说各话 |
| 包限值 < 声明下界 | **typed refusal**（不再启动） | fail-closed（ADR-0016 决策 6）：把不透明 trap 变成可诊断的 typed error |

**不改变任何发射字节** ⇒ 不需要重生成快照，不影响 `A == B == C` 的字节对拍基线。

## 4. 代码改动

| 文件 | 改动 |
|---|---|
| `crates/sico-codegen-wasm/src/canonical.rs` | 修正 `ARENA_LIMIT`/`MEMORY_PAGES` 的陈旧注释（原文把 arena 上限写成"64 MiB = 1,024 页"，实为 arena 上限 vs 线性内存声明）；新增 `pub const SCRIPT_MEMORY_BYTES: u64 = MEMORY_PAGES as u64 * 65536;` |
| `crates/sico-codegen-wasm/src/lib.rs` | re-export `SCRIPT_MEMORY_BYTES` |
| `crates/sico-package/src/lib.rs` | re-export `SCRIPT_MEMORY_BYTES`；新增 `script_profile_limits()`（`memory_bytes = SCRIPT_MEMORY_BYTES`，`body_bytes = 8 MiB`）；`build_script_v1` **拒绝** `limits.memory_bytes < SCRIPT_MEMORY_BYTES`（`PackageError::Manifest`） |
| `crates/sico-app-cli/src/lib.rs` | `script_limits()` 改为委托 `sico_package::script_profile_limits()` |
| `crates/sico-runtime/src/lib.rs` | `HostLimits::default().memory_bytes` 2048 MiB → **2560 MiB**；新增 `PackageRuntimeError::MemoryCeilingBelowDeclared { effective_bytes, declared_bytes }` 与公开函数 `effective_memory_limits()`，在启动前 typed refusal |
| `crates/sico-codegen-wasm/tests/script_abi.rs` | 新增**字节级**守卫：解析组件字节，断言 memory section 的 min == max == 33,792 页，并断言 `SCRIPT_MEMORY_BYTES == 33,792 × 65,536 == 2,214,592,512` |
| `crates/sico-package/tests/package.rs` | `script_input()` 改用 `script_profile_limits()`；新增"低于声明下界被拒"与"profile 限值覆盖声明"两条测试 |
| `crates/sico-runtime/tests/memory_ceiling.rs` | 新建：宿主上限 / 包层授予 / 默认上限可启动 / 低于下界 typed refusal 四条 |
| `runner/sico-runner/tests/memory_ceiling.rs` | 新建：runner 默认上限 ≥ 声明下界 |
| `docs/development/script-canonical-abi-layout-v0.md` | **修正陈旧数字**：§5 仍写"arena 上限 = 声明内存上限 64 MiB（1,024 页）"、§6 仍写"guest linear memory 最多 64 MiB"——与实现（arena 上限 2 GiB / 声明线性内存 33,792 页 = 2,214,592,512 B）不符；该文档自称"any disagreement is a defect"，故按纪律在同一范围内修复（附录式改写，不改布局表本体） |

**模块边界**：`runtime` 模块允许的 normal deps = `application, observability, runtime`，**不含 `compiler`** ⇒ `sico-runtime` 不新增 `sico-codegen-wasm` 生产依赖；改为经 `sico-package`（`application` 模块，已有例外允许其依赖 codegen）转出 `SCRIPT_MEMORY_BYTES`。`sico-codegen-wasm` 在 `sico-runtime` 仍只是 **dev-dependency**（dev-dep 不受边界校验）。

## 5. 四条不变量守卫（可执行，非口头约定）

| # | 层 | 断言 | 需要 wasmtime？ |
|---|---|---|---|
| 1 | codegen（字节级） | 发出组件的 memory section `min == max == 33,792` 页；`SCRIPT_MEMORY_BYTES == 2,214,592,512` | 否 |
| 2 | package | `script_profile_limits().memory_bytes >= SCRIPT_MEMORY_BYTES`；低于下界 ⇒ `build_script_v1` typed 拒绝 | 否 |
| 3 | runtime | `HostLimits::default().memory_bytes >= SCRIPT_MEMORY_BYTES`；effective 低于下界 ⇒ `MemoryCeilingBelowDeclared` | 否 |
| 4 | runner | `RunnerLimits::default().memory_bytes >= SCRIPT_MEMORY_BYTES` | 否 |

**为什么必须是常量级**：本红点的原教训是"本地绿灯是空跑"——`SICO_TEST_WASMTIME` 未设时启动缝用例**直接 return**（`crates/sico-app-cli/tests/cli.rs:213`），CI 才红。守卫 1–4 **不需要任何 runtime 二进制**，因此每台机器、每次 CI 都会真正执行。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 修复前复现 | `SICO_TEST_WASMTIME=<wasmtime> cargo test -p sico-app-cli --test cli run_executes_a_script_package_end_to_end` | **红**：`forcing trap when growing memory to 2214592512 bytes`（与 CI 逐字一致） |
| 红点转绿 | 同上 | **ok**（`1 passed / 0 failed`） |
| 守卫 1 | `cargo test -p sico-codegen-wasm --test script_abi` | **8 passed / 0 failed** |
| 守卫 2 | `cargo test -p sico-package` | **15 + 4 passed / 0 failed**（`package` 15 项含新增 2 项；`security_properties` 4 项） |
| 守卫 3 | `cargo test -p sico-runtime --test memory_ceiling` | **4 passed / 0 failed** |
| 守卫 4 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test memory_ceiling` | **1 passed / 0 failed** |
| **一键 CI（最终态）** | `tools/run-ci.ps1` | **CI GREEN**（exit 0）：fmt / clippy(workspace) / clippy(runner) / build(runner) / test(workspace 2:17) / test(runner 2:20) / module boundaries / planning contract (M14-M25) / application-profile matrix / cross-host matrix + UI corpus / `git diff --check` — **11/11 全绿** |
| 行尾审计 | 全部 37 个改动/新增文件逐字节扫描 | **纯 LF、零 CRLF、零 BOM** |

## 7. 验证电池（最终态）

`tools/run-ci.ps1` 是本仓库的权威一键 CI（`tools/run-ci.ps1` 直接调用 `.github/workflows/m22-w1-ci.yml` 的同一入口）。最终态 **CI GREEN**（exit 0），11 个步骤全绿：

| 步骤 | 结果 |
|---|---|
| fmt（root + runner） | PASS |
| clippy（workspace，`-D warnings`） | PASS |
| clippy（runner，`-D warnings`） | PASS |
| build（runner debug） | PASS |
| test（workspace `--all-targets --all-features`，`SICO_TEST_WASMTIME` 已设） | PASS（2:17） |
| test（runner，`--test-threads=1`） | PASS（2:20） |
| module boundaries | PASS |
| planning contract (M14-M25) | PASS |
| application-profile matrix | PASS |
| cross-host matrix + UI corpus | PASS |
| whitespace（`git diff --check`） | PASS |

另：**行尾审计**——全部改动/新增文件逐字节扫描确认纯 LF、零 CRLF、零 BOM。

> 首轮 CI 记录为 **CI RED: fmt**（仅 fmt）：该轮 CI 在本片改动落地前启动，其 fmt 步骤比对了 rustfmt 尚未对齐的两处排版；已按 rustfmt 输出手工对齐后重跑全量，得到最终态 **CI GREEN**。

## 7.1 独立 CI 裁决（已出）

owner 授权后本片与 STEP-0496 一起提交为 `7e5ad3c`（35 files, +4868/−99），推送到 `github/codex/m22-w1-ci`（fast-forward `6808569..7e5ad3c`，远端 ref 已核对 = `7e5ad3c21197e40848beb403c53074e3458feaf8`），触发 **独立 runner** 上的 CI：

| 项 | 值 |
|---|---|
| run id | `39524849306` |
| workflow | `M22 W1 independent CI`（`.github/workflows/m22-w1-ci.yml`） |
| head_sha | `7e5ad3c21197e40848beb403c53074e3458feaf8` |
| runner | `windows-2025`（GitHub Actions，独立于本机） |
| 开始 / 结束 | 2026-10-21T01:01:29Z → 2026-10-21T01:28:08Z |
| **裁决** | **`status: completed` / `conclusion: success`** |
| job | `windows-gnu`：`Set up job` / `checkout` / `Install GNU build tools` / **`Run repository CI on a fresh runner`** 全部 success |

即：M23 会话交接的 CI 红点**在独立 runner 上确认转绿**，本片的修复不再是"仅本机绿"。**本片到此闭合。**

## 8. 诚实性

- 本片**不推进门 3**：run 体发射、37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做，门 3 仍为 **NO-GO**。
- 本片**已提交并推送**（owner 授权）：提交 `7e5ad3c`，推送 `github HEAD:refs/heads/codex/m22-w1-ci`（fast-forward `6808569..7e5ad3c`），**独立 CI run 39524849306 裁决 `success`**（见 §7.1）。
- 本片**没有**改动 `MEMORY_PAGES`，因此**没有**重生成任何快照；发射字节与 STEP-0496 交付时逐字节相同（除本片新增的守卫测试外，无 Rust 生产字节变化）。
- **登记的设计债**：因为 memory 的 minimum == maximum，RFC-0029 原条款"除非有更严格的包/Host 限额"对脚本组件**不可实现**——低于声明下界的限额不是"更紧"，而是"不可满足"。彻底解决需要让声明内存按需增长（min < max）或改写该条款为"限额只能在声明之内约束执行"，属**未来 RFC** 议题，本片只登记不改写。
- 工作区仍有**非本线遗留**的未跟踪文件 `crates/sico-codegen-wasm/tests/scratch_dump.rs`（含 `include_str!("../../../target/tmp/echo_fixture.sico")`），它在干净检出上会**破坏 `cargo test --workspace`**；本片未改动它（属他人工作区产物），提请 owner 在提交前处置。
