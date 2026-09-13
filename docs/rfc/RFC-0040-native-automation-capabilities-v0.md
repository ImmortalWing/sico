# RFC-0040: Native Automation Host capability surface v0

> - status: accepted
> - accepted: 2026-09-10 (owner session directive "完成M15-17"; the directive accepts the drafted contract set as the basis for completing M15–M17, with the findings recorded below)
> - date: 2026-09-08
> - phase: M16 contract track
> - depends: RFC-0017 (capability closure), RFC-0037 (grant shapes), M16 threat model `docs/reports/m16-threat-model.md`, ADR-0013 (platform adapter, companion draft)
> - inputs: M16 plan §3/§4; STEP-0142 inventory; M14/M10/M11 identity and cancellation machinery

## Summary

Freeze the guest-visible capability surface of the Native Automation Host:
one WIT interface per grantable capability, single-use preview/commit
tokens bound to surface identity and observation revision, a Host-owned
audit-event stream, and an emergency stop the guest cannot suppress.
Everything not frozen here is refused with a typed identity. Names are
provisional until acceptance.

## 1. Capability interfaces (one grant each; names provisional)

```wit
package sico:automation@0.1.0;

interface observe {
    /// Stable, revocable identity of one Host-authorized surface.
    /// Survives repaint/focus change; invalidates on close/minimize/
    /// display change (a `surface-invalid` outcome on next use).
    record surface-id { token: string }
    /// Discover only surfaces Host policy exposes for selection.
    list<surface-id> surfaces: func();
    /// Current monotonic revision of a surface's observable state.
    revision: func(surface: borrow<surface-id>) -> result<u64, observe-error>;
    enum observe-error { surface-invalid, permission, limit }
}

interface capture {
    /// One owned frame of the bounded authorized surface/region.
    /// Explicit stride/format so the M17 image contract consumes it
    /// without re-definition. Size/rate/total-frame ceilings are
    /// Host-enforced; exhaustion is a typed `limit` error.
    record frame {
        width: u32, height: u32, stride: u32,
        format: pixel-format, pixels: list<u8>,
    }
    enum pixel-format { bgra8 }
    capture: func(surface: borrow<observe.surface-id>) -> result<frame, capture-error>;
    enum capture-error { surface-invalid, permission, limit, io }
}

interface pointer {
    /// One previewed, committed action. Coordinates resolve against the
    /// bound surface's CURRENT geometry, never raw screen space.
    type action-variant = variant { move, press, release, drag };
    record action {
        variant: action-variant, x: u32, y: u32, duration-ms: u32,
    }
    /// Minted by the Host preview path; binds surface identity, observation
    /// revision, action digest and expiry; single-use; Host-verified.
    type commit-token = string;
    commit: func(surface: borrow<observe.surface-id>, token: commit-token,
                 action: action) -> result<_, pointer-error>;
    enum pointer-error { surface-invalid, stale-revision, token-invalid,
                         expired, budget, permission, io }
}

interface keyboard {
    /// Separate grant. Printable/control distinction; per-window
    /// keystroke budget; sensitive-field refusal fails closed.
    type: func(surface: borrow<observe.surface-id>, token: pointer.commit-token,
               text: string) -> result<_, keyboard-error>;
    enum keyboard-error { surface-invalid, stale-revision, token-invalid,
                          expired, budget, sensitive-field, permission, io }
}

interface audit {
    /// Host-owned, append-only from the Host side; guests can neither
    /// append nor read it. Exists in the world so tools can enumerate
    /// that the stream is part of the granted closure (threat T6).
    // No guest-visible functions by design.
}

interface stop {
    /// Emergency stop is Host-owned. The guest surface deliberately has
    /// no stop functions (threat T7): the interface exists so the
    /// capability closure names the stop resource class for tooling.
    // No guest-visible functions by design.
}

world automation-guest {
    import observe;
    import capture;
    import pointer;
    import keyboard;
    import audit;
    import stop;
    /// Composed with the script world's `run` export (RFC-0029 shapes);
    /// automation guests are Script-profile programs with these imports.
}
```

## 2. Token and revision semantics

- A commit token binds: surface identity, observation revision at preview
  time, a digest of the exact action, a single-use marker, and an expiry.
  The Host verifies all five at `commit`; any mismatch is one of the typed
  errors above (threats T1, T8).
- Verification semantics of the loop (`observe → plan → preview → execute
  one → observe → verify`): the post-action observation must match the
  plan's expectation within a declared tolerance; mismatch is a typed
  outcome, never an automatic retry.
- Stop conditions (unchanged frames, unexpected dialogs, timeout,
  cancellation, focus drift) are typed outcomes; dry-run is Host-enforced
  and guest-invisible.

## 3. Authority composition

- Capture does not imply pointer; pointer does not imply keyboard;
  neither implies clipboard, filesystem, credentials, process control or
  full-desktop access. Grants compose only by explicit Host policy
  (RFC-0017 closure; RFC-0037 grant shapes).
- An automation guest receives NO ambient authority. Exfiltration of
  captured frames additionally requires a separately granted M12 endpoint
  (threat T5).

## 4. Refusal corpus (each is a committed fixture at implementation time)

Commit without token / expired token / reused token; stale observation
revision; surface drift (identity invalidation); duplicate commit;
authority-widening mutations (capture→pointer, pointer→keyboard,
keyboard→clipboard absence); limit+1 for rate, size, budget and depth on
every capability; audit-stream write/read attempts from the guest; stop
suppression attempt (in-flight action must terminate with no further
input). E-code family: `E9xxx` (allocated at implementation kickoff).

## 5. Non-goals (v0)

- Accessibility-tree reading (ADR-0013 draws the Win32-vs-UIA line).
- Clipboard, credentials, process control, full-desktop capture.
- Automation of third-party apps hardened against automation.
- General CV/ML recognition (M17); web automation (M15).

## 6. Accepted amendments

- A1 (2026-09-11, at implementation): `keyboard-error` gains
  `invalid-text` — non-printable input outside the v0 alphabet
  (printable Unicode scalars plus newline/tab) refuses closed (threat
  T3's printable/control distinction needs a typed case; reusing
  `sensitive-field` would misreport policy state).

## 7. Reconsideration gate

Any new capability, token field, or error case amends this RFC before
implementation. The synthetic adapter implements exactly this frozen
surface (`contract-verified` evidence class); Windows-native evidence is
a separate, later gate (M16 plan §8 platform sequence).
