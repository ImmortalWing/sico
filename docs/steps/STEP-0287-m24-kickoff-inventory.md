# STEP-0287: M24 开工盘点（执行卡 24-A）

> - status: complete / inventory frozen; no contract drafted, no gate moved, M24 stays planned
> - phase: M24 vision closure and GUI application pilot — kickoff inventory (execution card 24-A)
> - date: 2026-09-25
> - evidence class: internal-fixture, deterministic tree scan

## 1. Scope and decision

执行 [M22–M26 execution cards](../plans/M22-M26-execution-cards.md) 卡
**24-A**：按 [M24 计划 §7](../plans/M24-vision-closure-gui-application-pilot.md)
完成五条线的开工盘点（STEP-0129/0142 模式），冻结
[`m24-inventory-2026-09-25.json`](../reports/m24-inventory-2026-09-25.json)
（schema `sico.m24.inventory.v0`，无时间戳、可复算）。纯盘点，不起草
合同、不实现能力、不计 R3。

## 2. Inventory results（五条线，现有 vs 缺失）

| 线 | 现有 | 缺失 | 合同家 |
|---|---|---|---|
| M17 gate 2 加速证据主机 | 仅确定性参考路径（image-vision@1 纯函数）；全工作区 0 处 accelerator/GPU 引用 | **未命名证据主机**——owner 须指定设备/provider；缺输入则 gate 2 按 §8.1 延期条款保持 NO-GO | 加速 provider ADR（§8.1） |
| M17 gate 4 模型资产 | 0 个模型文件（.onnx/.tflite/.safetensors 扫描）；RFC-0041 accepted、RFC-0043 仍 draft | 资产 manifest schema 实现；§8.2 fixture 证据集（digest/typed 拒绝/预算/取消/恶意资产）零实现 | provenance RFC（§8.2） |
| Roster 包 | `sico:user/image-vision@1`（`package_builder.rs:2003`）：to-grey8/threshold/occupancy/occupancy-mask 四个纯函数，BGRA8，无 codec I/O（诚实缺口） | template match/grid/contour 三包**零代码**；`tetris_vision_chain.rs` 与 `pilots/tetris-vision/` 为历史证据（试点已删），不是队列 | RFC-0043 转正 + 各包语料（§8.3） |
| 原生 UI 面 | M15 声明域内：`sico-ui-v0.wit`（七种 node、render/next-event）+ 浏览器矩阵语料（RFC-0042/STEP-0156/0162）+ M16 console 证据 | **Sico 语言原生 UI 库整体不存在**：source binding RFC、renderer ADR、Windows 原生 renderer、确定性帧语料均为零；web/webview/M15 binding 不能算（卡 24-C 停手规则） | §8.6.1 workstream（RFC+ADR 先行） |
| JPEG↔PNG codec | 全工作区（root/crates/runner）0 个 codec 依赖，无解码/编码能力；§8.5 拒绝语料合同已逐项列举但零文件 | codec RFC 未起草——接受 profile、输出参数、确定性保证（PNG byte-stable、JPEG 编码参数入册）未冻结 | codec RFC（§8.4 第一步先行） |

## 3. Reading of the inventory（不含决定）

- M24 的**实现前置合同共四份**：加速 ADR（owner 门控输入）、
  provenance RFC、UI source-binding RFC + renderer ADR、codec RFC——
  全部可起草，其中只有加速 ADR 的**证据**被外部输入卡死；其余三份的
  起草不依赖外部。
- image-vision@1 与 M15 WIT 是唯二可复用的已验收面；两者都**不**等于
  M24 目标能力（roster 扩展、原生 UI 库、codec）。
- 转换器试点（§8.4）的四个关注点里，"conversion engine" 与
  "presentation" 都尚未有合同——试点不能早于这两份 RFC/ADR。

## 4. Executable evidence

- 直接扫描当前树：`grep` accelerator/gpu 引用（0）、模型资产扩展名
  （0）、codec 依赖（0）、roster 函数名（0）；`image_vision_package`
  函数清单直读源码。全部计数写入 JSON，无时间戳。
- 无 Cargo 工作区/runner 变更；`git diff --check` 与
  `validate-step-0124.ps1 -SelfTest` 通过。

## 5. Gate accounting

无 gate 变化。M24 = planned（盘点完成，四份合同待起草；gate 2 保持
延期直至 owner 命名证据主机）；M17 = NO-GO 不变；M22/M23 不受影响；
本 STEP 不计 R3。下一张可领取的卡：四份合同中任一份的起草（各自
RFC/ADR），或等待 owner 对 W1/RFC-0048/0049 的待决输入。
