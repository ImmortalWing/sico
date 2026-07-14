# Result error mapping

本组验证 SEM-034、SEM-064、SEM-070 和 SEM-071：

- 同一种错误可以显式传播；
- 不同错误类型必须显式、完整地映射；
- Result 必须处理、返回或传播；
- 封闭错误 enum 不能用 catch-all 隐藏新增变体；
- 映射后的业务错误仍与取消和 trap 分层。

## 必须接受

| Case | 规则 |
|---|---|
| [`RESULT-001`](./valid/same-error-try.sico) | `try` 传播相同错误类型 |
| [`RESULT-002`](./valid/total-error-map.sico) | 完整映射每个来源错误变体 |
| [`RESULT-003`](./valid/explicit-result-handle.sico) | 调用者显式处理 Ok 与 Error |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`RESULT-101`](./invalid/error-type-mismatch.sico) | `ERROR_TYPE_MISMATCH` | cannot propagate ParseError as AppError |
| [`RESULT-102`](./invalid/incomplete-error-map.sico) | `INCOMPLETE_ERROR_MAP` | missing error mapping: Invalid |
| [`RESULT-103`](./invalid/catch-all-error-map.sico) | `SEALED_ERROR_WILDCARD` | wildcard hides sealed error variants: Invalid |
| [`RESULT-104`](./invalid/unhandled-result.sico) | `UNHANDLED_RESULT` | Result must be handled, returned, or propagated |

错误映射允许主动合并多个已知来源变体，但必须逐个写明。未来新增来源错误后，旧映射必须重新检查。
