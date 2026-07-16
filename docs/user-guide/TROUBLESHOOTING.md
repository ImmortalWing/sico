# 故障排查

## 找不到 `sico` 或 `sico-app`

```powershell
cargo build --locked --release -p sico-cli -p sico-app-cli
.\target\release\sico.exe --version
.\target\release\sico-app.exe --version
```

或把 `target/release` 加入当前会话 `PATH`。

## Runtime 找不到

错误发生在 `sico-app run`：

```powershell
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
Test-Path $env:SICO_WASMTIME
```

也可以传 `--runtime <wasmtime.exe>`。

## `source contract error InvalidUtf8`

源码必须是 UTF-8（无 BOM），不能是 ANSI、UTF-16 或带 BOM 的 UTF-8。裸 CR 和禁止控制字符也会被拒绝。

## `refusing to overwrite`

`sico build` 和 `sico-app pack` 都拒绝覆盖已有产物。确认旧文件是否需要保留，再手工移动、删除或选择新输出路径。

## `entry function main() is missing`

当前可运行子集要求：

```sico
function main() returns Int:
  return 0
end function
```

## backend refusal

`sico check` 覆盖的语言范围比当前 codegen 更广。records、Result、部分调用、resource/async 源码可能检查通过但无法构建。参见[语言支持边界](./LANGUAGE-BASICS.md)。

## package 要求 `--trusted-key`

```powershell
sico-app run --trusted-key trusted-public-key.hex app.sapp
```

自己构建的 unsigned development 包可显式使用 `--allow-unsigned-dev`。

## capability 被拒绝

```powershell
sico-app inspect --json app.sapp
```

`--grant` 只能授予已经请求的能力；storage 还需要 `--storage-root`。

## 旧文档中的 `sico run`、源码缓存或 `--raw-component`

这些属于 `v0.0.1` 的一体化开发流程。当前 `main` 使用 `sico build`、`sico-app pack`、`sico-app run`，且不提供隐式源码运行缓存。需要复现旧行为时检出 `v0.0.1` 或归档分支。

## Windows 错误 740

部分测试可执行文件名称可能触发 Windows installer-name heuristic。仓库验证脚本在需要时使用 `__COMPAT_LAYER=RunAsInvoker`；它不会提升权限。不要以管理员身份运行不可信 `.sapp`。

## 获取机器可读输出

```powershell
sico check --json app.sico
sico-app inspect --json app.sapp
```

自动化应使用退出码和 schema 字段，不应解析完整自然语言文本。
