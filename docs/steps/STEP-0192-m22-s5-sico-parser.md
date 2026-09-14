# STEP-0192: M22 S5 — Sico parser over the token stream

> - status: complete — **S5 opened: the parser consumes the token stream
> (struct-of-arrays words from STEP-0191) and counts declaration keywords,
> byte-exact on the probe**
> - phase: M22 S5 (the parser slice of the L2 compiler self-host)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. What was done

`selfhost/parser.sico` builds on STEP-0191's token emitter: it lexes the
input to a `List[Text]` word stream (the struct-of-arrays token shape),
then walks the stream counting occurrences of the declaration keywords
(`function`, `end`, `record`). This is the first **parser-level** consumer
of the token stream — the S5 slice proves the lexer→parser seam in Sico.

On the probe source `function main() returns Int: / return 42 / end
function` the parser reports `function=2 end=1 record=0` — `function`
appears twice (the keyword and `end function`), `end` once, `record`
zero. The e2e asserts this byte-exact.

## 2. Differential note (honest)

The Rust `sico outline` reports **1** function for the same source (it
counts *declarations*, not keyword occurrences). The Sico parser currently
counts **keyword occurrences** — a deliberately simpler first shape. The
declarations-vs-occurrences gap is the parser's next refinement (matching
a keyword to its declaration context), not a defect: the token stream and
the counting are byte-exact for what they measure.

## 3. Residuals (the S5→S6 boundary)

- Declaration-context matching (pairing `function` with its header, not
  `end function`) needs the token stream to carry **positions** (the
  offset spans from STEP-0191 are emitted but not yet consumed) so the
  parser can distinguish a keyword's syntactic role.
- Full parse (AST shape) needs the token **kinds** (ident vs integer vs
  punctuation) as a parallel list, and the grammar structure over the
  stream — the S6 slice.
- The parser is correct for what it measures; growing it to declaration
  matching is the next bounded step.
