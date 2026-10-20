# RFC-0052 Amendment A1: re-home `ui@1` from a pure user-WIT package to a host-provider surface

> - status: **accepted by owner re-adjudication 2026-09-30 (STEP-0330)** — registered as proposed by STEP-0330 (surfaced by measuring RFC-0052 §2 against `sico-semantics::interface_value_offender` and the runner package/http2 link paths), then accepted by owner directive; this unblocks the #5 UI-binding slice down to the binding/authority-refusal layer (real render-frame evidence stays gated on #4's Direct2D external door)
> - date: 2026-09-30
> - amends: [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) §1 (home/namespace), §2 (surface form)
> - pairs: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md) (accepted — renderer home is a host-side isolated adapter crate)
> - precedent: `wit/script-http2-v0/http2.wit` (`sico:script/http@0.2.0`) + `runner/sico-runner/src/http2.rs::link_http2`

## 1. Two measured defects in RFC-0052 as accepted

RFC-0052 §1 freezes the UI vocabulary as a **"versioned package,
`sico:user/ui@1`"** authored "over the versioned package/user-WIT surface of
the RFC-0039 modules/imports line". Measurement against the executable gates
shows that home is wrong on two independent counts:

1. **Value-set defect (E8017).** §2 declares `record UiHandle { id: U64 }`,
   `record UiPoint { x: I64, y: I64 }`, and `poll_event() -> Option[UiEvent]`
   where `UiEvent` is a variant (`click(handle) | select(handle, index) |
   closed`). `interface_value_offender` (RFC-0039 §2.4 A7) fires on **every
   versioned `interface` declaration in Sico source** (`sico-semantics/src/lib.rs:497-518`)
   and accepts only scalars, one level of `List/Option` over a scalar, and
   `Result[ok, scalar-error]`. Records and variants are refused with **E8017**.
   So a Sico-source-authored `sico:user/ui@1` interface cannot carry §2's
   surface — the same class of defect RFC-0050 A1 fixed for the codec.

2. **Purity/host-effect defect.** `window()`, `run()`, `poll_event()`,
   `draw_image()` are **host effects** (open a native window, present a frame,
   pump the OS event queue). The runner's package path requires packages to be
   **pure** — `prepare_package` refuses any import, and `link_packages`
   bridges `sico:user/` guest imports only to *prepared pure package
   instances*. A surface that drives the desktop window system cannot be a
   pure `sico:user/` package; it is a guest **import** satisfied by a host
   provider, exactly like `sico:script/http@0.2.0`.

The http2 precedent resolves both: host-effect surfaces live in a **host
namespace** (`sico:script/...`) as a runner-linked provider WIT, where
records/enums/resources are allowed and `interface_value_offender` does not
apply (it only fires on Sico-source `interface` declarations, not on
runner-encoded host WIT).

## 2. A1 — corrected home and surface form

- **Namespace/home:** `ui@1` is a **host-provider import surface**, not a pure
  package. It is declared in a versioned host WIT under the `sico:script`
  (or an equivalent host) namespace — working name `sico:script/ui@1.0.0` —
  and linked by a new `runner/sico-runner/src/ui.rs::link_ui` to the
  ADR-0018 desktop-host renderer adapter, mirroring `link_http2`. The
  `sico:user/ui@1` package framing in RFC-0052 §1 is withdrawn.
- **Surface form:** because the host-provider path permits aggregates, §2's
  `record UiHandle`/`UiPoint` and the `UiEvent` variant **may stand as WIT
  records/variants** (http2 uses `record header`, `enum http-error`,
  `resource response-body`). No flattening is forced. The control set,
  signatures, and typed `UiError` refusal set (§2/§3) are otherwise unchanged.
- **Authority:** unchanged and reinforced — the UI capability is a versioned,
  declared, **default-deny** host grant composed from existing capability
  primitives (ADR-0018 §4); without it every call returns `authority-denied`.
  It grants no capture/input/clipboard/credentials/process control (M16
  untouched). File open/save stay separate least-privilege file capabilities.
- **Compiler impact:** the Sico-source binding is still **zero grammar
  growth** (RFC-0052 §1) — the source imports the host `ui@1` surface the
  same way it imports `http@0.2.0`; no new syntax, keywords, or lexer/parser
  changes. `interface_value_offender` is **not** relaxed: it continues to
  govern genuine `sico:user/` packages; host-provider WIT simply does not
  route through it.

## 3. What A1 does NOT change

- ADR-0018's renderer decision (Direct2D+DirectWrite Fluent-token subset,
  isolated adapter crate, frame determinism, default-deny authority).
- RFC-0052 §3 `UiError` closed set, §4 determinism/evidence bar, §5 honesty
  gates and non-goals.
- The M5 companion model and M15 Web scope (untouched).

## 4. Re-adjudication ask

Owner to accept A1 (re-home `ui@1` to a host-provider surface, http2
precedent) as the binding form of RFC-0052 §1/§2 before the UI binding
implementation STEP (#5). Until accepted, #5 is blocked; #4 (the host-side
renderer adapter crate that `link_ui` would bind to) proceeds under ADR-0018
independently, since it is not a Sico-source user-WIT interface and is not
subject to E8017.
