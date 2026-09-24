# STEP-0283: W2 收官片 — IND-004 首码 E2031 identity（unresolved call target）

> - status: complete（guest 修复 + 豁免清零已落地；runner 测试仓本机不可
>   重建——gcc/dlltool 缺失——集成链以 CI 裁决）
> - phase: M22 compiler self-host — W2（guest re-authorization；收官片）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机：checker 组件 235 条全量探针零豁免 strict 235/235；CI：集成链全量）

## 1. Objective

关闭最后一条豁免：IND-004（zero-indent-undeclared-capability）清单首码
E2031（unresolved call target）。STEP-0281 后 guest 已能以 E4001 拒绝
（emission 序在 E2031 之后），first-diagnostic identity 未闭；本片在
guest 补 E2031 判定并保证同线先于 E4001 发射。

oracle 语义（`crates/sico-semantics/src/lib.rs:3166-3177`）：表达式推断
的 callee 依次对照已知函数、imported 函数、enum 变体、类型构造器、
capability/resource 方法、Component/Stream 容差，全部未中 → E2031。

## 2. Changes（`selfhost/compiler_semantics.sico`）

- **`use` 行注册**（第一遍 0-indent 支）：`use math_util.twice` 形把
  module root（at(1)）记入 `imported_roots`——imported 函数调用的
  check-time 解析面。
- **local 类型表扩 move/borrow 形**：`let owned = move reader` /
  `let borrowed = borrow reader` 以源操作数的参数类型注册 local——
  resource 型 local 的后续方法调用（`owned.close()`）不再被误判。
- **E2031 body 检查**（E2023 块之后、return 线 E4001 之前，保证同线
  发射序先于 E4001）：对 body_parts 逐词扫描，词 w 同时满足
  ① 非内建 root（`builtin_call_root`：I64/U64/Float64/sico）
  ② 非本函数形参（parameter_type 为空）③ 非 local ④ 非 enum/record/
  nominal/resource 名 ⑤ 非 imported_roots ⑥ `has_dot_call(body_line, w)`
  （词边界 + **前字节非 `.`** + 引号外 + `.` + 字母词 + `(`）→ E2031。
- **`at_word_boundary` 收紧**：前字节为 `.` 视为非边界。E2011 的
  dot_access_field 同步受益（路径中段组件不再被当 root）。
- **E2011 local 扫描收紧**：仅当 local 类型 ∈ record_names 才扫——
  修本片引入的 resource 型 local 误报（`owned.close()` 被当未知字段）。

## 3. 误伤面与修复（探针/单条实证）

| 误伤 | 根因 | 修复 |
| --- | --- | --- |
| `sico.text.encode(` 的 `text` 被当 root | 中段组件前字节是 `.` 但旧边界判定放行 | `at_word_boundary` 收 `.` 前字节 |
| `owned.close()` 报 E2011（应 E5001） | move/borrow local 未注册 → 注册后 resource 型 local 被 E2011 当 record 扫 | E2011 仅扫 record 型 local + move/borrow 注册 |

## 4. 结果

- IND-004：E4001 → **E2031**（首码 identity 关闭）✅
- DIVERGENCE_WAIVER 1 → **0**（机制保留为空表，作未来 diverge 登记点）
- 分区 (116, 45, 73, 1) → **(116, 46, 73, 0)**——235 条全部 strict，
  **W2 收官**。

## 5. Validation（本机实测）

- `sico check selfhost/checker.sico` ok；`sico build --profile script-v0`
  成功。
- **checker 组件 235 条全量探针：零豁免 strict 235/235 pass**（296s）。
- 关键 8 条逐一核对 8/8（IND-004=E2031、use-after-move=E5001、
  borrow-across-await=E5003、modules-report/record-basic=accept 等）。
- `selfhost_checker.rs` 经 rustfmt；分区断言按 corpus-v0.json 独立复算
  (116,46,73,0) 一致。
- 本机不可运行（已知宿主限制）：runner 测试仓 cargo 构建，集成链全量
  以 CI 裁决。

## 6. Follow-ups

- S3/S4 records re-baseline（W2 收官后的下一波次）。
- guest 降阶前沿（R2 内容）：`close_code`/`direct_close` 的
  while-in-if（SKIP:GENERAL-WHILE-IF-REGION）。
- E2031 已知近似：裸调用（无点）的 unresolved target 未实现（语料无
  首码用例）；多行调用的跨行判定未建模——declared-subset 观察项。
