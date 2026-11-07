# RFC-0052 Amendment A3 (PROPOSED): `select(ui-selection)` model gap — honest narrowing vs model extension vs host-side event

> - status: **PROPOSED (draft, not accepted)** — owner adjudication pending. **This amendment implements nothing, changes no behaviour, and touches no WIT, source, test or existing RFC text.** On owner acceptance a separate implementation STEP will be allocated; the STEP number is **not reserved here** (see §0).
> - date: 2026-11-07
> - phase: M24 §8.6.1 (Sico-language native UI library 前置工作流), contract-form review of the `ui@1` host-provider surface in use
> - amends: [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) §2 (surface form, `poll_event` event vocabulary)
> - pairs: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md) (accepted), [`RFC-0052 Amendment A1`](./RFC-0052-amendment-A1-host-provider-ui-surface.md) (accepted), [`RFC-0052 Amendment A2`](./RFC-0052-amendment-A2-ui-error-render-failed-and-event-visibility.md) (PROPOSED, sibling)
> - scope: the single remaining gap of the `sico:script/ui@1.0.0` event vocabulary after the `click(ui-handle)` delivery (STEP-0597): `select(ui-selection)` is declared in the contract but **not deliverable by the current model** — a contract/model gap, not an implementation gap. This amendment records that gap and offers three candidate resolutions; nothing is implemented or pre-adjudicated.
> - evidence basis: executable evidence only; where a document and the implementation disagree the **lower** support claim is taken (AGENTS §1). Every assertion below cites `file:line`.
> - **citation caveat:** the `runner/sico-runner` and `crates/sico-ui-renderer-d2d` trees are **under concurrent agent edit** (per the parent brief, three agents are working the runner/renderer). All `file:line` references below are the exact anchors **verified in the worktree on 2026-11-07**; they will drift. Any implementer MUST re-anchor against the committed snapshot before relying on a line number. Symbol names (`UiNode`, `UiTree::set_options`, `route_click_event`, `classify_window_event`) are the stable identity.

## 0. Discipline of this slice

- **Contract surface is `wit/script-ui-v1/ui.wit`.** The runner hand-registers the surface via `func_wrap` and the compiler hand-encodes the instance type; neither parses this file (`wit/script-ui-v1/ui.wit:9-11`). Any WIT change is a **contract change** and therefore requires owner acceptance plus a versioning note — none of the options below is decided.
- **No option is pre-adjudicated.** A1/A2/A3 are all presented as candidates; the recommendation in §3.4 is advisory and non-binding.
- **Lower bound where docs conflict with code:** the only ground truth used is the executable code (`runner/sico-runner/src/ui.rs`, `crates/sico-ui-renderer-d2d/src/*`) and the frozen WIT. Where a STEP's prose makes a stronger claim than the code can support, the code wins and the discrepancy is recorded.
- **This amendment is the `select` sub-slice of the C-C gap.** The `click` sub-gap was closed by STEP-0592 (layout + hit-testing) and STEP-0597 (runner delivery); `select` remains, and its cause is **not** "missing hit-testing" (hit-testing now exists) but **"the model has no selection state"**. That is a different gap class — see §1.

## 1. Verified facts (executable evidence)

### 1.1 The contract declares `select(ui-selection)`

- The guest-visible event vocabulary is `variant ui-event { click(ui-handle), select(ui-selection), closed }` (`wit/script-ui-v1/ui.wit:59-63`), with `record ui-selection { handle: ui-handle, index: u64 }` (`ui.wit:53-56`) — "which dropdown's which index is selected".
- RFC-0052 §2 carries the same surface: `poll_event() -> Option[UiEvent]  // UiEvent: click(handle) | select(handle, index) | closed` (`docs/rfc/RFC-0052-native-ui-source-binding-v0.md:39`).
- The runner's event type mirrors it: `UiEvent::Select(UiSelection)` (`runner/sico-runner/src/ui.rs:95-97`), `UiSelection { handle: UiHandle, index: u64 }` (`ui.rs:79-86`).

### 1.2 The dropdown **has** items — the index domain exists

- The WIT creates a dropdown **with** an option list: `dropdown: func(parent: ui-handle, options: list<string>) -> result<ui-handle, ui-error>` (`wit/script-ui-v1/ui.wit:71`); RFC-0052 §2 mirrors it (`RFC-0052:30`).
- The tree stores that list: `pub options: Vec<String>` on `UiNode` (`crates/sico-ui-renderer-d2d/src/tree.rs:79`), set at creation (`backend.rs:146-150` inserts `NodeKind::Dropdown` then `set_options`) and mutable via `UiTree::set_options` (`tree.rs:228-249`).
- **Therefore the item list (the domain of the `index`) is present in the model** — this is NOT the "dropdown has no items" case.

### 1.3 The model has **no selection state** — the selected index is not representable

- `UiNode`'s full field set is `kind`, `parent`, `children`, `text`, `options`, `image`, `progress` (`tree.rs:69-84`) — **there is no `selected` field**. A crate-wide search for `selected`/`selection` in the renderer core and the runner binding finds only the *event-payload* types `UiSelection`/`UiEvent::Select` (`ui.rs:79-86,95-97`) and the WIT/contract declarations — **no model state field anywhere**.
- The state-op surface is closed at `set-text` / `set-progress` / `draw-image` (`wit/script-ui-v1/ui.wit:76-80`; RFC-0052 §2 `set_text`/`set_progress`/`draw_image` — `RFC-0052:35-36`). None of them sets a selection:
  - `set_text` replaces `node.text` (`tree.rs:207-219`) — a dropdown's own label text, not an option index;
  - `set_progress` sets `node.progress` (`tree.rs:258-268`);
  - `draw_image` sets `node.image`.
- There is **no `set-selected` op**, **no `selected` field**, and **no selection lifetime** (no default, no clear-on-options-change rule) anywhere in the surface or the model.

### 1.4 No per-option hit geometry — a click cannot yield an index

- A dropdown **cannot have children** in the tree: `accepts_children` is `Root | Column | Row` only (`tree.rs:45-47`) — the options are a `Vec<String>`, **not** child nodes.
- The layout treats a dropdown as a **single leaf box**: it falls in the leaf branch `Text | Button | Dropdown | ImagePreview | Progress => (own_width, own_height)` (`crates/sico-ui-renderer-d2d/src/layout.rs:429-433`), with `natural_size(Dropdown) = (text_advance + DROPDOWN_CHEVRON_WIDTH + 2*CONTROL_PADDING_H).max(MIN_BUTTON_WIDTH), CONTROL_HEIGHT` (`layout.rs:470-476`).
- Consequently the layout has **no per-option rectangle**: a point inside a dropdown resolves to the dropdown node itself (`Hit::Node(dropdown)`), and the runner routes it to `click(dropdown_handle)` via `route_click_event` (`ui.rs:304-316`, `layout.rs:197` `hit`). There is **no path from a coordinate to an option index**.

### 1.5 No dropdown-open/select OS message

- The window event vocabulary is closed at `Close` / `Resize` / `Paint` / `LeftButtonDown` (`crates/sico-ui-renderer-d2d/src/events.rs:38-57` `RawMessage`; `events.rs:65-85` `WindowEvent`); `translate` maps them deterministically (`events.rs:98-117`). There is **no** "dropdown item selected" or "dropdown opened" message, so the shell produces no selection event of its own.

### 1.6 Verdict (verified, lower-bound)

- The contract declares `select(ui-selection)` and the dropdown **has** items (the `index` domain exists), but **the model cannot express "which index is selected"**: no selection-state field (§1.3), no settable-selection op (§1.3), no per-option hit geometry (§1.4), and no OS selection message (§1.5).
- This is a **contract/model gap** — the gap is in the *model*, not in a missing implementation step. STEP-0597 records the same conclusion (`docs/steps/STEP-0597-m24-runner-click-handle-delivery.md:187-189`: "模型无选中状态、shell 不产生 select 事件 ⇒ 需要新的模型能力"); this amendment confirms it against the code and offers resolutions.
- **Consequence for a guest:** a guest that polls `poll-event` in a loop sees `click(handle)` and `closed` but **never** `select(ui-selection)`, and cannot tell "the user made no selection" from "a selection happened but is not deliverable" — a **guest-unobservable skip of a contract-visible event**, the same gap class STEP-0590's C-C recorded for clicks before layout landed, but with a deeper cause (a missing model capability, not a missing hit-test).

## 2. Decision point — how to resolve the `select` gap

The three candidates below are the three honest postures, mirroring the A2 amendment's A/C framing adapted to a *model* gap. All three are **owner-decided**; nothing here is implemented.

### 2.1 A1 (option) — narrow the contract: "this version does not deliver `select`"

- **Change:** no WIT change, no code change. Write into RFC-0052 §2 and the user documentation the explicit narrowing: "until the model carries dropdown selection state, the `ui@1` surface delivers only `click(handle)` and `closed`; `select(ui-selection)` is declared but **not produced** in the current model." Update the acceptance clause (STEP-0590 §6 G4) to split the click/select sub-items so the verdict is precise.
- **Effect:** converts the guest-unobservable skip into a **declared** narrowing (default-deny-style honesty, exactly STEP-0590's C3 for clicks). A guest reading the contract now knows `select` will not arrive; it no longer has to guess. It **does not** add the capability — that stays A2.
- **Invariant impact:** none on the model or the runner — the `select` discriminant stays unreachable (a declared-but-unproduced case, the same shape as `event-overflow` pre-A2/B1b). Wire-compatible; no versioning note beyond the doc text.
- **Compatibility:** additive doc-only; existing guests unchanged. The `select` payload (`ui-selection`) and discriminant keep their wire values.
- **Recommended slices:** documentation + acceptance-clause wording only. No STEP code.

### 2.2 A2 (option) — extend the model with dropdown items + selection state (the real fix)

- **Change:** introduce a selection state into the model and a deterministic source for it. Minimum shape:
  - a `selected: Option<u64>` field on `UiNode` (or an equivalent per-dropdown selection slot), with a declared lifetime (default = `None`/unselected; cleared or clamped when `options` change; dropped with the node);
  - a new **additive** state op (e.g. `set-selected(handle, index) -> result<bool, ui-error>`) so a guest can set the selection deterministically, **or** per-option hit geometry so a click on an option row yields the index;
  - the runner's `poll_event` routes a selection change to `select(ui-selection)`.
- **Determinism source:** the option list is already a frozen pure value (`tree.rs:79`, `tree.rs:228-249`); a selection index is an integer in `0..options.len()`, bounded and validated (out-of-range ⇒ typed refusal). Per-option geometry would be derived from the frozen token table exactly like `layout.rs` derives the dropdown box (`layout.rs:470-476`), so the hit stays cross-machine deterministic.
- **Selection lifecycle & relation to `click`:** must be declared in the new ADR/contract. Open sub-decisions (not pre-adjudicated here): does a click on an option row produce `select{handle,index}` **and/or** `click(handle)`? Does `select` require a prior "dropdown open" state, or is it a direct point-to-option hit? What is the initial selection? All must be fixed before implementation.
- **Effect:** makes `select` a **deliverable** event with a model-derived, deterministic index; closes the §1.6 gap at its root.
- **Cost:** the largest. A new ADR/contract decision for the selection model (the layout must gain per-option geometry — `layout.rs` is frozen-semantics and `tests/layout_corpus.rs` is frozen-hash, so either the corpus is re-frozen under owner authorization or the selection geometry is a separate corpus); the WIT append for `set-selected` (additive, wire-compatible under `ui.wit:25-26`); the model field + op; runner routing + `select` test; lockstep compiler hand-encoded instance type if the surface grows.
- **Compatibility:** WIT append is wire-compatible (append-only `ui.wit:25-26`); existing guests unchanged unless they exhaustively match the (unchanged) `ui-event`/`ui-error`.

### 2.3 A3 (option) — make `select` a host-side event (user selects through another channel; the UI library only observes)

- **Change:** no Sico-model selection state. The host owns the selection: a native OS dropdown/combobox control (created and driven by the host, e.g. the ADR-0018 renderer) reports the user's selection to the host, and the host maps that native control to a `dropdown` node handle and produces `select{handle, index}`. The Sico guest's UI library only **observes** the resulting event; it never sets or holds the selection in the tree model.
- **Determinism:** the index comes from the **host's** control state (user-driven), not from a pure function of the Sico tree — the event is a host event, like `closed`/`click`, not a derived model value. It is deterministic in the sense that the same user action yields the same event, but the selection state lives outside the model.
- **Invariant impact:** the tree model stays **selection-free** (no new field, no new op); the `select` payload's `handle` must name an existing `dropdown` node, so the host keeps a host-side handle↔native-control map. The model does **not** need to represent the selected index — the event carries it.
- **Compatibility:** no WIT change (the `select` discriminant and `ui-selection` payload already exist); needs the host-side native control + mapping + runner routing.
- **Recommended slices:** (a) ADR for a host-owned dropdown control; (b) native control wiring in the ADR-0018 renderer; (c) host-side handle↔control map; (d) runner routing to `select`; (e) test. This is a *different architectural posture* than A2: the guest cannot set or inspect selection in the model — it only receives the host event.

### 3.4 Recommendation (advisory)

**A1 now, A2 later.** Rationale, mirroring the already-accepted C3-then-C1 pattern for clicks (STEP-0590 §4.4, then STEP-0592 + STEP-0597 delivered `click`):

- A1 is the only option consistent with the codebase's honesty posture (declare a narrowing rather than silently skip — `ui.rs:347-356` posture, STEP-0581:58-60) and requires **no** new capability, no WIT change, no corpus re-freeze. It removes the guest-unobservable-skip honesty gap immediately.
- A2 is the eventual correct fix — the only option that makes `select` a **deliverable** event with a model-derived index — but it is a **new model capability** needing its own ADR/contract, a WIT append, layout corpus decisions and a lockstep compiler hand-encoded update; it must not be assumed into this slice.
- A3 is a viable alternative that keeps the model selection-free, but it moves selection ownership out of the Sico model entirely (the guest cannot set/observe selection state, only receive the event), which is a different posture; it is a larger renderer change (native control) and does not let a Sico guest drive a selection deterministically.

**Owner-decided.** Until accepted, the code stays as-is: `select` is not produced, `click`/`closed` are (STEP-0597), and the gap is registered by this amendment and by STEP-0599.

## 4. Compatibility impact (WIT versioning)

- The interface is versioned `sico:script/ui@1.0.0` (`wit/script-ui-v1/ui.wit:22`); the runner registers the instance by that name (`runner/sico-runner/src/ui.rs:34` `UI_INTERFACE`). Neither the runner nor the compiler parses the WIT (`ui.wit:9-11`).
- **A1** and **A3** need **no** WIT change (the `select` discriminant and `ui-selection` payload already exist; A3 only adds a host-side producer).
- **A2** is a **wire-compatible append** under the append-only rule (`ui.wit:25-26`): a new `set-selected` op appended to the surface keeps existing discriminants and signatures; existing guests need no change unless they want the new op. If A2 also adds per-option geometry it changes the layout corpus hashes and (if options are drawn) the frame corpus hashes — those re-freezes are **owner-authorized** actions, not part of any option's automatic scope.
- Any option that touches the contract text requires the RFC-0052 §2/§3 text and the versioning note to be updated together; no field name changes (identifiers stay English, AGENTS §7).

## 5. Open questions (owner-decision items, not pre-adjudicated)

1. **A1 vs A2 vs A3** — narrow now (A1), extend the model (A2), or host-side event (A3)?
2. **If A2:** is the selection set by a guest op (`set-selected`), by per-option click geometry, or both? What is the initial selection and the lifetime (clear-on-options-change)?
3. **If A2:** does a click on an option row produce `select{handle,index}` **and/or** `click(handle)`? (Currently it produces `click(dropdown_handle)` — `layout.rs:429-433,470-476`, `ui.rs:304-316`.)
4. **If A2:** does per-option geometry change the frozen layout/event corpus hashes, and is a re-freeze (owner-authorized) acceptable in the same milestone or deferred?
5. **If A3:** does a host-owned native dropdown compose with the existing "no file dialog / no native control yet" residual (STEP-0581:133-138), or does it need the pilot/ADR-0018 renderer to grow a control host first?
6. **Acceptance clause:** should G4 (STEP-0590 §6) be split into G4a `click` / G4b `select` permanently, so the verdict is precise and the two sub-items are tracked independently?

## 6. Honest narrowing / non-claims

- This amendment **does not implement** anything and **does not change** any runtime behaviour; it records the `select` model gap and offers options. On owner acceptance, a separate implementation STEP is allocated and the RFC-0052 text + versioning note + user docs are updated together.
- No option above is decided; no code path is proposed to be flipped here.
- **Non-claims:** this amendment claims no `select` delivery, no selection state, no new op, no per-option geometry, no native control, no WIT change, no model-semantics change. `click`/`closed` delivery remains as delivered by STEP-0597; the `select` gap remains open.
- **Relation to A2 (sibling):** A2 (PROPOSED) addresses `ui-error` render-failure + event-overflow observability; this A3 is the `select` sub-gap of the same C-C review. They are independent; A2's C-C wording ("do not deliver click/select until layout lands") is now **superseded for `click`** (STEP-0592/0597 delivered it) but **still applies to `select`** until this A3 is decided.
- **Citation caveat repeated:** all `file:line` values above were verified in the worktree on 2026-11-07 and the runner/renderer are under concurrent agent edit; re-anchor before use.
