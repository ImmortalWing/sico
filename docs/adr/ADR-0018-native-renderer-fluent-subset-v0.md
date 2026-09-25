# ADR-0018: Native desktop renderer architecture — Fluent-subset Direct2D v0

> - status: **proposed — owner acceptance required before any implementation STEP**
> - date: 2026-09-25
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -
> - depends: AGENTS.md §4 (authority/FFI boundaries), M24 plan §8.6.1, ADR-0017 lineage unaffected, RFC-0039 (modules/user-WIT), [`sico-ui-v0.wit`](../../crates/sico-host-core/wit/sico-ui-v0.wit) (M5 companion, unchanged), STEP-0287 inventory

## Context

The owner directive (2026-09-22, STEP-0258) re-designated the M18/M24
pilot as a GUI format converter authored in Sico over a **Sico-language
native UI library styled like WinUI 3 (Fluent)**, and explicitly ruled
out web/webview substitution (「原生界面库是sico语言原生界面库」). M24 §8.6.1
requires the renderer architecture to be frozen by an ADR before any
implementation STEP. Today only the M5 strict-companion desktop host UI
(`sico-ui-v0.wit`, no compiler-facing binding) and the M15 Web-scoped
renderer exist; neither can host a Sico-authored native application.

## Decision

1. **Renderer home: smallest isolated adapter crate inside the desktop
   host.** A new adapter crate owns all platform-specific unsafe code
   (Win32 window shell, Direct2D/DirectWrite drawing, event pump) per
   the AGENTS.md §4 FFI rule; compiler crates, IR, and shared security
   cores stay free of platform code. The desktop host owns the
   capability grant; the runner owns Store lifetime and cancellation.
2. **Drawing stack: Direct2D + DirectWrite custom rendering — not the
   WinUI 3/XAML runtime.** The owner pinned "WinUI 3-like Fluent style";
   the pinned *tokens* are what freeze: 4px corner radius geometry, the
   Fluent accent + light-theme color ramp (hex values frozen in the
   corpus), Segoe UI Variable typography with frozen size ramp, and
   Fluent-standard control geometries for the RFC-0052 control set.
   Taking the tokens and drawing them ourselves keeps the M7 installer
   self-contained (no Windows App SDK redistribution), makes every frame
   a pure function of UI state (no composition animations underneath),
   and keeps versioning in-repo. Pixel-perfect WinUI 3 parity is
   explicitly not claimed — "Fluent-like subset" is the contract.
3. **Determinism contract (corpus-facing):** fixed client size (96-DPI
   logical pixels in v0), pinned color scheme, no timers, animations,
   or environment-dependent drawing in conversion-relevant regions —
   every frame is a pure function of the UI state; the frame corpus
   hashes rendered output byte-stably on the evidence host.
4. **Authority:** the UI capability is versioned, declared, and
   default-deny, composed from existing capability primitives — no new
   trust path. It grants **no** capture, input injection, clipboard,
   credentials, or process control (M16 boundaries untouched); file
   open/save ride the existing least-privilege file capabilities with
   user-selected paths.
5. **Alternatives rejected:** WinUI 3 / Windows App SDK runtime
   (external redistribution, nondeterministic composition, versioning
   outside the repo); web/webview rendering (owner-ruled out; also
   breaks the determinism contract); extending the M5 companion model
   (no compiler-facing binding; stays unchanged); M15 Web binding (out
   of scope by owner directive; M15 GO unchanged).

## Consequences

- The frame-determinism corpus and the Fluent token table become frozen
  evidence with the first implementation STEP; any token change is a new
  package/renderer version.
- Windows-only in v0 (evidence host is Windows); cross-platform parity
  stays owner-gated as everywhere else.
- The M26 GUI PDF tool consumes this exact surface (owner directive
  2026-09-24); its control needs (page list, metadata view, rotate/merge
  actions, progress) are measured against RFC-0052 in the M26 kickoff
  inventory, and gaps close here before the M26 GUI gate starts.
- No implementation claim arises from this ADR until its own STEP lands
  with real-renderer evidence on Windows.
