# Sico syntax mutation corpus v1

> - status: verified design corpus
> - related step: [`STEP-0003`](../docs/steps/STEP-0003-syntax-error-injection-v0.md)
> - report: [`syntax error injection v0`](../docs/reports/syntax-error-injection-v0.md)
> - extended by: [`STEP-0012`](../docs/steps/STEP-0012-syntax-evidence-completion.md), [`syntax evidence v1`](../docs/reports/syntax-evidence-v1.md)

本目录保存 A0、B、C 三套候选语法的固定单点结构错误。它是未来解析器错误恢复和 AI 修复评测的输入，不是当前编译器实测结果。

## Corpus

| Mutation | Source case | Error | Recovery anchor |
|---|---|---|---|
| MUT-001 | NUM-003 | missing function close | next definition `main` |
| MUT-002 | MATCH-001 | missing match arm separator | next arm `Color.Blue` |
| MUT-003 | NOM-002 | missing record close | next definition `main` |
| MUT-004 | RESULT-001 | missing type argument close | `read_value` function body |
| MUT-005 | RESULT-002 | missing enum close | next definition `AppError` |
| MUT-006 | NOM-002 | missing call close | `main` function close |
| MUT-007 | CAP-001 | missing capability close | next definition `announce` |
| MUT-008 | RES-002 | missing resource close | next definition `read_once` |
| MUT-009 | RES-002 | missing using close | `read_once` function close |
| MUT-010 | TASK-002 | missing task-group close | `pair` function close |
| MUT-011 | COMP-001 | missing interface close | end of file |
| MUT-012 | REV-001 | missing parameter-list close | `save` function body |

每类分别包含：

- [`a0/`](./a0/)；
- [`b/`](./b/)；
- [`c/`](./c/)。

总数：12 类 × 3 套语法 = 36 个 `.sico`。MUT-001–006 是 STEP-0003 的 v0 基线；MUT-007–012 由 STEP-0012 增加，覆盖第二轮结构。

## Metadata

每个文件头包含：

```sico
// mutation: MUT-001
// syntax: A0
// source-case: NUM-003
// expect: reject(SYNTAX_MISSING_FUNCTION_CLOSE)
// recovery: next-definition(main)
```

`expect` 与 `recovery` 目前是设计要求。只有正式解析器运行后才能把实际诊断位置、恢复距离和级联数量标记为 measured。

## Manifest and validation

[`manifest.json`](./manifest.json) 对每个变体记录：

- 来源文件；
- 变体文件；
- 唯一来源片段；
- 唯一替换结果；
- 预期诊断；
- 预期恢复锚点。

运行：

```powershell
powershell -NoProfile -File tools/validate-syntax-mutations.ps1
```

脚本会证明每个变体的正文恰好等于“来源正文应用一次声明替换”，同时检查数量、ID、元数据和未登记文件。

## Current limitations

- 变异单位是固定文本片段，不是正式 lexer token；
- 尚未实现 parser，不能测量真实恢复行为；36 个 `recovery` 均为设计 oracle；
- 尚未运行模型，不能测量修复率；
- v1 覆盖 12 类结构错误，但仍不覆盖缩进宽度、字符串转义、数字字面量、模块、能力名称拼写或语义级错误。
