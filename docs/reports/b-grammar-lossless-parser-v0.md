# Report: B grammar lossless parser happy path v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0017
> - environment: Windows, Rust 1.97.0, rowan 0.16.1

## 1. Question

RFC-0005 B 的 54 个 canonical case 能否全部形成 byte-identical rowan tree 和稳定顶层 AST shape，同时不把 29 个 M2 语义负例误报为 syntax error？

## 2. Method

parser 只消费 STEP-0016 lexer token；按显式 opener/`end <kind>`、圆/方 delimiter 建树；对全部 B 文件检查 lex/parse zero error、root text equality、非空 declaration AST，并与 54-line snapshot 比较。

## 3. Results

```text
STEP_0017_OK b_cases=54 syntax_accept=54 designed_accept=25 semantic_reject_not_executed=29 snapshots=54 lossless=pass
```

- B parse：54/54；
- syntax root byte identity：54/54；
- AST shape snapshot：54/54；
- M0 design accept：25/25 syntax success；
- M2 semantic reject：29/29 仍 syntax success；
- syntax/parser Rust tests：3/3。

## 4. Interpretation and limits

这证明 happy-path structure，不证明 mutation recovery。semantic AST 当前只含顶层 declaration kind/name/range；没有名称绑定、类型、match 完整性、资源或能力判断。STEP-0018 必须在不改变 54 个成功 shape 的前提下加入 error/missing node 与有界恢复。

## 5. Links

- [`STEP-0017`](../steps/STEP-0017-b-grammar-lossless-parser.md)
- [`AST snapshots`](../../tests/parser/b-ast-shapes.txt)
- [`parser`](../../crates/sico-parser/src/lib.rs)
- [`syntax`](../../crates/sico-syntax/src/lib.rs)
