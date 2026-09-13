# Windows 手动发布

Sico 的 Windows Release 在已验证的本机环境中构建，不依赖 GitHub Actions。脚本只生成待上传文件，不创建标签、不推送、不调用 GitHub/GitCode API。

发布者只运行 `release-windows.ps1`。它负责完整编排，并调用 `tools/internal/` 中的打包器生成和验证 EXE；不要分别手工运行多个脚本拼装同一个 Release。脚本职责索引见 [`tools/README.md`](../../tools/README.md)。

## 发布边界

- 当前目标固定为 `x86_64-pc-windows-gnu`；
- Cargo workspace 版本是唯一版本来源；
- compiler ZIP 只包含 `sico.exe`；
- 独立的 `sico-init-*.exe` 是可双击、离线、自包含的用户级安装器；
- SDK ZIP 包含 `sico`、`sico-app`、LSP、AI tool、`sico-dev`、终端安装器和锁定的 Wasmtime；
- registry origin ZIP 单独包含只读 `sico-registry`、空 transport root、许可证和 operator README，不把服务端工具塞入普通用户安装；
- SDK 同时携带 Sico 与 Wasmtime 许可证；
- 当前二进制未做 Windows Authenticode 签名，Release Notes 和 manifest 会明确记录 `unsigned`；
- GitHub 与 GitCode 必须上传脚本同一次运行产生的完全相同文件。

## 1. 冻结版本

脚本要求参数版本与根 `Cargo.toml` 的 `[workspace.package].version` 完全一致。当前不传参数时自动使用 workspace 版本：

```powershell
.\tools\release-windows.ps1
```

若要发布 `v0.1.0.1`，应先把 workspace 版本改为 `0.1.0.1`，更新 `Cargo.lock`、完成测试并提交；不能只给发布脚本传一个不同版本。

## 2. 生成 Release

确认工作区已经提交且干净：

```powershell
git status --short
.\tools\release-windows.ps1
```

默认门禁包括：

1. `cargo fmt --check`；
2. workspace 全 target、全 feature Clippy，warning 视为错误；
3. workspace 全 target、全 feature tests；
4. 模块依赖边界验证；
5. release binary 构建；
6. 源码 `1 + 2` → Component → `.sapp` → Wasmtime，结果必须为 `3`；
7. SDK ZIP 解压后再次运行同一源码，结果必须为 `3`；
8. 生成并实际运行 `sico-init` EXE，验证安装、版本、PATH 与卸载；
9. 生成并自检 registry origin operator bundle；
10. 生成 SHA-256、SPDX SBOM、机器 manifest 和发布说明。

默认输出目录：

```text
dist/v<VERSION>/
```

只有以下条件同时满足时，`release-manifest.json` 才会写入：

```json
"publishable": true
```

- 工作区干净；
- 未使用 `-SkipQualityGates`；
- 所有构建和烟雾测试通过。

`-AllowDirty` 与 `-SkipQualityGates` 只用于调试脚本。两者任一出现都会生成 `DO-NOT-PUBLISH.txt`，这种目录不得上传。

无法通过仓库缓存取得 Runtime 时，可以显式指定已经审计的同版本文件：

```powershell
.\tools\release-windows.ps1 -RuntimePath C:\verified\wasmtime.exe
```

该文件旁边必须存在上游 `LICENSE`，版本必须是 `46.0.1`。

## 3. 审核产物

```powershell
$release = '.\dist\v0.1.0'
Get-Content "$release\release-manifest.json"
Get-Content "$release\SHA256SUMS"
Get-ChildItem $release
```

重新计算校验值：

```powershell
Get-Content "$release\SHA256SUMS" | ForEach-Object {
    $expected, $name = $_ -split '\s+', 2
    $actual = (Get-FileHash "$release\$name" -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -cne $expected) { throw "checksum mismatch: $name" }
}
```

上传前必须确认：

- 没有 `DO-NOT-PUBLISH.txt`；
- manifest 的 `publishable` 为 `true`；
- manifest commit 等于 `git rev-parse HEAD`；
- EXE、ZIP、SBOM、manifest、Release Notes 与 `SHA256SUMS` 全部来自同一输出目录。

## 4. 创建并推送标签

产物审核通过后，对 manifest 中记录的当前 commit 创建 annotated tag：

```powershell
git tag -a v0.1.0 -m "Sico v0.1.0"
git push github v0.1.0
git push origin v0.1.0
```

`github` 是 GitHub 远端，`origin` 是 GitCode 远端。两个远端的标签必须指向同一 commit。预发布标签不得移动；发现问题时发布更高版本。

## 5. 手动创建 Release

GitHub API 可用时，不需要 Actions：

```powershell
gh release create v0.1.0 `
  .\dist\v0.1.0\sico-compiler-*.zip `
  .\dist\v0.1.0\sico-sdk-*.zip `
  .\dist\v0.1.0\sico-init-*.exe `
  .\dist\v0.1.0\sico-registry-origin-*.zip `
  .\dist\v0.1.0\registry-origin-manifest.json `
  .\dist\v0.1.0\SHA256SUMS `
  .\dist\v0.1.0\SBOM.spdx.json `
  .\dist\v0.1.0\release-manifest.json `
  --repo ImmortalWing/sico `
  --verify-tag `
  --prerelease `
  --notes-file .\dist\v0.1.0\RELEASE-NOTES.md
```

GitHub API 不可用时，在网页创建 draft Release，选择已经推送的标签，上传 `SHA256SUMS` 列出的全部附件并粘贴 `RELEASE-NOTES.md`；检查完毕再发布。

GitCode 使用同一个标签创建预发布 Release，上传完全相同的六个附件并使用同一份说明。不要在另一个网络环境重新构建 EXE 或 ZIP。

## 6. 上传后复核

分别从 GitHub 与 GitCode 下载 `sico-init` EXE、compiler ZIP、SDK ZIP、SBOM 和 manifest，对照 `SHA256SUMS`。只有两个下载源的摘要都与本地输出一致，镜像发布才闭环。

GitHub 支持时可以再启用 immutable release；启用后标签和附件不能替换。任何修复都应提高版本号并创建新 Release。
