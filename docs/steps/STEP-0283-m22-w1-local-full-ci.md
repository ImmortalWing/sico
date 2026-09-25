# STEP-0283: M22 W1 前置裁决——本机全量 CI 绿 + 校验器 BOM 修复

> - status: complete / local full-CI green on the worktree; W1 independent CI still 未裁决, M22 stays NO-GO
> - phase: M22 compiler self-host — W1 adjudication prep (execution card 22-A, owner-authorized portion)
> - date: 2026-09-25
> - evidence class: internal-fixture, local one-command CI (tools/run-ci.ps1), Windows x64 GNU

## 1. Scope and decision

卡 22-A 要求 STEP-0278/0280 的**独立 CI** 裁决；本次仍无提交/推送授权，
按 [handoff §3.1](../../handoff-m22.md) 的边界执行其本机可做部分：对携带
STEP-0279/0280/0281/0282 全部未提交改动的工作树跑完整
`tools/run-ci.ps1`，提前暴露并修复 CI 红项。本 STEP 不计 R3（验证/修复）。

## 2. Findings and fixes

1. **真实潜在 CI 红（已修）：`tools/validate-step-0124.ps1` 缺 UTF-8
   BOM。** STEP-0279 加的五个负例含中文 owner 指令锚串；PowerShell 5.1
   在 ANSI 代码页（本机 cp936、en-US CI runner 的 cp1252 同理）按 ANSI
   解析无 BOM 文件，中文锚串处直接 ParserError。修复 = 文件头加 UTF-8
   BOM（内容零改动）；修复后 `powershell -File … -SelfTest` 直跑通过
   （主检查 + 五负例）。仓库其他可执行 ps1 均为纯 ASCII 或带 BOM，本
   文件是唯一违例——CI 管线第 8 步必然红。
2. **链接偶发崩溃（登记，不修产品）：** 首轮 run-ci 在 test (runner) 步
   红于 rustc 链接某测试二进制时 `exit 0xc0000409
   (STATUS_STACK_BUFFER_OVERRUN)`（windows-gnu binutils）。同一命令复跑
   链接通过、套件绿——归类为本机工具链链接 flake，非产品回归；按 M19
   flake 政策留痕：红→复跑绿，无代码变更。
3. **环境前置确认（非缺陷）：** 裸 shell（无 run-ci 环境）跑 runner 串行
   套件时 2 个 console-control 测试因缺 `SICO_PYTHON` 报 WindowsApps
   python 存根 panic——STEP-0196 已知前置，run-ci 自行 provision，不受
   影响。

## 3. Result

复跑 `powershell -File tools/run-ci.ps1`：**CI GREEN 11/11**——fmt、
clippy (workspace)、clippy (runner)、build (runner debug)、
test (workspace)、test (runner，串行)、module boundaries、planning
contract (M14-M25)、application-profile matrix、cross-host matrix + UI
corpus、whitespace 全 PASS。这是 STEP-0278 批以来第一个覆盖全部未提交
改动的全管线绿。

## 4. Gate accounting

**W1 仍为待裁决：** 本机全量 CI 绿显著压缩独立 CI 的残余风险（尤其
runner 测试仓在 GNU 工具链下的全新构建路径），但按卡 22-A 与 handoff
口径，它不替代独立 CI。W1 关闭的两条路径，由 owner 择一：

- **(a)** 授权提交/推送，由远端 CI 对 STEP-0278/0280 作独立裁决；
- **(b)** 显式接受"本机全量 run-ci 绿"为 W1 裁决依据（记录为 owner
  决定，独立 CI 缺口如实保留在案）。

任一路径 GO 前：W1 未关闭、M22 NO-GO、22-C 不得开始。本 STEP 未运行
额外 Cargo 目标（run-ci 已覆盖 fmt/clippy/test/构建）；
`validate-step-0124 -SelfTest` 与 `git diff --check` 通过。
