# RFC-0051 Amendment A1: package-carried manifest binding + budget mapping (ACCEPTED)

> - status: **accepted by owner re-adjudication 2026-10-01** (raised 2026-09-30, STEP-0334 follow-on). The owner accepted **all three recommended options** (§4 decision points 1–3): A1.1 name-keyed sibling carry `resources/<name>[.provenance.json]`, A1.2 name-keyed digest/size cross-check binding, A1.3 budget-must-be-package-ceiling-**subset** with a distinct over-ceiling (limit+1) fail-closed refusal (not run-time intersection). The **load/verify-boundary** half is now implemented in `crates/sico-package/src/provenance.rs` (`provenance_resource_path`/`payload_resource_path`, `verify_binding`, `check_budgets_within`; 6 new unit tests, internal-fixture, `sico-package` suite green, clippy `-D warnings`/fmt clean). The **run-time** budget *enforcement* (executing a fixture-model guest under the mapped limits) and **cancellation** mid-inference (RFC-0051 §3 properties 3-runtime/4) still require the runner wiring + a frozen fixture-model payload and stay **未达**; gate 4/M24 §3 gate 2 does **not** flip on this slice.
> - date: 2026-09-30 (accepted 2026-10-01)
> - amends: [`RFC-0051`](./RFC-0051-model-provenance-manifest-v0.md) §2 (how the manifest is *carried inside the package* and *bound* to its payload), §3 (how the four §3 evidence properties are produced at the package/runner boundary)
> - evidence: [`crates/sico-package/src/provenance.rs`](../../crates/sico-package/src/provenance.rs) (schema + `parse`/`validate`/`verify_asset`, STEP-0334, 10/10 unit); `crates/sico-package/src/lib.rs` `RuntimeLimits` fields `fuel/timeout_ms/memory_bytes/hostcall_fuel` (lib.rs:101-113) and the M7 install-time per-resource digest check (`prepare_resources`/`validate_manifest`, lib.rs:425-450/786-800)

## 1. Why this amendment exists (contract silence, not a measured contradiction)

RFC-0051 as accepted freezes the manifest **schema** (`sico.m17.provenance.v0`)
and states (§2) that it is "carried inside the package" and that "digest
verification happens at **package install** (M7, existing) and again at
**asset load**". The schema layer is now implemented and unit-tested
(STEP-0334; RFC-0051 §3.1/§3.2/§3.5-structural). But to turn the schema into the
package-carried, runner-enforced gate-4 evidence set, three wiring rules must
be fixed, and RFC-0051 §2 leaves them **open**:

1. **Carry location** — where inside the package the manifest lives. M7
   resources all sit under `resources/<path>` and are digest-checked at
   install; but RFC-0051 does not name the manifest's resource path, so the
   verifier cannot yet locate it.
2. **Name→payload binding** — which carried resource the `asset.sha256` /
   `asset.size` fields verify against. §3.5 lists "manifest/payload mismatch"
   as a probe, which presupposes a binding rule the contract has not pinned.
   (Two natural rules exist: name-keyed path vs content-addressed digest.)3. **Budgets→run mapping** — RFC-0051 §2 `budgets` are
   `max_memory_bytes / max_wall_ms / max_hostcalls`. The runner already owns
   exactly three matching limit kinds (`RuntimeLimits.memory_bytes`,
   `.timeout_ms`, `.hostcall_fuel`), so §3 property 3 must reuse them rather
   than add a second limit path. The precedence rule (does an asset budget
   replace, intersect, or have to be a subset of the package's declared
   limits?) is unspecified, as is the refusal when a budget exceeds the
   package ceiling.

This is the same class of gap RFC-0050-A1 filled for the codec surface. It is
raised as a **draft for owner re-adjudication** per 不静默改合同 — the
implementer does not pick the convention unilaterally.

## 2. A1 proposals (recommended defaults; each needs an owner decision)

### A1.1 Carry location

Pin the manifest to a deterministic resource path derived from the declared
asset name:

```text
resources/<asset.name>.provenance.json   ← the sico.m17.provenance.v0 manifest (UTF-8 JSON)
resources/<asset.name>                    ← the payload the manifest describes
```

v0 is single-asset per manifest (RFC-0051 §2 root `asset:` is singular, and
`provenance.rs` models exactly one `asset`), so one manifest + one payload per
asset; a package may carry several such pairs. Both entries remain ordinary
M7 resources — install-time digest verification is unchanged, no parallel
trust path. `ProvenanceManifest::parse` already rejects a path-like `name`
(`/ \ .. :`), so `resources/<name>` is safe to construct.

*Alternative (decision for owner):* an explicit manifest index resource
(e.g. `provenance/manifest.json`) rather than name-keyed siblings.

### A1.2 Name→payload binding

Recommended: **name-keyed** — `asset.name` selects the sibling resource
`resources/<asset.name>`, whose stored M7 artifact digest must equal
`asset.sha256` and whose stored byte length must equal `asset.size`; any
disagreement is the §3.5 "manifest/payload mismatch" typed refusal, distinct
from truncated (`Truncated`) and oversized (`LengthMismatch`) (the three
discriminants `verify_asset` already returns).

*Alternative:* content-addressed — bind by `asset.sha256` matching some
resource's digest and forbid a second resource with the same digest. Kept as
an option; it makes the §3.5 mismatch probe harder to express, so name-keyed
is recommended.

### A1.3 Budgets→run mapping + precedence

Map each budget onto the existing limit kind, no new field:

```text
budgets.max_memory_bytes → RuntimeLimits.memory_bytes
budgets.max_wall_ms      → RuntimeLimits.timeout_ms
budgets.max_hostcalls    → RuntimeLimits.hostcall_fuel
```

Recommended precedence: the asset budget must be a **subset** of the package's
own `RuntimeLimits` — each `budgets.*` must be `≤` its mapped field, else
`ProvenanceManifest`/install produces a **distinct typed refusal**
("declared budget exceeds package ceiling"), and the operative ceiling for a
run that loads the asset through a function named in `asset.interface` is the
**asset budget**. This keeps the package limit the outer hard bound (never
widened by an asset — AGENTS.md §"standard-library/helpers narrow, never
widen") and makes property 3 (budget enforcement) ride the already-tested
fuel/epoch/limiter mechanism verbatim.

*Alternative (decision for owner):* intersect at run time instead of a subset
check at load time; the two differ in whether an over-ceiling budget is a load
refusal or a silent clamp — A1.3 recommends refusal (fail closed).

### A1.4 §3.5 malicious-asset probes → typed load refusals

With A1.1-A1.3 the four §3.5 probes are all load-time and map to already-
returned discriminants: signature-confused / mismatched bytes →
`DigestMismatch`; unknown/extra fields → `Manifest` (serde `deny_unknown_fields`,
implemented); path-like names → `Manifest` (implemented); over-ceiling budget
→ the A1.3 refusal. None requires guest execution, so this half of §3.5 becomes
runtime-verified at the **load/verify boundary** without an inference engine.

## 3. What A1 does NOT change

- The §2 **schema** (fields, closedness, digest hex form, M7 reuse) — already
  implemented and frozen in `provenance.rs`; A1 is compatible with every
  option chosen above.
- The §3 evidence-set *definition*, §4 honesty gates (a fixture flips **gate 4
  only**, no live-model / quality / speed claim — ADR-0012 gap unchanged), and
  §4 non-goals (runtime download, multi-file models, quantization metadata,
  GPU residency, license enforcement, inference acceleration).
- Properties 3 (budget enforcement) and 4 (cancellation mid-inference) still
  require **executing a fixture-model guest under the mapped limits** through
  the real runner and a frozen **fixture-model payload** (model assets = 0,
  STEP-0287). A1 fixes the *mapping and refusal shape*; it does not manufacture
  a model or flip a gate. Property 4 rides the existing `CancelToken`/epoch
  deadline already used by the Script and HTTP paths.

## 4. Re-adjudication requested (owner)

Decision points, each accept-as-recommended or redirect:

1. A1.1 carry location: name-keyed sibling `resources/<name>[.provenance.json]`
   (recommended) vs an explicit index resource.
2. A1.2 binding: name-keyed digest/size cross-check (recommended) vs
   content-addressed.
3. A1.3 precedence: budget must be a package-ceiling **subset** with a distinct
   over-ceiling refusal (recommended, fail-closed) vs run-time intersection.

Until accepted, gate 4 stays **未达** and `provenance.rs` stays the unit-tested
schema/verification primitive it is today; no package format version, runner
load path, or `sico-package` verifier behavior changes on my own authority.
