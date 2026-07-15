# Syntax candidate static metrics v1

> - status: verified full P0 snapshot
> - scope: 54 mirrored cases per candidate
> - machine snapshot: [`metrics-v1.json`](./metrics-v1.json)
> - generator/checker: [`measure-syntax-candidates.ps1`](../tools/measure-syntax-candidates.ps1)

这些指标比较相同语义的源码表层，不是模型 tokenizer、parser 恢复或 AI 成功率。所有候选使用同一规则，删除 case 元数据与空行后统计非空源码行、UTF-8 bytes、候选无关词法 token、标点 token 和块关闭标记。

## Reproduce

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/measure-syntax-candidates.ps1 -CheckPath syntax-candidates/metrics-v1.json
```

成功输出以 `SYNTAX_METRICS_OK` 开头。若任何候选源码或快照漂移，检查失败。

## Full results

| Candidate | Files | Lines | UTF-8 bytes | Lexical tokens | Punctuation tokens | Block closers | Labeled closers |
|---|---:|---:|---:|---:|---:|---:|---:|
| A0 | 54 | 518 | 10,067 | 2,229 | 836 | 149 | 0 |
| B | 54 | 563 | 13,217 | 2,668 | 910 | 151 | 151 |
| C | 54 | 488 | 9,990 | 2,494 | 1,266 | 149 | 0 |

相对 A0：

| Candidate | Lines | UTF-8 bytes | Lexical tokens | Punctuation tokens |
|---|---:|---:|---:|---:|
| B | +8.7% | +31.3% | +19.7% | +8.9% |
| C | -5.8% | -0.8% | +11.9% | +51.4% |

## Round split

| Cohort | Candidate | Files | Lines | UTF-8 bytes | Lexical tokens | Punctuation tokens |
|---|---|---:|---:|---:|---:|---:|
| round 1 | A0 | 30 | 285 | 4,883 | 1,123 | 421 |
| round 1 | B | 30 | 316 | 6,770 | 1,386 | 456 |
| round 1 | C | 30 | 246 | 4,766 | 1,265 | 657 |
| round 2 | A0 | 24 | 233 | 5,184 | 1,106 | 415 |
| round 2 | B | 24 | 247 | 6,447 | 1,282 | 454 |
| round 2 | C | 24 | 242 | 5,224 | 1,229 | 609 |

第一轮数值与历史快照一致；v1 首次把 effects/capabilities、resource、Future/Task、Stream、Component 和 revision 的 24 个案例纳入同一生成器。

## Interpretation boundary

- A0 的 lexical token 最少，但 149 个通用 `end` 都不声明关闭的是哪种结构；
- B 多 19.7% lexical token，代价主要来自完整关键字和 151 个带结构名称的结束标记；
- C 的 bytes 最少，但 punctuation token 比 A0 多 51.4%；花括号提供配对种类，却不携带 `function`、`match`、`resource` 等结构名称；
- `labeled closers = 151` 是可验证的冗余信息量，不证明 parser 或模型一定恢复得更好；
- 字符、词法 token 和标点均不能冒充固定模型 token，真实 token 必须由已记录版本的 tokenizer 或供应商统计产生。

这些数据足以量化简洁度与结构冗余的取舍，但真实模型生成/修复率和 parser 级联仍保持 `not measured`。
