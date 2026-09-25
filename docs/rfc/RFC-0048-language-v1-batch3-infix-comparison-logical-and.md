# RFC-0048: Language v1 batch 3 item 1 — infix comparison conditions and `&&`

> - status: **draft — owner acceptance required before any implementation STEP** (M23 plan §2 dual gate still applies afterwards: Route A = M22 S7 GO, Route B = R3 stop-loss)
> - date: 2026-09-25
> - phase: M23 §8.1 (language v1 batch 3, item 1)
> - depends: RFC-0033 (evidence gate), RFC-0044 (infix equality precedent), RFC-0046 §2 (batch-3 queue), STEP-0284 census (measured demand)

## 1. Summary

Extend the RFC-0044 infix-condition mechanism from `==` to the ordered
comparisons `<` `<=` `>` `>=`, inequality `!=`, and the conjunction `&&`,
on the same operand domain (identical, fixed-width `I64`/`U64`), lowering
through the **two existing IR comparison operations** (`EqualFixed`,
`LessFixed`) and the existing branch structure — **no new IR operations,
no new intrinsics**. Disjunction `||` and prefix `!` are **excluded from
v0** (no measured consumer; see §3).

## 2. Measured demand (frozen evidence, not recollection)

From the STEP-0284 census (`m23-ceremony-census-2026-09-25.json`, 8
selfhost sources / 16,390 lines) and this RFC's probes:

- Comparison-intrinsic call sites in selfhost: `U64.less_than` 634,
  `U64.equal` 302, `I64.equal` 200, `I64.less_than` 59 — 1,195 total;
  range-scan code (lexer/ident/number scanning) is dominated by
  `less_than` chains (`b < 48`, `b < 58`, `b < 65`, …).
- 978 comparison-predicate `if` lines (59.7 per 1,000 lines);
  79 immediately-nested comparison-`if` sites (depth ≥ 2) are
  conjunction emulation — the `&&` consumer.
- Negation emulation (`if not …`) count: 0. Disjunction-emulation sites
  were not observable in the census patterns: **no measured `||`/`!`
  consumer**, so they stay out of v0.
- Current-surface probes (2026-09-25, this tree):
  - `if x < 4:` → lex error `UnexpectedCharacter('<')` (no `<` token);
  - `if x <= 4:` → **check passes with zero diagnostics, build refuses**
    with `unsupported call target x<=U64.literal` — the `<=` token
    (`TokenKind::LessEqual`) lexes today but no stage consumes it. This
    check/build split on an unconsumed token is a declared-gap debt this
    RFC closes in either direction: acceptance here, and the refusal
    corpus row lands with the implementation STEP either way.

## 3. The smallest closed set (v0)

| source | desugar | IR operation | reorder? |
|---|---|---|---|
| `a == b` | RFC-0044, unchanged | `EqualFixed(a, b)` | no |
| `a != b` | branch inversion of `==` | `EqualFixed(a, b)` | no |
| `a < b` | direct | `LessFixed(a, b)` | no |
| `a <= b` | branch inversion of `b < a` | `LessFixed(b, a)` | **yes (swap)** |
| `a > b` | operand swap | `LessFixed(b, a)` | **yes (swap)** |
| `a >= b` | branch inversion of `a < b` | `LessFixed(a, b)` | no |
| `c1 && c2` | nested branch (outer `c1`, inner `c2`) | existing Branch | no |

"Branch inversion" = the lowering emits the comparison operation with the
`then`/`else` block targets swapped; no Boolean-negation operation exists
or is added.

- **Reorder discipline.** `>` and `<=` evaluate their source-left operand
  second. Script-v0 effects are fuel/trap-order and Host-free, but the
  order is still observable at the margin; therefore for the two
  swap-form operators v0 restricts operands to **atoms**: integer/string
 /bool literals, identifier bindings, and field-access chains rooted at
  an identifier or literal. Anything else (call results, indexed reads,
  computed chains) is the typed refusal in §7. `<`, `==`, `!=`, `>=`
  keep the full RFC-0044 operand surface (arbitrary condition
  expressions, evaluated left then right).
- **`&&` semantics.** Short-circuit by construction: the right condition
  is not evaluated when the left branch is false — skipped fuel, skipped
  traps, skipped effects are defined behavior, not an optimization.
  Left-to-right evaluation order is preserved. v0 allows `&&` only at
  the top level of an `if`/`while` condition and only between
  comparison/equality conditions; nesting `&&` inside `&&` is allowed
  (same desugar applied recursively); mixing with `||` is impossible in
  v0 (`||` does not exist).
- **Operand domain (all operators).** Identical fixed-width `I64`/`I64`
  or `U64`/`U64`, exactly RFC-0044's rule; `Text`, `Bool`, `F64`,
  records, and mixed-width comparisons stay refused. There are no
  ordered-comparison operations for them in the IR and this RFC adds
  none.

## 4. Lexer changes (deliberate, RFC-frozen punctuation growth)

Add tokens: `<` (`LessThan`), `>` (`GreaterThan`), `>=` (`GreaterEqual`),
`!=` (`BangEqual`), `&&` (`AmpAmp`). `<=` (`LessEqual`) already exists.
`|`, bare `!`, bare `&` remain non-tokens (lex errors, as today). The
pair scan order is `<= <`, `>= >`, `!=`, `&&`, `==` (longest match
first). The guest lexer (`selfhost/compiler_lexer.sico`) gains the same
pair scan only in the re-baseline slice (§9) — the selfhost sources do
not use infix operators today (0 ` == ` occurrences), so the guest mirror
is not load-bearing for the M22 differential until adoption.

## 5. Compatibility audit (RFC-0033 reconsideration gate)

- No working program changes behavior: every new form is today either a
  lex error (`<`, `>`, `!`, `&`) or the measured build refusal (`<=`
  probe above). Widening a refusal into an accept is additive.
- `<=` already lexes, so today's check-green/build-refused programs move
  to accepted when their shape matches §3, and to a *narrower, named*
  refusal when it does not (§7) — never to silence.
- Keyword collisions: none (no new identifiers or keywords; `in`, `not`,
  `and`, `or` remain non-keywords and are not introduced).
- Pre-existing identifiers cannot collide with punctuation tokens.

## 6. Formatter

Canonical spacing: exactly one space on both sides of every operator in
this RFC (`a < b`, `a != b`, `c1 && c2`); no line-breaking inside a
condition introduced by these operators; idempotence (formatting twice =
once). `crates/sico-format` and the guest `formatter.sico` re-baseline
slice must agree byte-exactly on the frozen corpus.

## 7. Diagnostics (append-only; no renumbering)

- New typed refusal `E2025 INFIX_SWAP_OPERAND_NOT_ATOM`: non-atom operand
  in a `>` / `<=` (swap-form) condition; message names the rewrite
  (`b < a` form). Actionable, spans the offending operand.
- Mismatched or non-fixed operand types: the existing RFC-0044 refusal
  class (`unsupported: infix equality condition operand types`) is
  generalized to `unsupported: infix comparison condition operand types`
  — same refusal channel, narrowed condition set; the message text change
  is recorded here so diagnostics consumers are not surprised.
- `&&` outside a condition position, or with a non-condition operand, is
  a typed refusal reusing the existing condition-shape unsupported
  channel.
- Existing stable codes are untouched. Code inventory at drafting:
  E2001, E2002, E2010, E2011, E2020–E2024, E2031 in use; E2025 is the
  next append.

## 8. Exit corpus and evidence bar (implementation STEP, gated separately)

- `tests/end-to-end/infix-comparisons.sico` (+ `.test.json`): every
  operator on `I64` and `U64`, boundary equalities (`<=`/`>=` at the
  boundary), `&&` nesting (3 deep), a swap-form with atom operands, and
  an `!=` inversion — byte-exact via the real runner, `limit+1` nesting
  refusal at the documented block-depth budget.
- Refusal corpus: non-atom swap operand (E2025), mixed-width, `Text`/`Bool`
  operands, `&&` outside condition, `||`/bare `!`/bare `&`/bare `|` lex
  errors; single diagnostic each, stable spans.
- Machine support matrix row + validator; frozen snapshots append-only.
- Formatter idempotence fixture over the new corpus.
- Full RFC-0033 four-part evidence; `check`, `build`, `run` verified
  separately (the §2 `<=` probe shows why).

## 9. Dual-implementation and re-baseline entry

Landing the Rust-side surface is one implementation STEP. The selfhost
frontend adoption (guest lexer pair scan, guest condition lowering,
formatter mirror) is the explicit M23 §9 re-baseline decision, recorded
separately per the M22 dual-implementation register; until that entry,
the selfhost sources stay on their current intrinsic-call style, and the
M22 differential is unaffected.

## 10. Non-goals (v0)

`||`, prefix `!`, `and`/`or`/`not` keywords; comparisons on `Text`,
`Bool`, `F64`, records (record equality remains E2024-refused per
RFC-0047); chained comparisons (`a < b < c`); expression-position
conditions (M23 item 2's separate RFC); assignment operators; any new IR
operation or intrinsic.
