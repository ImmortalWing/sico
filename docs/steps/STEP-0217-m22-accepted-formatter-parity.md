# STEP-0217: M22 accepted-corpus formatter parity

> - status: complete (accepted-output slice; S1 remains partial)
> - phase: M22 S1 L1 formatter
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/formatter.sico`, `runner/sico-runner/tests/selfhost_formatter.rs`, `tools/validate-step-0217.ps1`

## 1. Result

The Sico formatter now implements the Rust formatter's canonical token spacing,
two-space block and match-arm indentation, `effects`/`capabilities` list
indentation, comment placement, blank-line collapse and final-newline policy.
It is compiled with the real Script v0 backend and executed by the real runner.

The differential test selects all 99 STEP-0214 sources accepted by the Rust
formatter. For each source, the Sico formatter output is compared byte-for-byte
with `sico_format::format`; that output is then passed through the same guest a
second time. Results: **99/99 byte-exact and 99/99 idempotent**.

## 2. Honest boundary

This closes accepted-output behavior only. The Sico formatter does not yet run
the full parser/error gate before emitting, so the 116 Rust-refused sources are
not yet proven to refuse with the declared typed identity. S1 and the L1 exit
gate therefore remain partial. STEP-0216's declaration-shape AST also remains a
separate partial S3 result; formatter parity is not evidence of full syntax AST,
checker, lowering, codegen, package, or bootstrap closure.

The next L1 work is the refusal gate and checker diagnostic subset over the same
215-source manifest. The next L2 work remains syntax/refusal AST metadata.
