# STEP-0631 — G2：`Option[T]` + `some`/`none`

状态：implementation complete locally（2026-11-19；G2 落地，单测+探针+clippy/fmt 全绿；
Option 的端到端 guest 证据随 G3-R1 的 `poll_event` 落地——v0 用户函数签名不跨
Option，生产者只有 host intrinsic）；未 commit/push；不翻任何 gate。
计划：[`docs/plans/language-v1-batch4-option-variant-match.md`](../plans/language-v1-batch4-option-variant-match.md)；
合同：[`RFC-0056`](../rfc/RFC-0056-language-v1-batch4-enum-payload-match-option.md) D4。

## 1. 内容（四层）

| 层 | 改动 |
|---|---|
| lexer | 新增 `TokenKind::SomeKeyword`（`"some"`）；`none` **复用既有 `TokenKind::None`**（它同时是 `effects: none`/`capabilities: none` 空列表关键字，两侧各自识别，b 语料全量实测无冲突） |
| semantics | `infer_call` 加 `some` 臂（`Option[参数类型]`，与 ok/error 同件的 `infer_result_constructor`）；裸 `none` 原子 = `Option[Unknown]`；`parse_match_pattern` 加 `some`/`none` case 名（key `"some"`/`"none"`）；`check_match` 把 Option match 跟 Result match 一样跳过 sealed-exhaustiveness（v0 先例） |
| IR/lower | verifier `Project` 加 `Type::Option` 臂（`field=="some"` + 内型相等——G1 时只接了 Variant op，Project 臂漏接，e2e 第一手抓出）；`call()` 加 `some` 特判（`unconstrained some`/`some arity` 拒，ok/error 模板）；表达式原子 `none`（`unconstrained none` 拒）；`parse_pattern` 裸 `None` → `Variant{"none",[]}`；`resolve_payload_binding` 加 Option 臂（`PayloadBinding::Some`）；三条 match 路由发射臂全接（lower_match 用 bindings+withdraw，A/B 路由用 `#match` cell） |
| codegen | `lower_local_types` 加 `Type::Option`（tag + inner flat）；`apply_option_fields`（ReadLocal 的 `some` field 映射）；Variant/ReadLocal 的 `variant` 标记本就含 Option |

## 2. 实测记录

- `cargo test -p sico-ir` 25/26（唯一红 = 先存 IND-003）；`-p sico-codegen-wasm` 全绿；`-p sico-semantics` 5/6（唯一红 = IND-003 同根因的 b 语料遍历，**stash 证实改动前即红**）；clippy `-D warnings` 净（`MatchPattern` 5 bool 定点 allow）；`fmt --all --check` 净。
- 新单测 3 个：`option_some_none_constructors_and_arms_lower_to_verified_ir`（构造 + `case some(v)` 投影到 `"some"` 字段 + verify 绿）、`option_constructor_refusal_boundaries_have_stable_identities`（return 位误用 = 语义层 E2001 先拦；let-bound match subject = lower `unconstrained some`）、G1 拒绝边界表维持绿。
- 探针冻结表更新至 11/11 `FRONTIER_UNCHANGED`：p5 身份从 `E2031` 移至 `codegen:non-scalar-result`（fixture 声明 Option 函数签名，v0 边界拒为设计）。
- p12 手工 e2e 撞到并修正两处真实缺口（见 §3）；因 v0 生产者边界（用户函数不跨 Option），独立 e2e 固件待 G3 的 `poll_event` 落地后以其为生产者补建。

## 3. e2e 第一手抓出的两个真缺口（已修）

1. **verifier `Project` 漏 `Type::Option` 臂**——G1 只接了 Variant op 校验；`case some(t)` 的投影被 IR verifier 以 TypeMismatch 拒（`function[2].block[1].instruction[1]` 精确点名）。修后绿。
2. **`some`/`none` 在无名期望位的行为定义**——`let x = match some("hi"):` 的 subject 无期望类型 ⇒ `unconstrained some` typed 拒（与 `unconstrained ok` 同模式）。这是设计决定而非缺口：v0 不做臂推断。

## 4. 诚实收窄

- 端到端 Option 证据（运行期判别字 + 投影）随 G3-R1 补：生产者 = `sico.ui.poll_event`。
- `none` 与 effects 空列表的关键字复用是现状延伸；若未来 `some`/`none` 需要出现在 RFC-0048 中缀操作数面，另开评估（v0 不需要）。
- examples/ 两份设计史草稿（`account-transfer`/`todo-store` 的 `ok(some(account))` 模式）早于本片预告了 Option 语法，未被执行路径消费、未改动。

## 5. 先存红（非本片引入，登记不修）

- IND-003（`zero-indent-recovery-declaration.sico` parser recovery）在 `sico-ir`/`sico-cli`/`sico-semantics` 三个测试面同根因红，stash 证实改动前即红。
