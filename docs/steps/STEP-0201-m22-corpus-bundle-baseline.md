# STEP-0201: M22 frozen corpus and canonical source-bundle boundary

> - status: complete
> - phase: M22 closure sequence 1 / ADR-0015 source bundle
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/corpus-v0.json`, `runner/sico-runner/src/bootstrap.rs`, `runner/sico-runner/tests/bootstrap_bundle.rs`, `tools/update-m22-corpus.ps1`, `tools/validate-step-0201.ps1`

## 1. Result

The previously unnamed “frozen corpus” is now a machine-readable inventory of
all 215 `.sico` files under `syntax-candidates/`, `semantic-cases/` and
`tests/end-to-end/`. Every row binds the repository-relative path, byte length
and source SHA-256, plus the current Rust oracle's formatter accept/refuse
result, accepted-output digest and idempotence result, checker accept/refuse
result, exit code, diagnostic identities and normalized diagnostic digest.

Measured baseline:

- Rust formatter: 99 accepted, 116 refused; every accepted output is
  idempotent.
- Rust checker: 65 accepted, 150 refused.

`tools/update-m22-corpus.ps1 -Verify` recomputes all outcomes through the real
`sico` CLI and fails when any source or oracle result drifts.

## 2. Canonical source bundle

The runner now owns the permanent native ADR-0015 bundle decoder. The binary
form has a fixed magic/version, little-endian count/length fields, normalized
UTF-8 repository-relative paths, source bytes and per-entry SHA-256. Encoding
and decoding require strict path order and reject empty/absolute/drive/
backslash/dot-segment paths, duplicates, invalid UTF-8, digest mismatch,
truncation, trailing data and integer overflow.

The accepted bounds are exact: 256 files, 8 MiB per source and 32 MiB aggregate.
Tests cover every limit and limit+1. Decoded entries remain data; the guest
compiler is never given a host path or filesystem authority.

## 3. Evidence and boundary

`bootstrap_bundle` is 4/4 green, including discovery of every manifest path,
all source hashes and byte-exact encode/decode/re-encode. The manifest verifier
re-executes all 215 Rust oracle cases.

This STEP closes only the first item in the STEP-0200 recovery sequence. It does
not claim L1 differential parity, an AST, general typed IR/codegen, A=B=C,
`.sapp` execution or M22 GO. The next implementation target is a lossless
kind/text/start/end token stream and bounded AST over this exact corpus.
