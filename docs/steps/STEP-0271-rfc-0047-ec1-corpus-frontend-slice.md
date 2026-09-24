# STEP-0271: RFC-0047 EC-1 corpus freeze and frontend slice (semantics + IR field collection)

> - status: complete (EC-1 corpus frozen; semantics/IR slice landed and verified at check level; script-v0 record ABI is the next slice)
> - phase: M22 R0 → records implementation (RFC-0047, accepted STEP-0270)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent (owner directive 2026-09-24「你起草完直接动手就行」)
> - evidence class: internal-fixture

## 1. Objective

RFC-0047 接受后的实现第一片。owner 指令起草 EC-1 后直接动手，本 STEP
按「先冻语料、再实现、用语料验证」的顺序一次完成两片：

1. **EC-1 语法语料冻结**：9 个 `tests/end-to-end/record-*.sico` 源文件
   （e2e 目录按文件单独接线，不 glob，新增不扰动现有 CI）。
2. **前端实现片**：语义层 + IR lowering 层的裸字段行收集，及 D7 拒绝面
   （重复字段 E2021、空 record E2022、点访问未知字段 E2011）。

## 2. Measured starting point（实现前实测）

预构建 `sico.exe check` 探针（零改动基线）：RFC-0047 D1 声明块与具名字
面量**解析与构造检查已然通过**（`record` 自 M2/M3 起就是 Component/WIT
面的保留字与类型构造器，E2010/E2011/E2001 码已存在）——缺口只在三处：
(a) 字段收集只认 `field name: Type` 行，裸 `name: Type` 行收集为零字段；
(b) 字面量重复字段静默 last-write-wins；(c) 点访问未知字段静默落
unknown 洞。build 期实测再暴露 script-v0 适配器只接受标量（下一片的真
前沿，见 §6）。

## 3. EC-1 corpus（冻结）

| 文件 | 期望 | 验证状态 |
| --- | --- | --- |
| [record-basic](../..tests/end-to-end/record-basic.sico) | accept + run → `record-basic-ok` | check ✓（run 待 ABI 片） |
| [record-nested](../..tests/end-to-end/record-nested.sico) | accept + run → `record-nested-ok`（嵌套字段 + 访问链 `seg.b.x`） | check ✓ |
| [record-list-field](../..tests/end-to-end/record-list-field.sico) | accept + run（`List[Text]`/`Bool` 字段 + 多行字面量） | check ✓ |
| [record-list-of-records](../..tests/end-to-end/record-list-of-records.sico) | accept + run（`List[record]`，RFC D5） | **gate 到 D5 片**（`sico.list.*[Point]` 单态不存在，E2031 如实拒绝） |
| [record-refusal-empty](../..tests/end-to-end/record-refusal-empty.sico) | refuse **E2022** EMPTY_RECORD | ✓ 单诊断 |
| [record-refusal-duplicate-field](../..tests/end-to-end/record-refusal-duplicate-field.sico) | refuse **E2021** DUPLICATE_FIELD | ✓ 单诊断 |
| [record-refusal-unknown-access](../..tests/end-to-end/record-refusal-unknown-access.sico) | refuse **E2011** UNKNOWN_FIELD（点访问） | ✓ 单诊断 |
| [record-refusal-nominal](../..tests/end-to-end/record-refusal-nominal.sico) | refuse **E2001** TYPE_MISMATCH（名义不等价，同形 Point≠Size） | ✓ 单诊断 |
| [record-refusal-missing-field](../..tests/end-to-end/record-refusal-missing-field.sico) | refuse **E2010** MISSING_FIELD | ✓ 单诊断 |

新诊断码 append-only：E2021、E2022（D7 家族）；复用 E2010/E2011/E2001。
`record` 作标识符的 typed 拒绝语料：现有 lexer 已保留该字（M2/M3 起），
`record` 标识符拒绝沿 RFC-0001 catalog，不另立码。

## 4. Changes

- `crates/sico-semantics/src/lib.rs`：record 声明收集接受裸
  `Identifier : Type…` 行（与 `field` 行并存，组件/WIT 语料零扰动）；
  空 record 拒绝 E2022；字面量重复字段拒绝 E2021（消灭 last-write-wins）；
  record 类型点访问未知字段拒绝 E2011（消灭 unknown 洞）。
- `crates/sico-ir/src/lower.rs`：`Definitions::fields` 同步接受裸字段行
  （`Operation::Project`  lowering 原本就绪）。
- 新增 `runner/sico-runner/tests/record_types.rs`：5 个拒绝测试即刻生效；
  3 个 run 测试 `#[ignore]` 标 D4 ABI 片解锁。
- 9 个语料源文件（§3）。

## 5. Validation（本机，Windows x64，debug）

- `cargo test -p sico-semantics`：11/11 绿；`-p sico-ir -p sico-format` 全绿；
- 实证 `sico check`：3 accept 文件 check ok；5 拒绝文件各出唯一目标码；
- 实证 `sico build --profile script-v0`：honest refusal 链验证——
  record-basic/record-list-field → `unsupported non-scalar parameter/local`、
  record-nested → `unsupported core expression`（多行嵌套字面量参数面），
  **无隐藏 gap**：check 过而 build 拒的每处都是 typed refusal；
- 实证 `sico format` 双程不动点（D8 第一证据；字面量规范间距/声明块
  形状在 EC-2/格式化片补语料）；
- 未运行：runner cargo 测试（本机无 gcc，`ring` 构建脚本不可重建，预构建
  runner 二进制复用正常——CI 绿为准）；`clippy`/`fmt --check`（机器受限，
  CI 把关）。

## 6. Next slice（下一个实现 STEP，不预留编号）

script-v0 组件 ABI 的 record 扁平化（D4「record 值 = 字段序列」）：
`sico-codegen-wasm` 的 `LocalLayout`/`lower_parameter_type`/
`lower_result_type`/non-scalar local 与 `ScriptEmit` 记录布局；解锁 3 个
`#[ignore]` 的 run 测试；顺带核 `record-nested` 多行嵌套字面量的
`unsupported core expression` 面。EC-5 机器矩阵行（check true /
build false / run false 起步）随该片登记。D5（`List[record]` 单态 +
`record-list-of-records.sico`）再下一片。已知债：字段序现存 BTreeMap
字典序，D4 规范序（声明序）随 IR canonical 片处理并补幂等语料。
