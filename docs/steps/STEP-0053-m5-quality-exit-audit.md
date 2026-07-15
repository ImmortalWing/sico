# STEP-0053: Real desktop app, quality/performance and M5 exit audit

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

用代表性 signed desktop app、security/lifecycle properties、release startup baseline 和全阶段回归关闭 M5，并产出可直接执行的 M6 Android Host 计划。

## 2. Changes

- 新增 Hello Desktop source 与 typed UI companion；
- 新增 10,240 个 app/signer identity、capability fingerprint、hostile UI/open request property inputs；
- 新增可复现 Windows release startup measurement，3 x 20 次真实 Wasmtime launch，median mean 32.344 ms，无 SLA；
- 更新 M4 exit validator 为阶段单调断言，避免后续阶段被误判为回退；
- 完成 M5 requirement audit、状态/路线图/index/README，并规划 STEP-0054–0061。

## 3. Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
SICO_TEST_WASMTIME=<wasmtime-46.0.1> cargo test --workspace --all-targets --all-features
signed representative app -> Desktop Host -> 42
typed UI companion -> validate-only -> 4 nodes
M5_PERFORMANCE_OK runs=3 iterations=20 median_mean_startup_ms=32.344
M5_EXIT_OK steps=8 threat_cases=24 host_tests=16 desktop_tests=6 properties=10240 windows=runtime-verified macos=contract-verified linux=contract-verified regression=M0-M4 audit=GO next=STEP-0054
```

## 4. Decision and limits

M5 GO is accepted. macOS/Linux stay contract-verified and are not counted as native runtime success. The performance number is a local comparison baseline, not an SLA. UI compiler binding, production signing/installer/update and Android remain outside M5.

## 5. Audit links

- [`M5 exit audit`](../reports/m5-exit-audit.md)
- [`M6 plan`](../plans/M6-android-host.md)
- [`performance record`](../../tests/performance/m5-desktop-host-windows-release.json)
- [`STEP-0052`](./STEP-0052-desktop-platform-adapters-parity.md)

