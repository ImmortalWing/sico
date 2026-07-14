# RFC-0001: Sico diagnostics protocol v0

> - status: accepted
> - date: 2026-07-14
> - authors: autonomous-agent
> - target language/platform version: 0.1-draft
> - supersedes: -
> - superseded-by: -

## Summary

Sico 诊断以稳定 `code` 为身份、以 `key` 和 `arguments` 为机器语义、以一行短 `message` 为人类视图，并通过 UTF-8 精确跨度和版本化 JSON envelope 提供确定、低 token、无需解析自然语言的工具接口。

## Problem

仓库已有 29 个 P0 非法语义案例和 24 个 reject key，但这些 key 只是 provisional 名称，默认消息分散在 10 个 README 中，也没有正式编号、跨度坐标、排序、级联或 JSON 兼容规则。

若 AI 只能读取自然语言报错，会出现四类问题：

- 消息改写导致自动化失效；
- 相同根因在不同位置缺少可比较身份；
- 冗长源码摘录和解释消耗上下文；
- 多条级联错误掩盖真正根因。

v0 必须先固定协议，未来 parser/type checker 才能生成可审计输出；当前没有编译器，因此本 RFC 不声称已有真实诊断实现。

## Goals and non-goals

目标：

- 默认文本一条主行，适用时最多一条 hint；
- AI 和工具使用稳定字段，不解析 `message`；
- 一个诊断只有一个主要根因和一个主要跨度；
- 字节跨度精确且不受 Unicode 显示宽度影响；
- 多诊断顺序、去重、截断和级联行为可复现；
- 24 个现有语义 key 获得唯一正式编号和默认短消息；
- 协议可扩展到 parser、warning、LSP 和本地化，而不破坏 v0 消费者。

非目标：

- 定义具体 parser/type checker 实现；
- 保证每个错误都有自动修复；
- 在 v0 稳定终端颜色、源码摘录或编辑器 UI；
- 把 STEP-0003 的 mutation 设计预期冒充 parser 实测结果；
- 允许修复建议猜测或改变业务语义。

## Semantics

### 1. 诊断身份

每个源程序诊断具有：

- `code`：稳定身份，例如 `E2001`；
- `key`：稳定的可读符号，例如 `TYPE_MISMATCH`；
- `arguments`：渲染消息和程序化处理所需的变化值；
- `message`：由目录模板渲染的默认 `en-US` 短消息。

工具按 `code` 或 `key` 分支，可以显示 `message`，但不得通过解析 `message` 推断错误类型、期望类型、字段名或修复动作。

同一个 `code` 永远表示同一类根因。编号废弃后保留，不能分配给其他含义。兼容别名必须通过新协议字段或 superseding RFC 明确声明，不能静默交换 code/key。

### 2. 编号分区

| 范围 | 领域 | v0 状态 |
|---|---|---|
| `E1000`–`E1999` | 词法、语法、恢复 | reserved；等待 M1 parser 证据 |
| `E2000`–`E2999` | 名称、类型、字段、契约 | active |
| `E3000`–`E3999` | 模式、Result、控制流 | active |
| `E4000`–`E4999` | 效果、能力、边界声明 | active |
| `E5000`–`E5999` | affine 资源、Future、Task、Stream | active |
| `E6000`–`E6999` | Component、WIT、跨组件调用 | active |
| `E7000`–`E7999` | revision、并发一致性 | active |
| `E8000`–`E8999` | reserved | reserved |
| `E9000`–`E9999` | 编译器/工具内部故障 | reserved；不用于普通源码错误 |

分区只表达管理所有权，不要求编号连续。新增编号必须进入目录、测试和兼容性审查。

### 3. 默认文本

默认文本为：

```text
E2001 user.sico:18:10 expected User, found Option<User>
hint: handle none before using the value
```

约束：

- 第一行顺序固定为 `code file:line:column message`；
- 不默认打印 key、源码摘录、类型推导链或背景教程；
- `message` 一行且不以句号结尾；
- hint 可省略，存在时只有一行；
- 默认最多输出调用者要求的错误数；被截断数量进入 JSON summary；
- 颜色和装饰不是协议的一部分。

### 4. 源码跨度

JSON 的 `range` 同时包含：

- `byte`：0-based、UTF-8 字节偏移、半开区间 `[start, end)`，是精确定位的权威坐标；
- `line`、`column`：1-based Unicode scalar value 坐标，供人阅读；行尾位置允许为该行最后一个 scalar 之后的位置。

`start` 必须不晚于 `end`。主要跨度应是触发修复的最小相关表达式，不指向整个函数或文件。跨文件信息进入 `related`，不能扩大主要跨度来覆盖多个位置。

LSP adapter 必须从 UTF-8 byte offset 显式转换为客户端协商的位置编码，不能直接把 `column` 当作 UTF-16 code unit。

### 5. 根因、相关信息和级联

默认输出只包含 `kind: root` 的诊断。由未知名称、错误类型或失败解析产生的后续错误必须抑制，并计入 `summary.suppressed`。

调试模式可以输出 `kind: cascade`，但必须提供 `caused_by` 指向同一 envelope 中较早诊断的 `id`。`related` 只用于声明位置、移动位置、先前消费位置等解释根因的事实，不创建新的根因。

### 6. 排序、去重和截断

诊断按以下键稳定排序：

1. 规范化后的 `file` UTF-8 字节序；
2. `range.start.byte`；
3. severity 顺序 `error`、`warning`、`info`；
4. `code`；
5. `id`。

相同 `code`、`file`、主要 byte range 和规范化 `arguments` 的重复诊断只保留一条。应用 `max-errors` 前先完成根因抑制、去重和排序。JSON summary 必须区分 `emitted`、`suppressed` 和 `truncated`。

### 7. 消息和提示

目录中的模板只允许 `{lower_snake_case}` 占位符；占位符必须与 `required_arguments` 完全一致。参数值是字符串或字符串数组，不嵌入未声明结构。

默认 case 消息以 UTF-8 计不超过 120 bytes。hint 以 UTF-8 计不超过 100 bytes。详细解释由未来的 `sico explain CODE` 按需获取，不进入默认诊断。

提示和自动 edit 只有在不会猜测业务含义时才允许。v0 JSON 只稳定一个可选 `hint`；机器可应用 edits 留待后续 RFC，避免在尚无 parser/type checker 时过早固定修改协议。

## Syntax candidates

### 方案 A：单行前缀（采用）

```text
E2001 user.sico:18:10 expected User, found Option<User>
```

优点是短、可 grep、与现有方向文档一致。

### 方案 B：Rust 风格多行块

```text
error[E2001]: expected User, found Option<User>
  --> user.sico:18:10
```

对人友好但默认 token 更多；未来可作为 `--render rich`，不属于稳定默认格式。

### 方案 C：只输出 JSON

自动化最直接，但终端体验差。保留 `--json`，不作为唯一输出。

## Positive and negative cases

24 个正式诊断及 29 个 case 映射分别位于：

- [`diagnostics/catalog.json`](../../diagnostics/catalog.json)；
- [`diagnostics/semantic-case-map.json`](../../diagnostics/semantic-case-map.json)。

每个非法 case 继续在源码头声明 `reject(KEY)`，校验器证明该 key 能唯一解析为 code，并证明 README 中的默认短消息与目录模板渲染一致。

协议正例：

- `E2001` 提供 `expected` 与 `found`；
- `E5001` 的主要跨度指向 move 后再次使用，`related` 可指向首次 move；
- `E3001` 的 `arguments.missing` 是缺失 variant 数组，不要求 AI 解析逗号文本。

协议反例：

- 只输出 `something went wrong`；
- 同一 `E2001` 有时表示语法错误；
- JSON 只保留 message、要求 AI 正则提取类型；
- 因一个未知名称继续输出数十个派生类型错误；
- hint 建议删除 revision 检查或吞掉领域错误。

## AST and IR

诊断不是 AST 或 Sico IR 节点。前端阶段应把稳定的 `code/key/arguments` 与 source span 交给统一诊断层；渲染器再从 catalog 生成 message。

类型、symbol、effect 或 IR 实体不得直接序列化为内部 debug 字符串。进入 `arguments` 前必须形成确定的用户可见名称。IR validator 若报告普通源码可归因问题，应使用对应源代码 code；真正无法归因的编译器内部故障使用预留 E9xxx，并避免泄漏内部路径或敏感数据。

## Component/WIT mapping

诊断协议属于编译时工具接口，不进入应用 Component ABI，也不是 WIT 领域错误。未来若 compiler service 以 Component 暴露，可以将本 JSON envelope 作为版本化数据传输，但 code/key 的含义仍由本 RFC 与 catalog 管理。

E6xxx 用于 Sico 源码与 Component/WIT 边界的静态诊断；Wasm trap、宿主权限拒绝和应用领域 `Result` 是运行时结果，不应伪装成 compile-time E6xxx。

## AI evaluation

未来编译器实现后，每类诊断至少测量：

- root code/key 准确率；
- 主要 byte range 准确率；
- 级联抑制数量；
- 默认文本与 JSON snapshot 稳定性；
- AI 只读 JSON 后的局部修复成功率；
- 单条和整次运行 token 大小。

当前步骤只 verified 目录、case 覆盖、模板渲染、schema fixtures 和排序规则示例，不产生真实编译器或模型指标。

## Compatibility

`schema: "sico.diagnostics.v0"` 标识 envelope 主版本。v0 规则：

- 已知字段含义不能改变；
- 生产者可以添加字段，消费者必须忽略未知字段；
- required 字段的删除、类型改变、坐标规则改变或 code 含义改变需要新主版本；
- 新 code/key 可在 v0 catalog 中追加；
- code 不删除、不复用；废弃项标记状态并保留；
- 默认消息可在不改变含义和 arguments 的情况下修正文案，但 snapshot 变化必须显式审查；
- 本地化消息不能改变 code/key/arguments。

JSON Schema 位于 [`diagnostics/schema/diagnostics-v0.schema.json`](../../diagnostics/schema/diagnostics-v0.schema.json)。Schema 用于结构验证，本 RFC 是排序、坐标、抑制和兼容行为的规范来源。

## Security and privacy

- `file` 默认使用相对于工作区的规范化 `/` 路径，不输出用户主目录绝对路径；
- message、hint 和 related message 不包含源码全文、环境变量、token 或宿主内部错误；
- 参数来自源码名称时必须转义为 JSON 字符串，文本 renderer 不解释 ANSI 控制序列；
- 诊断数量和 related 数量受限，防止恶意源码放大输出；
- 内部 panic/backtrace 默认不进入用户诊断；
- AI 消费诊断不意味着授权读取诊断范围外的文件。

## Alternatives

- **以 key 代替数字 code**：可读但重命名成本高；保留 key，同时以 code 稳定身份。
- **所有参数塞进 message**：结构最少，但迫使工具解析英文；拒绝。
- **只保存 byte range**：机器精确但人工不便；同时保存派生 line/column。
- **只保存 line/column**：Unicode 和编辑器编码会产生歧义；拒绝。
- **立即固定自动 edits**：没有正式语法树和 formatter，容易产生错误修复；推迟。
- **为 mutation 立即分配 E1xxx**：没有 parser 证据，无法证明根因和恢复行为；只预留分区。

## Validation and acceptance criteria

本 RFC 在以下条件下作为 M0 v0 设计契约接受：

- 24 个现有 key 均有唯一 code、模板和参数定义；
- 29 个非法 case 全部映射且无未使用目录项；
- case 默认消息可由 catalog 确定性渲染且满足长度限制；
- schema 的通过和失败 fixtures 被离线校验器覆盖；
- 语义案例、mutation 和 AI 离线工具回归通过；
- 文档明确区分 proposed protocol 与未实现 compiler behavior。

真实编译器接受门槛留到 M1/M2：所有 compile-fail case 的 code、跨度、JSON 和 cascade golden tests 必须通过。

## Links

- step: [`STEP-0006`](../steps/STEP-0006-diagnostics-protocol-v0.md)
- protocol: [`diagnostics/README.md`](../../diagnostics/README.md)
- source cases: [`semantic-cases/`](../../semantic-cases/README.md)
- direction: [`DIRECTION.md`](../../DIRECTION.md)
- architecture: [`DEVELOPMENT.md`](../../DEVELOPMENT.md)
