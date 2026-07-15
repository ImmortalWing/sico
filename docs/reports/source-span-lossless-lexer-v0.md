# Report: Source, span, and lossless lexer v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0016
> - environment: Windows, rustc/cargo 1.97.0, Unicode 17.0 tables

## 1. Question

RFC-0006 能否被实现为严格、有界且无损的 source/lexer，而不引入 parser 或 M2 语义？

## 2. Method

- source tests 覆盖 invalid UTF-8、BOM/control/default-ignorable、LF/CRLF/bare CR、byte↔scalar coordinate、range boundary、source/line limit；
- lexer tests 覆盖 token golden、XID/NFC、string/punctuation/trivia error、identifier/token/token-count/diagnostic limit；
- 直接读取 21 个 contract JSON case；
- 对 54 个 B canonical 文件逐一执行 strict source + lex + zero-error + byte reconstruction；
- Clippy `-D warnings`、workspace tests、M0 回归与链接检查。

## 3. Results

```text
STEP_0016_OK contract=21 b_cases=54 source_tests=6 lexer_tests=8 utf8=pass line_index=pass lossless=pass limits=pass
```

| Gate | Result |
|---|---:|
| source unit tests | 6/6 pass |
| lexer unit/corpus tests | 8/8 pass |
| RFC contract cases | 21/21 executed |
| B canonical lexical corpus | 54/54 zero errors |
| token byte reconstruction | 54/54 byte-identical |
| exact/+1 hard limits | pass |
| actual parser behavior | not implemented |

## 4. Interpretation

UTF-8 bytes、span、line index 和 tokenization 已有真实 Rust 行为证据，不再只是 RFC。source error 与 lexical error 保持 typed 分离；lexer 的 error token 总会消费至少一个 scalar；未达到 token limit 时所有非 EOF token range 连续覆盖完整 source。

## 5. Limitations

- E1xxx code、文本/JSON renderer 和 recovery 属于 STEP-0018；
- confusable/cross-script lint 仍 proposed；
- parser 尚未证明 54 个 B 文件的 grammar；
- Default_Ignorable table 随 Unicode 版本升级必须复审；
- 极端性能与任意 bytes fuzz 由 STEP-0021 审计。

## 6. Links

- [`STEP-0016`](../steps/STEP-0016-source-span-lossless-lexer.md)
- [`RFC-0006`](../rfc/RFC-0006-lexical-source-contract-v0.md)
- [`sico-source`](../../crates/sico-source/src/lib.rs)
- [`sico-lexer`](../../crates/sico-lexer/src/lib.rs)
