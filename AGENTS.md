# Sico repository agent guide

This file applies to the entire repository. It is the concise operating guide for coding agents; [`AGENT_GOAL.md`](./AGENT_GOAL.md) remains the long-form product goal.

## 1. Read before changing code

1. Run `git status --short` and preserve every pre-existing modification or untracked file.
2. Read [`docs/STATUS.md`](./docs/STATUS.md), the relevant section of [`docs/ROADMAP.md`](./docs/ROADMAP.md), and the matching file under [`docs/plans/`](./docs/plans/README.md).
3. Read the accepted RFC/ADR and the latest STEP for the subsystem being changed. A plan is not an implementation contract when a later accepted RFC/ADR says otherwise.
4. Inspect the actual compiler/Runtime/Host behavior and tests. Do not infer implementation support from design documents or successful `sico check` alone.

If documentation disagrees with executable evidence, keep the lower support claim, record the discrepancy, and repair the authoritative documents in the same scoped change.

## 2. Current roadmap boundary

- M10 and M11 are complete.
- M12 Secure HTTP Provider is `GO-core`; guest-visible runner integration and Linux provider parity must close before full GO.
- M13 is a parallel AI-tooling support track; live-model claims remain externally gated.
- M14–M18 are planned and have no implementation STEP numbers reserved:
  - M14: application-ready language baseline.
  - M15: Web platform and UI controls.
  - M16: Native Automation Host.
  - M17: vision and model package ecosystem.
  - M18: representative AI applications and external pilots.

Do not begin M14 implementation before M12 full GO and the M13 closure verdict. Contract or corpus preparation may be proposed earlier, but it must not be presented as feature support. Do not fold screen/window/input automation into M12; “Automation SDK” in M12 means secure HTTP/API automation.

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

For M16, capture and input are separate capabilities. Neither implies full-desktop access, clipboard, credentials, process control or keyboard access. The required loop is `observe -> plan -> preview -> execute one action -> verify or stop`.

For M17, image/CV/model features belong in versioned packages or providers, not core language syntax. Deterministic vision is the default when it solves the task; model inference must be explicit, budgeted and provenance-bound.

## 5. Source/backend honesty

Sico's frontend accepts more constructs than every executable profile supports. For changes involving language features:

- verify `check`, `build` and `run` separately;
- add or update the machine support matrix and backend-refusal corpus;
- preserve stable diagnostics and source spans;
- do not implement a partial lowering that emits unverifiable or semantically different code;
- prefer a typed refusal over a hidden fallback or native escape hatch.

M14's defining gate is that its application profile has no undeclared check/build/run gap. The offline block-game solver is an acceptance oracle: board rules, placement enumeration, clearing, scoring and search must run in Sico; Python may serve only as the fixed comparison oracle.

## 6. Repository locations

- `crates/`: compiler, IR, codegen, package, Runtime-facing libraries, tooling and Host cores.
- `runner/sico-runner/`: separate Cargo workspace for Script execution and DAP integration.
- `wit/`: versioned Component interfaces.
- `semantic-cases/`, `syntax-candidates/`, `syntax-mutations/`: language oracles.
- `tests/`: end-to-end, security, platform and performance evidence.
- `examples/`: language-design history and regression examples; do not rewrite them as product pilots.
- `pilots/`: clean-room workflows; they are not external adoption.
- `docs/rfc/`, `docs/adr/`, `docs/steps/`, `docs/reports/`: contracts, decisions, execution records and evidence.
- `案例项目/俄罗斯方块消除/`: M14/M16/M17/M18 acceptance case. Keep its Python implementation as an oracle until the corresponding Sico layer has passed its gate.

## 7. Editing rules

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

For documentation-only roadmap changes, at minimum run the relevant validator and `git diff --check`. The M14–M18 planning contract is checked by:

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
