# RFC-0052 Amendment A2 (PROPOSED): `ui-error` render-failure discriminant + event-overflow observability + click/select delivery honesty

> - status: **PROPOSED (draft, not accepted)** — owner adjudication pending. **This amendment implements nothing, changes no behaviour, and touches no WIT, source, test or existing RFC text.** On owner acceptance a separate implementation STEP will be allocated; the STEP number is **not reserved here** (see §0).
> - date: 2026-11-07
> - phase: M24 §8.6.1 (Sico-language native UI library 前置工作流), contract-form review of the `ui@1` host-provider surface in use
> - amends: [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) §2 (surface form) and §3 (typed refusals)
> - pairs: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md) (accepted), [`RFC-0052 Amendment A1`](./RFC-0052-amendment-A1-host-provider-ui-surface.md) (accepted)
> - scope: the three contract blanks **exposed in use** of the `sico:script/ui@1.0.0` host-provider surface (C-A / C-B / C-C), plus a §6 acceptance clause defining "guest-visible UI capability"
> - evidence basis: executable evidence only; where a document and the implementation disagree the **lower** support claim is taken (AGENTS §1). Every assertion below cites `file:line`.
> - **citation caveat:** the `runner/sico-runner` and `crates/sico-ui-renderer-d2d` trees are **under concurrent agent edit** (per the parent brief, three agents are working the runner/renderer). All `file:line` references below are the exact anchors **verified in the worktree on 2026-11-07**; they will drift. Any implementer MUST re-anchor against the committed snapshot before relying on a line number. Symbol names (e.g. `fold_error`, `classify_window_event`, `MAX_QUEUE_EVENTS`, `ShellState::push`) are the stable identity.

## 0. Discipline of this slice

- **Contract surface is `wit/script-ui-v1/ui.wit`.** The runner hand-registers the surface via `func_wrap` and the compiler hand-encodes the instance type; neither parses this file (`wit/script-ui-v1/ui.wit:9-11`). Any WIT change is a **contract change** and therefore requires owner acceptance plus a versioning note — none of the options below is decided.
- **No option is pre-adjudicated.** A1/A2, B1/B2, C1/C2/C3 are all presented as candidates; the recommendations in §2.4 / §3.3 / §4.4 are advisory and non-binding.
- **Lower bound where docs conflict with code:** the only ground truth used is the executable code (`runner/sico-runner/src/ui.rs`, `crates/sico-ui-renderer-d2d/src/*`) and the frozen WIT. Where a STEP's prose makes a stronger claim than the code can support, the code wins and the discrepancy is recorded.

## 1. Verified facts (executable evidence)

### 1.1 C-A — `ui-error` has no render-failure discriminant (closed set)

- The guest-visible `ui-error` set is **closed** at six cases, in wire-discriminant (append-only) order: `unknown-handle`, `wrong-parent-kind`, `limit`, `no-window`, `event-overflow`, `authority-denied` (`wit/script-ui-v1/ui.wit:29-36`; declaration order is the wire discriminant order, append-only — `ui.wit:25-26`).
- The runner folds the renderer's typed failures into this closed set (`runner/sico-runner/src/ui.rs:249-264` `fold_error`):
  - `UnknownHandle`→`unknown-handle`, `WrongParentKind`→`wrong-parent-kind`, `Limit`→`limit` (direct);
  - `InvalidArgument`→`limit` (`ui.rs:256`) — a range violation (e.g. out-of-range window size `crates/sico-ui-renderer-d2d/src/windowed.rs:306-315`, frame size `crates/sico-ui-renderer-d2d/src/shell/win.rs:561-564,605-608`) is reported as `limit`;
  - `NoWindow` | `UnsupportedPlatform` | `RenderFailed`→`no-window` (`ui.rs:260-262`).
- The renderer-side failure set that collapses onto `no-window` (`crates/sico-ui-renderer-d2d/src/error.rs:14-39`): a genuine absent window (`NoWindow`), a non-Windows host (`UnsupportedPlatform`), and the whole `RenderFailed` family — shell-thread loss (`windowed.rs:178-197`), shell protocol mismatch (`windowed.rs:262-268`), and the window-creation / class-registration / present / read-back failures (`shell/win.rs:314-323,427-460,529-534,541-573,598-624,643-646,653-661`).
- **Consequence:** a guest that calls `window()` / `run()` and receives `no-window` cannot distinguish (a) no window open, (b) unsupported platform, (c) window-creation / render / present failure. The runner itself documents the collapse as the only honest option in the closed set, and notes a dedicated `render-failed` discriminant would need a WIT append (`ui.rs:166-172`).
- **Guest-visible misreading:** `no-window` reads most naturally as "no window is open"; when the real cause is a render/present/create failure the guest is told the wrong thing and has no alternative typed channel to recover the distinction.
- **Other folded-away causes (enumerated):** `InvalidArgument`→`limit` (`ui.rs:256`) hides an invalid window/frame size behind the `limit` budget reading. `poll-event`'s `Err(_)`→`None` (`ui.rs:369`) hides a lost shell thread behind "no event" (see §1.2).

### 1.2 C-B — `event-overflow` is declared but unobservable to the guest

- `poll-event: func() -> option<ui-event>` carries **no error channel** (`wit/script-ui-v1/ui.wit:83`); `ui-event` is `click(ui-handle) | select(ui-selection) | closed` (`ui.wit:59-63`).
- The shell's event queue is capped at `MAX_QUEUE_EVENTS = 256` (`crates/sico-ui-renderer-d2d/src/events.rs:30`), pre-allocated, coalescing by kind (`Closed` idempotent-terminal, `Paint` coalesces, `Resized` latest-wins, `Clicked` point-event non-coalescing) and setting an `overflow` flag on a cap hit **instead of silently dropping** (`crates/sico-ui-renderer-d2d/src/shell/win.rs:86-131`; unit tests `shell/win.rs:747-860`).
- **The overflow flag is never surfaced on the guest path.** It is set in `push()` (`shell/win.rs:93,110,122,127`) and read only by `take_overflow()` (`shell/win.rs:132-134`), which is consumed **only** by `drain()` (`shell/win.rs:514-524`) — a host-side audit affordance, not wired into `pump_once()`. The guest path is `WindowedUiBackend::poll_event`→`ShellClient::pump`→`shell.pump_once()` (`windowed.rs:453-455`, `windowed.rs:218-223`, `shell/win.rs:465-512`), and `pump_once()` never reads the flag. `drain()` is referenced only in the shell wrapper (`shell/mod.rs:134-136`) — no guest-facing call site.
- **Consequence:** a guest polling `poll-event` cannot distinguish "no event pending" from "events were dropped at the cap". RFC-0052 §3 declares `event-overflow` (queue cap declared; events coalesce, never drop silently — `docs/rfc/RFC-0052-native-ui-source-binding-v0.md:53-58`) and the WIT carries the `event-overflow` case (`ui.wit:34`), but the discriminant currently has **no producer** reachable by a guest.
- **Related:** a lost shell thread is `RenderFailed` on the backend (`windowed.rs:178-197`), folded by `poll-event` to `None` (`ui.rs:369`) — the same "no event" signal, also unobservable.

### 1.3 C-C — `click(ui-handle)` / `select(ui-selection)` promised but never delivered (point→handle hit-testing missing)

- The contract **does** declare `click(ui-handle)` and `select(ui-selection)` variants (`wit/script-ui-v1/ui.wit:59-63`; RFC-0052 §2 `poll_event() -> Option[UiEvent]` with `click(handle) | select(handle, index) | closed` — `docs/rfc/RFC-0052-native-ui-source-binding-v0.md:39-40`).
- The renderer has **no layout**: a pointer press is captured only as **client coordinates** (`WindowEvent::Clicked { x, y }` — `crates/sico-ui-renderer-d2d/src/events.rs:65-82`, `events.rs:79-81`; shell mapping `shell/win.rs:280-291`; coordinate filter `events.rs:109-113`). Mapping a point to a node handle needs the renderer layout, which is the next slice — the backend honestly delivers no node handle for a click (`windowed.rs:24-26`, `shell/mod.rs:27-30`; STEP-0577 `docs/steps/STEP-0577-m24-window-shell-event-pump.md:85-86`).
- **The runner skips the coordinate click rather than fabricating a handle** (`runner/sico-runner/src/ui.rs:346-372` `poll_event`): `classify_window_event` (`ui.rs:277-285`) maps `Closed`→`closed`, `Clicked`→`(None, skipped=true)`, `Resized`/`Paint`→`(None, false)`; a skipped click increments the host-side `skipped_click_count` (`ui.rs:180,362`), exposed by `skipped_click_count()` (`ui.rs:221-224`). STEP-0585 has therefore made the skip **observable at the host** ("a real click that was ignored" is distinguishable from "no event was pending") — but the counter is **not a guest contract member** (`ui.rs:218-219`), so it remains **unobservable to the guest**.
- **Consequence:** the contract promises a `click(handle)` delivery, the implementation never delivers one. A guest that polls `poll-event` in a loop sees no `click` ever, and cannot tell "no click" from "a click happened but was skipped" — a **guest-unobservable skip of contract-visible events** (host-side counted since STEP-0585, still not guest-discernible). STEP-0581 registers the capability gap as a residual (`STEP-0581:118-119`); the STEP README records it as a gap to be made explicit (`docs/steps/README.md:519`); the host-side observability is verified in `ui.rs:175-180,212-224,277-285`.

## 2. Decision point A — `ui-error` render-failure discriminant

### 2.1 A1 (option) — append `render-failed` (+ optional `invalid-argument`) to `ui-error`

- **Change:** append `render-failed` at the **end** of the `ui-error` enum (`ui.wit:29-36`), preserving the existing 0–5 wire discriminants (append-only contract, `ui.wit:25-26`). Optionally also append `invalid-argument` so the `InvalidArgument`→`limit` fold (`ui.rs:256`) gets a true home — one WIT append can carry both.
- **Wire compatibility:** appending is additive — existing guests that construct/match discriminants 0–5 are unaffected; only a guest that match-exhaustively on `ui-error` must add the new case. The runner's `#[repr(u8)]` discriminant test asserts 0–5 (`ui.rs:688-696`) and stays green.
- **Change set (implementation, not this slice):** the runner's `UiError` enum + `fold_error` (`ui.rs:40-59,249-264`); the compiler's hand-encoded instance type for the host `ui@1` import (neither parses the WIT — `ui.wit:9-11`); any guest fixture that exhaustively matches `ui-error`. A versioning note (e.g. a `@1.1.0` wire note or an append record) is required because the interface is versioned (`ui.wit:22` `package sico:script@1.0.0`).
- **Effect:** the guest can distinguish "window absent" from "render failed" from "unsupported platform", and "invalid argument" from "limit". Removes the misleading `no-window` reading in §1.1.
- **Cost:** a contract change (WIT) requiring owner acceptance, plus lockstep runner + compiler hand-encoded updates; no renderer change needed.

### 2.2 A2 (option) — keep the fold; declare it in RFC/ADR and user docs

- **Change:** no WIT change. Document the collapse explicitly: "`no-window` covers window-creation / present / unsupported-platform / render failures" in RFC-0052 §3 and the user documentation, and state there is **no** guest-discernible alternative.
- **Guest-discernible alternative:** none exists. Every guest-visible result channel that can fail (`window`/`run`/`set-text`/… all `result<_, ui-error>`) folds the same way, and `poll-event` has no error channel at all (§1.2). The only non-typed signal is the host-side `window_title()` audit affordance (`ui.rs:208-210`), which is **not** a guest contract member.
- **Effect:** removes no ambiguity, but the collapse is a *documented* lossiness rather than an implicit one; the guest still cannot distinguish the causes.
- **Cost:** documentation only; no code/contract change.

### 2.3 Note on "其它被折叠掉的失败原因"

Beyond `render-failed`, the current closed set **also** folds `InvalidArgument`→`limit` (`ui.rs:256`). If the owner elects A1, appending `invalid-argument` in the **same** append (a) fixes the second lossy fold and (b) avoids a second WIT bump. A2 covers neither.

### 2.4 Recommendation (advisory)

**A1 (append `render-failed`, and in the same append `invalid-argument`).** Rationale: the collapse to `no-window` hides a real failure class and actively misleads a guest ("no window" when the renderer failed). A2 is an honest interim posture but leaves the ambiguity permanent. Because the interface is append-only (`ui.wit:25-26`), A1 is wire-compatible for existing discriminants; it is a contract change and is therefore **owner-decided**. Until accepted, the code stays as-is (documented collapse — `ui.rs:166-172`).

## 3. Decision point B — `event-overflow` observability

### 3.1 B1 (option) — extend the contract so overflow is observable

- **B1a — change `poll-event`'s return type** to `result<option<ui-event>, ui-error>` so a cap hit yields `err(event-overflow)` and a lost shell thread yields `err(render-failed)` instead of `None` (`ui.rs:369`). **Breaking:** this changes the wire signature — a guest compiled against `option<ui-event>` no longer matches; requires an interface version bump (or a new interface) and lockstep runner + compiler hand-encoded updates.
- **B1b — additive companion query** (e.g. `poll-event-overflow: func() -> result<bool, ui-error>`, mirroring the shell's one-shot `overflow` flag `shell/win.rs:132-134`). **Non-breaking:** additive; existing `poll-event` stays; `event-overflow` becomes producible without changing any existing signature.
- Both make `event-overflow` reachable and make a pump failure typed rather than `None`.

### 3.2 B2 (option) — status quo; register as a declared limitation

- **Change:** no WIT change. Declare in RFC-0052 §3 and the user docs that "a full event queue may drop events; the cap is 256 and overflow is not observable through `poll-event`", and require UI library users to accept the drop.
- **Effect:** removes no ambiguity; the guest still cannot distinguish "no event" from "dropped events". Note the queue coalesces by kind and only a burst of non-coalescible point events (clicks) can actually overflow (`shell/win.rs:106-127`), so the practical exposure is narrow but non-zero.

### 3.3 Recommendation (advisory)

**B1b (additive overflow query), consolidated into the same WIT append as A1.** Rationale: `event-overflow` is already a declared discriminant (`ui.wit:34`) with **no producer**; B1b gives it one without a breaking signature change and without touching the existing `poll-event` wire form. B1a is simpler to read but breaks the ABI; B2 is the honest status quo but leaves the ambiguity. Both B1 forms are contract changes → **owner-decided**; until accepted, overflow stays unobservable (documented — `STEP-0581:120-122`).

## 4. Decision point C — click/select point→handle hit-testing

### 4.1 C1 (option) — add layout + hit-testing to the renderer

- **Change:** implement a layout geometry (node bounds) and point→handle hit-testing so `click(handle)` / `select(ui-selection)` can be delivered for real coordinates. **New capability**, not a bugfix; needs its own ADR/contract decision for the layout model (the renderer today has no layout — `windowed.rs:24-26`, `shell/mod.rs:27-30`).
- **Effect:** closes the C-C gap; the guest receives a real `click(handle)` with the correct node.
- **Cost:** the largest of the three; a layout/hit-testing capability slice, likely a new ADR + implementation STEP + frame-corpus/event-corpus evidence.

### 4.2 C2 (option) — narrow delivery to the window root

- **Change:** before layout exists, deliver every coordinate click as `click(root-handle)` (the window root), and declare the narrowing.
- **Effect:** **worse than skipping.** Every click — including one on a child button — is attributed to the root, which is a wrong handle and mis-attributes a child-element click to the root. This violates the "never fabricate a wrong handle" posture the codebase holds (`ui.rs:347-356`, `STEP-0581:58-60`).

### 4.3 C3 (option) — do not deliver click/select until layout lands; write the narrowing into the contract + user docs

- **Change:** no code change beyond what exists (the skip is already the behaviour — `ui.rs:346-372`; it is now host-side counted since STEP-0585, `ui.rs:221-224`). State in RFC-0052 §2 and the user docs that, until point→handle hit-testing exists, `click`/`select` are **not** delivered (only `closed` is), and that a coordinate click is not observable **by the guest** (it is host-side countable). Default-deny-style **honest narrowing**: the surface promises the variants, the implementation narrows delivery to `closed` only, and the narrowing is declared rather than silent.
- **Effect:** removes the guest-unobservable-skip honesty gap; a guest now knows (from the contract) that `click`/`select` are not delivered in the current form. It does not add the capability — that stays C1.
- **Cost:** documentation + the in-flight STEP-0585 explicit registration; no renderer change.

### 4.4 Recommendation (advisory)

**C3 now, C1 later.** Rationale: C3 is the only option consistent with the codebase's existing honesty posture ("skipped rather than fabricated" — `ui.rs:347-356`, `STEP-0581:58-60`) and requires no new capability; it converts a skip that is guest-unobservable into a declared narrowing. C2 is rejected because it fabricates a wrong handle for every child click. C1 is the eventual correct fix but is a new capability that needs its own ADR/contract decision and must not be assumed into this slice. **Owner-decided**; until accepted the skip remains (documented as a residual — `STEP-0581:118-119`).

## 5. Compatibility impact (WIT versioning)

- The interface is versioned `sico:script/ui@1.0.0` (`ui.wit:22`); the runner registers the instance by that name (`runner/sico-runner/src/ui.rs:34` `UI_INTERFACE`). Neither the runner nor the compiler parses the WIT (`ui.wit:9-11`).
- **A1 (append) and B1b (additive query)** are both **wire-compatible appends** under the append-only rule (`ui.wit:25-26`): existing discriminants 0–5 keep their values, `poll-event`'s signature is unchanged. Existing guests need no change unless they exhaustively match `ui-error` or want the new query.
- **B1a (signature change)** is **breaking**: `poll-event`'s ABI changes, so any existing guest binary must be recompiled against the new instance type; a version bump (or a new `ui@1.1` interface) is required.
- Any of A1/B1/B2/C3 that touches the contract text requires the RFC-0052 §2/§3 text and the versioning note to be updated together; no field name changes (identifiers stay English, AGENTS §7).

## 6. "Guest-visible UI capability" — acceptance clause (definable, reviewable)

**Current state (verified, lower-bound):** a granted guest's `window()` call creates a real OS window and the guest observes handle 1 — runtime-verified on Windows x64 with an internal-fixture guest (STEP-0581:114-117; e2e `runner/sico-runner/tests/ui_authority.rs`). `run()` (present) and the `closed` event run through the production backend **host-side handle** in e2e, but the **guest-level fixture covers only `window` and `poll-event`'s `none` branch**; guest-level `run`/`closed` fixtures are deferred (STEP-0581:115-117). Hit-testing (`click(handle)`/`select`), file dialogs, and the pilot application are **not done** (STEP-0581:118-119,137-138). The M24 exit audit v2 records the §8.6.1 independent item as "point→handle hit-testing, file dialogs, pilot app all not done ⇒ **no guest-visible UI capability**" (`docs/steps/README.md:521`); STEP-0581 itself says "guest 可见 UI = 部分达成（诚实）" (`README.md:519`). The two framings differ on what "capability" means — §6 resolves it as a checklist, not a sentence.

**Proposed minimal evidence set for "guest-visible UI capability" (each item executable and reviewable, G1–G5 the capability core):**

| # | Evidence | Executable check | Current |
|---|---|---|---|
| G1 | guest `window()` real-window evidence | guest fixture calls `window()`; `result` is `Ok(handle)`; host proves a real OS window exists (`window_title()` is `Some` — `ui.rs:208-210`) | **done** (STEP-0581:114-117) |
| G2 | guest `run()` presented-frame evidence | guest fixture calls `run()`; host `read_presented_frame()` returns the expected client size and byte-identical frame (`windowed.rs:483-485`) | **not done at guest level** (host-side only — STEP-0581:115-117) |
| G3 | guest `poll-event` delivers `closed` | guest fixture pumps `poll-event` after a close and observes the `closed` variant | **not done at guest level** (host-side only) |
| G4 | guest `poll-event` delivers `click(handle)`/`select` with correct node routing | guest fixture delivers a click at a known child's bounds and receives `click(that-handle)`; needs hit-testing | **not done** (hit-testing missing — §1.3) |
| G5 | guest-observed frame determinism | guest renders the same tree N times → N byte-identical frames (ADR-0018 §3 `docs/adr/ADR-0018-native-renderer-fluent-subset-v0.md:40-44`) | **not done at guest level** (renderer-core corpus only, STEP-0338) |
| G6 | clean-room pilot application (gate 5) | a Sico consumer app over `ui@1` builds/run with a real window | **not done** (STEP-0581:138) |

**Definable verdict:** "guest-visible UI capability" is claimed iff **G1 ∧ G2 ∧ G3 ∧ G4** all hold (a guest can create a real window, present frames it observes, receive the `closed` event, and receive a click routed to a node handle), with G5 as the determinism companion and G6 as the separate pilot/gate-5 item. Under this definition the capability is **currently NOT achieved** (G1 only), which matches the exit-audit framing; STEP-0581's "部分达成" is the honest sub-statement that G1 is done. The gap between "部分达成" and "达成" is exactly G2∧G3∧G4∧G5.

## 7. Open questions (owner-decision items, not pre-adjudicated)

1. **A1 vs A2** — is the `no-window` lossiness acceptable at v0, or must the contract carry `render-failed`?
2. **B1b vs B1a vs B2** — additive query (non-breaking) vs `result` signature (breaking) vs declared limitation?
3. **C1 vs C3** — build layout + hit-testing as a new capability (C1), or declare the narrowing (C3) until C1 lands?
4. **One consolidated WIT append (A1+B1b) vs two separate appends** — the draft recommends consolidating to avoid a second bump.
5. **Versioning form** — `@1.0.0` append record vs a `@1.1.0` wire note, once the owner accepts any WIT change.
6. **"Guest-visible UI capability" verdict** — accept §6's checklist as the definition, or a narrower/looser one (the parent brief's framing allows G1 alone to be argued as partial capability).
7. **STEP-0585 in-flight scope** — whether its "explicit gap registration + observability" covers C-C's declared narrowing (C3) in the same slice, or a separate slice is needed. (The host-side `skipped_click_count` observable is verified landed in `ui.rs:175-180,212-224,277-285`; the guest-facing declared narrowing is still open.)

## 8. Honest narrowing / non-claims

- This amendment **does not implement** anything and **does not change** any runtime behaviour; it only records the three contract blanks and offers options. On owner acceptance, a separate implementation STEP is allocated and the RFC-0052 text + versioning note + user docs are updated together.
- No option above is decided; no code path is proposed to be flipped here.
- The `click`/`select` skip is **guest-unobservable but host-side counted since STEP-0585** (a real gap, but no longer fully silent at the host); the practical exposure is bounded — the queue coalesces by kind and only non-coalescible point events overflow (`shell/win.rs:106-127`); `closed` is terminal (`shell/win.rs:89-98`).
- The `no-window` fold is **not** a panic or a silent swallow — it is a typed, documented collapse (`ui.rs:166-172,260-262`); the gap is lossiness/misreading, not a crash.
- Non-claims: this amendment claims no new UI capability, no layout, no hit-testing, no file dialogs, no pilot — those remain the registered residual slices (STEP-0581:133-138).
- **Citation caveat repeated:** all `file:line` values above were verified in the worktree on 2026-11-07 and the runner/renderer are under concurrent agent edit; re-anchor before use.
