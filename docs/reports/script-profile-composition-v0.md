# Script Profile Program/Adapter composition evidence v0

> - status: verified prototype evidence
> - date: 2026-07-17
> - related step: STEP-0076
> - evidence kind: immutable composition run record

## Summary

STEP-0076 的 hard-coded Program + versioned Adapter composition 已在 Wasmtime 46.0.1 上真实运行。direct 与 composed 两条路径使用同一组 Script 值和六个机器用例；两条路径连续 20 次均为 6/6，通过 Unicode arguments、任意二进制 stdin、stdout/stderr 分离、guest exit、结构化错误、1 MiB roundtrip 与 8 MiB+1 host refusal。

Adapter `sico:script/adapter@0.1.0` 不再尝试转导出导入函数。它在私有线性内存中 canonical-lower Program 函数，经 core trampoline 调用，再 canonical-lift 为 Adapter 的 `run` 导出；外层 Component 显式实例化 Program 与 Adapter。因此 composed path 真实跨越两次 Canonical ABI 边界，并由 Wasmtime 验证类型与实例连接。

结论为 `composition-go`：composition 正确、确定、稳定且远低于冻结延迟门槛。direct path 保留为诊断与兼容回退。该结论选择 M8 架构，不等同于 RFC-0029 或 ADR-0009 的最终 acceptance；WASI CLI adapter、compiler-generated guest、package closure、Runtime limits 与 malicious-guest evidence 仍属于 STEP-0077–0084。

## Environment metadata

| Field | Value |
|---|---|
| host OS | Microsoft Windows 11 Pro, 10.0.26200 (Build 26200) |
| CPU | AMD Ryzen 5 2600 Six-Core (6 cores / 12 logical processors) |
| RAM | 17,088,839,680 bytes (≈15.9 GiB) |
| rustc | 1.97.0 (2d8144b78 2026-07-07) |
| target | x86_64-pc-windows-msvc；VS 2022 BuildTools MSVC 14.44 + Windows SDK 10.0.26100 |
| Wasmtime | 46.0.1 crate |
| wasm-encoder / wat | 0.251.0 / 1.251.0 |
| date | 2026-07-17 |

## Raw run JSON

Release binary 一次完整运行的 stdout 原文：

```json
{
  "adapter": {
    "bytes": 915,
    "identity": "sico:script/adapter@0.1.0",
    "sha256": "fc049155cbf37bb727425461f0cdf38ba2d3440d839a3c5137c9425aed675f8f"
  },
  "composed": {
    "cases_passed": 6,
    "cases_total": 6,
    "cold_component_compile_ms": 14.6251,
    "component_bytes": 8770,
    "components": {
      "echo-stdin": 2130,
      "join-args": 2351,
      "script-error": 2142,
      "split-channels": 2147
    },
    "warm_instantiate_call_median_ms": 0.0876,
    "warm_instantiate_call_p95_ms": 0.1014,
    "warm_iterations": 40
  },
  "decision": "composition-go",
  "direct": {
    "cases_passed": 6,
    "cases_total": 6,
    "cold_component_compile_ms": 5.7543999999999995,
    "component_bytes": 3414,
    "components": {
      "echo-stdin": 791,
      "join-args": 1012,
      "script-error": 803,
      "split-channels": 808
    },
    "warm_instantiate_call_median_ms": 0.0478,
    "warm_instantiate_call_p95_ms": 0.0758,
    "warm_iterations": 40
  },
  "prototype": "script-profile-program-adapter-composition-v0",
  "wasmtime": "46.0.1"
}
```

## Repeated-run evidence

同一 release binary 独立启动 20 次，每次都重建、验证并编译四个 direct 与四个 composed artifacts，随后运行全部用例及两条路径各 40 次 warm iteration：

| Metric | Direct | Composed |
|---|---:|---:|
| successful fixture runs | 20/20 × 6/6 | 20/20 × 6/6 |
| cold compile min / median / P95 / max | 5.3781 / 5.6439 / 7.3747 / 7.3747 ms | 13.9611 / 14.5024 / 15.8033 / 15.8033 ms |
| per-run warm P95 min / median / P95 / max | 0.0558 / 0.0667 / 0.1857 / 0.1857 ms | 0.0930 / 0.1024 / 0.2038 / 0.2038 ms |
| total four-artifact bytes | 3,414 | 8,770 |

20 次运行只产生一个 Adapter SHA-256。编码函数还在进程内二次生成 Program、Adapter 与 composed bytes 并逐字节比较，因此确定性差异会在执行前失败。

## Memory sample

以 1 ms 间隔轮询 10 个独立 release 进程的 `WorkingSet64`；每个进程包含 direct 和 composed 两阶段、全部六用例及 warm iterations：

- sampled peak minimum: 90,394,624 bytes；
- sampled peak median: 102,539,264 bytes；
- sampled peak maximum: 103,518,208 bytes（≈98.72 MiB）。

这是整个双路径原型进程的离散采样，不是 composition-only 或 allocator 精确峰值。

## Gate comparison and decision

| Gate | Threshold | Measured composed path | Verdict |
|---|---|---:|---|
| correctness | same six cases as direct | 20 × 6/6 | pass |
| cold latency | P95 < 200 ms | 15.8033 ms | pass |
| warm latency | P95 < 120 ms | worst recorded per-run P95 0.2038 ms | pass |
| 1 MiB binary roundtrip | exact | 20/20 | pass |
| deterministic adapter | stable bytes/digest | 915 bytes; one digest | pass |
| bounded process memory | measured, no invented SLA | sampled max 98.72 MiB for both paths | pass as bounded prototype evidence |
| no ambient env/fs/network | no such imports | only types-only import remains at outer boundary | pass for this prototype |

Artifact growth from 3,414 to 8,770 aggregate bytes is acceptable for the prototype and is recorded rather than hidden. The Adapter itself is 915 bytes; nesting and outer composition metadata account for the remaining per-Program growth.

Decision: retain versioned Program/Adapter composition as the preferred M8 path and retain direct Program invocation as a tested fallback. RFC-0029 and ADR-0009 stay `proposed` until their broader compiler/package/runner/security gates are satisfied.

## Evidence boundary

Verified here:

- nested Component instantiation and type identity wiring;
- Program function canonical-lower through Adapter memory;
- core trampoline invocation and canonical-lift Adapter export;
- exact same Script values and result validation on direct/composed paths;
- deterministic Adapter/Program/composed bytes;
- Wasmtime 46.0.1 correctness, latency, artifact-size and sampled-RSS evidence.

Not verified here:

- WASI CLI arguments/stdin/stdout/stderr adaptation;
- compiler-generated Program Components or dynamic scalar/aggregate codegen;
- randomized Canonical ABI property tests;
- manifest v1, package/import/capability closure and caches;
- fuel/epoch/memory limits, typed traps and malicious-guest host survival;
- Linux/macOS/mobile runner behavior.

## Reproduce

Run from a VS 2022 x64 Developer PowerShell so `link.exe` and the Windows SDK are on `PATH`:

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'
cargo run --release --locked --manifest-path .\prototypes\script-profile\Cargo.toml
```

## Links

- [`prototype`](../../prototypes/script-profile/README.md)
- [`direct-path evidence`](./script-profile-prototype-v0.md)
- [`STEP-0076`](../steps/STEP-0076-script-profile-vertical-prototype.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
