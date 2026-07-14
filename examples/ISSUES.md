# Cross-sample issue matrix

> 状态：第一轮问题汇总
> 样本：10 个代表性程序
> 原则：本文件归纳问题和决策顺序，不在此直接定稿语法

## 1. 样本代号

| 代号 | 样本 |
|---|---|
| H | Hello World |
| C | Calculator |
| O | Order State |
| A | Account Transfer |
| T | Todo Store |
| W | Word Count |
| HTTP | HTTP Service |
| F | Concurrent Fetch |
| P | Component Plugin |
| N | Notes UI |

当前共有 1 个最小程序、9 份行为规格和 9 份候选 A 源码。

## 2. 第一轮总体结论

样本说明 Sico 的主要难点不在词法分析，而在以下边界：

1. 如何让类型、错误、状态和副作用足够显式，同时避免源码过度冗长；
2. 如何用一种统一模型覆盖同步值、异步值、流、资源和 Component handle；
3. 如何区分业务错误、能力拒绝、取消、超时、trap 和 Runtime 崩溃；
4. 如何表达事务、不变量和状态转换，使 AI 可以直接理解并分析影响；
5. 如何让语言核心、标准库、WIT 平台接口和 UI SDK 各自保持边界；
6. 如何避免候选 A 中同一个符号承担多种语义；
7. 如何让编译器生成比源码更紧凑、可信的语义概要。

因此不能直接按候选 A 实现解析器。必须先确定核心语义，再生成候选 B、C。

## 3. 类型与数据模型

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| T01 | 整数、浮点、Decimal、Money、溢出和舍入 | C、A、W | 财务与跨平台结果不一致 |
| T02 | `newtype`/opaque 类型的语义和零成本表示 | A、T、N | AI 混用相同底层类型的领域值 |
| T03 | record 构造、不可变更新和字段演进 | A、T、N、HTTP | 更新语法分裂、schema 难迁移 |
| T04 | enum/代数数据类型和携带数据的变体 | C、O、A、T、F、N | 状态和错误无法完整表达 |
| T05 | `Option`、`Result` 与嵌套组合 | C、A、T、HTTP、N | 分支复杂、错误传播含糊 |
| T06 | List、Map、集合唯一性和持久集合 | T、F、N | 更新成本与语义不清 |
| T07 | Text 的 UTF-8、标量、字素簇和单词边界 | W、HTTP、P、N | 跨平台统计和限额错误 |
| T08 | Path、Url、Duration、Size 等标准领域类型 | W、HTTP、F、P | 单位混淆和权限泄漏 |
| T09 | 泛型、类型推导和显式类型参数 | C、T、P、N | AI 难判断实例化类型 |
| T10 | 函数值、lambda、回调和捕获 | T、HTTP、F、N | 隐藏状态与调用目标 |

### 必须统一回答

- 哪些类型属于语言核心，哪些属于标准库；
- 值类型与资源类型是否使用同一泛型模型；
- 类型推导在哪些边界停止；
- record schema 和 Component interface 版本如何关联；
- 闭包捕获是否进入效果、能力和语义索引。

## 4. 控制流、匹配与错误

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| C01 | `if`、`match`、guard 和表达式/语句边界 | C、O、A、T、HTTP、F、N | 同一逻辑多种写法 |
| C02 | 通配分支是否隐藏新增状态或事件 | O、HTTP、F、N | 新功能静默进入旧默认路径 |
| C03 | 元组、列表和嵌套 Result 模式 | C、O、HTTP、N | 模式语法复杂、错误恢复困难 |
| C04 | 多行表达式的续行与结束规则 | O、A、HTTP、F、P、N | AI 易产生缩进和配对错误 |
| E01 | `ok/error/try/map_error` 的正式语义 | C、A、T、HTTP、P | 错误被隐式转换或吞掉 |
| E02 | 业务错误、解析错误和存储错误的映射 | C、A、T、HTTP | 错误类型爆炸或信息丢失 |
| E03 | 业务错误、取消、超时、trap、panic 的分层 | F、P、HTTP、W | 调用者采取错误恢复策略 |
| E04 | `main`、handler 和 Component export 如何失败 | C、HTTP、P、N | Runtime 与用户程序职责不清 |

### 必须统一回答

- 可预期失败是否全部使用一个 `Result` 模型；
- 是否存在不可恢复 panic，以及谁可以触发；
- `try` 是否只传播相同错误，还是允许声明式转换；
- match 通配分支在封闭类型新增变体时如何进入影响分析；
- 函数体是表达式还是显式 return 模型。

## 5. 状态、契约与事务

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| S01 | 默认不可变与显式局部可变 | A、T、W、N | 数据流难追踪或流处理过度冗长 |
| S02 | record 更新与集合内元素更新 | A、T、N | 隐式原地修改破坏推理 |
| S03 | 状态机使用普通 enum+match 还是专用模型 | O、T、N | 两套表达或状态图不可信 |
| K01 | 前置条件、后置条件和不变量 | O、A、T | 正确性只存在于注释和测试 |
| K02 | 契约证明、运行时检查和测试生成边界 | A、O | 性能、可靠性和实现复杂度冲突 |
| X01 | 事务的语言、库或 WIT 边界 | A、T、N | 部分写入和跨组件原子性不明 |
| X02 | revision、stale result 和并发更新 | T、N、A | 后返回结果覆盖新状态 |

### 必须统一回答

- 状态变化是否始终返回新值；
- 哪些场景允许受控内部可变；
- `contract` 是否进入语言核心；
- 事务只适用于存储还是通用效果作用域；
- 编译器如何把不变量加入 AI 概要和影响分析。

## 6. 效果、能力与资源

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| FX01 | 效果由显式声明还是局部推导 | H、C、A、W、HTTP、F、P、N | 签名冗长或隐藏副作用 |
| FX02 | `use` 是模块导入、能力申请还是二者 | H、C、A、W、HTTP、F、P、N | 源码与 `.sapp` 权限不一致 |
| FX03 | 能力通过参数、import、effect 还是 manifest | A、W、HTTP、F、P、N | 权限无法静态闭合 |
| R01 | file、stream、component 等资源生命周期 | W、HTTP、F、P | 泄漏、use-after-close、取消不清理 |
| R02 | `with`、RAII、所有权和显式 close | A、W、P | 同一语法承担更新和资源作用域 |
| R03 | CPU、内存、body、文件、时间限额 | W、HTTP、F、P、N | 不可信应用资源耗尽 |
| R04 | 能力拒绝时的信息泄漏 | W、P、N | 暴露宿主路径和身份 |

### 必须统一回答

- effect 与 capability 是否是两个独立静态集合；
- 编译器如何从调用图汇总 `.sapp` 权限；
- 资源类型是否可复制、可跨 async、可跨 Component；
- 限额由类型、调用参数、manifest 还是 Runtime policy 决定；
- 同一关键字不能同时表达 record update 和资源作用域，除非语法完全无歧义。

## 7. 异步、并发与流

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| A01 | `async/await` 是否进入表层语法 | HTTP、F、P、N | 函数颜色与调用链污染 |
| A02 | future、task、stream、iterator 的关系 | W、HTTP、F、N | 多套组合和取消模型 |
| A03 | 结构化并发、task group 和子任务逃逸 | F、HTTP | 任务泄漏、生命周期不可见 |
| A04 | 取消是错误、状态还是效果 | W、HTTP、F、P | 清理和恢复策略不一致 |
| A05 | timeout 和时钟能力 | F、P、HTTP | 测试依赖真实时间 |
| A06 | collect-all、fail-fast、顺序和背压 | F、W、HTTP | 数据丢失、内存失控 |
| A07 | WASI 0.3 async/future/stream 映射 | W、HTTP、F、P、N | Runtime ABI 版本锁定 |

### 必须统一回答

- Sico 异步模型是否直接对应 Component async ABI；
- 所有异步任务是否必须属于可见作用域；
- 取消时契约和资源清理是否保证执行；
- stream 与普通集合能否共享同一变换 API；
- 测试如何注入虚拟时钟、网络和调度器。

## 8. 模块、Component 与平台接口

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| P01 | Sico interface 到 WIT 的一一映射 | P、HTTP、N | 重复 ABI 和绑定层 |
| P02 | Component interface 版本和 adapter | P、N | 插件无法安全升级 |
| P03 | 静态组合与运行时动态加载 | P | 依赖图和权限难分析 |
| P04 | HTTP、storage、UI 属于 WASI 还是 Sico WIT | A、T、HTTP、N | 平台边界反复变化 |
| P05 | Component trap、限额和宿主错误 | P、F、HTTP | 沙箱错误混入业务错误 |
| P06 | app exports 和生命周期 | HTTP、N | Runtime 依赖特殊函数名 |
| P07 | 跨组件调用图和语义索引 | P、N | AI 只能理解单个源码包 |

### 必须统一回答

- Sico `interface` 是否只是 WIT 的语言映射；
- 何时生成 adapter，谁负责兼容检查；
- Runtime 提供的最小 world；
- 动态插件是否允许请求能力；
- `.sapp` manifest、Component imports 和源码能力必须如何一致。

## 9. UI 与应用模型

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| UI01 | 保留、即时、声明式树或原生控件映射 | N | Runtime 和应用职责不清 |
| UI02 | Model/Event/Effect 是否为标准模式 | N、O、T | 为 UI 发明语言专用语义 |
| UI03 | View 节点、布局、样式和 key | N | 跨平台不一致、性能差 |
| UI04 | handler 闭包与隐藏状态 | N | AI 无法追踪事件影响 |
| UI05 | 生命周期、暂停恢复和旧异步结果 | N、F | 移动端数据丢失 |
| UI06 | 可访问性、输入法、键盘与触摸 | N | 后期无法补齐平台语义 |

UI 目前只有一个样本，不能据此确定专用语法。需要先把 UI 需求限制为 WIT/SDK 问题，除非更多样本证明语言级支持必要。

## 10. AI 理解与工具协议

| ID | 问题 | 样本 | 风险 |
|---|---|---|---|
| AI01 | 模块 purpose 从源码、文档还是行为推导 | 全部 | 摘要不可信或依赖自然语言 |
| AI02 | 状态图和拒绝路径 | O、T、N | AI 漏掉终止和非法状态 |
| AI03 | 契约和不变量进入语义索引 | A、T | 修改破坏业务约束 |
| AI04 | async/task/effect 调用图 | HTTP、F、N | 只追踪同步调用导致漏影响 |
| AI05 | 跨 Component 影响分析 | P、N | 插件升级影响不可见 |
| AI06 | 测试与符号、转换、路由的关联 | C、O、HTTP、N | 修改后不知道应运行哪些测试 |
| AI07 | 低 token slice 的完整性 | A、HTTP、P、N | 压缩时删除关键上下文 |

必须为 `outline/describe/slice/impact/flow` 定义统一 JSON 协议，并用全部样本验证，不为单一场景输出专用自然语言。

## 11. 候选 A 语法冲突

候选 A 已出现以下潜在过载：

| 形式 | 当前用途 | 问题 |
|---|---|---|
| `use` | 模块导入、能力获得 | 两种概念混合 |
| `with` | record 更新、资源作用域 | 完全不同的生命周期语义 |
| `error` | Result 构造、模式 | 与业务 Error 类型易混 |
| `_` | 忽略绑定、默认分支、错误吞并 | 容易隐藏新增情况和信息丢失 |
| `=>` | match 分支、lambda | 多行边界和错误恢复困难 |
| `end` | 函数、match、if、record update、resource scope | 长距离配对过多 |
| `<...>` | 泛型、Component interface 版本组合 | 解析和错误提示复杂 |
| 方法链换行 | error 映射、异步组合 | 续行规则尚未定义 |

这不是候选 A 失败的结论，而是候选 B、C 必须重点改进和测量的部分。

## 12. 统一决策顺序

按照依赖关系处理，不按样本顺序处理：

1. **值与类型语义**：数字、Text、record、enum、newtype、Option、Result、集合、资源；
2. **函数与控制流**：参数、返回、match、guard、lambda、不可变与局部可变；
3. **错误分层**：业务错误、能力错误、取消、超时、trap、panic；
4. **效果与能力**：静态集合、调用图传播、manifest 和 Component imports；
5. **资源与异步**：所有权、清理、future、stream、结构化并发、WASI 映射；
6. **契约与事务**：不变量、证明边界、原子性、revision 和 stale result；
7. **模块与 Component**：interface/WIT、版本、组合、动态加载；
8. **平台 SDK**：storage、HTTP、UI、应用生命周期；
9. **AI 语义协议**：索引、概要、程序切片、状态图、影响分析；
10. **候选 B/C 表层语法**：在语义稳定后为同一批样本重新编码；
11. **量化比较**：生成成功率、理解 token、诊断质量、单轮修复率；
12. **最小规范定稿**：只保留被多场景证明必要的结构。

## 13. 优先级

### P0：阻塞语法和编译器

- T01-T10：核心类型与函数模型；
- C01-C04：控制流与语法边界；
- E01-E04：错误分层与传播；
- FX01-FX03：效果和能力；
- A01-A07：异步和资源模型。

### P1：阻塞 Runtime 和 `.sapp`

- R01-R04：资源和限额；
- P01-P07：WIT、Component 和平台边界；
- X01-X02：事务与并发更新。

### P2：进入生态前完成

- UI01-UI06：UI 与生命周期；
- AI01-AI07：完整 AI 语义工具协议；
- 签名、更新、发布者身份和分发。

AI 的基础 `check/describe` 仍属于早期能力；P2 指完整跨组件、跨平台影响分析。

## 14. 下一项交付物

不再增加新样本。第一版跨样本决策已写入 [`SEMANTICS.md`](../SEMANTICS.md)，其中从值与类型开始，覆盖错误、效果、资源、异步、事务、Component 边界和 AI 语义要求。

全部样本的第一轮复核已经记录在 [`SEMANTICS-AUDIT.md`](./SEMANTICS-AUDIT.md)。下一步为 P0 语义建立最小成功/失败案例，再为同一语义设计候选 B、C。

在关键 P0 语义完成验证前，不创建正式解析器实现。
