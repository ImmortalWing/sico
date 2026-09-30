# M22–M26 完成计划 v1（全程执行队列）

> - 状态：registered by STEP-0323（owner directive 2026-09-30 「完整完成M22-M26，规划至进程」）；planning-only——不改任何 gate 文本与证据等级，仅排序与登记
> - 本指令同时构成以下 draft/proposed 合同的接受裁决（owner 可在任何实现 STEP 前复审修订具体条款，修订以新草案+复裁决登记）：RFC-0048、RFC-0049、RFC-0050、RFC-0051、RFC-0052、ADR-0018、RFC-0053、RFC-0054
> - 纪律不变：每个工作项执行时分配唯一 STEP + 可复现 validator + 证据 + STATUS/ROADMAP/plan 同步；不预留未来 STEP 号；外部门绝不伪造成完成

## 1. 执行拓扑

```
M22 降低→S5→S6→S7 ────────────┐（关键路径，串行手术）
M24 视觉/UI/codec（并行轨，即刻可开工）│
                                v
                M23 batch 3（Route A：M22 S7 GO 后；
                            Route B：R3 止损后即刻）
                                v
        M25 v1.0（需 M23 GO + M24 转换器 GO + M19 release 门）
                                v
                    M26 PDF 包 + GUI（需 M25 release GO）
```

## 2. M22 完成分解（当前：parser 11/115、compiler_parser 3/32、S5 8 族、R3=0）

- **A1 降低族**（顺序即队列）：① user-call RHS/参数位族（解锁 matching_close/
  find_body_end/count_params/fn_entry/constant_argument_kind 等 7+）；
  ② text.encode/text.length/bytes.at/bytes.length/split_lines let-RHS 宿主调用族
  （解锁 is_digit_token/is_string_token/json_string_token/has_return_at 等 6+）；
  ③ lex_words 组合（A1①② 就位后探针推进）；④ 逐函数覆盖驱动至 parser 115/115
  （Flash 普查清单 + 每函数探针先行）；⑤ compiler_parser 32/32、lexer 4、
  driver 2、checker 1。退出测试：全部 selfhost 源**全源**差分字节全等（非子集）。
- **B S5 扩面**：checked_mul/div（oracle 探针切体法，模板已验证）→ 其余
  RFC-0011 冻结子集族 → 37 可构建 artifact 全部字节全等、178 拒绝逐项对齐。
- **C S6 自举**：C1 冻结语料 A==B==C；C2 guest 自编译；C3 `.sapp` 经 M7 链 +
  hostile/limit+1 语料；C4 预算实测（wall/fuel/峰值内存/hostcalls；栈高水位
  依赖 runner CLI 缺口——若不可测，如实登记 unavailable，不推算）。
- **D S7**：全量 M0–M21 回归 + 独立 CI + 七门复审。**独立 CI 依赖 GitHub
  可达性（当前主机不可达）——STEP-0316 起全部快照已在 gitcode，可达后按
  exact-SHA 逐个补裁决；CI 不可达本身不阻塞本地实现，但阻塞正式 GO。**

## 3. M23（M22 裁决后）

按已接受 RFC-0048（中缀比较/逻辑）与 RFC-0049（let 绑定 if/match）实现，
逐项走 RFC-0033 门（显式反糖、source-map 身份、格式化幂等、typed 诊断、
e2e byte-exact 语料 + limit+1 拒绝）。item 3 裸字面量复称：先内部语料测量，
live-model 复测保持 owner 凭据门（不冒充）；text.chars 按消费者证据决定。
实现后重跑 selfhost S1/S2 差分（re-baseline）+ 出口审计。

## 4. M24（并行轨，即刻开工）

RFC-0050/0051/0052 + ADR-0018 已按指令接受。顺序：codec 包（RFC-0050 分片）
→ 原生 UI 库（源绑定 RFC-0052 → 渲染器 ADR-0018 → 帧语料 → 控件）→
JPEG↔PNG 转换器试点（过 §3 gate 5 后触发 M18 组合项 4 增补审计）→
视觉 roster 包（template match/grid/contour，确定性 CV）。gate 2（具名硬件
加速对照）保持 owner 外部门：计划将其推至"reference-only 对照就绪、仅缺
真实加速器实测"的最后可完成点，M17 整体 NO-GO 如实保持。

## 5. M25（M23 + M24 转换器 GO 后）

release-candidate CI 全绿 → 两次独立构建字节相同 bundle → 干净 Windows
install/run/upgrade/uninstall 签名演练 → registry 对 v1.0 bundle 发布演练 →
§13 逐项审计以 M21–M24 全证据链重发。平台广度按实际提供 runner 声明
（当前仅 Windows x64 有真实证据；macOS/Linux/Android 为显式延期）。
发布 GO/NO-GO 与项目 §13 完成裁决分别发布。

## 6. M26（M25 release GO 后）

PDF 结构化阅读包（RFC-0053 分片：对象模型→xref→对象流→页树）→
限额/拒绝/写入器（RFC-0054）→ 合并/旋转冻结语料 byte-stable →
基于 M24 UI 库的 GUI PDF 工具 → 审计。

## 7. 外部门诚实表（代码无法真实完成、只能推到最后可完成点并登记）

| 门 | 所属 | 缺失输入 | 计划处置 |
|---|---|---|---|
| 独立 CI 裁决 | M22 每实现 STEP | GitHub 可达性 | gitcode 快照就绪，可达后 exact-SHA 补裁决 |
| M17 gate 2 加速对照 | M24 | 具名加速硬件/provider | reference-only 就绪 + 延期登记 |
| macOS/Linux/Android runner | M25 平台广度 | owner 供给的 runner/设备 | 无 runner 处 contract-verified/延期 |
| live-model 复测 | M23 item 3 AI 噪音主张 | owner 凭据 | 内部语料测量 + 主张保持门控 |
| 公网部署 | M7 延续（不在 M22–M26 内） | 域名/TLS/身份 | 不涉 |

## 8. 执行经济

验证环 1.8s/探针（STEP-0317）；parser.sico 手术串行（拼接教训）；
Flash 子代理（owner 指定 GLM-5.3-Flash）承担只读普查/规格准备/文档草拟，
字节级手术与裁决留主线；每完成一个工作项即提交推送 gitcode。
