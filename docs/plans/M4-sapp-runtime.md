# M4 plan: `.sapp` and secure Runtime

> - status: complete
> - created: 2026-07-15
> - phase: M4
> - entry evidence: [`M3 exit audit`](../reports/m3-exit-audit.md)
> - execution boundary: raw deterministic Component + RFC-0014 CLI

## 1. Outcome

把 M3 的 raw Component 编译/执行链升级为可重复构建、可检查、默认拒绝、受资源约束的 `.sapp` 应用包和 Runtime。M4 交付 package/manifest/hash/development signature、能力闭包、隔离存储、限额与故障分类，并冻结 package 级 `build/run/inspect`；不实现 Desktop/Android UI host、公共 registry 或生产发布者身份。

## 2. Entry and decision gates

- M3 exit gate 与 raw Component deterministic baseline 必须保持通过；
- package canonicalization、manifest version、hash/signature input、capability namespace、permission intersection、storage identity、limit/trap 分类必须先有 RFC/ADR 与恶意 fixture；
- manifest 声明、source effects 和 Component imports 必须闭合，任何未知/额外能力默认拒绝；
- 不可信包只加载 raw validated Component，禁止反序列化外部分发的 Wasmtime precompiled artifact；
- 开发签名不能伪装成生产发布者身份或平台信任；
- 所有限额必须在真实 Runtime 施加并有 bypass/cleanup 测试，不能只做 manifest 静态检查。

## 3. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0038 | `.sapp` threat model、package/manifest/canonical hash RFC | zip/path traversal、duplicate entry、size bomb、unknown field/version、hash confusion fixtures；schema 与 canonical bytes 决策 |
| STEP-0039 | deterministic `.sapp` builder、loader 与 `inspect` | byte-identical package；manifest/component/resource hashes；strict parser；corrupt/tampered package refusal |
| STEP-0040 | development signing 与 trust policy | deterministic signature input；valid/invalid/wrong-key/replay fixtures；明确非生产身份与 key handling boundary |
| STEP-0041 | capability closure 与 permission intersection | source effects = manifest requests = Component imports closure；host grant intersection；unknown/undeclared import default deny |
| STEP-0042 | WASI capability host 与隔离 storage | per-app identity/root；path traversal/symlink/cross-app denial；file/clock/random/network 的最小显式 adapter 与 cleanup |
| STEP-0043 | CPU/memory/task/handle/body/time limits 与 fault taxonomy | fuel/epoch/memory/table/handle/stream bounds；trap/domain error/cancel/timeout/host fatal 分通道；host survives malicious corpus |
| STEP-0044 | package CLI、source-run cache、args/stdio contract | `build/run/inspect` 以 `.sapp` 为正式 surface；cache key 可审计；stale/corrupt cache 不执行；参数和退出码兼容测试 |
| STEP-0045 | security fuzz、determinism/performance 与 M4 exit audit | package/parser/runtime fuzz；权限 bypass corpus；non-SLA baseline；M0–M3 regression；M5 entry plan |

## 4. Test matrix

- Package unit/property：canonical entry order、normalized paths、duplicate/unknown rejection、size/count/depth caps；
- Cryptographic fixtures：manifest/component/resource tamper、wrong key、signature stripping、version/domain separation；
- Capability integration：missing/extra/unknown imports、host grant subsets、no ambient authority；
- Storage security：absolute/parent/symlink/device names、cross-app identity、quota、crash cleanup；
- Runtime adversarial：infinite loop、memory growth、handle leak、deep call、trap during drop、cancel/timeout races；
- CLI/cache：file/stdin、build/run/inspect text+JSON、atomic output、cache determinism/corruption；
- Regression：M3 raw Component evidence remains available as internal/compiler test boundary，不再冒充最终应用包。

## 5. M4 exit gate

- 相同 source/config/key material 生成 byte-identical `.sapp`，`inspect` 能独立验证版本、hash、signature 和 capability closure；
- tampered、malformed、oversized、path-confusing 或 unknown-version package 在执行前拒绝；
- Runtime 只授予 source/manifest/import/host 四方交集，恶意包不能越权访问宿主或其他 app storage；
- CPU、memory、task、handle、body 与 time limits 在真实 Wasmtime execution 生效，trap/timeout 不使 host 崩溃；
- package `build/run/inspect`、cache、args、stdio、exit/fault contract 有审计证据；
- security fuzz/property/limit 与非 SLA performance baseline 通过，M0–M3 regression 全绿；
- Desktop/Android UI host、生产签名/registry/更新仍明确属于 M5+ / M7。

## 6. Completion

STEP-0038–0045 已全部完成，结论见 [`M4 exit audit`](../reports/m4-exit-audit.md)。下一执行项是 M5 [`STEP-0046`](../steps/README.md)：Desktop Host threat/lifecycle/platform contract。
