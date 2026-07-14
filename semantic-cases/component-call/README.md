# Component calls

本组验证 SEM-074、SEM-120—123：Sico interface 映射 WIT，版本属于组件身份元数据，跨组件调用必须保留领域错误、取消与 trap 的分层。

## 必须接受

| Case | 规则 |
|---|---|
| [`COMP-001`](./valid/wit-safe-interface.sico) | 可导出接口只使用 WIT 可表示类型 |
| [`COMP-002`](./valid/separated-call-outcome.sico) | 调用返回 `ComponentCall<T, E>`，不折叠故障层次 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`COMP-101`](./invalid/version-as-type.sico) | `COMPONENT_VERSION_IN_TYPE` | component version is package metadata, not a type argument |
| [`COMP-102`](./invalid/trap-as-domain-error.sico) | `COMPONENT_FAILURE_COLLAPSE` | ComponentCall cannot be returned as a domain Result |

## 语义摘要

- `interface` 是 WIT 接口的语言映射，不定义另一套 ABI；
- `Component<Formatter>` 是实现该接口的资源句柄；
- `call` 返回 `ComponentCall<T, E>`，其结果可区分 `Returned(Result<T,E>)`、取消和 trap；
- 组件版本由包解析与 adapter 处理，不进入 `Formatter` 或泛型类型。

## Component/WIT 映射

`Formatter.format` 直接映射 WIT function，`FormatError` 映射 WIT variant。Component call 的领域 Result、异步取消和 trap 在绑定层保持不同通道；版本保存在包/接口身份中。
