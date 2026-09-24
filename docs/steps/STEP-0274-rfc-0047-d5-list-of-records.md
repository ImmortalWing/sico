# STEP-0274: RFC-0047 D5 — `List[record]` 列式单态（empty/length/get/append）

> - status: complete（D5 v0 收窄面落地 + gate 语料 run 解锁）
> - phase: records implementation (RFC-0047) — M22 路线（owner 指令 2026-09-24「完成M22，途中发现问题要反馈和回顾」）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture

## 1. Objective

落实 STEP-0273 §5 的 D5 决策：`List[record]` 按 **ADR-0016 列式** 实现四个 list 单态
（`empty`/`length`/`get`/`append`），解锁 `record-list-of-records.sico` 的 run gate，
保持「无未声明 check/build/run 缺口」的 M14 纪律。

## 2. v0 收窄面（owner 反馈项）

- **元素记录字段限 `I64`/`U64`**（每字段恰好一个 i64 cell）：字段含 Bool/Text/Bytes/List/嵌套
  record 的 `List[record]` 在语义层与 `flat_ir_types` 双层保持 typed 拒绝（E2031 /
  build 拒绝），不静默降级。for 迭代 over `List[record]` 不在本片（下一刀）。
- **`get` 越界错误的 error-tag 槽位 v0 未定义**：general Result 形状
  `[tag, field..., error-tag]` 强制 error 侧有槽，但 `NumericError` 无 out-of-range 变体
  （STEP-0077 冻结枚举）。v0 写 0 且语料只按 `case error(_)` tag 匹配；若未来允许读出
  该码，需先给 `NumericError` 增变体（跨 RFC 面，单独提案）。

## 3. 布局与实现（列式）

- list 值仍是 `(table, count)` 对；第 `f` 列在 `table + f*capacity*8`（**按 capacity 步距**——
  按 count 就地 append 会覆盖下一列，本片实现中实测踩到并修正）。
- header 在 `table-16`：magic `0x5349_4c4e`、count@4、capacity@8、tail `0x4c49_5354`@12，
  与标量数值 list 同方案；就地 append 条件（magic/tail/count 一致且 capacity > count）
  与几何增长拷贝逐列 MemoryCopy（旧步距 = 旧 capacity）。
- `get` 返回 general Result flat 形状 `[I32(tag), I64×C, I32(errtag)]`（record 无
  packed 特判）；`case ok(first)` 的字段图由 Project 分支现有 `record_fields` 补齐，未改。
- 语义 `infer_record_list_call` 要求记录已声明且字段全 I64/U64；IR
  `record_list_intrinsic` 解析 `sico.list.{empty,length,get,append}[Ident]`（排除五个
  标量元素名，保持闭文法）。

## 4. 途中发现并修正的实现缺陷（回顾项）

1. **append 拷贝守卫反置**：`count == 0` 时反而执行逐列拷贝，对 table=0 读 header 越界
   （wasm 内存错误 0xfffffff8）。改为 `count != 0` 执行拷贝。
2. **新元素写入被包进拷贝守卫**：空表首个 append 时元素存储整体被跳过，get 恒回 0/0。
   元素写入移到守卫之外。两者均靠 `record-list-of-records.sico` 实跑暴露（先 trap 后
   恒零），非静态审查发现——列式 helper 的指令序审查清单应加入「写入是否在所有路径上」。

## 5. Changes

- `crates/sico-ir/src/lib.rs`：`RecordListIntrinsic`/`RecordListOperation`、
  `record_list_intrinsic` 解析、`record_list_intrinsic_name`、`record_list_signature`。
- `crates/sico-semantics/src/lib.rs`：`parse_record_list_suffix` +
  `infer_record_list_call`（仅已声明 flat 记录可得类型，其余保持 E2031）。
- `crates/sico-codegen-wasm/src/canonical.rs`：`flat_ir_types` 的 `List(Named)` 分支
  （flat 全 I64 的记录 → `[I32, I32]`）。
- `crates/sico-codegen-wasm/src/stdlib.rs`：`record_list_helper_signature`、
  `emit_record_list_get`/`emit_record_list_append`（列式）；`helper_signature`/`emit_helper`
  增 `abi` 参数（仅 record-list 单态需要字段数）。
- `crates/sico-codegen-wasm/src/lib.rs`：`helper_dependencies`、stdlib 调用点、
  `emit_collection_call` 的 length 特判接入 record-list 名（该函数本就布局泛型，直接复用）。
- `tests/end-to-end/record-list-of-records.sico`：按脚手架风格重写为全 gate 语料
  （4 次 append 覆盖几何增长与就地路径、get 两端与越界、length），run →
  `record-list-of-records-ok`。
- `runner/sico-runner/tests/record_types.rs`：+1 run 测试（编译/运行以 CI 为准，本机
  runner 不可重建）。

## 6. Validation（本机实测）

- `record-list-of-records.sico`：check ok → build ok → 预构建 runner 实跑
  `record-list-of-records-ok`（exit 0）。
- 既有 15 语料零回归：8 accept（含 list-i64-family/sort/numeric、record-basic/nested/
  list-field/cow）+ 7 refusal（E2010/E2011/E2001/E2021/E2022/E2023/E2024）全部复测通过。
- `cargo test -p sico-ir -p sico-semantics -p sico-codegen-wasm` 全绿；
  `clippy --all-targets -D warnings` 三 crate 绿；`fmt --check` 绿
  （顺手修复 STEP-0271/0272 在途 WIP 的三处存量 clippy/fmt 违规：lower.rs
  `from_declarations` 行数、dump_oracle_ir.rs format 参数、canonical.rs 单模式 match）。
- `validate-step-0124.ps1` 绿、`validate-module-boundaries.ps1` 绿、`git diff --check` 绿。

## 7. Follow-ups

- for 迭代 over `List[record]`（按值绑定字段图重组）+ `List[List[record]]` 深度预算核对；
- EC-3 canonical IR JSON 快照（records 表序列化 byte-exact 对拍）；
- `record-nested` 多行字面量债仍在（STEP-0271 遗留）；
- M22 主线：records v0 至此全封 → 按 ADR-0017 进 W1（legacy lexer/parser 合并债，
  **在途 WIP parser.sico +228 行须先落地为独立 STEP 或吸收**）→ S3/S4 re-baseline →
  S5/S6/S7 → 自举闭环。
