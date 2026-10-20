# STEP-0289: M24 UI 合同起草——RFC-0052 + ADR-0018（卡 24-A 合同面收口）

> - status: complete / RFC-0052 draft + ADR-0018 proposed registered; no implementation, no gate moved
> - phase: M24 §8.6.1 prerequisite workstream (execution card 24-A contract surface, final draftable pair)
> - date: 2026-09-25
> - evidence class: contract drafting over the STEP-0287 inventory and accepted contracts (RFC-0039, RFC-0041)

## 1. Scope and decision

收口卡 **24-A** 的合同面：起草 §8.6.1 要求的最后一对前置合同。

1. **源侧 [`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)
   （draft）**：Sico 语言原生 UI 库的编译器面向词汇表 =
   **`sico:user/ui@1` 版本化包**——window/column/row/text/button/
   dropdown/image_preview/progress 构树函数 + set_text/set_progress/
   draw_image + poll_event/run。**零语法增长**（无新关键字/词法/解析
   变更）；image_preview 直接吃 RFC-0041 BGRA8 Bytes（与
   image-vision@1、image-codec@1 同一数据面组合）；文件对话框**不是**
   包函数——组合既有 least-privilege file 能力，UI 授权与文件授权
   分离；封闭 `UiError`（unknown-handle/wrong-parent-kind/limit+1/
   no-window/event-overflow/authority-denied 默认拒绝）。
2. **渲染侧 [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)
   （proposed）**：渲染栈 = **Direct2D+DirectWrite 自绘 Fluent token
   子集**，明确否决 WinUI3/XAML 运行时（外部重分发、合成动画破坏帧
   确定性）、web/webview（owner 排除）、M5 companion 扩展（无编译器
   面向绑定）。平台 unsafe 收进最小隔离 adapter crate（AGENTS §4）；
   帧确定性契约（固定 client 尺寸/96-DPI、钉死配色、无 timer/动画，
   帧 = UI 状态纯函数）；UI 能力版本化 default-deny，不含 capture/
   input/剪贴板/凭据/进程控制（M16 边界不动）。"WinUI 3-like" 钉死为
   token 子集契约（4px 圆角、accent+浅色 ramp、Segoe UI Variable、
   Fluent 控件几何），不主张像素级 parity。

## 2. Grounding facts

- §8.6.1 义务逐条映射：source 侧经 RFC-0039 user-WIT 面暴露（零新
  trust path）；host 侧 ADR 先行；确定性契约原文照抄收严；M26 PDF
  工具的控制需求（page list/metadata/rotate/merge/progress）标注为
  M26 开工盘点对本面的测量点。
- sico-ui-v0.wit（M5 companion）与 M15 Web GO 状态均不改、不替代
  （card 24-C 停手规则）。

## 3. Discipline

- RFC draft + ADR proposed：两者都被 owner 接受之前不开任何 UI 实现
  STEP；实现 STEP 另行分配编号，并须在真实 Windows 原生 renderer 上
  交付帧语料字节稳定 + 事件序列 + authority 拒绝证据。
- 本 STEP 不计 R3（合同起草）；M22/M23/M25/M26 不受影响。

## 4. Executable evidence

- 无 Cargo 工作区/runner 变更；`git diff --check` 与
  `validate-step-0124.ps1 -SelfTest` 通过；adr/README 与 rfc/README
  各登记一条（下一可用编号 ADR-0019 / RFC-0053）。

## 5. Gate accounting

无 gate 变化。M24 = planned（**四份前置合同现已全部起草完毕**：加速
ADR 的证据仍外部卡死，其余三对均 draft/proposed 待 owner 接受）；M17
= NO-GO 不变。owner 待决现为五项：W1 裁决方式；RFC-0048/0049；
RFC-0050/0051；RFC-0052 + ADR-0018；（可选）M17 加速证据主机命名。
免输入工作至此全部收口。
