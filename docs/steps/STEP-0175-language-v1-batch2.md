# STEP-0175: Language v1 batch 2 — for-loops, error propagation, numeric list elements (RFC-0046)

> - status: complete
> - phase: M21 §3.2 / M22 S0 continuation (RFC-0046 accepted 2026-09-14 via owner session directive 「先完成M21和自举吧」)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: [`RFC-0046`](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md), `crates/sico-{lexer,ir,semantics,codegen-wasm}/`, `tests/end-to-end/{for-loop-words,for-loop-collections,error-propagation,list-numeric}.sico`, `runner/sico-runner/tests/language_batch2.rs`, `tests/language-matrix/application-profile-v0.json`, `tools/validate-step-0131.ps1`

## 1. What was done

The four RFC-0046 decisions are executable end to end:

1. **For-loops** over the full executable iterable set: `List[Text]` (plain
   frozen intrinsics), `List[I64]`/`List[U64]` (bracket monomorphs), and
   `Map[Text,V]`/`Set[Text]` subjects (lowered by materializing
   `map.keys`/`set.to_list` up front, then the ordinary list loop — key
   order is the frozen first-insertion order). `break`/`continue` route
   through the increment block; the loop variable is a loop-scoped cell, so
   sequential loops may reuse the binding name (RFC-0046 D1).
2. **Error propagation**: `let x = expr?` lowers to the match shape — the
   source Result spills into a cell, a `match` terminator routes ok (payload
   binds the `x` cell, control continues) and error (the spilled Result is
   returned **verbatim**: tag, payload and NumericError discriminant intact,
   no re-encoding). Valid only in functions returning
   `Result[T, NumericError]` (E3101 otherwise, both error types frozen).
   There is no value-level unwrap operation; the `try expr` keyword form
   stays check-time only as before.
3. **Bare-literal typing**: recorded as a NO-GRAMMAR-CHANGE decision per
   RFC-0046 D3 (NUM-001 oracle preserved); no implementation.
4. **List element extension**: `List[I64]`/`List[U64]` are executable as
   8-byte-slot tables; `sico.list.length/get/append/sort/min/max[I64|U64]`
   are monomorphized collection intrinsics (helper plumbing self-registers
   through the closed collection grammar); `map.values[Text,I64|U64]` is
   unlocked (RFC-0045 D4). Numeric ordering is signed for I64, unsigned for
   U64 (verified with `u64::MAX` in the corpus). `List[Bool]`/`List[Bytes]`
   stay typed refusals.

## 2. Defects found by the new content corpus (all fixed here)

1. **`list.sort` shift loop never iterated** (STEP-0083-era, both the Text
   and the new numeric helper): the in-If `br 0` targeted the If itself
   (wasm if-labels sit at the end), so each element shifted exactly once —
   `[5,4,3,2,1]` rotated to `[4,3,2,1,5]` instead of sorting. No prior corpus
   asserted a multi-shift case. Fixed to `br 1` (the inner loop) in both
   emitters.
2. **`continue` inside `for` spun forever** (this step): the loop frame
   pointed `continue` at the condition block, skipping the index increment.
   The frame now targets the increment block; the RFC-0046 §2 exit corpus
   (continue + break) covers it.
3. **`case error(e)` on a packed fixed-width Result panicked the compiler**
   (A6 seam, STEP-0143/0148 records): the layout planner mapped the error
   field past the two-slot packed layout. The field is now omitted when it
   would exceed the layout, turning the panic into the existing typed
   unknown-field refusal. The `?` desugar never projects the error payload —
   it forwards the whole Result — so propagation is exact regardless.
4. **`sico.text.split_words` final-token loss** was fixed in STEP-0174; this
   step's corpora keep it covered.

## 3. Validation

- `runner/sico-runner/tests/language_batch2.rs` 7/7 through the real
  Component Runtime: for-loops over lists/maps/sets byte-exact
  (`alpha|beta|gamma|delta|`, `ac\nxyz\n`), `?` happy + chained + division-
  by-zero propagation byte-exact (`half=8\ninv=0\ndiv0=caught\n`), numeric
  list content byte-exact (signed sort with negative element, unsigned U64
  sort with `u64::MAX`, min/max defaults, for-loop sum), repeated-run
  determinism, and three check-time refusals (for-over-Text E2001,
  wrong-error `?` E3101, `List[Bool]` E2031).
- `stdlib_batch2.rs` refusal updated: `map.values[Text,I64]` is executable
  now; the refusal corpus uses `map.values[Text,Bool]` (3/3 green).
- `validate-step-0131` extended (matrix step `STEP-0131..0175`, batch-2
  corpus existence checks, list-family registry probe incl. refusals): OK.
- Full regression: see the STEP-0175 closure note in STATUS (both workspaces
  + clippy + fmt recorded there).

## 4. Residuals

- **v1 batch 3 candidates recorded in the M22 plan §8 friction register**:
  infix comparison/logical operators (`< > >= != && || !`), expression-
  position conditions (`let x = if/match`), and the bare-literal AI-noise
  re-measurement policy (D3 stays an accepted cost until measured again on
  real generated corpora).
- The A6 packed-Result `case error(e)` payload binding remains a typed
  refusal (recorded since STEP-0143; the `?` desugar and all frozen corpora
  avoid it). Its repair is a separate defect STEP if a consumer needs it.
- Text iteration (`for c in text`) stays refused; `sico.text.chars` is the
  queued batch-3 intrinsic (RFC-0046 §2).
