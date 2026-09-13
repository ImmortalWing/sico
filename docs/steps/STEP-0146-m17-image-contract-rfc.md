# STEP-0146: M17 image data contract RFC draft

> - status: complete (RFC drafted; owner acceptance pending)
> - phase: M17 contract track (M17 plan §3.1/§7; the image contract is producer-agnostic and drafted in parallel with M16 per M17 plan §7)
> - completed: 2026-09-08
> - owners: autonomous-agent
> - artifacts: [`RFC-0041`](../rfc/RFC-0041-image-data-contract-v0.md) (draft)

## 1. What was done

The portable image data contract drafted to owner-review quality:
`bitmap`/`region` types with explicit stride/format/orientation; typed
`image-error` for every construction limit+1 case; the four open decisions
resolved into acceptance candidates (D1 packed default, D2 normalized
orientation, D3 explicit color-space conversion only, D4 measured
allocation ceiling); alignment with M16 capture frames (one image
definition, no re-modeling); and the exit corpus (limit+1 fixtures,
byte-exact crop determinism, exact-integer tolerance policy, and the
tetris recognition chain as the content-asserting consumer).

## 2. Validation

Document-only step: no code changed; `validate-step-0124.ps1` and
`git diff --check` green. On acceptance, the deterministic CV package RFC
(M17 plan §3.2 roster with per-package corpora and the numpy oracle
pattern) is the next M17 contract; packages need only this contract plus
the M7 binary-carrying proof, not M16.

## 3. Open decisions left to the owner

- Accept/amend D1–D4 (packing, orientation, conversion, ceiling).
- Accept the error taxonomy and the corpus scope.
