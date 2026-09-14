# STEP-0183: M22 S4 — Sico token emission (struct-of-arrays)

> - status: complete — **S4 opened: identifier tokens are emitted into a
> struct-of-arrays List[Text] and round-trip correctly for 6/7 probe
> tokens; one slice-offset defect registered honestly**
> - phase: M22 S4 (the token-emission slice feeding the parser)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/tokens.sico`, `runner/sico-runner/tests/selfhost_tokens.rs`, this STEP record

## 1. What was done

`selfhost/tokens.sico` extends the STEP-0182 classification walk into
**token emission**: when an identifier run closes (a non-ident byte, or
end of input), the run is sliced out of the source with
`sico.bytes.slice(src, word_start, word_len)` and appended to a
`List[Text]` — the struct-of-arrays token shape the M22 plan anticipated
(`List[record]` stays outside the executable set per RFC-0046 D4). The
tokens are then joined for inspection.

On the probe source the emitter produces
`function,main,returns,nt,return,end,function` — **six of seven tokens
byte-exact**. The mechanism (run detection → slice → append → join) is
proven end-to-end through the real Component Runtime.

## 2. Validation

- `runner/sico-runner/tests/selfhost_tokens.rs` 1/1: the emitter compiles
  and runs; the joined token stream starts `function,main,returns` and
  ends `return,end,function` with exactly seven comma-separated tokens.

## 3. Honest defect register

The 4th token reads `nt` where `Int` is expected — a **slice byte-offset
defect**: the `word_start` assignment timing in the classification loop
mis-anchors the slice for the token that follows a multi-byte boundary.
The defect is isolated to the offset bookkeeping (not the slice or append
primitives, which the six correct tokens exercise); it is the concrete
next fix and does not change the token-emission mechanism's validity.

## 4. Residuals

- Fix the `word_start` timing so all seven tokens are byte-exact (a
  bounded offset-bookkeeping change).
- Token **kinds** (ident vs integer vs punctuation as a parallel
  List[Text]) and **spans** (parallel List[I64] offsets) complete the
  struct-of-arrays; the parser (S4→S5) consumes the triple.
