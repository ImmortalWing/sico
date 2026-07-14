# AI evaluation run format v0

运行文件是 UTF-8 JSON：

```json
{
  "schema_version": 1,
  "protocol_id": "sico-ai-eval-v0",
  "run": {
    "run_id": "provider-model-date-sequence",
    "kind": "model",
    "synthetic": false,
    "scope": "full",
    "task_ids": ["所有 42 个 task id"],
    "repetitions": 30,
    "model": {
      "provider": "provider",
      "name": "model family",
      "version": "exact immutable version or snapshot"
    },
    "date": "YYYY-MM-DD",
    "parameters": {
      "temperature": 0,
      "top_p": 1,
      "seed_supported": false,
      "seed": null
    },
    "token_accounting": "provider-reported",
    "cost_currency": "USD",
    "cost_total": 0,
    "rate_limits": "record the applied request/token limits",
    "prompt_packet_sha256": "lowercase SHA-256"
  },
  "responses": [
    {
      "attempt_id": "unique-id",
      "task_id": "GEN-A0-NUM-003",
      "repetition": 1,
      "raw_output": "exact unmodified model text",
      "input_tokens": 0,
      "output_tokens": 0,
      "cost": 0,
      "latency_ms": 0
    }
  ]
}
```

## Rules

- 正式候选比较必须使用 `kind: model`、`synthetic: false`、`scope: full` 和至少 30 次重复；
- `version` 必须是可辨认的模型快照，不能只写 `latest`；
- 不支持 seed 时必须同时记录 `seed_supported: false` 与 `seed: null`；
- `raw_output` 必须是供应商原始文本，不能先人工修正或去掉 Markdown；
- 每个 `task_id × repetition` 必须恰好出现一次，`attempt_id` 全局唯一；
- token、成本和延迟使用供应商值或外部适配器测量值，不允许估算后冒充供应商报告；
- 原始运行文件可能较大，默认保存在版本控制外的受控实验存储中；报告记录哈希、位置、保留策略和评分器提交；
- `smoke` scope 只用于适配器联调，不得用于候选结论；
- `synthetic: true` 仅允许 fixture，评分结果会保持 synthetic 标记。
