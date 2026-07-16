# 包、签名与信任

## 三个不同概念

1. **结构有效**：`.sapp` framing、hash、manifest 和 Component 通过验证。
2. **签名有效**：package 的 development signature 能由内含公钥验证。
3. **本机信任**：调用者显式提供相同公钥作为 trust input。

签名有效不自动产生本机信任。development signature 也不是 production publisher identity。

## unsigned development

不传 `--sign-key` 构建 unsigned `.sapp`：

```powershell
sico build -o demo.sapp app.sico
sico inspect demo.sapp
```

运行时必须显式选择本地开发策略：

```powershell
sico run --allow-unsigned-dev demo.sapp
```

不要把此开关用于来源不明的 package。

## 创建 development seed

seed 文件必须包含恰好 64 个 lowercase hex 字符，可带一个末尾换行。使用操作系统 CSPRNG 创建，不要复制仓库测试 seed：

```powershell
$bytes = New-Object byte[] 32
$rng = [Security.Cryptography.RandomNumberGenerator]::Create()
$rng.GetBytes($bytes)
$rng.Dispose()
$hex = -join ($bytes | ForEach-Object { $_.ToString('x2') })
[IO.File]::WriteAllText((Join-Path $PWD 'development-seed.hex'), "$hex`n", [Text.UTF8Encoding]::new($false))
```

限制该文件的访问权限，不要提交到 Git。

## 签名并提取公钥

```powershell
sico build --sign-key development-seed.hex -o signed-demo.sapp app.sico
$info = sico inspect --json signed-demo.sapp | ConvertFrom-Json
$publicKey = $info.trust.public_key
[IO.File]::WriteAllText((Join-Path $PWD 'trusted-public-key.hex'), "$publicKey`n", [Text.UTF8Encoding]::new($false))
```

未传 `--trusted-key` 时，检查结果应为 `development-valid-untrusted`。

## 以本地公钥信任运行

```powershell
sico inspect --trusted-key trusted-public-key.hex signed-demo.sapp
sico run --trusted-key trusted-public-key.hex signed-demo.sapp
```

公钥、app ID、version、package digest、capability closure 中任何不一致都会拒绝执行。

## 安全边界

- development seed 不得升级或复用为 production key；
- 仓库 fixture key 只能用于测试；
- 本地 `sico-ecosystem` policy/registry 不是公开服务；
- `inspect` 不执行 guest，但仍应把 package 当作不可信输入；
- 不要把 seed、token 或 credential 写入源码、package resource、命令日志或 AI prompt；
- 正式 publisher identity、key custody、revocation 和 public registry 当前仍是外部 gate。
