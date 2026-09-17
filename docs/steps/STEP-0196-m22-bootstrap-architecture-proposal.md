# STEP-0196: M22 bootstrap architecture proposal

> - status: complete-contract-accepted (owner directive "完成M22，规划M23-24", 2026-09-16)
> - phase: M22 architecture gate before bootstrap closure
> - completed: 2026-09-16
> - owners: autonomous-agent
> - artifacts: `docs/adr/ADR-0015-compiler-bootstrap-architecture-v0.md`, `tools/validate-step-0196.ps1`, this STEP record

## 1. What was done

The previously open M22 architecture decision is now reviewable as ADR-0015.
The proposal fixes one byte-exact bootstrap route:

- bounded canonical source bundle with path/digest/limit+1 rejection;
- canonical `sico.ir.v0` JSON rechecked and reserialized by the permanent Rust
  verifier before codegen;
- complete Component bytes validated and compared without a semantic-equivalence
  escape hatch;
- Rust→A, A→B, B→C with `A == B == C`, followed by full accept/refuse corpus
  replay;
- B packaged, independently verified and executed through the existing M7
  `.sapp` trust chain;
- one warm-up plus five isolated samples for wall time, fuel, peak guest memory
  and output bytes, with failures retained as failures.

This is contract preparation, not an implementation or support claim. ADR-0015
was accepted by the owner on 2026-09-16; the implementation gates remain to be
closed by executable evidence.

## 2. Validation

- `tools/validate-step-0196.ps1`: checks ADR status and the byte-exact,
  three-generation, verifier, trust-chain, authority and measurement invariants.
- `tools/validate-step-0124.ps1`: M14-M22 planning contract remains green.
- `git diff --check`: required before handoff.

## 3. Residuals

- Implement typed-IR emission and Rust-verifier differential,
  then deterministic codegen, bounded source-bundle decoding and the measured
  bootstrap/package closure in separately allocated STEPs.
- STEP-0195's generic-typed parameter limitation remains until structured type
  parsing replaces the word-pair heuristic.
