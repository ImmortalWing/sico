# Report: 剩余 P0 语义案例矩阵

> - status: complete
> - date: 2026-07-14
> - related-step: STEP-0005
> - environment: Windows NT 10.0.26100.0, Windows PowerShell 5.1.26100.8655

## 1. Question

能否把效果/能力、affine 资源、Future/Task、Stream、Component 调用和 revision 的现有语义草案转换为单根因、可镜像、可供未来编译器直接使用的最小正反例？

## 2. Method

每组固定 4 个 case：2 个 `accept` 覆盖声明与消费/调用路径，2 个 `reject(KEY)` 覆盖常见遗漏或非法操作。A0 是设计判定源，B/C 只改变表层写法，不改变 case ID、预期判定或 SEM 引用。

| 组 | Valid | Invalid | 主要拒绝边界 |
|---|---:|---:|---|
| effects-capabilities | 2 | 2 | 未声明能力、未声明效果 |
| affine-resources | 2 | 2 | move 后使用、close 后使用 |
| future-task | 2 | 2 | Future 重复等待、Task 逃逸 |
| stream | 2 | 2 | 无界收集、遗漏 await |
| component-call | 2 | 2 | 版本进入类型、trap 折叠为领域错误 |
| revision | 2 | 2 | 提交缺少 revision、未检查旧结果 |

每组 README 同时记录默认短诊断、语义摘要和 Component/WIT 映射边界。全量 [`manifest.json`](../../semantic-cases/manifest.json) 固定组计数，校验器检查路径、case ID、accept/reject、SEM 引用及 B/C 镜像。

## 3. Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-cases.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
```

## 4. Raw evidence

- 判定集入口：[`semantic-cases/`](../../semantic-cases/README.md)；
- B/C 镜像：[`syntax-candidates/`](../../syntax-candidates/README.md)；
- 机器可读组计数：[`manifest.json`](../../semantic-cases/manifest.json)；
- 校验器：[`validate-semantic-cases.ps1`](../../tools/validate-semantic-cases.ps1)；
- 执行记录：[`STEP-0005`](../steps/STEP-0005-remaining-p0-semantic-cases.md)。

## 5. Results

| 检查项 | 结果 | 证据等级 |
|---|---:|---|
| 新增语义 case | 24 | verified |
| 新增 `.sico` 程序（A0/B/C） | 72 | measured |
| 全量 case | 54 | verified |
| valid / invalid | 25 / 29 | verified |
| A0 / B / C 程序 | 54 / 54 / 54 | verified |
| 三套候选总语义程序 | 162 | measured |
| provisional reject key | 24 unique / 29 invalid cases | measured |
| B/C 路径与元数据镜像 | 54 / 54 each | verified |
| SEM 引用存在 | 162 / 162 | verified by mirrored metadata |
| STEP-0003 mutation 回归 | pass | verified |
| STEP-0004 AI harness 回归 | pass | verified |
| parser/type checker 接受率 | 无 | not measured |
| 实际诊断位置和消息 | 无 | not measured |
| WIT/Component 运行 | 无 | not measured |

核心输出：

```text
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
```

## 6. Interpretation

六个原本只有文字规则的 P0 领域现在具备最小判定边界。未来 parser/type checker 可以直接把 25 个合法 case 作为 compile-pass、29 个非法 case 作为 compile-fail，并让三套候选使用同一语义 oracle。

这些结果证明的是语料组织和设计一致性，不证明案例表层语法已经可解析，也不证明 proposed 诊断键是最终稳定编号。

## 7. Limitations

- 每组只有 4 个 case，不覆盖完整 borrow/share、取消观察点、任务策略、Stream 组合、adapter 或 revision 回绕；
- A0/B/C 新增写法仍是候选，尚无 parser 或 formatter 验证；
- Component/WIT 映射只固定语义形状，没有锁定 ABI/WASI 版本；
- `UNCHECKED_STALE_RESULT` 等规则需要未来静态分析证明来源与应用之间的 revision 关系；
- 新 24 个 case 尚未加入 AI 评测和静态体积指标。

## 8. Decision impact

- M0 的 P0 设计判定集从 4 组扩展为 10 组；
- STEP-0006 可以基于全部 29 个非法 case 和 24 个 provisional key 定义诊断编号、短消息与 JSON 协议；
- 当前仍没有数据支持选择 A0、B 或 C；
- 资源、异步和 WIT 的实现可行性仍需独立 Rust/Component 原型。

## 9. Links

- 核心语义：[`SEMANTICS.md`](../../SEMANTICS.md)；
- 候选语法：[`SYNTAX.md`](../../SYNTAX.md)；
- 跨样本审计：[`examples/SEMANTICS-AUDIT.md`](../../examples/SEMANTICS-AUDIT.md)。
