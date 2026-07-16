# 故障排查

## `sico` 找不到

确认已经构建，并使用绝对/相对路径：

```powershell
.\target\release\sico.exe --version
```

或把 `target/release` 加入当前会话 `PATH`。

## Rust 工具链不可用

安装仓库固定工具链：

```powershell
rustup toolchain install 1.97.0-x86_64-pc-windows-gnu --profile minimal --component clippy,rustfmt
```

检查：

```powershell
rustup toolchain list
rustc --version
```

## Runtime 找不到

错误通常发生在 `sico run`：

```powershell
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
Test-Path $env:SICO_WASMTIME
```

也可以传 `--runtime <wasmtime.exe>`。

## `source contract error InvalidUtf8`

源码必须是 UTF-8，无 BOM。不要保存为 ANSI、UTF-16 或带 BOM 的 UTF-8。裸 CR 和禁止控制字符也会被拒绝。

## `refusing to overwrite`

`sico build` 不覆盖已有产物。确认旧文件是否仍需保留，再手工移动/删除，或选择新的 `--output`。

## `entry function main() is missing`

可运行应用必须定义：

```sico
function main() returns Int:
  return 0
end function
```

## `Unsupported` 或 backend refusal

`sico check` 覆盖的语言比当前 codegen 更广。records、Result、function call、resource/async 等源码可能检查通过但不能构建。先缩小为同步 scalar `main()`，并查看[语言支持边界](./LANGUAGE-BASICS.md)。

## package 要求 `--trusted-key`

development-signed `.sapp` 不会自动被本机信任。传入匹配的 public key 文件：

```powershell
sico run --trusted-key trusted-public-key.hex app.sapp
```

unsigned development package 使用 `--allow-unsigned-dev`，但仅限可信本地构建。

## capability 被拒绝

用 `sico inspect --json` 查看 package 实际请求。`--grant` 只能授予已请求能力；storage 还需要 `--storage-root`。

## corrupt source cache

缓存验证失败时 Sico 会拒绝执行，不会自动覆盖。确认没有并发或磁盘损坏后，删除明确的 cache directory，或用 `--no-cache` 重跑。不要把删除 cache 当成绕过 package trust 的方式。

## Windows 错误 740

部分测试 executable 名称可能触发 Windows installer-name heuristic，在 Rust 测试代码运行前要求 elevation。项目验证脚本使用 `__COMPAT_LAYER=RunAsInvoker` 避免该启发式；它不会提升权限。普通用户不应以管理员身份运行不可信 `.sapp`。

## 获取机器可读错误

源码诊断使用：

```powershell
sico check --json app.sico
```

包信息使用：

```powershell
sico inspect --json app.sapp
```

自动化优先使用退出码和 schema 字段，不要依赖完整自然语言文本。
