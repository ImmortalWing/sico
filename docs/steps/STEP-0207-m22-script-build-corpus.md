# STEP-0207: M22 Script v0 build corpus and artifact oracle

> - status: complete (native oracle baseline; S4-S6 remain open)
> - phase: M22 closure sequence / L2 corpus
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/script-build-corpus-v0.json`, `selfhost/corpus-v0.json`, `tools/update-m22-corpus.ps1`, `tools/validate-step-0207.ps1`

## 1. Result

The frozen source manifest now has a separately versioned, digest-bound Script
build companion recording the real Rust `sico build --profile script-v0`
outcome. Keeping `sico.m22.corpus.v0` unchanged preserves its strict
`deny_unknown_fields` contract. Successful
entries carry the complete Component SHA-256; refusals carry stable diagnostic
classes and are asserted to leave no artifact.

Every successful source is built twice in an isolated temporary directory and
the complete Component bytes must match before the manifest can be written or
verified. The measured baseline is:

- **37/215 accepted**, each with a non-null Component SHA-256 and 37/37
  byte-reproducible second builds;
- **178/215 refused**, each with no artifact: 116 `LEXICAL`, 28 `SCRIPT_ABI`,
  and 34 declared semantic diagnostic identities;
- zero accepted entries without a digest and zero refused entries with a
  digest.

This is now the artifact/refusal oracle the Sico compiler must reproduce; it
replaces any inference from `check` acceptance.

## 2. Honest boundary

This STEP freezes and replays the native oracle. It does not claim the current
Sico compiler can compile the 37 accepted sources: STEP-0197 and STEP-0199 are
still bounded IR/fixed-codegen seams. Canonical IR parity, guest-produced full
Component equality, A=B=C, package closure and budgets remain open.
