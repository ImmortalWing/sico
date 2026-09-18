# STEP-0218: M22 formatter full-corpus refusal gate

> - status: complete (S1 formatter complete)
> - phase: M22 S1 L1 formatter
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/formatter.sico`, `runner/sico-runner/tests/selfhost_formatter.rs`, `tools/validate-step-0218.ps1`

## 1. Result

The guest formatter now performs a source-content lexical refusal pass before
formatting. It uses the same token classification path as its formatter rather
than corpus paths or a manifest allowlist. Every STEP-0214 source refused by the
Rust formatter contains a lexical error and is returned as the declared typed
guest domain result `invalid-input / LEXICAL`.

The real Script v0 build and runner test now covers the complete 215-source
manifest:

- 99/99 Rust-accepted sources produce byte-exact canonical output;
- those 99 outputs are 99/99 idempotent through a second guest run;
- 116/116 Rust lexical refusals produce the declared typed guest refusal and no
  formatter output.

This closes the bounded S1 formatter slice.

## 2. Honest boundary

The guest refusal intentionally exposes the stable class `LEXICAL`, not the
Rust formatter's internal lexical-error count. Exact per-error spans and lexer
error kinds remain part of S3 syntax/refusal AST work. S1 completion does not
close the L1 checker subset, S3 parser, typed IR, codegen, A=B=C, package, budget
or M22 exit gates.
