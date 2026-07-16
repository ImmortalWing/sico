# 包、签名与信任

## 三个不同概念

1. **结构有效**：`.sapp` framing、hash、manifest 与 Component 通过验证；
2. **签名有效**：development signature 可由包内公钥验证；
3. **本机信任**：调用者显式提供相同公钥作为 trust input。

签名有效不会自动产生本机信任；development signature 也不是 production publisher identity。

## unsigned development

```powershell
sico build -o demo.component.wasm app.sico
sico-app pack -o demo.sapp demo.component.wasm
sico-app inspect demo.sapp
sico-app run --allow-unsigned-dev demo.sapp
```

不要对来源不明的包使用 `--allow-unsigned-dev`。

## 创建 development seed

seed 文件必须包含恰好 64 个 lowercase hex 字符，可带一个末尾换行。使用操作系统 CSPRNG 创建，不要复制仓库测试 seed：

```powershell
$bytes = New-Object byte[] 32
$rng = [Security.Cryptography.RandomNumberGenerator]::Create()
$rng.GetBytes($bytes)
$rng.Dispose()
$hex = -join ($bytes | ForEach-Object { $_.ToString('x2') })
[IO.File]::WriteAllText(
  (Join-Path $PWD 'development-seed.hex'),
  "$hex`n",
  [Text.UTF8Encoding]::new($false)
)
```

限制该文件权限，且不要提交到 Git。

## 签名并提取公钥

```powershell
sico build -o demo.component.wasm app.sico
sico-app pack `
  --sign-key development-seed.hex `
  -o signed-demo.sapp `
  demo.component.wasm
$info = sico-app inspect --json signed-demo.sapp | ConvertFrom-Json
$publicKey = $info.trust.public_key
[IO.File]::WriteAllText(
  (Join-Path $PWD 'trusted-public-key.hex'),
  "$publicKey`n",
  [Text.UTF8Encoding]::new($false)
)
```

未传 `--trusted-key` 时，检查结果应为 `development-valid-untrusted`。

## 以本地公钥信任运行

```powershell
sico-app inspect --trusted-key trusted-public-key.hex signed-demo.sapp
sico-app run --trusted-key trusted-public-key.hex signed-demo.sapp
```

公钥、app ID、version、package digest 或 capability closure 不一致时必须拒绝执行。

## 安全边界

- development seed 不得复用为 production key；
- fixture key 只能用于测试；
- `sico-app inspect` 不执行 guest，但仍把 package 当作不可信输入；
- seed、token 或 credential 不得进入源码、package resource、日志或 AI prompt；
- production publisher identity、key custody、revocation 与 public registry 仍受外部证据 gate 限制。
