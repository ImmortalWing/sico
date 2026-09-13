# RFC-0042: Web UI controls and events contract v0

> - status: accepted (owner session directive "完成M15-17", 2026-09-10; M15 plan §3.2)
> - date: 2026-09-10
> - depends: ADR-0014 (substrate), RFC-0039 (source modules/user WIT), M10 redaction rules, RFC-0037 (authority shapes)

## Summary

Freeze the deterministic, AI-writable control tree for the Web Host v0:
a closed widget set with stable identity, a fully specified layout
subset, typed events with deterministic dispatch, explicit property
assignment only, and accessibility as first-class contract fields. The
tree is application-owned state rendered by the Host — never ambient DOM
ownership; guest strings never become unchecked HTML (hostile-content
corpus, M15 exit gate 4).

## 1. Widget set (v0, closed)

`container` (stack/flow/grid layout, fully specified below), `text`,
`button`, `text-input`, `list`, `image`. Every node carries:

- `id`: application-assigned, unique within the tree, stable across
  updates (the identity anchors events, focus and ARIA);
- `properties`: typed per widget (text content, placeholder, source,
  disabled, item lists) — assignment only, no reactive bindings;
- `accessibility`: `role` (default per widget), `name`, `focus_order`
  (host-validated total order).

Everything outside the closed set is a typed refusal (`E-ui-unknown-widget`,
renderer-side, declared in the M15 matrix).

## 2. Layout (deterministic subset)

- `stack`: children in declaration order; vertical; each child's
  border box exactly `width = container inner width`, stacked
  top-to-bottom; margins are contract fields (numbers, not CSS).
- `flow`: children left-to-right, wrapping by measured width against the
  container inner width; the measurement rules are the fixed per-widget
  metrics table in the renderer (no font-dependent surprises in v0).
- `grid`: explicit `columns` count; children fill row-major; cell size =
  inner-width / columns. No spans, no auto-flow variants in v0.

CSS parity is explicitly future scope (M15 plan §3.2); nothing here
reads or emits stylesheet text.

## 3. Events (typed, deterministic)

- `click` (button), `change` (text-input commit on Enter/blur), `select`
  (list item). Payloads are typed structs `{ node_id, kind, value? }` —
  no raw DOM events, no ambient listeners, nothing else subscribable.
- Dispatch order: queue order, FIFO, one event per host turn; the
  application handles each event by returning an updated tree
  (property mutations only). M10 redaction applies to any event content
  that leaves the run boundary.

## 4. State

Explicit property assignment in the returned tree only. No two-way
binding, no diffing magic, no framework semantics: the Host re-renders
the tree deterministically from the assignment, and identical trees
produce identical DOM (corpus-asserted).

## 5. Accessibility

`role` maps to ARIA roles 1:1 (`button`, `textbox`, `list`, `listitem`,
`img`, `group`); `name` maps to accessible name; `focus_order` drives
tab order. IME composition is labelled per engine by the harness, never
silently fallback-rendered (M15 §3.2).

## 6. Hostile-content rules (exit gate 4 corpus)

Guest strings are rendered as text nodes, never markup; URLs render only
under an explicit `source` property validated to `https` scheme; sizes
are bounded (text length, list length, image byte size — limit+1 typed);
event flooding is rate-limited Host-side. Every rule is a committed
corpus case.

## 7. Non-goals (v0)

Styling/CSS parity, reactive state frameworks, ambient DOM access,
script injection, canvas rendering, custom widgets.
