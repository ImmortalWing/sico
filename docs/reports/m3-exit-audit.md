# M3 exit audit

> - status: complete
> - date: 2026-07-15
> - conclusion: GO to M4 `.sapp` and secure Runtime
> - environment: Windows; Rust 1.97.0 x86_64-pc-windows-gnu; Wasmtime 46.0.1; wasmparser/wasm-encoder 0.253.0

## 1. Conclusion

M3 的退出门槛全部满足：Sico 已有独立验证的 typed IR、确定 lowering、Core Wasm/Component、真实 WIT/Runtime 边界，以及同步 scalar `build/run` 的 source→semantics→IR→Component→Wasmtime 链。该 GO 只接受仓库中已证明的子集，不声称 aggregate adapter、通用 arbitrary `Int`、effect/capability compiler host、async compiler lowering、`.sapp` 或 sandbox 已实现。

## 2. Requirement-by-requirement evidence

| Gate | Evidence | Result |
|---|---|---|
| typed IR 与独立 verifier | RFC-0008；14 types、19 operations、5 terminators；canonical JSON；9 core + 4 flow mutation classes；100 diagnostic cap | proven |
| invalid source 不进入 IR | 25/25 M2 valid、29/29 exact invalid；lowering cumulative 17/25 valid，29 invalid 全部在 semantic gate 拒绝 | proven |
| deterministic lowering/artifact | canonical IDs/order；2,048 scalar sources 重复 Component byte-equal；1,000-function Component 重复 byte-equal | proven |
| real Core Wasm/Component validation | wasmparser 0.253；Core numeric/control 与 compiler Component artifacts | proven |
| selected Runtime execution | Wasmtime 46.0.1；CLI Int/Bool/Unit 输出 `42`/`true`/`()` | proven |
| WIT host boundary | Result Ok/Err、512-byte Int record、Decimal record、owned/borrowed/drop resource、host output 50 | proven structural subset |
| async Runtime contract | future roundtrip/close/cancel；stream roundtrip/close/capacity 1-of-5；stable hashes | proven Runtime subset |
| unsupported compiler async | Task/Future/Stream feature-specific typed refusal，不模拟执行 | proven boundary |
| minimal application entry/CLI | RFC-0014；file/stdin build、Runtime lookup、stdout/stderr/exit、no-overwrite/no-artifact failure、temporary cleanup | proven |
| property/limits/performance | 2,048 source inputs；1,000 functions；3 Runtime cases；3 × 1,000-iteration release runs；median 51.055 ms / 3.213 MiB/s；无 SLA | measured/proven |
| prior phase regression | STEP-0030–0037、M2、M1、M0 validators；workspace format/strict Clippy/tests | proven |

## 3. Semantic and representation boundary

- compiler-generated Component 只接受 flat Bool、Unit 与 compile-time proven fitting `Int`；RFC-0003 的 arbitrary `Int`/Decimal 仍只有 canonical encoding 与真实 WIT record roundtrip，不是通用 compiler representation；
- Result/value record/resource 已证明 WIT/Canonical ABI 结构和 host behavior，但 compiler 尚无 aggregate memory/realloc、resource import 或 effect adapter；
- RFC-0004 接受 Wasmtime Future/Stream Runtime behavior；compiler 因缺少 task scope、cancel edge、stream bound 的完整 IR contract 继续 typed-refuse；
- effectful function 现在在 codegen 前显式拒绝，修复了可能静默忽略 effect metadata 的风险；
- Wasmtime compile cache 在 M3 source-run 显式关闭，避免在 package identity、权限和 invalidation contract 前产生隐式持久状态。

## 4. CLI and artifact contract

`sico build` 当前输出 raw `.component.wasm`，不是应用包。已有目标拒绝覆盖；所有 source diagnostic/backend failure 不创建或修改目标。`sico run` 使用内存 artifact 和进程唯一临时文件，Runtime stdout/stderr 分离转发，非零 Runtime 状态归类 exit 2。trap/domain error/cancel/timeout/capability denial、cache、args/stdio package behavior 在 M4 重新冻结。

## 5. Quality evidence

三轮 release measurement 每轮构建 3,000 个 Components（Int/Bool/Unit corpus × 1,000），每轮 artifact 总字节 303,000。原始数据见 [`m3-component-windows-release.json`](../../tests/performance/m3-component-windows-release.json)。这些数值只建立当前 Windows/GNU/release 基线，不是跨平台 SLA。

## 6. Deferred and risks

| Deferred item | Phase/reason |
|---|---|
| `.sapp` manifest/archive/hash/development signature/inspect | M4 package identity and trust contract |
| capability closure、WASI host、isolated storage、limits/fault taxonomy | M4 security boundary |
| aggregate/arbitrary Int/effect/resource compiler adapters | M4，必须随 package capability/runtime contract 验证 |
| compiler Task/Future/Stream lowering | M4+，需完整 structured concurrency IR/cancel/bound case gate |
| Desktop/Android host 与 UI | M5/M6 |
| production publisher identity、registry、updates、LSP | M7 |
| real model benchmark 与跨平台 performance | 外部凭据/环境；不阻塞 M3 semantic/toolchain gate |

## 7. Authorization

M4 只按 [`M4 plan`](../plans/M4-sapp-runtime.md) 从 threat model 和 package contract 开始。不得把 raw Component 改名为 `.sapp`，不得在 manifest/hash/signature/capability closure 前接受任意 package，也不得用开发签名冒充生产发布者身份。

`GO: M3 complete; M4 entry gate satisfied; next STEP-0038.`

## 8. Links

- [`M3 plan`](../plans/M3-sico-ir-component.md)
- [`STEP-0036`](../steps/STEP-0036-minimal-end-to-end-cli.md)
- [`STEP-0037`](../steps/STEP-0037-m3-quality-exit-audit.md)
- [`M4 plan`](../plans/M4-sapp-runtime.md)
