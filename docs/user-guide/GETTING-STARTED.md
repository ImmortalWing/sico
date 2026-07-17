# 五分钟入门

完成[安装与构建](./INSTALLATION.md)后，在空目录创建 UTF-8（无 BOM）的 `hello.sico`：

```sico
function main() returns Int:
  return 40 + 2
end function
```

以下命令假设 Sico SDK 已安装，或 `target/release` 已加入当前会话 `PATH`。

只想立即查看运行结果时执行：

```powershell
sico-app dev .\hello.sico
```

该命令依次调用独立的 `sico build`、内部 pack 和 Runtime，成功或失败后清理临时产物。需要检查中间产物时加 `--keep-artifacts`。仓库中的 `tools/sico-dev.ps1` 继续作为旧脚本兼容入口。

## 1. 检查和格式化

```powershell
sico check hello.sico
sico check --json hello.sico
sico format --check hello.sico
sico format --write hello.sico
```

## 2. 查看顶层结构

```powershell
sico outline hello.sico
sico outline --json hello.sico
```

## 3. 编译 Component

```powershell
sico build -o hello.component.wasm hello.sico
```

`sico build` 只编译语言源码，不创建签名、信任或 Runtime 状态。目标已存在时会拒绝覆盖。

## 4. 打包 `.sapp`

```powershell
sico-app pack `
  --app-id dev.example.hello `
  --app-version 0.0.2 `
  -o hello.sapp `
  hello.component.wasm
```

## 5. 检查包但不执行

```powershell
sico-app inspect hello.sapp
sico-app inspect --json hello.sapp
```

## 6. 显式运行本地开发包

```powershell
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
sico-app run --allow-unsigned-dev hello.sapp
```

预期 stdout：

```text
42
```

`--allow-unsigned-dev` 仅适合自己构建且来源可信的本地包。需要可重复的开发签名流程时，阅读[包、签名与信任](./PACKAGES-AND-TRUST.md)。

## 下一步

- [语言支持边界](./LANGUAGE-BASICS.md)
- [CLI 参数](./CLI.md)
- [编译、打包与运行](./BUILD-RUN.md)
