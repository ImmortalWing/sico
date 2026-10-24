# 工具脚本索引

本目录同时保存用户入口、开发工具、发布编排和历史验证脚本。请从下表选择稳定入口，不要把 `validate-step-*` 或内部打包器当作用户命令。

## 用户安装与更新

| 入口 | 用途 | 直接使用 |
|---|---|---|
| Release 中的 `sico-init-*.exe` | 双击安装或更新 Windows 用户级 SDK | 是，普通用户首选 |
| `install-windows.cmd` | 从已解压 SDK 安装或更新 | 是，终端简写入口 |
| `install-windows.ps1` | 安装实现、机器级安装和卸载 | 是，高级入口 |

安装和更新使用同一套收敛逻辑：重复运行新版 EXE 或新版 SDK 中的 `install-windows.cmd`，会先验证新工具，再替换受 Sico 安装标记管理的目录，并去重 PATH。当前没有后台自动更新服务。

## 日常开发

| 入口 | 用途 |
|---|---|
| `sico-dev.ps1` | 一条命令完成源码编译、打包和本地运行 |
| `ensure-wasmtime.ps1` | 定位或准备仓库锁定的 Wasmtime |

已安装 SDK 优先直接使用 `sico-app dev <SOURCE.sico>`；`sico-dev.ps1` 保留暂停窗口和旧脚本兼容。

## Registry origin 运维

```powershell
.\tools\package-registry-origin.ps1
```

该脚本构建、自检并打包独立的只读 registry origin。生产边界、TLS/edge 要求和证据限制见 [`deploy/registry-origin/README.md`](../deploy/registry-origin/README.md)。

## Windows 发布

公开发布只有一个入口：

```powershell
.\tools\release-windows.ps1
```

职责链如下：

```text
release-windows.ps1
  ├─ 质量门禁与模块边界
  ├─ 构建 compiler / app CLI / LSP / AI tool / registry origin
  ├─ 组装 compiler ZIP 与 SDK ZIP
  ├─ internal/package-windows-installer.ps1
  │    ├─ 把 SDK ZIP 封装为 sico-init EXE
  │    └─ 验证 EXE 安装、版本、PATH 与卸载
  ├─ package-registry-origin.ps1
  │    └─ 生成并自检独立 operator ZIP
  └─ 生成 SHA256SUMS / SBOM / manifest / Release Notes
```

`internal/package-windows-installer.ps1` 和 `internal/windows-installer-bootstrap.ps1` 是发布编排的内部步骤，不是普通用户安装命令。`package-windows-host.ps1` 只处理 Desktop Host 试验产物，不属于语言 SDK Release。

完整发布操作见[Windows 手动发布](../docs/development/RELEASING.md)。

## 验证、测量与 AI 评估

- `validate-module-boundaries.ps1`：稳定的架构边界门禁；
- `validate-step-*.ps1`、`validate-m*-exit.ps1`：历史里程碑证据，不是用户入口；
- `validate-*.ps1`、`measure-*.ps1`：专项验证与性能测量；
- `validate-m22-s5.ps1`：M22 S5 自举线的**版本化验证电池**（fmt / clippy / 一键 CI / 0245 / 0262 / 两个前沿探针 / 注释纪律 / `match` 站点普查与覆盖 / 固件 / 空白检查；每步记录命令、耗时与 exit，任一步失败即整体失败）。**必须单实例运行**：两个实例并发会争用同一 `target/` 与同一 `battery.log`，日志交错、步骤顺序失真 ⇒ 这种运行的「绿灯」不可引用，必须终止陈旧实例后单实例重跑；
- `probe-m22-match-coverage.ps1`：**`match` 构造族的真实站点覆盖探针**（账本 `selfhost/match-component-coverage-v0.json` 的逐站 IR + 组件差分与真实 validator）；
- `census-m22-match-sites.py`：上述账本的**唯一写者**（`--check` 校验 / `--update` 回填实测裁决）；
- `run-ci.ps1`：一键 CI（fmt / clippy / 构建 / workspace 与 runner 测试 / 模块边界 / 注释纪律 / 规划合同 / 跨主机矩阵 / 空白检查）。**失败时**把失败子步骤的名字与该子步骤输出的末尾若干行写成 GitHub **工作流注解**，并把逐步骤结果表写进 **job summary** ⇒ 独立 CI 的红灯在运行页面上**无需登录**即可定位（Actions 日志本身需要登录会话）；
- `prepare-ai-eval.ps1`、`test-ai-eval.ps1`、`score-ai-eval.ps1`、`validate-ai-eval.ps1`：AI 评估流水线。

新增脚本时应优先放入以上职责之一，并在本页登记；不要继续向 `release-windows.ps1` 内嵌独立安装器实现。
