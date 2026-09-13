# STEP-0162: M15 RFC-0042 v0 UI renderer and hostile/limit corpus

> - status: complete
> - phase: M15 exit gates 2/4 (M15 plan §3.2/§3.4 per accepted RFC-0042)
> - completed: 2026-09-13
> - owners: autonomous-agent
> - artifacts: [`web/ui-corpus.html`](../../web/ui-corpus.html) (self-contained renderer + corpus), generator `crates/sico-cli/tests/generate_ui_corpus.rs` (#[ignore], explicit), evidence [`docs/evidence/m15/ui-corpus.json`](../evidence/m15/ui-corpus.json), validator extension `tools/validate-step-0156.ps1`

## 1. What was done

The RFC-0042 v0 renderer implemented as a host-side DOM renderer for the
web substrate (ADR-0014):

- Closed widget set (container/text/button/text-input/list/image) with
  required stable `id` per node; unknown widgets are a typed
  `E-ui-unknown-widget` refusal.
- Deterministic stack/flow layout attributes; canonical serializer that
  pins roles, aria-labels, text, tabindex focus order and list item
  counts (the render-basic expectation is recorded from the reference
  run and asserted byte-exact — render stability).
- Typed FIFO events: only `click` with a string `node_id` queues; hard
  cap 256 per host turn (`E-ui-rate`); dispatch is strictly FIFO.
- Hostile-content rules: guest strings render via `textContent` only —
  hostile-text case proves `<img onerror>`, `<script>`, `<svg onload>`
  payloads create zero elements/attributes and render literally;
  hostile-url case proves non-https image sources are a typed
  `E-ui-scheme` refusal; size case proves >64 KiB text is typed
  `E-ui-size`.
- Accessibility: ARIA role/name mapping per widget plus tabindex focus
  order, asserted in the DOM.

## 2. Validation

Headless **Microsoft Edge (Chromium) 152.0.4191.66** executed the
self-contained corpus page: **8/8 cases pass**
(`docs/evidence/m15/ui-corpus.json`; per-case output carried in the DOM
and extracted from `--dump-dom`). STEP-0158's audit rows for gates 2/4
were flipped from NO-GO to GO on this evidence; the validator now
asserts the corpus page exists and all cases pass.

## 3. Declared limits (honesty)

- v0 renderer subset: click events only, stack/flow layout attributes
  (grid/image sizing details extend per RFC-0042's own versioning);
  `text-input` renders but the event type set stays click-only in v0.
- Accessibility = ARIA mapping + focus order; a real screen-reader
  walkthrough is NOT claimed.
- Rendering is host-side: guest components supply the declarative tree
  data; live event round-trips into a guest component (beyond the
  harness's typed queue) arrive with the UI-application pilot (M18).
