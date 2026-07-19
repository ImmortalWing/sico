# Sico tooling protocols

`sico.execution-plan.v0` is the shared editor/AI process-launch plan. It contains a fixed executable plus argument array, always sets `shell: false`, leaves `cwd` unset, caps captured logs at 1 MiB, and makes client-owned process-tree cancellation explicit.

The plan does not execute anything. Compile diagnostics use UTF-8 half-open byte coordinates; Runtime source locations and Debug Adapter support are explicitly false in v0.

Normative schema: [`execution-plan-v0.schema.json`](./schema/execution-plan-v0.schema.json).
