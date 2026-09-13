# RFC-0044: Language v1 batch 1 — infix equality conditions

> - status: accepted (owner session directive "完成M19-M20", 2026-09-13; RFC-0033 reconsideration gate: additive lowering, previously-refused programs remain refused elsewhere, no working program changes behavior)
> - date: 2026-09-13
> - phase: M20 §3.1 language v1 (first batch from the measured pilot friction log)

## Summary

Infix `==` conditions lower directly: `left == right` in an `if` condition
emits the same `EqualFixed` comparison the typed `I64.equal`/`U64.equal`
calls emit, for same-width fixed operands (`I64`/`I64`, `U64`/`U64`).
This removes the measured pilot blocker (the Web/UI application had to
rewrite `length(acc) == U64.literal(0)` into match form, STEP-0162/0164).

## Contract

- Operand types must be identical and fixed-width (`I64` or `U64`);
  anything else is the typed refusal `unsupported: infix equality
  condition operand types` (unchanged refusal class, narrowed condition).
- The frozen revision-guard path is unaffected: revision equality keeps
  lowering through `lower_revision_if`; non-Revision equality in the
  revision-if *shape* now falls back to the general CFG instead of being
  refused (this is what makes `if 7 == 7: return ...` shape functions
  lowerable).
- Bare integer literals remain unbounded `Int` (NUM-001 arbitrary-
  precision oracle unchanged); the bare-literal typing question (bare =
  I64 vs Int, and its interaction with the arbitrary-precision story) is
  recorded as the v1 decision point with this measured evidence: bare=I64
  breaks NUM-001's `999999999999999999999999 + 1` fold.
- `-7`-style negative fixed literals continue via `I64.literal(-7)`.

## Corpus

`tests/end-to-end/infix-equality.sico` (+ `.test.json`): typed-literal
equality, `text.length(...) == U64.literal(0)`, and equality inside
checked-arithmetic match arms — byte-exact via the real runner.
