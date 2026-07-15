# Report: M0 exit audit

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0014
> - conclusion: GO to M1 with non-blocking deferred evidence register

## 1. Decision

M0 的设计与技术基线门槛已满足，允许进入 M1 compiler frontend and diagnostics。该结论是 `GO`，不是对语言稳定、Android 支持、真实 AI 优胜、Future/Stream Runtime 或沙箱安全的声明。

允许进入 M1 的理由：M0 要求的方向、代表程序、P0 判定、三套候选、语法决定、诊断/查询协议、语义草案、Rust 技术原型、真实 Component 链和 Runtime 平台选择均有直接证据；所有未完成事项已按后续阶段、RFC/ADR 或外部授权实验登记，不会由 parser 实现隐式决定。

## 2. Evidence standard

- `proven`：当前仓库文件与本次真实命令直接证明；
- `proven for M0 design scope`：证明设计 oracle/protocol，不冒充未来 compiler/platform behavior；
- `deferred, non-blocking`：M0 明确只需草案、协议或选型，真实实现属于后续阶段或外部授权；
- `missing/contradicted`：没有足够证据；若任何必需项落入此类则不得 GO。

## 3. Requirement-by-requirement audit

| ID | M0 requirement | Strong evidence | Result |
|---|---|---|---|
| M0-01 | 方向、非目标、Rust/Component/no-JS 路线 | [`DIRECTION.md`](../../DIRECTION.md)、[`DEVELOPMENT.md`](../../DEVELOPMENT.md) | proven |
| M0-02 | 至少 10 个代表程序与跨样本审计 | [`examples/PLAN.md`](../../examples/PLAN.md)、[`ISSUES.md`](../../examples/ISSUES.md)、[`SEMANTICS-AUDIT.md`](../../examples/SEMANTICS-AUDIT.md) | proven: 10/10 |
| M0-03 | 核心语义、P0 正反例与明确根因 | [`SEMANTICS.md`](../../SEMANTICS.md)、[`semantic-cases/`](../../semantic-cases/README.md)、diagnostic map | proven for design scope: 54 = 25 accept + 29 reject，24 keys |
| M0-04 | 至少三套语法与同语义对照 | [`syntax-candidates/`](../../syntax-candidates/README.md)、[`metrics-v1.json`](../../syntax-candidates/metrics-v1.json) | proven: A0/B/C 各 54，162 programs |
| M0-05 | 语法错误注入、AI 理解/生成/修复基线 | [`syntax-mutations/`](../../syntax-mutations/README.md)、[`ai-eval/`](../../ai-eval/README.md)、[`syntax-evidence-v1`](./syntax-evidence-v1.md) | proven offline: 36 mutations、96 tasks；真实模型 deferred |
| M0-06 | AI-oriented 错误分类 | [`error-taxonomy.json`](../../ai-eval/error-taxonomy.json) | proven designed taxonomy: 12 类覆盖 24 diagnostics、29 semantic cases、12/36 mutation；frequency not measured |
| M0-07 | 数据驱动语法工程决定 | [`RFC-0005`](../rfc/RFC-0005-labeled-block-syntax-baseline.md)、[`STEP-0013`](../steps/STEP-0013-syntax-baseline-decision.md) | proven: B 是 M1 唯一 baseline，A0/C 保留对照 |
| M0-08 | 诊断协议 v0 | [`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md)、[`diagnostics/`](../../diagnostics/README.md) | proven design contract: accepted，24 code，29 mappings，fixtures pass |
| M0-09 | 语义索引与查询 JSON v0 | [`RFC-0002`](../rfc/RFC-0002-semantic-index-query-v0.md)、[`semantic-index/`](../../semantic-index/README.md) | proven design contract: accepted，5 operations，fixtures pass |
| M0-10 | 契约、效果、能力、资源、异步草案与可实现性 | `SEM-080–113`、[`RFC-0003`](../rfc/RFC-0003-numeric-representation-v0.md)、[`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md)、STEP-0008/0009 | proven for M0 draft/prototype；RFC-0003/0004 保持 proposed 条件明确 |
| M0-11 | Rust → Component → Runtime → WIT host call | [`component-host-call`](../../prototypes/component-host-call/README.md)、[`STEP-0010`](../steps/STEP-0010-component-runtime-host-call.md) | proven by real rerun: sync/resource/numeric/async func |
| M0-12 | Desktop/Android Runtime 对比与选择 | [`ADR-0002`](../adr/ADR-0002-runtime-platform-baseline.md)、[`runtime report`](./runtime-desktop-android-v0.md) | proven for M0 choice: Wasmtime；Android true run deferred to probe |
| M0-13 | 关键开放项不由 compiler 隐式决定 | `SEMANTICS.md §17`、`DEVELOPMENT.md §21–22`、本报告 §6、[`M1 plan`](../plans/M1-compiler-frontend.md) | proven: 每项有 RFC/ADR/阶段/STEP owner |
| M0-14 | 审计、可复现文档与阶段状态 | STEP-0001–0014、ROADMAP/STATUS、validators、Markdown link check | proven after final validation |

没有 M0 必需项为 `missing` 或 `contradicted`。

## 4. Offline and document validation

```text
SYNTAX_METRICS_OK metric=files/lines/bytes/tokens/punctuation/close-label-coverage A0=54/518/10067/2229/836/0 B=54/563/13217/2668/910/1 C=54/488/9990/2494/1266/0
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
MUTATION_CORPUS_OK entries=36 A0=12 B=12 C=12
AI_EVAL_DATASET_OK protocol=v1 tasks=96 generation=30 understanding=30 repair=36 A0=32 B=32 C=32
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=96 deterministic=true
ERROR_TAXONOMY_OK classes=12 diagnostics=24 semantic_cases=29 mutation_intents=12 mutation_variants=36 related_issues=26 frequency=not-measured
DIAGNOSTICS_OK catalog=24 cases=29 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=58
SEMANTIC_QUERY_OK modules=10 symbols=24 relations=13 samples=10 operations=5 fixtures=8 accepted=5 rejected=3 max_result_bytes=3630
M0_EXIT_DOCS_OK steps=14 decisions=7 markdown=93 local_links=509 official_ai_runs=0 current_phase=M1 next=STEP-0015
```

AI fixture/smoke 只证明 harness；`ai-eval/runs/` 没有正式 model run。错误 taxonomy 明确禁止 frequency ranking。

## 5. Rust and Component validation

环境：

```text
rustc 1.97.0 (2d8144b78 2026-07-07)
cargo 1.97.0 (c980f4866 2026-06-30)
rustfmt 1.9.0-stable (2d8144b788 2026-07-07)
```

本次重跑：

- 5/5 Cargo manifest `cargo fmt --check`；
- numeric/resource-async/component-host/guest/async-guest Clippy `-D warnings`；
- numeric 10/10 unit tests；
- resource/async 10/10 unit tests + 1/1 WIT parser + 2/2 expected Rust compile-fail；
- numeric/resource release probe 成功；
- sync Component 连续两次输出 50、4096-bit 往返 512 bytes、Decimal roundtrip true、事件顺序逐字一致；
- sync Component SHA-256 `849a03066d38a599cfd680bcc3c692223e58ae595a592839698b7f527b2e4701`；
- native async Component 连续两次输出 42、host calls 1；
- async Component SHA-256 `e43809a7ce88bdad84e04ee01b413c7b828d641ddd90f57e6013d1dda4904c9f`。

结果：`RUST_M0_AUDIT_OK manifests=5 fmt=pass clippy=pass tests=pass probes=pass component=pass`。

## 6. Deferred evidence register

| Item | Current truth | Why M0 can exit | Required owner/gate |
|---|---|---|---|
| Real AI model comparison | 96-task v1 ready；no model scores | credentials/cost/model choice are external；AGENT_GOAL requires explicit block, not fabricated data | parallel after authorization；RFC-0005 revisit thresholds |
| Parser recovery/formatter | no formal compiler | these are M1 deliverables, not M0 evidence | STEP-0015–0021 / M1 exit |
| RFC-0003 numeric acceptance | Windows prototype + Component record roundtrip proven；cross-platform/limit diagnostics remain | M0 requires feasibility and draft, not stable numeric spec | M2/M3 or parallel non-Windows probe |
| RFC-0004 Future/Stream acceptance | resource + state machine + WIT parser + native async func proven；future/stream Runtime not | M0 requires mapping feasibility; RFC correctly remains proposed | before M3 async lowering acceptance |
| Android Wasmtime/Pulley | official Tier 3 and policy analysis only | M0 requires audited route, Android product is M6 | ADR-0002 minimum probe before M6 implementation claim |
| Unicode/identifier/file grammar | explicit open decision | RFC-0005 intentionally excludes it; lexer must not guess | STEP-0015 before lexer implementation |
| Compiler-generated diagnostics/index | schemas/fixtures only | implementation belongs to M1/M2 | STEP-0018/0020 and M2 |
| `.sapp`, signatures, sandbox, UI | designs/open platform choices only | M4–M6 deliverables | respective phase gates |

任何递延项都不能在 M1 文档中改写为已完成。若 STEP-0015 无法先固定 lexical contract，则 M1 实现暂停并新建 RFC。

## 7. Decision record states

Accepted：ADR-0001、ADR-0002、RFC-0001、RFC-0002、RFC-0005。

Proposed with explicit unmet acceptance criteria：RFC-0003、RFC-0004。M0 不需要把它们错误升级为 accepted；其 verified 子集已经足够证明实现方向可行。

## 8. M1 authorization and next plan

进入 M1 不等于开始类型系统。执行 [`M1 compiler frontend plan`](../plans/M1-compiler-frontend.md)：

1. STEP-0015：workspace + lexical/source RFC；
2. STEP-0016：source/span + lossless lexer；
3. STEP-0017：B grammar + happy-path parser；
4. STEP-0018：recovery + E1xxx diagnostics；
5. STEP-0019：canonical formatter；
6. STEP-0020：`sico check/format/outline`；
7. STEP-0021：fuzz/performance + M1 exit audit。

每一步继续使用独立 STEP 文档、实现/测试同提交和验证后推送。M1 parser 只接受 B canonical corpus；A0/C 继续作为实验输入，不进入正式 grammar。

## 9. Final conclusion

`GO: M0 complete; M1 entry gate satisfied.`

该结论的范围是“可以开始实现前端”，不是“项目完成”。项目的类型检查、IR、Component codegen、`.sapp`、Runtime sandbox、Desktop/Android Host 和生态仍属于 M2–M7。
