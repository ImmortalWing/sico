# STEP-0045: M4 security quality and exit audit

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

完成 package/parser/runtime security property corpus、determinism/performance baseline、M0–M3 regression、M4 requirement audit，并产出可执行 M5 计划。

## 2. Context and evidence

STEP-0038–0044 已逐层完成 threat contract、strict package、signing/trust、capability closure、storage/WASI、limits/fault 与 package CLI/cache。本步骤只在全部证据重跑通过后给出 M4 GO。

## 3. Scope

包含 signed/package mutation、Component integrity、resource-order determinism、explicit trust、真实 Runtime adversarial regression、workspace quality、Windows release non-SLA baseline、M4 exit audit 和 M5 plan。不包含 Desktop Host 实现。

## 4. Decision

接受 [`M4 exit audit`](../reports/m4-exit-audit.md) 的 GO，并将 [`M5 Desktop Host plan`](../plans/M5-desktop-host.md) 标记为 ready。下一步必须先执行 STEP-0046 threat/lifecycle/platform contract。

## 5. Changes

- 新增 3,328-case package security/determinism property corpus和 empty-trust refusal；
- 新增 3 × 1,000 package build/verify/signature-verify release baseline；
- 增加 M4 exit validator，串行重跑 STEP-0038–0045 与 M0–M3 exit；
- 修正 M3 exit validator 为 monotonic phase assertion，防止后续阶段产生伪 regression；
- 完成 M4 audit、状态/路线图/index 和 STEP-0046–0053 M5 plan。

## 6. Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
SICO_TEST_WASMTIME=<wasmtime-46.0.1> cargo test --workspace --all-targets --all-features
STEP_0045_OK signed_mutations=2048 component_mutations=1024 resource_permutations=256 threat_cases=18 perf_runs=3 perf_operations=3000 median_build_ms=5.459 median_verify_ms=8.571 median_signature_ms=48.177 sla=not-established audit=GO next=STEP-0046
M4_EXIT_OK steps=8 threat_cases=18 package_properties=3328 trust=ed25519-dev capability=closed storage=isolated runtime=limited cli=build-run-inspect cache=verified regression=M0-M3 audit=GO next=STEP-0046
```

## 7. Metrics

18 threat classes；3,328 property cases；12 package tests；6 Runtime tests；8 CLI tests；3 performance runs × 3,000 operations；median build/verify/signature verify 5.459/8.571/48.177 ms；SLA not established。

## 8. Risks and follow-ups

microbenchmark 不代表 Desktop startup。非 Windows platform、production identity、long-running storage quota、guest args/stdin、async lifecycle 和 GUI isolation 均已明确递延；M5 不得把计划当成已验证实现。

## 9. Audit links

- [`M4 exit audit`](../reports/m4-exit-audit.md)
- [`M5 plan`](../plans/M5-desktop-host.md)
- [`performance evidence`](../../tests/performance/m4-sapp-windows-release.json)
- [`STEP-0044`](./STEP-0044-package-cli-source-cache.md)
