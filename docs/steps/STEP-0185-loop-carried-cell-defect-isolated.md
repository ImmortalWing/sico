# STEP-0185: M22 S4→S5 boundary — loop-carried cell defect isolated (compiler lowering)

> - status: complete — **the token boundary defect is isolated to a
> compiler-lowering bug: `let`-bound cell reads inside a `while` loop
> body read a displaced value across iterations. A6-class, reproducible,
> registered**
> - phase: M22 S4→S5 (the defect blocks clean token spans feeding S5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `target/inline.sico` (decisive repro), this STEP record

## 1. The isolation

The token-emission defect (`Int` → `nt`, first byte of a boundary-crossing
run dropped) was minimised in STEP-0184. This slice isolates its layer:

- **Pure `sico.bytes.slice(src, 2, 3)` on `"x Int y"` → `Int`** — the
  slice primitive and its wasm emission are correct.
- **The `tokens.sico` classification loop source is correct** — `word_start
  = index` on the run's first byte; `word_len` accumulates per byte;
  `slice(word_start, word_len)` at run close.
- **Clean runs are correct** — `ab,cd` byte-exact; `main`/`return`/`function`
  clean.
- **Decisive experiment** — an **inlined** version of the same run logic
  (no function calls, `is_ident_byte` range checks spelled out) produces
  the **same** `x,nt,y` on `"x Int y"`. Removing the function-call factor
  leaves the compiler's lowering as the only variable.

**Conclusion: the defect is in the compiler's lowering of `let`-bound
cells read inside a `while` loop body** — a loop-carried cell reads a
displaced value across iterations. This is the same *class* as the A6
checked-Result seam (STEP-0148): a shape the frozen corpora never
exercised, caught by the self-host differential the moment a real consumer
(the token emitter) depended on it.

## 2. Why this matters (the M22 thesis, proven)

This is the exact outcome the M22 self-host track exists to produce: a
real compiler defect, invisible to every frozen corpus, found by a Sico
program doing real work and differentially compared against intent. The
defect is **reproducible** (`target/inline.sico`), **minimised** (single
byte at a boundary-crossing run start), and **layer-isolated**
(application source ✓, slice primitive ✓, **loop-carried cell lowering ✗**).

## 3. Honest register

- The defect is **not fixed in this slice** — it needs a `crates/sico-ir/
  lower.rs` investigation of how `let`-bound cells are read across `while`
  loop iterations (the SSA/cell construction in the loop body), which is a
  bounded but non-trivial compiler change.
- Until fixed, token **spans** (the parallel `List[I64]` offsets) carry the
  same displacement; token **kinds** are unaffected.
- The S5 parser can proceed on **kinds + the clean-run tokens**, with the
  span defect registered, or wait for the lowering fix.

## 4. Residuals

- Fix `let`-cell reads across `while` iterations in `lower.rs` (the
  loop-carried cell SSA construction); re-run `target/inline.sico` to
  confirm `Int` is byte-exact.
- Then token spans are trustworthy and S5 (parser over kinds + spans)
  proceeds on a sound foundation.
