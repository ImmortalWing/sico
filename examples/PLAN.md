# Representative program plan

> 状态：M0 样本集
> 原则：先暴露问题，后统一设计

## 方法

每个代表性程序包含：

- `SPEC.md`：与具体语法尽量无关的行为规格；
- `candidate-a.sico`：使用同一套候选 A 风格表达；
- 开放问题：记录该场景暴露的语言、工具和平台问题。

在 10 个样本全部完成前，不因为单一样本增加正式专用语法。全部完成后，根据跨样本共性统一设计候选 B、候选 C。

## 样本集

| # | 样本 | 主要覆盖 | 状态 |
|---|---|---|---|
| 1 | Hello World | 入口、导入、控制台能力 | 已完成 |
| 2 | Calculator | 类型、枚举、匹配、Result、错误传播 | 已完成 |
| 3 | Order State | 状态转换、终止状态、测试、影响分析 | 已完成 |
| 4 | Account Transfer | 记录、领域类型、契约、事务、存储 | 已完成 |
| 5 | Todo Store | 集合、CRUD、可选值、持久化、迁移 | 已完成 |
| 6 | Word Count | 文件、流、资源关闭、Unicode、限额 | 已完成 |
| 7 | HTTP Service | 异步、路由、序列化、验证、并发请求 | 已完成 |
| 8 | Concurrent Fetch | 并发、取消、超时、任务错误、顺序 | 已完成 |
| 9 | Component Plugin | WIT、组件接口、版本、能力与隔离 | 已完成 |
| 10 | Notes UI | UI 状态、事件、生命周期、存储、跨平台 | 已完成 |

## 覆盖矩阵

| 设计问题 | 相关样本 |
|---|---|
| 入口与能力 | Hello、Calculator、HTTP、Notes UI |
| 数值与基础类型 | Calculator、Account Transfer |
| 错误模型 | Calculator、Account Transfer、Word Count、HTTP、Concurrent Fetch |
| 封闭状态与匹配 | Order State、Todo Store、Notes UI |
| 契约与不变量 | Account Transfer、Order State |
| 持久化与事务 | Account Transfer、Todo Store、Notes UI |
| 资源生命周期 | Word Count、HTTP、Component Plugin |
| 异步与并发 | HTTP、Concurrent Fetch、Notes UI |
| 取消与超时 | Concurrent Fetch、HTTP |
| Component/WIT | Component Plugin、Notes UI |
| 权限与沙箱 | Word Count、HTTP、Component Plugin、Notes UI |
| UI 与应用生命周期 | Notes UI |
| AI 状态/影响理解 | Order State、Account Transfer、Todo Store、Notes UI |

## 统一总结阶段

全部样本完成后，按以下步骤处理问题：

1. 合并重复问题；
2. 统计每个问题出现的样本数量；
3. 评估错误风险和 AI 理解成本；
4. 区分语言问题、标准库问题、WIT 平台问题和工具问题；
5. 优先复用 Wasm Component、WIT、WASI 和成熟算法；
6. 为剩余语言问题设计候选 B、C；
7. 使用同一批样本比较生成、阅读、诊断和修复指标；
8. 最后才确定语法和最小语言规范。
