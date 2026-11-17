# STEP-0613f — M22 W3-a: co-gap refusal identity — the masked blocker is a frontend construct-field-value lowering gap, not an emitter arm

- Date: 2026-11-09
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation
- Status: finding recorded; emitter-arm repair (G2/G3) premise corrected
- Evidence level: internal-fixture (deterministic selfhost diagnostic probe, `--ignored`)

## 1. Why this slice

`STEP-0613d` and `STEP-0613e` attributed the co-gap blocker (a `run` body that
needs `sico.bytes.utf8_decode` / `sico.text.encode` / `text.trim` but still
refuses) to **emitter arms** in `selfhost/codegen.sico` (`cg_run_body_value_hex`,
`cg_expr_string_hex`, `emit_trim`). That attribution was inferred from the
driver's message `"unsupported codegen source shape"` plus host-oracle reading.

This slice tests the attribution directly: it surfaces the **specific** refusal
identity the way `STEP-0608` did — by feeding the guest's own canonical IR to
`codegen.component_refusal_for_ir` instead of trusting the driver's collapsed
message.

**Result: the premise was wrong.** The real first wall is in the **frontend**
(`selfhost/parser.sico`), before any emitter runs. This re-scopes G2/G3.

## 2. Probe

`runner/sico-runner/tests/selfhost_formatter_component_measurement.rs` ::
`m22_w3a_cogap_refusal_identity` (`#[ignore]`, run with `--ignored --exact
--nocapture`). It builds the selfhost frontend through the Rust compiler, then:

- runs each fixture through the normal driver (what the runner reports), and
- separately feeds the guest canonical IR to `codegen.component_refusal_for_ir`
  to name the first shape refused.

Log: `target/tmp/step0613f/cogap-refusal-identity.log`.

## 3. Finding — the driver masks a frontend refusal

For the whole co-gap fixture family the driver reports the generic mask:

```
[F*:driver] outcome = Domain code="invalid-input" message="unsupported codegen source shape" (typed refusal)
[F*:driver] ----
[F*] frontend: REFUSED (no IR reached the emitter)
```

`frontend=REFUSED (no IR reached the emitter)` is the decisive line: **no IR
reached `cg_component_hex`**, so the previously-blamed emitter arms were never
executed for these fixtures. The generic "unsupported codegen source shape" was
the *driver* wrapping a frontend error, not an emitter message.

`guest_refusal_identity` confirms this — when the frontend refuses,
`guest_canonical_ir` returns `None`, so the driver's own Domain message IS the
specific code and no emitter identity is reachable.

## 4. Bisect — pinning the exact shape

Varying one construct at a time against a known-good single-block echo `run`
(B0) isolates the trigger:

| Case | Construct under test | Frontend | Refusal identity |
|------|----------------------|----------|------------------|
| B0 | control echo run (`stdout: input.stdin`) | ok (baseline) | n/a |
| B1 | `stdout: sico.text.encode("hi")` | REFUSED | `ERR:E-SH-IR-CONSTRUCT-VALUE` |
| B2 | `let raw = utf8_decode(...)` (result **unused**) | ok | no-refusal |
| B3 | `let t = text.concat("a","b")` (result **unused**) | ok | no-refusal |
| B4 | `let raw = input.stdin` + `stdout: raw` | REFUSED | `ERR:E-SH-IR-CONSTRUCT-VALUE` |
| B5 | inline `text.encode(utf8_decode(...))` (no let) | REFUSED | `ERR:E-SH-IR-CONSTRUCT-VALUE` |
| B6 | inline `text.encode(text.concat("a","b"))` (no let) | REFUSED | `ERR:E-SH-IR-CONSTRUCT-VALUE` |

**Trigger**: an intrinsic call **or a bare let-bound identifier** appearing as a
**record-construct field value**. The `let` statement itself is fine (B2/B3
unused → frontend ok). `I64.literal(...)` as a field value already works (it is
a recognized name-qualified call); the 3+-segment `sico.text.*` / `sico.bytes.*`
intrinsics and bare identifiers as field values do not.

## 5. Root cause

`selfhost/parser.sico` :: `construct_field_value_ir`
(`parser.sico:11207`–`:11466`, failure points `:11231`, `:11277`, `:11331`,
`:11465`) only lowers these construct field-value shapes:

- a string token → `const_string` value IR, or
- a `name.field` projection.

Anything else — a fully-qualified `sico.*` intrinsic call, or a bare let-bound
identifier used as a field value — falls through to `return
"ERR:E-SH-IR-CONSTRUCT-VALUE"`. The host oracle lowers these; the selfhost
parser does not.

**Oracle contrast (repair design fact, grounded this slice)**: the host oracle
lowers construct field values through its **general expression resolver** —
`crates/sico-ir/src/lower.rs:3836-3841` calls
`self.expression(argument_value(argument), expected_field, output)` for each
field, so intrinsics, identifiers, literals and projections all flow through one
path. The selfhost `construct_field_value_ir` instead special-cases only string
tokens and `name.field` projections. The repair direction is therefore to route
the field value through a general expression-value resolver (or extend the
special-casing to `sico.*` calls and bare identifiers) rather than adding more
narrow cases. This is design grounding only — **not implemented here**.

**Secondary gate** (not hit by F1/F2/F3 because they bind a `let` first):
`selfhost/parser.sico:11494` :: `ERR:E-SH-IR-VARIANT-PAYLOAD` — a `run` body
that is a bare single `return ok(ScriptOutput(...))` with no preceding `let`/`if`
also trips a frontend variant-payload check. Worth noting for the eventual repair
but it is downstream of the construct-value gap for the current fixtures.

## 6. Consequence for G2/G3

- The **map.get / map.put / map.empty / text.trim** gaps recorded in
  `STEP-0613d`/`STEP-0613e` are **real emitter-arm gaps in the host oracle**, but
  they are **not the first wall** for a `run` guest: the frontend
  construct-field-value gap fires first and prevents constructing a fixture that
  would even reach those arms.
- Therefore fixing `construct_field_value_ir` to accept `sico.*` intrinsic calls
  and bare identifiers as construct field values is the **prerequisite** that
  unblocks building co-gap fixtures at all — and may, once fixtures can be
  constructed, expose the emitter arms for separate verification.

## 7. Masking hygiene (secondary finding)

`selfhost/compiler.sico:878`–`:882` collapses any `cch_ir` beginning with
`ERR:` into the generic `"unsupported codegen source shape"` under
`--emit-component-hex`. This is why the frontend code was invisible in every
prior driver run. Surfacing the specific `ERR:` code (or at least distinguishing
frontend vs emitter provenance) would have surfaced this gap immediately and is
recommended as a companion fix — otherwise the next frontend gap is masked the
same way.

## 8. Not done here / next

- The construct-field-value repair touches `selfhost/parser.sico`
  (`construct_field_value_ir`), which triggers the known coordination cost:
  the frozen match-site ledger (387 `parser.sico` sites in
  `selfhost/match-component-coverage-v1.json`) line numbers/digests shift, and
  re-freezing needs the census scripts (Python). **Correction (this slice):** an
  earlier draft of this record claimed Python was "not available on this host";
  that is **false** — Python 3.15.2 is on PATH via the WindowsApps alias and
  passes the exact `tools/ensure-python.ps1` usability criterion
  (`python -c "import os, signal, subprocess, sys"`), and
  `python tools/census-m22-match-sites.py --check` runs (reports `LEDGER_STALE`
  as expected). The ledger re-freeze is therefore **not** Python-blocked. Per the
  `STEP-0605` precedent the repair + re-freeze must still be ONE coordinated
  slice. It is a large, single-writer-hot-file change (owner-scoped per AGENTS
  §10.1.4) and is deliberately **not** started in this documentation slice; it is
  dispatched as the next coordinated slice.
- The masking-hygiene change (`compiler.sico:878-882`) was applied and verified in
  the follow-on slice **`STEP-0613g`** (not left as a recommendation).

## 9. Validation

- Probe run: `cargo test -p sico-runner --test selfhost_formatter_component_measurement -- --ignored --exact --nocapture m22_w3a_cogap_refusal_identity` — 1 passed (MSVC toolchain, `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-msvc`).
- `cargo fmt --check` (runner workspace): clean.
- `git diff --check`: clean.
- `frozen_match_site_ledger`: known-red, pre-existing, unrelated to this
  documentation-only slice (see `STEP-0613c` §6).

## 10. Files

- `runner/sico-runner/tests/selfhost_formatter_component_measurement.rs` — added `m22_w3a_cogap_refusal_identity` probe + `guest_refusal_identity` helper.
- `target/tmp/step0613f/cogap-refusal-identity.log` — probe evidence (generated).
- `docs/steps/STEP-0613f-m22-w3a-cogap-refusal-identity-frontend-construct-value.md` — this record.
