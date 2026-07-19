# Bounded REPL session v0 evidence

> - date: 2026-07-19
> - step: STEP-0091
> - status: verified implementation evidence

## Contract and isolation

Cells use the existing compiler-backed constant `Int` expression path. A successful cell stores only source, deterministic ID and result. History/export replay all cells before observation; reset replaces the session with empty state. No runner Store, resource handle, provider grant or output queue is retained.

## Measured evidence

The final `tools/validate-step-0091.ps1` run proved:

- `sico.repl.event.v0` JSON lines and typed stderr errors;
- invalid expressions roll back without consuming an ordinal;
- reset makes the same first source reproduce the same cell ID;
- a 4,097-byte line is drained/rejected and the following expression succeeds;
- exactly 256 cells are accepted and the 257th is rejected;
- exactly 16 KiB of valid source history is accepted in unit evidence;
- export creates `sico.repl.session.v0`, and a second export cannot overwrite it;
- the 256-cell process completed in 53 ms with 7.8 MiB sampled peak working set.

## Limits

This is deliberately not a dynamic-language VM. Only the existing compile-time `Int` expression subset is available. Cross-cell declarations, statements, multiline interaction, completion, Script IO/providers and top-level syntax remain deferred.
