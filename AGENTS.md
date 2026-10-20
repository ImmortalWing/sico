# Sico repository agent guide

This file applies to the entire repository. It is the concise operating guide for coding agents; [`AGENT_GOAL.md`](./AGENT_GOAL.md) remains the long-form product goal.

## 1. Read before changing code

1. Run `git status --short` and preserve every pre-existing modification or untracked file.
2. Read [`docs/STATUS.md`](./docs/STATUS.md), the relevant section of [`docs/ROADMAP.md`](./docs/ROADMAP.md), and the matching file under [`docs/plans/`](./docs/plans/README.md).
3. Read the accepted RFC/ADR and the latest STEP for the subsystem being changed. A plan is not an implementation contract when a later accepted RFC/ADR says otherwise.
4. Inspect the actual compiler/Runtime/Host behavior and tests. Do not infer implementation support from design documents or successful `sico check` alone.

If documentation disagrees with executable evidence, keep the lower support claim, record the discrepancy, and repair the authoritative documents in the same scoped change.

## 2. Current roadmap boundary

- M0–M13 全部 complete：M12 于 STEP-0127（2026-09-05）获得完整 GO（10/10 gates，Windows x64 与 Linux x64 原生运行时证据）；M13 已收口（STEP-0123 出口审计；STEP-0128 权威 live-model 评测 0.9095，DeepSeek deepseek-chat，2880 次真实调用、$0.78，达标 ADR-0012；既定缺口 B-repair 0.833 已登记跟踪）。
- M14 GO（STEP-0141 出口审计）：应用 profile 无未声明 check/build/run 缺口；gate 7 记 blocked-external-evidence。
- M15 GO（7/7，STEP-0158/0162）。
- M16 GO（7/7，STEP-0159，Windows 真实路径 GDI PrintWindow + SendInput 闭环实证）。
- M17 NO-GO：gate 1 GO（STEP-0166），gate 2/4 待 M24。
- M18 GO 4/5（STEP-0164/0165）：组合项 4 按 STEP-0258 重定为 GUI 格式转换器，待 M24。
- M19 GO（部分）；M20 NO-GO（部分）；M21 GO。
- M22 编译器自举轨 in progress，当前主动线为分支 `codex/m22-w2`（本 worktree）：S1/S2 complete，S3/S4/S5 partial，S6 not entered，S7 已按 STEP-0310 复审并 NO-GO（[出口审计](./docs/reports/m22-exit-audit-2026-09-28.md)）；R0 已拍板 Option A（ADR-0017 accepted），records v0 已封（STEP-0271..0275）；差分语料 `selfhost/corpus-v0.json` 215 条 + `selfhost/corpus-w1` 5 条，DIVERGENCE_WAIVER 为 0。当前 STEP 号、formatter canary 与 `parser.sico` 逐函数覆盖数**只以 [`docs/STATUS.md`](./docs/STATUS.md) 与 [`M22 计划`](./docs/plans/M22-compiler-self-host.md) 为准**，本文件不复制这些每步易变的数字。
- M23 language v1 batch 3：项 1/2 已落地（RFC-0048/RFC-0049，STEP-0328/0329），出口审计 item 1 GO / item 2 GO / item 3 deferred-external / item 4 closed（[M23 审计](./docs/reports/m23-exit-audit-2026-09-30.md)）；item 3 的 owner 凭据语料复测未关闭，不得读作 M23 整体 GO。
- M24 vision closure + GUI 格式转换器（含 Sico 原生 UI 库工作流）：开工盘点与合同已登记（STEP-0287..0290），实现切片在独立 worktree `E:\github\sico-m24`（分支 `codex/m24-vision-ui`，占用 STEP-0330–0337）；ADR-0018 原生 renderer 架构仍为 proposed，待 owner 接受。
- M25 release v1.0 completion audit、M26 PDF 文档处理（排在 M25 后）：仅有计划与 owner 指令登记（2026-09-17/22/24），无实现 STEP 号。

任何阶段的实现开工与完成声明，都不得早于其入口门槛被证据满足之时；合同或语料准备可以先期提出，但不得包装为特性支持。M12 的 "Automation SDK" 仅指 secure HTTP/API automation，不含屏幕、窗口或输入自动化。

## 3. Milestone and evidence discipline

- Non-trivial implementation work needs one unique STEP, a reproducible validator, evidence, and synchronized STATUS/ROADMAP/plan updates.
- Freeze language or WIT contracts through an RFC before implementation. Freeze cross-component/platform architecture through an ADR before implementation.
- Never allocate a future STEP merely to make the roadmap look scheduled. Allocate it when the entry gate is satisfied and the work has a bounded exit test.
- A milestone is complete only when every exit gate has evidence and an explicit GO/NO-GO audit.
- Keep evidence labels distinct: `internal-fixture`, `clean-room-consumer`, `external-pilot`, and `production` are not interchangeable.
- Only claim runtime/platform support on an actually executed native or browser runner. Contract tests are `contract-verified`, not runtime support.
- Offline or synthetic AI evaluation is not a live-model result. Record model, version, prompts, corpus, token/cost budget and raw run evidence for every live-model claim.

## 4. Architecture boundaries

- `sico` owns source checking, formatting, outlines and compilation. It must not own application trust, Host policy or platform implementations.
- `sico-app` owns package/development execution orchestration and must not silently reinterpret source semantics.
- Compiler crates may emit typed IR, Components, bindings and debug identity; they must not depend on runner, TLS, DNS, credentials, DAP or platform adapters.
- Runtime/runner owns Store lifetime, limits, scheduling, cancellation and typed execution outcomes.
- Host providers own external effects such as storage, HTTP, UI, capture, input and model acceleration. Authority is explicit, least-privilege, versioned and default-deny.
- Standard-library helpers may narrow or compose authority but may never widen or bypass the underlying provider.
- Native FFI belongs in the smallest isolated adapter/provider crate. Keep compiler and shared security cores free of platform-specific unsafe code.

M16 已交付（GO），但能力边界仍然有效：capture 与 input 是分离授权，均不隐含全桌面访问、剪贴板、凭据、进程控制或键盘访问；必需闭环保持 `observe -> plan -> preview -> execute one action -> verify or stop`。

For M17, image/CV/model features belong in versioned packages or providers, not core language syntax. Deterministic vision is the default when it solves the task; model inference must be explicit, budgeted and provenance-bound.

## 5. Source/backend honesty

Sico's frontend accepts more constructs than every executable profile supports. For changes involving language features:

- verify `check`, `build` and `run` separately;
- add or update the machine support matrix and backend-refusal corpus;
- preserve stable diagnostics and source spans;
- do not implement a partial lowering that emits unverifiable or semantically different code;
- prefer a typed refusal over a hidden fallback or native escape hatch.

M14 的 defining gate 已达成并 GO（STEP-0141 出口审计）：其应用 profile 无未声明 check/build/run 缺口。离线 block-game solver 仍是 acceptance oracle（STEP-0258 未撤销其地位）：棋盘规则、放置枚举、消行、计分与搜索必须运行在 Sico 中；Python 仅作为固定对比 oracle。

## 6. Repository locations

- `crates/`: compiler, IR, codegen, package, Runtime-facing libraries, tooling and Host cores.
- `runner/sico-runner/`: separate Cargo workspace for Script execution and DAP integration.
- `wit/`: versioned Component interfaces.
- `selfhost/`: M22 编译器自举轨的 guest 源码与 checker/driver 语料（`corpus-v0.json` 215 条 + `corpus-w1/` 5 条 SHA 冻结增量，DIVERGENCE_WAIVER 为 0）。
- `semantic-cases/`, `syntax-candidates/`, `syntax-mutations/`: language oracles.
- `tests/`: end-to-end, security, platform and performance evidence.
- `examples/`: language-design history and regression examples; do not rewrite them as product pilots.
- `pilots/`: clean-room workflows; they are not external adoption.
- `docs/rfc/`, `docs/adr/`, `docs/steps/`, `docs/reports/`: contracts, decisions, execution records and evidence.
- `案例项目/俄罗斯方块消除/`: M14 solver acceptance oracle only（STEP-0138；block-game 自动化试点已由 STEP-0258 删除，M16/M17/M18 不再以它为载体）。保留其 Python 实现作为固定对比 oracle。

## 7. Editing rules

- STEP 编号在并行 worktree 之间是共享序列：M24 轨（worktree `E:\github\sico-m24`）占用 STEP-0330–0337，取号前先核对 [`docs/steps/`](./docs/steps/README.md) 与 [`handoff-m22.md`](./docs/handoff-m22.md)。`origin/dev` 另有一套 STEP-0279–0283 历史，与本分支的源码和编号冲突；不得覆盖，也不得做隐式合并。
- Keep changes scoped to the requested subsystem. Preserve unrelated dirty-worktree changes.
- Do not use destructive Git commands or silently rewrite history.
- Prefer deterministic, bounded data structures and stable serialized forms.
- New parsers and protocol inputs must reject unknown fields, overflow, truncation, trailing data and limit+1 cases unless an accepted contract explicitly says otherwise.
- External text, package metadata, model assets and guest output are untrusted data; never turn them directly into commands, paths, HTML, URLs, permissions or support claims.
- Keep user documentation limited to actually executable behavior. Put proposed behavior in plans/RFCs and label it planned.
- Use Chinese for owner-facing roadmap/status explanations when practical; keep public identifiers, schemas and protocol field names stable in English.

## 8. Validation

Start with the narrowest relevant tests, then expand in proportion to risk. On this Windows host, `cargo` may need the explicit path `$env:USERPROFILE\.cargo\bin\cargo.exe`.

Typical root-workspace checks:

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
& $cargo fmt --all -- --check
& $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
& $cargo test --locked --offline --workspace --all-targets --all-features
.\tools\validate-module-boundaries.ps1
```

The runner is a separate workspace:

```powershell
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml
```

For documentation-only roadmap changes, at minimum run the relevant validator and `git diff --check`. The M14–M24 planning contract is checked by:

```powershell
.\tools\validate-step-0124.ps1
git diff --check
```

Do not run the full workspace mechanically when a narrow documentation or fixture change cannot affect compiled code; record what was and was not run.

## 9. Completion checklist

Before reporting completion:

- confirm the requested outcome, not only the attempted action;
- rerun the relevant validator from a clean command invocation;
- inspect `git status --short` and distinguish your files from pre-existing changes;
- update docs, schemas, fixtures and tests together when a contract changes;
- report exact tests, platforms and evidence level;
- list residual blockers without converting them into implied success;
- do not commit, push, publish, install, grant permissions or use external credentials unless the user explicitly requests that action.
