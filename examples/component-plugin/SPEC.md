# Formatter component plugin specification

> 状态：代表性程序草案
> 目的：验证 WebAssembly Component、WIT 风格接口、版本、能力与隔离

## 功能

宿主加载一个实现 `Formatter` v1 接口的 `.sapp` 组件，并调用它格式化文本。

接口提供：

```text
name() -> Text
format(input: Text) -> Result<Text, FormatError>
```

样例插件 `UppercaseFormatter` 将输入转换为大写。

## 安全规则

- 插件默认无文件、网络、存储、UI 和系统能力；
- 只通过声明的 Component imports/exports 交互；
- 输入和输出有大小限制；
- 插件 trap、超时或资源超限不能使宿主崩溃；
- 接口版本不兼容时拒绝加载；
- 不允许加载原生动态库；
- 插件实例之间不共享内存和状态。

## 正式错误

```text
InterfaceMismatch(required, found)
InvalidComponent
PluginTrap
PluginTimeout
InputTooLarge
OutputTooLarge
FormatRejected(reason)
```

## 测试重点

- 正确加载 v1；
- 格式化成功；
- 接口版本不匹配；
- 缺少 export；
- 请求未授权能力；
- trap；
- 超时；
- 输入和输出超限；
- 两个插件实例隔离；
- 相同输入产生确定输出。

## AI 语义概要目标

```text
host requires: Formatter@1
plugin exports: Formatter@1
plugin imports: none
limits: input, output, memory, time
isolation: per instance
```

## 暴露的开放问题

- Sico 源码如何声明 WIT-compatible interface；
- Component 接口版本语法与兼容规则；
- host、guest、component、plugin 的术语；
- 插件加载是静态组合还是运行时动态加载；
- component handle 的类型和生命周期；
- trap、业务错误、取消和超时如何区分；
- capability deny 如何同时体现在类型、清单和 Runtime；
- 文本跨 Canonical ABI 的复制和限额；
- interface adapter 和小版本兼容；
- 插件签名、发布者身份和信任策略；
- 测试替身与真实 Component 是否使用同一接口；
- AI 如何理解跨组件调用图和影响范围。
