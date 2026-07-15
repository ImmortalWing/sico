# STEP-0021: 完成 fuzz/performance 与 M1 exit audit

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

用确定性属性输入证明 source/lexer/parser/formatter 在任意 bytes、有效随机 UTF-8、深嵌套、长 token、缺失 close 和随机 delimiter 下不 panic/hang，并冻结 parser depth/error 上限；记录 54-file corpus 的可复现性能基线，逐项审计 M1 exit gate。

## 2. Limits under review

- source bytes：16 MiB；physical line：1 MiB（RFC-0006 已接受）；
- lexer token：1 MiB；token count：1,000,000；lex diagnostics：100（STEP-0016 已验证）；
- parser block/delimiter nesting：256；parser diagnostics：100（本步骤新增并验证）；
- 超限只产生 bounded engineering failure/error tree，不进入 semantic AST，不分配未经证据审查的稳定 E code。

## 3. Evidence plan

1. 固定 seed 的 arbitrary-byte source validation；
2. 固定 seed 的 valid UTF-8 token/delimiter/block 组合，执行 lex/parse，成功时执行 format/parse；
3. depth=256 边界成功、257 超限 bounded failure；大量 unexpected delimiter 诊断截断到 100；
4. 54 canonical + 12 mutation 重复运行，记录总 bytes、iterations、elapsed 和 throughput，只作当前机器 baseline，不承诺跨机器 SLA；
5. 重跑 STEP-0016–0020、workspace Clippy/tests、M0 validator；
6. requirement-by-requirement M1 audit；只有全部 proven 才把 M1 标记 complete。

## 4. Non-goals

不引入 libFuzzer 在线服务、不把单机 timing 当产品 SLA、不实现 M2 检查、不为随机输入扩展 grammar 或自动修复、不声称消除所有安全漏洞。

## 5. Commit

`test(frontend): [STEP-0021] prove M1 fuzz limits and exit`

## 6. Changes and validation

- parser 冻结 block/delimiter depth 256 与 diagnostic cap 100；超限保持 lossless error tree，禁止 semantic AST；
- 新增 4,096 arbitrary-byte + 4,096 valid-random-UTF-8 确定性 property suite；
- 精确验证 depth 256/257、delimiter depth、2,000 unexpected delimiters→100 errors；
- release benchmark 对 66-file corpus 执行 200 iterations × 3，median 1061 ms / 4.190 MiB/s，明确无 SLA；
- [`M1 exit audit`](../reports/m1-exit-audit.md) 逐项证明全部 12 项 gate，结论 GO to M2；
- workspace fmt/Clippy/tests、STEP-0015–0021 和 M0 regression 全部通过。

```text
M1_EXIT_OK steps=7 source=pass lexer=54/54 parser=54/54 recovery=12/12 formatter=54/54 cli=3 property_inputs=8192 depth_limit=256 parser_errors=100 semantic_reject_syntax_success=29 m2_plan=ready next=STEP-0022
```

## 7. Risks and next

固定 seed property suite 不是长期 coverage-guided fuzz 服务；performance 只用于当前机器回归。M2 从 STEP-0022 full AST/HIR/name contract 开始；遇到 prelude/type/control-flow 语义歧义必须先 RFC/case，不得提前实现。
