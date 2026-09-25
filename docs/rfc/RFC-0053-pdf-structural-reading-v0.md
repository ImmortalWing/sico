# RFC-0053: M26 PDF structural reading subset v0

> - status: **draft — owner acceptance required** (acceptance does not unlock implementation: the route replan gates PDF implementation behind M25 v1.0 release GO)
> - date: 2026-09-25
> - phase: M26 §7 kickoff contracts (data/reading side)
> - depends: RFC-0045 (Bytes surface), RFC-0047 (records/`List[record]`), M26 plan §2/§3/§7, STEP-0290 inventory

## 1. Summary

Freeze the v0 **structural reading subset** of PDF as the versioned
package `sico:user/pdf@1`: open a PDF byte buffer, read its document
structure (trailer, catalog, page tree), and report page-level
geometry/rotation — with **every unsupported construct a typed refusal**,
never a guess. No rendering, no content extraction, no writing in v0
(merge/rotate take their own slice per the plan's contract → reading →
extension → merge/rotate → GUI order).

## 2. Surface

```text
record PdfPageInfo:
  field width:  U64        // MediaBox width in points
  field height: U64
  field rotation: I64      // inherited /Rotate, normalized to 0/90/180/270
end record

record PdfDocument:
  field pdf_version: Text      // "1.4" .. "1.7"
  field page_count: U64
  field object_count: U64      // highest indirect object number seen
  field pages: List[PdfPageInfo]
  field trailer_root_ref: Text // "N 0 R" of the catalog, for audit output
end record

open(data: Bytes) -> Result[PdfDocument, PdfError]
```

## 3. Accepted subset (v0; everything else typed-refused)

- Header `%PDF-1.4` … `%PDF-1.7` (other versions refused, single
  diagnostic naming the version found).
- **Classic xref table** + `trailer` dict + `startxref` (v1.4-style
  layout); free entries `(f)` accepted per spec; xref **streams**,
  object streams, and incremental updates are refused in v0 and carry
  their own extension corpora later (plan §3 slice order).
- Indirect objects `n 0 obj … endobj`: booleans, integers/reals, name
  objects, literal + hex strings, arrays, dictionaries, indirect
  references — parsed structurally. **Streams**: the `stream`/`endstream`
  byte range is recorded for audit but **any stream with a `/Filter` is
  refused** in v0 (`unsupported-filter`); filter-less streams parse.
- Required structure: valid `/Root` reachable catalog with `/Type /Pages`
  tree; `/MediaBox` per page or inherited; `/Rotate` inheritance rules.
- **Encryption is refused** (`encrypted`), as is anything outside this
  list.

## 4. Determinism and evidence

- Pure function of the input bytes: same input → same `PdfDocument`,
  byte-for-byte stable audit output; no hostcalls, no I/O.
- Corpus: the STEP-0290 generator's frozen classic-xref fixtures
  (positive matrix across 1.4–1.7, page-tree shapes, inheritance) plus
  the RFC-0054 refusal matrix; oracle discipline per the inventory
  (generator round-trip until the pinned independent oracle is
  authorized; evidence class internal-fixture; no interoperability
  claim).
- Non-goals (v0): xref/object streams, incremental updates, any filter
  (Flate et al.), encryption, content streams, text/image extraction,
  rendering, writing, linearization hints, digital signatures.
