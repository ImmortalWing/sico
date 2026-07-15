# STEP-0027: 实现 revision/contract dataflow

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 revision 的 4 个 B case 建立静态 contract dataflow：versioned commit 必须携带 expected Revision，应用 loaded payload 前必须由 revision comparison guard 支配。

## 2. Boundaries

- 只激活 E7001、E7002；
- expected revision 来自 capability method signature，不按文件/case ID 判断；
- loaded payload 只在同一函数内受显式 revision equality guard 支配时可应用；
- 不添加自动 retry、merge、last-write-wins 或不可见 runtime conflict handling。

## 3. Acceptance

- revision 2/2 valid 零诊断；
- 2/2 invalid 各恰好一个 primary diagnostic，完整匹配 catalog map；
- revision requirement/guard/use facts 有 stable composite ID/range；
- 其余 B group 不产生未拥有 E7xxx；
- workspace/M1/M0 regression。

## 4. Commit

`feat(semantics): [STEP-0027] check revision contracts`

## 5. Changes and validation

- capability method signature-derived expected Revision requirement；
- versioned record payload 与显式 equality guard dominance；
- 2/2 valid 零诊断；2/2 invalid 各一个精确 primary diagnostic；
- 其余 B group 无 E7xxx；无隐藏 runtime conflict handling；
- workspace/M1/M0 regression 通过。

```text
STEP_0027_OK valid=2 invalid=2 exact_primary=2 codes=E7001,E7002 expected_revision=signature-derived stale_guard=dominating-branch stable_facts=pass runtime_conflict=absent
```

## 6. Next

STEP-0028 由真实 compiler facts 生成并验证 10-module Semantic Index/query v0。
