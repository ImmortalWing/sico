# 五分钟入门

完成[安装与构建](./INSTALLATION.md)后，在空目录创建 `hello.sico`，保存为 UTF-8（无 BOM）：

```sico
function main() returns Int:
  return 40 + 2
end function
```

以下命令假设 `target/release` 已加入当前会话的 `PATH`。

## 1. 检查源码

```powershell
sico check hello.sico
```

成功时退出码为 `0`。需要机器可读诊断时：

```powershell
sico check --json hello.sico
```

## 2. 检查并应用规范格式

```powershell
sico format --check hello.sico
sico format --write hello.sico
```

不带 `--check` 或 `--write` 时，规范格式写到 stdout，不修改文件。

## 3. 查看顶层结构

```powershell
sico outline hello.sico
sico outline --json hello.sico
```

## 4. 直接运行源码

先设置 Runtime：

```powershell
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
sico run hello.sico
```

预期 stdout：

```text
42
```

源码运行是用户主动发起的本地开发流程，会构建并验证缓存。可用 `--no-cache` 禁用缓存。

## 5. 构建 `.sapp`

```powershell
sico build --app-id dev.example.hello --app-version 0.0.1 hello.sico
```

默认生成 `hello.sapp`。已有目标文件时构建会拒绝覆盖；请先确认并移动或删除旧产物。

## 6. 检查包但不执行

```powershell
sico inspect hello.sapp
sico inspect --json hello.sapp
```

`inspect` 会验证 framing、hash、Component、manifest 和已有签名，但不会运行 guest。

## 7. 显式运行 unsigned development 包

```powershell
sico run --allow-unsigned-dev hello.sapp
```

`--allow-unsigned-dev` 只适合本地开发。需要可重复的开发签名和本地信任流程时，阅读[包、签名与信任](./PACKAGES-AND-TRUST.md)。

## 下一步

- 了解哪些语法能检查、哪些能真正 codegen：[语言基础与可运行子集](./LANGUAGE-BASICS.md)
- 查看完整命令参数：[CLI 参考](./CLI.md)
- 了解 Runtime、缓存和能力授权：[构建、运行与缓存](./BUILD-RUN.md)
