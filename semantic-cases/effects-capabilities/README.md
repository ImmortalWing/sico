# Effects and capabilities

本组验证 SEM-060、SEM-080—084：效果描述行为，能力表示授权资源；公开边界必须分别声明二者，导入名称或接收普通值都不能伪造权限。

## 必须接受

| Case | 规则 |
|---|---|
| [`CAP-001`](./valid/explicit-boundary.sico) | 公开函数同时声明效果和能力资源 |
| [`CAP-002`](./valid/pure-boundary.sico) | 纯函数明确没有外部效果和能力 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`CAP-101`](./invalid/undeclared-capability.sico) | `UNDECLARED_CAPABILITY` | capability console is not declared at this boundary |
| [`CAP-102`](./invalid/undeclared-effect.sico) | `UNDECLARED_EFFECT` | missing declared effect: console.write |

## 语义摘要

- `Console` 是 Runtime 提供的不可伪造能力资源；
- `effects console.write` 不授予 Console 能力；
- `capabilities console` 不隐藏函数实际执行的写效果；
- `none` 是空集合，不是自动推导或通配。

## Component/WIT 映射

`Console` 映射为受 Runtime policy 约束的 WIT import/resource。函数效果进入 Sico 语义索引，能力需求进入 Component imports 与 `.sapp` 权限闭合检查；WIT 本身不复制 Sico 的效果集合。
