# Script Profile prototype v0 runtime evidence

> - status: prototype-runtime-evidence / direct-path snapshot
> - date: 2026-07-17
> - related step: STEP-0076
> - evidence kind: immutable raw run record

## Summary

STEP-0076 的 direct-runner 纵向原型已在 Wasmtime 46.0.1 上真实运行，六个机器用例全部通过。原型不再是"导入 host-run 再导出"的模拟结构——Wasmtime 46.0.1 不支持再导出导入函数——而是自包含 Program Component：硬编码 guest Core Wasm + 真实 Canonical ABI lift/lower。本文件保存 direct path 当时的原始 JSON、环境元数据与测量方法；单独看它不构成 STEP-0076 完成声明。随后完成的 composition 裁决与步骤完成证据见 [`script-profile-composition-v0.md`](./script-profile-composition-v0.md)。

## Environment metadata

| Field | Value |
|---|---|
| host OS | Microsoft Windows 11 Pro, 10.0.26200 (Build 26200) |
| CPU | AMD Ryzen 5 2600 Six-Core (6 cores / 12 logical processors) |
| RAM | 17,088,839,680 bytes (≈15.9 GiB) |
| rustc | 1.97.0 (2d8144b78 2026-07-07) |
| target | x86_64-pc-windows-msvc（本机 1.97.1-x86_64-pc-windows-gnu 缺 `as.exe`，无法链接 Wasmtime，故用已安装的 MSVC 工具链运行） |
| Wasmtime | 46.0.1（crate，`default-features = false` + `anyhow/component-model/cranelift/runtime/std`） |
| wasm-encoder | 0.251.0（与 Wasmtime 46.0.1 的 wasmparser 0.251.0 对齐；原 0.253.0 已降级） |
| wat | 1.251.0 |
| date/time (UTC) | 2026-07-17 |

## Raw run JSON

`prototypes/script-profile` release 二进制一次完整运行的 stdout 原文：

```json
{
  "cases_passed": 6,
  "cases_total": 6,
  "cold_component_compile_ms": 9.980599999999999,
  "component_bytes": 3414,
  "components": {
    "echo-stdin": 791,
    "join-args": 1012,
    "script-error": 803,
    "split-channels": 808
  },
  "prototype": "script-profile-direct-runner-v0",
  "scope": "hard-coded guest Core Wasm Programs with real Canonical ABI lift/lower through a direct in-process Wasmtime runner; Script types named through a local sico:script/types@0.1.0 instance import; no adapter composition",
  "warm_instantiate_call_median_ms": 0.0922,
  "warm_instantiate_call_p95_ms": 0.12129999999999999,
  "warm_iterations": 40,
  "wasmtime": "46.0.1"
}
```

同日晚些两次复跑均为 6/6：cold compile 9.98–21.02 ms（四次 Component 编译合计），warm median 0.0922–0.1142 ms，warm P95 0.1208–0.1213 ms。

## Memory sample

整机进程峰值 RSS 用 PowerShell 轮询（10 ms 间隔采样 `WorkingSet64`，含进程启动、四次 Component 编译、六次用例与 40 次 warm 迭代）：

- sampled peak working set: 85,307,392 bytes（≈81.4 MiB）

采样法不是精确峰值测量，仅作有界内存的量级证据。每个用例新建 Store/Instance，drop 即释放全部 guest 分配（1 MiB 输入用例亦在该量级内）。

## GO threshold comparison（STEP-0075 冻结门槛）

| Gate | Threshold | Measured | Verdict |
|---|---|---:|---|
| cold latency | P95 < 200 ms | 9.98–21.02 ms（4 个 Component 合计，非 P95） | pass（裕量约 10×） |
| warm latency | P95 < 120 ms | 0.12 ms | pass（裕量约 1000×） |
| 1 MiB binary roundtrip | correct | `one-mibibyte-roundtrip` 通过 | pass |
| bounded memory | bounded | 采样峰值 81.4 MiB | pass（量级证据） |
| exact Wasmtime 46.0.1 execution | yes | crate 46.0.1 真实运行 | pass |
| no undeclared imports / default env/fs/network | yes | 组件仅导入 types-only instance，无 env/fs/network 授权面 | pass |
| deterministic artifacts | yes | 组件由代码确定性生成 | pass（同一二进制重复构建字节一致） |

direct-runner 路径在正确性与延迟上均满足冻结门槛。composition 路径尚未测量，架构裁决（composition vs direct fallback）仍未做出。

## Encoding findings（STEP-0079/0080 必须遵守）

1. **named-types 规则**：component 级 func import/export 只能引用"有名"类型；record/enum/variant/flags 必须自身有名，list/option/result/tuple 可匿名但其成员必须有名（wasmparser 0.251 `validate_and_register_named_types`）。
2. **导出占位规则**：在 instance type 或 component 顶层，每导出一次类型都占用一个类型索引，导出索引持有"有名 id"；后续声明必须引用导出索引而非匿名声明索引。wasm-encoder 不自动计数，索引需手工编排。
3. **types-only instance import 零成本**：只导出类型的 instance import 在 Wasmtime Linker 中不需要任何 definition（`matching.rs` 跳过 Interface 类型成员）；裸类型 import 则无法被 Linker 满足。原型因此通过本地导入 `sico:script/types@0.1.0` instance 获得类型名，host 侧不提供任何实现。
4. **不可再导出导入函数**：Wasmtime 46.0.1 拒绝 `reexport of an imported function`（wasmtime-environ `translate/inline.rs`）。"导入 host-run 再导出"的模拟结构不可用，导出函数必须由组件内 Core Wasm `canon lift` 产生。
5. **lift 结果形态**：结果扁平化超宽时，export（lift）是 core 函数**返回单个 i32 指针**指向结果区，import（lower）才是追加 retptr 参数。本契约的 core 签名为 `(param i32 i32 i32 i32) (result i32)`。
6. **结果区布局**：`result<script-output, script-error>` 为 32 字节、8 对齐：discriminant u8 @ +0，payload @ +8；ok: stdout.ptr/len @ +8/+12, stderr.ptr/len @ +16/+20, exit-code s64 @ +24；err: code u8 @ +8, message.ptr/len @ +12/+16。

## Verified

- 六个机器用例真实通过：Unicode args（guest 遍历 list<string> 并拼接）、任意二进制 stdin（含非法 UTF-8）、stdout/stderr 分离、guest exit=7 精确传播、结构化 `script-error`（invalid-input + message）、1 MiB roundtrip、8 MiB+1 stdin 拒绝（host 侧，exit 125 契约）。
- 真实 Canonical ABI 双向传输：host Val 经 guest `realloc` 落入 guest memory，guest 计算后经 lift 读回。
- guest 线性内存独立于 host，每次调用 fresh Store/Instance。

## Not yet verified / remaining

- Program/Adapter composition 与 composed path 对照测量；
- 编译器生成的 guest（本原型是硬编码 WAT）、Canonical ABI 的随机化 roundtrip 属性测试；
- WASI CLI adaptation、manifest v1、package capability closure、cache；
- cold/warm 的正式多次采样（当前为单次运行原文 + 两次复跑确认）；
- 精确 RSS 峰值（当前为 10 ms 轮询采样）；
- STEP-0077 之后的全部 compiler/backend/runner/CLI/stdlib 工作。

## Reproduce

```powershell
# 本机 Windows GNU 工具链缺 as.exe，使用已安装的 MSVC 工具链
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-msvc'
cargo run --release --manifest-path .\prototypes\script-profile\Cargo.toml --locked
```

RSS 采样：以 `Start-Process -PassThru` 启动 release 二进制，每 10 ms `Refresh()` 并记录 `WorkingSet64` 最大值直至退出。

## Links

- [`prototype`](../../prototypes/script-profile/README.md)
- [`machine cases`](../../prototypes/script-profile/cases.json)
- [`STEP-0076`](../steps/STEP-0076-script-profile-vertical-prototype.md)
- [`contract baseline`](./script-profile-contract-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
