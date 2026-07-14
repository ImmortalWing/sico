# Report: Component Runtime host-call chain v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0010
> - environment: Windows 11 x86_64; rustc/cargo 1.97.0; GNU target + WinLibs + rust-lld

## 1. Question

锁定版本的 Rust/WIT/Component/Wasmtime 工具链能否真实承载 Sico 计划中的 host call、affine resource、原生 async 和任意精度数值记录，而不依赖 JavaScript 或自研 ABI？

## 2. Method

同步 guest 由 `wit-bindgen 0.59.0` 生成绑定，调用宿主函数与 `counter` resource，并导出数值 identity 函数。async guest 使用 WIT `async func`。两个 Core Wasm 都由 `wit-component 0.253.0` 编码并验证，再由 Wasmtime 46.0.1 typed bindings 加载。统一脚本分别执行两次并比较结果或 SHA-256。

## 3. Reproduction

```powershell
& .\prototypes\component-host-call\tools\validate.ps1
```

## 4. Raw evidence

运行时生成：

- `prototypes/component-host-call/results/run-1.json`；
- `prototypes/component-host-call/results/run-2.json`；
- `prototypes/component-host-call/artifacts/*.component.wasm`。

生成物由 `.gitignore` 排除；锁定源码、WIT、Cargo.lock 和验证器提交仓库。

## 5. Results

| Evidence | Result | Status |
|---|---:|---|
| sync output | 50 | verified |
| resource lifecycle | new/add/value/drop | verified |
| borrowed method handles | 2 | verified |
| owned destructor handles | 1 | verified |
| 4096-bit roundtrip | 512 bytes unchanged | verified |
| Decimal roundtrip | unchanged | verified |
| native async output | 42 | verified |
| native async host calls | 1 | verified |
| repeated sync JSON | identical | verified |
| repeated Component hashes | identical | verified |

同步 Component SHA-256：`849a03066d38a599cfd680bcc3c692223e58ae595a592839698b7f527b2e4701`。

async Component SHA-256：`e43809a7ce88bdad84e04ee01b413c7b828d641ddd90f57e6013d1dda4904c9f`。

## 6. Interpretation

- verified：Rust guest → Component → Wasmtime → WIT host 的完整链可用，不需要 JavaScript；
- verified：WIT resource 和 Wasmtime `ResourceTable` 能保持 owned/borrowed 区分与恰好一次析构；
- verified：WASI 0.3/Component async ABI 在当前 guest 与 Runtime 版本中能真实往返；
- verified：RFC-0003 的记录形状能经过 Canonical ABI 保持 4096-bit payload 与 Decimal 字段；
- rejected：用同步 host lowering 实现 async import，Wasmtime 以 `type mismatch with async` 拒绝；
- proposed：Future/Stream Runtime API、跨平台重现和生产级资源限额策略。

## 7. Limitations

- 只在 Windows x86_64 GNU 环境运行；
- 未验证 Android、Linux、macOS、AOT 或受限设备；
- async 探针只有一次 host await 边界，未覆盖取消、Future/Stream pipe 和高并发；
- 数值边界只证明 ABI 保真，规范化与限额拒绝仍由 STEP-0008 原型证明；
- 没有 Sico 编译器或 Sico 源码参与 codegen。

## 8. Decision impact

确认 Rust + WebAssembly Component + Wasmtime 是 Sico Runtime 基线的可行组合。RFC-0003/0004 的部分关键风险已经关闭，但各自剩余接受条件仍存在，因此不提前改为 accepted。

## 9. Links

- [STEP-0010](../steps/STEP-0010-component-runtime-host-call.md)
- [prototype](../../prototypes/component-host-call/README.md)
- [RFC-0003](../rfc/RFC-0003-numeric-representation-v0.md)
- [RFC-0004](../rfc/RFC-0004-resource-async-mapping-v0.md)
