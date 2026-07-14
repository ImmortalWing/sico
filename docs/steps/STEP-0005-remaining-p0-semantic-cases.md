# STEP-0005: 补齐剩余 P0 语义正反例

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

为效果/能力、affine 资源、Future/Task、Stream、Component 调用和 revision 六组 P0 语义分别建立 2 个最小合法程序与 2 个单根因非法程序，并同步 A0、B、C 三套候选语法表示。

## 2. Context and evidence

- 效果与能力：SEM-080—084；
- affine 资源：SEM-090—093；
- Future/Task：SEM-072、SEM-100—104、SEM-106；
- Stream：SEM-105；
- Component：SEM-074、SEM-120—123；
- revision：SEM-113；
- [`examples/SEMANTICS-AUDIT.md`](../../examples/SEMANTICS-AUDIT.md) 要求每组至少包含合法、非法、短诊断、语义摘要和 Component/WIT 映射说明；
- 当前 4 组首批 P0 判定集有 30 个 A0 程序和各 30 个 B/C 镜像。

## 3. Scope

包含：

- 6 组 ×（2 valid + 2 invalid）= 24 个新 A0 案例；
- 24 个 B 镜像与 24 个 C 镜像；
- 每组 README 中的判定、默认短诊断、语义摘要与 WIT/Component 映射；
- 候选语法新增结构的临时表层约定；
- 全量案例 manifest 和镜像/元数据校验器。

不包含：

- 接受任何新语义规则或选出语法赢家；
- 声称案例已经通过 parser/type checker；
- 确定最终 WIT 包名、WASI 版本或异步 ABI；
- 扩展 STEP-0004 的 AI 任务集；
- 实现编译器、Runtime 或 Component 原型。

## 4. Options and decision

### 每组只做一正一反

满足最低审计要求，但只能证明单一表面路径，无法同时覆盖声明边界和消费/故障边界。

### 直接把代表性应用作为测试

覆盖丰富，但一个程序包含多个未决规则，非法结果无法保持单一主要诊断。

### 决定：每组 2 正 2 反的最小矩阵

每组一个合法案例覆盖核心声明/类型关系，另一个覆盖调用或消费路径；两个非法案例分别覆盖最常见的遗漏和越权/重复消费。案例继续使用设计判定，不能冒充编译结果。

新增表层形式只属于 A0/B/C 候选镜像：A0 使用通用 `end`，B 使用带名称结束标记，C 使用花括号和方括号集合。它们不改变 `SEMANTICS.md` 的语义结论。

## 5. Plan

1. 建立 6 组 README 和 24 个 A0 案例；
2. 生成并人工复核 B/C 的一一对应表示；
3. 创建全量案例 manifest；
4. 实现 case ID、accept/reject、语义引用、镜像路径和组计数校验；
5. 更新 `SEMANTICS.md`、`SYNTAX.md` 和案例索引；
6. 运行 JSON、PowerShell、链接、whitespace 和回归检查；
7. 编写报告、提交并推送。

## 6. Changes

- 新增 effects-capabilities、affine-resources、future-task、stream、component-call、revision 六组语义案例；
- 每组新增 2 valid + 2 invalid，并为 B/C 创建相同相对路径和判定元数据的镜像；
- 新增 [`semantic-cases/manifest.json`](../../semantic-cases/manifest.json) 和 [`validate-semantic-cases.ps1`](../../tools/validate-semantic-cases.ps1)；
- 更新 `SEMANTICS.md` 的验证状态、`SYNTAX.md` 的第二轮临时表层形式以及案例/候选索引；
- 新增 [`remaining-p0-semantic-cases`](../reports/remaining-p0-semantic-cases.md) 报告并更新项目状态。

## 7. Validation

已验证：

- A0/B/C 各 54 个正式语义案例；
- 54 个 case ID 唯一并在三套候选中一一对应；
- `valid/` 全部为 `accept`，`invalid/` 全部为 `reject(KEY)`；
- B/C 的 case、expect、semantics 与 A0 完全一致；
- 6 个新增组各为 2 valid + 2 invalid；
- STEP-0003 mutation 来源与 STEP-0004 AI 任务仍通过校验；
- Markdown 链接、JSON、PowerShell 与暂存区 whitespace 检查。

核心输出：

```text
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
```

## 8. Metrics

实际新增：24 个 A0 + 24 个 B + 24 个 C = 72 个 `.sico`。正式语义判定集从 30 增至 54 个 case，三套候选共 162 个对应程序；仓库总 `.sico` 为 190。

没有 parser/type checker，因此接受率、诊断位置和运行结果均为 not measured。

## 9. Risks and follow-ups

- 新增异步、资源和 Component 写法只是候选表层，必须等 parser/AI 数据后决定；
- WIT 映射只规定语义形状，不锁定未稳定的 ABI 版本；
- 案例数量仍不足以覆盖资源借用、任务策略、流组合和 adapter 兼容全部边界；
- STEP-0006 应建立诊断协议 v0 和稳定错误编号分区。

## 10. Audit links

- report: [`remaining-p0-semantic-cases`](../reports/remaining-p0-semantic-cases.md)
- commit subject: `test(semantics): [STEP-0005] add remaining P0 case matrix`
- next: STEP-0006 diagnostics protocol v0
