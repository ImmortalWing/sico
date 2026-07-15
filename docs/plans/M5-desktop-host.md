# M5 plan: Sico Desktop Host

> - status: ready after M4 GO
> - created: 2026-07-16
> - phase: M5
> - entry evidence: [`M4 exit audit`](../reports/m4-exit-audit.md)
> - execution boundary: verified/authorized/limited `.sapp` Runtime v0

## 1. Outcome

把 M4 的安全 package Runtime 封装为可安装、可打开、可授权、可恢复的 Sico Desktop Host，并在 Windows 上交付真实 shell/file-open 流程，同时冻结 macOS/Linux 的 core behavior adapter contract。M5 包含 host lifecycle、permission UI、最小 UI WIT/SDK 和一个真实 `.sapp`；不包含 Android、公共 registry、production publisher identity 或自动更新。

## 2. Entry and decision gates

- M4 exit gate、package threat parser、trust/capability/storage/limit/CLI regression 必须持续通过；
- Desktop Host 只能消费 `AuthorizedPackage`，不得复制宽松 package parser 或绕过 trust policy；
- install/open association、permission record、storage identity、crash recovery、window/webview/native UI backend 必须先有 threat/lifecycle/platform RFC/ADR；
- package 内容和 guest output 不得成为 shell command、host path 或 UI HTML 的未转义输入；
- 平台暂不可用时记录真实缺口，Windows 证据不得冒充 macOS/Linux 已验证。

## 3. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0046 | Desktop Host threat model、lifecycle/platform contract | install/open/association、TOCTOU、permission phishing、window/storage identity、crash/cleanup、single/multi-instance matrix；RFC/ADR 接受 |
| STEP-0047 | shared host core 与 install/open pipeline | `.sapp` copy-by-digest、strict reverify、atomic install metadata、open by immutable identity；tamper/stale association refusal |
| STEP-0048 | permission UI 与 durable decision records | manifest/import/host grant review、allow-once/deny/persistent scope、record versioning；unknown/new capability re-prompt |
| STEP-0049 | lifecycle、process supervision 与 crash isolation | start/foreground/background/close/cancel/crash state machine；guest kill/timeout 不退出 host；storage/temp cleanup |
| STEP-0050 | minimal UI WIT/SDK and renderer boundary | deterministic UI model/events、escaping、size/rate limits、accessibility baseline；malicious UI payload refusal |
| STEP-0051 | Windows Desktop Host integration | real `.sapp` double-click/open-with/install/uninstall、icon/window、permission dialog、crash recovery；Windows packaging smoke |
| STEP-0052 | macOS/Linux adapters and parity corpus | shared core compile + platform adapter contract；available CI/runner evidence，缺失平台明确 not-verified |
| STEP-0053 | real desktop app、quality/performance 与 M5 exit audit | representative app、security/lifecycle property corpus、M0–M4 regression、non-SLA startup baseline、M6 plan |

## 4. Test matrix

- install/open：partial copy、same ID different signer、same signer downgrade、tampered installed bytes、association spoof、concurrent install/open；
- permissions：missing/extra/unknown capability、record corruption/version drift、allow-once expiry、denied launch no side effects；
- lifecycle：rapid reopen/close、timeout/cancel race、guest crash, host restart、orphan process/temp/storage audit；
- UI：oversized/deep model、event flood、HTML/path/control-character injection、focus/accessibility order；
- platforms：shared package digest/identity/permission/fault corpus，platform-specific shell behavior 单独记录；
- regression：M4 `.sapp` deterministic/trust/capability/storage/limit gates 与 M3 raw compiler boundary。

## 5. M5 exit gate

- Windows Desktop Host 能从 shell/open-with 安装并运行受信 `.sapp`，每次 open 前以 immutable digest reverify；
- permission UI 展示实际 capability closure，durable record 不会因 app ID spoof、signer change 或 manifest change 被复用；
- guest timeout/trap/crash/cancel 不崩溃 Host，不遗留越权 process/temp/storage；
- minimal UI WIT/SDK 对输入、事件、资源和 escaping 有真实恶意 corpus；
- Windows 真实流程通过；macOS/Linux 只在有真实 runner 时声明通过，否则以 adapter contract + 明确缺口进入后续；
- real app、security/property、startup non-SLA baseline、M0–M4 regression 全绿，并形成 M6 entry plan。

## 6. Immediate next step

`STEP-0046`：先建立 Desktop Host threat/lifecycle/platform matrix，冻结 install/open immutable identity、permission record key、single/multi-instance、crash/cancel cleanup 与 Windows/macOS/Linux 证据边界；该 gate 通过前不实现会接受任意 `.sapp` 的 GUI Host。
