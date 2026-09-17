# ADR-0015: Compiler bootstrap closure and trust boundary v0

> - status: accepted (owner directive “完成自举”, 2026-09-17)
> - date: 2026-09-17
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -
> - depends: RFC-0008, RFC-0011, RFC-0015, RFC-0016, ADR-0006, M22 plan

## Context

M22 requires a Sico-written Script-profile compiler to compile its own
source without moving compiler semantics, package trust, or Runtime
authority into the guest. The Rust compiler and independent IR verifier
already define the executable oracle; RFC-0011 freezes deterministic
backend bytes; RFC-0015/0016 and ADR-0006 define package and trust
boundaries. The missing decision is what constitutes a reproducible
bootstrap closure rather than a compiler merely compiling one fixture.

## Decision drivers

- byte-level determinism and a fixed-point test;
- no circular trust claim based only on self-produced output;
- reuse of the existing IR verifier, runner limits, and `.sapp` chain;
- explicit measurement of self-compile cost;
- default-deny authority and recoverable rollback to the Rust compiler.

## Considered options

### Replace the Rust compiler after one successful self-compile

Rejected. A single self-produced artifact has no independent semantic
oracle, hides seed dependence, and makes rollback and defect triage
ambiguous.

### Accept semantic equivalence between different Wasm artifacts

Rejected for v0. The repository already has an RFC-0011 byte oracle;
introducing a second equivalence relation before a mismatch exists would
weaken the gate. Any future exception requires an ADR amendment with a
canonical normalizer and adversarial corpus.

### Three-stage byte-identical bootstrap with the Rust implementation retained

Accepted. It proves both cross-implementation agreement and a stable
self-hosted fixed point without treating the self-hosted compiler as its
own root of trust.

## Decision

Let `S` be the frozen, ordered self-host compiler source set and `R` the
pinned Rust compiler from the same revision:

1. `A = R(S)`: build the seed Sico compiler through the normal Rust
   frontend, independent IR verifier, and RFC-0011 backend.
2. `B = run(A, S)`: execute `A` in the normal runner with `S` supplied as
   bounded data; its output is a Component artifact.
3. `C = run(B, S)`: execute `B` under identical inputs and limits.
4. Require `A == B == C` byte-for-byte and validate each artifact as a
   Component. A mismatch is a typed bootstrap failure with stage and
   digests; no artifact is promoted.
5. Run the frozen compiler corpus once with `R` and once with `B`; require
   byte-identical outputs and identical typed refusals. The Rust IR
   verifier remains the oracle and is never rewritten or bypassed.
6. Package `B` using RFC-0015 canonical `.sapp` bytes. Development
   execution uses RFC-0016 explicit local trust; release rehearsal uses
   ADR-0006 role-separated metadata. The compiler guest receives no
   ambient filesystem, network, credential, UI, capture, or input
   authority: sources enter through declared bounded input and artifacts
   leave through declared output.
7. Pin in the evidence manifest: source paths and SHA-256 digests,
   compiler revision, toolchain/runner versions, target triple, limits,
   corpus identity, `A/B/C` digests, package digest, and signer class.
   Unknown fields, duplicate paths, path traversal, truncation,
   trailing data, overflow, and every limit+1 case fail closed.
8. Measure cold and warm wall time, runner fuel, peak guest memory,
   host-call count/fuel, input/output bytes, and timeout/cancellation
   outcome for `B(S)`. These are recorded observations, not an SLA.
   Limits are fixed before the qualifying run and cannot be raised after
   a failure without recording a new attempt.
9. Promotion is additive: the Rust compiler remains the default and
   permanent differential oracle through M22. Selecting the self-hosted
   compiler requires an explicit CLI/profile choice until a later ADR.
   Last-known-good `A` and Rust build paths remain recoverable.

## Consequences

The closure is independently auditable and detects seed drift as well as
self-host instability. It costs one extra self-compile and retains two
implementations. Exact bytes may expose harmless future backend metadata
changes; those changes must update RFC-0011 or amend this ADR rather than
being silently normalized.

## Validation

- STEP-0198 makes the planning validator require this accepted ADR and
  the `A == B == C`, Rust-oracle, `.sapp`, and budget clauses.
- The implementation STEP must provide the executable harness, hostile
  manifest/limit+1 corpus, raw evidence manifest, package inspection,
  real-runner execution, and the M0–M21 regression result.
- Document presence is contract evidence only; it is not bootstrap
  runtime evidence and does not advance M22 to GO.

## Revisit conditions

- RFC-0011 ceases to guarantee byte-identical output;
- the Component toolchain injects unavoidable nondeterministic metadata;
- the self-host compiler needs authority beyond bounded input/output;
- promotion to the default compiler or retirement of the Rust oracle is
  proposed.

## Links

- [`M22 plan`](../plans/M22-compiler-self-host.md)
- [`RFC-0011`](../rfc/RFC-0011-deterministic-core-wasm-backend-v0.md)
- [`RFC-0015`](../rfc/RFC-0015-sapp-package-format-v0.md)
- [`RFC-0016`](../rfc/RFC-0016-development-signing-trust-v0.md)
- [`ADR-0006`](./ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [`STEP-0198`](../steps/STEP-0198-m22-bootstrap-architecture.md)
