# STEP-0174: Standard-library batch 2 — byte/text access, collections, formatting (RFC-0045)

> - status: complete
> - phase: M21 §3.1 / M22 S0 (shared entry prereq; RFC-0045 accepted 2026-09-14 via owner session directive 「先完成M21和自举吧」)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: [`RFC-0045`](../rfc/RFC-0045-stdlib-batch2-byte-text-collections-v0.md), `crates/sico-ir/src/lib.rs`, `crates/sico-semantics/src/lib.rs`, `crates/sico-codegen-wasm/src/{lib,stdlib}.rs`, `tests/end-to-end/stdlib-batch2.sico`, `runner/sico-runner/tests/stdlib_batch2.rs`, `tests/language-matrix/application-profile-v0.json`

## 1. What was done

Nine standard-library operations frozen by RFC-0045 and implemented as
guest-side emitted helpers (no world/import change, grammar unchanged):

- `sico.bytes.at` — total function `(Bytes, U64, I64) -> I64` with an
  explicit fallback (D1 as amended: the packed `Result[U64, NumericError]`
  match shape is the unexercised A6 class and is deferred).
- `sico.bytes.equal` — content equality.
- `sico.text.compare` — byte-order lexicographic `-1/0/+1`, shared via
  cross-helper calls by the ordering family.
- `sico.text.char_at` — char-indexed `(Text, U64) -> Result[Text,
  NumericError]` (the proven 4-slot Text-payload shape).
- `sico.text.format` — `{N}` placeholders, `{{`/`}}` escapes, documented
  verbatim pass-through (D3); single pass into an upper-bound allocation.
- `sico.list.sort` — stable insertion sort, byte-order ascending, into a
  fresh table (copy-on-write; the source list is untouched).
- `sico.list.min` / `sico.list.max` — total functions with an explicit
  empty-list default argument (D2).
- `sico.map.values[Text,Text]` — values in first-insertion key order;
  every other instantiation is a typed check-time refusal (D4), as is
  `map.entries` (pair records stay proposed).

Surface wired end to end: IR `intrinsic_signature` + the collection
registry (`CollectionOperation::MapValues` with the `[Text,Text]`-only
executable gate), check-time resolution tables in semantics, codegen
emission arms, helper signatures/bodies, `helper_dependencies` registration
(including the `sico.text.compare` cross-call), and the machine matrix.

## 2. Defects found by the new content corpus (all fixed here)

1. **`sico.text.split_words` dropped the final token** (STEP-0083-era):
   the final-token write was gated on `in_token == 0` — inverted — so any
   input ending inside a token lost its last word and kept an empty slot.
   No prior corpus asserted per-token content. Fixed and covered.
2. **`bytes.at` first draft read an out-of-bounds address** (this step):
   the source pointer was never added to the index before the load; the
   wasm validator caught the leaked operand. Fixed with an explicit
   `I32Add`; the whole arm then moved to the stack-neutral total-function
   shape (D1 amendment).
3. **`text.format` two-pass draft diverged** (this step): replaced by the
   single-pass upper-bound-allocation form (D3 slack is bounded and the
   arena is reclaimed per call).

## 3. Validation

- `runner/sico-runner/tests/stdlib_batch2.rs` 3/3: content test with
  byte-exact stdout for all nine intrinsics (incl. escapes, verbatim
  pass-through, empty-list defaults, out-of-range fallbacks, multi-byte
  `char_at`), repeated-run determinism, and the D4 check-time refusal.
- Every function of the produced core module validated operator-by-
  operator with `wasmparser::FuncValidator` during development; the
  temporary probe harness was removed after use.
- `validate-step-0131` (matrix fixture, corpus paths, registry surface):
  OK; matrix gains `stdlib-batch-2-byte-text` and
  `stdlib-batch-2-collections` rows.
- Full clippy/fmt/both-workspace regression: see the STEP-0174 closure
  note in STATUS (run-ci round recorded there).

## 4. Residuals

- The tetris board-reader's ASCII-mask retirement (RFC-0045 exit corpus
  item 4) is tracked as its own slice: the reader rewrite needs the
  `List[I64]` extension to keep the board arithmetic, which is the
  follow-up collection RFC.
- for-loops / error propagation / bare-literal decision / `List[I64]`
  extension: RFC-0046 (next STEP).
