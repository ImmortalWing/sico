# M22 交接手稿（living handoff，STEP-0280 本机基线）

> - 更新：2026-09-25（STEP-0291 基线）；工作目录 `E:\github\sico`，分支 `dev`，当前 HEAD `0e4cbb9`（含 STEP-0264–0278）。
> - **未提交、未推送（owner 此前明确要求）。** 工作树现含 STEP-0279..0291 共 13 个 STEP（路线审查/W1 C/D/执行卡/W2 基线/本机全量 CI/M23 普查与 RFC-0048·0049/M24 盘点与 RFC-0050·0051·0052+ADR-0018/M26 盘点与 RFC-0053·0054/nearest_match 设计）；先运行 `git status --short`，逐项保留。
> - 权威裁决：[STATUS](./STATUS.md)、[M22 计划](./plans/M22-compiler-self-host.md)、[各 STEP 文档](./steps/README.md)；本手稿提供恢复路径，不代替 gate 审计。
> - 交给较弱模型时，只分配 [M22–M26 单步执行卡](./plans/M22-M26-execution-cards.md) 的一张卡；每步按当前证据重新裁决入口。

## 0. 一句话状态

M22 仍为 **NO-GO**。S1 与声明的 S2 子集完成；S3/S4/S5 partial；S6/S7 未进入。R0 已由 [ADR-0017](./adr/ADR-0017-selfhost-data-model-architecture-v0.md) 的 Option A 和 [RFC-0047](./rfc/RFC-0047-record-types-v0.md) 接受记录确定；R1 本机 canary 为 **22/30（73.3%）**，前沿 `nearest_match` / `ERR:E-SH-IR-GWPACK-OTHER`，整份 formatter typed refusal exit 122。栈高水位未测得。

W1 A/B 位于 HEAD 的 STEP-0278；W1 C/D 位于当前未提交工作树的 STEP-0280。本机 Windows x64 GNU 真实 runner 已通过原 215 项冻结差分、5 项增量差分以及 STEP-0221/0245/0261/0262 校验器。**STEP-0278/0280 的独立 CI 尚未裁决，因此 W1 不能宣告关闭，也不能把 M22 改为 GO。**

## 1. 当前工作树的两批未提交内容

1. **M14–M26 审查与路线修订（STEP-0279）**：`docs/reports/m14-m26-milestone-audit-2026-09-24.md`、`docs/steps/STEP-0279-m14-m26-milestone-audit-route-sync.md`、ROADMAP/STATUS、各里程碑计划、ADR-0017/RFC-0047 说明、`tools/validate-step-0124.ps1` 等。规划校验器含五个负例，均本机通过。历史 M14–M21 STEP 裁决未追改。
2. **M22 W1 C/D 实现（STEP-0280）**：`selfhost/compiler_semantics.sico`；`selfhost/corpus-w1/` 五个 SHA 冻结用例；`runner/sico-runner/tests/selfhost_checker.rs` 的 Rust oracle/guest 差分；`bootstrap_bundle.rs` 对 RFC-0047 后增 12 个 record 用例的显式盘点；`record_types.rs` 的 `RunOutcome::Output` 解构修复；M22 runner 测试文件的格式修复；STEP-0261/0262 的 IR 快照和现行前沿校验器重钉；STEP、ROADMAP、STATUS、计划及审查表同步。

这两批修改尚未提交。`git diff --check` 不覆盖未跟踪文件；继续工作时也要查看新增 STEP 和 `selfhost/corpus-w1/`。不要用 `git reset`、`git clean` 或覆盖式检出清掉它们。完整改动及准确测试边界见 [STEP-0280](./steps/STEP-0280-w1-e7002-and-zero-indent.md)。

## 2. 已运行的证据与限制

| 检查 | 结果 | 证据边界 |
|---|---|---|
| `selfhost_checker` | 9/9 绿；原 215 项分区为 116 lexical / 34 identity / 65 accepted / 0 unsupported；W1 新增 5 项 Rust/guest 一致 | Windows x64 GNU，真实 runner；internal-fixture |
| `record_types` | 12/12 绿 | Windows x64 GNU，真实 runner；internal-fixture |
| `validate-step-0221.ps1` | 21/21 绿 | compiler frontend 差分 |
| `validate-step-0245.ps1` | 完整绿：Rust semantic 6+11、bundle 4、checker 9、compiler 21、runner all-target clippy | L1 checker/语义与相邻 compiler 回归 |
| `validate-step-0261.ps1` / `0262.ps1` | 完整绿；17/22 函数 IR 快照含 STEP-0276 后的 `records` 表；历史拒绝点更新为当前前沿 | 当前源码和真实 runner；不是覆盖率提升 |
| `report-m22-canary.ps1` | 22/30；`nearest_match` / `ERR:E-SH-IR-GWPACK-OTHER`；full-source exit 122 | R1 当前阶段指标，和 STEP-0266 基线持平 |
| root/runner `cargo fmt --check`、runner all-target clippy、`validate-step-0124.ps1 -SelfTest`、`git diff --check` | 绿 | 未运行完整 Cargo 工作区或独立 CI |
| `tools/run-ci.ps1` 完整管线（STEP-0283） | **CI GREEN 11/11**（workspace/runner 测试、全部校验器、whitespace） | 本机工作树全量；仍不代替独立 CI（owner 门：授权推送或显式接受本机裁决） |

初次直接运行 runner cargo 会选 MSVC 并报 `link.exe` 缺失。已验证的本机环境是 `1.98.0-x86_64-pc-windows-gnu` 加仓库内 `target\tooling\msys2-binutils\mingw64\bin`（含 gcc/dlltool）进 `PATH`；旧交接文档“本机不可重建 runner”的说法已经失效。运行命令见 §4。

## 3. 下一条有界工作（全部在等 owner 决策）

1. **W1 裁决（唯一关键路径阻塞）。** 两条路径由 owner 择一：(a) 授权提交/推送由远端 CI 裁决 STEP-0278/0280；或 (b) 显式接受本机 run-ci GREEN 11/11（STEP-0283）为裁决。已两次直接呈请（含选择题）未获答复；**不得代行**。
2. **W1 GO 后立即执行 22-C S1**（[STEP-0291 设计](./steps/STEP-0291-m22-nearest-match-design.md) 已就绪）：while 体 let 的 `sico.u64.to_text` 内建 RHS 入 `gw_rhs_packed`/`gw_intrinsic_call_packed` 面；出口 = A2 探针形状 exit 0、前沿移动到 match 主语层；随后 S2（`sico.map.get` match 主语）、S3（match-in-while 框架+臂内 if/return）。R3 记账已预声明：前沿移动 = 拒绝码变化；canary 保持 22/30 直至 nearest_match 整体降低。对照 [W2 基线](./reports/m22-w2-baseline-2026-09-25.json) 记录前后值。
3. **五组 RFC/ADR 待 owner 接受**：RFC-0048/0049（M23）、RFC-0050/0051（codec/provenance）、RFC-0052+ADR-0018（原生 UI）、RFC-0053/0054（PDF；实现另等 M25 GO）；可选：M17 加速主机命名、pikepdf oracle 安装授权。
4. 免输入的开工/测量/合同工作已全部收口（M22–M26 各里程碑盘点、普查、基线、设计齐备）；不要重新盘点。

## 4. 恢复命令（PowerShell，仓库根目录）

```powershell
git status --short
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$binutils = Join-Path (Get-Location) 'target\tooling\msys2-binutils\mingw64\bin'
$env:Path = "$binutils;$env:Path"
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_checker
.\tools\validate-step-0221.ps1
.\tools\validate-step-0245.ps1
.\tools\validate-step-0261.ps1
.\tools\validate-step-0262.ps1
.\tools\report-m22-canary.ps1
.\tools\validate-step-0124.ps1 -SelfTest
git diff --check
```

STEP-0261/0262 会重复跑较慢的 compiler/parser 差分；只在复核当前门槛或相关源码变化后重跑。`sico check` 不能把带 `module` 的 `compiler_semantics.sico` 当 CLI 根文件单独检查；用 `target/debug/sico.exe check selfhost/checker.sico` 或 `selfhost/compiler.sico`。当前完整 `tools/run-ci.ps1` 与独立 CI **未在 STEP-0280 执行**，不要把局部校验当成全仓回归。未来 S6 还必须测到实际 peak guest memory/栈相关预算，不能引用 STEP-0266 的缺失值。

## 5. 接手完成条件

- 当前工作树与未跟踪文件完整保留；任何新变动有唯一、当时分配的 STEP。
- W1 是否关闭必须有独立 CI 的可追溯结果和明确 GO/NO-GO；此前保持 M22 NO-GO。
- W2 进入前冻结新的 records-surface canary；R3 计数起点与每一步前后值清晰。
- 不把 Rust oracle、原生权限边界、M24 GUI 转换器或 M26 PDF 目标改写成已实现支持。
