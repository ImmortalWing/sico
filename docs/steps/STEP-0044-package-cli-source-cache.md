# STEP-0044: Package CLI, source cache, args and stdio

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

把 `.sapp` 固定为正式 `build/run/inspect` surface，加入可审计 source-run cache，并冻结 trust、args、stdio 与 exit contract。

## 2. Context and evidence

STEP-0039–0043 已提供 canonical package、development signature、capability closure、isolated storage 和 Runtime limits；本步骤把这些 gate 串入用户入口，不允许 raw Component 冒充最终包。

## 3. Scope

包含 deterministic package build、strict inspect、explicit-trust run、source cache、raw regression flag、scalar args refusal、stdio/exit mapping。不包含 production publisher、registry、guest stdin 或非 scalar argument ABI。

## 4. Decision

接受 [`RFC-0019`](../rfc/RFC-0019-package-cli-cache-stdio-v0.md)。cache 不是 trust root；corrupt/stale cache fail closed，不静默修复后执行。

## 5. Changes

- CLI 新增 `.sapp` default build 与 `inspect` text/JSON；
- development seed/public-key file strict parsing 与 explicit package run trust；
- source-run domain-keyed canonical package cache，每次 hit 重新 verify；
- capability grants、isolated storage 与 STEP-0043 limits 接入正式 run；
- exit 0–6、guest stdout/stderr separation、non-empty scalar args refusal；
- M3 validator 改用显式 `--raw-component` 保存 raw compiler evidence。

## 6. Validation

```text
cargo clippy -p sico-cli --all-targets --all-features -- -D warnings
SICO_TEST_WASMTIME=<wasmtime-46.0.1> cargo test -p sico-cli
STEP_0044_OK cli=build,run,inspect artifact=sapp-v0 inspect=text,json trust=explicit-dev cache=domain-keyed,verified,corrupt-refused args=scalar-refused stdio=guest-separated exits=0-6 cli_tests=8 raw_component=internal next=STEP-0045
```

## 7. Metrics

6 CLI commands；8 integration tests；3 package-facing commands；7 stable exit codes；1 signed/trusted real Runtime corpus；1 deliberate cache corruption fixture。

## 8. Risks and follow-ups

args 和 guest stdin 必须等 WIT entry ABI 后新增，不能在 v0 静默模拟。cache key 当前包含 seed bytes 后再整体哈希；production key agent/OS keystore 不属于 development policy。Windows/macOS/Linux cache placement 的产品 policy 留到 M5 host。

## 9. Audit links

- [`RFC-0019`](../rfc/RFC-0019-package-cli-cache-stdio-v0.md)
- [`review report`](../reports/package-cli-cache-v0.md)
- [`STEP-0043`](./STEP-0043-runtime-limits-fault-taxonomy.md)
