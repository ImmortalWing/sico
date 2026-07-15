# RFC-0004: Resource and async mapping v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target language/platform version: draft / WASI 0.3
> - supersedes: -
> - superseded-by: -

## Summary

Sico affine resource 映射到 WIT owned/borrowed resource handle；结构化 Task 保持语言内概念，跨 Component 的异步调用、单值结果和增量序列直接使用 WASI 0.3 `async func`、`future<T>` 和 `stream<T>`。

## Problem

若资源被降为可复制整数、借用可逃逸、任务可脱离父作用域，AI 生成的代码会出现 use-after-close、重复 await、泄漏和隐式并发。若继续为新接口自造 pollable/resource 异步层，又会复制 WASI 0.3 已正式提供的 Canonical ABI。

## Goals and non-goals

目标：唯一所有权移动、调用期借用、所有退出路径恰好一次清理、结构化取消、Future 单次结果、Stream 有界背压，以及与当前 WIT 的直接映射。

非目标：本 RFC 不定义正式 borrow checker、调度器、公平性、线程共享、WASI 0.2 adapter 或 detached task。

## Semantics

- resource handle 默认 affine：可移动或丢弃，不可隐式复制；
- 显式 `close` 消费 owned resource；未消费资源离开作用域时自动 drop；两者不会重复释放；
- 普通方法只借用 resource，借用不得跨调用、await 或返回边界逃逸；
- Task 必须属于词法 scope；完成/await 消费 task handle，丢弃 pending handle 或取消父 scope 会取消子任务；
- Future 是一次性异步结果；reader 被丢弃等同取消/关闭，producer 必须观察关闭；
- Stream 增量传输，不允许无界 collect；producer 在容量满时观察背压；reader 关闭会取消未完成生产；
- 取消是控制结果，不自动变为领域错误；资源清理仍执行；
- trap、领域 `Result`、取消和正常完成保持不同通道。

## Syntax candidates

本 RFC 固定语义而非表层拼写。候选语法必须能表达 owned 参数、调用期 borrow、结构化 scope、await 和 bounded collect，且不能通过 catch-all 隐藏取消或 trap。

## Positive and negative cases

现有 `affine-resources`、`future-task` 和 `stream` 正反例继续作为语言判定。Rust 原型额外验证：显式 close、错误路径 drop、父 scope 取消、Future reader 关闭和容量 2 的 Stream 背压。

负例 `use-after-move` 对应 Rust E0382；借用逃逸对应 E0515。未来 Sico 诊断沿用自身稳定 code，不暴露 Rust 错误文本。

## AST and IR

语义 AST/IR 必须区分 value、owned resource、borrowed resource、Task、Future 和 Stream。每个 resource move、borrow region、drop edge、task scope、await consumption、cancel edge 和 stream bound 都进入验证器及语义索引。

Task 不是普通可复制泛型值。Future/Stream 的 Component lowering 必须保留 Canonical ABI 类型，不能先降为整数句柄再丢失身份。

## Component/WIT mapping

- owned `resource` 参数/结果转移所有权；
- `borrow<resource>` 只在一次 Component 调用期间有效；
- 可挂起调用使用 `async func`；
- 单个延迟值使用 `future<T>`；
- 增量序列使用 `stream<T>`，通常和终止 `future<result<_, E>>` 成对；
- Sico Task scope、取消树和权限不作为通用 WIT resource 暴露；Runtime 负责把 reader close、instance termination 和父取消传播到调度器。

正式 [`async-flow-v0`](../../wit/async-flow-v0/world.wit) WIT 已由 `wit-parser 0.253.0` 解析。STEP-0010 真实验证了 owned/borrowed resource 与原生 `async func` Component 往返；STEP-0035 又用 Wasmtime 46.0.1 真实验证 `future<T>`/`stream<T>` identity roundtrip、reader close、pending future cancel acknowledgement 与 capacity 1/5 的 bounded stream demand。因此本 RFC 的映射与取消/背压方向已接受；compiler codegen 仍只实现其 IR 能显式表达并验证的子集。

## AI evaluation

评测任务应覆盖 move 后使用、borrow 逃逸、await 两次、task 逃逸、missing await、unbounded collect 和 reader 未关闭。修复答案必须指出最小所有权或 scope 变化，不能建议复制资源或改成 detached task。

## Compatibility

目标基线是 WASI 0.3。WASI 0.2 的 pollable adapter 属于兼容层，不进入 Sico 核心类型；接口包必须显式版本化。改变取消传播、owned/borrowed 方向或 Stream 终止形状是破坏性变更。

## Security and privacy

Runtime 必须限制 task 数、stream buffer、执行时间和 Component memory。关闭 reader/实例时回收宿主资源；诊断不泄漏宿主资源标识。借用不能跨不可信调用悬挂，资源表必须防止陈旧 handle 重用。

## Alternatives

- 自研 pollable resource：兼容旧 Preview 2，但重复 WASI 0.3 且组合性较差；仅保留 adapter。
- 回调：控制流和取消不局部，AI 理解成本高；不采用。
- detached task 默认：易泄漏且破坏结构化清理；不采用。
- 无界 Stream collect：可触发内存耗尽；不采用。

## Validation and acceptance criteria

已验证：10 个动态测试、2 个 Rust compile-fail、WIT parser、Clippy 零 warning、确定性 release 探针。

STEP-0010 已证明 owned/borrowed resource 调用和原生 `async func` 能由锁定版本工具生成、加载和往返，并实测同步 host lowering 会因 async 类型不匹配被拒绝。STEP-0035 已真实往返 `future<u32>` 与 `stream<u32>`，关闭两类 reader，确认 pending future cancel，并验证 `stream<u8>` consumer capacity 1 只接收 1/5 item、capacity 100 接收 5/5 后完成。未使用旧 pollable adapter。

## Links

- [STEP-0009](../steps/STEP-0009-resource-async-wit-prototypes.md)
- [report](../reports/resource-async-wit-v0.md)
- [STEP-0035](../steps/STEP-0035-async-task-stream-backend.md)
- [backend report](../reports/async-task-stream-backend-v0.md)
- [WIT reference](https://component-model.bytecodealliance.org/design/wit.html)
- [Component Model FAQ](https://component-model.bytecodealliance.org/reference/faq.html)
- [Wasmtime component API](https://docs.wasmtime.dev/api/wasmtime/component/index.html)
