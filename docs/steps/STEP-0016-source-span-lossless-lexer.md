# STEP-0016: 实现 source/span 与无损 lexer

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

按 RFC-0006 实现严格 UTF-8 source、统一 UTF-8 byte span、line index 与 lossless lexer；让 STEP-0015 的 21 个 contract case、54 个 B canonical 文件和 trivia byte reconstruction 成为真实 Rust behavior tests。

## 2. Context and evidence

- [`RFC-0006`](../rfc/RFC-0006-lexical-source-contract-v0.md) 是本步骤唯一 lexical/source 规范；
- [`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md) 固定 byte/line/column 坐标；
- [`contract-v0.json`](../../tests/lexical/contract-v0.json) 提供 7 positive/14 negative oracle；
- 54 个 B canonical case 是正式 lexer corpus；A0/C 不进入主前端。

## 3. Scope

包含：SourceId/TextRange/Span、严格 bytes 校验、line index/roundtrip、TokenKind/Token/LexError、Unicode XID + NFC、trivia/literal/operator、无损覆盖、限额与测试。

不包含：B grammar、parser recovery、稳定 E1xxx code、formatter、CLI 或 M2 语义。

## 4. Options and decision

- source 先验证 bytes，再创建 `SourceFile`；非法 UTF-8 不进入 lexer；
- range 使用锁定的 `text-size`，所有 line/column 由一个 `LineIndex` 派生；
- lexer 使用显式单遍状态机而非 regex，便于 byte range、限额和错误前进性审计；
- source error 与 lexical error 先用 typed kind；稳定 code 等 STEP-0018 的真实 recovery/span 数据。

## 5. Plan

1. 实现 source validation、span 与 line index；
2. 实现 RFC-0006 token/trivia 和 Unicode/literal 错误；
3. 接入 contract JSON、token golden、line property、limit 与 B corpus tests；
4. 运行 fmt/Clippy/test、STEP validator、M0 回归；
5. 完成审查记录、提交并推送。

## 6. Changes

- `sico-source`：SourceId/Span、strict bytes validation、SourceFile、LineIndex 与双向 byte/scalar coordinate；
- `sico-lexer`：RFC-0006 全 token/trivia、XID/NFC、string error、无损 reconstruction 与硬限额；
- 21 contract case 由 Rust test 读取并执行；54 个 B file 逐一 zero-error/reconstruct；
- 新增 STEP validator 与 [`review report`](../reports/source-span-lossless-lexer-v0.md)。

## 7. Validation

实际：workspace format pass；Clippy `-D warnings` pass；source 6/6、lexer 8/8；contract 21/21；B lexer 54/54；byte reconstruction 54/54；line index scalar-boundary roundtrip pass；exact/+1 limits pass；STEP-0015/M0 回归和 Markdown links pass。

```text
STEP_0016_OK contract=21 b_cases=54 source_tests=6 lexer_tests=8 utf8=pass line_index=pass lossless=pass limits=pass
```

## 8. Metrics

- Rust behavior tests: 14 pass；
- contract: 7 positive + 14 negative；
- B lexical corpus: 54 files，0 lexical errors，54 byte-identical reconstruction；
- verified limits: 16 MiB source、1 MiB line/token、1024-byte identifier、1,000,000 token、100 diagnostics。

## 9. Risks and follow-ups

- Unicode/security table 随版本升级仍需复审；
- confusable lint 和 E1xxx 尚未实现；
- parser 只能消费 lexer 结果，不能重新切分 source；
- STEP-0017 才定义 grammar/tree shape。

## 10. Audit links

- commit topic: `feat(lexer): [STEP-0016] implement source spans and lossless lexer`
- report: [`source/span and lossless lexer v0`](../reports/source-span-lossless-lexer-v0.md)
- next: STEP-0017 B happy-path parser
