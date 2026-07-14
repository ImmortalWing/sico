# RFC-0002: Sico Semantic Index and Query JSON v0

> - status: accepted
> - date: 2026-07-14
> - authors: autonomous-agent
> - target language/platform version: 0.1-draft
> - supersedes: -
> - superseded-by: -

## Summary

Sico 以 snapshot-scoped semantic ID、带证据的事实、显式完整性和确定性预算建立 Semantic Index，并让 `outline`、`describe`、`slice`、`impact`、`flow` 共享统一 request/response envelope 和核心 JSON 类型。

## Problem

AI 阅读现有程序时，如果只能全文搜索源码，会反复消耗上下文并遗漏远距离语义：错误映射、效果/能力传播、资源生命周期、状态拒绝路径、契约、测试和 Component 边界。

仓库已有 SEM-130—133 和 10 个代表性程序，但尚无正式字段、ID、snapshot、一致预算或“事实可信度”规则。五类查询若分别返回临时 JSON，还会产生以下失败：

- 相同符号在不同查询中使用不同身份；
- 注释目的、结构推断和编译器证明混成同一层；
- 结果因预算截断却仍声称完整；
- stale response 被应用到另一版源码；
- slice 为了省 token 删除错误、效果、契约或拒绝分支；
- impact 只给名单，不说明路径和不确定边界；
- flow 只列成功边，隐藏其余组合的拒绝结果。

当前没有 compiler，本 RFC 固定设计契约和 fixtures，不声称仓库已自动生成任何索引。

## Goals and non-goals

目标：

- 五类查询共享 semantic ID、snapshot、source、fact、evidence、budget 和 completeness；
- 默认返回最小充分语义上下文，不重复源码全文；
- `declared`、`verified`、`inferred` 可机器区分且不可静默升级；
- 部分程序、阻塞诊断和跨边界未知关系能安全表达；
- 查询有确定的 item/depth/UTF-8 byte 硬预算；
- 响应能给出继续查询引用，而不是一次展开整个图；
- 10 个代表性程序都进入协议覆盖矩阵；
- JSON 结构、跨字段规则和正反 fixtures 可离线验证。

非目标：

- 实现 parser、type checker、索引器或查询服务；
- 稳定 CLI 文本渲染、缓存存储或网络 API；
- 将模型 tokenizer 作为语言标准的一部分；
- 在没有依赖元数据时承诺完整跨包/跨 Component impact；
- 立即稳定语义哈希的 canonical semantic AST 算法；
- 允许目的摘要或注释承担类型、效果、错误或契约证明。

## Semantics

### 1. Semantic ID

v0 ID 使用 URI 形状：

```text
sico://<package-identity>/<module-path>/<kind>/<qualified-name>
```

例如：

```text
sico://examples/order-state/function/transition
sico://examples/order-state/enum/OrderStatus/variant/Draft
```

规则：

- ID 不包含 byte、line、column 或其他源码 offset；
- 格式化、注释变化和同一声明内的位置移动不改变 ID；
- package、module、kind 或声明名称改变会创建新 ID；
- rename 迁移关系将来可用显式 relation 表达，不能让一个 ID 同时代表两个名称；
- package version 不进入实体 ID，版本由 snapshot 隔离，便于跨版本比较同一逻辑身份；
- 尚未定义函数重载，因此 v0 不发明 overload ordinal；若语言未来接受重载，必须先定义稳定 disambiguator；
- 消费者把 ID 当作不透明字符串，不能靠拆路径推断类型或可见性。

### 2. Snapshot

每个 index、request 和 response 都绑定同一 `snapshot_id`。生产 compiler 使用：

```text
sha256:<64 lowercase hex>
```

hash 输入至少覆盖 compiler semantic version、package identity、语义相关配置、规范化源码内容和依赖公开接口身份。精确 snapshot descriptor 的二进制 canonicalization 在实现原型中固定；不同 compiler semantic version 的 snapshot 不假定可互换。

`fixture:` 前缀只允许测试与文档 oracle。生产工具不得发出 fixture snapshot。

response snapshot 与 request 不同必须拒绝，不能只标 warning。snapshot 解决 stale 查询关联，不替代定义级语义哈希。

### 3. Index

`sico.semantic-index.v0` 包含：

- producer 与 snapshot；
- package 和 modules；
- symbols：类型、函数、常量、variant、状态、测试等；
- relations：调用、类型使用、错误返回、测试、状态/数据/效果/Component 关系；
- source range、visibility、quality 和 facets。

索引是编译工具资产，不进入 `.sapp` 运行载荷。实体 ID 唯一，relation 两端必须存在于 snapshot。源码范围复用诊断协议坐标：0-based UTF-8 半开 byte range 为权威坐标，line/column 是 1-based Unicode scalar value 坐标。

定义级 semantic hash 仍是目标，但 v0 不要求 fixture 伪造它。未来只有在 canonical semantic representation 与等价变换测试确定后，才登记 hash profile；未知 profile 的 hash 不能用于兼容或缓存决定。

### 4. Fact provenance

语义字段放在命名 `facets` 中，每个 facet 是：

```json
{
  "basis": "declared | verified | inferred",
  "value": "...",
  "evidence": [{ "kind": "source | analysis | semantic-rule | diagnostic | user-change", "ref": "..." }]
}
```

含义：

- `declared`：源码显式签名、类型、contract、branch 或测试声明；必须有 source evidence；
- `verified`：compiler 通过类型、控制流、效果、契约或 SEM rule 证明；必须有 `compiler:` analysis 或 `SEM-nnn` evidence；
- `inferred`：索引器、查询器或变更模型根据结构得出的有用结论，尚未证明。

自然语言 purpose/summary 通常是 `declared` 或 `inferred`。注释、SPEC、模型输出和普通 source evidence 都不能单独产生 `verified`。消费者展示 summary 可以简化，但正式修改决定必须读取相关结构化 facets 和 evidence。

### 5. Completeness

snapshot、module、symbol、result 和 item 使用：

- `complete`：相对于声明的查询目标和边界没有已知省略；
- `partial`：有已知省略、阻塞诊断、未知依赖或预算截断；
- `unknown`：工具无法判断覆盖范围。

`complete` 必须有空 `reasons` 和空 `blocked_by`。`partial/unknown` 必须至少给一个 reason 或稳定诊断 code。完整性不是事实真值：一个 complete response 仍可包含 inferred facts；basis 回答“如何知道”，completeness 回答“是否已覆盖所声明范围”。

局部查询可以在 partial snapshot 上成为 complete，但仅当目标所需闭包已知完整。跨动态加载、未索引 package 或 Component 的 impact 默认 partial。

### 6. Query envelope

统一 request 字段：

- `request_id`：本次关联 ID；
- `snapshot_id`：必须读取的索引版本；
- `operation`：五类之一；
- `target.id`：semantic ID；
- `parameters`：operation 专用输入，例如 change 或 flow_kind；
- `options`：预算、私有符号、源码正文和传递展开策略。

统一 response 字段：

- 同一 request/snapshot/operation；
- 实际 budget 使用和截断状态；
- `result.target/completeness/reasons/blocked_by`；
- operation 专用角色的 `items` 和关系 `edges`；
- `next_queries`，用于按需继续，不自动展开。

### 7. Budget

v0 有三个确定硬限制：

- `max_depth`：从 target 出发的最大关系边数；
- `max_items`：result 顶层 semantic items 数；
- `max_bytes`：result canonical JSON 的 UTF-8 bytes。

canonical JSON 规则：对象键按 UTF-16 code unit ordinal 升序，数组保序，字符串使用 JSON escaping，数字使用无冗余 JSON 表示，不添加空白。`used_bytes` 必须可由消费者复算。

若任一硬限制导致省略：`truncated=true`、`omitted_items>0`、result 不得 complete，并必须提供 reason。先按稳定 ID 和 operation 规定的相关性顺序选择结果，再应用预算，避免同一 snapshot 随运行顺序漂移。

`token_budget` 是可选适配字段，存在时必须带 tokenizer 名称。它不能替代 byte 硬限制，也不能让 compiler 核心依赖某个模型供应商。

`include_source=false` 禁止源码正文，不禁止精确 source range。源码内容应由后续受权限约束的读取操作获取。

### 8. outline

`outline` 返回 package/module 地图：公开或获准可见的模块、主要符号角色、结构化 purpose、效果/能力边界和继续查询入口。

默认不递归列出函数体、所有私有声明或源码。purpose 不是正式行为保证。项目 outline 应让 AI 先定位模块，再 describe 目标，不要求一次加载整仓库。

### 9. describe

`describe` 对单个 target 返回一个 `role: target` item，优先包含：

- 完整签名和 visibility；
- 参数、返回、错误；
- effects 与 capabilities；
- contract/invariant 及证明状态；
- resource/async/Component 边界；
- 直接 callers/callees、tests 和 source range 的按需引用。

不存在或 snapshot 不匹配不是空成功结果，应返回工具错误/诊断，而不是伪造 unknown symbol。

### 10. slice

`slice` 返回为指定 goal 理解或修改 target 所需的闭包。每个非 target item 必须有结构化 inclusion reason。

行为 slice 不能因压缩删除会改变结论的：

- 参数和返回类型；
- Result/Option 与显式错误映射；
- effects、capabilities 和 Component imports/exports；
- resource move/drop 和 async/task 边界；
- contract/invariant；
- 状态成功与拒绝路径；
- revision/事务要求。

若预算或缺失索引使上述任一类别未知，slice 必须 partial 并说明缺口。v0 不要求返回源码正文；semantic items、edges、source ranges 和 next query 足以先定位。

### 11. impact

`impact` request 必须说明 change kind 与短 summary。每个影响 item 必须包含：

- direct/test/component/capability 等 role；
- basis 与 reason；
- 从变更 target 开始的 `via` 路径；
- 当前 item completeness。

同名文本引用不等于语义影响。反射、动态 Component、未索引依赖和权限/manifest 外部状态存在时，结果必须 partial。impact 不保证建议修改代码，只负责给出可审计影响候选和应运行测试。

### 12. flow

`flow_kind` 支持 `state/data/error/effect/resource/task/revision`。flow 返回 semantic nodes 与有依据的 edges。

complete state flow 除成功转换外必须表达其余输入组合的结果。可以显式枚举，也可以用唯一 `covers_remaining: true` 的 rejection edge 表示封闭剩余集合；不能只画成功路径后声称 complete。

data/resource/task/revision flow 必须保留 move、await、取消、错误、revision 比较等改变结论的边。无法证明的边使用 inferred，不能标 verified。

## Syntax candidates

本 RFC 不决定 Sico 源码表层语法。查询入口保留两种外观：

### CLI 适配

```text
sico outline <target> --json
sico describe <semantic-id> --json
sico slice <semantic-id> --json
sico impact <semantic-id> --change <file> --json
sico flow <semantic-id> --kind state --json
```

### 服务/库 request（采用的稳定层）

```json
{
  "schema": "sico.semantic-query.v0",
  "protocol_version": 0,
  "request_id": "q1",
  "snapshot_id": "sha256:...",
  "operation": "describe",
  "target": { "id": "sico://acme/orders/function/transition" },
  "options": {
    "max_depth": 1,
    "max_items": 16,
    "max_bytes": 16384,
    "include_private": false,
    "include_source": false,
    "transitive": false
  }
}
```

CLI 名称以后可以增加别名，JSON operation 和字段才是 v0 兼容层。

## Positive and negative cases

正例位于 [`semantic-index/fixtures/`](../../semantic-index/fixtures/README.md)：

- outline 覆盖全部 10 个代表性模块；
- describe 展开订单 transition 的签名、错误、效果和测试；
- slice 保留账户转账的类型、错误和守恒 contract；
- impact 为新增订单状态返回 transition 与测试路径；
- flow 表达 5 条成功边和覆盖其余 15 组的拒绝规则。

反例分别拒绝：

- request/response snapshot 不同；
- 只有 source evidence 却标 verified；
- truncated/omitted 结果仍标 complete。

[`sample-matrix.json`](../../semantic-index/sample-matrix.json) 证明 10 个样本均进入协议审计，并覆盖五类 operation 与 AI01–AI07；它不是 compiler 运行结果。

## AST and IR

Semantic Index 从已解析/已检查 AST 和后续 Sico IR 分析产生，但不是 AST/IR 的公开序列化。内部节点编号、Rust debug 字符串和临时推导变量不能泄漏为稳定 ID。

M1 可先产生 module/declared signature outline。M2 加入 resolved type、call/error/effect/capability 和初步 slice/impact。M3 加入经过 IR 验证的 resource、async、Component 和跨边界 flow。

索引生成失败不能改变程序语义。部分前端成功时允许 partial index，并引用 STEP-0006 的稳定 blocking diagnostic。

## Component/WIT mapping

Component import/export 和 WIT interface 必须作为 symbols/relations，而不是拼进 purpose 文本。跨组件 semantic ID 使用提供方 package identity；adapter 和版本兼容关系显式成边。

缺少组件索引、动态选择 provider 或 Runtime 才决定 capability 时，impact/flow 必须 partial。编译时查询不把 Runtime trap、权限拒绝或领域 Result 混成同一错误边。

## AI evaluation

真实实现后至少测量：

- 限定 byte/token 预算下的理解正确率；
- slice 必需实体召回率与多余实体比例；
- impact 的 direct/transitive/test/component 召回率；
- flow 成功、拒绝、错误和终止路径准确率；
- stale snapshot、partial index 和阻塞诊断处理正确率；
- 全文读取与查询工作流的输入 token、延迟和修复成功率。

当前只 verified JSON/fixture 一致性，不生成真实性能、模型或 compiler 指标。

## Compatibility

三个 schema 分别是：

- `sico.semantic-index.v0`；
- `sico.semantic-query.v0`；
- `sico.semantic-response.v0`。

v0 兼容规则：

- 消费者忽略未知字段；
- 已知字段含义、basis、completeness 和坐标不能改变；
- 可新增 operation 专用 facet/role/relation kind；
- 删除 required 字段、改变 ID/snapshot/budget 含义或复用字段需要新主版本；
- semantic ID 在同一 package identity 和声明身份下稳定，rename 创建新 ID；
- query 只能与同 snapshot response 配对；
- 不同 protocol major 的 index/query/response 不能静默混用。

## Security and privacy

- 默认不返回源码正文、绝对路径、环境变量、secret 或 Runtime 内部状态；
- source file 使用 workspace-relative `/` 路径；
- include_private/include_source 是请求上限，不是授权；服务端仍需独立权限检查；
- purpose、summary 和 user-change 是不可信文本，renderer 必须转义控制字符；
- budget、深度和 items 有硬上限，避免恶意图导致输出放大；
- partial/unknown 不能被缓存层升级为 complete；
- semantic index 不进入未授权发布包，私有符号索引与源码具有相同保密等级；
- AI 获得 semantic ID 不代表获得读取对应文件或 Component 的权限。

## Alternatives

- **五套独立 JSON**：局部字段更直接，但身份、预算和可信度分裂；拒绝。
- **只返回通用 property graph**：实现统一，但常见任务需要大量低层节点/边，token 和解析成本更高；保留统一核心、采用 operation 专用 roles/facets。
- **返回自然语言摘要**：方便演示但不可审计；summary 只作辅助 fact。
- **用源码位置作 ID**：声明移动会使缓存和影响路径失效；拒绝。
- **把版本写进每个 ID**：跨版本比较困难；使用 snapshot 隔离。
- **以模型 token 作为唯一预算**：供应商和 tokenizer 不稳定；byte 是硬协议，token 是命名适配。
- **截断后不报告**：会让 AI 把缺失依赖当作不存在；拒绝。
- **立即稳定语义 hash**：没有 canonical semantic AST 和等价变换数据；推迟到独立原型/RFC。

## Validation and acceptance criteria

M0 v0 设计契约接受条件：

- 三个 JSON Schema 和 protocol manifest 可解析；
- 10 个样本、五类 operation、AI01–AI07 全覆盖；
- index 的 semantic ID 唯一、关系端点存在、source byte/line/column 与真实样本一致；
- declared/verified/inferred 证据规则可校验；
- 五个合法 pair 分别覆盖一个 operation；
- snapshot、verified 和 truncation 三个非法 fixture 按单根因拒绝；
- result canonical byte 预算可复算；
- STEP-0003—0006 全量回归通过；
- 文档不把 fixture 冒充 compiler 或 AI 实测。

M1/M2 实现接受条件另行记录：真实索引 snapshot、golden responses、partial recovery、slice/impact/flow accuracy 和资源上限必须有测试证据。

## Links

- step: [`STEP-0007`](../steps/STEP-0007-semantic-query-json-v0.md)
- protocol: [`semantic-index/`](../../semantic-index/README.md)
- semantics: [`SEMANTICS.md`](../../SEMANTICS.md)
- diagnostics: [`RFC-0001`](./RFC-0001-diagnostics-protocol-v0.md)
- representative issues: [`examples/ISSUES.md`](../../examples/ISSUES.md)
