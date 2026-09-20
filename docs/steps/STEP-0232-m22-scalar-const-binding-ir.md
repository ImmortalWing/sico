# STEP-0232: M22 scalar constant bindings in the loop-driven straight environment

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0231
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0232.ps1`

## 1. Objective

Unify scalar constant (`Int`/`Bool`/`Text`) bindings with the loop-driven
straight binding environment so constant bindings are not restricted to the
return-expression special paths.

## 2. Contract and mechanism

Any `let` binding in a three-or-more-binding straight block may now bind a
bare scalar constant: a canonical magnitude (`const_int`, kind `int`),
`true`/`false` (`const_bool`, kind `bool`) or a string literal
(`const_string`, kind `string`, JSON-escaped and `\0`-decoded exactly as the
return path). The constant instruction takes the next SSA id, its range is
the literal token span, and the binding records the constant kind and value
id; the next statement starts from the following SSA id. Constants of one
width cannot feed fixed-width operations or calls expecting another width —
position type checking still applies.

Non-canonical magnitudes (leading zeros), unknown bare atoms, invalid string
escapes and unclosed strings remain typed refusals; bare alias bindings
(`let y = param`) are still outside the executed subset.

## 3. Executable evidence

Six positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and match
Rust `lower_core` byte-for-byte: a lone integer constant returned directly;
an integer constant consumed by a typed user call with an int literal
argument; bool constants consumed by a call with a `false` literal; an
escaped string constant consumed by a Text call; and a mixed
int/bool/unicode-text block feeding a three-arity call. Three negative
fixtures prove an unknown bare atom, an invalid string escape and a
non-canonical integer constant fail closed with typed refusals. The
cumulative positive set is 64 programs.

## 4. Residuals

Bare alias bindings (`let y = param`) and scalar constants returned
directly from long-block return statements are not yet unified. Mutation,
nested control flow, alternate match-arm shapes, full-corpus lowering, S5
codegen and S6 bootstrap remain open.
