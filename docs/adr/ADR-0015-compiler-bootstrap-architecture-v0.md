# ADR-0015: Compiler bootstrap architecture v0

> - status: accepted (owner directive "完成M22，规划M23-24", 2026-09-16)
> - date: 2026-09-16
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -

## Context

M22 rebuilds the Script-profile compiler in Sico while retaining the Rust
compiler and `sico-ir` verifier as permanent oracles. S1-S5 established a
runner-executed Sico formatter, checker, lexer and parser path, but bootstrap
closure needs one cross-component decision before typed-IR and codegen work can
be treated as the route to S6: what bytes cross the differential boundary, how
self-compilation reaches a fixed point, where untrusted compiler output is
validated, how the result enters the M7 package trust chain, and how budgets are
measured without turning a local fixture into a product claim.

The existing contracts already constrain the choice:

- RFC-0008 makes canonical `sico.ir.v0` JSON plus the independent Rust verifier
  the typed-IR boundary.
- RFC-0011 makes deterministic Core Wasm bytes, verifier-before-codegen and
  real binary validation mandatory.
- M7 makes `.sapp` verification, not a loose Component, the packaged execution
  boundary.
- M10-M12 keep runner lifetime, limits, cancellation and Host authority outside
  the compiler.
- The M22 plan keeps Rust as the permanent differential oracle and forbids a
  de-Rust support claim.

## Decision drivers

- A bootstrap result must be reproducible from recorded bytes, not inferred
  from source acceptance or a successful `sico check`.
- A compiler compiled by itself is an untrusted producer until the native
  verifier and binary validators accept its output.
- The harness must not grant filesystem, network, credential or process
  authority merely because the guest is a compiler.
- The equivalence rule must be machine-decidable and stable across Windows and
  Linux runners.
- Performance evidence must distinguish a hard safety limit from a measured
  regression budget.

## Considered options

### Option A — byte-exact three-generation fixed point

The Rust implementation builds generation A. A compiles the same canonical
source bundle to B, and B compiles it to C. Canonical IR and final Component
bytes must be equal for A/B/C at their respective comparison points. Each
guest-produced boundary is independently validated before it can become the
next compiler generation.

This is strict, simple to audit and aligned with RFC-0008/RFC-0011. It may
require eliminating harmless serialization differences instead of accepting
them, but that work is exactly the determinism claim M22 exists to prove.

### Option B — semantic or normalized equivalence

Allow different Component bytes when disassembly, execution or a normalization
pass judges them equivalent. This reduces implementation friction but creates a
second compiler-like trusted component, weakens the existing byte oracle and
makes reproducibility depend on the normalizer. Rejected for v0.

### Option C — replace the Rust verifier with the Sico implementation

Use self-hosting as the point where the native verifier is retired. This makes
the bootstrap circular and violates the M22 dual-implementation boundary.
Rejected permanently for this ADR version.

## Decision

Choose **Option A**. The owner accepted this ADR with the 2026-09-16
directive to complete M22 and plan M23-M24.

### 1. Canonical source bundle

Every differential or bootstrap run receives one bounded, data-only source
bundle. Entries are ordered by normalized repository-relative UTF-8 path and
contain path length, source length, source bytes and SHA-256 digest. Paths must
be non-empty, slash-separated and free of absolute roots, drive prefixes,
backslashes, `.` or `..` segments and duplicates. The decoder rejects unknown
fields, invalid UTF-8 paths, digest mismatch, truncation, trailing bytes,
overflow and every limit+1 case. No entry is materialized as a host path by the
compiler guest.

The initial v0 limits are 256 files, 8 MiB per file and 32 MiB total source
bytes. Changing these limits requires an ADR amendment plus limit and limit+1
fixtures.

### 2. Differential boundaries

The lowering boundary is canonical `sico.ir.v0` JSON bytes as defined by RFC-0008. The Sico
implementation emits those bytes; the native harness deserializes them with
unknown-field rejection, runs the Rust `sico-ir` verifier and reserializes the
accepted module canonically. The emitted and reserialized bytes must be equal.
Only then may the module enter either backend.

The codegen boundary is the complete Component byte vector. Both the Rust and
Sico outputs must pass `wasmparser` validation and the selected real runner's
prepare/link path. Equality means byte-for-byte equality of the complete
artifact; timestamps, absolute paths, random salts and unordered map iteration
are forbidden inputs. There is no semantic-equivalence exception in v0.

Diagnostics remain a separate refusal corpus. A refused source must yield the
same declared diagnostic identity and must produce neither verified IR nor a
Component. Successful compilation does not erase or downgrade a disagreement
on the refusal corpus.

### 3. Three-generation bootstrap

For one pinned source bundle and one pinned toolchain/lock set:

1. The Rust compiler builds the Sico compiler source into **A**.
2. A runs through the real runner, consumes that identical bundle and emits
   **B**.
3. B is validated, then runs through the same runner and emits **C**.
4. The gate is `A == B == C` for complete Component bytes. Canonical IR bytes
   for the compiler source must likewise match the Rust oracle before B is
   admitted.
5. B re-runs the frozen accept/refuse corpus. Every accepted case must match the
   Rust compiler's canonical IR and Component bytes; every refused case must
   preserve the declared diagnostic identity and no-artifact result.

A failed equality, validation, refusal or corpus comparison is a NO-GO result;
the harness retains hashes and bounded diagnostics but never promotes the
candidate generation.

### 4. Trust and execution boundary

Guest compiler stdout is untrusted bytes. Before those bytes are loaded or
packaged, the native harness enforces the output limit, validates the canonical
IR or Component form and verifies its digest. It never turns guest text into a
command, host path, URL, permission or support claim.

After the fixed point and corpus replay pass, B is packaged through the existing
M7 `.sapp` producer using only the repository's local fixture identity. The
package is then independently verified and executed through `sico-app`; direct
execution of a loose B artifact does not close the packaging gate. This is
`internal-fixture` evidence only and grants no production publisher, registry
or external-pilot status.

The compiler run receives no network, storage, credential, UI, capture or input authority.
Runner cancellation, fuel, epoch timeout, memory/output bounds and
typed outcomes remain native responsibilities. The self-hosted compiler does
not embed or replace runner/Host policy.

### 5. Measurement policy

The closure run records exact toolchain versions, runner/platform, source-bundle
digest, A/B/C digests, package digest, limits and raw per-run evidence. After
one untimed warm-up, five isolated runs measure wall time, fuel, peak guest
memory and output bytes for A→B and B→C. The report records every sample plus
median and maximum; a timeout, cancellation, resource exhaustion or missing
counter is a typed failed sample, not discarded noise.

Runner hard limits are pass/fail safety gates. Comparison with the M19 compile
throughput evidence is a reported regression ratio, not an SLA or byte-equality
waiver. Any future performance budget promoted to a release gate requires a
separate accepted amendment backed by measurements on each claimed platform.

## Consequences

- Sico lowering and codegen can advance only against stable native verifier and
  byte oracles; a locally plausible AST summary is not bootstrap progress by
  itself.
- Byte-exact closure may expose nondeterministic metadata or iteration order.
  Those are compiler defects to remove, not reasons to weaken the gate.
- The source-bundle decoder and limit+1 corpus become security-critical native
  harness code and must land with the first multi-file bootstrap STEP.
- Rust compiler, verifier and backend stay buildable and tested after M22 GO.
- The accepted ADR unlocks the bounded typed-IR/codegen/bootstrap STEPs; it does
  not by itself constitute implementation evidence.

## Validation

The contract is machine-checked by `tools/validate-step-0196.ps1`. Implementation
validation consists of the bounded
decoder/refusal corpus, canonical-IR differential, A/B/C equality, frozen corpus
replay, M7 package verify/execute and the five-sample measurement record.

## Revisit conditions

- RFC-0008 changes the canonical IR schema or serialization.
- RFC-0011 changes deterministic artifact layout.
- A supported platform proves byte equality impossible because of an accepted,
  unavoidable platform field; the amendment must name and validate the exact
  field rather than introduce general semantic equivalence.
- M7 changes the package or signing boundary.
- Measurements justify a cross-platform release performance budget.

## Links

- [`M22 plan`](../plans/M22-compiler-self-host.md)
- [`RFC-0008`](../rfc/RFC-0008-typed-sico-ir-contract-v0.md)
- [`RFC-0011`](../rfc/RFC-0011-deterministic-core-wasm-backend-v0.md)
- [`STEP-0196`](../steps/STEP-0196-m22-bootstrap-architecture-proposal.md)
