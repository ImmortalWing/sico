# M21 Developer experience, standard-library batch 2 and ecosystem activation

> Status: historical GO 6/6 per STEP-0177, with later M20–M21 quality-review findings requiring focused reproduction before v1 freeze; no STEP numbers reserved

## 1. Objective

Convert the completed language/platform arc into a language people enjoy
using: remove the remaining measured usability taxes (checked-arithmetic
ceremony, missing byte/text access, no sorting/formatting stdlib), bring
editor and diagnostic quality to parity with mainstream tools, and seed
a real package ecosystem so the second and third consumers arrive
through the registry rather than the compiler.

## 2. Entry gate

| # | Condition | Status (measured 2026-09-13) |
|---|---|---|
| 1 | M19 CI + release engineering green | **satisfied** — STEP-0168: run-ci green (9 steps), reproducible-build rehearsal OK, registry rehearsal OK |
| 2 | M18 portfolio runs without core patches | **satisfied for four applications** — STEP-0165; unfinished item 4 was later re-designated as the M24 native GUI format converter (STEP-0258) |
| 3 | M20 language v1 decisions landed | **partial** — infix equality landed (RFC-0044); bare-literal typing and for-loops/closures RFCs remain the entry work |
| 4 | Live-model credentials for AI-DX measurement | owner-gated, as M13/M14 |

## 3. Required workstreams

### 3.1 Standard-library batch 2

- Byte/text access: `bytes.at(i) -> u8-result`, `bytes.equal`,
  `text.char-at` — the two pilot friction gaps (STEP-0166 decision point).
- Collections: `list.sort` (deterministic comparator), `list.min/max`,
  `map.values/entries`.
- Formatting: `text.format(template, args...)` replacing concat pyramids.
- Each with content-asserting corpus + oracle per the RFC-0043 pattern.

### 3.2 Language v1 batch 2 (RFC-gated)

- Bare-literal typing decision (bare = I64 with explicit Int literals, or
  keep Int + typed-let syntax) — the measured NUM-001 trade-off.
- For-loops (`for x in expr:`) lowering to the general CFG.
- Error-propagation shorthand (`?`-equivalent) for Result chains.
- Reserved-word migration notes (`task` et al.).

### 3.3 Developer experience quality

- Diagnostic improvement pass: every E-code gets an action-oriented hint
  and a fixture in the diagnostics corpus; measured against the M13
  error-taxonomy classes.
- LSP completion/hover for modules, packages and intrinsics (bounded
  claims only).
- `sico explain <code>` CLI surface.

### 3.4 Ecosystem activation

- Publish the standard library itself as versioned packages through the
  registry (dogfood the M19 bundle path).
- A second clean-room consumer with an independent author (external-pilot
  class) — recruiting and authority are owner-gated.
- Live-model DX measurement (generation/repair on v1 surface) —
  owner-gated credentials.

## 4. Non-goals

- Float arithmetic, ML surface (M17 scope), new host capabilities.
- Breaking the frozen application profile without an accepted RFC.
- Renaming internal-fixture evidence as external adoption.

## 5. Exit gates

1. Stdlib batch 2 shipped with content-asserting corpora; byte/text
   access closed as a friction gap (re-run the tetris reader against the
   new intrinsics — the ASCII-mask workaround retired).
2. Language v1 batch 2 RFCs accepted and landed (bare-literal decision,
   for-loops, error propagation).
3. Diagnostics corpus: every stable code has an action hint; AI repair
   measurement on the v1 surface recorded (offline protocol; live-model
   owner-gated).
4. LSP completion/hover demonstrated against the pilot workspaces.
5. Standard library published as packages and consumed through the
   registry by at least one pilot (ecosystem path proven).
6. Full M0–M20 regression green; exit audit explicit.

## 6. Sequencing rule

No implementation STEP is reserved. First STEP: stdlib batch 2 RFC +
byte/text access implementation (closes the oldest measured gap). Each
subsequent STEP closes one workstream item with cumulative regression
green.
