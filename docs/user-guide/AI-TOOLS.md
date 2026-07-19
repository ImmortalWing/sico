# AI 工具

`sico-ai-tool` 是 compiler-backed JSON 工具，不是模型客户端。它不会访问文件系统、启动进程、联网或调用模型。

## 构建与调用

```powershell
cargo build --locked --release -p sico-ai-tools
Get-Content request.json -Raw | .\target\release\sico-ai-tool.exe
```

输入一个 UTF-8 JSON request，stdout 输出一个 JSON response。不要把日志混入 stdout。

## Inspect request

```json
{
  "schema": "sico.ai-tool.request.v0",
  "protocol_version": 0,
  "request_id": "inspect-1",
  "operation": "inspect",
  "budget": {
    "max_files": 16,
    "max_input_bytes": 1048576,
    "max_symbols": 256,
    "max_diagnostics": 100,
    "max_response_bytes": 1048576
  },
  "input": {
    "files": [
      {
        "uri": "file:///workspace/main.sico",
        "text": "function main() returns Int:\n  return 1\nend function\n"
      }
    ]
  }
}
```

结果包含源码 SHA-256/bytes、compiler diagnostics、Semantic Index symbols、snapshot quality 和预算使用情况，不回显输入源码。

## Validate fix

`validate_fix` 用于验证由编辑器、用户或模型提出的一个候选修复。请求必须绑定：

- URI；
- original 和 candidate；
- exact original SHA-256；
- original diagnostic-key multiset；
- `1..=256` 的 `max_changed_bytes`。

只有以下条件全部满足才成功：原始摘要和诊断仍匹配、candidate 无 syntax/semantic diagnostics、差异能表达为一个有界连续 edit、canonical format 仍无诊断、response 不超预算。

工具只描述经过验证的 edit，不会自动写文件。调用方应用 edit 前仍应检查当前 buffer digest。

## Plan execution

`plan_execution` 接受 `run`、`watch` 或 `repl`、可选 program/guest arguments 与 `1..=1048576` 的 `max_log_bytes`。结果是与 LSP 相同的 `sico.execution-plan.v0`，只包含 direct argv 与执行边界；工具不会启动这个 plan。

路径和参数中的空格、`;`、反引号或 `$()` 都保留为单独字面参数，`shell` 永远为 false。调用方必须执行 argument array、限制日志捕获，并按 plan 终止 direct child tree。

## 证据边界

仓库的 54-source/12-fix/96-task 数据用于验证工具链和评测协议，不是模型成绩。真实模型评测仍需要准确 model metadata、raw output、重复次数、凭据和成本授权。

完整 schema 与示例见 [tooling protocol](../../ai-eval/tooling-protocol.md)。
