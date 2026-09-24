# STEP-0275: RFC-0047 D5 补片 — for 迭代 over `List[record]` + 嵌套深度核对

> - status: complete（for 迭代落地 + 深度 2 不可构造核对完毕）
> - phase: records implementation (RFC-0047) — M22 路线（owner 指令 2026-09-24「完成M22，途中发现问题要反馈和回顾」）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture

## 1. Objective

补上 STEP-0274 遗留的 D5 尾片：`for x in list`（元素为 record）按值迭代，
并完成 `List[List[record]]` 嵌套深度预算核对。

## 2. Changes

- `crates/sico-semantics/src/lib.rs`：for 循环可执行元素集扩入「已声明且字段全
  I64/U64 的 record 类型」；E2001 行动提示同步更新。绑定变量类型 = 元素 record
  类型，循环体内点访问走既有字段图。
- `crates/sico-ir/src/lower.rs`：for 反糖的单态名选择为 `Type::Named(record)`
  开臂，用 `record_list_intrinsic_name` 拼 D5 括号名。反糖本体（subject spill
  cell + U64 索引 cell + `index < length` 的 while + `get` + `Project ok`）
  本就布局泛型，未动；`Project ok` 的 record 字段图由 STEP-0272 既有
  `apply_result_fields`/`value_layout` 覆盖。
- `tests/end-to-end/record-list-of-records.sico`：gate 语料扩入 for 迭代
  （对 `l4` 逐元素 `checked_add(p.x)`/`checked_add(p.y)` 累加，校验总和 36），
  run marker 不变 `record-list-of-records-ok`。

## 3. 嵌套深度核对（结论：v0 深度 2 不可构造，无需预算）

- `List[List[Point]]` 在**文法层即不可命名**：`sico.list.*[...]` 括号内只接受
  标识符（STEP-0274 的闭文法），`sico.list.empty[List[Point]]` → E2031
  unresolved call target（实测）。map/set 单态同理（ELEMENTS 封闭）。
- 语义层双保险：for 的 executable_element 只认 Text/I64/U64/flat record，
  `List[List[T]]` 元素是 Generic 而非 Named → E2001（实测）。
- codegen 层三保险：`flat_ir_types` 的 `List(Named)` 分支对内层 List 落
  `_ => None`，即便绕过前两态也拒于 build。
- 结论：v0 任何 list 嵌套（不限 record 元素）都不可构造，**无深度预算债**；
  未来若开嵌套（如 `List[List[I64]]`），需先扩文法再定深度预算，属独立 RFC 面。

## 4. Validation（本机实测）

- `record-list-of-records.sico`（含 for 迭代）：check ok → build ok → 预构建
  runner 实跑 `record-list-of-records-ok`（exit 0）。
- 深度核对两探针（for over record 字段 E2001；`List[List[Point]]` E2031+E2001）
  如 §3 实测。
- 既有语料零回归：record 4 accept + 7 refusal、list-numeric/list-i64-family/
  list-i64-sort 全复测通过。
- `cargo test -p sico-ir -p sico-semantics -p sico-codegen-wasm` 15 套件全绿；
  `clippy --all-targets -D warnings` 三 crate 绿（修一处 needless_borrow）；
  `fmt --check` 绿；`validate-step-0124.ps1`、`validate-module-boundaries.ps1`、
  `git diff --check` 绿。

## 5. Follow-ups

- records v0 全封（STEP-0271..0275）：剩余已知债 = `record-nested` 多行字面量、
  EC-3 canonical IR JSON 快照；
- M22 主线下一刀 = W1 合并债（legacy lexer/parser，**在途 WIP parser.sico
  +228 行须先落地为独立 STEP 或吸收**），随后 S3/S4 records re-baseline →
  S5/S6/S7 → 自举闭环。
