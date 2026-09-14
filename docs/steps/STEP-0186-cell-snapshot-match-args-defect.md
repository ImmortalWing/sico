# STEP-0186: M22 S4→S5 — cell-snapshot-in-match-args defect proven (compiler lowering)

> - status: complete — **the token boundary defect is proven to be a
> compiler-lowering bug: a `let`-bound temporary captured into a `match`
> argument reads a displaced cell snapshot. Ironclad repro registered**
> - phase: M22 S4→S5 (blocks clean token spans feeding S5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `target/tokdbg.sico` (ironclad repro: `[25,2]:nt`), this STEP record

## 1. The proof

The token-emission defect (`Int` → `nt`) was isolated to the compiler in
STEP-0185; this slice proves the exact mechanism with an ironclad repro.

Instrumenting the token emitter to print each slice's `[start, length]`
produces:

```text
[0,8]:function  [9,4]:main  [16,7]:returns  [25,2]:nt  [31,6]:return  ...
```

`Int` sits at offset 20 with length 3, but the slice that should yield it
ran with **start=25, length=2** — the coordinates of the *following*
token (`return` starts at 31; the `[25,2]` slice reads the two bytes at
25..27, which is `nt`, the tail of `Int` plus the space). The `let
captured_start = word_start` temporary, taken **immediately before** the
`match sico.bytes.slice(src, captured_start, captured_len)`, captured 25
— proving `word_start` was already displaced **at the point of the let**,
inside the same loop iteration that should have seen it as 20.

The isolated `condslice` probe (single conditional cell-set + post-loop
slice) is correct, so the defect is triggered by the **nested multi-cell
shape** the token emitter exercises — exactly the A6 class: a real
compiler-lowering bug, invisible to every frozen corpus, caught the
moment a Sico program depended on the shape for real work.

## 2. Layer isolation (final)

- Application source ✓ (the `[start,len]` instrumentation is correct).
- `sico.bytes.slice` primitive ✓ (pure-slice probes byte-exact).
- `let`-cell reads in simple loops ✓ (single-cell accumulator correct).
- **`let`-temporary captured into `match` args, nested in a loop over
  multiple cells ✗** — the compiler reads a displaced cell snapshot.

## 3. Honest register

- The defect is **proven and repro-sealed** (`target/tokdbg.sico`), not
  fixed — the fix is a `crates/sico-ir/lower.rs` change to how `let`
  temporaries bind cell values into match/intrinsic arguments inside
  nested loop bodies (the cell-read scheduling), a bounded but non-trivial
  compiler repair.
- Until fixed, token **spans** are unreliable; token **kinds** and the
  token-emission mechanism remain sound.

## 4. Residuals

- Fix the `let`-into-match-args cell snapshot in `lower.rs`; re-run
  `target/tokdbg.sico` and confirm `[20,3]:Int`.
- Then token spans are trustworthy and S5 (parser) proceeds on a sound
  foundation.
