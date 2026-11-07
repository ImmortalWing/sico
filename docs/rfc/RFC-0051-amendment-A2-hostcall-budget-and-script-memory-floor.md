# RFC-0051 Amendment A2: hostcall budget semantics + Script memory floor (PROPOSED)

> - status: **PROPOSED — draft for owner adjudication** (raised 2026-11-07, STEP-0583).
>   **This amendment drafts two contract fixes but does NOT implement anything and does
>   NOT change behavior.** No option below is "decided" or "recommended-as-accepted";
>   the owner picks, and only then is a separate implementation STEP opened.
>   Until the owner accepts, gate 4 stays as STEP-0578 left it (hostcall run-time
>   enforcement and the real-Script-component memory path **未达**), and the
>   `provenance.rs`/`lib.rs`/`narrowed_by` behavior stays exactly as it is today.
> - date: 2026-11-07
> - amends: [`RFC-0051`](./RFC-0051-model-provenance-manifest-v0.md) §2 (the *meaning* of
>   `budgets.max_hostcalls` and the *admissible range* of `budgets.max_memory_bytes`) and
>   §3 (evidence properties 3-runtime/4), and
>   [`Amendment A1.3`](./RFC-0051-amendment-A1-carriage-and-budget-mapping.md) (the
>   `budgets → RuntimeLimits` mapping/precedence). A1 is unchanged except where A2 is
>   accepted; the two amendments are compatible with every option below.
> - evidence: [`STEP-0578`](../steps/STEP-0578-m24-gate4-provenance-runtime-enforcement.md)
>   §6 (measured contract gaps C1/C2); `runner/sico-runner/src/lib.rs`;
>   `crates/sico-package/src/lib.rs`; `crates/sico-package/src/provenance.rs`;
>   `crates/sico-codegen-wasm/src/canonical.rs`. All line numbers below are against the
>   worktree snapshot of 2026-11-07 (other agents are concurrently editing
>   `runner/`/`crates/`; re-verify at implementation time).

## 1. Why this amendment exists (measured contract silence, not a design debate)

STEP-0578 §6 recorded two run-time contract gaps that the accepted body left open.
Both are **measured facts about the current implementation**, not hypotheticals.

### C1 — the hostcall budget does not take effect at run time, and its unit is wrong

The manifest field is declared and documented as a **count**:

- `crates/sico-package/src/provenance.rs:66-67` — `max_hostcalls` is "Host-call **count** ceiling";
- `docs/rfc/RFC-0051-model-provenance-manifest-v0.md:31-34` — §2 `budgets.max_hostcalls: U64`
  (a count); §3 property 3 (line 53) speaks of "hostcall-**count** and wall-time brackets".

But A1.3 maps it onto `RuntimeLimits.hostcall_fuel`, which is **not** a count:

- `runner/sico-runner/src/lib.rs:238-239` — the field is documented as "Canonical-ABI
  transport budget (elements are charged per value)";
- `runner/sico-runner/src/lib.rs:1342` — `STREAM_HOSTCALL_FUEL_PER_CALL = 64 << 20` (named
  **per call**), applied through `store.set_hostcall_fuel(...)` at `lib.rs:1346`;
- `runner/sico-runner/src/lib.rs:2235` and `lib.rs:2517` — `store.set_hostcall_fuel(needed)`
  sets a **fuel cost per host call** (Wasmtime semantics), i.e. a *rate*, not a total count.

So the same field is described three incompatible ways (manifest: *count*;
`RunnerLimits` doc: *transport elements per value*; Wasmtime setter: *fuel units per
hostcall*). A1.3's mapping `max_hostcalls → hostcall_fuel` therefore assigns a count to a
per-call rate:

- `runner/sico-runner/src/lib.rs:278-279` (`narrowed_by`) — `min(hostcall_fuel, max_hostcalls)`;
- `crates/sico-package/src/provenance.rs:269` (`check_budgets_within`) — `max_hostcalls > package.hostcall_fuel`.

Neither is a count comparison; the "count ceiling" the manifest promises is never
enforced as a count.

Independently, the run path floors the value, so even the (mis-typed) mapping is inert
below a large floor:

- `runner/sico-runner/src/lib.rs:2229-2235` (`start_debug`) and `lib.rs:2511-2517`
  (`invoke`) compute `needed = stdin.len()*64 + (64 << 20)` and then
  `needed = needed.max(limits.hostcall_fuel)`, i.e. `set_hostcall_fuel(max(limits.hostcall_fuel, floor))`.

Concrete numbers with the frozen fixture (`model-fixture.provenance.json:17`,
`max_hostcalls: 100000`): `narrowed_by` sets `hostcall_fuel = min(128 << 20, 100000) = 100000`
(`RunnerLimits::default().hostcall_fuel = 128 << 20`, `lib.rs:248`), then the empty-stdin
floor is `64 << 20 = 67,108,864`, so `set_hostcall_fuel(max(100000, 67108864)) = 67108864`.
The 100000 budget is **fully covered** by the 64 MiB floor: lowering it further changes
nothing. A hostcall budget below `stdin_len*64 + 64 MiB` has no observable effect.

Net: the hostcall budget is simultaneously (1) unit-mismatched (count vs per-call fuel
rate) and (2) floored away below 64 MiB. Neither defect makes the run safer or the
budget meaningful.

### C2 — a real Script component's declared memory makes small memory budgets unsatisfiable

- `crates/sico-codegen-wasm/src/canonical.rs:28` — `MEMORY_PAGES = 33792`;
  `canonical.rs:39` — `SCRIPT_MEMORY_BYTES = 33792 * 65536 = 2,214,592,512` (≈ **2.06 GiB**),
  pinned by `crates/sico-codegen-wasm/tests/script_abi.rs:35-36`.
- Every Script component declares linear memory with `minimum == maximum == MEMORY_PAGES`
  (`crates/sico-codegen-wasm/src/package_builder.rs:119-120`, `lib.rs:1255-1256`,
  `lib.rs:1337-1338`, `canonical.rs:975-976`): the declaration is **fixed**, not a range.
- `crates/sico-package/src/lib.rs:151-157` — `script_profile_limits().memory_bytes = SCRIPT_MEMORY_BYTES`;
  `lib.rs:143-149` (STEP-0497 note) documents that a package limit below the declared
  minimum is **unsatisfiable**, not tighter: "wasmtime cannot satisfy the minimum at
  instantiation and the launch dies with an opaque 'forcing trap when growing memory to … bytes'".
- The build-time guard already rejects this at `lib.rs:441-446`
  (`if input.limits.memory_bytes < SCRIPT_MEMORY_BYTES → Manifest refusal`).

The provenance budget re-opens the same hole on the **run-time** path, because
`check_budgets_within` only checks the subset direction and `narrowed_by` then sets the
ceiling below the declared minimum:

- `crates/sico-package/src/provenance.rs:255-275` — `check_budgets_within` checks
  `max_memory_bytes > package.memory_bytes` only (subset), **not** a floor; the frozen
  fixture `max_memory_bytes = 1,048,576` (`model-fixture.provenance.json:15`) passes the
  subset check against the script ceiling (`2,214,592,512`).
- `runner/sico-runner/src/lib.rs:275-276` — `narrowed_by` sets
  `memory_bytes = min(package, 1,048,576)`; `lib.rs:2155`/`lib.rs:2369` set
  `memory_ceiling` to that; `lib.rs:1350-1360` (`memory_growing`) denies any growth past it.
- Instantiating a real Script component needs `SCRIPT_MEMORY_BYTES` immediately
  (`min == max`), which the 1 MiB ceiling cannot satisfy → the same opaque instantiation
  trap STEP-0497 fixed at build time, now arriving from the *budget* instead of the
  *package limit*.

**Honesty note:** STEP-0578 §4/§5 deliberately ran the run-time memory test with a
hand-built minimal guest (1-page declared memory), not a real Script component, precisely
because of this gap (`STEP-0578:51`). So the C2 run-time trap is the documented
STEP-0497 mechanism applied to the budget path, **not a fresh measurement on a real
Script component under a provenance budget**. It is mechanism-documented, not
newly-measured; re-measure before claiming the floor is the fix.

## 2. A2 proposals (each decision point needs an owner pick)

### A2.1 C1 — hostcall budget semantics

Two mutually exclusive options. Both are behavior-relevant; neither is a no-op.

**Option (a) — re-interpret `max_hostcalls` as the per-hostcall fuel cost (recommended).**

Keep the A1.3 mapping byte-for-byte (`max_hostcalls → hostcall_fuel`, subset+limit+1
refusal), and change only the *meaning*: the manifest field stops being a "count ceiling"
and becomes "fuel units charged per host call". Update the schema doc
(`provenance.rs:66-67`), the `RunnerLimits.hostcall_fuel` doc (`lib.rs:238-239`), and
RFC-0051 §2/§3 wording. `check_budgets_within` (`provenance.rs:269`) and `narrowed_by`
(`lib.rs:278-279`) then compare/assign like for like (fuel units vs fuel units).

Consequences:

- **No new upper-limit path.** Preserves the A1.3 / STEP-0578 invariant ("only narrow,
  never a second ceiling path", `lib.rs:1554-1561`; AGENTS.md §helpers narrow never
  widen). No new field, no new refusal discriminant, no default-deny change.
- **Semantic mismatch remains with the §2 field name** (`max_hostcalls`) and RFC-0051
  §3 property 3 ("hostcall-count ... brackets"): under (a) the budget does **not** bound
  the number of host calls. The *effective* call count becomes `~ total_fuel / per_call_cost`,
  governed by the caller's own `limits.fuel`, not by the budget.
- **The fixture threshold must be re-derived, not reused.** `max_hostcalls = 100000`
  under (a) means "each host call costs 100000 fuel units"; with `RunnerLimits::default().fuel
  = 10_000_000` (`lib.rs:245`) that is `~100` host calls, not 100000. Whether that is the
  intended bound must be re-derived; the value is not interchangeable.
- **Guest-observable difference vs (b):** a guest whose host calls are individually cheap
  (each below the per-call cost) is **not** bounded in call count by the budget — only the
  total fuel caps it. A host call at or above the per-call cost exhausts fuel and yields
  typed `FuelExhausted`. There is no distinct "hostcall-count" outcome.

**Option (b) — keep "count" semantics; add a literal run-time hostcall-count limit.**

Add a new limit kind (e.g. `hostcall_count_max` on `RunnerLimits` and a run-time counter
in `RunState`), checked after each host call, with its own typed refusal discriminant.
`check_budgets_within` keeps comparing `max_hostcalls` (count) against the new count field.

Consequences:

- **New upper-limit path.** This is exactly the "second ceiling path" the A1.3 body and
  STEP-0578 deliberately avoided (`lib.rs:1554-1561`). It therefore needs its own RFC
  text and a **default-deny posture** for the new field (absent/unknown → a defined
  default, never an open ceiling), plus a per-call counter increment point in the host
  call path (a measured overhead — currently zero on the hostcall hot path).
- **Semantics match the §2 name and property 3 exactly** ("hostcall-count bracket");
  a guest that makes many cheap calls is bounded in *count*, independent of total fuel.
- **Guest-observable difference vs (a):** cheap host calls count against the budget; the
  guest gets a typed "hostcall budget exceeded" outcome even when its fuel is far from
  exhausted. Total fuel no longer implicitly bounds the call count.
- **M7 / `.sapp` impact:** a `.sapp` carrying `max_hostcalls` keeps installing (the new
  field is a limit kind, not a schema field), but its budget now constrains a count, so
  an old `.sapp`'s meaning changes silently unless the limit kind is versioned.

**Recommendation: (a).** It reuses the existing limit kind, preserves the no-new-path
invariant, and costs only a documentation/semantics correction plus a threshold
re-derivation. Its honest cost is that "count" is not literally enforced — the owner must
accept the field meaning *is* per-call fuel cost. If the owner's intent for RFC-0051 §3
property 3 is a literal count, **(b)** is the only faithful option, and the "no second
upper-limit path" line must be re-adjudicated (it was recorded as an A1.3 preference, not
an absolute).

### A2.2 C2 — Script component memory floor

**Option (a) — require `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES` for Script packages,
else an install-time typed refusal (recommended).**

For a package whose manifest carries a script identity (`Package.script.is_some()`,
`lib.rs:51`; set by `build_script_v1` at `lib.rs:471`; `None` for `build_unsigned` at
`lib.rs:409`), the install-time check gains a floor: `budgets.max_memory_bytes <
SCRIPT_MEMORY_BYTES` → a distinct typed `Manifest` refusal at install, before any guest
execution. The budget's admissible range becomes
`[SCRIPT_MEMORY_BYTES, package.memory_bytes]`; it still only narrows.

Consequences:

- **Fail-closed earlier and typed**, no guest execution, no opaque trap. Consistent with
  the already-accepted STEP-0497 build-time principle (`lib.rs:441-446`), applied at the
  budget boundary. **No default-deny weakening** — a budget below the floor is now
  refused instead of silently trapping.
- **Semantics:** the budget's memory field means "the asset's guest memory is at least the
  component's declared linear memory"; a budget below it is not a tighter budget, it is
  an unsatisfiable one (the same reasoning STEP-0497 codified for the package limit).
- **Breaks the frozen fixture and three tests.** `max_memory_bytes = 1,048,576 <
  2,214,592,512` ⇒ install refusal. `crates/sico-package/tests/provenance_budget.rs:75`,
  `runner/sico-runner/tests/provenance_runtime.rs:183` and the
  `provenance_install.rs` state-1 "budget ⊆ ceiling installs" test all assert
  `1,048,576`; `provenance_runtime.rs:321-372` end-to-end also depends on the 1 MiB value.
  **Re-baseline is required**: raise the fixture `max_memory_bytes` to `≥ SCRIPT_MEMORY_BYTES`
  (which makes the memory budget non-narrowing and so the narrowing tests exercise the
  floor, not the narrowing) — or re-scope the test guest. This is the real cost of (a).
- **RuntimeLimits / fuel / timeout:** unchanged. Only the memory budget's admissible
  range gains a floor; the total fuel and wall bounds are untouched.
- **M7 / `.sapp`:** existing `.sapp` packages carrying a Script identity and a memory
  budget below the floor would newly refuse at install. If any such `.sapp` exists in
  the corpus this is a behavior change on old packages; search says the only frozen
  fixture is `model-fixture` (1 MiB), which would be affected.

**Option (b) — re-describe the budget as "asset operation memory", decoupled from the
component's declared memory.**

`max_memory_bytes` bounds only the growth above the declared baseline, not total guest
linear memory. `narrowed_by` computes `memory_bytes = SCRIPT_MEMORY_BYTES + max_memory_bytes`
(baseline + growth budget), or leaves `memory_bytes` at the package value and adds a
separate growth accounting. The install-time subset check stays
`max_memory_bytes ≤ package.memory_bytes` (or is re-scoped to a growth ceiling).

Consequences:

- **Semantic weakening of the total-memory promise.** A reader of
  `max_memory_bytes = 1 MiB` would believe the asset cannot exceed ~1 MiB; under (b) the
  asset can use `2.06 GiB + 1 MiB`. This must be stated to the owner **plainly**: "this
  budget does NOT constrain the guest's linear memory; it constrains only the growth
  above the declared baseline." The default-deny posture for the memory ceiling is
  relaxed for the declared baseline.
- **The frozen fixture keeps working** (1 MiB stays a valid growth budget; no
  re-baseline of the three tests, modulo the growth-accounting change).
- **RuntimeLimits / fuel / timeout:** the memory ceiling becomes baseline + growth; total
  fuel and wall bounds unchanged.
- **M7 / `.sapp`:** an old `.sapp` carrying the field keeps installing but its budget now
  means something different — a silent semantic change unless the field is renamed or
  versioned (a schema change, which A2 is not claiming).

**Recommendation: (a).** It keeps "the budget constrains guest memory" as the honest
contract, refuses the unsatisfiable case early and typed, and mirrors STEP-0497's already
accepted principle (fixed `min == max` declaration ⇒ a smaller budget is unsatisfiable,
not tighter). (b) is defensible only if the owner intends the budget to govern
asset-relative growth; if chosen, the honest framing above is mandatory and the field
should arguably be renamed/versioned to avoid implying a total-memory bound.

## 3. What A2 does NOT change

- The §2 **schema** (field names, closedness, `sico.m17.provenance.v0`); only the
  *meaning* of `max_hostcalls` (under A2.1-a) and the *admissible range* of
  `max_memory_bytes` (under A2.2-a) change, and only if the owner accepts.
- The **subset + limit+1 refusal shape** (A1.3): A2.1-a keeps it byte-for-byte;
  A2.2-a keeps it and adds a floor (still fail-closed).
- The §3 evidence-set *definition*, §4 honesty gates (a fixture flips gate 4 only, no
  live-model / quality / speed claim — ADR-0012 gap unchanged), and §4 non-goals.
- Properties 3-runtime/4 **still need** the executed fixture evidence STEP-0578 produced
  (memory / wall / fuel / cancel). What A2 fixes is the hostcall-count path (C1) and the
  real-Script-component memory path (C2) — the two paths STEP-0578 §7 explicitly left
  **未达** because of these gaps.

## 4. Adjudication requested (owner)

Decision points, each accept-as-recommended or redirect:

1. **C1:** (a) re-interpret `max_hostcalls` as per-hostcall fuel cost (recommended) vs
   (b) add a literal hostcall-count limit (new limit path, needs its own RFC).
2. **C2:** (a) require `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES` for Script packages at
   install (recommended) vs (b) re-describe as asset-operation memory (weakens the
   total-memory promise).
3. **Invariant question:** is "no second upper-limit path" (A1.3; `lib.rs:1554-1561`)
   a **hard invariant** (then C1 must be (a) and C2 must be (a)) or a preference that
   C1-b may re-adjudicate?
4. **CLI/app wiring:** should `VerifiedPackage::provenance_budgets` /
   `Runner::run_program_with_budgets` be wired into `sico-cli` / `sico-app-cli` **before**
   or **after** these two decisions? (Orthogonal either way; but if C1-b adds a limit
   kind, the wiring's limit set changes.)
5. **Fixture re-baseline:** if A2.2-a is accepted, who re-baselines the frozen fixture
   budget and the three asserting tests, and is a separate non-narrowing test needed
   (since a `≥ SCRIPT_MEMORY_BYTES` memory budget on a `SCRIPT_MEMORY_BYTES` ceiling makes
   the memory-narrowing test vacuous)?

Until accepted, gate 4 stays **未达** on the hostcall-count and real-Script-component
paths, and no behavior, package format, or runner limit changes on my own authority.
