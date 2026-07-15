# STEP-0051: Windows Desktop Host integration

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

交付 Windows-first Desktop Host，将 signed `.sapp` 的安装、打开、授权、运行、卸载与当前用户文件关联连成真实闭环。

## 2. Scope

包含 Windows CLI/OS adapter、原生权限窗口、typed UI 原生预览、per-user association plan/apply/remove、图标与 zip packaging smoke。不静默修改系统默认应用，不实现 production signing 或 updater。

## 3. Decision

注册表变更只能由显式 `association-apply` 触发，默认命令是只读 plan。打开流程始终重新验证 package digest、development trust、app identity、capability closure 与 permission record，再交给固定 Wasmtime adapter。卸载同时清除该 immutable app identity 的持久权限。

## 4. Changes

- 新增 `sico-desktop-host` binary 与 install/open/uninstall 命令；
- 新增 HKCU `.sapp` Open With 注册计划和显式 apply/remove；
- 新增 Windows Forms permission dialog 与 typed UI preview；
- 新增 Windows `.ico` 与可复现 zip packaging smoke；
- 新增真实 signed package -> Wasmtime `42` -> uninstall 集成测试；
- 修正注册表默认值使用 `/ve`，打开参数保持独立 quoted `%1`，不经过 shell。

## 5. Validation

```text
cargo clippy -p sico-desktop-host --all-targets -- -D warnings
SICO_TEST_WASMTIME=<wasmtime-46.0.1> cargo test -p sico-desktop-host
powershell -STA native Windows Forms load/dispose probe
tools/package-windows-host.ps1
STEP_0051_OK platform=windows host=sico-desktop-host commands=install,open,uninstall association=per-user-explicit dialog=windows-forms ui_preview=native icon=ico runtime=wasmtime-46.0.1 desktop_tests=3 host_tests=13 package=zip next=STEP-0052
```

## 6. Risks and follow-ups

Windows is runtime-verified. macOS/Linux registration artifacts and parity labels remain STEP-0052. The zip is a smoke artifact, not a signed production installer. Registry mutation was deliberately not performed by automated validation.

## 7. Audit links

- [`review report`](../reports/windows-desktop-host-v0.md)
- [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
- [`STEP-0050`](./STEP-0050-minimal-ui-wit-renderer.md)

