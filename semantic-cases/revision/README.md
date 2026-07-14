# Revision and stale results

本组验证 SEM-032、SEM-083、SEM-113：可能覆盖并发状态的写入和异步完成必须携带、比较显式 revision；不匹配产生具名冲突或明确忽略结果。

## 必须接受

| Case | 规则 |
|---|---|
| [`REV-001`](./valid/versioned-commit.sico) | 存储提交携带 expected revision 并返回新 revision/冲突 |
| [`REV-002`](./valid/stale-load-ignored.sico) | 异步加载完成先比较 revision，再应用或忽略 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`REV-101`](./invalid/missing-commit-revision.sico) | `MISSING_REVISION` | commit requires the expected revision |
| [`REV-102`](./invalid/unchecked-stale-result.sico) | `UNCHECKED_STALE_RESULT` | compare loaded.revision before applying loaded.text |

## 语义摘要

- `Revision` 是名义 token，不能与任意 Int 混用；
- commit 使用 compare-and-set 语义，不做隐藏的最后写入获胜；
- 异步完成事件携带启动时的 revision；
- 旧成功和旧失败都必须经过同一新鲜度判断。

## Component/WIT 映射

Revision 映射为具名 WIT type/record；具体整数宽度由存储接口决定。commit 返回 `result<Revision, StaleRevision>`，expected/actual token 均保留。UI 内部 revision 同样进入事件和语义索引。
