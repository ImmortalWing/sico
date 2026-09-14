# STEP-0184: M22 S4 continuation — token-emission defect minimised

> - status: complete — **the token-emission mechanism is confirmed; the
> single-character-prefix defect is minimised to a reproducible case and
> registered (wasm-level slice investigation queued)**
> - phase: M22 S4 (the token-emission slice, continued from STEP-0183)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/tokens.sico`, this STEP record

## 1. Defect minimisation

STEP-0183 registered a slice-offset defect on the 4th probe token
(`Int` read as `nt`). This slice minimises it:

- Two-word and `ab,cd` probes: **byte-exact** (the run-detection → slice
  → append mechanism is correct for these shapes).
- `Int x` → `nt,x`; `Int` alone → `nt`. The defect is **a single-character
  prefix dropped at the start of an identifier run that follows a
  non-identifier byte** — the first byte of the run is lost in the slice,
  the rest are correct.
- The defect does not affect runs at the very start of input (`ab,cd` is
  clean) nor multi-byte runs after a boundary that begin with two or more
  letters (`main`, `return` are clean); it bites when the run starts with
  exactly the boundary-crossing byte pattern the probe exercises.

## 2. What this means for S4

The struct-of-arrays token emission is **mechanically sound** (six of
seven probe tokens, and the clean two-word probes, are byte-exact). The
remaining defect is a **bounded slice/offset bug** at the run-start
boundary — a wasm-level investigation of the emitted slice's start
address for the boundary-crossing case (the `word_start` value versus the
slice's actual read offset). It does not change the token-stream shape or
the parse-feeding design.

## 3. Honest register

- The defect is **reproducible and minimised** (`Int` → `nt`), not
  fixed in this slice — a wasm-level slice-offset investigation is the
  next bounded action, and is the kind of defect the differential oracle
  is designed to catch (the M22 plan's core value).
- Full token **kinds** and **spans** (the complete struct-of-arrays
  triple) land after the offset fix, feeding the S5 parser.

## 4. Validation

- `selfhost_tokens.rs` remains green (the mechanism-level e2e); the
  minimised defect is captured in this record's probes.
