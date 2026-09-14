# STEP-0188: M22 S4→S5 — minimal repro: flag-cell + data-dependent set (compiler lowering)

> - status: complete — **the defect is minimised to a minimal repro
> (`withflag.sico`): a flag cell driven by `bytes.at` + a data-dependent
> `word_start` set together trigger the loop-carried cell misread**
> - phase: M22 S4→S5 (blocks clean token spans feeding S5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `target/withflag.sico` (minimal repro), this STEP record

## 1. The minimal repro

Bisecting from the full classification loop, every simplification that
**removes either factor** is correct; the combination triggers the defect:

| Probe | Shape | Result |
|---|---|---|
| `oneif` | single-if cell-set + post-loop `let` + slice | ✓ correct |
| `inloop` | two single-ifs + in-loop slice (no nesting) | ✓ correct |
| `nested` | nested if/else + in-loop slice | ✓ correct |
| `innerif` | cell-set + inner if/else (else slices) | ✓ correct |
| **`withflag`** | **+ `in_ident` flag cell set from `bytes.at`, driving a data-dependent `word_start` set** | ✗ `x,nt` |

`withflag.sico` is the minimal trigger: a flag cell (`in_ident`) whose
value is computed from `sico.bytes.at`, combined with a `word_start` set
gated on `word_len == 0` (data-dependent), inside the `while` body. On
`"x Int y"` the source is correct by inspection (the flag tracks the run;
`word_start` is set once per run at `word_len == 0`), yet the run yields
`x,nt` — the `Int` slice reads a displaced `word_start`.

## 2. Why this is the compiler (final)

The source is minimal enough to audit exhaustively: the flag logic, the
`word_start` gate, and the slice are each correct in isolation (the
simpler probes prove it). The defect emerges only when the **flag cell
and the data-dependent cell-set coexist** in the loop — the compiler's
read/write scheduling for these two interacting cells across the
iteration reads `word_start` before the flag's effect on it has landed.
This is the A6 class precisely.

## 3. Honest register

- `withflag.sico` is the **repro to fix against**: a `lower.rs` change to
  the loop-carried cell scheduling for interacting cells (flag + data-
  dependent set) should flip it to `x,Int,y` without disturbing the
  simpler probes (all currently green).
- The fix is bounded (one scheduling rule) but the blast radius is the
  whole loop lowering; it needs the frozen-shape guard (`cargo test -p
  sico-ir --test core_lowering`) green after.

## 4. Residuals

- Fix `lower.rs`; confirm `withflag.sico` → `x,Int,y` and all simpler
  probes stay green; then token spans are trustworthy and S5 proceeds.
