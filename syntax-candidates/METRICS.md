# Syntax candidate static metrics v1

> - status: verified full P0 snapshot
> - scope: 58 mirrored cases per candidate
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
| A0 | 58 | 685 | 20,678 | 2,575 | 885 | 224 | 0 |
| B | 58 | 730 | 24,424 | 3,164 | 1,024 | 226 | 226 |
| C | 58 | 655 | 20,601 | 2,915 | 1,465 | 224 | 0 |

相对 A0：

| Candidate | Lines | UTF-8 bytes | Lexical tokens | Punctuation tokens |
|---|---:|---:|---:|---:|
| B | +6.6% | +18.1% | +22.9% | +15.7% |
| C | -4.4% | -0.4% | +13.2% | +65.5% |

## Round split

| Cohort | Candidate | Files | Lines | UTF-8 bytes | Lexical tokens | Punctuation tokens |
|---|---|---:|---:|---:|---:|---:|
| round 1 | A0 | 30 | 285 | 4,883 | 1,123 | 421 |
| round 1 | B | 30 | 316 | 6,770 | 1,386 | 456 |
| round 1 | C | 30 | 246 | 4,766 | 1,265 | 657 |
| round 2 | A0 | 28 | 400 | 15,795 | 1,452 | 464 |
| round 2 | B | 28 | 414 | 17,654 | 1,778 | 568 |
| round 2 | C | 28 | 409 | 15,835 | 1,650 | 808 |

第一轮数值与历史快照一致；v1 首次把 effects/capabilities、resource、Future/Task、Stream、Component 和 revision 的 24 个案例纳入同一生成器，M11 (STEP-0104) 又为 resource/task 规则补充 4 个结构化并发负例。

## Interpretation boundary

- A0 的 lexical token 最少，但 224 个通用 `end` 都不声明关闭的是哪种结构；
- B 多 22.9% lexical token，代价主要来自完整关键字和 226 个带结构名称的结束标记；
- C 的 bytes 最少，但 punctuation token 比 A0 多 65.5%；花括号提供配对种类，却不携带 `function`、`match`、`resource` 等结构名称；
- `labeled closers = 226` 是可验证的冗余信息量，不证明 parser 或模型一定恢复得更好；
- 字符、词法 token 和标点均不能冒充固定模型 token，真实 token 必须由已记录版本的 tokenizer 或供应商统计产生。

这些数据足以量化简洁度与结构冗余的取舍，但真实模型生成/修复率和 parser 级联仍保持 `not measured`。
