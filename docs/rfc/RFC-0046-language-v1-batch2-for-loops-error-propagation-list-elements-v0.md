# RFC-0046: Language v1 batch 2 — for-loops, error propagation, bare literals, list elements

> - status: accepted
> - accepted: 2026-09-14 (owner session directive 「先完成M21和自举吧」 covering the M21 §3.2 decision points; D-points resolved as recorded in §3)
> - date: 2026-09-14
> - phase: M21 §3.2 / M22 S0 continuation
> - depends: RFC-0033 (syntax decision gate), RFC-0038 (application profile), RFC-0044 (batch 1), RFC-0045 (batch 2 stdlib)

## 1. Summary

Four language decisions, each closing a measured gap with the smallest
grammar surface that satisfies the consumers (M21 pilots and the M22
self-host frontend):

1. **For-loops** (`for x in expr:` … `end for`) over the executable
   iterable set, lowering to the general CFG (STEP-0130 machinery).
2. **Error propagation** (`expr?` in `let` bindings) desugaring to the
   existing `match` shape.
3. **Bare-literal typing decision**: recorded, no grammar change.
4. **List element extension**: `List[I64]`/`List[U64]` join
   `List[Text]` as executable types.

## 2. Iterable and operand domains (v1)

- `for x in expr`: `expr` may be `List[Text]`, `List[I64]`,
  `List[U64]`, or a `Map[K,V]`/`Set[K]` value (iterating its
  first-insertion key order — the same order `map.keys` reports).
  `Text` iteration stays refused in v0 (char iteration composes via
  `text.length`/`char_at`, and a dedicated `text.chars` intrinsic is
  cleaner than implicit decoding; queued for batch 3).
- `break`/`continue` work inside `for` exactly as in `while`.
- Limit+1: nesting depth reuses the existing block-depth budget.

## 3. Decision points (resolved at acceptance)

- **D1 — for-loop binding**: `for x in expr` binds `x` as an immutable
  per-iteration value (a fresh binding each iteration, visible after the
  loop only via explicit `set` into an outer variable). No index form in
  v0 (`for i, x in …` is proposed; compose with `list.length` + index
  variable when needed).
- **D2 — error propagation**: `let y = expr?` desugars to
  `match expr { case ok(v): y = v … case error(e): return error(e) }`.
  Valid only inside functions whose return type is
  `Result[T, NumericError]` for `expr: Result[U, NumericError]`
  (payload type unification `U ≤ T`); the error payload is forwarded
  verbatim. `expr?` in expression position (nesting) is proposed, not
  frozen — chaining multiplies the desugar paths.
- **D3 — bare literals**: recorded as a NO-GRAMMAR-CHANGE decision:
  bare integer literals remain arbitrary-precision `Int` (NUM-001
  oracle preserved; the STEP-0169 fallback stays); executable code names
  widths explicitly via `I64.literal`/`U64.literal`. This closes the
  M21 §3.2 decision point with the measured trade-off unchanged.

  **2026-10-21 re-measurement (M23 plan §8.3, STEP-0498) — D3 stands.**
  The item-3 re-measurement ran both legs; the decision is unchanged and
  **no grammar change is made**.

  - *selfhost leg* (pinned commit `2531427`, 9/9 selfhost blobs
    byte-identical to the commit): literal-constructor calls 2,169 →
    **3,236**, but density **3.199 → 3.055 per KB** (17.638 per 100
    expression lines) — the explicit-width ceremony did not get denser;
    the growth came from code unrelated to literals (the arena-treadmill
    machine). A second, independent measurement tool agrees exactly and
    prices the hypothetical typed-literal form at **42,042 bytes (3.88%
    of the corpus)**.
  - *AI leg* (100 provider-billed `deepseek-chat` calls under the M13
    identity, USD 0.027196, corpus frozen and owner-reviewed at
    [`ai-eval/runs/item3-2026-10-21`](../../ai-eval/runs/item3-2026-10-21/)):
    **0 literal-constructor calls (0.000/KB)**. That zero is a
    surface-syntax artifact, not a property of AI-written code: A0 is an
    M1-era *retained control* syntax whose guide defines no
    explicit-width literal form and which no compiler can execute (100/100
    candidates are rejected by the current front-end with `E1013`). The
    AI leg is therefore **not commensurable** with the selfhost leg and
    neither corroborates nor refutes its density.
  - *type facts, measured on the current compiler*: a bare integer
    literal is **already accepted** where an `Int` is expected (`return 0`
    → syntax ok, type checker ok, zero diagnostics); `return 0` in an
    `I64` context is `E2002` ("expected I64, found Int; convert
    explicitly"); `I64.literal(0)` in an `Int` context is `E2001`.
    `Int` and `I64` have no implicit conversion, so the explicit
    constructor is forced **only** where an `I64`/`U64` value is actually
    needed — the selfhost's density reflects how width-typed that code
    is, not a tax on all integer use.
  - *consequence*: the open question is narrower than "should literals be
    bare". It is **whether an `I64`/`U64` value can be written without an
    explicit constructor call** (a context-inferred width literal). That
    is **not decided here** and no such RFC is opened: the evidence above
    is a measurement, and any future typed-literal RFC would have to
    carry the full RFC-0033 evidence (explicit desugar, source-map
    identity, formatter idempotence, typed diagnostics) plus its own
    measured consumer.
  - evidence:
    [`ai-eval-item3-live-model-2026-10-21`](../reports/ai-eval-item3-live-model-2026-10-21.md),
    [`STEP-0498`](../steps/STEP-0498-m23-item3-corpus-freeze-and-d3-record.md).
- **D4 — list elements**: `List[I64]`/`List[U64]` become executable
  with `list.length/get/append` (and `sort/min/max` numeric variants);
  the intrinsics are monomorphized per element like the M14 map/set
  family (`sico.list.get[I64]` spelling). `List[Bool]` is refused
  (no consumer); records in lists stay proposed.

## 4. Grammar (exact)

```
ForBlock   := "for" IDENT "in" Expression ":" BlockBody "end for"
Propagate  := Expression "?"
```

`for`/`in` become reserved keywords (RFC-0033 reconsideration gate:
explicit desugar, source-map identity, formatter idempotence, and
typed diagnostics for pre-existing identifiers named `for`/`in`).

## 5. Compatibility

- Single-module programs that never write `for`/`in` compile
  byte-identically; sources using them as identifiers receive typed
  diagnostics (E1xxx family) with spans.
- The lowering desugars for-loops to `while` + index + `list.get` over
  the general CFG; no new codegen machinery. Frozen lowering snapshots
  are additive-only.
- `expr?` desugars to the existing `match` IR; no new IR operation.

## 6. Exit corpus (binding when accepted)

1. e2e: for-loop over `List[Text]` and over `Map[K,V]` keys with
   `break`/`continue`, byte-exact stdout.
2. e2e: `expr?` happy path + propagation path through a real runner.
3. e2e: `List[I64]` get/append/length/sort/min/max content test.
4. Refusal corpus: `for` over `Text`; `expr?` in a non-Result function;
   `List[Bool]` — typed diagnostics each.
5. Matrix rows + validator extension; formatter idempotence on the new
   blocks; frozen snapshot additions follow the append discipline.
