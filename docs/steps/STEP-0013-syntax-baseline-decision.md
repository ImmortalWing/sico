# STEP-0013: 决定 M1 表层语法基线

> - status: complete
> - phase: M0
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

基于 STEP-0012 的完整离线证据和 Sico 的 AI-first 设计优先级，在 A0、B、C 中选择一个可立即进入 M1 parser/formatter 的唯一基线；保留未测量边界和可执行复审条件，不把设计推断写成真实模型或 parser 实验。

## 2. Decision inputs

- 三套候选各 54 个镜像 P0 程序；
- A0/B/C 全量 lines、bytes、lexical/punctuation token 与关闭标签覆盖；
- 每套 12 个单点结构 mutation 及设计 recovery oracle；
- 96-task AI v1 协议与离线评分路径；
- SYN-001–006，尤其是唯一规范写法、局部恢复、可独立阅读签名和不同语义使用不同形式；
- 真实 parser/model 数据明确为 `not measured`。

## 3. Plan

1. 把硬约束、静态事实、推断和未知项分开；
2. 比较接受 A0、B、C 或立即创建混合语法的代价；
3. 用 RFC 固定 M1 baseline、淘汰/保留状态和 canonical surface；
4. 定义 parser、formatter、mutation 与真实 AI 的量化复审门槛；
5. 更新语法文档、候选索引、路线图和状态；
6. 运行全部离线回归、链接检查、提交并推送。

## 4. Non-goals

- 不实现 lexer/parser；
- 不删除 A0/C 历史证据；
- 不根据 fixture 生成模型排名；
- 不在未评测的混合语法上直接宣布胜者；
- 不冻结模块、属性、UI SDK 或全部标准库表层。

## 5. Commit

`docs(syntax): [STEP-0013] select labeled-block baseline`

## 6. Decision

接受 [`RFC-0005`](../rfc/RFC-0005-labeled-block-syntax-baseline.md)：

- B Labeled Blocks 是 M1 lexer/parser/formatter 的唯一 canonical surface；
- A0、C 不进入主 parser，但完整保留为最简与紧凑对照；
- 不立即创造 `fn` + labeled end 等未评测混合语法；
- 接受 B 相对 A0 +19.7% lexical token、+31.3% bytes 的静态代价，换取 151/151 具名块结束；
- 具名关闭改善恢复/AI 理解仍是 inference，真实 parser/model 保持 not measured。

## 7. Revisit gates

- M1：54/54 B case 可解析并稳定格式化；12/12 B mutation 命中根因与 recovery anchor；construct 外最多 1 个额外语法诊断；
- AI：同协议下 A0/C 的 generation 与 repair 均领先 B ≥5 percentage points，且 understanding 不落后 >2 points；
- token：B 比正确率相当候选多 ≥20% 供应商报告 token；
- 跨模型：两个模型家族出现同方向结果。

触发门槛只重开 RFC，不自动切换；fixture/smoke 不计入。

## 8. Validation

```text
SYNTAX_METRICS_OK metric=files/lines/bytes/tokens/punctuation/close-label-coverage A0=54/518/10067/2229/836/0 B=54/563/13217/2668/910/1 C=54/488/9990/2494/1266/0
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
MUTATION_CORPUS_OK entries=36 A0=12 B=12 C=12
AI_EVAL_DATASET_OK protocol=v1 tasks=96 generation=30 understanding=30 repair=36 A0=32 B=32 C=32
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=96 deterministic=true
DIAGNOSTICS_OK catalog=24 cases=29 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=58
SEMANTIC_QUERY_OK modules=10 symbols=24 relations=13 samples=10 operations=5 fixtures=8 accepted=5 rejected=3 max_result_bytes=3630
```

## 9. Audit links

- RFC: [`RFC-0005`](../rfc/RFC-0005-labeled-block-syntax-baseline.md)
- evidence: [`syntax-evidence-v1`](../reports/syntax-evidence-v1.md)
- syntax overview: [`SYNTAX.md`](../../SYNTAX.md)
- next: STEP-0014 M0 exit audit
