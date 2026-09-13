# ADR-0013: Native Automation Host platform adapter v0 (Windows-first)

> - status: accepted
> - accepted: 2026-09-10 (owner session directive "完成M15-17"), with amendment A1 below
> - date: 2026-09-08
> - depends: RFC-0040 (capability surface, companion draft), M16 threat model, M16 plan §5.3/§8

## Decision

The Native Automation Host's v0 platform adapter targets **Windows only**,
in-process, with these settled choices:

1. **Surface enumeration: Win32 API, not UIA.** Enumerate top-level and
   child windows via `EnumWindows`/`FindWindowEx` plus Win32 titles,
   classes and rectangles. Accessibility-tree reading via UIA is a
   **non-goal** and stays out (threat T9): the ADR draws the line
   explicitly — the Host reads window geometry and title only; pixels and
   programmatic input are the entire interaction vocabulary.
2. **Capture: Windows.Graphics.Capture (WGC) with GDI `PrintWindow`
   fallback.** WGC gives per-window, compositor-consistent frames without
   screen-wide capture; the frame is bounded to the granted surface
   rectangle, converted once to BGRA8 at a Host-chosen stride (the
   RFC-0040 `frame` record). Rate and size ceilings enforced at the
   adapter boundary, below the RFC's `limit` taxonomy.
3. **Input: `SendInput` with absolute coordinates derived from the bound
   surface's current rectangle at commit time.** The Host re-derives the
   rectangle, verifies the observation revision, and drops the action
   (typed `stale-revision`) when the surface moved between preview and
   commit. In-flight input calls cannot be cancelled mid-call (an OS
   constraint): M11 cancellation marks the task cancelled and the adapter
   completes or abandons the single bounded action at its boundary — a
   committed action is atomic and never split (threat T7).
4. **Dry-run enforcement: Host-side.** The adapter never receives a commit
   without a verified token; dry-run mode makes the Host withhold token
   minting entirely. Guests cannot distinguish or disable it.
5. **Adapter isolation: smallest isolated crate, in-process for v0.**
   `sico-automation-host` is a standalone workspace crate (like the HTTP
   provider) holding all Win32/WGC unsafe code; the compiler and shared
   security cores stay free of platform-specific unsafe. Process
   isolation for the adapter is deferred: the capability closure and
   Host-side enforcement already gate every call, and a crash-isolation
   boundary is re-evaluated with M17's acceleration-provider ADR where
   third-party code (OpenCV, runtimes) makes process isolation necessary.
6. **Fixture app first.** Windows evidence is only claimed on the
   controlled fixture app (M16 plan §5.4); synthetic-adapter results are
   `contract-verified` and never advertised as runtime support.

## Accepted amendments (at acceptance, 2026-09-10)

- **A1 — v0 ships GDI `PrintWindow` capture only.** WGC requires the
  Windows Runtime projection surface, which the locked offline toolchain
  does not carry; GDI `PrintWindow` (with `PW_RENDERFULLCONTENT`) is this
  ADR's own fallback path and becomes the v0 capture implementation.
  Recorded GDI limitations: occluded or hardware-accelerated-unfriendly
  windows may capture stale content; the controlled fixture app is the
  only runtime-evidence target, so the limitation is bounded. The WGC
  upgrade returns through an amendment when the projection crates enter
  the vendored dependency set. The capability surface (RFC-0040) is
  unaffected: `capture-error::io` covers capture-path failures.

## Alternatives considered and rejected

- **UIA-tree reading as the observation channel** — more robust targeting,
  but it is ambient accessibility-tree scraping (M16 §6 non-goal), drags
  in a COM apartment model, and converts the Host into an automation
  framework indistinguishable from the ones Sico must not become.
- **Raw screen capture (`BitBlt` on the desktop DC)** — unbounded scope;
  per-surface WGC capture keeps the grant boundary real.
- **Out-of-process broker for input (v0)** — stronger crash isolation but
  doubles the audit/token surface before the loop is proven on the
  fixture app; revisit with M17's provider isolation ADR.

## Consequences

- macOS/Linux/Android stay unsupported until licensed toolchains and
  native runners provide equivalent evidence (M16 plan §8).
- The adapter crate is the only place Windows FFI may appear (architecture
  boundary rule); module-boundary validator gains its package entry.
- Cancellation semantics for in-flight OS calls are "bounded action
  atomicity" — recorded as an invariant with its own corpus case, not
  hidden as a general cancellation claim.
