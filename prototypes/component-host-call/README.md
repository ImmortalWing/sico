# Component host-call probe

STEP-0010 的真实 Canonical ABI 探针：

- 同步 Rust guest 通过 WIT 导入宿主函数与 owned/borrowed 宿主资源；
- async Rust guest 通过原生 Component async ABI 调用 async 宿主导入；
- 两者均由 `wit-component` 编码为 WebAssembly Component，再由原生 Rust/Wasmtime 宿主加载执行。

运行：

```powershell
& .\tools\validate.ps1
```

可审计结果写入 `results/`，Component 二进制写入 `artifacts/`；二者都是可复现的生成物，不提交仓库。
