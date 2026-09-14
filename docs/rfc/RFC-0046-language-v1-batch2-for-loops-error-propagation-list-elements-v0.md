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
