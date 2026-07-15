# STEP-0052: Desktop platform adapters and parity corpus

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

冻结 Windows、macOS、Linux Desktop Host adapter 的共同身份、打开参数和 trust ownership，并对当前不可运行的平台给出诚实的证据等级。

## 2. Decision

三个 adapter 都使用 `.sapp`、`application/vnd.sico.sapp`、package digest + app/signer identity，并把单一路径事件交回 `sico-host-core`。平台文件关联不是新的 trust root，也不能经 shell 展开 guest 路径。Windows 标记 runtime-verified；当前没有 macOS/Linux runner 或 target，因此两者只标记 contract-verified。

## 3. Changes

- 新增版本化 `PlatformContract` 与 runtime/contract evidence enum；
- 输出 Windows registry plan、macOS `Info.plist`、Linux desktop/shared-MIME/mimeapps artifacts；
- macOS 使用 Alternate handler，不声明强制默认；
- Linux 使用 `%f` 单文件参数且无 `sh -c`；
- 新增 3 个 parity/artifact tests，保持 shared trust core ownership。

## 4. Validation

```text
cargo clippy -p sico-desktop-host --all-targets -- -D warnings
SICO_TEST_WASMTIME=<wasmtime-46.0.1> cargo test -p sico-desktop-host
sico-desktop-host platform-artifacts --output <dir> --executable <host.exe>
STEP_0052_OK adapters=windows,macos,linux parity=extension,identity,open-argument windows=runtime-verified macos=contract-verified linux=contract-verified unavailable_runners=macos,linux platform_tests=3 next=STEP-0053
```

## 5. Risks and follow-ups

`contract-verified` 不等于 compile/runtime verified。M5 退出报告必须保留 macOS/Linux runner 缺口；后续拿到真实 runner 后，应安装生成的声明、触发真实文件打开并复跑同一 signed package corpus。

## 6. Audit links

- [`review report`](../reports/desktop-platform-parity-v0.md)
- [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
- [`STEP-0051`](./STEP-0051-windows-desktop-host-integration.md)

