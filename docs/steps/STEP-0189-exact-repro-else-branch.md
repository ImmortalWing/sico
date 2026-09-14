# STEP-0189: M22 S4→S5 — exact-shape repro preserved (else-branch re-evaluation)

> - status: complete — **the defect is captured in an exact minimal repro
> (`exact.sico`): the flag-if/else + data-dependent set + else-slice shape
> produces `b` for `ab cd`, proving the else branch re-evaluates the slice
> with a displaced cell**
> - phase: M22 S4→S5 (blocks clean token spans feeding S5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `target/exact.sico` (exact-shape repro), this STEP record

## 1. The exact repro

Bisecting from `withflag`, every simplification that removes a structural
element is correct; the **exact combination** triggers the defect. The
repro `exact.sico` on input `"ab cd"`:

```text
shape: flag-if/else (is_ident from bytes.at)
       + then: word_start=index, word_len accumulate
       + else: inner if(word_len==0){} else { slice(word_start, word_len); word_len=0 }
result: b        (expected ab,cd — the else branch re-slices with a displaced word_start)
```

The simpler shapes are all correct: `oneif` (single-if + post-loop let),
`inloop` (two single-ifs + in-loop slice), `nested` (nested if/else +
slice), `innerif` (cell-set + inner if/else), `cellstart`/`litstart`
(cell-start / literal-start + slice), `twoif` (two independent ifs + both-
cell slice), `flagdrive` (flag + single-if gate). Only `exact`'s full
shape — flag-if/else with the slice in the **else** arm and a data-
dependent set in the **then** arm — produces the displaced read.

## 2. Conclusion

The defect is a **compiler-lowering bug in the if/else arm construction**
when one arm writes a cell and the other arm reads it for an intrinsic
argument: the else arm's slice reads `word_start` before the then arm's
write has landed across the arm boundary, or the else arm re-enters with
a stale cell. The exact repro distinguishes it from every simpler (correct)
shape, pinning the blast radius to the if/else arm cell-passing rule.

## 3. Honest register

- The exact repro is **preserved** (`target/exact.sico`), not fixed — the
  fix is a `crates/sico-ir/lower.rs` change to the if/else arm cell
  passing (the `bindings`/`cells` snapshot-restore across arms), a bounded
  but delicate compiler repair that must keep all simpler probes green.
- Token **spans** remain unreliable until the fix; **kinds** and the
  emission mechanism remain sound.

## 4. Residuals

- Fix the if/else arm cell passing in `lower.rs`; confirm `exact.sico` →
  `ab,cd` and every simpler probe stays green; then token spans are
  trustworthy and S5 proceeds.
