# M26 PDF document processing — reading, merge, rotate

> Status: planned (owner directive 2026-09-24 「确认，要使用sico原生UI库」 — the PDF milestone registered as M26 is confirmed, and its GUI portion must be authored over the Sico-language native UI library, M24 plan §8.6.1; same-day owner directive 「如果原生UI库不完善那就完善M24」 — if the kickoff inventory measures the UI library as insufficient for the PDF tool, the gaps are closed under the M24 §8.6.1 workstream before the M26 GUI gate starts); no STEP numbers reserved

## 1. Objective

Deliver PDF document processing as a versioned package capability, not as
core language syntax and not as new host authority:

1. **Reading (structural)**: parse the PDF object model — header, xref
   table / xref stream, object streams, page tree (catalog → pages →
   page), document metadata, page geometry (MediaBox/CropBox with
   inheritance), raw content-stream byte access.
2. **Merge**: combine multiple input PDFs into one document — page-tree
   grafting, object renumbering, xref rebuild; byte-stable output.
3. **Rotate**: per-page `/Rotate` read and write (90° steps, with
   inherited-attribute resolution); byte-stable output.
4. **GUI pilot**: a PDF tool application authored in Sico over the
   Sico-language native UI library (M24 plan §8.6.1) — file open/save
   dialogs, page list and metadata view, rotate and merge actions,
   typed failures end to end.

Text extraction (content-stream parsing + CMap subset) is a later slice
inside this milestone or a follow-up; it is not in the first increment.
Page rasterization is explicitly out of scope (§4): the GUI shows
structured document information and hosts file operations, it does not
render page pixels.

## 2. Entry gate

- M14 GO ✓（STEP-0141）: byte/text access, streams, bounded recursion,
  dynamic collections and the file capability are executable end to end.
- M7 package/trust/update boundary measured to carry versioned binaries ✓
  (M17 gate-1 evidence, STEP-0166); the PDF package rides the same
  digest discipline.
- RFC-first: the PDF data contract RFC and the limits/refusal contract
  RFC (typed fail-closed for encrypted, corrupt, over-limit and
  unsupported features; no silent fallback) are accepted before any
  implementation STEP.
- GUI prerequisite: the Sico-language native UI library contract is
  frozen (RFC source binding + ADR renderer architecture per M24 plan
  §8.6.1) before the GUI pilot's implementation STEP; GUI evidence
  closes only on the real native renderer, never on a web/webview path.
  Owner scheduling rule (2026-09-24): if the kickoff inventory measures
  the UI library surface as insufficient for the PDF tool (missing
  controls, unfrozen contract or missing renderer evidence), the gaps
  are registered and closed under the M24 §8.6.1 workstream first —
  that workstream's contract-first discipline (RFC + ADR before
  implementation) is unchanged — and the M26 package slices (exit
  gates 1–5, 7) proceed independently while the GUI gate (gate 6)
  waits; no web-UI fallback is permitted at any point.
- Authority: the application runs as a local application under the
  user's authority on the evidence host; file access composes existing
  file capabilities with user-selected paths (dialogs, least-privilege);
  no capture/input grants and no new authority surface.

## 3. Exit gates

1. Contract RFCs accepted: PDF data contract + limits/refusal contract;
   every illegal input class (encrypted document, corrupt xref,
   truncation, trailing data, limit+1 dimensions) maps to a typed
   refusal; no trap, hang, partial output or silent fallback.
2. Reading slice: on a frozen synthetic corpus (single/multi-page, xref
   table and xref stream, object streams, incremental updates) the
   reader's structural output is byte-exact against the oracle; the
   malicious corpus is 100% typed refusals.
3. Merge slice: frozen N-input corpus produces byte-stable outputs;
   object renumbering and xref rebuild are cross-verified against the
   oracle; repeat merge of the same inputs is byte-identical.
4. Rotate slice: 90°-step rotation with inherited `/Rotate` resolution;
   non-90°-multiple requests are typed refusals; outputs byte-stable.
5. Inflate decision recorded: the kickoff inventory measures the pure
   Sico inflate candidate against the deterministic host-provider
   candidate (decompression limits, bomb bounds, performance); the
   chosen contract is frozen in the RFCs before the reading slice that
   needs content-stream decompression lands.
6. GUI pilot: the PDF tool application authored in Sico over the
   Sico-language native UI library builds and runs on the Windows
   evidence host; file open/save dialogs, page list/metadata view and
   rotate/merge actions work over the `pdf-doc` package with typed
   failures; the UI frame corpus is deterministic (fixed client size,
   pinned Fluent color scheme, no timers/animations in the
   operation-relevant regions).
7. Consumer evidence: the package is installed through the M7 registry
   path and consumed without compiler/Runtime changes; M0–M25 regression
   green + explicit exit audit.

## 4. Non-goals

- Page rasterization or visual rendering of page content (registered
  future work, dependent on a renderer contract; until then the GUI
  shows structured information only).
- Content-stream editing, form filling, annotations, digital signature
  creation/verification.
- Opening encrypted documents (typed refusal is the specified behavior).
- OCR and any model-dependent reading path.
- A web/webview UI for the pilot; the GUI is native-only per §8.6.1.
- macOS/Linux/Android UI claims without native evidence.
- External pilot recruitment or third-party adoption claims (owner-gated,
  outside this milestone).
- Any change to core language semantics, compiler surfaces or host
  authority; the PDF package may only *narrow* existing capabilities.

## 5. Risks

- **Spec breadth**: ISO 32000-2 defines far more than this milestone
  needs; the contract RFC must pin the accepted subset (PDF versions,
  xref forms, filter set) and refuse everything else typed. The frozen
  corpus is generated by a fixed writer with Python as the reference
  oracle (the 俄罗斯方块 case precedent), so fixtures pin the accepted
  dialect rather than claim general interoperability.
- **Decompression bombs**: content-stream decompression needs explicit
  output ceilings and nested-limit accounting regardless of the inflate
  candidate chosen; limit+1 cases must be typed, not traps.
- **Incremental updates and object streams** complicate byte-stable
  merge; the contract may pin the accepted input dialect (e.g. refuse
  encrypted, pin max object count) rather than implement the full spec.
- **GUI dependency**: the native UI library is an M24 prerequisite
  workstream; if its contract or renderer evidence is late or measured
  insufficient, the gap items are closed under M24 §8.6.1 per the owner
  scheduling rule (2026-09-24), the package slices (gates 1–5, 7) still
  close and the GUI gate (gate 6) waits — the milestone does not fall
  back to a web UI.
- **Determinism**: merge/rotate outputs must be pure functions of the
  inputs (no timestamps, no random IDs) or byte-stability cannot be
  evidenced; the contract RFC freezes this.

## 6. Evidence classes

`internal-fixture` (synthetic corpora, refusal fixtures, UI frame
corpus) and `clean-room-consumer` (the GUI PDF tool application on the
Windows evidence host) only. Nothing in this milestone constitutes
external adoption, production deployment or cross-platform UI support.

## 7. Kickoff requirement

First STEP is an inventory in the STEP-0129/0142 pattern, before any
package or RFC implementation: measured probes of the current
byte/text/List/Map surface against xref/object-stream parsing needs; a
micro-benchmark of the pure-Sico inflate candidate vs the provider
candidate; the UI-library surface readiness vs the PDF tool's control
needs; and the frozen corpus generation plan (writer version pinned,
oracle documented).
