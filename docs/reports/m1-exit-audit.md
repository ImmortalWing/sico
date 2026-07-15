# Report: M1 exit audit

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0021
> - conclusion: GO to M2 static semantics

## 1. Decision

M1 compiler frontend and diagnostics 的全部退出门槛已满足，允许进入 M2 static semantics。该 `GO` 只证明 B source/lexer/parser/recovery/formatter/CLI frontend；它不是 type checker、Sico IR、Component codegen、`run`、REPL 或 Runtime 已实现的声明。

## 2. Evidence standard

- `proven`：当前实现、测试、golden、validator 和本次真实命令直接证明；
- `measured, non-SLA`：当前 Windows/release corpus timing，可复现但不承诺跨机器阈值；
- `deferred by phase`：明确属于 M2/M3 及以后，不是 M1 缺口；
- `missing/contradicted`：任一必需项落入此类则不得退出。

## 3. Requirement-by-requirement audit

| ID | M1 requirement | Strong evidence | Result |
|---|---|---|---|
| M1-01 | strict UTF-8 source、统一 byte span/line index 与冻结限额 | STEP-0015/0016、RFC-0006、21 contract cases | proven |
| M1-02 | B lossless lexer，trivia/comment 可供 formatter 重建 | STEP-0016，8 tests，54/54 corpus | proven |
| M1-03 | B lossless parser 与 semantic declaration AST，语义负例不被 syntax 拒绝 | STEP-0017，54 snapshots，25 accept + 29 semantic reject syntax success | proven |
| M1-04 | 12 mutation 主要根因、有界 recovery、E1xxx text/JSON span | STEP-0018，12 snapshots，E1001–E1012，construct 外 cascade max 0 | proven |
| M1-05 | error tree 不进入后续 semantic AST | `Parse::ast() == None` on error；12/12 tests | proven |
| M1-06 | canonical formatter AST stable、幂等、comment/trivia policy | STEP-0019，54/54 + 54/54，12 error-tree refusals | proven |
| M1-07 | `sico check` file/stdin、text/JSON、exit contract | STEP-0020 real-binary integration；RFC-0001 envelope | proven |
| M1-08 | `format`/`outline` integration 与 M2 capability boundary | STEP-0020，stdout/check/write、text/JSON，`type_checker=unavailable` | proven |
| M1-09 | fuzz/property/size/depth/token/error limits，无 panic/hang | 4,096 arbitrary bytes + 4,096 valid UTF-8；depth 256；parser errors 100；STEP-0016 lexer limits | proven for deterministic M1 suite |
| M1-10 | corpus performance 有可复现基线且不伪造 SLA | 66 files × 200 iterations × 3 release runs；median 1061 ms / 4.190 MiB/s | measured, non-SLA |
| M1-11 | workspace quality、前序 STEP 与 M0 资产持续回归 | fmt、Clippy `-D warnings`、29 Rust tests、STEP-0015–0021、M0 validator | proven |
| M1-12 | 未实现语义/后端不伪成功 | 29 semantic reject 仍 syntax success；CLI 明示 type checker unavailable；`run/build/-c/REPL` 未注册 | proven boundary |

没有 M1 必需项为 `missing` 或 `contradicted`。

## 4. Fuzz and limit evidence

```text
STEP_0021_OK arbitrary_bytes=4096 valid_utf8=4096 depth_limit=256 parser_errors=100 canonical=54 mutations=12 perf_runs=3 perf_iterations=200 median_ms=1061 median_mib_s=4.190 sla=not-established
```

- arbitrary bytes 先经过 strict source contract；被接受的输入继续 lex/parse；
- valid random UTF-8 组合覆盖 Unicode identifiers、trivia、comments、strings、blocks 与 delimiters；
- 每个 validated source 的 syntax tree text 与 source byte-identical；成功 parse 才 format，并再次验证 parse/idempotence；
- block/delimiter depth 256 边界接受，257 产生一个 bounded `NestingLimitExceeded` error；
- 2,000 个 unexpected delimiter 只保留 100 个 parser diagnostics；
- source/line/token/token-count/lex-diagnostic 限额继续由 STEP-0016 精确边界测试覆盖。

这是固定 seed 的确定性 property suite，不等同于长期覆盖引导 fuzz 服务或形式化安全证明。

## 5. Performance evidence

环境：Windows、Rust 1.97.0、release。每轮 54 canonical + 12 mutation，共 23,314 bytes；成功 source 执行 source+lex+parse+format，mutation 执行 source+lex+parse+diagnostic classification。

| Run | Iterations | Total bytes | Elapsed | Throughput |
|---:|---:|---:|---:|---:|
| 1 | 200 | 4,662,800 | 1061 ms | 4.190 MiB/s |
| 2 | 200 | 4,662,800 | 1049 ms | 4.236 MiB/s |
| 3 | 200 | 4,662,800 | 1121 ms | 3.966 MiB/s |
| median | 200 | 4,662,800 | 1061 ms | 4.190 MiB/s |

原始受控资产见 [`m1-frontend-windows-release.json`](../../tests/performance/m1-frontend-windows-release.json)。该数据只用于回归比较和发现数量级退化；M1 不建立延迟或吞吐 SLA。

## 6. Deferred register

| Item | Current truth | Owner/gate |
|---|---|---|
| E2xxx–E7xxx semantic diagnostics | 24 codes/29 mappings 是设计 oracle，尚无 type checker producer | M2 plan STEP-0022–0029 |
| 完整 expression/pattern HIR 与 name/type/effect/resource facts | M1 AST 只有稳定顶层 declaration shape | M2 STEP-0022 起 |
| Semantic Index real producer | RFC-0002 fixtures only | M2 STEP-0028 |
| Sico IR、Component codegen、`run`/`build` | 未实现 | M3 |
| `-c`、REPL、package/runtime/host | 未实现 | M3/M4 及以后 |
| real AI、Android、Future/Stream Runtime roundtrip | 仍按 M0 register，无新外部证据 | 各自授权/阶段 gate |

## 7. M2 authorization

进入 M2 不授权实现未确定语义。执行 [`M2 static semantics plan`](../plans/M2-static-semantics.md)；若 P0 oracle、SEMANTICS 草案、RFC-0003/0004 proposed 状态或 prelude/intrinsic surface 不足以唯一决定行为，必须先暂停并新增 RFC/案例，不得把选择藏进 Rust 类型或 visitor 分支。

## 8. Final conclusion

`GO: M1 complete; M2 entry gate satisfied; next STEP-0022.`
