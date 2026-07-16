# Sico Linux Desktop Host 详细开发手册

> 状态：`contract-verified / not-compile-verified / not-runtime-verified`
> 适用范围：Linux Desktop Host 恢复、实现与验收
> 最后复核：2026-07-16

本手册用于把现有跨平台 Desktop Host 契约补齐为可在 Linux 上构建、安装、打开 `.sapp`、授权、显示原生 UI 并运行 Component 的实现。它不是 Linux 已支持的声明。

## 1. 当前真实状态

仓库已经具备：

- 跨平台 `sico-host-core` 的安装、签名校验、身份、权限记录、生命周期与 UI schema；
- `sico-desktop-host` 的 Rust CLI、严格 package open 和 Wasmtime 子进程执行路径；
- Linux `.desktop`、Shared MIME XML、`mimeapps.list` 字符串生成器；
- `.sapp`、`application/vnd.sico.sapp`、单文件参数、无 shell、共享 trust owner 的 contract tests；
- Windows Runtime 证据与 Desktop 平台证据分级规则。

仓库仍缺少：

- Linux runner、Linux 原生 build/Clippy/test、glibc 兼容与依赖证据；
- 可在 Linux 上应用/撤销文件关联的实现；
- Linux 原生权限对话框和最小 UI renderer；
- GNOME/KDE、Wayland/X11、Orca/AT-SPI 与输入法测试；
- Linux 单实例 IPC、文件管理器真实打开、进程组监管和 Runtime 运行证据；
- deb/rpm/tar/Flatpak/AppImage 中任一种经过验收的发行产物。

当前代码还有两个具体断点：

1. `platform-artifacts` 会先构造 Windows association plan，并要求传入可 canonicalize 的 `.exe`；Linux 二进制不能作为该参数，因此生成器必须先拆成按平台入口。
2. 生成的 `Exec=sico-desktop-host open %f` 只传一个包路径，但当前 `open` CLI 还强制要求 `--store`、`--trusted-key` 与 `--runtime`；在实现 XDG 默认路径和可信配置解析前，真实文件管理器打开必然失败。

非 Windows 分支的 `native_permission_dialog` 与 `show_native_ui_preview` 当前也明确返回 unavailable。现状只能标记 `contract-verified`。

## 2. 权威契约

实现前依次阅读：

- [ADR-0004 Desktop identity/lifecycle/platform boundary](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)；
- [RFC-0020 Desktop UI/permission contract](../rfc/RFC-0020-desktop-ui-permission-contract-v0.md)；
- [M5 Desktop Host 计划](../plans/M5-desktop-host.md)；
- [STEP-0052 Desktop platform parity](../steps/STEP-0052-desktop-platform-adapters-parity.md)；
- [Desktop platform review](../reports/desktop-platform-parity-v0.md)；
- [M5 exit audit](../reports/m5-exit-audit.md)。

以下边界不得由 Linux adapter 改写：

- `.sapp` 外部路径只用于复制/安装，实际执行使用私有 store 中重新校验的 bytes；
- `AppIdentityKey` 同时绑定 app id 与 signer trust identity；
- revision 使用精确 canonical `.sapp` bytes 的 SHA-256；
- 文件关联不是 trust root，MIME/扩展名也不是 package 验证；
- guest path、文件名、UI 文本和事件永不进入 `sh -c`、`bash -c` 或字符串拼接命令；
- 一个 app identity 同时最多一个受监管 guest，后续 open 进入有界队列；
- permission 是 package capability、用户决策与 Host/platform 可用性的交集；
- Linux 证据必须独立，Windows 结果不能转移。

## 3. 首版支持矩阵与待冻结决策

建议 v0 先冻结以下产品面，但它们在新 ADR 接受前仍是 proposed：

| 项目 | 首版建议 | 验收要求 |
|---|---|---|
| Rust target | `x86_64-unknown-linux-gnu` | 首要发行目标、真实桌面 runner |
| 第二架构 | `aarch64-unknown-linux-gnu` | 原生 arm64 runner；不只 cross-check |
| libc | glibc | 在最老受支持 glibc 环境构建并向新环境测试 |
| 桌面 | GNOME + KDE Plasma | 都验证文件关联、picker、窗口、主题 |
| 显示协议 | Wayland 主路径，X11 fallback | 分别记录 backend，不能互相替代 |
| UI toolkit | GTK 4 adapter | 先证明原生 permission 与最小 UI schema |
| Runtime | 当前显式 Wasmtime executable，后续可评估嵌入 | backend/version/path 必须入证据 |
| 安装范围 | 无 root 的 per-user tar 安装优先 | system package 另行验收 |
| 沙箱包 | 后置 Flatpak profile | portal、Runtime、文件权限必须重新建模 |

必须单独决定：支持的发行版与版本、glibc 最低版本、是否发布 aarch64、GTK 最低版本、是否允许 X11、包格式、自动更新责任、production signing 和 Flatpak/商店范围。不要用“Linux”一个标签掩盖这些差异。

Rust 的 GNU Linux targets 与支持层级以 [rustc platform support](https://doc.rust-lang.org/rustc/platform-support.html) 为准。Wasmtime 支持也应在恢复当天复查 [platform support](https://docs.wasmtime.dev/stability-platform-support.html) 和 [stability tiers](https://docs.wasmtime.dev/stability-tiers.html)。

## 4. 开发环境审计

Linux runner 至少需要：

- Rust stable、Cargo、rustfmt、Clippy；
- GCC 或 Clang、GNU binutils、pkg-config；
- GTK 4 development files（实现 GUI adapter 后）；
- `shared-mime-info`、`desktop-file-utils`、`xdg-utils`；
- Wayland session、X11 session 或明确可切换的测试环境；
- xdg-desktop-portal 及与桌面匹配的 backend；
- Wasmtime 受审计版本；
- Orca/AT-SPI、至少一种复杂文本输入法；
- x86_64 实机/VM；支持 aarch64 时提供 arm64 runner。

环境快照命令：

```bash
uname -a
cat /etc/os-release
uname -m
getconf GNU_LIBC_VERSION || true
rustc --version --verbose
cargo --version
rustup target list --installed
cc --version
pkg-config --modversion gtk4 || true
desktop-file-validate --version || true
update-mime-database -v 2>&1 | head -n 1 || true
xdg-mime --version || true
echo "desktop=$XDG_CURRENT_DESKTOP session=$XDG_SESSION_TYPE"
busctl --user --no-pager status org.freedesktop.portal.Desktop || true
```

把完整非敏感输出写入 evidence；删除用户名、主机名、HOME、设备 ID 和 session token。不要在脚本中自动安装系统包或接受发行版/商店条款。

## 5. 目标工程布局

建议在保持共享 core 不变的前提下增加窄平台层：

```text
crates/sico-desktop-host/
  src/main.rs
  src/lib.rs
  src/platform.rs
  src/linux/
    mod.rs
    paths.rs
    association.rs
    ingress.rs
    permissions.rs
    lifecycle.rs
    ui_gtk.rs
    portal.rs
    process.rs
linux/
  packaging/
    sico.desktop.in
    sico-sapp.xml
    icons/hicolor/...
    install-user.sh
    uninstall-user.sh
  flatpak/                 # 后置，独立 profile
tools/build-linux-host.sh
tools/test-linux-host.sh
tests/platform/evidence/linux/<date>/<run-id>/
```

平台模块只翻译 XDG、桌面入口、窗口、portal、POSIX process/filesystem。package parsing、trust、permission schema、UI validation 和 Runtime limits 继续由共享 crates 拥有。当前 `sico-desktop-host` 使用 `#![forbid(unsafe_code)]`，应优先用安全封装保持该边界；若某个 syscall 确实无法安全表达，另建最窄的 `sico-linux-sys` crate，而不是放宽整个 Host。每个 unsafe 块必须说明 fd、ownership、thread 与 signal invariants。

## 6. 构建与 ABI 基线

原生 x86_64 构建：

```bash
cargo fmt --all -- --check
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
cargo test --offline --locked --workspace --all-targets --all-features
cargo build --offline --locked --release -p sico-desktop-host
```

aarch64 target：

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --offline --locked --release -p sico-desktop-host \
  --target aarch64-unknown-linux-gnu
```

cross build 还需要与目标 glibc 匹配的 sysroot/linker；如果没有经过审计的 sysroot，应使用原生 aarch64 CI，而不是让 host linker 兜底。`cargo check --target` 不能替代链接与运行。

产物检查：

```bash
file target/release/sico-desktop-host
readelf -h -d -W target/release/sico-desktop-host
readelf --version-info -W target/release/sico-desktop-host
ldd target/release/sico-desktop-host
objdump -T target/release/sico-desktop-host | sort -u > dynamic-symbols.txt
sha256sum target/release/sico-desktop-host
```

不要在新发行版上构建后假定旧 glibc 可运行；应在最老支持环境构建或使用冻结 sysroot，并在每个支持环境运行。不得 bundle glibc。若 bundle GTK/其他 `.so`，必须审计 license、RPATH/RUNPATH、`$ORIGIN`、加载顺序和篡改风险；首版优先使用发行版 GTK 依赖并明确版本。

## 7. XDG 目录与文件权限

按 [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/) 映射：

| 内容 | 环境变量 | 默认位置 | Sico 子目录 |
|---|---|---|---|
| 安装包、permission records、应用数据 | `XDG_DATA_HOME` | `~/.local/share` | `sico/` |
| 配置、可信公钥引用、Runtime 配置 | `XDG_CONFIG_HOME` | `~/.config` | `sico/` |
| 日志、崩溃/恢复状态 | `XDG_STATE_HOME` | `~/.local/state` | `sico/` |
| 可重建 cache | `XDG_CACHE_HOME` | `~/.cache` | `sico/` |
| socket、lock、短期 staging | `XDG_RUNTIME_DIR` | 无持久 fallback | `sico/` |

`XDG_RUNTIME_DIR` 必须归当前 UID 所有且 mode 0700；不满足时单实例 IPC fail closed。不要把 package store 或大文件放在 runtime dir。若变量为空，data/config/state/cache 使用规范默认；runtime fallback 必须是本地、0700、不可共享并显式警告，安全条件无法证明时禁用 IPC 而不是退到 `/tmp/sico.sock`。

创建规则：

- 目录默认 0700，文件默认 0600，进程初始 `umask 077`；
- 拒绝非当前 UID 所有或 group/world-writable 的敏感父目录；
- 写入使用同目录 create-new 临时文件、fsync（按 durable policy）和 atomic rename；
- 打开/清理时防 symlink、hardlink、mount 和 rename races；
- 不递归 chmod 用户已有 XDG 根目录；
- app storage 继续使用 `AppIdentityKey` 隔离，不使用 display name/path。

## 8. CLI 默认值与配置解析

要让 `.desktop` 的 `open %f` 可工作，先把当前开发型必填参数改为受审计默认值：

```text
--store       -> $XDG_DATA_HOME/sico/host
--trusted-key -> $XDG_CONFIG_HOME/sico/trust/development-public-key.hex
--runtime     -> signed/bundled runtime manifest or audited configured absolute path
```

规则：

- CLI 显式参数只用于开发/测试，生产关联入口使用配置 resolver；
- runtime 必须是绝对路径、regular executable、受信目录中当前 UID/root 所有，且不能 group/world-writable；
- trust config 只存公钥/策略引用，不存生产私钥；
- 环境变量 override 必须逐项登记，privileged/system 安装中默认忽略危险 override；
- 缺配置时显示明确错误/设置页，不能搜索当前目录或任意 `$PATH`；
- `open` 解析结果写入脱敏 audit log，但不输出 package 内容或公钥以外的密钥材料。

完成后为无参数 file-open 增加集成测试，确保带空格、换行、非 ASCII、前导 `-` 和 `%` 的文件名仍是一个 `OsString` 参数。

## 9. `.desktop`、MIME 与关联安装

现有 v0 声明：

```ini
[Desktop Entry]
Type=Application
Name=Sico Desktop Host
Exec=sico-desktop-host open %f
Icon=sico-desktop-host
Terminal=false
NoDisplay=true
MimeType=application/vnd.sico.sapp;
```

`%f` 是单个本地文件参数；desktop 实现必须把包含空格的路径仍作为一个参数，不经 shell。规范也允许桌面环境先把非本地资源复制为本地临时文件再传 `%f`，所以 Host 仍必须把输入视为不可信并 copy-before-verify。详见 [Desktop Entry Exec](https://specifications.freedesktop.org/desktop-entry/latest-single/)。

先重构生成器：

- `write_linux_artifacts(output, executable)` 只接受 Linux executable，不依赖 `.exe`；
- 模板根据安装前缀生成经过 Desktop Entry quoting 的 `Exec`，或使用固定安全 launcher；
- desktop file id、`mimeapps.list` 引用和实际文件名必须完全一致；
- 生成后用 `desktop-file-validate`；
- XML 用 schema-aware parser，并保持 MIME `application/vnd.sico.sapp`、glob `*.sapp`；
- icon 安装到 hicolor 主题的多个标准尺寸，文件名一致。

无 root 的安装目标：

```text
~/.local/bin/sico-desktop-host
$XDG_DATA_HOME/applications/sico.desktop
$XDG_DATA_HOME/mime/packages/sico-sapp.xml
$XDG_DATA_HOME/icons/hicolor/<size>/apps/sico-desktop-host.png
```

应用 MIME XML 后必须执行：

```bash
update-mime-database "$XDG_DATA_HOME/mime"
update-desktop-database "$XDG_DATA_HOME/applications"
desktop-file-validate "$XDG_DATA_HOME/applications/sico.desktop"
xdg-mime query filetype /absolute/path/to/example.sapp
```

[Shared MIME-info specification](https://specifications.freedesktop.org/shared-mime-info/latest-single/) 要求修改 package XML 后更新 MIME database。[MIME Apps specification](https://specifications.freedesktop.org/mime-apps/latest/) 区分 added association 与 default。安装器只添加 Open With 关联，不自动写 `[Default Applications]`；用户明确选择后才可执行：

```bash
xdg-mime default sico.desktop application/vnd.sico.sapp
```

卸载必须先移除 owned desktop/XML/icon，再更新两个 cache；默认保留用户安装包、permission 和 app data，并提供显式 `--purge-data`。绝不能删除整个 `$XDG_DATA_HOME` 或重写用户的完整 `mimeapps.list`。

## 10. 文件入口与 TOCTOU

文件管理器、命令行与 portal 最终都进入一个入口：

```text
one OsString/path or portal FD
  -> reject missing/multiple/invalid form
  -> open without shell and stream-copy to private staging with size guard
  -> close external FD/path dependency
  -> strict .sapp parse/signature/capability verification
  -> atomic install under identity/revision
  -> reopen installed bytes and verify digest again
  -> permission resolution -> Runtime
```

扩展名、MIME、desktop entry、xattr、文件所有者和文件管理器均不可信。外部文件可以来自 FUSE、NFS、removable media、portal document mount 或攻击者可修改目录；不能 verify 后再次执行原路径。

输入处理需覆盖：symlink、hardlink、FIFO、device、socket、directory、sparse file、procfs/sysfs、FUSE read error、rename/truncate race、超限文件和权限撤销。首版只接受 regular readable file/已打开的只读 FD；复制过程中任何变化或 I/O error 都删除 staging 并 fail closed。

## 11. 权限对话框

当前 `--permission deny|allow-once|allow-persistent` 仅适合自动测试与显式开发操作。真实 file-open 必须提供 Linux 原生 prompt，展示共享 `PermissionPrompt` 中已验证的：

- app id/version；
- signer fingerprint；
- revision/package digest；
- exact sorted capability names；
- deny、allow once、allow persistent 的作用域。

GTK 4 adapter 只把选择回传共享 `PermissionStore`，不自行写 grant。关闭窗口、portal cancel、后台/进程结束都按 deny 处理；allow-once 只存在内存 session；persistent record 保持共享 schema/identity/fingerprint。未知 capability 或 UI 显示失败不能降级成 allow。

权限窗口需要键盘导航、默认焦点、Escape/关闭拒绝、屏幕阅读器 label、长文本滚动和 bidi/控制字符防护。敏感权限不要用桌面通知代替模态决策。

## 12. GTK 4 最小 UI adapter

建议首版映射：

| Sico UI node | GTK 4 | 约束 |
|---|---|---|
| `window` | `GtkApplicationWindow` | 仅一个活动 window |
| `column` | vertical `GtkBox` | depth/children 有界 |
| `row` | horizontal `GtkBox` | depth/children 有界 |
| `text/status` | `GtkLabel` | 文本，不解析 markup |
| `button` | `GtkButton` | stable node id/event |
| `input` | `GtkEntry` | UTF-8 4 KiB、IME composition 有界 |

GTK 主循环只在 UI thread 更新 widget；Runtime/IPC 结果通过有界 channel 调度。guest 文本必须使用 plain-text setter，禁止 `use_markup`、Pango markup、URI action 或动态 class/widget name。输入事件继续服从 256 queue、120 event/s 和 4 KiB payload 限制。

GTK 标准控件实现 `GtkAccessible` 并在 Linux 使用 AT-SPI，但应用仍要补充 label/relation/state，参考 [GTK accessibility](https://docs.gtk.org/gtk4/section-accessibility.html)。设备验收使用 Orca 与键盘，覆盖焦点顺序、button/entry role、错误状态、窗口标题、字体放大、高对比度与动态状态读出。

UI toolkit 引入新的 native dependency 与 event-loop ownership。正式实现前新增 ADR，冻结 GTK minimum version、Rust binding、线程模型、binary distribution 和退出/Runtime cancel 协调。

## 13. Wayland、X11 与 portals

GTK 可以通过 GDK 在 Wayland/X11 后端运行；测试中记录 `GDK_BACKEND`、`XDG_SESSION_TYPE` 和桌面，不要靠设置 `GDK_BACKEND=x11` 掩盖 Wayland bug。Wayland 下不能依赖全局窗口坐标、任意激活或 X11-only handle。

Host 非沙箱发行版可用 `GtkFileDialog`/适当 GTK native chooser；Flatpak 或其他受限 profile 优先使用 XDG Desktop Portal。官方 [FileChooser portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html) 默认单选并返回获授权 URI；[OpenURI portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.OpenURI.html) 对本地文件使用 FD 型 `OpenFile`。

Portal 请求是异步的：先订阅预期 Response handle，再发请求；处理 success/cancel/error、request close、Ability/window destroy 类似生命周期取消。使用 toolkit 高层 API 时仍要记录最终 portal backend/version。URI/portal path/FD 只用于复制，不进入 guest。

Flatpak profile 不得直接复用 unsandboxed 假设：document portal 路径、session bus filter、Runtime executable、guest subprocess、动态库和 app storage 都需单独测试。Portal 是桌面资源访问适配器，不替代 Sico capability/trust boundary。

## 14. 单实例与生命周期

推荐每个用户一个 Host coordinator，在 `$XDG_RUNTIME_DIR/sico/host.sock` 接收单文件 open event，再由共享 lifecycle 按 app identity 决定启动或排队。实现必须：

- 创建 socket 前验证 runtime dir owner/mode/local filesystem；
- socket 目录 0700，不允许 symlink；
- 验证对端 UID（Linux 可用 `SO_PEERCRED`），拒绝其他用户；
- length-prefix message、64 KiB 上限、schema version、request id；
- 队列有界，重复/终止 race 只有一个 winner；
- stale socket 只在确认无活 peer 且 owner 正确后删除；
- coordinator 崩溃后从 immutable descriptor 重开，不序列化活 Runtime；
- logout/runtime dir 删除后安全重建。

如果暂不实现 coordinator，desktop environment 每次启动一个进程也必须通过锁与共享 lifecycle 防双 Runtime；不能用锁失败后无声退出丢失 open event。

## 15. guest 进程监管

当前 Host 以显式 Wasmtime executable 运行已授权 Component。Linux 实现需补齐：

- 直接 `execve`/Rust `Command` 参数数组，不经过 shell；
- 清理不必要环境变量，固定 locale/log policy，不传入 secret；
- stdin/stdout/stderr 有界，关闭无关 inherited FD；
- guest 进入独立 process group/session；
- graceful cancel 发送受控信号，deadline 后杀整个 process group；
- 正确处理 SIGCHLD、PID reuse、Host crash 与 orphan；可用时评估 pidfd；
- timeout、trap、memory、cancel 映射共享 fault taxonomy；
- temporary Component 与 storage handle 在所有 terminal path 清理。

seccomp、Landlock、namespaces、cgroups/systemd scope 可作为 defense-in-depth 研究，但在独立 threat model、内核/发行版矩阵和真实测试前不能宣称为 Sico sandbox。核心安全边界仍是 Component/WASI capability 与 Host limits。

## 16. Runtime 探针

按逐级可停止方式验证：

1. Linux Host clean build 与 `--help`；
2. strict signed install/open，不启动 Runtime；
3. Wasmtime version/hash/path 验证；
4. 同一 M5 `.sapp` 执行得到 `42`；
5. bad signature、tamper、capability mismatch 均在 Runtime 前拒绝；
6. infinite loop、memory、trap、cancel 后 Host 存活；
7. storage app identity 隔离与 symlink/mount race；
8. file manager open、permission prompt、UI event、关闭窗口协调；
9. x86_64 与 aarch64 分别记录；
10. GNOME/KDE、Wayland/X11 结果不静默合并。

如果 Runtime binary 缺失或不可信，Host 显示结构化错误，不能从网络自动下载，也不能搜索当前目录。backend/fault/limits 进入 evidence。

## 17. 打包与安装策略

建议顺序：

1. 可审计 `tar.zst`/`tar.xz` per-user bundle，包含 binary、desktop、MIME、icons、license、install/uninstall scripts；
2. 在目标发行版构建原生 deb/rpm，依赖由发行版表达；
3. AppImage 只作为便携打包候选，不把它描述为沙箱；
4. Flatpak 作为独立安全/portal profile，不能仅套 manifest；
5. 发行版/商店提交需所有者授权和相应 policy review。

安装脚本必须支持 `--dry-run`、prefix 检查、create-new/atomic replace、已有文件 ownership 校验和幂等卸载。系统级 `/usr` 安装交给 package manager，不在普通脚本中调用 `sudo`。用户级安装不修改 shell rc，不强制把 `~/.local/bin` 加 PATH；缺 PATH 时给出明确说明或在 desktop `Exec` 使用安全绝对路径。

release bundle 至少包含：

```text
sico-desktop-host
sico.desktop
sico-sapp.xml
icons/
LICENSES/
THIRD-PARTY-NOTICES
manifest.json
SHA256SUMS
```

production publisher/signing、SBOM、provenance 与更新策略服从 M7 契约。开发摘要或开发签名不能冒充生产发布身份。

## 18. 测试矩阵

| 层级 | 用例 | 通过条件 |
|---|---|---|
| Rust core | workspace fmt/clippy/test | locked/offline 全通过 |
| native build | x86_64；若支持则 aarch64 | link、ELF、依赖、glibc 基线通过 |
| XDG paths | unset/custom/空/坏 owner/mode | 默认正确，危险 runtime dir fail closed |
| association | install/query/open/uninstall | MIME 正确、单参数、无 shell、不强制默认 |
| ingress | spaces/non-ASCII/`-`/symlink/FIFO/FUSE/race/oversize | copy-before-verify 或稳定拒绝 |
| trust | bad signature/tamper/downgrade/capability drift | Runtime 前 fail closed |
| permission | deny/once/persistent/close/revoke | exact identity/fingerprint，关闭即 deny |
| lifecycle | concurrent open/crash/timeout/logout/stale socket | 单 Runtime、有界 queue、可恢复 |
| UI | GTK tree/input/rate/IME/theme | schema limits 与文本语义一致 |
| accessibility | keyboard + Orca + scaling | role/label/focus/state 可用 |
| desktop | GNOME/KDE Wayland，X11 fallback | 每种单独归档；无推断通过 |
| packaging | clean install/upgrade/uninstall/purge | ownership/data policy 与文档一致 |

推荐真实矩阵至少包含：最老支持发行版 + GNOME/Wayland、较新发行版 + KDE/Wayland、一个 X11 session、aarch64 Linux（若承诺）。容器可以验证 CLI/ABI/包安装，但不能替代真实 session、portal、文件管理器、IME 与无障碍。

## 19. CI 与可重复性

Linux CI 分为：

- headless core job：fmt/clippy/test、package/trust/property tests；
- native integration job：Wasmtime `42`、fault、storage、process supervision；
- desktop session job：DBus、portal、MIME、file manager、GTK、Wayland/X11；
- packaging job：干净 VM install/upgrade/uninstall；
- aarch64 job：原生或可证明等价的 runner，最终仍需原生运行。

锁定 Rust/Cargo.lock、toolkit/native dependency、Wasmtime 和 build image digest。两次 release build 比较摘要；若不可复现，记录差异来源，不伪造 reproducible claim。测试日志不得包含 HOME、用户名、socket token、完整外部路径或 package 私密内容。

## 20. 证据目录

每次运行保存到 `tests/platform/evidence/linux/<YYYY-MM-DD>/<run-id>/`：

```text
environment.json
build.log
binary.json
dependencies.txt
glibc-symbols.txt
association.json
ingress-security.json
permission.json
lifecycle.json
runtime.json
ui.json
accessibility.md
packaging.json
artifacts.sha256
summary.md
```

`environment.json` 包含 distro/version、kernel、arch、glibc、desktop、session/backend、portal、GTK、Rust 和 Wasmtime；`runtime.json` 记录 exact `.sapp` digest、backend、limits、result/fault；`summary.md` 明确 evidence label。截图和日志脱敏。

## 21. 常见故障

| 症状 | 首查 | 禁止的“修复” |
|---|---|---|
| `platform-artifacts` 拒绝 Linux binary | 现有 `.exe` 验证与生成器耦合 | 伪造 `.exe` 或提交 Windows 路径 |
| 双击 `.sapp` 后 CLI 报缺参数 | XDG defaults/trust/runtime resolver | 把 key/runtime 路径硬编码进 desktop file |
| MIME 仍是 octet-stream | XML 路径、cache update、glob conflict | 只改扩展名后跳过 strict loader |
| desktop entry 不启动 | `desktop-file-validate`、Exec quoting、PATH | 改成 `sh -c` |
| GNOME 正常 KDE 失败 | desktop id、mimeapps precedence、portal backend | 把一个桌面结果泛化为 Linux |
| Wayland 无法激活窗口 | activation token、parent handle、portal/toolkit API | 强制所有用户切 X11 |
| permission/UI unavailable | 当前 non-Windows stub、GTK feature/build | 用自动 allow 绕过 prompt |
| Orca 不读控件 | GtkAccessible role/label/relation/focus | 只凭视觉截图验收 |
| 旧发行版报 GLIBC not found | 构建环境与 symbol version | bundle glibc |
| Wasmtime child 残留 | process group、signal deadline、PID race | 只 kill 直接 child 后报告完成 |
| Flatpak 看不到包/Runtime | portal/document path、manifest、subprocess policy | 开放整个 host filesystem |

## 22. 开工与完成门槛

开工前必须冻结 Linux ADR：发行版、架构、glibc、GTK、Wayland/X11、安装范围、Runtime delivery、portal/Flatpak 范围。然后先修复按平台 artifact generator 和无参数 file-open config，再实现 GUI/IPC。

只有以下全部完成，Linux 才能从 `contract-verified` 升级为 `runtime-verified`：

- clean Linux build、Clippy、tests；
- 真实文件关联安装、单路径 open 与卸载；
- 同一已签名 `.sapp` digest 和结果 `42`；
- permission、GTK UI、IME、Orca、GNOME/KDE、Wayland/X11 证据；
- process crash/timeout/cancel 与 Host 生存；
- 最老支持 glibc、x86_64 和承诺的 aarch64 通过；
- 负向入口/trust/storage/lifecycle 测试全部 fail closed；
- 可审计安装包与摘要归档；
- 无 debug/development identity 被提升为 production。

缺任一项保持 NO-GO，但不阻塞平台无关工作。

## 23. 官方参考

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/)
- [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/desktop-entry-spec-latest.html)
- [Desktop Entry Exec field](https://specifications.freedesktop.org/desktop-entry/latest-single/)
- [MIME Applications Specification](https://specifications.freedesktop.org/mime-apps/latest/)
- [Added/Removed Associations](https://specifications.freedesktop.org/mime-apps/latest/associations)
- [Default Application](https://specifications.freedesktop.org/mime-apps/1.0/default.html)
- [Shared MIME-info Database](https://specifications.freedesktop.org/shared-mime-info/latest-single/)
- [GTK 4 API](https://docs.gtk.org/gtk4/)
- [GTK accessibility](https://docs.gtk.org/gtk4/section-accessibility.html)
- [GDK Wayland interaction](https://docs.gtk.org/gdk4/wayland.html)
- [XDG Desktop Portal](https://flatpak.github.io/xdg-desktop-portal/docs/)
- [FileChooser portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html)
- [OpenURI portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.OpenURI.html)
- [Documents portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Documents.html)
- [Portal request lifecycle](https://flatpak.github.io/xdg-desktop-portal/docs/requests.html)
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Wasmtime platform support](https://docs.wasmtime.dev/stability-platform-support.html)

规范、toolkit、portal 和发行版生命周期会变化。每次恢复 Linux 工作都记录复核日期；若官方行为与手册冲突，先更新 ADR/手册和测试，再修改实现。
