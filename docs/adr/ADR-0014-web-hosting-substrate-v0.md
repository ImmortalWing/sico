# ADR-0014: Web hosting substrate v0 (browser shim first)

> - status: accepted (owner session directive "完成M15-17", 2026-09-10; M15 plan §3.1)
> - date: 2026-09-10
> - depends: M15 plan §3.1–§3.4, RFC-0012 (Component/WIT boundary), RFC-0039 (modules/packages/user WIT, accepted), ADR-0013 (companion Host ADR for M16)

## Decision

The Sico Web Host v0 targets **staged coexistence**, beginning with a
**JavaScript canonical-ABI shim** that executes the compiled Component's
embedded core module inside a real browser engine:

1. **Substrate — JS canonical-ABI shim over the embedded core module.**
   Stable browsers (Chromium, Firefox, WebKit) do not ship the Component
   Model to web content (Chromium gates it behind an origin-trial-style
   flag; it is not a default capability), so a native-Component path is
   not a claimable substrate today. The v0 shim implements exactly the
   frozen v0 Script world surface in JS: the `run(ScriptInput) ->
   Result[ScriptOutput, ScriptError]` export plus the imported interfaces
   the guest actually uses (`sico:script/*` Host channels and, after
   RFC-0039, `sico:user/<interface>@<version>` package imports bound by
   the same lock). The core module is extracted from the compiled
   Component artifact by the build tooling; the shim never re-compiles or
   re-interprets Sico semantics — it is pure canonical-ABI transport.
   The controlled-webview alternative stays a documented later stage of
   the coexistence ladder (its isolation story differs; nothing in v0
   depends on it).
2. **HTTP authority.** The browser path grants no M12 HTTP authority in
   v0: `sico:script/http@0.2.0` imports are refused by the shim unless a
   separately-granted endpoint policy is composed into the run manifest,
   and fetch-based transport then enforces the RFC-0037 endpoint/DNS
   policy shim-side with browser-managed TLS declared as such (per-Store
   connection pooling is **declared unrepresentable** on this substrate —
   not approximated). Secrets stay Host-side (shim-side) and opaque.
3. **Identity.** The M10 debug triplet degrades declared, not hidden: the
   browser executes the same core module bytes (module identity survives),
   but source-frame mapping in-browser is v0-refused and stays a
   native-runner capability; the shim surfaces the component digest as the
   run identity alongside the page origin.
4. **Store lifetime.** One load = one Store equivalent: the shim owns one
   core-module instance per page execution; cancellation and the M11
   scheduler semantics stay native-runner capabilities (a page reload is a
   new run, never a resumed Store).
5. **Limits.** Fuel and memory limits are enforced in v0 as shim-side
   logical bounds (byte budgets, step ceilings on host-call loops) with
   the degradation declared: the precise wasmtime fuel/epoch machinery is
   native-only, and any future engine integration must re-earn the typed
   `RunOutcome` taxonomy (no silent engine-trap degradation).
6. **Rendering.** v0 has no canvas rendering; UI controls follow
   RFC-0042's DOM-backed deterministic control tree (accessibility via
   ARIA roles/name/focus as contract fields). v0 cross-host evidence runs
   headless-equivalent: byte-identical `run` outputs, DOM state asserted
   by the harness.

## Alternatives considered and rejected

- **Browser-native Component Model** — not a default capability in any
  stable engine; claiming it would fabricate platform evidence.
- **Controlled webview (WebView2) first** — real, but it re-states the
  native isolation story inside a Chromium wrapper without adding a
  browser-evidence class; the shim path produces the honest
  browser-engine evidence first, and the webview stage can reuse the same
  shim assets.
- **Transpiling Sico to JavaScript** — violates the no-JS-semantics rule
  (M15 §1) and abandons the Component artifact as the unit of behavior.

## Consequences

- Platform claims name the shim surface and exact engine versions from
  the real-browser harness (M15 §3.4); no engine is claimed untested.
- The shim is the only JS in the system's trust perimeter that may touch
  guest memory; its contract surface is the same frozen v0 world the
  native runner links, kept in sync by the cross-host matrix.
- Any future native-Component browser substrate amends this ADR.
