# STEP-0269: RFC-0047 record-types draft registration

> - status: complete (draft registered; **owner acceptance required before any implementation STEP**)
> - phase: M22 R0 outcome (ADR-0017 Option A)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: planning-only (contract draft; measured-consumer chapters are frozen-corpus commitments, not yet measured)

## 1. Objective

owner 接受 ADR-0017 Option A（records RFC 先行，2026-09-24「开始」）后，
起草 record 类型 RFC 作为 R0 产出：[`RFC-0047`](../rfc/RFC-0047-record-types-v0.md)
（status: draft）。最小封闭集：record 声明（具名、不可变字段、标量 /
嵌套 record / `List[T]` 字段类型面）、具名字面量（复用现有标点
`(` `)` `:` `,`，**词法表零增长**）、点字段访问（与限定名共存的消歧规则 =
 typed 拒绝）、`List[record]` 可执行（单态化，List 边界降级到 ADR-0016
现有并行数组表示——guest 运行时成本不变，只有授权表面与语义层不变式改
变）。明确排除：可变字段/字段级 set、位置式字面量、record 模式匹配、
方法、泛型、`Map`/`Set` record 键值、Script WIT 面 record。

## 2. RFC-0033 四证据落点

- 显式降级故事：record 字面量 → 字段序列求值；`List[record]` → 现有
  SOA 并行数组表示（诚实声明：record 类型本身对 RFC-0008 typed IR 是
  附加面，非降级）；
- source-map 身份：声明块/字面量/字段访问全跨度，降级边界指向字面量
  而非合成临时量；
- 格式化幂等：声明序 = 规范字段序 + 字面量规范间距表（EC-1 语料）；
- typed 诊断：`record` 成保留字，既有同名标识符按 RFC-0001 catalog 给
  跨度 + 行动提示；新 E 码 append-only。

双实测消费者（EC-4，实现前冻结普查表）：selfhost 语料的列形状普查
（SOA 模拟行数占比 → 授权规模缩减投影）+ 应用 profile 语料（RFC-0038
M14 e2e 源码 + M22 §8 AI 生成噪音方法）。

## 3. Changes

- 新增 `docs/rfc/RFC-0047-record-types-v0.md`（Summary / 实测消费者 /
  D1–D8 决策点 / Grammar / source-map 身份 / 兼容迁移 / EC-1..EC-5 出
  口语料 / 证据等级 / 风险）；
- `docs/rfc/README.md`：登记 0043–0047 五行（顺带补齐 0043–0046 索引
  缺口——同类 P1-E 债务）、"下一可用编号"更正为 RFC-0048；
- 本记录 + steps 索引；STATUS 与 M22 计划头指针更新。

## 4. Validation

- 结构符合 RFC-0046 体例与 RFC-0033 reconsideration gate 要件；
- `tools/validate-step-0124.ps1` 通过；`git diff --check` 通过；
- 未运行：编译/测试（纯合同草案）。

## 5. Metrics

无执行指标。EC-4 普查表冻结后，"SOA 模拟行数占比"成为 R0 决议的验证
数字。

## 6. Risks and follow-ups

- 接受后实现顺序（ADR-0017 §Validation）：EC-4 普查冻结 → Rust 内核
  全链实现（parser/HIR/semantics/IR/verifier/codegen/formatter）→
  逐 STEP 冻结语料 → M22 Route-B 式 re-baseline（canary 重钉基线后
  R3 计数重启）。
- 与 M23 的关系：RFC-0047 独立于 M23 四项，可并行推进；M23 的开工
  盘点（四探针）不受影响。
- 下一步：owner 接受 RFC-0047（可附 D1–D8 修改意见）；接受后第一个
  STEP = EC-4 双普查冻结（纯测量，零语法改动）。
