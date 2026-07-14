# AI evaluation fixtures

这些文件只测试离线工具，不是模型输出：

| Fixture | Purpose |
|---|---|
| [`pass-run.json`](./pass-run.json) | 三种类别均提供规范正确输出，应得满分 |
| [`fail-run.json`](./fail-run.json) | 提供代码围栏、非法 JSON 和错误修复，应命中固定失败分类 |
| [`invalid-model-run.json`](./invalid-model-run.json) | 缺少正式模型元数据，评分器必须拒绝 |

fixture 运行使用 `kind: fixture` 与 `synthetic: true`。评分器输出的 `official_comparison` 必须为 `false`。
