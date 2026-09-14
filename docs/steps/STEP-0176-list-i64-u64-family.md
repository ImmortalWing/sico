# STEP-0176: List[I64]/List[U64] numeric list monomorphs — signed/unsigned ordering

> - status: complete
> - phase: M21 §3.2 / RFC-0046 D4 (the list-element extension slice that unlocks scalar for-loops)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `crates/sico-ir/src/lib.rs`, `crates/sico-semantics/src/lib.rs`, `crates/sico-codegen-wasm/src/stdlib.rs`, `tests/end-to-end/{list-i64-family,list-i64-sort,for-loop-i64}.sico`, `runner/sico-runner/tests/list_i64_family.rs`, `tests/language-matrix/application-profile-v0.json`

## 1. What was done

The RFC-0046 D4 bracket-spelled numeric list family is now live end to end:
`sico.list.empty[I64]`/`[U64]`, `append`, `get` (packed Result form),
`length`, `sort`, `min`, `max`. The ordering comparators are the crux of
this slice:

- **I64 elements sort with signed compares** (`i64.lt_s` / `i64.gt_s`) —
  negatives order before positives (`-5 < -1 < 3`), so `min[I64] = -5`.
- **U64 elements sort with unsigned compares** (`i64.lt_u` / `i64.gt_u`) —
  the two's-complement high-bit range orders above every non-negative.

Wiring: the IR collection registry gains `ListEmpty`; `collection_signature`
gains the empty arm (the list branch already covered the rest); semantics
`infer_collection_call` and `parse_collection_suffix` gain the empty row
and confirm sort/min/max for I64/U64; codegen `collection_helper_signature`
and `emit_collection_helper` add the empty entry (sharing
`emit_collection_empty`) and call the scalar sort/extreme emitters with the
signedness flag. The for-loop's bracket-name selection (the D4 monomorph
spelling for numeric elements) was already in place from STEP-0175 and
is now exercised end-to-end.

## 2. Validation

- `runner/sico-runner/tests/list_i64_family.rs` 3/3:
  - family: `list.empty[I64]` + two `append`s + `get[0]` + `length` →
    `42/2` (byte-exact);
  - signed sort: `[-5, 3, -1]` sorts to `[-5, -1, 3]`; `min = -5`,
    `max = 3` → `-5,3,-5` `3` concatenated (`-5,3,-53`);
  - for-loop: iterates `[10, 20, 30]`, accumulates `60` → `102030=60`.
- `validate-step-0131` (matrix + corpus paths): OK; matrix gains
  `list-i64-u64-family`, `for-loops`, `error-propagation` rows.
- `git diff --check`: clean. Full `tools/run-ci.ps1` 10/10: CI GREEN
  (fmt, both clippies, both test suites, module boundaries, planning
  contract, matrix, cross-host corpus, whitespace).

## 3. Frozen-shape note

No STEP-0148 frozen snapshot changed: the numeric family is bracket-spelled
and never appears in the `RESULT-001`/`NUM-*` frozen cases; `Text` lists
keep the frozen plain spelling untouched.

## 4. Residuals

- `map.entries` (pair records in lists) stays proposed per RFC D4.
- `Text` list iteration via for-loop keeps the plain names; a dedicated
  `text.chars` intrinsic is queued for batch 3 rather than implicit
  decoding.
