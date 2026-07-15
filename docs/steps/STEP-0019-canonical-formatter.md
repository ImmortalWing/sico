# STEP-0019: 实现 canonical formatter

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 RFC-0005 B 成功语法树建立唯一文本表示：2-space block indentation、LF、单一 token spacing、最多一个空行和单个文件末尾换行；54/54 canonical case 必须 format→parse AST shape 稳定，第二次 format byte-identical。

## 2. Boundaries

- 只格式化 lex/parse zero-error source；error tree 返回 typed failure，不猜测修复；
- 不改 identifier、literal、keyword 或 comment 的内容；只规范 trivia、缩进、换行和 comment 周边布局；
- 不换行长表达式，不排序 declaration，不执行 M2 语义；
- `effects:`/`capabilities:` 清单和 match arm indentation 只按 RFC-0005 已出现的结构处理。

## 3. Canonical policy

1. 输出仅用 LF，移除行尾空白，文件恰好一个末尾换行；
2. block 每层 2 spaces，`end <kind>` 与 opener 对齐；match arm body、effects/capabilities 清单额外一层；
3. `()[],:.@` 使用紧凑标点规则，逗号后一个空格，二元/声明 token 之间一个空格；
4. 连续空行折叠为一个，文件首尾不保留空行；
5. comment 内容保持，独占行随上下文缩进，尾随 comment 前两个空格。

## 4. Acceptance

- 54/54 B format→parse AST shape stable；
- 54/54 second format byte-identical；
- CRLF/tab/excess-space/comment fixture 归一化到唯一 golden；
- 12/12 mutation 明确拒绝，不产生部分输出；
- workspace、STEP-0016–0018 和 M0 regression 保持绿色。

## 5. Commit

`feat(format): [STEP-0019] add canonical B formatter`

## 6. Changes and validation

- `sico-format` 新增 parse-success-only API、typed lexical/syntax failure 和 token-driven canonical layout；
- 2-space block/match arm/list indentation、紧凑标点、LF、空行与末尾换行规则已固定；
- comment 内容、独占行缩进和尾随 comment 间距由外部 golden 覆盖；
- 54/54 AST stable、54/54 idempotent、12/12 mutation rejected；workspace Clippy/tests、STEP-0016–0018 与 M0 regression 全部通过。

```text
STEP_0019_OK b_cases=54 ast_stable=54 idempotent=54 mutations_rejected=12 indent=2 line_endings=LF blank_lines=max-1 comments=preserved golden=pass
```

## 7. Risks and next

v0 不做长行重排，section 清单只覆盖 RFC-0005 corpus 已固定的 identifier/dotted entries。STEP-0020 的 CLI 必须复用本 API，并让 syntax error 保持诊断失败而不是产生格式化输出。
