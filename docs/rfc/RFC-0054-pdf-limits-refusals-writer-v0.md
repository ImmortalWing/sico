# RFC-0054: M26 PDF limits and refusal taxonomy v0 (incl. byte-stable writer contract)

> - status: **draft — owner acceptance required** (acceptance does not unlock implementation: PDF implementation waits for M25 v1.0 release GO)
> - date: 2026-09-25
> - phase: M26 §7 kickoff contracts (limits/refusal side)
> - depends: RFC-0053 (reading subset, draft), RFC-0018 lineage (typed Runtime fault taxonomy), M26 plan §3/§5, STEP-0290 inventory

## 1. Summary

Freeze the resource limits, the closed refusal taxonomy, and the
byte-stable writer contract that every later M26 slice (reading,
extensions, merge/rotate) inherits. Limits are checked **before
allocation** from headers/structure; the +1 case of every limit is a
typed refusal, never truncation, OOM, or a partial document.

## 2. Declared limits (v0)

| limit | value | checked from |
|---|---|---|
| input size | 64 MiB | buffer length, before parse |
| object number | 1,048,576 (2^20) | object headers / xref entries |
| xref entries | 1,048,576 | xref subsection header |
| dict/array nesting | 64 | parser depth |
| string length | 1 MiB | string header |
| name length | 512 B | name token |
| page count | 10,000 | page tree walk |
| stream length | 16 MiB (structural audit only in v0) | `/Length` resolution |

## 3. Refusal taxonomy (`PdfError`, closed set, append-only)

`bad-header`, `bad-version`, `missing-startxref`, `bad-xref-offset`,
`xref-loop`, `malformed-object`, `malformed-trailer`, `missing-root`,
`page-tree-loop`, `encrypted`, `unsupported-construct` (carries the
construct name: xref-stream / object-stream / incremental-update /
stream-filter / …), `limit` (carries which limit), `trailing-garbage`
(pre-header junk beyond the spec allowance), `truncated` (EOF mid
structure). Each open/parse failure yields **exactly one** `PdfError`
with a byte span; parse errors never escape as traps, hangs, or panics
(fuzz-style corpus rounds out the matrix).

## 4. Byte-stable writer contract (for the later merge/rotate slices)

- Output is a **classic xref table** document (never xref streams) with
  uncompressed objects in v0; object renumbering is canonical (depth-first
  page-tree order, then remaining objects in ascending original number);
  xref offsets are computed, not cached.
- For a fixed input document and a fixed operation list
  (merge order, rotate angles), the output bytes are **identical across
  runs and hosts**; repeating the same operation on a previous output is
  also byte-identical (idempotence probe in the corpus).
- Non-90° rotations, encrypted inputs, unsupported-construct inputs, and
  limit-exceeding merges are typed refusals — no partial output file is
  ever written; the final write rides the existing file capability with
  its typed save failures.

## 5. Evidence bar (implementation STEP, gated separately)

Positive: frozen generator corpus byte-exact structural reads (RFC-0053).
Refusal: one `PdfError` per case, stable spans, the full §2 limit ladder
including every +1. Writer (later slice): byte-stable merge/rotate on the
frozen corpus with an independent structural re-read. All through the
real runner; machine support matrix row + validator; snapshots
append-only. No general PDF interoperability claim — the supported
subset is exactly §2/§3 of these two RFCs.
