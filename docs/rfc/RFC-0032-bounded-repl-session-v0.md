# RFC-0032: Bounded REPL session v0

> - status: proposed
> - created: 2026-07-19
> - phase: M9 / STEP-0091

## 1. Decision

`sico repl` v0 accepts one existing compile-time `Int` expression per line. It does not introduce top-level declarations or statements; STEP-0092 remains the only authority for that syntax decision.

Each accepted cell has a deterministic SHA-256 ID over `sico.repl.cell.v0\0`, its one-based ordinal and exact normalized line bytes. Failed cells never enter history. `:history` and `:export PATH` recompile and replay every accepted expression and reject any value divergence before exposing a transcript.

## 2. Commands

| Input | Behavior |
|---|---|
| existing `Int` expression | compiler-backed evaluation and append on success |
| `:history` | replay all cells, then emit bounded history |
| `:reset` | discard all cells, byte accounting and ordinal state |
| `:export PATH` | replay, then exclusively create `sico.repl.session.v0` JSON |
| `:quit` or EOF | exit 0 |

Unknown commands, invalid UTF-8, invalid expressions and limit violations are non-terminal typed stderr events. JSON mode uses `sico.repl.event.v0` JSON lines.

## 3. Bounds

- one line/cell: 4 KiB;
- accepted cells: 256;
- total accepted source history: 16 KiB;
- export path: 4 KiB;
- input is read and oversized lines are drained with constant bounded buffering;
- output is written synchronously; there is no in-memory output queue.

Export uses `create_new` and never overwrites an existing path. Reset discards the full session state. REPL v0 has no Store, resource table, capabilities, filesystem access other than explicit export, network or child-process authority.

## 4. Deferred

Declarations, cross-cell names, top-level statements, Script Host providers, multiline editing, completion and terminal UI are deferred. After STEP-0092, a future version may extend cells through an explicit versioned protocol rather than silently changing v0.
