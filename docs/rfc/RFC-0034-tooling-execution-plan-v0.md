# RFC-0034: Tooling execution plan v0

> - status: accepted
> - created: 2026-07-19
> - phase: M9 / STEP-0093

## 1. Decision

Editors and AI tools share one non-executing `sico.execution-plan.v0` value for check/run/watch/REPL workflows. The implementation lives in `sico-tooling-protocol`; callers receive a fixed executable and argument array, never a command string.

Every plan fixes:

- `executable: "sico"`, `shell: false`, `cwd: null`;
- `--json` protocol mode and the expected stdout/stderr schemas;
- caller-selected capture at `1..=1 MiB`, with `truncate-and-mark` overflow behavior;
- client-owned direct-child process-tree termination with a 1-second grace;
- compile coordinates as UTF-8 half-open byte ranges;
- Runtime locations and Debug Adapter support as unavailable in v0.

Paths and arguments are byte-bounded and reject controls/NUL. Spaces and shell metacharacters remain literal array elements. The protocol library performs no file read/write, process launch, shell invocation, network access or capability grant.

## 2. Mode mapping

| Mode | Direct argv prefix | stdout | typed stderr schemas |
|---|---|---|---|
| check | `check --json PROGRAM` | `sico.diagnostics.v0` JSON | none |
| run | `run --json PROGRAM` | guest bytes | `sico.runner.outcome.v0` |
| watch | `watch --json PROGRAM` | guest bytes | outcome + `sico.runner.watch.v0` |
| repl | `repl --json` | `sico.repl.event.v0` JSONL | `sico.repl.event.v0` |

Run/watch may append at most 64 guest arguments after a literal `--`; each program/argument is at most 4 KiB and all guest arguments total at most 16 KiB.

## 3. Integration

LSP advertises `sico.check`, `sico.run`, `sico.watch`, `sico.repl` and returns this plan. `sico.debug` remains typed error `-32004`.

The AI protocol adds `plan_execution`. It validates and returns the same value but does not execute it. Execution authority remains with an editor/user-controlled client.

## 4. Honest cancellation and source-map boundary

The plan tells a client how to stop the direct process tree. Abrupt client cancellation is not claimed to produce runner exit 123, so `typed_runner_exit_guaranteed` is false. The existing deterministic runner cancellation hooks remain separate Runtime evidence.

Compile diagnostics retain byte ranges and LSP converts them to UTF-16 when displaying. Runtime outcomes do not yet contain source locations; v0 therefore sets `runtime_locations: false` and cannot support pause/step/stack/variables. A real Debug Adapter requires new Runtime hooks and a new protocol revision.

Normative JSON shape: [`tooling/schema/execution-plan-v0.schema.json`](../../tooling/schema/execution-plan-v0.schema.json).
