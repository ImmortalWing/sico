# STEP-0613d — M22 W3-a G2 scoping 订正:嵌套 `if` 已由 G1 覆盖;真缺口 = 4 个 stdlib helper 发射臂

> 状态:**recon/scoping complete locally(探针 + 决定性 bisect)** —— **零热文件改动**(codegen/parser/lib.rs SHA 未变);**未翻任何 gate、未 commit/push**。
> 日期:2026-11-10。基线 HEAD `e93f59ad`(工作树未提交)。续 [STEP-0613c](./STEP-0613c-m22-w3a-g1-span-and-echo-fixes.md)。

## 1. 本片目标与结论(订正 STEP-0612 的 G2 前提)

按 STEP-0612 切片,G2 = "formatter 实形多块 run(2 分支 + 3 return)逐字节 == oracle 28459 B"。本片探针 + bisect 得出**两个决定性订正**:

1. **嵌套 `if` 多块 run 的发射能力 G1 已经做到** —— formatter `main` 的形状(外层 `if is_utf8` + 内层 `if source_has_lex_error` + 3 return,~4-5 块)guest **已逐字节发射并通过 validator**(隔离 fixture 2085 B,`byte_equal=yes`、`validator=pass`,日志 `target/tmp/step0613c/g2-verify.log`)。⇒ **G1 的 per-block terminator + per-return-point result-area 推导已泛化到嵌套**;STEP-0612「G2 = 嵌套 if 处理」的前提**被推翻**,无需为此改 codegen。
2. **整份 formatter 的真挡板是 helper 函数,不是 run body** —— 决定性 bisect(日志 `target/tmp/step0613c/g2-bisect.log`):整份 formatter 拒 `ERR:E-SH-CG-IR-EMIT`;**把 run body 换成 echo(保留全部 helper)仍拒同一 identity** ⇒ 挡板在 helper,不在 run 形状。

## 2. 具名缺口:4 个无发射臂的 intrinsic

`cgs_intrinsic_hex`(`selfhost/codegen.sico:3614-3858`)分派固定 intrinsic 集,其余落到 `:3858` 的 `return "ERR:E-SH-CG-IR-EMIT"`。formatter helper 用到 **4 个无臂 intrinsic**:

| intrinsic | 使用点(formatter.sico) |
|---|---|
| `sico.map.get[Text,U64]` | `nearest_match`(:563)、`match_arm_levels`(:594)、`normalize_source`(:694) |
| `sico.map.put[Text,U64]` | `set_nearest_match`(:580)、`normalize_source`(:694) |
| `sico.map.empty[Text,U64]` | `normalize_source`(:694) |
| `sico.text.trim` | `normalize_source`(:694) |

**oracle 侧它们是运行时 helper(非 hostcall)**:`sico.text.trim` 走 `helper("sico.text.trim")`(`lib.rs:4585`);map 系走 `emit_collection_call`(`lib.rs:4986`)→ `stdlib.rs` 单态化 helper(`emit_map_put`:1182、`emit_map_get`:1353、`emit_find_entry`、`emit_collection_empty`)。**guest 逐字节复刻 = 数百行、高自举形状前沿风险 ⇒ 属 G3+ 范围,非 G2「最小改动」。** 前端已降低全部四者(整份 formatter IR pin `sico_compiler_lowers_full_formatter_byte_exactly` 绿)⇒ 缺口纯在发射器。

## 3. 探针当前状态(G1 后,改动前)

整份 formatter.sico 经 guest `--emit-component-hex`(标准预算 fuel 800e9 / 120s / 2560 MiB)仍 **typed refusal `ERR:E-SH-CG-IR-EMIT`**(与 STEP-0608 同 identity);~7.7 s wall,`A:main`(8.1 s)与 `B:run`(8.0 s)均拒。非燃料墙(远未触限额)。

## 4. 改动面

- **零热文件改动**:`selfhost/codegen.sico`(`5042ad42…`)、`selfhost/parser.sico`(`2649984b…`)、`crates/sico-codegen-wasm/src/lib.rs` SHA 均未变。
- 仅 `runner/sico-runner/tests/selfhost_formatter_component_measurement.rs` **+118 行**两个 `#[ignore]` 诊断探针(`g2_verify_nested_if_run_body`、`g2_bisect_whole_formatter_emission`)记录本发现;fmt 净。
- 未削弱任何钉、未改任何 expected、未翻任何 gate。

## 5. 验证(coordinator 独立复验)

- **7/7 钉绿**(`--exact`):`sico_run_emits_the_minimal_multiblock_run_component_byte_exactly`、`sico_is_utf8_gated_run_lowers_byte_exactly`、`sico_compiler_emits_the_run_instruction_segment_byte_exactly`、`step0507`、`step0513`、`step0526`、`sico_compiler_lowers_full_formatter_byte_exactly`(formatter IR pin)。
- `git diff --check` 净;热文件 SHA 与改动前一致。

## 6. W3-a 状态与 G3 待办

**W3-a 组件面仍 open。** G3 必须做:
1. 为 `sico.map.empty[Text,U64]` / `sico.map.put[Text,U64]` / `sico.map.get[Text,U64]` / `sico.text.trim` 补发射臂(helper 表项 + intrinsic 分派 + 从 `stdlib.rs` 逐字节复刻运行时 helper 体)。
2. 重跑整份 formatter 逐字节等同 —— 补齐这 4 臂后**可能暴露进一步挡板**(这 4 个 helper 自身还用 `match` on map 结果与嵌套 `while`)⇒ **G3 是迭代片**。

## 7. Gate 与边界

- **未翻任何 gate,M22 overall 仍 NO-GO**。
- 未动 WIT/RFC/ADR/限额;未 commit/push(遵 AGENTS §10.2 owner 授权)。
- `frozen_match_site_ledger` MS-024 为已知独立项(见 STEP-0613c §6:span 修复致 26 函数闭包 digest 过期,重冻结待 Python/owner),非本片范围。