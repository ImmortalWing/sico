# STEP-0091: bounded REPL session model

> - status: complete
> - phase: M9
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## Objective

Add a deterministic, state-bounded REPL without deciding the top-level Script syntax reserved for STEP-0092.

## Delivered

- `sico repl [--json]` evaluates existing compiler-backed `Int` expressions.
- SHA-256 cell IDs bind domain, ordinal and exact cell source; failed cells do not mutate history.
- `:history` and `:export` replay all accepted cells; `:reset` discards cells, accounting and ordinal state.
- Bounds are 4 KiB per line, 256 cells, 16 KiB accepted history and 4 KiB export path. Oversized lines are drained before the next cell.
- Export is canonical `sico.repl.session.v0` JSON and uses exclusive creation without overwrite.

## Exit evidence

`tools/validate-step-0091.ps1` is green: typed JSON events, deterministic reset identity, failed-cell rollback, 4 KiB line recovery, exact 256-cell and 16 KiB history limits, exclusive export, 7.8 MiB measured peak RSS and 53 ms for the 256-cell growth case.

Report: [`bounded-repl-session-v0`](../reports/bounded-repl-session-v0.md).

## Honest limits

v0 is an expression REPL, not a declaration or statement REPL. It has no cross-cell names, multiline editor, Host capabilities or hidden guest Store. STEP-0092 may accept or reject top-level syntax without invalidating this explicit v0 contract.
