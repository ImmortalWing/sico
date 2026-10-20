# STEP-0334: M24 #7(a) — gate 4 模型 provenance manifest v0（封闭 schema + 载入校验原语）

> - status: complete（实现切片：schema + 载入校验原语；**不移动任何 §3 gate**）
> - phase: M24 M17 线 #7 的 gate-4 provenance 子项（第一片：manifest schema + 结构/摘要校验）
> - completed: 2026-09-30
> - contracts: [`RFC-0051`](../rfc/RFC-0051-model-provenance-manifest-v0.md)（`sico.m17.provenance.v0`，owner 指令 2026-09-30 接受）
> - evidence: **internal-fixture / unit-level**（`crates/sico-package` 根 workspace，`1.98.0-x86_64-pc-windows-gnu`；`cargo test -p sico-package` provenance 10/10 绿）。**非 runtime-verified**：尚未接进 runner 的包安装/资产载入路径，故无「经真实 runner 的 digest-verified load」端到端证据。

## Objective

落地 RFC-0051 §2/§3 的**第一块、自包含、可单测**的切片：`sico.m17.provenance.v0`
的**封闭 manifest schema** 与**载入期校验原语**——即 §3 证据集的
property 1（digest-verified load）、property 2（坏摘要 / 截断 / 超长各为
**distinct 类型化拒绝**、无部分载入）以及 property 5 的**结构半部**（未知字段、
缺必填、畸形摘要、路径式名称、未知 `kind` 全部拒绝）。复用既有 M7 摘要纪律
（`validate_digest` + `sha256_hex`），**不引入第二条信任路径**（RFC-0051 §2）。

## Changes

- **新增模块**：[`crates/sico-package/src/provenance.rs`](../../crates/sico-package/src/provenance.rs)
  —— RFC-0051 §2 schema 的 serde 结构体（`AssetKind`/`Provenance`/`Budgets`/`Asset`/
  `ProvenanceManifest`，全部 `#[serde(deny_unknown_fields)]` 满足「封闭」要求）＋两个方法：
  - `ProvenanceManifest::parse(&[u8])`：JSON 解析后做结构校验（`validate()`：`validate_digest`
    钉摘要形制、拒空/路径式（含 `/ \ .. :`）`name`、拒空 `interface`、拒 `size == 0`）；
    任何 schema/结构违规统一为 [`PackageError::Manifest`]。
  - `verify_asset(&[u8])`：§3.2「无部分载入」——先比长度（`actual > declared` →
    `LengthMismatch`、`actual < declared` → `Truncated`），长度精确相等才比摘要
    （不符 → `DigestMismatch`）；三态互为 **distinct** 拒绝，超长（limit+1）与截断
    分别可测。
- **接线**：`crates/sico-package/src/lib.rs` 加 `pub mod provenance;`（`pub` 以便 runner 后续
  跨 workspace 复用同一原语，而非在 host 侧重写一套摘要/结构门）。
- 复用根 `PackageError`（`Truncated`/`Manifest`/`DigestMismatch`/`LengthMismatch` 既有变体，
  零新错误枚举、零信任面扩张）。

## Executed validation

- `cargo test --offline -p sico-package provenance` → **10 passed; 0 failed**：
  - `digest_verified_load_succeeds`（§3.1）；
  - `truncated_payload_is_distinct_refusal` / `oversized_payload_limit_plus_one_is_distinct_refusal`
    / `bad_digest_same_length_is_distinct_refusal`（§3.2 三态 distinct，含 limit+1）；
  - `unknown_field_refuses` / `missing_required_field_refuses` / `unknown_kind_variant_refuses`
    / `malformed_digest_refuses` / `path_like_name_refuses`（§3.5 结构半部）；
  - `round_trip_serialization_is_stable`（规范化序列化稳定）。
- `cargo clippy --offline -p sico-package --all-targets -- -D warnings` 绿；
  `cargo fmt -p sico-package -- --check` 绿；`cargo test -p sico-package`（含既有
  package/security/binary-carrying 13+4+1 测试）全绿，无回归。
- `git diff --check` 绿。工作树改动仅限：`provenance.rs`（新）+ `lib.rs`（一行 `pub mod`）。

## Honest residuals

- **本片不移动任何 M24/M17 §3 gate**。provenance 属 **M17 gate 4**（RFC-0051 标题即
  「M17 gate 4 — model/package provenance」），在 M24 plan §3 的七门编号里对应
  **§3 gate 2「model provenance 线」**（STEP-0333 出口审计即用此 §3 编号）——两套编号
  并存，勿混：加速器/速度门是 **M17 gate 2 = M24 §3 gate 1**，为外部 owner 硬件门，与本
  provenance 子项无关。gate 4（=§3 gate 2）的**完整**验收（RFC-0051 §3）要求**经真实
  runner** 逐条出证据：digest-verified load 通过 runner、budget 强制（property 3）、中途
  取消（property 4）、malicious-asset 探针在 runner 载入路径上拒绝。本片只交付**纯 crate
  级的 schema + 校验原语**，是它的**必要积木**，**不是** gate 的满足——如实按
  AGENTS.md「契约/原语层不冒充 runtime 证据」登记，不写支持矩阵行（无 runtime 声明可登记）。
- **property 3（budget 强制）与 property 4（中途取消）未实现、未声称**：二者是
  **推理执行路径**的运行期行为，属 runner 接线，不在 schema 层；在安全敏感路径上
  按 AGENTS.md §5「不做半成品 lowering」的告诫，**刻意**止于完整、可单测的原语层，
  不留半接的执行面。
- **下一步（gate 4 收口的 runner 接线，未在本片做）**：包内携带 `provenance.json` 制品 →
  runner 安装期摘要复核（M7 既有）＋ 载入期 `ProvenanceManifest::parse` +
  `verify_asset` → 由 `Budgets` 驱动运行期 typed 拒绝（hostcall 计数 / wall-time / 内存
  三闸）→ 中途取消产 typed outcome 且无 orphaned 写、无泄漏 host 态。此接线需一个真实
  **fixture model** 载荷（当前模型资产 = 0，见 STEP-0287 盘点），因此 gate 4 的「载荷级」
  证据还依赖先造一份冻结 fixture model。
- #7 的另一子项 **roster 三包**（match/grid/contour）仍被 **E8017 合同门**卡住
  （region `record` 返回不在 v0 包接口值集内，见 STEP-0330 §8），须 owner 三选一裁决，
  与本 provenance 子项相互独立；本片的推进不代表 roster 解锁。
- 未 commit / push（M24 分支无授权）；仅本地工作树。

## Follow-on: carry/verify/budget 接线的合同空白 → A1 草案（同轮，probe + 起草，未实现）

尝试把本片原语接进「包携带 + 安装校验 + 预算映射」时，发现 RFC-0051 §2/§3 把三条
**接线约定**留白（且非可自裁）：① manifest 在包内的携带位置；② `asset.name`→被校验
payload 资源的绑定规则（§3.5「manifest/payload mismatch」探针预设了该规则却未规定）；
③ `budgets`→ 运行期上限的映射与优先级——实测 `RuntimeLimits` 已有 `memory_bytes`/
`timeout_ms`/`hostcall_fuel`（`lib.rs:101-113`）三字段与三个预算一一对应（故预算强制应
**复用既有 fuel/epoch/limiter 机制**，不新建第二条上限路径）。按 AGENTS.md「不静默改合同」，
据此起草 [`RFC-0051 Amendment A1 (PROPOSED)`](../rfc/RFC-0051-amendment-A1-carriage-and-budget-mapping.md)
（推荐默认 + 备择 + owner 三决策点），并在 RFC-0051 头加 open-item 指针；**未自裁、未据
A1 改动任何 package/verifier/runner 行为**。gate 4 在 owner 复裁决 A1 前保持未达。

## Addendum（同轮）：冻结 fixture-model 语料（§3.1/§3.2/§3.5 internal-fixture，实盘字节）

把原语从「测试内联字节」升到**已提交的冻结语料**（不触 A1 接线、不改包格式，故不需要
owner 裁决即可落）：

- **语料**（`crates/sico-package/tests/fixtures/provenance/`）：
  - `model-fixture.bin` —— 256 B 确定载荷（字节 `0x00..=0xFF`）；
  - `model-fixture.provenance.json` —— 携带其**真实** SHA-256（`40aff2e9…944880`）与
    `size:256` 的合法 manifest（常量为提交值，非测试内计算）；
  - `malformed-unknown-field.provenance.json` —— 多带一个 `surprise` 字段的 §3.5 探针。
- **测试**（`crates/sico-package/tests/provenance_fixture.rs`，`include_bytes!` 编译期钉入，
  路径无关）：`fixture_corpus_is_self_consistent`（双夹具互锁、防漂移）+
  `digest_verified_load_over_frozen_fixture`（§3.1 真实字节）+
  truncated / oversized(limit+1) / bad-digest 三态 distinct（§3.2/§3.5）+
  unknown_field（§3.5 结构）。

**Executed**：`cargo test -p sico-package --test provenance_fixture` → **6/6 绿**；root
`sico-package` 全套 **36/36 绿**（12 单元含 10 provenance + 6 语料 + 13 package + 4
security + 1 binary-carrying，无回归）；`clippy --all-targets -D warnings` / `fmt --check`
绿；`git diff --check` 绿。

**仍为 internal-fixture / unit-level，非 runtime-verified，不翻任何 gate**；此语料**不**执行
A1 的 carry/binding/budget-mapping 任一约定，仅为 A1 获批后 runner 接线片预备一份可复用、
可复现的 fixture-model（回应 STEP-0333 §B「还需一份真实冻结 fixture-model 载荷」）。
