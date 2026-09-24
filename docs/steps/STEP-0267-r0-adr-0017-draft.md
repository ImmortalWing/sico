# STEP-0267: R0 decision document draft — ADR-0017 (SOA permanence vs records)

> - status: complete (draft registered; **owner acceptance pending** — ADR-0017 status: proposed)
> - phase: M22 R-phase (replan v1 §3 R0)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: planning-only (inputs = STEP-0265 cost table + STEP-0266 measurements)

## 1. Objective

按 STEP-0264 重划片 R0 的要求起草架构决策文档，终结质量审查 P0-2 的
悬置状态（"SOA 过渡架构尚无通用性出口判据"）：两条路径二选一——

- **Option A**：现在开 records RFC（RFC-0033 全门），M22 内 re-baseline
  自举源码到 `List[record]`；月级语言面成本。
- **Option B（推荐）**：SOA 升级为 M22 永久数据模型（修订 ADR-0016 的
  "过渡形状"框定为"带重评条件的永久架构"），并以合同补齐三件套——
  ① canary：`report-m22-canary.ps1` 的 formatter 覆盖率 + 拒绝前沿；
  ② coverage-growth 指标：每个实现 STEP 必须推进 canary 函数数或移动
  拒绝前沿；③ 失败阈值：R3 止损（连续 4 个 STEP 零增长即宣告 S6 不收敛）。
  records RFC 脱钩不阻塞（M14/M23 驱动），落地后经显式跟进决议 re-baseline。
- **Option C**：不做决定——拒绝（正是 P0-2 债务本身）。

否决 Option A 的证据：STEP-0266 R1 实测燃料近线性、无悬崖（16k 行 ∈
(1e9,5e9]），瓶颈在复杂度主导的 wall-time 而非数据模型；机器逻辑与语
言形状无关，records 不改变 S6 的算法形状，只改授权表面——其收益是应用
profile 收益，不是 M22 收敛收益。

## 2. Scope

包含：新建 `docs/adr/ADR-0017-selfhost-data-model-architecture-v0.md`
（status: proposed，模板六段齐备：Context / Decision drivers /
Considered options / Decision / Consequences / Validation / Revisit
conditions / Links）；ADR README 登记行；本记录与 steps 索引；M22 计划
状态头与 STATUS next-step 指针更新。

不包含：owner 接受（待拍板）；W1 合并债的实现（ADR 接受后第一个
STEP）；ADR-0016 文本修改（本 ADR 经 depends 引用而非改写，避免在未接
受前动已接受合同）。

## 3. Decision shape

ADR-0017 以显式条件化了 owner 的两个选择：接受 Option B → status 转
accepted，R 阶段进入 W1；改选 Option A → 本 ADR 拒绝，按 M23 Route-B
式规则起草 records RFC 作为 R0 产出。Revisit conditions 固定了三条重
评触发（预算超阶 / records RFC 落地 / R3 止损实发）。

## 4. Validation

- 模板与登记格式符合 `docs/templates/ADR.md` 与 ADR README 约定；
- `tools/validate-step-0124.ps1` 通过（ADR 不属该校验器范围，但伴随的
  计划文本改动保持其绿色）；
- `git diff --check` 通过。
- 未运行：编译/测试（纯文档步骤）。

## 5. Metrics

无执行指标。ADR 自带的验证门：接受后第一个 STEP 必须重跑 canary 上报
（当前树预期 22/30、前沿 `nearest_match` @ `ERR:E-SH-IR-GWPACK-OTHER`）
并把 W1 项登记为队列条目；R3 计数从 W2 第一个 STEP 起算。

## 6. Risks and follow-ups

- owner 若选 Option A，M22 进入 M23 Route-B 式重启，本 ADR 的 W1 清单
  中除 records 相关外仍有效（合并债与数据模型无关）。
- 下一步：owner 拍板 ADR-0017；接受后按 STEP-0265 §6 顺序进入 W1 合
  并债（或在途 WIP 先落地为独立 STEP——两者互不阻塞）。

## 7. Audit links

- [`ADR-0017`](../adr/ADR-0017-selfhost-data-model-architecture-v0.md)
- 输入：STEP-0265（成本表）、STEP-0266（R1 数字）
- 前置：STEP-0264（R0 门定义）、ADR-0016（compilation-unit 合同）、
  review P0-2
