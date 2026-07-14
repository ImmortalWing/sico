# Semantic query fixtures

五个合法 request/response pair 分别固定 `outline`、`describe`、`slice`、`impact`、`flow` 的 v0 结构。三个非法响应只保留一个协议根因：

| Fixture | 期望 | 根因 |
|---|---|---|
| `outline` | accept | 10 个代表性模块的低 token 地图 |
| `describe` | accept | 签名、效果、能力、错误和测试依据 |
| `slice` | accept | 保留类型、错误、契约和依赖理由 |
| `impact` | accept | 变更、直接影响、传递测试与路径 |
| `flow` | accept | 成功状态边和覆盖剩余组合的拒绝边 |
| `stale-snapshot` | reject | response 不能来自另一 snapshot |
| `untrusted-verified` | reject | source 文本本身不能把推断升级为 verified |
| `complete-but-truncated` | reject | 截断结果不能声称 complete |

fixtures 是设计 oracle，不是编译器输出。`used_bytes` 是 response `result` 的 canonical JSON UTF-8 bytes：对象键按 UTF-16 code unit ordinal 升序、数组保序、无额外空白，由离线校验器复算。
