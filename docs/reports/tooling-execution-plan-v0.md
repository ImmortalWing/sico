# Tooling execution plan v0 evidence

> - date: 2026-07-19
> - step: STEP-0093
> - status: verified implementation evidence

## Contract

One shared crate builds deterministic execution-plan JSON for LSP and AI callers. It validates path/argument/log limits and returns data only. The fixed plan carries argv, protocol schemas, capture behavior, cancellation ownership and source-map/debug availability.

## Evidence

The final validator proved:

- check/run/watch/REPL plans always use direct argv and `shell: false`;
- spaces, `;`, backticks and `$()` remain literal arguments;
- program ≤4 KiB, at most 64 guest args, each ≤4 KiB, total ≤16 KiB;
- capture limit is `1..=1 MiB`, overflow is marked truncation;
- real LSP framing returns three execution plans and `sico.debug` returns `-32004`;
- real AI-tool JSON returns the same schema without starting a process;
- compile coordinates are explicit while Runtime locations/debug stay false;
- 24 workspace packages are assigned once and all four cross-module exceptions are narrow, live and reasoned.

## Limits

No editor extension or DAP server is shipped. Client cancellation terminates the child tree and is not reclassified as a typed guest cancellation. The protocol never widens Script provider grants.
