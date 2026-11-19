# STEP-0639 — M22 门 7 组件轴覆盖按选项 B 收窄（owner 裁决落地）

- 日期：2026-11-19
- 类别：implementation complete locally（**门 7 测试语义重定范围 = 选项 B 收窄；未翻任何 gate、未上调任何限额、未改测量逻辑/账本、未 commit（待 owner 授权）**）
- 关联：闭合 [`STEP-0627`](./STEP-0627-m22-hidden-issues-c1-c5-a1-a5-landing.md) B2 项登记的预存红 `every_match_site_is_measured_on_the_component_axis`（该 STEP 明确「HEAD 已红、非本片引入、留 owner 裁决」）
- 决策依据：owner 2026-11-19 就 [`m22-fuel-wall-decision-brief-2026-11-19.md`](../reports/m22-fuel-wall-decision-brief-2026-11-19.md) 裁决「接受，不等」——接受「不上调燃料 → 选项 B 收窄 + 随 B 修门 7 + 选项 C 立项」，不等剩余 4× 数字

## 1. 问题

`every_match_site_is_measured_on_the_component_axis`（门 7 回归）在 HEAD 即红：它断言「任何站点都不得 `closure-too-large`」，但 `parser.sico:scalar_ir` 的 25 个 match 站点（MS-375..399）closure = **165 fn / 1,049,726 B > 1 MiB closure fixture bound**，恒为 `closure-too-large`。

STEP-0550 写该断言时的前提是「closure bound 是 fixture 上限、抬 bound 即可覆盖最大真实闭包（当时 164 fn / 1,027,015 B）」。但 `scalar_ir` 此后长过 bound，**且**其 canonical IR 长过注入通道：

- 本片独立实测：`scalar_ir` closure 经 Rust oracle 降为 canonical IR ≈ **9,101,116 B**，注入时 `guest_emission_with_fuel` 直接 panic `the injected IR bounds hold: ChannelBytes(9101116)` —— 超 guest stdin `MAX_CHANNEL_BYTES`（8 MiB）。
- 佐证 [`STEP-0627`](./STEP-0627-m22-hidden-issues-c1-c5-a1-a5-landing.md) B2 独立测得的 9,102,533 B（8.68 MiB，超 8.5%；两数因注入前置不同略异，同指超通道）。

⇒ 对 `scalar_ir`，**抬 closure bound 不会得到组件**，只会把失败从 `closure-too-large` 位移成**注入 panic**。组件轴对它**本质上不可测量**（size + channel 双重超限），且这是永久的（除非拆小 `scalar_ir`），非「fixture 太小、抬 bound 可修」。

## 2. 选项 B 收窄（owner 裁决落地）

owner 裁决接受「不上调燃料 → 选项 B」。选项 B 的组件轴诚实口径：**组件轴覆盖每个站点，除了一个具名 not-comparable 残差集**。

落地改法（仅 `every_match_site_is_measured_on_the_component_axis`，`runner/sico-runner/tests/selfhost_compiler.rs`）：

- 该测试从「`closure-too-large` 站点数必须为 0」**重定范围为**「每个 `closure-too-large` 站点必须 **whole-file IR pin 背书**：其文件在 `MATCH_WHOLE_FILE_PINNED` 中 **且** 其 note 提名该 pin」。
- 无 pin 背书的 `closure-too-large` 仍判为**静默 fixture skip** 并失败（保留 STEP-0550 反腐烂意图）。
- `source_sha256` 为空的「未测量」检查同步豁免 pin 背书的 `closure-too-large` 残差（其 IR 由 whole-file pin 见证，非 per-site digest）。

据此，25 个 `scalar_ir` 站点被诚实接纳为 not-comparable 残差：其 IR 由 whole-file pin `sico_guest_arena_accommodates_full_parser_selfhost_input`（整份 `parser.sico` 字节级 IR 差分钉）见证，note 已提名该 pin。**无 pin 的静默 skip 仍会失败**——STEP-0550 的反腐烂意图保留。

## 3. 验证

- `every_match_site_is_measured_on_the_component_axis` = **ok**（此前 HEAD 红）。
- `cargo clippy --locked --offline --test selfhost_compiler` 干净；`rustfmt`（runner edition）干净。
- **测量路径零改动**：本片只改门 7 测试的**断言语义**，未改 `match_sico_closure` / `match_compare_site` / 账本 JSON / 任何 verdict ⇒ `sico_compiler_reproduces_the_frozen_match_site_ledger` 不受影响（未全量复跑——311 个 FuelExhausted 站点各烧至 120 s，全量 ~小时级；本片改动与其无交集）。
- 附带测量钩（均 `#[ignore]`，不影响默认测试）：`component_fuel()`（`SICO_M22_COMPONENT_FUEL` env 覆盖发射预算）+ `the_raised_fuel_flip_fraction_of_exhausted_sites_is_measured`（4× flip 产率探针，`SICO_M22_FLIP_GROUPS`/`SICO_M22_FLIP_FUEL`）。**未上调任何默认限额**（默认仍 800e9）。

## 4. 未做 / 留后续

- **选项 C（拆小 `scalar_ir`）未开工**：把巨型 `scalar_ir`（165 fn）拆成多个小函数以降单函数指令数、从根上降 O(I²) 发射成本——唯一既不抬预算又不收窄语义的真闭合路径。已在本片同期起草立项草案 [`M22-option-c-split-scalar-ir-proposal.md`](../plans/M22-option-c-split-scalar-ir-proposal.md)（draft，待 owner review）。
- **311 个 FuelExhausted 站点未闭合**：组件轴已尝试发射但发射器超 800e9 预算（燃料墙）。选项 B 不收窄它们（它们是「已测量、发射耗尽」，非 fixture skip）；闭合它们需选项 C 或数量级发射优化。
- **STATUS.md / steps README 登记**：因并发 M24 代理正持有这两个热点文件（§10.2 双写者约束），本片**未**写入，留并发代理提交后补登。
- **未 commit**：待 owner 明确授权。

## 5. 证据等级

internal-fixture / 单机快照。**未翻任何 M22 出口门；M22 overall 仍 NO-GO。**