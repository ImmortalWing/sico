# RFC-0049: Language v1 batch 3 item 2 — let-bound `if`/`match` blocks

> - status: **draft — owner acceptance required before any implementation STEP** (M23 plan §2 dual gate still applies afterwards: Route A = M22 S7 GO, Route B = R3 stop-loss)
> - date: 2026-09-25
> - phase: M23 §8.2 (language v1 batch 3, item 2)
> - depends: RFC-0033 (evidence gate), RFC-0046 D2 (propagating-let desugar precedent), STEP-0284 census (measured demand)

## 1. Summary

Introduce exactly two let-bound statement forms —

```text
let x = if cond:
    <arm expr>
else:
    <arm expr>
end if

let x = match subject:
    case ok(v):
        <arm expr>
    case error(e):
        <arm expr>
end match
```

— lowering through the **already-landed cell + branch writes + join read
machinery** (the `let y = expr?` desugar of RFC-0046 D2,
`lower_propagating_let`, `crates/sico-ir/src/lower.rs:1894) with **no new
IR operations**. General expression-position conditions (`if`/`match` as
an operand inside a larger expression, in call arguments, in `return`
position, or nested inside another condition) stay refused in v0.

## 2. Measured demand (frozen evidence, not recollection)

- STEP-0284 census field `bind_value_match_blocks`: **248** sites in the
  8 selfhost sources (16,390 lines) whose `match` block exists only to
  bind one value into a cell via `set` in each arm; **121** in the 51-file
  application corpus. Each is the manual emulation of `let x = match`.
- The same shape over `if`/`else` (pre-initialized cell + `set` per arm +
  join read) is the standard conditional-assignment idiom across the
  selfhost tree and `tests/end-to-end` (e.g. `checked-mul-div.sico`
  `failures` pattern).
- Current-surface probes (2026-09-25, this tree): `let x = if …` and
  `let x = match …` are parse-refused today with
  `MismatchedClose { expected: Function, actual: If|Match }` — honest,
  typed, and the form this RFC widens into acceptance.

## 3. Contract (smallest form only)

- **Arms.** Each `if` arm (then/else) and each `match` arm body is
  **exactly one expression statement** — the same single-line expression
  grammar as today's `let`-init RHS. No `return`, no `set`, no nested
  blocks, no multi-statement arms. Anything else is the typed refusal
  `E2026 LET_BIND_ARM_NOT_EXPRESSION`.
- **Exhaustiveness.** The `if` form **requires** `else:`; missing else is
  `E2027 LET_BIND_MISSING_ELSE`. The `match` form inherits the statement
  `match` exhaustiveness rules unchanged.
- **Subject and conditions.** `match` subjects and `if` conditions use
  exactly the statement-position grammar (comparison/equality conditions
  per RFC-0044/0048; `Result`/enum subjects per the statement `match`).
  A condition or arm that itself contains an `if`/`match` expression is
  refused by the existing condition-shape/unsupported channels.
- **Typing.** Both arms must unify to one type (existing
  `E2001 TYPE_MISMATCH` channel points at the offending arm); the bound
  name gets that type. No type annotation on the binding in v0
  (`let x: T = if …` is refused; the annotation adds nothing on a fresh
  binding). Pattern variables (`v`, `e`) scope to their arm expression.
- **Lowering.** Desugar to a fresh cell + unconditional write per branch
  + join read — the D2 machinery, one more caller; both branches write,
  so the join read is always defined; no default value exists or is
  needed. No new IR operation, no new intrinsic.
- **Determinism.** Only the taken branch's arm expression evaluates;
  fuel/trap behavior follows the taken path, identical to the manual
  cell pattern it replaces.

## 4. Compatibility audit (RFC-0033 reconsideration gate)

- Both forms are parse-refused today (§2 probes): widening a refusal into
  an accept is additive; no working program changes behavior.
- No new tokens, keywords, or diagnostic-code renumbering; `end if` /
  `end match` close tokens are reused — the let-header binds the block,
  and the close mismatch probe messages disappear in favor of real
  lowering.
- Interaction with RFC-0048: independent forms; both may compose
  (`let x = if a < b:` inside an accepted comparison surface) without
  either depending on the other's acceptance.

## 5. Formatter

Canonical shape: header `let x = if cond:` (or `let x = match subject:`),
arms indented one level, `else:`/`case …:` at header depth, closing
`end if`/`end match` at header depth; exactly one expression per arm, on
its own line. Idempotence (formatting twice = once). `crates/sico-format`
and the guest `formatter.sico` re-baseline slice must agree byte-exactly
on the frozen corpus.

## 6. Diagnostics (append-only; no renumbering)

- `E2026 LET_BIND_ARM_NOT_EXPRESSION` — arm contains anything other than
  one expression; span = the offending statement.
- `E2027 LET_BIND_MISSING_ELSE` — `let x = if` without `else:`; span =
  the let header.
- Arm type mismatch: existing `E2001 TYPE_MISMATCH` (no new code).
- Nested expression-position condition: existing condition-shape
  unsupported channel (no new code).
- Code inventory at drafting: E2001, E2002, E2010, E2011, E2020–E2024,
  E2031 in use; RFC-0048 (draft) takes E2025; this RFC takes E2026/E2027
  — appended in RFC-acceptance order, no gaps assumed.

## 7. Exit corpus and evidence bar (implementation STEP, gated separately)

- `tests/end-to-end/let-bind.sico` (+ `.test.json`): `let x = if` with
  intrinsic-call arms and identifier arms; `let x = match` over
  `Result` (both arms) and a user enum; value used after the join
  (returned/encoded); byte-exact via the real runner; `limit+1` nesting
  refusal at the documented block-depth budget.
- Refusal corpus: two-statement arm, `return` inside an arm, missing
  `else`, annotated bind form, arm type mismatch, nested `if`-expression
  condition — single typed diagnostic each, stable spans.
- Machine support matrix row + validator; frozen snapshots append-only;
  formatter idempotence fixture; full RFC-0033 four-part evidence;
  `check`, `build`, `run` verified separately.

## 8. Dual-implementation and re-baseline entry

Rust-side landing is one implementation STEP. Selfhost adoption (rewriting
any of the 248 manual sites onto the new form is **not** required or
automatic) is the explicit M23 §9 re-baseline decision per the M22
dual-implementation register; the M22 differential is unaffected until
that entry. The guest formatter mirror follows the RFC-0048 §9 pattern.

## 9. Non-goals (v0)

`if`/`match` expressions in operand, argument, or `return` position;
nesting a let-bind inside another condition or arm; annotated
(`let x: T = if …`) form; multi-statement or block-valued arms; a ternary
operator; `while`-bound or `for`-bound analogues.
