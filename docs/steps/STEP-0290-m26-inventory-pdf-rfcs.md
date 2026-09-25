# STEP-0290: M26 开工盘点 + PDF 数据/限额 RFC 起草（执行卡 26-A）

> - status: complete / inventory frozen; RFC-0053/0054 registered as **draft**; no implementation, no gate moved
> - phase: M26 PDF document processing — kickoff inventory and contracts (execution card 26-A)
> - date: 2026-09-25
> - evidence class: internal-fixture, deterministic scans over the pinned toolchain

## 1. Scope and decision

执行卡 **26-A**（路线重划明确允许 M25 前先行盘点与 RFC 起草；实现仍等
M25 v1.0 release GO）。冻结盘点
[`m26-inventory-2026-09-25.json`](../reports/m26-inventory-2026-09-25.json)
（schema `sico.m26.inventory.v0`），并起草两份合同草案。

## 2. Inventory results（五条线）

1. **byte/text/List/Map 面对解析需求的适配**：`sico.bytes.length/at`
   （parser.sico:71-82 在用）覆盖 header/xref/对象头的字节扫描；
   dict/array 结构映射 Map/List；**v0 classic-xref 阅读子集无语言缺口**。
2. **语料生成器与 oracle**：生成器 = 仓内确定性脚本（仅 stdlib，钉在
   ensure-python 的 CPython 3.12.10）产 classic-xref 未压缩对象 PDF。
   **缺口（owner 门控）**：无独立 PDF oracle——pikepdf 为命名候选，
   未安装；到位前 oracle = 生成器回环 + 人工核对，证据级
   internal-fixture，不作互操作主张。
3. **inflate 候选**：全仓 0 处 deflate/inflate 引用；pure-Sico 候选与
   provider 候选均未实现，微基准**不可测**（诚实登记），filters 决策
   推迟到对应 slice 实测。
4. **M24 UI 控件缺口**：PDF 工具五需求中 page list/metadata/rotate/
   merge/progress/dialogs 均可由 RFC-0052 面组合；**命名缺口 = 滚动/
   视口**（RFC-0052 v0 无），缺口归宿 M24 §8.6.1（新版本）或 v0 用
   dropdown 限定页选择，M26 GUI gate 时定，无 Web 回退。
5. **阅读子集钉定**：v0 = PDF 1.4–1.7 classic xref table + 未压缩对象，
   stream 带 /Filter 一律 typed 拒绝、加密拒绝；xref stream/object
   stream/incremental update 各自扩展语料另列。

## 3. Contracts drafted (both **draft**)

- **[`RFC-0053`](../rfc/RFC-0053-pdf-structural-reading-v0.md)**：
  `sico:user/pdf@1` `open()` 结构阅读—— PdfDocument（版本/页数/对象数/
  页几何+继承 /Rotate/trailer root）；接受子集按名冻结；任何不支持的
  构造 = 单一 typed 拒绝。
- **[`RFC-0054`](../rfc/RFC-0054-pdf-limits-refusals-writer-v0.md)**：
  八项限额先检后分配（+1 typed）；封闭 `PdfError` 十四码带字节 span；
  merge/rotate 后续 slice 的字节级稳定 writer 契约（classic xref、
  规范重编号、幂等探测）。

## 4. Discipline

- 两份 draft：owner 接受前不实现；**且**实现另等 M25 v1.0 release GO
  （路线重划 M26 门）。本 STEP 不计 R3。
- oracle 缺口如实保留：不把生成器回环写成独立验证。

## 5. Executable evidence

- 库可用性扫描（pikepdf/PyPDF2/pypdf/fitz/reportlab 全无、zlib 有）、
  inflate 引用扫描（0）、bytes 面直读源码——全部入 JSON，无时间戳。
- 无 Cargo 工作区/runner 变更；`git diff --check` 与
  `validate-step-0124.ps1 -SelfTest` 通过。

## 6. Gate accounting

无 gate 变化。M26 = planned（盘点完成、两 RFC draft；实现双门 = owner
接受 + M25 GO）。至此**全部里程碑的免输入开工/测量/合同工作均已
收口**；剩余工作全部等 owner 决策（W1 裁决、五组 RFC/ADR 接受、
oracle 与加速主机命名）。
