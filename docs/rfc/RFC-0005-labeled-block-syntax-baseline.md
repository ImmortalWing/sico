# RFC-0005: Labeled-block syntax baseline for M1

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target language/platform version: M1 draft
> - supersedes: -
> - superseded-by: -

## Summary

Sico 选择候选 B（Labeled Blocks）作为 M1 lexer/parser/formatter 的唯一表层语法基线；A0 与 C 不进入 M1 主语法，但完整保留为实验对照。该决定优先结构自描述和局部诊断，接受已量化的源码/token 冗余，并在获得真实 parser 与模型数据后按预注册门槛复审。

## Problem

同时实现 A0/B/C 会把 parser、formatter、诊断、教程和 AI 生成空间扩大三倍，使 M1 无法建立唯一规范格式。立即创造未评测混合语法又会绕过 162 个镜像程序和 36 个 mutation 的现有证据。Sico 必须选择一个足够明确的起点，同时不能把静态统计冒充“AI 已证明更喜欢它”。

核心目标不是语法新颖或最少字符，而是：AI 能快速识别结构和签名；一个缺失/错配关闭标记能产生短、局部、带结构类型的错误；formatter 能把 AST 映射到唯一源码。

## Goals and non-goals

目标：为 M1 固定一个语法；保持 P0 语义不变；让块开始/结束携带同一结构身份；定义 mutation、formatter 和 AI 复审门槛；保存候选历史。

非目标：本 RFC 不稳定全部语言语法，不定义模块/import、属性、文档注释、UI SDK、宏、完整借用语法或未来关键字；不声明 B 已在真实模型或 parser 中获胜。

## Decision basis

STEP-0012 对每套 54 个程序执行同一静态方法：

| Candidate | Lines | UTF-8 bytes | Lexical tokens | Punctuation | Labeled block closes |
|---|---:|---:|---:|---:|---:|
| A0 | 518 | 10,067 | 2,229 | 836 | 0 / 149 |
| B | 563 | 13,217 | 2,668 | 910 | 151 / 151 |
| C | 488 | 9,990 | 2,494 | 1,266 | 0 / 149 |

事实：A0 lexical token 最少；C bytes/lines 最少；B 比 A0 多 19.7% lexical token、31.3% bytes；B 的已统计块结束 100% 带结构名称；C punctuation 比 A0 多 51.4%。

推断：`end function`、`end match`、`end resource` 等冗余能让人、AI 和 parser 在关闭处直接看到期望结构，并允许错配诊断包含 expected/actual。该好处尚未由真实 parser recovery 或模型正确率测量。

未知：三套候选的真实生成、理解、修复率、供应商 token、parser 级联和恢复距离。AI v1 的 96-task 协议已准备好；真实 full run 等待模型范围、凭据和成本授权。

## Normative surface for M1

M1 以 [`syntax-candidates/b/`](../../syntax-candidates/b/) 的 54 个程序为 canonical corpus，并遵守：

- nominal type：`newtype Name from Base`；
- record：`record Name:` ... `end record`；
- enum：`enum Name:`、`case Variant` ... `end enum`；
- 有函数体的 function：`function name(...) returns Type:` ... `end function`；interface/capability/resource 内的方法签名没有函数体，不另写 `end function`；
- match：`match value:`、`case Pattern:` ... `end match`；
- generic：`Type[A, B]`；
- capability/resource/interface 分别以同名 `end capability/resource/interface` 关闭；
- deterministic cleanup：`using value:` ... `end using`；
- structured concurrency：`task group:` ... `end task`；
- `effects:` 与 `capabilities:` 是函数签名的有序声明子句，不是可任意嵌套的语句块；
- `ignore` 只忽略已知 variant payload，`else` 才表示通配；
- `Result`、trap、版本和泛型继续使用不同表层形式。

源码中的缩进由 formatter 规范化为两个空格；块边界由关键字和带名称结束标记决定。签名子句、字段、变体、arm 和语句的换行规则由 M1 grammar/formatter 固定，不引入可选分号、可选逗号或同义关键字。

未出现在 54-case corpus 的结构不能由 parser 实现临时猜测；必须先补 positive/negative case，必要时新建 RFC。

## Semantics

本 RFC 只选择表层语法，不改变 [`SEMANTICS.md`](../../SEMANTICS.md) 的 accept/reject 判定。A0/B/C 的同 case ID 必须继续具有相同语义；parser 只建立结构，M2 才执行名义类型、完整匹配、Result、效果、能力、资源和异步检查。

语法错误不得被自动插入可能改变 AST 的隐式节点后继续当作成功。恢复节点必须带 `missing`/`error` 身份、原始 span 和期望结构，且不能进入可执行 IR。

## Positive and negative cases

所有 [`syntax-candidates/b/`](../../syntax-candidates/b/) 程序都应在 M1 成功解析，包括其中预期由 M2 拒绝的语义负例。所有 [`syntax-mutations/b/`](../../syntax-mutations/b/) 程序都应在 M1 产生对应主要语法诊断：

- MUT-001/003/005/007/008/011：缺少具体声明关闭；
- MUT-002：缺少 match arm 分隔；
- MUT-004：缺少类型参数关闭；
- MUT-006：缺少调用关闭；
- MUT-009/010：缺少 using/task 关闭；
- MUT-012：缺少参数列表关闭。

默认短消息必须遵守 RFC-0001 的 120-byte 上限；结构错配应优先使用类似 `expected end match; found end function` 的一个根因，而不是连续报出后续定义全部非法。

## AST and IR

parser 输出的 syntax tree 必须保留 block kind、open/close token、名称、完整 token/trivia 和 error node。semantic AST 去除 `function`/`returns`/`end function` 等纯表层差异，得到候选无关的 function、match、resource 等节点。

Sico IR 不记录 B 的词法关键字或缩进；它只接收已通过语法与静态语义验证的 typed AST。任何 error/missing node 到 IR lowering 都是内部验证失败，不能静默生成代码。

## Component/WIT mapping

本决定不改变 Component Model、WIT world、resource ownership、WASI 0.3 async 或 `.sapp` ABI。同一 semantic AST 的 Component 输出不得因未来 formatter 或表层关键字变更而改变。WIT 接口版本不是 Sico 泛型参数，继续由不同语法表示。

## AI evaluation

正式比较使用 `sico-ai-eval-v1`：每候选 10 generation、10 understanding、12 repair task，每 task 至少 30 次；保存精确模型版本、原始输出、参数、token、成本、延迟和 prompt hash。

真实模型实验的复审触发条件：

1. A0 或 C 在相同模型/协议上，generation 与 repair 的 fully-correct rate 均比 B 高至少 5 个百分点，且 understanding 不低超过 2 个百分点；或
2. B 的供应商报告总 token 比正确率相当（差距不超过 2 个百分点）的候选高至少 20%；或
3. 至少两个不同模型家族重复出现同方向结果。

触发只要求重开 RFC，不自动切换语法。统计方法、排除规则和模型集合必须在读取结果前登记；fixture、smoke 和人工修正输出永不计入。

## Parser and formatter acceptance

M1 至少满足：

- 54/54 B canonical cases 产生语法树且 formatter roundtrip 后 AST 不变；
- formatter 对 canonical corpus 二次运行逐字不变；
- 12/12 B mutation 产生声明的主要根因，不因恢复而成功编译；
- 恢复到 manifest anchor，anchor 后第一个完整顶层定义/arm 可继续解析；
- 单个 mutation 不在 recovery construct 外产生超过 1 个额外语法诊断；
- 错配关闭诊断同时携带 expected block kind、actual close kind 和 open span；
- parser fuzz/property tests 不 panic、不无限循环、不生成可 lowering 的 error tree。

若 B 无法达到上述门槛而 A0/C 的同构 parser 能达到，必须重开本 RFC。

## Compatibility

RFC 接受后，A0 和 C 文件仍留在候选/实验目录，但不被 `sico check` 当作正式源码。M1 尚未发布稳定语言，因此选择 B 不需要用户迁移工具。首次公开 alpha 后，改变块、函数、generic 或 Result canonical syntax 必须有新 RFC、版本说明和 formatter/migration 策略。

## Security and privacy

明确结构边界降低错误恢复吞掉后续 capability 声明的风险，但不构成安全证明。parser 恢复不得把未解析能力列表继承给后续函数；诊断不得输出源码范围外数据。AI 运行凭据、原始私有源码和供应商日志继续保存在仓库外受控位置。

## Alternatives

### Accept A0

优点是 lexical token 最少、标点少；拒绝作为 M1 baseline，因为通用 `end` 在关闭处不携带 block kind，嵌套丢失结束标记时需要更多上下文才能指出错配。保留为最简对照。

### Accept C

优点是 bytes/lines 最少、形态接近大量现有语言；拒绝作为 M1 baseline，因为标点最多，`}` 只提供 delimiter kind 而不提供 semantic block kind。保留为紧凑/训练熟悉度对照。

### Create a hybrid now

可能用 `fn`/`->` 配合 `end function` 降低 B 冗余，但当前没有 54-case、mutation 和 AI task 镜像。立即选择会丢弃可审计比较，因此拒绝。未来可把这种方案作为 B2 新候选，不得直接悄悄改 grammar。

### Support all three

会扩大 grammar、formatter 和 AI 输出空间，违背一种结构一种规范写法；拒绝。

## Validation and acceptance criteria

RFC 的 M0 接受依据是：162 个镜像程序通过元数据校验；全量静态快照可重建；36 个 mutation 可由单次替换重建；96-task AI v1 数据集与评分器回归通过；决定清楚标注 inference/not measured，并具有 M1/真实模型复审门槛。

这不是 B 的生产稳定承诺。完成上述 parser/formatter acceptance 后，RFC 才获得 parser 级 measured 证据；真实模型数据满足授权条件后单独追加报告。

## Links

- [`STEP-0013`](../steps/STEP-0013-syntax-baseline-decision.md)
- [`STEP-0012`](../steps/STEP-0012-syntax-evidence-completion.md)
- [`syntax evidence v1`](../reports/syntax-evidence-v1.md)
- [`SYNTAX.md`](../../SYNTAX.md)
- [`syntax-candidates/`](../../syntax-candidates/README.md)
- [`ai-eval/`](../../ai-eval/README.md)
