# Sico Semantic Index and Query JSON v0

本目录定义可信语义索引，以及 `outline`、`describe`、`slice`、`impact`、`flow` 五类查询的统一 JSON 契约。规范行为见 [`RFC-0002`](../docs/rfc/RFC-0002-semantic-index-query-v0.md)；真实 compiler producer/query engine 见 [`sico-index`](../crates/sico-index/src/lib.rs)。

## Files

- [`protocol.json`](./protocol.json)：版本、操作、预算默认值、可信度和完整性词汇；
- [`sample-matrix.json`](./sample-matrix.json)：10 个代表性程序对五类查询和 AI01–AI07 风险的覆盖；
- [`schema/index-v0.schema.json`](./schema/index-v0.schema.json)：索引 snapshot 结构；
- [`schema/query-v0.schema.json`](./schema/query-v0.schema.json)：统一查询 request；
- [`schema/response-v0.schema.json`](./schema/response-v0.schema.json)：统一查询 response；
- [`fixtures/index.json`](./fixtures/index.json)：覆盖全部 10 个模块、详细展开 order-state/account-transfer 的设计 fixture；
- [`fixtures/manifest.json`](./fixtures/manifest.json)：五类合法查询与三类非法响应；
- [`tools/validate-semantic-query.ps1`](../tools/validate-semantic-query.ps1)：离线结构和跨字段校验器。

## Core rules

1. 所有实体使用不含源码 offset 的 `sico://...` semantic ID；
2. request 和 response 必须引用同一个 snapshot；
3. 事实必须标记 `declared`、`verified` 或 `inferred` 及证据；
4. `verified` 必须有 compiler analysis 或 SEM rule 证据，注释和模型摘要不够；
5. 截断、未知边或阻塞诊断会使结果成为 `partial`；
6. 默认不返回源码正文，只返回 source range 和继续查询引用；
7. 硬预算使用规范 JSON result 的 UTF-8 bytes，不能依赖模型 tokenizer；
8. 自动化读取结构化字段，不解析 `summary`、purpose 或其他自然语言。

## Minimal request

```json
{
  "schema": "sico.semantic-query.v0",
  "protocol_version": 0,
  "request_id": "q-describe-transition",
  "snapshot_id": "fixture:semantic-query-v0",
  "operation": "describe",
  "target": { "id": "sico://examples/order-state/function/transition" },
  "options": {
    "max_depth": 1,
    "max_items": 16,
    "max_bytes": 16384,
    "include_private": false,
    "include_source": false,
    "transitive": false
  }
}
```

## Validate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-query.ps1
```

这些 fixtures 仍是 M0 设计 oracle；fixture 中的 `complete` 只表示相对于 fixture/query oracle 没有省略。STEP-0028 已新增独立的 compiler producer：成功 B source 可生成 production `sha256:` snapshot；当前候选 A examples 会诚实产生 partial modules，不复用 fixture 的成功声明。
