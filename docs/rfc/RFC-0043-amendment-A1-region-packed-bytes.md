# RFC-0043 Amendment A1: region carried as packed `Bytes` over the v0 package value set (ACCEPTED)

> - status: **accepted by owner re-adjudication 2026-10-01** (raised 2026-09-30, STEP-0333 §B follow-on). The owner accepted **A1 — region/list-of-region as packed little-endian `Bytes`, `image-error` as case-name `Text`** (recommended; reuses the accepted RFC-0050-A1 in-band precedent, zero language/ABI change, roster stays on the M7 install/registry path). This unblocks the three roster packages as **pure `sico:user/` packages**; implementation is **not yet started** — each package still needs its own ADR-0019 Rust core + independent numpy-oracle content corpus + runner content test, one STEP each. **M24 §3 gate 3 / M17 gate-3 remains 未达 until all three land with byte-exact corpora; this acceptance flips nothing by itself.**
> - date: 2026-09-30 (accepted 2026-10-01)
> - amends: [`RFC-0043`](./RFC-0043-deterministic-vision-packages-v0.md) §Summary + §A4 (the `region`-returning roster signatures `image-match@1` / `image-grid@1` / `image-contour@1`)
> - depends: [`RFC-0041`](./RFC-0041-image-data-contract-v0.md) §`record region` (the geometry A1 encodes, does not redefine), RFC-0039 §2.4 (accepted package value set / E8017), [`RFC-0050 Amendment A1`](./RFC-0050-amendment-A1-flattened-surface.md) (the accepted precedent for carrying a `record` in-band as packed `Bytes`)
> - evidence: `crates/sico-semantics/src/lib.rs:745` `interface_value_offender`/E8017 accepted v0 value set = scalars(`Bool/I64/U64/Text/Bytes`) + level-1 `List<Text>` + `Unit` + `Result[level1|Unit|Bool, scalar]` — **no `record`** (measured, STEP-0330 §8)

## 1. Why this amendment exists (measured gate conflict, not preference)

The three roster packages named by M24 §3 gate 3 / §8.3 all return `region`
(or `list<region>`), and none has a scalar/`Bytes`-returning entry point:

```text
image-match@1   : template(bmp, tpl)     -> result<region, image-error>
image-grid@1    : detect-lines(bmp, min) -> list<region>
image-contour@1 : find(bmp, min-area)    -> list<region>
```

`region` is a `record` (RFC-0041 §: `record region { x:u32, y:u32, width:u32,
height:u32 }`). The **accepted** v0 package value set (RFC-0039 §2.4, enforced
by `interface_value_offender`/E8017) permits no `record` in a package
interface — so the accepted RFC-0043 signatures cannot lower through the pure
-package path. Contract and executable gate disagree; per AGENTS.md §1 we keep
the lower support claim (gate 3 stays 未达) and repair the authoritative
document for re-adjudication rather than self-invent a binding. `threshold` /
`color` / `grey8` were already delivered as `image-vision@1` (four `Bytes ->
Bytes` ops) precisely because they avoid `region`; the outstanding gate-3
roster is exactly these three `region`-returning packages.

This is structurally identical to what blocked the codec surface before
[`RFC-0050 Amendment A1`](./RFC-0050-amendment-A1-flattened-surface.md): a
`record` result carried **in-band as packed `Bytes`** is within the accepted
value set and needs no language/ABI change. That amendment was accepted and
unblocked #2/#3.

## 2. A1 — region / list-of-region as packed `Bytes`

`region` is four `u32` = 16 bytes. Freeze a little-endian packed encoding
(no new type, no ABI widening):

```text
region  -> Bytes(16): x u32 LE, y u32 LE, width u32 LE, height u32 LE
list<region> -> Bytes: count u32 LE, then count × the 16-byte region records
```

The flattened package signatures become (all within E8017):

```text
image-match@1   : template(bmp: Bytes, tpl: Bytes)          -> Result[Bytes, Text]   // ok = one 16 B region; err = image-error case name
image-grid@1    : detect-lines(bmp: Bytes, min-run: U64)    -> Result[Bytes, Text]   // ok = count-prefixed region blob
image-contour@1 : find(bmp: Bytes, min-area: U64)           -> Result[Bytes, Text]   // ok = count-prefixed region blob
```

- **Error as case-name `Text`**: the `image-error` enum
  (`dimension-overflow` / `allocation-limit` / `stride-mismatch` /
  `region-out-of-bounds` / `unsupported-format`) is delivered as its case-name
  string in the `Result`'s `Text` arm — the same precedent RFC-0050-A1 §2 and
  the HTTP provider use. No `enum`/`record` crosses the boundary.
- **Structural validation on decode** (fail closed, like the codec header):
  an `ok` blob whose length is `≠ 16` (match) or `≠ 4 + 16·count` (list) is a
  self-describing-corruption typed refusal at the **consumer** side; the
  producer guarantees the framing.
- **Empty result** (`detect-lines` / `find` find nothing): a `count = 0`
  blob (`4` zero bytes) as **ok**, matching §A4's "typed refusals *or empty
  lists*" — degenerate min-area/min-run `limit+1` stays a typed refusal.
- **No semantic change**: SAD template match, deterministic row-major scan,
  ties by lowest `(y,x)`, integer-only, and region containment checks
  (RFC-0041) are all preserved; A1 only fixes the *wire encoding* of the
  result, never the algorithm or the `region` geometry definition.

## 3. Alternatives considered (owner may choose one of these instead)

- **A2 — widen the v0 package value set to allow `record`/`enum`.** Requires a
  new ADR + an E8017/`interface_value_offender` change in `sico-semantics`,
  i.e. editing the *language/package* contract, not just this RFC. Heavier,
  higher blast radius; touches every package consumer.
- **A3 — re-home the roster as a host provider.** Rejected for M24 §8.3:
  shareable, content-asserting CV operations belong on the M7 pure-package
  path (as `image-vision@1` proved); a provider adds authority surface these
  pure functions do not need.

A1 is recommended: smallest, precedent-matched (RFC-0050-A1), no language or
ABI change, keeps the roster on the existing install/registry path.

## 4. What A1 does NOT change

- RFC-0041 `record region` definition and `from-rows`/`crop` semantics.
- RFC-0043 determinism, tolerance policy (exact-integer), scaling limits
  (every `limit+1` typed), authority (pure packages), and the content-asserting
  corpus requirement (Python/numpy fixed oracle, element contents not counts).
- M24 §3 gate 3 remains **未达** until A1 (or a chosen alternative) is accepted
  **and** each package lands its own corpus + STEP. This draft flips nothing.

## 5. Re-adjudication requested (owner)

Accept A1 (packed-`Bytes` region / case-name `Text` error, recommended) or
redirect to A2 (widen value set) or A3. Until then the three roster packages
cannot be built as pure packages on the accepted value set, gate 3 stays
未达, and I will not implement against an un-accepted amendment or silently
substitute a `Bytes` encoding. If accepted, the implementation slices mirror
#2 PNG: a Rust-compiled deterministic core (ADR-0019) + a content-asserting
corpus with an independent numpy oracle + runner content tests, one STEP each.
