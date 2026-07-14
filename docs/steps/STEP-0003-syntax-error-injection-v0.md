# STEP-0003: 建立候选语法单点错误注入集 v0

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

为 A0、B、C 三套候选语法建立第一批可复现的单点结构错误变体，用相同来源 case 比较错误局部性、预期主要诊断和恢复锚点，并为未来解析器恢复测试准备固定输入。

## 2. Context and evidence

- 语法评测规则：[`SYNTAX.md`](../../SYNTAX.md)；
- A0 判定集：[`semantic-cases/`](../../semantic-cases/README.md)；
- B/C 镜像：[`syntax-candidates/`](../../syntax-candidates/README.md)；
- 静态指标：[`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)；
- 当前三套候选各有 30 个一一对应案例，但没有正式词法器或解析器。

## 3. Scope

包含 6 类单点变异，每类覆盖 A0/B/C：

1. 删除函数结束标记；
2. 删除 match arm 分隔标记；
3. 删除 record 结束标记；
4. 删除泛型类型参数结束标记；
5. 删除 enum 结束标记；
6. 删除构造调用结束括号。

合计 18 个非法 `.sico`。

不包含：

- 实现词法器、解析器或恢复算法；
- 声称实际诊断数量或级联错误数量；
- 类型、Result、能力或资源等语义变异；
- 真实 AI 修复实验；
- 根据本轮结构样本选择语法胜者。

## 4. Options and decision

### 只保存人工变异文件

简单，但无法证明变体确实只修改一个位置，后续容易与来源漂移。

### 运行时随机 mutation

覆盖面大，但第一批结果难复现，且尚无正式词法器确认 token 边界。

### 决定：固定 manifest + 派生文件 + 校验脚本

使用 JSON manifest 记录来源、变体、唯一 source fragment、replacement、预期诊断和恢复锚点；保存人类可读变体文件，并用脚本从来源重建期望文本进行逐字验证。

这可以证明“一个声明的替换”，但不能证明解析器恢复质量。实际诊断与级联数量必须等 M1 解析器存在后追加 measured report。

## 5. Plan

1. 选择能覆盖下一定义或下一分支恢复的来源案例；
2. 为 A0/B/C 生成 18 个固定变体；
3. 编写 `manifest.json`；
4. 编写结构校验脚本；
5. 运行数量、元数据、单点替换、链接和 whitespace 检查；
6. 编写 v0 报告，明确 verified 与 not measured；
7. 更新 STATUS、ROADMAP 和步骤索引；
8. 提交并推送。

## 6. Changes

- 新增 [`syntax-mutations/`](../../syntax-mutations/README.md)，包含 A0/B/C 各 6 个、合计 18 个固定非法程序；
- 新增 [`manifest.json`](../../syntax-mutations/manifest.json)，记录来源、单点替换、预期诊断和恢复锚点；
- 新增 [`validate-syntax-mutations.ps1`](../../tools/validate-syntax-mutations.ps1)，机械验证来源、数量、元数据与单次替换等价性；
- 新增 [`syntax-error-injection-v0`](../reports/syntax-error-injection-v0.md) 报告，分离 verified 语料事实与尚未测量的 parser/AI 指标；
- 更新项目状态、路线图、步骤索引、报告索引和根文档入口。

## 7. Validation

已执行：

- 18 个 mutation 文件数量与 ID 唯一性；
- 每个 mutation 来源必须是 accept case；
- manifest 的 source fragment 在来源中唯一出现；
- 应用一次 replacement 后必须逐字等于 mutation body；
- 每个变体包含 reject diagnostic 与 recovery anchor；
- Markdown/JSON/PowerShell 基础检查；
- `git diff --cached --check`。

核心校验输出：

```text
MUTATION_CORPUS_OK entries=18 A0=6 B=6 C=6
```

没有解析器，因此没有输出虚构的 parser diagnostic 或 cascade count。

## 8. Metrics

完成数据：6 类 × 3 套语法 = 18 个变体；18/18 来源为 accept case；18/18 可由一次声明替换逐字重建。

后续 measured 指标需要 M1：主要诊断位置、级联数量、恢复到下一定义/分支的距离、修复轮数。

## 9. Risks and follow-ups

- 字符片段替换不是正式 token mutation；正式词法器完成后要升级 manifest；
- 不同候选的结束标记 token 数不同，本轮比较的是同一结构错误而非相同字符删除数；
- 固定样本规模小，只用于建立恢复测试骨架；
- 下一步骤应定义 AI/离线修复评测协议，不直接选择胜者。

## 10. Audit links

- report: [`syntax-error-injection-v0`](../reports/syntax-error-injection-v0.md)
- commit subject: `test(syntax): [STEP-0003] add syntax error injection corpus`
- next: STEP-0004 AI evaluation protocol
