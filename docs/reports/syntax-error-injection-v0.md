# Report: 候选语法单点错误注入 v0

> - status: complete
> - date: 2026-07-14
> - related-step: STEP-0003
> - environment: Windows NT 10.0.26100.0, Windows PowerShell 5.1.26100.8655

## 1. Question

能否为 A0、B、C 三套候选语法建立一组来源合法、变异方式明确、可以机械复现的单点结构错误输入，作为未来解析器恢复和 AI 修复评测的固定基线？

本报告不回答哪套语法的真实解析恢复或 AI 修复效果更好。

## 2. Method

从现有 `accept` 语义案例中选择 6 种结构，为每套候选语法各创建一个非法变体：

| ID | 单点错误 | 预期恢复锚点 |
|---|---|---|
| MUT-001 | 缺少函数结束标记 | 下一定义 `main` |
| MUT-002 | 缺少 match arm 分隔标记 | 下一分支 `Color.Blue` |
| MUT-003 | 缺少 record 结束标记 | 下一定义 `main` |
| MUT-004 | 缺少类型参数结束标记 | `read_value` 函数体 |
| MUT-005 | 缺少 enum 结束标记 | 下一定义 `AppError` |
| MUT-006 | 缺少构造调用结束括号 | `main` 函数结束位置 |

[`manifest.json`](../../syntax-mutations/manifest.json) 为每个变体声明来源、唯一来源片段、替换文本、预期诊断键和恢复锚点。校验器去除来源与变体的元数据行后，确认来源是 `accept` case、来源片段只出现一次，并验证执行一次声明替换后的正文与变体正文逐字相同。

## 3. Reproduction

在仓库根目录执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
```

成功输出：

```text
MUTATION_CORPUS_OK entries=18 A0=6 B=6 C=6
```

## 4. Raw evidence

- 变异清单：[`syntax-mutations/manifest.json`](../../syntax-mutations/manifest.json)；
- 人类可读变体：[`syntax-mutations/`](../../syntax-mutations/README.md)；
- 校验器：[`tools/validate-syntax-mutations.ps1`](../../tools/validate-syntax-mutations.ps1)；
- 来源基线：[`semantic-cases/`](../../semantic-cases/README.md) 与 [`syntax-candidates/`](../../syntax-candidates/README.md)。

变体和 manifest 一同纳入版本控制；后续修改来源案例时，校验器会因逐字不一致而失败，要求显式更新基线。

## 5. Results

| 检查项 | 结果 | 证据等级 |
|---|---:|---|
| manifest 条目 | 18 | verified |
| A0/B/C 变体 | 6 / 6 / 6 | verified |
| 唯一 `mutation + syntax` 键 | 18 | verified |
| 来源为 `accept` case | 18 / 18 | verified |
| 来源片段恰好出现一次 | 18 / 18 | verified |
| 单次替换逐字重建变体 | 18 / 18 | verified |
| 预期诊断和恢复元数据完整 | 18 / 18 | verified |
| 实际 parser 主要诊断 | 未测量 | not measured |
| 实际级联诊断数量 | 未测量 | not measured |
| 实际恢复距离 | 未测量 | not measured |
| AI 单轮修复率 | 未测量 | not measured |

结构上，B 的带名称结束标记携带更多闭合类型信息，A0 的通用 `end` 携带的信息最少，C 的括号边界紧凑且常见；这些只是待验证假设，不能由当前静态语料推导出优胜者。

## 6. Interpretation

STEP-0003 已建立可审计的错误输入基线：每个非法程序都能追溯到同一语法候选中的合法程序，并能由一项声明变异重建。未来 parser、诊断器或 AI 评测可以复用相同输入，避免不同实验临时手写不同错误。

当前结果只验证语料构造，不验证 `SYNTAX_MISSING_*` 是否会成为最终错误编号，也不验证解析器能否到达声明的恢复锚点。

## 7. Limitations

- 变异是文本片段替换，不是正式 lexer token 变异；
- 只有 6 类结构错误和 18 个样本；
- 不覆盖缩进、字符串、数字、模块、能力、资源、异步和 Component 语法；
- 尚无 parser、类型检查器或模型运行结果；
- 不同候选删除的字符和 token 数并不完全相同，本轮对齐的是结构错误意图。

## 8. Decision impact

- STEP-0004 可直接使用该语料定义离线修复评分与模型执行协议；
- M1 parser 完成后，应把实际主要诊断、级联数量和恢复距离追加为 measured 报告；
- 本报告不触发 A0/B/C 的选择或合并决定。

## 9. Links

- 执行记录：[`STEP-0003`](../steps/STEP-0003-syntax-error-injection-v0.md)；
- 语法评测原则：[`SYNTAX.md`](../../SYNTAX.md)；
- 静态语法指标：[`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)。
