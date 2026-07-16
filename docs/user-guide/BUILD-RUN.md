# 构建、运行与缓存

## 正式用户产物

```text
source.sico → syntax/semantics → typed Sico IR → WebAssembly Component → .sapp
```

`.sapp` 是用户应用分发格式，包含 canonical manifest、Component、资源、hash 和可选 development signature。`--raw-component` 只保留给编译器回归，不携带应用级 trust/capability contract。

## 从源码直接运行

```powershell
sico run --app-id dev.example.demo --app-version 0.0.1 app.sico
```

流程会编译、构建本地 development package、验证 package 和 capability closure，再交给 Wasmtime。源文件不是因为位于本地就跳过语法、语义或 package verification。

## 源码缓存

源码运行默认使用 domain-separated SHA-256 key。缓存位置由以下规则选择：

1. `--cache-dir <DIR>`；
2. `SICO_CACHE_DIR`；
3. OS 临时目录下的 Sico cache。

禁用缓存：

```powershell
sico run --no-cache app.sico
```

cache hit 会重新执行 strict verification 和 app identity/version 比对。损坏或 stale cache 会被拒绝，不会自动覆盖后继续执行。`.sapp` 输入不进入源码缓存。

## 构建 package

```powershell
sico build --app-id dev.example.demo --app-version 0.0.1 -o demo.sapp app.sico
```

同一输入和配置产生 deterministic package bytes。构建失败或目标已存在时不会留下半成品，也不会覆盖旧文件。

## 运行 package

unsigned development package：

```powershell
sico run --allow-unsigned-dev demo.sapp
```

development-signed package：

```powershell
sico run --trusted-key trusted-public-key.hex demo.sapp
```

详细签名流程见[包、签名与信任](./PACKAGES-AND-TRUST.md)。

## Runtime stdout/stderr

- stdout 只转发 guest stdout/result；
- guest stderr 先转发到 stderr，Host/tool error 也使用 stderr；
- cache hit/miss 不污染 guest channel；
- Runtime stdin 当前关闭；
- 非空应用参数尚不支持。

## 能力授权

Host grant 不能创造 guest 没有请求的能力：

```powershell
sico run --grant storage.read-write --storage-root .\app-storage demo.sapp
```

具体 capability 名称必须来自 package 的已验证声明/import closure。storage grant 缺少 `--storage-root` 会被拒绝。不要为了让程序运行而盲目添加 grant；先用 `sico inspect --json` 查看请求。

## Runtime 限额

manifest 限额只能收紧 Host ceiling，不能扩大权限。timeout、资源超限和 trap 使用稳定退出码。当前同步 scalar Runtime 是已验证路径；一般 imports、完整 async transport 和源级调试仍受限制。
