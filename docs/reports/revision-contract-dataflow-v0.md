# Report: revision contract dataflow v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0027
> - environment: Windows, Rust 1.97.0

## 1. Question

revision 的 4 个 B source 能否只依靠 capability signature 与显式 control-flow guard 静态检查，并拒绝缺失 expected revision 或未检查 stale payload，而不引入隐藏 runtime 行为？

## 2. Method

从 capability method signature 解析首个 `Revision` parameter，并在 receiver call 上验证实际首参类型；从 record field model 识别带 `text`/`revision` 的 versioned payload。对 `if payload.revision == ...` 建立精确 block range，只有被该 range 支配的 `payload.text` use 才被接受。guard/requirement/use 事实按 HIR ID 与参数 slot 区分。测试读取 4 个 B source 与权威 map，并确认其他 B group 无 E7xxx。

## 3. Results

```text
STEP_0027_OK valid=2 invalid=2 exact_primary=2 codes=E7001,E7002 expected_revision=signature-derived stale_guard=dominating-branch stable_facts=pass runtime_conflict=absent
```

- 2/2 valid：零诊断；
- 2/2 invalid：各一个 primary diagnostic，完整匹配 catalog map；
- commit requirement：从 `Store.commit(expected: Revision, ...)` 推导，不读取 case ID；
- stale guard：只有显式 revision equality 所支配分支可应用 loaded text；
- facts：requirement、guard、use 有稳定复合 ID/range；同一 guard 的两个 operand 使用不同 slot；
- 未生成 retry、merge、last-write-wins 或 runtime conflict handler。

## 4. Limits

分析限于当前 P0 的单函数结构化 `if` dominance，不承诺任意 CFG join、跨函数 contract 或并发 transaction proof。扩展必须先补最小 case 与语义决定。

## 5. Links

- [`STEP-0027`](../steps/STEP-0027-revision-contract-dataflow.md)
- [`sico-semantics`](../../crates/sico-semantics/src/lib.rs)
- [`diagnostic case map`](../../diagnostics/semantic-case-map.json)
