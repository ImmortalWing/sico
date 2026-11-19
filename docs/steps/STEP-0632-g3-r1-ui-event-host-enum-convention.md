# STEP-0632 — G3-R1：ui_event 宿主面 enum 约定 + poll_event typed 回收

状态：implementation complete locally（2026-11-19；runner `ui_source` **6/6 绿**
（typed 断言携带 handle/index）、探针 13/13 `FRONTIER_UNCHANGED`、clippy/fmt 净）；
未 commit/push；不翻任何 gate。
计划：[`docs/plans/language-v1-batch4-option-variant-match.md`](../plans/language-v1-batch4-option-variant-match.md)；
合同：[`RFC-0056`](../rfc/RFC-0056-language-v1-batch4-enum-payload-match-option.md) D5/D7。

## 1. 内容

| 件 | 改动 |
|---|---|
| IR intrinsic 表 | `sico.ui.poll_event` 签名 `Text` → `Option[ui_event]`（`sico-ir/src/lib.rs` intrinsic 表） |
| semantics | 签名同步 `Option[ui_event]`；**E2028 HOST_ENUM_SHAPE_MISMATCH**——引用 `poll_event` 时校验程序已按钉死形状声明 `enum ui_event`（`click(U64) | select(U64, U64) | closed`，名/序/payload 逐字），缺失或不符 = typed 拒（`check_ui_event_shape`）；诊断目录 `catalog.json` 登记 E2028 |
| codegen 发射 | `emit_ui_poll_event` 重写：canonical ABI 区（option tag @0 / variant tag @8 / payload @16）直接绑进 guest 的 `Option[ui_event]` 并集——分支路由 + fail-closed（未知 option/variant 判别字 `unreachable`）；case tag 取模块 `VariantTags`（与 match 分派同表）；`UI_EVENT_TEXTS` 常量、data 段事件表、`ui_event_table` 管道**全退役**（源码零命中） |
| 布局修正（e2e 抓出） | ① `Option[user enum]` 内型经 `user_enums` 展平（`flat_ir_types` 仍拒——边界不变）；② **嵌套判别字**：Option cell = `[option tag, 内 enum tag, 最宽 payload]`——`some(ev)` 的投影结果保留自身 discriminant，内层 `match ev` 才能分派；`apply_option_fields`/`apply_enum_fields` 同步 Project 结果与 ReadLocal 的 field 映射 + `variant` 标记 |
| fixture/测试 | `tests/end-to-end/script-ui-window.sico`：加钉死形状 `enum ui_event` 声明、poll 段改嵌套 typed match；`runner/sico-runner/tests/ui_source.rs` 断言转 typed（`\|poll=click:5`、`\|poll=select:6:2`、`\|poll=none`、`\|poll=closed`） |

**`wit/script-ui-v1/ui.wit` 与 `runner/sico-runner/src/ui.rs` 字节未动**（D7 边界）；guest 对 `poll_event` 现在拿到完整类型化事件（handle/index 可归因）。

## 2. 实测记录

- runner `--test ui_source` **6/6 绿**（compile→IR→codegen→Component→runner→ScriptedUi 全链；click(5)/select(6,2)/closed/none 四事件形态）。
- 探针 13/13 `FRONTIER_UNCHANGED`：新增 p13（typed poll_event 嵌套 match，build accepted）与 p14（形状不符 → `E2028`）。
- `cargo clippy -p sico-ir -p sico-semantics -p sico-codegen-wasm --all-targets -D warnings` 净；`fmt --all --check` 净。
- G2 悬置的 Option 端到端运行期证据由本片闭合：`poll_event` 即 v0 唯一的 Option 生产者（用户函数签名不跨 Option 的边界规则不变）。

## 3. e2e 第一手抓出的布局缺口（2 个，已修）

1. `Option[ui_event]` local 拒 `non-scalar`——Option 内型回退 `user_enums`。
2. **内层判别字丢失**——`some(ev)` 的 ev 若无自身 discriminant，内层 `match ev` 无法分派；修正为嵌套布局（option tag / enum tag / payload），并把 field 映射与 `variant` 标记推广到 Project 结果与 ReadLocal 两侧。

## 4. 诚实收窄

- 收口全量验证（workspace clippy/test + runner workspace + matrix/refusal corpus 同步 + STATUS/ROADMAP）为下一片（STEP-0633）。
- select 事件的**模型侧 producer** 仍缺（STEP-0599 登记的 RFC-0052-A3 未裁）——本片保证的是：宿主一旦产出 select，guest 侧 typed 归因链路全通（runner 测试用 ScriptedUi 直接注入 `UiEvent::Select` 证明）。
- guest 前端（selfhost）对新形状的拒绝面未复测（D6 可接受中间态）。

## 5. 先存红（登记不修）

IND-003 同根因三测试面（sico-ir / sico-cli / sico-semantics）。
