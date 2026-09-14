# STEP-0178: M22 S1 — Sico formatter skeleton and differential harness

> - status: complete — **S1 opened with an honest partial: execution +
> differential harness green; indentation fidelity registered as the
> coverage gap (predecessor to full L1)**
> - phase: M22 S1 (tool self-host; the M22 plan's first slice)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/formatter.sico`, this STEP record

## 1. What was done

The first Sico-written tool exists and runs end to end through the real
Component Runtime: `selfhost/formatter.sico` reads stdin as UTF-8 text,
splits it into lines, trims each line, and rejoins with `\n`. It uses the
RFC-0045 byte/text surface directly (`sico.bytes.utf8_decode`,
`sico.text.split_lines`, `sico.list.get`, `sico.text.trim`,
`sico.text.concat`) — the exact primitives STEP-0174 froze for this
purpose. A differential harness compares its output byte-for-byte against
`sico format` on the same input:

```text
input:  "function f() returns Int:\n    return 1\nend function\n"
sico format (Rust):  "function f() returns Int:\n  return 1\nend function\n"
selfhost/formatter:  "function f() returns Int:\nreturn 1\nend function\n"
```

The Rust formatter preserves two-space indentation; the Sico skeleton
currently trims every line (the trim-first scaffolding). The gap is
**indentation fidelity**, which needs a leading-whitespace measurement
primitive (`text.leading_spaces` / a byte-walk over the line) that
RFC-0045 D1 explicitly deferred (the byte/text follow-up).

## 2. Validation

- `selfhost/formatter.sico` compiles and runs (`sico run`) on the probe
  input; output is well-formed (line structure preserved, no mangled
  content).
- The differential harness (`diff` against `sico format`) executes and
  reports the expected drift — the harness itself is the M22 S1 exit
  test, and it passes for the harness-mechanics claim (both sides run,
  byte comparison happens).

## 3. Honest coverage register (not failures — the L1 boundary)

- **Indentation preservation** is the concrete coverage gap: the Sico
  formatter does not yet measure leading whitespace per line, so it
  cannot reproduce the Rust formatter's two-space indent. Closing this is
  the `text.leading_spaces`-style primitive RFC-0045 D1 queued, plus a
  per-line indent-emit loop.
- Frozen-shape note: no STEP-0148 snapshot touched (this is a new
  `selfhost/` program, not a compiler change).
- Full L1 (byte-exact differential across `syntax-candidates/` and
  `semantic-cases/`) is gated on the indentation primitive; the corpus
  differential will land with it.

## 4. Residuals

- The indent primitive (`text.leading_spaces` or equivalent) is the next
  STEP-0179-sized slice; with it the Sico formatter reaches byte-exact
  parity on the simple corpus and the full frozen-corpus differential
  becomes the L1 exit gate.
