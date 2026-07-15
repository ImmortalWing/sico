# STEP-0014: M0 退出审计

> - status: complete
> - phase: M0
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

对 M0 的显式 exit gate、计划交付物、RFC/ADR 状态、可执行验证和残余风险逐项取证，给出可审计的 `go`、`conditional-go` 或 `no-go` 结论；若允许进入 M1，同时明确哪些未完成项是 M1 工作、后续平台探针或需要外部授权的并行实验。

## 2. Sources of requirements

- [`docs/ROADMAP.md`](../ROADMAP.md) 的 M0 work packages 与 exit gate；
- [`DEVELOPMENT.md`](../../DEVELOPMENT.md) 的阶段、组件和即时任务；
- [`AGENT_GOAL.md`](../../AGENT_GOAL.md) 的真实性、测试、文档和 Git 纪律；
- STEP-0001–0013、已接受/提议 RFC 与 ADR；
- 当前工作树、锁定依赖、脚本输出和真实 Runtime 行为。

## 3. Audit method

1. 为每项门槛指定强证据与禁止替代的弱证据；
2. 检查文件/索引/状态一致性，不只搜索“complete”文字；
3. 重跑全部离线验证器、Rust format/lint/test、compile-fail、WIT 和 Component 实链；
4. 检查 deterministic artifact/hash、Git 清洁度、文档链接与未登记开放项；
5. 将结果分类为 `proven`、`partial/deferred`、`contradicted` 或 `missing`；
6. 只有所有 M0 必需门槛为 `proven`，或明确被 M0 定义允许递延且不阻止 M1 时，才给出进入 M1 的结论。

## 4. Non-goals

- 不把 M1 parser/type checker 当作 M0 必需品；
- 不把 Android 官方可编译性当作真机实测；
- 不把 WIT parser 当作 Future/Stream Runtime 往返；
- 不把 synthetic AI fixture 当作模型数据；
- 不把 proposed RFC 写成 accepted semantics；
- 不开始实现 M1。

## 5. Commit

`docs(audit): [STEP-0014] close M0 and authorize M1`

## 6. Gap closed during audit

ROADMAP 把 AI 常见错误分类标为 partial。新增 [`error-taxonomy.json`](../../ai-eval/error-taxonomy.json) 与独立校验器：12 类精确覆盖 24 个诊断 key、29 个语义负例、12 个 mutation intent/36 variants，并强制 `frequency=not-measured`。这完成分类 schema，不伪造真实模型频率。

同时更新过期开放项，并建立 [`M1 compiler frontend plan`](../plans/M1-compiler-frontend.md)，明确 STEP-0015–0021、workspace 边界和退出证据。

## 7. Audit result

[`M0 exit audit report`](../reports/m0-exit-audit.md) 对 14 项要求逐一取证；没有必需项 missing/contradicted。结论：

```text
GO: M0 complete; M1 entry gate satisfied.
```

真实 AI、Android、Future/Stream Runtime、RFC-0003/0004 接受条件均进入非阻塞递延登记，不改变其未测/提议状态。

## 8. Validation summary

- 全部离线验证器通过，包含新 taxonomy；
- 5 个 Cargo manifest rustfmt/Clippy 通过；
- numeric 10 tests；resource/async 10 tests + WIT 1 + compile-fail 2；
- sync/async Component 各重复两次、输出与 SHA-256 稳定；
- M0 聚合校验通过：14 steps、7 decisions、93 Markdown、509 local links、0 official AI runs；
- JSON/manifest 与 `git diff --check` 在提交前通过。

完整命令、输出、哈希和证据强度见审计报告。

## 9. Next

`STEP-0015`：按 M1 计划建立正式 Rust workspace，并在 lexer 代码前完成 source/token/Unicode/identifier/newline/trivia lexical RFC。
