# STEP-0266: R1 instrumentation — canary coverage report and budget probe

> - status: complete
> - phase: M22 R-phase (replan v1 §3 R1)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture / measured (debug-runner host, ABI variant per local build cache)

## 1. Objective

落地 STEP-0264 重划片 R1 的两件仪器，并在当前工作树（含未提交 WIP）上
实测出数：

1. `tools/report-m22-canary.ps1` — canary 覆盖率上报：以执行为准测量
   `formatter.sico` 30 个函数中前缀 byte-exact（guest 编译退出 0）的函
   数个数，并给出全量源码的 typed 拒绝前沿（函数 + 身份码）。
2. `tools/probe-m22-budget.ps1` — 预算探针：对合成输入（`scan_string`
   捐赠函数 + `add` 依赖的改名副本模块）在 1k/4k/8k/16k 行规模上做燃
   料阶梯探测（exit 125 + `resource-limit.fuel` = 燃料耗尽；最小可完成
   cap = 消耗燃料估计），并记录 wall-time。

runner CLI 无燃料余量上报（lib.rs 仅内部 `get_fuel()` 判定），故用阶梯
探测替代二分；栈高水位 CLI 不可观测（仅 StackLimit outcome 类作为可见
天花板），如实记录为未测量。

## 2. Measured results (2026-09-24, this host, debug runner, cap 5e9)

### Canary（当前 WIP 树，组件构建成功、locals 未越界）

| 指标 | 值 |
|---|---|
| formatter 函数总数 | 30 |
| 已覆盖（前缀 byte-exact） | **22（73.3%）** |
| 全量拒绝前沿 | `nearest_match` @ `ERR:E-SH-IR-GWPACK-OTHER` |

前沿较 STEP-0262 的 `E-SH-IR-UNRESOLVED`（Map 参数面不可解析）**深入
了一步**：未提交 WIP（`parameters_ir` 的 Map 分支 +228 行）已让
`Map[Text,U64]` 参数面通过解析，现在拒绝发生在 `nearest_match` 本体
的 gw-pack 形状。覆盖计数未变（22/30），前沿深度前进——WIP 方向被实
测确认为有效进展。

### 预算曲线（合成输入，燃料 = 最小可完成 cap 区间）

| 输入规模 | 副本数 | 消耗燃料 | wall-time |
|---|---|---|---|
| 1,054 行 | 36 | (1e8, 1e9] | 62 ms |
| 4,128 行 | 142 | (1e8, 1e9] | 107 ms |
| 8,217 行 | 283 | (1e9, 5e9] | 189 ms |
| 16,395 行 | 565 | (1e9, 5e9] | 307 ms |

天然输入：22 函数前缀（610 行）exit 0、**wall 45.9 s**；全量 formatter
（827 行）122 拒绝 @ GWPACK-OTHER、wall 41.9 s。

### 解读（R0 输入）

1. **行数维度的燃料缩放近线性、无悬崖**：16k 行仍在 5e9 内， consumed
   ≥ 100× runner 默认燃料（1e7）——S6 闭环须按 ADR-0015 申报 5e9 级
   cap（资格运行前固定并记录，合规）。
2. **wall-time 由函数复杂度主导而非行数**：565 个平凡函数 0.3 s vs
   22 个 formatter 函数 46 s（≈2 s/函数，debug runner）。外推 selfhost
   全集（16.9k 行、含 parser.sico 深机器）自编译 wall 为**数十分钟
   级**——S6 的 timeout/cancellation 预算须按此 provisioning；
   ADR-0015 资格运行前须在 release runner + GNU 证据主机复测。
3. 审查 P0-3（"燃料悬崖从未测量"）在本路径上关闭：有数字了，且不是
   悬崖；真正的风险项从 fuel 移到 **debug-runner wall-time**。

## 3. Scope

包含：两个新工具（见 §1）；本记录；M22 计划状态头与 STATUS next-step
行追加 R1 结果指针。不包含：栈高水位仪器（需 runner CLI 变更，登记为
后续）；compilation-unit 上下文草案（R1 第三项，留待 R0 后）；任何
`parser.sico` / 测试文件改动（在途 WIP 原样保留）。

## 4. Tool notes（复现与已知限制）

- `report-m22-canary.ps1 -ComponentPath <path>` 可复用已构建组件跳过
  CLI 构建；覆盖率用前缀二分（ landed 前缀从顶部连续），前沿函数 +
  身份码是主信号。
- `probe-m22-budget.ps1`：合成模块 = `add` 前导 + N 个改名
  `scan_string` 副本（不自带依赖的副本会命中 `E-SH-IR-CALL-TARGET`，
  已修）；天然输入每轮 ~46 s，阶梯 6 轮 ≈ 276 s，超过 300 s 调用上限
  会被外层杀死——天然输入请用 `-Sizes` 单独跑或少档阶梯。
- 燃料耗尽签名经源码 + 实测双重确认：run 中耗尽 = exit 125 +
  `"class":"resource-limit.fuel"`；launch 期耗尽 = exit 126 +
  `all fuel consumed`（lib.rs:810-833 exit_code/class 映射）。

## 5. Validation

- canary 上报脚本实测输出 `sico.m22.canary.v0` JSON（30/22/73.3%，
  前沿 nearest_match @ GWPACK-OTHER）。
- 预算探针四档规模 + 两档天然输入全部实测出数（§2）。
- 既有 M22 校验器未重跑（本 STEP 不改正被它们守卫的代码；重跑留给
  WIP 落地 STEP）。
- `git diff --check` 通过；规划校验器通过（纯文档 + 新工具，不改
  计划必备节）。

## 6. Metrics

见 §2 两表。下一步复测点：WIP 落地后 canary 前沿是否移出
nearest_match；release runner 的 wall-time 复测；栈高水位仪器化。

## 7. Risks and follow-ups

- debug-runner 的 2 s/函数 wall 可能掩盖 release 下的真实曲线——
  ADR-0015 预算记录以 release 复测为准。
- 合成输入只含平凡函数，复杂度维度由天然输入补；parser.sico 级深机
  器的单函数成本仍未直接测量（W2 波次用 canary 自然覆盖）。
- 下一步（按重划片）：R0 决策文档（输入齐备：评估成本表 + 本 STEP
  数字）；或先落地在途 WIP（Map 参数面，已实测有效）为独立 STEP。

## 8. Audit links

- 计划：[`M22–M26 route replan v1`](../plans/M22-M26-route-replan-v1.md) §3 R1
- 输入：[`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md)
- 关闭项：review P0-3（燃料实测）
