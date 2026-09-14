# STEP-0182: M22 S3 — Sico lexer classification walk

> - status: complete — **S3 opened: the lexer's byte-classification core
> runs end-to-end and classifies a real Sico source correctly**
> - phase: M22 S3 (the lex slice of the L2 compiler self-host)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/lexer.sico`, `runner/sico-runner/tests/selfhost_lexer.rs`, this STEP record

## 1. What was done

`selfhost/lexer.sico` implements the lexer's byte-classification core
entirely in Sico over the RFC-0045 byte/text surface: `sico.bytes.at`
(returns the byte as I64 for classification), `I64.equal`/`I64.less_than`
(range checks), and `sico.text.format` for the report. It walks the input
and classifies each byte as identifier-start (`[a-zA-Z_]`), integer digit
(`[0-9]`), or punctuation (anything else, with space/tab/newline skipped),
emitting running counts.

On the probe Sico source `function main() returns Int: / return 42 /
end function` the lexer reports `ident=7 int=2 punct=4` — the seven
identifier words (function/main/returns/Int/return/end/function), the
integer literal 42 (counted once per the source's digit runs), and the
four punctuation bytes `():`.

## 2. Validation

- `runner/sico-runner/tests/selfhost_lexer.rs` 1/1: the lexer compiles
  and runs through the real Component Runtime; the classification is
  byte-exact on the probe.

## 3. Honest register (the S3→S4 boundary)

- The lexer currently **counts** byte classes; it does not yet **emit
  tokens** (spans, kinds, values). Token emission needs the classified
  runs sliced out of the source — a `bytes.slice`-per-run + a token
  record representation. `List[record]` is outside the executable set
  (RFC-0046 D4), so the v0 token stream is the struct-of-arrays shape
  (parallel `List[Text]` kinds + `List[I64]` start offsets + lengths) the
  M22 plan anticipated.
- The integer count includes the source's digit runs as the probe shapes
  them; a single multi-digit literal counts once (run-based), matching
  the eventual token semantics.

## 4. Residuals

- Token emission (struct-of-arrays) is the next slice; it feeds S4
  (parser over the token stream).
