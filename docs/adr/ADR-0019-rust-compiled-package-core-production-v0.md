# ADR-0019: Rust-compiled package core production path v0

> - status: **accepted by owner re-adjudication 2026-09-30 (STEP-0330)** — registered as proposed by STEP-0330, surfaced by the `image-codec-probe` measurement (verdict `package-route-feasible`), then accepted by owner directive; this unblocks the #2/#3 codec implementation slices
> - date: 2026-09-30
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -
> - depends: AGENTS.md §3 (ADR-before-implementation for cross-component architecture), AGENTS.md §4 (compiler/Runtime boundaries), AGENTS.md §7 (deterministic, bounded, reproducible), RFC-0050 §4 (package on the M7 path, "zero compiler/Runtime changes"), RFC-0015/0024/0026 (M7 digest + registry discipline), `prototypes/image-codec-probe` (evidence)

## Context

Every package the repository ships today — `csv_package`,
`table_stats_package`, `image_vision_package` — is a **hand-emitted** wasm
core: `package_builder.rs` builds the core instruction-by-instruction via
`build_package_core`, then wraps it with `build_package_component`. Runner
tests call the Rust producer function directly and assert two builds are
byte-identical.

A real image codec cannot be hand-emitted: PNG deflate/inflate and JPEG
DCT/Huffman are large, correctness-critical, and already implemented by
vetted crates. The M24 `image-codec-probe` measured the alternative: a Rust
crate compiled to `wasm32-unknown-unknown`, wrapped by the **same**
`build_package_component`, executes correctly through wasmtime's canonical
ABI. The probe's component instantiated against an **empty** linker, which
proves the Rust core is **import-free** — so it satisfies the runner's
package purity check (`prepare_package` refuses any import) and rides the
existing package path with no compiler or Runtime change.

What the probe does **not** settle is the *production path*: how a
Rust-compiled core becomes a reproducible, digest-pinned repository artifact
under the `--locked --offline` CI discipline. That is cross-component
architecture and, per AGENTS.md §3, must be frozen by an ADR before the codec
implementation STEP. This ADR freezes it.

## Decision

1. **Two-class package cores, one wrapper.** `build_package_component` stays
   the single Component wrapper. A package core is produced either by
   (a) hand emission (`build_package_core`, existing packages) or
   (b) **Rust compilation** of an out-of-workspace guest crate to
   `wasm32-unknown-unknown` (codec-class packages). Class (b) is the only
   path for codecs; it never replaces class (a) where hand emission is
   feasible.

2. **The Rust guest crate lives outside the root workspace** (the probe's
   `exclude` pattern), with its own `Cargo.lock`, a pinned toolchain
   (`1.98.0` + `wasm32-unknown-unknown`), and a release profile fixed for
   reproducibility (`opt-level=3, lto=true, codegen-units=1, panic="abort",
   debug=0, strip=true`). The root workspace and the compiler/Runtime crates
   gain **no** dependency on it.

3. **The committed artifact is the digest-pinned core `.wasm`, not the
   build.** The repository commits (i) the guest source, (ii) the built core
   `.wasm`, and (iii) its SHA-256, carried through the **existing M7 lock
   discipline** (`sico-lock.json` artifact path + SHA-256). A
   `image_codec_package()` producer in `package_builder.rs` embeds the
   committed core bytes and calls `build_package_component` — mirroring
   `image_vision_package()`'s shape, so consumers and the runner see no
   difference.

4. **Reproducibility is a validator, not a claim.** A
   `tools/validate-package-core.ps1` (probe.ps1 generalized) rebuilds the
   guest from a clean tree and asserts the rebuilt core is **byte-identical**
   to the committed artifact (SHA-256 match). The codec STEP is not green
   unless this validator passes on the evidence host. A crate or toolchain
   bump that changes bytes is a new package version, re-pinned deliberately.

5. **Boundaries honored (AGENTS.md §4/§7).** The guest crate is untrusted
   build input: it is pure (import-free, no ambient FS/net/screen), its
   outputs are bounded by RFC-0050 §3 limits, and it never widens authority.
   Compiler crates, IR, and shared security cores stay free of platform code
   and of any dependency on the guest crate or the codec crates.

6. **Alternatives rejected.**
   - *Hand-emit the codec core* — infeasible for deflate/DCT; correctness
     risk unacceptable.
   - *Vendor the codec crates into the root workspace and build the core
     in-tree* — pulls `png`/`jpeg`/`flate2` (and their transitive graph) into
     the compiler workspace, breaking the §4 boundary and the `--offline`
     root build; rejected.
   - *Host provider instead of a package* — contradicts RFC-0050 §1/§4
     ("zero compiler/Runtime changes", "built on the M7 path") and M24 §3
     gate 5 ("no core compiler/Runtime patches"); the probe shows the package
     route works, so the provider route is unnecessary. (This reverses the
     earlier provisional "host provider" answer, as the "probe first" option
     anticipated.)
   - *Build the core in CI without committing the artifact* — makes the
     package non-reproducible from a clean checkout under `--offline` and
     couples every consumer build to the wasm32 toolchain; rejected in favor
     of committing the digest-pinned artifact.

## Consequences

- The codec line (#2 PNG, #3 JPEG) is unblocked once this ADR is accepted:
  each lands as a guest crate + committed digest-pinned core +
  `image_codec_package()`-style producer + runner content tests + the
  reproducibility validator + a support-matrix row.
- A new binary artifact class enters the repo (committed `.wasm`). It is
  digest-pinned and reproducibility-validated, so it is auditable, but the
  supply-chain review of the codec crates (`png =0.18.1`, JPEG candidates)
  is part of each codec STEP, not deferred.
- The same path generalizes to any future vetted-codec or heavy-algorithm
  package (e.g. M26 PDF), without re-litigating the architecture.
- No implementation claim arises from this ADR until the codec STEP lands
  with real-runner evidence and a green reproducibility validator.
