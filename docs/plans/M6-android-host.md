# M6 plan: Sico Android Host

> - status: ready after M5 GO
> - created: 2026-07-16
> - phase: M6
> - entry evidence: [`M5 exit audit`](../reports/m5-exit-audit.md)
> - execution boundary: M5 portable app/UI/permission/lifecycle contracts

## 1. Outcome

让同一份 development-signed `.sapp` 无需重编译即可经 Android 文件、链接或分享入口进入 Sico Android Host，并在 Android lifecycle、permission、storage、touch/IME/accessibility 边界内保持与 Desktop Host 一致的 trust/identity/capability 行为。

## 2. Entry and decision gates

- 不把 M0 的 Android 可行性报告冒充真机证据；第一步重新冻结 threat/lifecycle/packaging contract；
- shared Rust core 仍是 package/trust/identity/permission 的唯一权威，Kotlin/JNI 只做平台翻译；
- `.sapp` bytes、URI、Intent extra、文件名与 guest UI text 不得成为 shell/HTML/未验证路径；
- Android Runtime backend、ABI、Pulley/Wasmtime availability 必须以当前版本和真实 emulator/device 证据决定；
- 无真实 runner 时允许完成纯 Rust/JNI contract 工作，但不得把 M6 标记 complete。

## 3. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0054 | Android threat/lifecycle/platform/packaging contract | Intent/URI/provider、reinstall/update、process death、permission phishing、ABI/package matrix；ADR/RFC accepted |
| STEP-0055 | shared mobile host core and JNI boundary | Rust core reuse、typed JNI calls、byte/URI ownership、panic/error mapping、host-side unit/property corpus |
| STEP-0056 | Android package/open/share/deep-link adapter | SAF/content URI copy-by-digest、MIME/intent filters、share/open-with/deep-link refusal corpus |
| STEP-0057 | Android permissions and isolated storage mapping | capability closure to runtime permissions/SAF scopes、app+signer storage、record invalidation/re-prompt |
| STEP-0058 | Android Runtime and lifecycle supervision | selected ABI/backend probe、Activity/process lifecycle、timeout/cancel/crash recovery、no orphan work |
| STEP-0059 | touch/input/IME/accessibility UI adapter | typed UI nodes/events mapped to native Android controls；focus、IME、TalkBack、rate/size ceilings |
| STEP-0060 | same `.sapp` desktop/Android parity app | one signed package on Windows and Android runner with identical identity/result/permission behavior |
| STEP-0061 | M6 quality/performance and exit audit | security/property corpus、device/emulator matrix、startup baseline、M0–M5 regression、M7 plan |

## 4. M6 exit gate

- same exact `.sapp` digest runs on verified Desktop and Android runners without recompilation；
- file/share/deep-link entry always copies and reverifies bytes before execution；
- Android permissions and storage cannot be inherited by app ID spoof, signer change or capability drift；
- background/process death/timeout/cancel/crash do not bypass cleanup or terminate the Host UI process；
- touch/IME/accessibility and typed event ceilings have native evidence；
- M0–M5 regression remains green and unavailable ABI/device gaps are explicit。

## 5. Immediate next step

`STEP-0054`：审计当前 Android toolchain/runner/ABI 和 Wasmtime/Pulley availability，冻结 Intent/URI/provider、identity/storage、Activity/process lifecycle 与 packaging threat contract；在该 gate 前不创建会接受任意外部 `.sapp` 的 Activity。

