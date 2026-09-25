# M22–M26 单步执行卡（2026-09-25）

> 本文是交接用的**下一步边界**，不是新的能力合同或 GO 裁决。权威顺序：实际可执行结果与最新 STEP → 已接受 RFC/ADR → [STATUS](../STATUS.md) 与 [审查表](../reports/m14-m26-milestone-audit-2026-09-24.md) → 各里程碑计划 → 本文。冲突时取较低支持结论，在同一 STEP 修复现行文档。每次只交给执行者**一张卡**；完成或遇停手条件后重新读状态，不按表格自动推进。未来实现 STEP 编号不预留。

## 0. 每张卡的共同协议

1. 先运行 `git status --short`，列出已存在的修改和未跟踪文件；保留它们。当前工作树的 STEP-0279/0280 尚未提交；用户已要求**不提交**，也不推送。读本卡所列计划、接受的 RFC/ADR、最近 STEP 和真实源码/测试；计划文本不等于实现支持。
2. 在 STEP 开头写明：卡号、入口证据链接及裁决、唯一出口测试、允许修改的文件范围、平台、证据等级和本次不处理的门槛。缺任一入口证据时只做盘点/合同草案，不实施能力；RFC/ADR 未接受时停在合同审查。不能自行选择未冻结的语法、权限、PDF 方言、编解码参数或 UI 架构。
3. 运行最窄的真实检查，记录**精确命令、退出码、通过/失败数、runner/平台、语料 SHA 或冻结清单、未执行项**。`sico check`、静态合同测试、合成 reference 结果不能替代 build/run、真实加速器或原生 UI 证据。失败时保留 typed refusal 和最小反例，不扩大范围绕过失败。
4. STEP 末尾给每一道受影响 gate 明确 `GO`、`NO-GO` 或 `待裁决`，并同步 STATUS、ROADMAP、计划和证据索引；历史 STEP 原裁决不追改。只有当前卡出口已通过才领取下一张卡。文档修改至少运行 `./tools/validate-step-0124.ps1 -SelfTest` 与 `git diff --check`；能力实现运行卡内对应的真实 runner 和邻近回归，不机械替代为全仓 Cargo。

### 发给单个执行模型的固定输入

```text
只执行 docs/plans/M22-M26-execution-cards.md 的 [卡号]。
先读 AGENTS.md、docs/STATUS.md、该卡所列计划/接受合同/最新 STEP，运行 git status --short 并保留已有修改。
入口未通过则只记录缺口并停下；禁止提前做下一张卡、预留 STEP 编号、把本机测试写成独立 CI，或把合同/fixture 写成运行支持。
交付：本次 STEP、最小改动、可复现命令及原始结果、前后 gate/证据等级、未执行项、STATUS/ROADMAP/计划同步。不要提交或推送。
```

## 1. M22：先裁 W1，再进入 W2

| 卡 | 入口与本次唯一输出 | 执行与停手 |
|---|---|---|
| **22-A：W1 独立裁决** | [M22](./M22-compiler-self-host.md) §3.2、[STEP-0278](../steps/STEP-0278-w1-legacy-retirement-and-sha-gate.md)、[STEP-0280](../steps/STEP-0280-w1-e7002-and-zero-indent.md)、[本机交接](../handoff-m22.md)。取得与当前变更对应的**独立 CI**运行记录；逐项列 STEP-0278/0280 的结果，形成 W1 GO/NO-GO。 | 若只有本机 215+5、canary 或缓存构建结果，保持 `待裁决`。CI 红项只修具体失败并复跑相应 runner；不进入 22-C。没有授权提交/推送时不为触发 CI 自行提交。 |
| **22-B：W2 测量基线** | W1 可未裁决，但这张卡仅做测量。运行 `./tools/report-m22-canary.ps1`，以 22/30、`nearest_match` / `ERR:E-SH-IR-GWPACK-OTHER` 为待复核起点；记录真实 runner、冻结语料、fuel 区间、栈高水位实际有无，并列出当前 tracked `selfhost/*.sico`。输出可复算基线和差分/拒绝前沿表。 | 测量 STEP **不计** R3。栈高水位缺失写“未测得”，不可估算。记录旧 `lexer.sico`/`tokens.sico`/`declaration_parser.sico` 已由 STEP-0278 退役；它们不是实现队列。 |
| **22-C：W2 S3/S4 一个前沿** | 22-A W1 GO，22-B 基线已冻结；ADR-0017 Option A、RFC-0047 accepted。只挑当前首个 typed refusal（目前 `nearest_match`），对照 Rust oracle 做**一个**声明形状的 byte-exact lowering、拒绝恢复与 limit+1 回归，重钉下一个前沿。 | 先后运行 canary 和对应 `validate-step-0261.ps1` / `0262.ps1`、受影响 runner 差分。每个实现 STEP 记录前后分子/分母、拒绝码和 frontier 是否移动；当前阶段既无增长也无前沿移动则 R3 连续停滞计数加一。达到 4 次立即记录止损与 M23 Route B，停止同一路径实现。CI 尚未实际运行则该 STEP 待裁决。 |
| **22-D：S5 / S6 / S7** | formatter 30/30 后，S3/S4 改以其余**现存**自举源码的冻结差分计；S5 另建 RFC-0011 codegen 字节比较语料；S6 按 ADR-0015 做 `A == B == C`、M7 `.sapp` 包装与真实 runner 自编译；S7 才汇总审计。每个子门单独 STEP。 | S5 不能用 formatter 百分比代替；S6 必须实测 wall-time、fuel、hostcall、内存/栈预算。未到 S7 不写 M22 GO；Rust oracle 保留。 |

22-A/22-B 可按本机交接 §4 复现 PowerShell 命令；新的实现卡以**当时实际代码和失败点**选择最窄 runner 测试，不能照抄旧拒绝码。R3 的“连续”只跨 W2 S3/S4 实现 STEP；测量/文档 STEP 不计，也不清零。详见[路线重划](./M22-M26-route-replan-v1.md) §3。

## 2. M23：四个项目分别决定，不猜语法

| 卡 | 入口与本次唯一输出 | 执行与停手 |
|---|---|---|
| **23-A：消费证据盘点** | [M23](./M23-language-v1-batch-3.md) §7–9。枚举当前 tracked selfhost 源码和行数，冻结应用/AI 语料来源，分别测四项 ceremony、bind-value、bare literal、`text.chars` 成本；写精确分母与可复算计数。可与 M22 并行。 | 不改 grammar/IR。历史“11 文件/14,018 行”不可直接当当前分母；若语料来源或 owner 审核缺失，就列缺口。 |
| **23-B：逐项 RFC** | 23-A 有实测消费者；按 RFC-0033 为每项写 desugar、source-map、formatter、稳定诊断及兼容性证据。bare literal 可裁“D3 维持”；`text.chars` 可裁“保留组合形式”。 | 没有 accepted RFC 不写实现；不得把 RFC 草案当支持。四项独立裁决，未接受项目保持待定。 |
| **23-C：单项实现与出口** | 已记录 Route A（M22 S7 GO）或 Route B（R3 四步止损），且该项 RFC accepted；只做一个 RFC 声明形状。 | 真实 runner 的正例、typed 拒绝/limit+1、formatter 幂等、源码 span、matrix、冻结快照、M22 双实现重基线逐项核验。缺任一项该项 NO-GO；M23 总 GO 等全部接受项审计完成。Route B 不能写成 M22 自举 GO。 |

## 3. M24：四条独立裁决线

| 卡 | 入口与本次唯一输出 | 执行与停手 |
|---|---|---|
| **24-A：盘点与合同** | [M24](./M24-vision-closure-gui-application-pilot.md) §7–8。分别列 M17 加速证据主机、模型资产、roster 包、原生 UI 控件/renderer、JPEG↔PNG codec 与转换器拒绝语料。每条合同按 RFC/ADR 冻结，允许与 M22/M23 并行。 | 未选真实加速器则 M17 gate 2 `NO-GO/延期`；reference-only 不能翻 GO。原生 UI 的 source binding RFC 和 renderer ADR 未接受时不开始 UI 实现；codec RFC 未接受时不写解码器。 |
| **24-B：M17 门** | 加速 ADR、模型 provenance RFC、roster 各自已接受。对真实 Windows 加速器跑 reference 对照/崩溃/资源耗尽；模型走 digest/预算/取消/恶意资产；包走真实 M7 消费差分。 | 分别裁 gate 2、gate 4、roster；只有全部实证通过再裁 M17 总 GO。无硬件结果仅保留延期，不借转换器结果补足。 |
| **24-C：Sico 原生 UI 库** | source binding RFC + renderer ADR accepted；只实现已冻结控件的一个有界切片，在真实 Windows native renderer 测确定性帧、事件、权限拒绝。 | Web/webview 帧、M15 Web binding、native Rust companion 均不能算 Sico 原生库。M26 所需 page list/metadata/rotate/merge 控件不足时，在本工作流补合同和版本化能力；M25 冻结后须新版本兼容语料。 |
| **24-D：JPEG↔PNG 与转换器** | codec RFC accepted，所需原生 UI 切片有真实 runner 证据；先独立裁双向 codec，再裁 Sico 写的 GUI 应用。 | [M24 §8.5](./M24-vision-closure-gui-application-pilot.md) 的 bad magic、截断、尾随、超限、取消、保存失败、重复输出逐项 typed；Windows 原生 GUI 实跑且无 compiler/Runtime 临时补丁。分别裁 codec、UI、converter、M18 增补；M24 审计覆盖届时所有已落地能力。 |

## 4. M25：审计完成与两个 GO 分开

| 卡 | 入口与本次唯一输出 | 执行与停手 |
|---|---|---|
| **25-A：证据/冻结盘点** | [M25](./M25-release-v1-completion.md) §2/§7；M23 和 M24 各有明确 GO/NO-GO 与残余；M22 为 S7 GO 或有 R3 止损登记。逐项重发 AGENT_GOAL §13 表、v1.0 bundle composition、runner/平台/evidence 等级。 | 缺 M23/M24 裁决或 M22 明示状态时不能进入发布裁决；外部 runner/身份/采用缺口分别列延期门，不填“已完成”。 |
| **25-B：发布核验** | M23 batch 3 GO、M24 原生 GUI 转换器 GO、语言 v1 冻结；在实际目标主机执行同一命令的 M0–M24 回归、两次独立 clean build 字节一致、签名安装/运行/升级/卸载、文档对行为审计。 | 缺任何内部门为 `v1.0 release NO-GO`；审计仍可完成。未运行的平台留 `contract-verified` 或延期；不能引用旧 run-ci 结果当当前候选结果。 |
| **25-C：双结论** | 25-A/B 证据表完整。分别发布 `v1.0 release GO/NO-GO` 和 `project §13 completion GO/NO-GO`，附逐项缺口及 M22 Rust-oracle/selfhost 状态。 | v1.0 GO 不自动推出 §13 GO；外部采用、生产或平台缺口未满足则 §13 NO-GO。M23 或转换器 NO-GO 时 v1.0 NO-GO。 |

## 5. M26：合同、阅读、写入、GUI 顺序验收

| 卡 | 入口与本次唯一输出 | 执行与停手 |
|---|---|---|
| **26-A：早期盘点与 RFC** | [M26](./M26-pdf-document-processing.md) §2/§7。可在 M25 前测 byte/text/List/Map、PDF 语料生成器及 oracle 版本、xref/对象限制、pure-Sico 与 provider inflate 候选、M24 UI 控件缺口。分别接受 PDF 数据和限制/拒绝 RFC。 | 明确 PDF 版本/方言、xref 形式、filter、上限、非法输入 typed 码、byte-stable writer 和 inflate 选择；未冻结不实现。UI 缺口回 M24 §8.6.1，不用 Web 替代。 |
| **26-B：阅读切片** | M25 **v1.0 release GO** 且 26-A 两 RFC accepted；先做被合同选中的最小结构阅读与恶意语料，再按独立 STEP 扩展 xref stream、object stream、incremental update。 | 每片冻结 positive/refusal corpus、byte-exact oracle、limit+1；未支持形式 typed-refuse，不宣称通用 PDF 兼容。 |
| **26-C：合并/旋转** | 阅读所需形状已 GO，writer 确定性合同冻结。分别实现 N 输入合并和继承 `/Rotate` 的 90° 步进；重复输出 byte-identical，xref/object renumbering 对照 oracle。 | 非 90°、加密、损坏、超限等 typed-refuse；无部分文件。两个动作分别裁决。 |
| **26-D：原生 GUI 与出口** | M24 Sico 原生 UI 库具有 PDF 工具所需的真实 Windows 控件证据；以版本化依赖构建/运行 PDF GUI，做打开/保存、页面/元数据、合并/旋转及拒绝路径。M7 安装消费、M0–M25 回归、M26 审计另行裁决。 | 缺控件先回 24-C；M25 后补 UI 用**新版本**和对 v1.0 快照的兼容检查，不能回改冻结的 M25 结论。PDF GUI 只显示结构数据，不宣称页像素渲染。 |

## 6. 最终审查记录格式

每个实施 STEP 末尾至少填下面六行；缺数据写“未运行/未测得”，不得留空或推断：

```text
卡号 / 当前 STEP / 入口门和接受合同：
改动范围 / 冻结语料清单与 SHA / 真实 runner 和平台：
前值 → 后值（canary、拒绝前沿、字节差分、R3 连续计数，如适用）：
执行命令、退出码与原始结果 / 未执行的 CI 或外部项：
各 gate 的 GO、NO-GO 或待裁决 / evidence class：
下一张允许领取的卡 / 尚缺的确切输入：
```
