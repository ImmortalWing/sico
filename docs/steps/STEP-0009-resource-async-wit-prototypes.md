# STEP-0009: 验证 resource、async 与 WIT 映射

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

用独立 Rust 动态原型与可解析 WIT 验证 Sico affine resource、结构化 Task、Future 和有界 Stream 的所有权、清理、取消与 Component 映射，并确定 STEP-0010 应实测的最小链路。

## 2. Context and evidence

- SEM-090—093 规定 resource 默认 affine、所有退出路径清理、跨 Component 使用 resource handle；
- SEM-100—106 规定 Future/Task/Stream、结构化并发、取消和背压草案；
- Component Model WIT 规定 owned resource handle 转移唯一所有权，`borrow<T>` 只在调用期间借用；
- WASI 0.3 已于 2026-06-11 发布，`async func`、`future<T>`、`stream<T>` 已是 Canonical ABI 原生类型；
- Wasmtime 当前提供 component-model-async 的 Future/Stream reader/producer API，并要求 reader 被 close/pipe 以免泄漏；
- STEP-0008 已证明“语言语义、WIT 值和 Runtime policy”必须分层。

官方依据：

- <https://component-model.bytecodealliance.org/design/wit.html>
- <https://component-model.bytecodealliance.org/reference/faq.html>
- <https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md>
- <https://docs.wasmtime.dev/api/wasmtime/component/index.html>

## 3. Scope

包含：

- owned move、borrow 作用域、explicit close 与 drop-on-scope-exit；
- 正常、错误、提前返回和取消路径恰好一次清理；
- Future 单次完成/取消，Task 父子作用域与有界 Stream 背压；
- WASI 0.3 原生 `async func`、`future<T>`、`stream<T>` 和 WIT `resource` 草案；
- WIT parser 工具验证、Rust 单元/compile-fail 测试和可复现报告。

不包含：

- 正式 Sico borrow checker、调度器或 Runtime；
- 自研 pollable ABI 或兼容 WASI 0.2 的长期适配层；
- 在本步骤执行真实 Component host call（属于 STEP-0010）；
- 无界 collect、detached task 或资源跨线程共享默认许可。

## 4. Options and decision

异步边界候选包括：旧式 resource/pollable、回调接口、WASI 0.3 原生 async 类型。由于 WASI 0.3 已正式加入 Canonical ABI future/stream，本步骤采用原生类型；pollable 只可能作为 0.2 adapter，不进入 Sico 核心语义。

resource 使用 WIT owned handle 表示所有权移动、`borrow<T>` 表示调用期借用。Sico Task 是语言内结构化调度概念，不直接伪装成可随意复制的 WIT value；跨界异步结果映射为 future/stream，取消与 reader close 必须触发生产端清理。

## 5. Plan

1. 建立 Rust resource/async 状态机与恰好一次清理计数；
2. 增加 move 后使用等 compile-fail 证据；
3. 实现单次 Future、父子 Task scope 和有界 Stream/backpressure 模拟；
4. 编写 WASI 0.3 WIT world，并用当前工具真实解析；
5. 运行正反测试和确定性探针，记录结果；
6. 写 RFC/报告，更新状态，执行全量回归并提交推送。

## 6. Changes

- 新增 `prototypes/resource-async`，实现 consuming close、Drop、调用期 borrow、scope-bound Task、one-shot Future 和 bounded Stream；
- 新增 10 个动态单元测试、2 个真实 compile-fail 案例与执行器；
- 新增 WASI 0.3 `resource`/`async func`/`future`/`stream` WIT，并由 `wit-parser 0.253.0` 解析；
- 新增 release 探针和机器可读结果；
- 新增 [`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md)（proposed）与实验报告；
- 取消使用依赖 GNU `dlltool` 的 trybuild，compile-fail 改为直接调用 rustc metadata 编译，不降低拒绝证据。

## 7. Validation

已运行 rustfmt、Clippy `-D warnings`、10 个动态测试、1 个 WIT parser 测试、2 个 compile-fail 和 release 探针；全部通过。

## 8. Metrics

探针记录 explicit close 1、implicit drop 1、task complete 1、cancel 1、max in-flight 2、backpressure 1；WIT parser 1 passed；compile-fail 2 rejected/0 accepted。

## 9. Risks and follow-ups

- WASI 0.3 与 Wasmtime async API 很新，版本与 feature flag 必须锁定；
- Rust 所有权只能证明原型可实现，不能替代未来 Sico 静态语义实现；
- async reader 未 close 会泄漏/挂起，必须进入 Runtime 守卫策略；
- STEP-0010 需选择最小同步 host call 与一个原生 async 往返，避免一次混入过多风险。

## 10. Audit links

- semantics: SEM-090—093、SEM-100—106、SEM-121
- cases: affine-resources、future-task、stream
- RFC: [`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md)
- report: [`resource-async-wit-v0`](../reports/resource-async-wit-v0.md)
- commit subject: `feat(async): [STEP-0009] validate resource and async mappings`
- next: STEP-0010 real Component host-call chain
