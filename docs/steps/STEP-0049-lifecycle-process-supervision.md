# STEP-0049: Lifecycle, process supervision and crash isolation

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现 single-instance guest supervision、bounded open events、timeout/cancel/crash classification、process-tree kill 与 terminal cleanup。

## 2. Context and evidence

ADR-0004 要求重复打开不能创建无界 guest；RFC-0018 已区分 Runtime fault，但 Desktop 还需管理 OS child、permission session 和 Host temp lifecycle。

## 3. Scope

包含 direct Command program/args、single app identity child、256 open queue、foreground/background、poll/wait/cancel、Windows process-tree kill、exact cleanup。不包含 graceful guest protocol 或窗口 renderer。

## 4. Decision

shared `ProcessSupervisor` 作为唯一 child owner。timeout/cancel 先进入 Closing 再 kill process tree；terminal 必须清 session、清 exact registered files、remove active identity，之后可健康 relaunch。

## 5. Changes

- `CommandSpec` 无 shell string；
- `ProcessSupervisor` 与 Starting/Running/Background/Closing active states；
- Exited/Crashed/TimedOut/Cancelled terminal outcomes；
- 256 duplicate-open ceiling；
- Windows `taskkill /T /F` + child kill fallback；
- cleanup root escape/link refusal；
- single-instance、overflow、timeout/crash/relaunch、cancel cleanup tests。

## 6. Validation

```text
cargo clippy -p sico-host-core --all-targets -- -D warnings
cargo test -p sico-host-core
STEP_0049_OK supervision=single-child queue=256 states=running,background,closing terminals=exited,crashed,timeout,cancelled process_tree=windows-kill cleanup=exact-host-owned lifecycle_tests=3 host_tests=9 next=STEP-0050
```

## 7. Metrics

3 lifecycle tests；256 queued event ceiling；4 terminal classes；3 active transition states exercised；1 outside-cleanup negative fixture；healthy relaunch after timeout+crash。

## 8. Risks and follow-ups

Non-Windows kill currently targets direct child only; platform adapters must use their process-group/job mechanisms before runtime-verified claims. No graceful close WIT exists yet；forced deadline remains mandatory。

## 9. Audit links

- [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
- [`review report`](../reports/desktop-host-lifecycle-v0.md)
- [`STEP-0048`](./STEP-0048-permission-records.md)
