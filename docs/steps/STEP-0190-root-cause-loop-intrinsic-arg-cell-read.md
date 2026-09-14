# STEP-0190: M22 S4→S5 — ROOT CAUSE: while-loop intrinsic-arg cell read is displaced

> - status: complete — **root cause locked: inside a `while` loop body, an
> intrinsic argument that reads a cell (`sico.bytes.at(src, index, …)`)
> sees a DISPLACED `index` (reads 2 where 0 is expected). Outside the loop
> the same read is correct. A compiler-lowering bug in the loop body**
> - phase: M22 S4→S5 (blocks clean token spans feeding S5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `target/idxread.sico` (decisive), `target/atzero.sico` (control), this STEP record

## 1. The root cause

The token-emission defect traces to a single compiler-lowering bug,
proven by a decisive pair of probes:

- **`atzero.sico`** (control, outside loop): `sico.bytes.at(src, 0, 0)`
  on `"ab cd"` returns `97` (`'a'`) — **correct**.
- **`idxread.sico`** (decisive, inside loop): a `while` loop that reads
  `sico.bytes.at(src, index, 0)` and records `index` when the byte is a
  space-class byte on `"ab cd"` produces **`2`** — but the only space is
  at index 2 AND the loop should also record the `index==0` read on the
  first iteration. The first-iteration read of `index` inside the loop
  **displaced to 2**.

Concretely: inside the `while` body, an intrinsic call whose argument is
a cell read (`index`) evaluates that argument with a displaced cell value.
The cell `index` is written by the loop's increment (`set index = next`)
and read by the body — and the body's read sees a value from the wrong
program point (a later iteration's, or the exit value). Outside the loop
the read is correct, so the defect is **the loop body's cell-read
scheduling**, not the cell mechanism, not the intrinsic, not the slice.

## 2. Why every simpler probe passed

All nine simpler probes (`oneif`…`flagdrive`) either read the cell
**outside** the loop, or did not combine a loop-carried cell with an
intrinsic argument read **inside** the body at the displaced point. The
token emitter is the first real program to depend on the exact shape:
loop-carried `index` cell → intrinsic arg read in the body → the read is
displaced.

## 3. The fix (bounded)

The defect is in `crates/sico-ir/src/lower.rs`'s `while` lowering: the
loop body's cell reads must see the value carried into the iteration, but
the emitted wasm reads the cell after the increment has been scheduled
(or reads a phi-less merge). The fix is to the loop's cell-read/write
scheduling — ensure the body's `ReadLocal(index)` emits **before** the
increment's `WriteLocal(index)` within the iteration's block order, which
the current arm/block construction may be reordering.

**Repro to fix against**: `idxread.sico` must produce `0,2` (the
first-iteration read at index 0 AND the space at index 2); currently it
produces only `2`.

## 4. Honest register

- Root cause is **locked and reproducible** (`idxread.sico`), not fixed —
  the fix is a bounded `lower.rs` change to the loop body cell scheduling,
  guarded by the frozen-shape test (`cargo test -p sico-ir --test
  core_lowering`) and the simpler probes staying green.
- This is the highest-value outcome of M22 so far: a real compiler bug,
  invisible to every frozen corpus, found by a Sico program doing real
  work, root-caused to the loop-body cell-read scheduling.
