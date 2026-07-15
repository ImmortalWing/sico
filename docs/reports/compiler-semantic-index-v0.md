# Report: compiler-produced Semantic Index/query v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0028
> - environment: Windows, Rust 1.97.0

## 1. Question

真实 `sico-semantics` facts 能否生成 RFC-0002 production snapshot，并在不把候选 A examples 伪装成 B compiler 成功输入的前提下，支持五类有界 query 与可审计 completeness/blocking diagnostics？

## 2. Method

新增 `sico-index` crate。输入先由真实 analyzer 处理；declaration facts 映射为 offset-free semantic ID、精确 name range、verified facet 与 contains relation。snapshot descriptor 覆盖 compiler profile、package、规范化 file identity 与 source bytes，并由内置标准向量验证的 SHA-256 生成。query engine 在同一 snapshot 上执行 outline/describe/slice/impact/flow，按 stable ID 排序并执行 item/UTF-8 canonical JSON byte budget。

测试使用每个 B group 一个成功 module，共 10 modules；另对 10 个代表性候选 A example 运行同一入口，确认它们因当前 B frontend 不接受而保持 partial。semantic invalid B module 的 E2001 进入 module/snapshot `blocked_by`。

## 3. Results

```text
STEP_0028_OK complete_modules=10 representative_partial=10 invalid_blocked=E2001 operations=5 snapshot=sha256 schema=index-query-response-v0 stable_ids=pass ranges=pass budgets=pass
```

- compiler snapshot：producer mode `compiler`，ID 为真实 `sha256:<64 hex>`；
- complete modules：10/10 成功 B module；
- honest partial：10/10 代表性 A example 不产生伪 symbols/complete 声明；
- blocking：semantic invalid module 与 snapshot 引用 E2001；
- semantic IDs：不含 source offset，构建顺序变化不影响结果；
- source：symbol range 精确选择 identifier，byte/line/column 一致；
- queries：5/5 operation 同 snapshot 成功，stale snapshot 拒绝，item/byte truncation 降级为 partial；
- schema：index/query/response required contract 与 RFC-0002 fixtures 全部回归。

## 4. Limits

v0 只索引当前 compiler declaration facts 和直接 contains relation；跨包/Component dependency discovery、持久缓存、服务权限与 M3 IR-verified flow 仍未实现。候选 A examples 继续是 partial 证据，不应据此迁移语言语法。

## 5. Links

- [`STEP-0028`](../steps/STEP-0028-compiler-semantic-index.md)
- [`sico-index`](../../crates/sico-index/src/lib.rs)
- [`RFC-0002`](../rfc/RFC-0002-semantic-index-query-v0.md)
- [`Semantic Index protocol`](../../semantic-index/README.md)
