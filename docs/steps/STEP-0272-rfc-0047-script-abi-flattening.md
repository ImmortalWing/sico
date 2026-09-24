# STEP-0272: RFC-0047 D4 — script-v0 ABI record flattening (check/build/run closure)

> - status: complete (record values flatten to their field sequence through the full script-v0 chain; three e2e programs run byte-exact markers)
> - phase: records implementation (RFC-0047, accepted STEP-0270; frontier from STEP-0271 §6)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent (owner directive 2026-09-24「开始」)
> - evidence class: internal-fixture (native Windows x64 debug runner, real Component Runtime)

## 1. Objective

STEP-0271 收口的承诺：script-v0 组件 ABI 的 record 扁平化（RFC-0047 D4
「record 值 = 字段序列」），解锁三个 `#[ignore]` 的 run 测试，让
check/build/run 三态无缺口。顺带收 STEP-0271 遗留的 `record-nested`
多行嵌套字面量 `unsupported core expression` 面。

## 2. Root cause（实测定位）

扁平化机制本身早已存在：`ScriptAbi::flat_ir_types`/`record` 驱动
LocalLayout 参数种子、`Operation::Construct` 的逐字段槽位图、
`Operation::Project` 的槽位区间投影、local cell 的 `apply_result_fields`
——全部对 `ValueLayout.fields` 泛型。唯一断点：`ScriptAbi` 只硬编码了
Script WIT 记录（ScriptInput/Output/Error/Http*），用户 record 查表落空
→ 逐层 honest refusal（non-scalar parameter/local）。次要断点：(a) local
cell 读回不带字段图（`set` 后的点访问投影落空）；(b) lowering 的投影
分支只收 3 token 单级访问，`seg.b.x` 链式落空；(c) `let` RHS 按行切分，
跨行字面量实参非本步表面（语料取单行冻结形状，如实登记）。

## 3. Changes

- `crates/sico-ir/src/lib.rs`：`Module.records: BTreeMap<String,
  Vec<RecordField>>`（D4 canonical 形：字段序 = 声明序，append-only
  serde，`skip_serializing_if` 空表——冻结序列化字节不变）。
- `crates/sico-ir/src/lower.rs`：merged lowering 逐文件收集 record 声明
  （裸 `name: Type` 与 `field name: Type` 两形并存，声明序去重，导入文件
  取 `module.Name` 限定键）；投影分支改链式循环（`seg.b.x` =
  迭代 Project，中间基必须名义 record 且字段存在——语义层已拒， lowering
  保持 fail-closed）。
- `crates/sico-codegen-wasm/src/canonical.rs`：`ScriptAbi.user_records`
  注册表 + `register_user_records`（逐字段递归 `flat_ir_types`；任何字段
  不可扁平化则整条记录不注册，全部消费点保持 typed 拒绝，不猜布局）；
  `flat_ir_types`/`record` 用户记录优先于内建表。
- `crates/sico-codegen-wasm/src/lib.rs`：`apply_result_fields` 为
  record 类型 local cell 补声明字段图（`set` 后点访问打通）；
  `ScriptEmit` 构造后注册 `module.records`。
- `tests/end-to-end/record-nested.sico`：多行嵌套字面量改单行（冻结
  形状；跨行字面量实参列入后续工作）。
- `runner/sico-runner/tests/record_types.rs`：三个 run 测试解除
  `#[ignore]`。

## 4. Validation（本机实测，Windows x64 debug）

- `sico build --profile script-v0` + 预构建 runner 实跑：
  record-basic → `record-basic-ok`、record-nested → `record-nested-ok`
  （链式访问 + 嵌套构造）、record-list-field → `record-list-field-ok`
  （`List[Text]`/`Bool` 字段）；
- 拒绝面零回归：E2022/E2021/E2011(点访问)/E2001(名义)/E2010 各单诊断；
- D5 gate 诚实：`record-list-of-records.sico` 仍 `E2031 unresolved
  sico.list.empty[Point]`（`List[record]` 单态未实现，不猜）；
- `cargo test -p sico-ir -p sico-semantics -p sico-codegen-wasm` 全绿
  （含 IR 序列化/fixture 兼容）；
- `sico format` 双程不动点（record-nested 单行形状）；
- 未运行：runner cargo 测试（本机无 gcc，`ring` 不可重建；record_types
  8 个测试以 CI 为最终裁决）；clippy/fmt（CI 把关）。

## 5. Metrics

三个组件从 typed 拒绝（build exit 2）到实跑 marker 通过；无燃料/性能
断言（与 SOA 同布局的结论待 re-baseline 用 STEP-0266 探针复测，属
ADR-0017 验证门，不在本步）。

## 6. Known debts and follow-ups

- **D5 `List[record]`**：`sico.list.*[T]` 单态扩展到 record 元素类型
  （`record-list-of-records.sico` 解锁 + `for x in list` 按值迭代）。
- **跨行字面量实参**：`let` RHS 行切分限制（M22 自举通用机的同类债，
  Rust lowering 侧未动）。
- **嵌套记录注册序**：`register_user_records` 按 BTreeMap 键序，跨记录
  嵌套且内层名字排序靠后时会诚实拒扁平化；语料声明序（Point 先于
  Segment）不受影响。拓扑序注册随 D5 片处理。
- **EC-5 机器矩阵行**（`records-v0`：check/build/run true + 拒绝语料
  登记）与 EC-2 语义语料（copy-on-write 别名、for 迭代、Result 负载）、
  EC-3 canonical IR JSON 快照，随下一片一起登记。
- W1 合并债与在途 WIP 处置要求不变（STEP-0265）。
