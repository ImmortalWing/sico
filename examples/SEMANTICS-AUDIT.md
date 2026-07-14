# Candidate A semantic audit

> - 状态：第一轮完成
> - 对象：1 个最小程序与 9 个 `candidate-a.sico`
> - 语义基线：[`SEMANTICS.md` 0.1-draft](../SEMANTICS.md)
> - 原则：检查程序含义，不在本轮修改表层语法

## 1. 审计目的

本报告回答三个问题：

1. 十个代表性程序能否使用当前核心语义表达；
2. 候选 A 源码在哪里违反或无法完整表达语义草案；
3. 哪些缺口属于语言核心，哪些应留给标准库、WIT、Runtime 或表层语法。

审计结论不把候选 A 当作可编译规范，也不因为发现问题就立即增加新关键字。

## 2. 结果分类

| 标记 | 含义 | 后续动作 |
|---|---|---|
| **符合** | 行为与语义草案一致 | 保留为候选 B/C 的语义基线 |
| **冲突** | 候选源码表达的行为违反语义条目 | 候选 B/C 必须改写 |
| **缺口** | 行为合理，但正式类型、接口或协议尚未定义 | 进入语义案例、标准库或 WIT 设计 |
| **语法** | 含义明确，只有写法尚未定稿 | 在候选 B/C 中比较 |

## 3. 总体结论

十个程序的目标行为都可以建立在当前核心语义之上，没有证据要求为状态机、事务、路由或 UI 增加专用语言运行机制。

候选 A 不能原样成为语法规范。主要原因不是分号、缩进或 `end`，而是部分重要行为仍然隐藏：

- 导入名称与获得能力混在 `use` 中；
- effectful 公共函数没有完整效果与能力签名；
- 资源移动、借用和共享没有写出来；
- 封闭枚举的 `_` 分支隐藏了新增变体；
- 多处用 catch-all 把取消、权限、I/O 和业务错误压成一个错误；
- Future、Stream 和 Component 调用的分层结果没有完整展开；
- 部分样本的契约只写在自然语言中，类型仍允许构造非法状态；
- 有三个真实逻辑问题：分块文本统计、父任务取消、Notes 旧异步结果。

因此下一阶段不能只是把候选 A 换一种括号或关键字。必须先用最小正反例固定 P0 语义，再设计候选 B、C。

## 4. 跨样本共同问题

### AUD-G01：导入与能力来源混合【冲突】

相关样本：H、C、A、T、W、HTTP、F、P、N。

`use sico.console`、`use sico.storage` 等形式目前同时暗示“名称可见”和“拥有权限”。这与 SEM-012、SEM-082、SEM-083 不一致。

候选 B/C 必须让以下事实分别可见：

- 模块或接口依赖；
- 函数实际使用的效果；
- 能力资源从入口参数、Component import 或受限委托中的哪一处获得；
- manifest 只声明权限上界，不制造能力。

### AUD-G02：效果签名不完整【缺口】

相关样本：H、C、A、T、W、HTTP、F、P、N。

候选源码通过注释说明纯函数和副作用边界，但公共/导出签名没有表达效果集合与能力要求，尚不能满足 SEM-060、SEM-081。

需要先确定效果分类和公共签名规则，再比较“显式写出”与“编译器生成可见签名”两种表层方案。

### AUD-G03：资源参数缺少移动、借用和共享语义【缺口】

相关样本：A、T、W、HTTP、F、P、N。

`Store`、`ReadFile`、`Request`、`HttpClient`、`Component` 和 UI/存储句柄都可能是资源。按 SEM-061、SEM-090，它们默认移动；候选源码却多次复用或跨任务共享，没有说明：

- 调用是否消费资源；
- 是否为临时只读/可写借用；
- 是否允许跨任务共享；
- 资源由谁关闭。

资源案例必须先于正式函数调用语法完成。

### AUD-G04：通配与“有意处理剩余情况”没有区分【冲突】

相关样本：C、O、A、HTTP、F、P。

候选 A 的 `_` 同时用于：

- 忽略一个已经匹配到的载荷；
- 匹配列表的剩余形状；
- 接住封闭枚举的全部剩余变体；
- 丢弃来源错误；
- 将未来未知错误折叠为旧行为。

只有前两类不必然隐藏类型演进。SEM-031、SEM-064 要求封闭枚举新增变体时触发影响分析，候选 B/C 必须区分这些用途。

### AUD-G05：平台错误没有统一分层【缺口】

相关样本：A、T、W、HTTP、F、P、N。

标准库/WIT 尚未定义 storage、filesystem、HTTP、UI 和 Component 调用的正式错误类型，所以候选程序无法稳定区分：

- 领域拒绝；
- 能力拒绝；
- 数据无效；
- 可恢复 I/O 失败；
- 取消与超时；
- 资源限额；
- Component trap。

在这些接口确定前，`map_error(_ => ...)` 只能是风险标记，不能成为推荐模式。

### AUD-G06：单位值仍使用裸 Int【缺口】

相关样本：A、W、HTTP、F、P、N。

字节数、并发数、revision、schema version 和状态码仍普遍使用 `Int`。SEM-021 建议把 `Size`、`Duration` 等作为标准领域类型；审计进一步说明至少需要验证：

- `Size` 与非负计数；
- `Revision`；
- `SchemaVersion`；
- `HttpStatus`；
- 有上界的 `Concurrency`。

不是所有值都必须建立新类型，但跨边界和带单位参数不能只靠变量名区分。

### AUD-G07：入口与宿主调用结果尚未定义【缺口】

相关样本：H、C、HTTP、N。

`main`、HTTP server、UI exports 的载入和失败语义尚未映射到 WIT world。尤其需要确定：

- CLI 参数、stdout/stderr 和退出码；
- `server.serve` 正常返回、失败和取消；
- UI `init/update/view/handle` 的生命周期；
- Runtime 如何把 trap 与应用 Result 分开报告。

## 5. 逐样本审计

### 5.1 Hello World（H）

**符合**

- 入口返回 `Unit`，正常路径明确：SEM-021、SEM-065；
- 控制台写入位于唯一边界，没有隐藏全局状态：SEM-005、SEM-051；
- 程序足够小，可作为效果与能力诊断的最小样本。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-H01 | 冲突 | `use sico.console` 看起来直接获得 console 能力 | SEM-012、082 |
| AUD-H02 | 缺口 | `main` 签名没有可见的 `console.write` 效果和 console 能力来源 | SEM-060、081 |
| AUD-H03 | 缺口 | `write_line` 是否可能返回 I/O 失败尚未定义 | SEM-070、084 |
| AUD-H04 | 缺口 | CLI Component world 和退出语义未定义 | SEM-120、121 |

结论：行为成立，但候选源码还不是完整语义程序。

### 5.2 Calculator（C）

**符合**

- Operator 与 CalculatorError 是封闭名义类型：SEM-030、031；
- `try` 只传播同一个 CalculatorError：SEM-071；
- 纯计算与控制台外壳分离：SEM-080、081；
- 运算符匹配完整，没有隐藏新增 Operator：SEM-064。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-C01 | 缺口 | `Number` 未决定是 Int、Decimal 还是 Float，除法与格式化因而没有唯一含义 | SEM-021、024、142 |
| AUD-C02 | 冲突 | `Number.parse` 的封闭解析错误被 `error(_)` 隐藏，新增来源错误无法进入影响分析 | SEM-031、064、071 |
| AUD-C03 | 语法 | 列表剩余形状的 `_` 是完整性分支，不应与枚举 catch-all 使用同一含义 | SEM-064 |
| AUD-C04 | 缺口 | Text 使用 `+` 拼接是否允许、是否只接受同类型尚未定义 | SEM-003 |
| AUD-C05 | 缺口 | 控制台错误与进程退出码未处理 | SEM-070、120 |

结论：先把本样本拆为 `Int calculator` 与 `Decimal calculator` 语义案例，不能继续使用抽象 `Number` 掩盖决定。

### 5.3 Order State（O）

**符合**

- 使用封闭状态、封闭事件和纯转换函数即可表达状态机：SEM-031、111；
- 非法转换返回具名 Result 错误：SEM-070；
- 错误携带原状态和事件，便于 flow/impact：SEM-130；
- 没有证据需要专用状态机运行机制。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-O01 | 冲突 | `_` 把 15 个非法组合合并，也会把未来新增状态或事件自动判为非法 | SEM-031、064 |
| AUD-O02 | 缺口 | 需要一种“当前剩余 15 个组合已被有意拒绝，但类型扩展后重新检查”的静态表达 | SEM-064、111 |
| AUD-O03 | 缺口 | 只写了 5 个测试，尚未验证完整 `5 × 4` 矩阵 | SEM-111、130 |
| AUD-O04 | 缺口 | 终止状态目前只从转换图推导，没有正式契约或验证结果 | SEM-110、111 |

结论：问题不是是否增加 `state machine` 关键字，而是完整匹配必须能表达“有意拒绝当前余项”。

### 5.4 Account Transfer（A）

**符合**

- AccountId/Money 使用名义领域类型：SEM-032；
- 纯业务计算返回新记录，不原地修改账户：SEM-005、030、050；
- 业务错误是封闭 Result：SEM-031、070；
- 总余额守恒适合正式契约：SEM-110。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-A01 | 缺口 | Decimal 精度、Money 币种、舍入和构造验证均未定义 | SEM-021、032 |
| AUD-A02 | 冲突 | 通用 `transaction store` 块暗示语言可以赋予任意块事务性 | SEM-112 |
| AUD-A03 | 缺口 | Store 在 read/write 间复用，但资源借用和事务句柄所有权未表达 | SEM-061、090、112 |
| AUD-A04 | 冲突 | 存储错误的 `error(_)` 会隐藏权限、冲突、取消和未来错误 | SEM-064、071、084 |
| AUD-A05 | 缺口 | 源码契约只写总额守恒，遗漏 ID 不变与两边精确差额 | SEM-110 |
| AUD-A06 | 缺口 | 并发转账隔离、revision、重试和死锁不在接口契约中 | SEM-112、113 |
| AUD-A07 | 缺口 | storage read/write/transaction 效果与能力没有进入函数签名 | SEM-060、081—083 |

结论：事务应改为存储 provider 提供的 transaction resource/API；纯 `apply_transfer` 保留。

### 5.5 Todo Store（T）

**符合**

- 命令、错误和状态使用封闭数据：SEM-031；
- `apply` 是纯值转换，记录和 List 更新返回新值：SEM-030、035、111；
- revision 冲突是具名预期失败：SEM-070、113；
- schema 不认识时显式失败，不猜测迁移。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-T01 | 冲突 | TodoList 可被公开构造，无法保证 ID 唯一；load/save 也不验证该不变量 | SEM-030、110 |
| AUD-T02 | 冲突 | Todo 字段可直接构造，无法保证 revision 单调或 completed 只向 true 转换 | SEM-030、110、113 |
| AUD-T03 | 缺口 | `List.replace(todo, changed)` 依赖整条记录相等，未说明重复值和目标定位语义 | SEM-035、043 |
| AUD-T04 | 缺口 | `TodoId` 的可信生成来源未定义 | SEM-032、080 |
| AUD-T05 | 缺口 | Store 资源借用、错误映射和效果签名未定义 | SEM-061、071、081、090 |
| AUD-T06 | 缺口 | 只拒绝旧 schema，没有正式迁移和序列化 schema 规则 | SEM-030、121 |

结论：需要验证模块可见性、受验证构造器或类型不变量，不能只把约束写在 `apply` 注释中。

### 5.6 Word Count（W）

**符合**

- 字节、字素、单词和行使用不同操作：SEM-023；
- reader 是有确定关闭范围的资源：SEM-090、091；
- 局部计数器的显式可变不形成共享别名：SEM-050、051；
- 大小限制在收集前检查：SEM-093。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-W01 | 冲突 | `file.open_text()` 的错误未映射为 WordCountError，却使用同错误传播 `try` | SEM-071 |
| AUD-W02 | 冲突 | `reader.text_chunks()` 的中途读取与 UTF-8 错误没有 Result/Stream 错误分支 | SEM-070、105 |
| AUD-W03 | 冲突 | 对每个 chunk 单独调用 `graphemes/words/line_breaks` 会破坏跨 chunk 的字素、单词和 CRLF 边界 | SEM-023、105、142 |
| AUD-W04 | 缺口 | 同步文件流是否允许阻塞、如何响应任务取消尚未说明 | SEM-091、100、105 |
| AUD-W05 | 缺口 | `max_bytes` 应使用非负 Size，并在打开/读取两层受到限额 | SEM-021、093 |
| AUD-W06 | 缺口 | 路径型错误需要去敏展示，不能默认暴露宿主真实路径 | SEM-084 |

结论：需要状态化的 streaming text decoder/segmenter；不能把独立 Text chunk 当作完整文本逐块统计。

### 5.7 HTTP Service（HTTP）

**符合**

- handler 无全局可变状态，可并发调用：SEM-051、106；
- body 在解析前限额：SEM-093；
- API 领域错误集中映射为响应；
- 路由可以使用普通匹配，不需要 DSL。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-HTTP01 | 冲突 | health 响应使用匿名记录 `{ status: "ok" }`，不符合公共 schema 的具名记录要求 | SEM-030、121 |
| AUD-HTTP02 | 冲突 | BodyError 的 `_` 把取消、权限、读取失败等都映射成 InvalidJson | SEM-064、071—074 |
| AUD-HTTP03 | 冲突 | `(_, "/greet")` 隐藏未来 Method 变体，无法可靠进入影响分析 | SEM-031、064 |
| AUD-HTTP04 | 缺口 | Request/body/Response 的资源所有权与取消清理未表达 | SEM-090—092、102 |
| AUD-HTTP05 | 缺口 | Json 未知字段、schema 版本和编码失败语义未定义 | SEM-030、070、121 |
| AUD-HTTP06 | 缺口 | `server.serve` 是否返回 Result/取消，以及为何满足 `Never` 未定义 | SEM-021、065、072 |
| AUD-HTTP07 | 缺口 | HTTP effects、能力和 Runtime 并发限额没有进入 export 签名 | SEM-060、081—084、093 |

结论：普通匹配足够，但必须使用具名响应 schema，并显式保留请求取消和宿主失败层。

### 5.8 Concurrent Fetch（F）

**符合**

- 并发由 task group 显式创建：SEM-101、102；
- 每个请求时限和整体并发上限可见：SEM-073、093；
- collect-all 与保持输入顺序是明确策略：SEM-103、104；
- 单项 HTTP 失败被建模为列表元素，不触发 fail-fast。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-F01 | 冲突 | 子任务把 `HttpError.Cancelled` 转成 FetchResult.Cancelled，可能吞掉父任务取消 | SEM-072、102、103 |
| AUD-F02 | 冲突 | `error(_)` 把 body 超限、能力拒绝和未来错误都折叠为 NetworkFailure | SEM-064、071、084 |
| AUD-F03 | 缺口 | HttpClient 被多个任务共享，但类型没有声明可共享及并发安全 | SEM-092、106 |
| AUD-F04 | 缺口 | `tasks.all().ordered()` 的返回类型没有显式表现 Completed/Cancelled | SEM-103、104 |
| AUD-F05 | 缺口 | `max_bytes` 和 concurrency 使用裸 Int，负值与上界只部分检查 | SEM-021、093 |
| AUD-F06 | 缺口 | FakeHttpClient、虚拟时钟与真实 WIT 接口的一致性协议未定义 | SEM-073、121 |

结论：父取消必须终止整个 `fetch_all`；只有独立请求在未取消父任务的情况下产生的可恢复结果才能成为 FetchResult。

### 5.9 Component Plugin（P）

**符合**

- 插件通过 WIT-compatible interface 交互：SEM-121；
- 默认无能力，加载时限制内存、输入、输出和时间：SEM-083、093、123；
- trap 与 FormatError 在概念上分层：SEM-070、074；
- 动态加载作为受能力约束的平台操作，而非原生动态库。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-P01 | 冲突 | `Formatter@1` 被直接放入 `Component<...>` 类型表达，混合接口身份、版本与泛型 | SEM-122 |
| AUD-P02 | 冲突 | `plugin.format` 的领域 Result 与 Component 调用故障被当成同一层 error 匹配 | SEM-070、074、121 |
| AUD-P03 | 冲突 | catch-all 把能力拒绝、限额、取消和未知宿主故障都映射为 InvalidComponent/PluginTrap | SEM-071—074、084 |
| AUD-P04 | 缺口 | Component 资源按值传入会被消费，重复调用需要借用或共享实例语义 | SEM-061、090、092 |
| AUD-P05 | 缺口 | 接口兼容范围、adapter、包身份和版本解析未定义 | SEM-121、122 |
| AUD-P06 | 缺口 | Unicode uppercase 的数据版本与跨平台确定性未固定 | SEM-023、142 |
| AUD-P07 | 缺口 | Runtime 内存/时间硬限额与 API 请求限额的关系未进入调用结果 | SEM-093、123 |

结论：组件调用至少有“调用层结果”和“接口返回值”两层，候选 B/C 不能用一个扁平 error 模式伪装成同一类型。

### 5.10 Notes UI（N）

**符合**

- Model/Event/Effect 是普通数据与 SDK 模式，不要求语言专用语义：SEM-111、124；
- `update` 返回新 Model 与显式 effects：SEM-005、030、080；
- UI 回调主要携带领域 Event，避免直接修改控件资源；
- 保存使用 revision，方向符合 SEM-113。

**冲突或缺口**

| ID | 分类 | 发现 | 相关语义 |
|---|---|---|---|
| AUD-N01 | 冲突 | `view` 内调用 `NoteId.new()`；若生成唯一 ID，需要随机/时钟/状态能力，因此 view 不再纯 | SEM-080—083、124 |
| AUD-N02 | 冲突 | 旧 revision 的保存成功被忽略，但旧保存失败仍会把当前状态改成 Failed | SEM-113 |
| AUD-N03 | 冲突 | Loaded 事件没有请求 revision，较晚完成的初始加载可能覆盖用户已经编辑的 notes | SEM-102、113 |
| AUD-N04 | 缺口 | effect 的执行顺序、取消、去重、并发上限和生命周期没有正式协议 | SEM-102—104、124 |
| AUD-N05 | 缺口 | `problem.message()` 可能把敏感存储错误直接送入 UI | SEM-084 |
| AUD-N06 | 缺口 | View 是普通值、资源还是宿主描述树尚未定义 | SEM-020、124 |
| AUD-N07 | 缺口 | app_storage 能力来源、隔离范围和资源借用未表达 | SEM-082—084、090 |
| AUD-N08 | 缺口 | Model/Note 的公开构造仍允许绕过领域不变量 | SEM-030、110 |

结论：ID 必须在 effect/Runtime 边界生成或由事件携带可信 ID；所有异步完成事件都必须携带可判定的新鲜度信息。

## 6. 已确认不需要加入核心语言的功能

样本复核没有证明以下专用语法或运行机制是必要的：

- 状态机声明：封闭 enum、完整 match、Result 和契约足以表达；
- 通用 transaction 块：应由能力资源接口定义原子性；
- HTTP 路由 DSL：普通数据和匹配足以表达；
- UI 专用状态语义：Model/Event/Effect 可由普通类型和 SDK 表达；
- Component 私有 ABI：Sico interface 应映射 WIT；
- 可捕获异常：Result、取消和 trap 分层更清楚。

候选语法可以为常见结构提供不改变语义的简写，但不能建立第二套执行模型。

## 7. 必须先解决的 P0 语义案例

按依赖顺序建立最小成功/失败案例：

| 顺序 | 案例组 | 必须回答的问题 | 来源 |
|---:|---|---|---|
| 1 | 数字与单位 | Int、Decimal、窄化、除法、Money、Size | C、A、W、F |
| 2 | 名义数据与不变量 | record/newtype 构造可见性、受验证构造器 | A、T、N |
| 3 | 完整匹配 | 当前剩余组合、未来新增变体、忽略载荷 | C、O、HTTP、F |
| 4 | Result 映射 | 同错误传播、显式映射、禁止 catch-all 吞层 | 全部错误样本 |
| 5 | 效果与能力 | import、effect、capability、manifest 的闭合 | H、A、W、HTTP |
| 6 | affine 资源 | move、borrow、share、close、取消清理 | A、W、HTTP、P |
| 7 | Future/Task | 返回 Result、父取消、collect-all、顺序 | HTTP、F、N |
| 8 | Stream | 中途错误、背压、Unicode 跨 chunk | W、HTTP |
| 9 | Component 调用 | 接口 Result、调用故障、trap、版本 | P |
| 10 | revision | stale success、stale failure、stale load | T、N、A |

每组至少包含：一个合法程序、一个最小非法程序、期望短诊断、语义索引摘要和 Component/WIT 映射说明。

## 8. 下一项交付物

[`semantic-cases/`](../semantic-cases/README.md) 已创建，并已完成前四组正反例：

1. 数字与单位；
2. 名义数据与不变量；
3. 完整匹配；
4. Result 错误映射。

下一步复核这些案例的语义决定与诊断根因，再为相同含义编写候选 B、C。候选设计必须修复本报告中的“冲突”，并对“缺口”给出可查询的正式信息。
