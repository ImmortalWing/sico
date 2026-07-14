# STEP-0010: 验证 Component Runtime WIT host call 实链

> - status: complete
> - phase: M0
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

构建真实 Rust guest WebAssembly Component，由原生 Rust/Wasmtime Runtime 加载，完成 WIT import/export、resource owned/borrowed 生命周期、原生 async ABI、数值记录往返和确定性重跑。

## 2. Context and evidence

- STEP-0008/0009 已验证数值、resource 和 async 的语言原型与 WIT 草案，但没有真实 Canonical ABI；
- 工具链锁定为 Rust 1.97.0、`wit-bindgen 0.59.0`、`wit-component 0.253.0`、Wasmtime 46.0.1；
- 本机使用 Windows x86_64 GNU target、WinLibs 与 rust-lld；
- RFC-0003/0004 的接受条件要求根据本步骤的真实往返结果重新判断。

## 3. Scope

包含同步与 async Rust guest、版本化 WIT、Component 编码、Wasmtime import/export 调用、宿主 resource table、4096-bit `big-int`/Decimal 记录往返、二次执行和二进制哈希。

不包含正式 Sico codegen、`.sapp`、完整 WASI 平台、桌面/Android UI、Future/Stream 的真实 Runtime 往返或性能选型。

## 4. Options and decision

不依赖全局 `cargo-component` CLI。guest 用 `wit-bindgen` 生成 Core Wasm 和 Component metadata，仓库锁定的 `wit-component::ComponentEncoder` 编码 Component，再由 Wasmtime typed bindings 加载。这样版本、输入和验证逻辑都由 Cargo.lock 与仓库代码固定。

同步链验证普通 host call、resource 和数值记录；async 链单独启用 `component-model-async` 与 concurrent store。实测证明同步宿主实现不能冒充 async import，最终使用真正的 `HostWithStore` async lowering。

## 5. Plan

1. 固定 WIT、guest 与 host 依赖版本；
2. 完成同步 import/export 和 owned/borrowed resource 生命周期；
3. 增加 `big-int`/Decimal Canonical ABI 往返；
4. 增加原生 `async func` guest/host 往返；
5. 重复编码与执行，比较 JSON 与 Component SHA-256；
6. 更新报告、RFC 判断、状态并执行回归。

## 6. Changes

- 新增 `prototypes/component-host-call`；
- 同步 guest 调用 `log`、`host-add` 和宿主 `counter` resource；
- 宿主使用 `ResourceTable`，两次方法调用收到 borrowed handle，析构收到 owned handle；
- 增加 512 字节 magnitude 的 4096-bit `big-int` 和 Decimal identity roundtrip；
- 增加原生 async WIT、async guest export、async host import 和 Wasmtime concurrent store；
- 增加统一 PowerShell 验证器和三个独立 Cargo.lock。

## 7. Validation

执行：

```powershell
& .\prototypes\component-host-call\tools\validate.ps1
```

同步链两次输出 JSON 逐字节相同；async Component 两次编码哈希相同。最终结果：

- sync output `50`；
- resource 事件 `new → add(borrow) → value(borrow) → drop(owned)`；
- 4096-bit roundtrip `512` bytes；Decimal roundtrip `true`；
- async output `42`，async host calls `1`；
- sync Component SHA-256 `849a03066d38a599cfd680bcc3c692223e58ae595a592839698b7f527b2e4701`；
- async Component SHA-256 `e43809a7ce88bdad84e04ee01b413c7b828d641ddd90f57e6013d1dda4904c9f`。

## 8. Metrics

2 个真实 Component、2 个 Rust guest、2 个原生 host binary、6 个确定性 resource/host 事件、2 次同步执行、2 次 async 执行、0 个运行失败。

## 9. Risks and follow-ups

- RFC-0003 的真实记录往返已完成，但非 Windows 环境和稳定限额诊断分类仍未完成，因此保持 proposed；
- RFC-0004 的 resource 与原生 async 往返已完成，但 `future<T>`/`stream<T>` 尚未由 Runtime 往返，因此保持 proposed；
- Component 生成物不提交，只提交可复现输入、锁文件与验证器；
- STEP-0011 需比较桌面与 Android 的 Runtime 体积、JIT/AOT、沙箱和发布约束。

## 10. Audit links

- prototype: [`component-host-call`](../../prototypes/component-host-call/README.md)
- report: [`component-runtime-host-call-v0`](../reports/component-runtime-host-call-v0.md)
- RFC: [`RFC-0003`](../rfc/RFC-0003-numeric-representation-v0.md)、[`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md)
- commit subject: `feat(runtime): [STEP-0010] validate real component host calls`
- next: STEP-0011 desktop/Android Runtime feasibility
