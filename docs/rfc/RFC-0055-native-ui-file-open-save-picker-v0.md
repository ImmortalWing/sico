# RFC-0055: Sico-language native UI library — file open/save path-selection v0

> - status: **accepted as the frozen picker contract** (owner directive 2026-11-04) — surfaced by
>   STEP-0555 while closing [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) §2's
>   file-dialog item: measuring the M24 §8.4 source-selection/persistence rows against the
>   existing `fs-read`/`fs-write` capability surface shows there is **no user-selected-path
>   primitive** in the host-provider import surface. This RFC freezes that missing vocabulary;
>   implementation follows in a later STEP. The gate 5 (GUI format-converter pilot) verdict stays
>   owner-gated / M24 exit audit.
> - port: **ported to `dev` from `codex/m24-vision-ui-v2` `6274ae6` under owner porting
>   authorization (2026-11-18)**. Base RFC-0055 accepted as the **frozen picker contract**;
>   [`Amendment A1`](./RFC-0055-amendment-A1-root-relative-picker-path.md) is recorded as
>   accepted-contract but **explicitly NOT yet implemented** (contract-only on codex; separate
>   follow-up STEP).
> - date: 2026-11-04
> - phase: M24 §8.6.1 prerequisite workstream (the last file-dialog residual)
> - depends: [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) + [`Amendment A1`](./RFC-0052-amendment-A1-host-provider-ui-surface.md)
>   (accepted — `ui@1` is a host-provider import surface), [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)
>   (accepted — renderer home), the script-profile `fs-read`/`fs-write` capability
>   (`wit/script-profile-v0/world.wit`), M24 plan §8.4/§8.5
> - authority posture: default-deny, least-privilege, **composes no new trust**; the dialog is
>   pure path selection and grants no read/write authority by itself

## 1. Summary

Freeze the **file open/save path-selection** vocabulary for the Sico-language native UI
library. The M24 GUI format-converter pilot needs a user-selected path for its source
selection and persistence rows (M24 plan §8.4), but the existing file capability —
`fs-read.read(path)` / `fs-write.write(path, bytes)` — only accepts a **path string under a
Host-granted root**. Nothing lets the *user* choose that path at run time. This RFC adds a
host-provider **dialog** surface that returns a user-selected path, leaving all actual
read/write authority in the existing default-deny file capability.

## 2. Two measured gaps

1. **No user-selected-path primitive.** `wit/script-profile-v0/world.wit:30-41` defines
   `fs-read { read(path), exists(path) }` and `fs-write { write(path, bytes) }`. Every path is
   a caller-supplied string bounded by a Host-granted root; the guest cannot ask the user to
   pick one. The M24 pilot's "file open dialog over the existing file capability" (§8.4) is
   therefore unsatisfiable with the current vocabulary — the missing piece is exactly the
   dialog, not more file authority.

2. **Authority must stay decomposed.** A dialog that both chose a path *and* opened a
   read/write handle would widen authority past the existing file capability and blur the
   default-deny boundary. Per A1 §4 and the M24 plan §8.5 fail-closed corpora, the dialog must
   compose no read/write/capture/input/clipboard/credential/process authority.

## 3. Proposal — `path-picker` host-provider surface

A new host-provider surface, authored beside `ui` in the `sico:script` namespace and linked by
the runner like `link_ui` (A1 §2 home/namespace rule; records/enums permitted on the
host-provider path, `interface_value_offender`/E8017 does not apply). Zero grammar growth —
Sico source imports it exactly like `sico:script/ui@1.0.0`.

```wit
package sico:script@1.0.0;

interface path-picker {
  enum picker-error {
    cancelled,        // user dismissed the dialog (not an error to surface)
    no-window,        // no native dialog host on this platform
    limit,            // filter/option budget exceeded (limit+1 typed, before allocation)
    authority-denied, // default-deny: no picker grant
  }

  /// Ask the user to choose an existing file to open. `filters` entries are
  /// flattened `"name|pattern"` strings, e.g. `"PNG image|*.png"`; the count
  /// is bounded (see §5). Returns the user-selected path, or `cancelled`.
  open: func(title: string, filters: list<string>)
        -> result<option<string>, picker-error>;

  /// Ask the user to choose a save destination. Does NOT write; the caller
  /// persists through `fs-write` under the existing write grant.
  save: func(title: string, suggested-name: string, filters: list<string>)
        -> result<option<string>, picker-error>;
}
```

- **`filters` is the flattened `list<string>` form** (`"name|pattern"` per entry). The codex
  WIT draft declared `list<tuple<string, string>>`, but the runner binding, codegen and guest
  source all use the flattened `list<string>` (`"name|pattern"` per entry); the effective
  surface is `list<string>`. This port corrects the contract to `list<string>` so it matches
  what will actually be implemented (the WIT is corrected in lockstep).
- **Dialog returns a path only.** It performs no I/O. The caller then uses the existing
  `fs-read`/`fs-write` capability; if that grant is absent, the file capability refuses
  `denied` and nothing is read/written — the two capabilities stay independently default-deny.
- **No path in errors.** Mirroring `ui@1` and `fs-read`, the host never leaks window, path or
  diagnostic text across the boundary; `picker-error` is typed-only (`cancelled` /
  `no-window` / `limit` / `authority-denied`).

## 4. Authority

The picker is a versioned, declared, **default-deny** host grant composed from existing
capability primitives (ADR-0018 §4). Without it, `open`/`save` return `authority-denied`
before any native dialog is shown. It grants **no** capture, input, clipboard, credentials or
process control (M16 untouched), and **no** read/write authority by itself — file authority
stays in `fs-read`/`fs-write`.

## 5. Budgets

`filters` count is bounded (limit+1 refused before allocation, never truncation), mirroring the
`ui@1` node-count/tree-depth budgets; `title`/`suggested-name`/`patterns` are bounded strings.
No `limit` is reported after allocation begins.

## 6. Compiler impact

Zero grammar growth: the source imports the host `path-picker` surface the same way it imports
`sico:script/ui@1.0.0`. The compiler-side binding mirrors the `ui@1` path (IR
`intrinsic_signature` + semantics `infer_stdlib_call` registered in lockstep, codegen-wasm
`InstanceType`/emit/error-name-table mirroring http2+ui). The slice may land incrementally
(`open` first) as with `ui@1`.

## 7. Honest residuals / boundaries

1. **Contract accepted, not yet implemented** — this RFC freezes the vocabulary; the
   implementation, fixture and evidence land in a later STEP. No gate flips on acceptance alone.
2. Gate 5 (GUI format-converter pilot) and M24 verdicts remain **owner-gated / M24 exit
   audit**; acceptance of this RFC only unblocks the *contract* for the file-dialog slice —
   implementation evidence follows in that later STEP.
3. `poll_event`'s source-side representation (returns `option<ui-event>` variant) was the other
   §8.6.1 residual — **resolved 2026-11-04 as Option A** (flatten to `List[Text]`,
   zero-grammar-growth); see the
   [`poll-event decision memo`](./poll-event-source-representation-decision-memo.md).
4. Cross-platform dialog behaviour (native vs. headless) follows the ADR-0018 adapter model;
   headless/evidence-host behaviour is not specified by this RFC.
