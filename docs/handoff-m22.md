# M22 交接手稿（living handoff，STEP-0293 本机基线）

> - 更新：2026-09-25；工作目录 `E:\github\sico`，分支 `codex/m22-w1-ci`。
> - `origin/dev` 已前进至 `550059a`，包含另一套 STEP-0279–0283 W1/W2 历史，与本分支源码及 STEP 编号冲突。不得覆盖或隐式合并。
> - 权威裁决：[STATUS](./STATUS.md)、[M22 计划](./plans/M22-compiler-self-host.md)、[STEP-0292](./steps/STEP-0292-m22-w1-independent-ci.md) 与 [STEP-0293](./steps/STEP-0293-m22-w2-u64-to-text-while-rhs.md)。本手稿只提供恢复路径。

## 0. 当前状态

M22 整体 **NO-GO**。S1/S2 声明子集完成，S3/S4/S5 partial，S6/S7 未进入。R0 已接受 ADR-0017 Option A 与 RFC-0047。STEP-0292 对 `e8495f9` 的独立 Windows GNU [GitHub CI](https://github.com/ImmortalWing/sico/actions/runs/36087222284) 成功，**W1 在本隔离分支快照裁为 GO**。该裁决不覆盖 `origin/dev` 的并行历史。

STEP-0293 是 22-C 首个 R3 计数实现片：while 体 `sico.u64.to_text` RHS 已与 Rust oracle byte-exact；formatter canary 为 **22/30（73.3%）**，`nearest_match` 的 typed 拒绝由 `ERR:E-SH-IR-GWPACK-OTHER` 移至 **`ERR:E-SH-IR-CALL-TARGET`**，整份 exit 122。前沿移动，R3 连续停滞数 **0**。本片本机验证通过，**独立 CI 尚待运行**，因此 S1 最终裁决待定。

## 1. 证据与边界

| 检查 | 当前结果 | 证据边界 |
|---|---|---|
| STEP-0292 fresh-runner CI | `e8495f9` 的完整 `run-ci.ps1` 作业 success | W1 快照；Windows 2025 GNU；不替代 S1 本片 CI |
| `selfhost_checker` | 9/9，原 215 项 + W1 五项 Rust/guest 对照 | Windows x64 GNU 真实 runner；internal-fixture |
| S1 新增 compiler 用例 | while 体 `u64.to_text` canonical IR byte-exact，双参数 typed 拒绝 | 真实 runner；internal-fixture |
| `validate-step-0262.ps1` | compiler 22/22、parser 2/2、local-bounds 1/1、22 函数 IR 快照、现行前沿均绿 | S1 本机复跑 |
| `validate-step-0261.ps1` | 同上及 17 函数历史快照绿 | S1 本机复跑 |
| `validate-step-0245.ps1` | semantic 6+11、bundle 4、checker 9、compiler 22、runner clippy 全绿 | S1 本机复跑；首次 fmt 红已修复并整条复跑 |
| `report-m22-canary.ps1` | 22/30，`nearest_match` / `CALL-TARGET`，exit 122 | S1 当前树；栈高水位仍未测得 |

## 2. 下一步

1. 提交并推送 STEP-0293 到同一隔离分支，查看该提交的独立 CI。若失败，只修具体失败并复跑；成功后登记 S1 GO。
2. S1 GO 后按 [STEP-0291](./steps/STEP-0291-m22-nearest-match-design.md) 开始 22-C S2：`sico.map.get[Text,U64]` match 主语。每个实现 STEP 记录覆盖数、typed 前沿、R3 停滞计数和独立 CI。
3. 合并 `origin/dev` 的并行历史前逐项处理重复 STEP 编号、235 项语料和 `compiler_semantics.sico` 差异；不得把本分支 CI 声称为远端 dev 的验证结果。
4. M23 的 RFC-0048/0049、M24 的 RFC-0050/0051/0052 与 ADR-0018、M26 的 RFC-0053/0054 仍各自等待 owner 接受；它们不阻挡本分支 22-C S2。

## 3. 本机复核命令（PowerShell，仓库根目录）

```powershell
git status --short
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$binutils = Join-Path (Get-Location) 'target\tooling\msys2-binutils\mingw64\bin'
$env:Path = "$binutils;$env:Path"
.\tools\validate-step-0245.ps1
.\tools\validate-step-0261.ps1
.\tools\validate-step-0262.ps1
.\tools\report-m22-canary.ps1
.\tools\validate-step-0124.ps1 -SelfTest
git diff --check
```

STEP-0261/0262 会重复执行较慢的 compiler/parser 差分；只在源码或当前前沿变化后重跑。S6 还必须实测 guest memory/stack 预算，不能将缺失的栈高水位写成已有证据。
