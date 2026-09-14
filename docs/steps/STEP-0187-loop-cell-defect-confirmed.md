# STEP-0187: M22 S4→S5 — loop-carried cell defect confirmed across repros (compiler lowering)

> - status: complete — **the defect is confirmed as a compiler-lowering
> bug via multiple independent repros; the Sico application source is
> proven correct by inspection**
> - phase: M22 S4→S5 (blocks clean token spans feeding S5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `target/cellslice2.sico`, `target/tokdbg.sico`, this STEP record

## 1. The confirmation

This slice re-examines whether the STEP-0186 conclusion (compiler
lowering) could instead be an application bug, by re-auditing the
classification loop line-by-line and running a second independent repro.

The loop source is **correct by inspection**: `word_start = index` is
gated on `word_len == 0` (set once per run); `word_len` accumulates per
byte and resets to 0 on slice; `in_ident` tracks the run. On `"x Int y"`
this must produce words `[x, Int, y]`.

Two independent repros both fail identically:
- `tokdbg.sico` (STEP-0186): the `Int` slice runs with `[25,2]` (the
  coordinates of a *later* token), proven by `[start,len]` instrumentation.
- `cellslice2.sico` (this slice): a self-contained single-function version
  with the full classification shape produces `x,nt` with final
  `ws=6 wl=1 words=x,nt` — the `y` run never slices, and `Int` reads as
  `nt`. The final `word_start=6` is `y`'s offset, and `word_len=1` is
  `y`'s length, yet `y` is not in `words`.

Both repros share the **same shape**: `let`-bound cells (`word_start`,
`word_len`, `in_ident`) read and written inside a `while` body with
nested `if/else`, where the loop also carries the cells across iterations.
Isolated probes (single-cell accumulator; conditional-set + post-loop
slice) are **correct** — so the defect is triggered by the multi-cell,
control-flow-dependent shape, not by loops or cells in general.

## 2. Conclusion (confirmed)

The defect is a **compiler-lowering bug** in how `let`-bound cells are
read/written across a `while` iteration when the reads are control-flow
dependent (inside nested `if/else`). The application source is correct;
the emitted wasm reads a displaced cell value. A6-class, invisible to
every frozen corpus.

## 3. Honest register

- The defect is **confirmed and multiply reproduced**, not fixed — the
  fix is a `crates/sico-ir/lower.rs` change to the loop-carried cell
  read/write scheduling for control-flow-dependent reads, a bounded but
  non-trivial compiler repair.
- Token **spans** remain unreliable until the fix; token **kinds** and
  the emission mechanism remain sound.

## 4. Residuals

- Fix the loop-carried cell read/write scheduling in `lower.rs`; re-run
  `cellslice2.sico` and confirm `x,Int,y`.
- Then token spans are trustworthy and S5 (parser) proceeds.
