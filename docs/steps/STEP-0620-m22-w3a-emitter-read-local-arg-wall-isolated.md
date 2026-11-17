# STEP-0620 — M22 W3-a: the next emitter wall isolated — `read_local` of a let-bound value used as an intrinsic argument

- Date: 2026-11-17
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **finding recorded; emitter arm NOT landed** — precise single-shape
  isolation, then honest boundary at a clean commit. No gate moved.
- Evidence level: internal-fixture (deterministic selfhost probe `--ignored`,
  single-variable isolation ladder)

## 1. Why this slice

`STEP-0619` closed the frontend construct-field-value gap, so the co-gap
fixtures (F1/F2/F3/B4) now reach the emitter and meet `ERR:E-SH-CG-IR-EMIT` —
the emitter-arm consequence STEP-0613f §6 predicted. This slice isolates **which
shape** the emitter refuses, to pre-cut the next knife.

## 2. Isolation ladder (single-variable, deterministic)

A temporary probe built four `run` fixtures that differ in one variable each and
fed each canonical IR to the refusal driver (`component_refusal_for_ir`):

| fixture | emitter |
|---------|---------|
| `encode(literal)` — `sico.text.encode("hi")`, no `let` | **no-refusal** |
| `utf8_decode` only — `let d = …utf8_decode(…)`, `d` **unused** | **no-refusal** |
| `encode(let utf8_decode)` — `let d = …`; `encode(d)` | **`E-SH-CG-IR-EMIT`** |
| `concat` — `let t = …concat("a","b")`; `encode(t)` | **`E-SH-CG-IR-EMIT`** |

**Conclusion:** the gap is **not** a missing intrinsic arm — `cgs_intrinsic_hex`
already dispatches `sico.text.encode` / `sico.bytes.utf8_decode` (both to
`cgs_pair_passthrough_hex`, `codegen.sico:4025-4029`) and `sico.text.concat`
(`:3885`). The refused shape is specifically **a `read_local` of a let-bound
value consumed as an intrinsic argument**: the `let` statement itself lowers
(the unused case is fine), and the intrinsic handler works on a literal
argument (the `encode(literal)` case is fine) — the failure is the
combination. This is the host-call-in-a-`let`-binding line STEP-0515 recorded,
newly exposed by the STEP-0619 construct closure.

## 3. The suspected mechanism (design grounding, not yet confirmed)

`cgs_pair_passthrough_hex` (`codegen.sico:6013-6052`) resolves the argument
through `cgs_value_slot(vindex, arg)` and `cgs_value_ty(vindex, arg)`, requires
`cgs_type_slots(arg_ty) == 2` (a (pointer,length) pair) and
`cgs_type_slots(result_ty) == 2`, then emits `local.get (slot-1)`, `local.set`,
`local.get slot`, `local.set`. A `const_string` argument satisfies this (it
produces a 2-slot pair). A `read_local` argument resolves through the same
vindex, and one of those lookups / the 2-slot requirement fails for the
let-bound value — that is the shape to confirm and repair.

## 4. What blocked landing it (honest)

Two diagnostic attempts were masked by the refusal-collapsing layer:
`component_refusal_for_ir` (`codegen.sico:490-499`) turns an empty
`cg_component_hex` result into the generic `E-SH-CG-IR-EMIT`, so tagged
diagnostics planted in `cgs_pair_passthrough_hex` and in `cg_core_module_hex`'s
empty-return points did **not** surface — the rejection happens upstream of (or
orthogonally to) those points and is collapsed before it reaches the probe.
Each attempt costs a full refusal-driver-component rebuild. With the budget
spent on the STEP-0619 closure, the disciplined choice was to revert the
temporary diagnostics and record the isolation at a clean commit rather than
leave a half-instrumented tree.

**Recovery (verified):** `selfhost/codegen.sico` and the test file restored to
HEAD `84060c1`; `git status` clean; no temporary probes or diagnostics remain.

## 5. Next (fresh session, well-budgeted)

1. Instrument at the layer that actually rejects — either the run-body
   instruction walk (before `cg_component_hex` collapses) or by making
   `component_refusal_for_ir` surface the inner `ERR:` instead of collapsing an
   empty core (the masking-hygiene companion STEP-0613f §7 already recommends).
2. Confirm whether `cgs_value_slot` / `cgs_value_ty` resolve a `read_local`
   result id for a let-bound value, and whether the 2-slot operand requirement
   is the failing check.
3. Land the `read_local`-as-intrinsic-argument arm, byte-equal to the Rust
   oracle, and verify F1/F2/F3/B4 emit (not just reach the emitter).
4. Re-freeze the `match`-site ledger + emitter-vocabulary census (the
   STEP-0619 residual, independent ~6000 s measurement axis).

## 6. Gate discipline

**No gate flipped.** M22 overall remains **NO-GO** (S3/S4/S5 partial, S6 not
entered, S7 NO-GO per STATUS/ROADMAP). Internal-fixture evidence only.

## 7. Files

- `docs/steps/STEP-0620-m22-w3a-emitter-read-local-arg-wall-isolated.md` — this record.
- `docs/steps/README.md` — STEP index row.
- Committed base: `84060c1` (STEP-0619 construct closure); this record is the
  only new change.
