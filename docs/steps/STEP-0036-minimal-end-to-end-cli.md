# STEP-0036: 打通最小端到端 CLI build/run

> - status: in-progress
> - phase: M3
> - started: 2026-07-15
> - completed: -
> - owners: autonomous-agent

## 1. Objective

为已经通过真实 Component codegen 与 Wasmtime 46.0.1 验证的同步 scalar 子集建立 source → semantics → typed IR → Component → Runtime CLI 链，并冻结 stdout、stderr、退出码和失败无产物契约。

## 2. Scope and gates

本步只接入 `main()` 的同步 scalar Component。语义错误必须继续在 IR 前以 exit 1 拒绝；后端未支持、I/O、Runtime 启动或执行失败使用 exit 2。`.sapp`、构建缓存、参数透传、capability host、trap 分类、`sico -c` 与 REPL 留给 M4+。

正式命令、产物覆盖策略、Runtime 查找规则和稳定文本仍需通过本步 RFC/测试审查后冻结，当前 WIP 不构成已接受 CLI 契约。

## 3. Implementation plan

1. 建立独立 `sico-runtime` 进程边界，临时 Component 必须清理；
2. 审查并接受最小 build/run CLI contract；
3. 接入 frontend/semantic/IR/codegen，成功后原子写产物；
4. 用真实 Wasmtime 验证 `main()`，覆盖 file/stdin、错误通道和确定性；
5. 运行 workspace、M2、M1、M0 回归，完成审查报告后再标记本步完成。

## 4. Handoff checkpoint

2026-07-15 下班交接时仅完成以下 WIP：

- workspace 已登记新的 `sico-runtime` crate；
- Runtime API 可把 Component 写入进程唯一临时文件，调用外部 Wasmtime `run --invoke`，分别返回 status/stdout/stderr，并在返回前清理文件；
- `sico-cli` 已预接入 IR、codegen、Runtime 依赖与测试用 `wasmparser`。

尚未实现 `build`/`run` 命令，尚未创建 RFC、集成测试、专项 validator 或最终报告，也尚未运行真实 Wasmtime 端到端验证。因此 STEP-0036 保持 `in-progress`，下一位执行者应从 CLI contract 审查和实现继续，不能把本提交视为完成证据。

## 5. Validation at checkpoint

交接提交前只运行 Rust format、`sico-runtime` tests/Clippy、locked workspace metadata 与 `git diff --check`。完整 STEP-0036 验收和前序阶段回归留给完成提交。

## 6. Audit links

- [`M3 plan`](../plans/M3-sico-ir-component.md)
- [`RFC-0012`](../rfc/RFC-0012-component-wit-boundary-v0.md)
- [`STEP-0035`](./STEP-0035-async-task-stream-backend.md)
