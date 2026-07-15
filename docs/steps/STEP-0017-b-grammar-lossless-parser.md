# STEP-0017: 实现 B grammar happy-path 无损 parser

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 RFC-0005 B labeled-block 实现 rowan lossless syntax tree 与最小 semantic AST shape；54/54 canonical case 必须成功，且其中 29 个 M2 语义负例不得被 parser 拒绝。

## 2. Context and evidence

- RFC-0005 是唯一 grammar surface；
- STEP-0016 lexer 已证明 B 54/54 零词法错误和 byte reconstruction；
- semantic-case `expect: reject(...)` 是 M2 oracle，不是 syntax error。

## 3. Scope

包含：SyntaxKind/Language/tree、B declaration/block/delimiter happy path、顶层 declaration AST shape、54-file snapshot/corpus tests。

不包含：mutation recovery、稳定 E1xxx、formatter、CLI、名称/类型/效果/能力/资源检查。

## 4. Decision

parser 使用手写 line-aware event/tree construction：lexer token 是唯一切分；块由 RFC-0005 opener 与 `end <kind>` 匹配；rowan token raw kind 编码 lexer kind，原始 text/trivia 全保留。semantic AST 仅列出顶层声明 kind/name/range。

## 5. Validation plan

- 54/54 B parse without syntax/lex errors；
- 25 accept + 29 semantic reject 均 syntax success；
- syntax root text 与 source byte-identical；
- 54 AST shape snapshot stable；
- representative nesting tests；workspace/M0 regression。

## 6. Non-goals and risks

STEP-0018 才实现 recovery；本步骤不得为通过 mutation 放宽 grammar。parser 深度/size fuzz 在 STEP-0021。

## 7. Commit

`feat(parser): [STEP-0017] parse B happy paths losslessly`

## 8. Changes and validation

- `sico-syntax` 新增 raw node/token kind、rowan Language 与 typed aliases；
- `sico-parser` 新增 B block/delimiter parser、lossless tree、ModuleAst/Declaration shape；
- 54-line [`AST snapshot`](../../tests/parser/b-ast-shapes.txt) 入库；
- 54/54 B parse、root text byte-identical、snapshot stable；25 accept + 29 semantic reject 全部 syntax success；
- syntax/parser tests 3/3，workspace Clippy/tests、STEP-0016/M0 regression 和 links pass。

```text
STEP_0017_OK b_cases=54 syntax_accept=54 designed_accept=25 semantic_reject_not_executed=29 snapshots=54 lossless=pass
```

## 9. Risks and next

当前 error 只是结构类型，尚无稳定 code、missing/error node 或 recovery anchor；STEP-0018 必须命中 12 mutation，并保持本步骤 54 个 shape 不变。
