# Report: canonical formatter v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0019
> - environment: Windows, Rust 1.97.0

## 1. Question

成功解析的 RFC-0005 B source 能否收敛到唯一布局，并在全部 54 个 canonical case 上保持 AST shape、二次格式化 byte-identical，同时拒绝对 error tree 猜测修复？

## 2. Method

formatter 以 lossless lexer token 为内容事实源，以成功 parser 结果作为前置条件；逐行重建 token spacing，并按显式 labeled block、match arm 和 effects/capabilities 清单计算 2-space indentation。外部 golden 注入 CRLF、tab、多余空白、连续空行和行注释；corpus test 对每个文件执行 parse→format→parse→format。

## 3. Results

```text
STEP_0019_OK b_cases=54 ast_stable=54 idempotent=54 mutations_rejected=12 indent=2 line_endings=LF blank_lines=max-1 comments=preserved golden=pass
```

- format→parse AST shape stable：54/54；
- second format byte-identical：54/54；
- B mutation 拒绝：12/12，每例返回 typed syntax failure，无部分输出；
- policy golden：CRLF→LF、tab/多空格→canonical spacing、2-space nested indentation、最多一个空行、恰好一个末尾换行；
- comment：内部文本保留，独占行随上下文缩进，尾随 comment 前两个空格，行尾 layout 空白移除。

## 4. Interpretation and limits

v0 不做 line wrapping、declaration sorting 或自动修复。token spelling 不改变，因此该结果证明布局唯一性，不暗示 M2 名称、类型、效果或资源规则已经实现。格式化错误源必须先由 parser/diagnostics 报告。

## 5. Links

- [`STEP-0019`](../steps/STEP-0019-canonical-formatter.md)
- [`formatter`](../../crates/sico-format/src/lib.rs)
- [`policy golden`](../../tests/formatter/policy.snap)
