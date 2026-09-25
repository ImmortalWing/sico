# RFC-0052: Sico-language native UI library — source binding v0

> - status: **draft — owner acceptance required before any implementation STEP** (pairs with [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md), proposed)
> - date: 2026-09-25
> - phase: M24 §8.6.1 prerequisite workstream (source side)
> - depends: RFC-0039 (modules/packages/user WIT), RFC-0041 (BGRA8 image contract), ADR-0018 (renderer architecture, proposed), M24 plan §8.6.1/§8.6.2, STEP-0287 inventory

## 1. Summary

Freeze the compiler-facing UI vocabulary as a **versioned package,
`sico:user/ui@1`**, authored surface-side so Sico source builds and runs
native Fluent-subset UI with **zero grammar growth** — no new syntax, no
new keywords, no lexer/parser changes. The vocabulary covers the M24
pilot's needs and the M26 PDF tool's declared controls: window, layout
panels (column/row), text, button, dropdown/selection, file open/save
dialogs, image preview, progress indication, and pointer/click events.

## 2. Binding surface (v0 closed set)

```text
record UiHandle: field id: U64 end record
record UiPoint:  field x: I64  field y: I64  end record

// tree construction (parent must be a window or panel handle)
window(title: Text, width: I64, height: I64)      -> Result[UiHandle, UiError]
column(parent: UiHandle)                          -> Result[UiHandle, UiError]
row(parent: UiHandle)                             -> Result[UiHandle, UiError]
text(parent: UiHandle, content: Text)             -> Result[UiHandle, UiError]
button(parent: UiHandle, label: Text)             -> Result[UiHandle, UiError]
dropdown(parent: UiHandle, options: List[Text])   -> Result[UiHandle, UiError]
image_preview(parent: UiHandle, w: U64, h: U64)   -> Result[UiHandle, UiError]
progress(parent: UiHandle)                        -> Result[UiHandle, UiError]

// state + events
set_text(handle: UiHandle, content: Text)         -> Result[Bool, UiError]
set_progress(handle: UiHandle, done: U64, of: U64) -> Result[Bool, UiError]
draw_image(handle: UiHandle, w: U64, h: U64, pixels: Bytes) -> Result[Bool, UiError]
   // pixels: RFC-0041 BGRA8 — composes with image-vision@1 / image-codec@1
poll_event() -> Option[UiEvent]   // UiEvent: click(handle) | select(handle, index) | closed
run() -> Result[Bool, UiError]    // present one frame; frame = pure function of UI state
```

- **Dialogs are not package functions**: file open/save compose the
  existing least-privilege file capabilities with user-selected paths
  (M24 §8.4 rows), invoked from the same Sico source — the UI authority
  and the file authority stay separate grants.
- **Data flow**: `Bytes`/`Text`/records cross the existing user-WIT
  boundary (RFC-0039); no hostcall outside the declared `sico:user/ui@1`
  surface.

## 3. Typed refusals (v0 closed set, append-only codes)

`UiError`: `unknown-handle`, `wrong-parent-kind`, `limit` (node-count and
tree-depth budgets declared, +1 refused before allocation), `no-window`,
`event-overflow` (queue cap declared; events coalesce by kind, never drop
silently into wrong delivery), `authority-denied` (UI capability not
granted — default-deny). Nesting-depth and node-count limit+1 cases are
typed refusals, not truncation.

## 4. Determinism and evidence bar (implementation STEP, gated separately)

- Every frame is a pure function of UI state (ADR-0018 §3): the frozen
  corpus renders fixed node trees at fixed client size and hashes frames
  byte-stably on the real Windows native renderer.
- Event evidence: scripted click/select sequences deliver exactly the
  recorded events in order; `closed` terminates the app deterministically.
- Authority evidence: without the UI grant every call returns
  `authority-denied`; the pilot consumes no capture/input grants.
- Full RFC-0033 four-part evidence through the real runner; the guest
  (selfhost) consumption is the M23 §9 re-baseline decision, not part of
  the first implementation STEP.

## 5. Honesty gates

- Nothing here claims any UI binding exists today (M24 §8.6.1 honest
  scoping); the M5 companion and M15 Web GO states are untouched.
- This RFC plus proposed ADR-0018 are both required before any UI
  implementation STEP; web/webview, M15 bindings, or a native Rust
  companion cannot substitute for the Sico-authored surface (card 24-C
  stop rule).
- Non-goals (v0): styling APIs (tokens are renderer-frozen), custom
  drawing surfaces, timers/animations, multi-window, DPI scaling, menus,
  keyboard navigation beyond default focus, drag-and-drop, clipboard.
