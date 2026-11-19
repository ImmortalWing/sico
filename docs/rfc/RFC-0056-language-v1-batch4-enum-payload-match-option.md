# RFC-0056: Language v1 batch 4 — enum payload match + `Option`

> - status: **accepted（owner 指令 2026-11-19「RFC-0056 接受」）**
> - date: 2026-11-19
> - authors: autonomous-agent
> - target language/platform version: draft（script-v0 profile；component-v0 不在本片范围）
> - depends: RFC-0033（evidence gate）、RFC-0047 D4（`Module.records` schema 先例）、STEP-0629（前沿实测）、STEP-0628（poll_event §1 residual）
> - supersedes: -
> - superseded-by: -

## Summary

为已声明的 payload enum 放开**单层 payload 绑定 match 臂**、引入
`Option[T]` 与 `some`/`none`、并约定**宿主面 enum 的源侧声明形状**，
使 `sico.ui.poll_event()` 的源可见签名回到 `Option[ui_event]`（WIT 侧仍为 `option<ui-event>`）
（RFC-0052 §2 的 WIT 字节不动），STEP-0628 的 Text 解码面按 D7 回收。
三个缺口一批闭合，全部实证驱动（STEP-0629 八探针实测，见 §2）。

## Problem

`wit/script-ui-v1/ui.wit:83` 的 `poll-event: func() -> option<ui-event>`
在源语言不可表达：match case 名只有 `else`/`ok`/`error`/`Enum.variant`/
`true`/`false`，且 `some`/`none` 构造不存在（STEP-0494 fact-fix：
`E2031 unresolved call target some`）。STEP-0628 以 codegen 侧 Text 解码
绕过（`UI_EVENT_TEXTS`），代价 = guest 失去静态类型**且当前实现不读
payload**（`emit_ui_poll_event` 从不读 @16 的 union），
guest 无法做事件归因——不知道点的是哪个按钮、选的哪一项。

## Measured demand（STEP-0629 冻结实测，2026-11-19）

| 实测 | 结论 |
|---|---|
| p3/p4 带绑定臂（单元数/二元数） | lower 哨兵拒：`unsupported match payload binding`（`crates/sico-ir/src/lower.rs:2955-2994`，仅 `ok`/`error`+`Type::Result` 放行） |
| p4b 嵌套臂 `Wrap.Box(Notice.Stop)` | 语义按**叉积**展开 + E3001 完备性（`Wrap.Box\|Notice.Message, …`）——嵌套语义需独立裁决，不在本片顺手放开 |
| p1/p2 enum 作函数返回/形参 | 两 profile 同拒 `unsupported non-scalar parameter/result` |
| p7 瞬态 enum 值（不跨函数边界） | **完整构建成功**（1294 B）⇒ 发射器缺的只是「判别字读取 + payload 投影」的局部布局，不是 enum 值的 first-class |
| p5 `some(...)` | `E2031`（与 STEP-0494 一致） |
| p6 `case ok(h)` 对照锚 | 构建成功（1170 B），Result 绑定路径 = 本片泛化的直接模板 |

探针：`tools/probe-language-v1-batch4-frontier.ps1`（8/8 `FRONTIER_UNCHANGED`）。

## Goals and non-goals

**Goals**
- G1：match 单层 payload 绑定臂泛化到 `Named` enum 与 `Option` subject，
  支持多绑定（`case select(h, i)`）；IR 增 enum 声明表 + project 字段约定。
- G2：`Option[T]` 类型、`some(expr)`/`none` 构造、`some(x)`/`none` 臂、
  完备性规则。
- G3-R1：宿主面 enum 约定——程序按钉死形状声明 `enum ui_event`（§4 D5 命名修正），
  语义层对照校验；`poll_event` 源签名改 `Option[ui_event]`；
  STEP-0628 Text 面回收（`UI_EVENT_TEXTS` 退役，
  `ui_source.rs` 断言携带 handle/index）。

**Non-goals（v0 明确不做）**
- 嵌套 payload 绑定臂（`case some(click(h)):`）——保持 typed 拒绝
  （身份见 §6），v0 用顺序 match 幂等（`case some(ev):` 再 `match ev`）。
- 用户 enum 的函数边界 first-class——`unsupported non-scalar
  parameter/result` 两 profile 维持（p1/p2 实测身份不变）。
- RFC-0039 user-WIT 值集扩展（带 payload variant 跨界仍拒，该 RFC 不动）。
- component-v0 profile 的同名 widening（本片只动 script-v0 面）。
- M22 guest 前端支持——新形状在 guest 未跟进前 = typed refusal
  （可接受中间态，corpus/账本不动，零 selfhost 热文件改动）。

## Semantics

- **静态**。臂 payload 元数/类型对照 enum 声明逐槽校验；绑定名臂内作用域；
  完备性规则不变（按 variant 名）；`Option` match 跟 Result match 先例
  （语义层 v0 不强制 sealed-exhaustiveness）。`none` 与 `some` 构造受
  expected-type 约束（对齐 `unconstrained ok` 拒绝模式）。绑定臂体形状
  遵守 Result 绑定路径的现行规则
  （`crates/sico-ir/src/lower.rs:2983` 的 return-body 约束保持）。
- **宿主面 enum 形状表（D5）**。程序引用 `sico.ui.poll_event` 时，语义层要求
  已声明 `enum ui_event`，形状逐字匹配钉死表
  `click(U64) | select(U64, U64) | closed`（名/序/payload 类型；
  `select` 的两个 U64 = `ui-selection` record 的既有展平先例；
  **源侧名必须 snake_case**——连字符名（`ui-event`）在表达式/模式位置被
  词法为减号，STEP-0630 slice 1 实测构造调用报 `E2031`，
  `ui_event` 经 `boundary_kebab` 映射 WIT 名 `ui-event`），
  不符 = `E2028`（§6）。
- **动态**。subject 判别字读取 + 命中臂 payload 投影进绑定局部；
  仅命中臂求值；fuel/trap 行为与既有 Result 绑定路径同规则。

## Syntax candidates

1. **（采纳）单层绑定 + 关键字构造**：`case Click(h):`、`some(e)`、`none`
   ——与 `ok`/`error` 先例完全一致（新增 `TokenKind::SomeKeyword`；
   **`none` 复用既有 `TokenKind::None`**——它同时是 effects/capabilities
   空列表的关键字，两侧各自识别、实测共存无冲突）；`Option[T]` 走
   `Generic`（`List[T]`/`Result[T,E]` 先例），零新语法类别。
   **v0 生产者边界**：用户函数签名不跨 `Option`（与 user enum 同规则，
   `non-scalar parameter/result` 拒）——guest 侧生产者 = host intrinsic
   （`sico.ui.poll_event`，G3-R1）；`some`/`none` 在无名期望位（如
   let-bound match subject）保持 `unconstrained some/none` typed 拒。
2. （拒绝）全嵌套模式 `case some(click(h)):`：p4b 实测语义现为叉积展开，
   分层解构 vs 叉积是独立语义裁决，本片不捆绑；顺序 match 在
   `poll_event` 场景表达能力等价（事件类型内层 match 的 subject 是
   臂绑定值，正是 G1 放开的形状）。
3. （拒绝）`Option.some(e)` 限定构造：与 `ok`/`error` 非限定先例不一致，
   且 `none` 无法自然限定。

## Positive and negative cases

正例（e2e `.sico`，script-v0，构建成功）：单层绑定 `case Notice.Message(t)`；
多绑定 `case PairBox.Pair(a, b)`；`Option` 构造 + `some(x)`/`none` 臂；
`match sico.ui.poll_event():` 三分支 + `none` 分支完整事件循环。
负例（单层 typed 拒绝）：元数错 `case PairBox.Pair(a):`；
类型错绑定到不存在 payload 的 case `case Notice.Stop(t):`；
未声明 case 名；`case some(click(h)):` 嵌套绑定（lower 拒，
身份 §6）；宿主面形状错（`E2028`）；`none` 无期望类型（lower 拒）。

## AST and IR

- **HIR**：match 模式绑定从 Result 特例泛化到声明 enum / Option；
  模式文法不变（`parse_pattern` 已产出嵌套 `Pattern::Variant`，
  `crates/sico-ir/src/lower.rs:4053-4075`）。
- **IR schema（D2）**：`Module.enums` 声明表，镜像 RFC-0047 D4 的
  `records` 先例——`BTreeMap<String, Vec<EnumVariant{name, payload}>>`，
  `serde(default)` 空表省略 ⇒ 冻结序列化字节不变；lowerer 在
  `crates/sico-ir/src/lower.rs:505-513` 的 enum walk 处填充。
- **project 字段约定（D3）**：单元数 payload = `"<case>"`（对齐 `"ok"`
  现状）；多元数 = `"<case>.<index>"`（`"select.0"`）。
- **verifier**：`verify_data_operation` 的 Project 分支
  （`crates/sico-ir/src/lib.rs:865-869`）从硬编码 `"ok"` 泛化为
  对照 `Module.enums` 校验字段拼写与投影类型；Result 分支不变。
- 无新 IR 操作（`variant`/`project` 复用）；无新 intrinsic。

## Component/WIT mapping

- Script core 布局：判别字 word + payload 槽，局部布局内联
  （`emit_match_condition` 路径扩展，p7 证明瞬态形状已通）；
  函数边界维持 scalar-only（non-scalar 拒绝不变）。
- ui@1 边界：`option<ui-event>` 的 canonical ABI 读取
  （`UI_EVENT_RESULT_SIZE` 区）改 typed 绑定——option tag @0、
  variant tag @8、payload union @16 直接绑进 guest 局部，
  `UI_EVENT_TEXTS` 退役。`wit/script-ui-v1/ui.wit` 与
  `runner/sico-runner/src/ui.rs` 字节不动。

## AI evaluation

生成/理解指标：事件循环类程序的小样本语料（poll/match/归因）；
新诊断 E2028 的可恢复性（一次修复成功率）；无 live-model 主张，
离线语料不计入任何 gate。

## Compatibility

- 全部改动 = 把现有 typed 拒绝放宽为接受（additive），无在役程序
  行为变化；冻结序列化字节不变（D2 serde default）；`some`/`none` 为
  新关键字 token（lexer/formatter 同步，与用户标识符冲突时按既有
  关键字纪律处理——参照 `from` 保留字先例）。
- STEP-0628 的 Text 解码面退役 = guest 可见面 breaking change，
  但该面从未进任何已接受合同（RFC-0052 在 dev 仍 draft），
  且 ui@1 的 guest 可见能力尚未声明达成任何 M24 gate——回收窗口
  在 accepted 之前，成本最低。

## Security and privacy

无新 authority、无新 host 能力；事件 payload 为进程内数据；
ui@1 的 default-deny 授权面不变；限额检查不变。

## Alternatives

- 改 `ui.wit` 为扁平 `list<string>`（codex 分支做法）：合同变更 +
  guest 字符串协议，已被 owner 明确否决，不采纳。
- 绑定用投影 helper 函数而非 match 臂：match 是既有幂等，
  helper 引入新 IR/运行时面，拒绝。
- v0 一并放开嵌套绑定：捆绑未裁决的语义问题（叉积 vs 分层），拒绝。

## Validation and acceptance criteria

- `tools/probe-language-v1-batch4-frontier.ps1` 冻结表更新后 8/8 转绿
  （p3/p4 → accepted，p5 → accepted，p4b/p1/p2 身份维持）。
- e2e `.sico` fixtures + 拒绝边界（元数错/类型错/未声明 case/
  未声明 ui_event/`E2028`/嵌套绑定/unconstrained `none`）。
- `cargo fmt/clippy/test` 根工作区 + runner 工作区全绿；
  machine support matrix 与 backend-refusal corpus 同步。
- `crates/sico-codegen-wasm` 内 `UI_EVENT_TEXTS` 零命中；
  `ui_source.rs` 断言携带 handle/index（click(5)/select(6,2)）。
- 接受门槛：owner 裁决（本片登记为 draft；owner 接受前 G1–G3
  实现片按 M23 并行轨先例推进，实现不预设接受）。

## Links

- [`STEP-0629`](../steps/STEP-0629-language-v1-batch4-frontier-probe.md) 前沿实测
- [`STEP-0628`](../steps/STEP-0628-source-level-ui-stdlib-port.md) poll_event §1 residual
- [`计划`](../plans/language-v1-batch4-option-variant-match.md) D1–D7
- [`RFC-0047`](./RFC-0047-record-types-v0.md) D4（Module schema 追加先例）
- [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) ui@1 合同（draft）
- [`RFC-0039`](./RFC-0039-source-modules-package-resolution-user-wit-v0.md) v0 值集（本片不动）
- STEP-0494 fact-fix（`some` 不存在）
