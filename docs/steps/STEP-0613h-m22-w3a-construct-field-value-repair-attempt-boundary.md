# STEP-0613h — M22 W3-a: `construct_field_value_ir` repair attempt — substantial design landed, then reverted at a clean boundary

- Date: 2026-11-10
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (dispatched sub-agent; coordinator recovery)
- Status: **NOT landed** — attempted, substantial design written, reverted to clean state. Boundary recorded for a fresh session.
- Evidence level: internal-fixture (design + partial runtime verification; not a green gate)

## 1. Why this slice

`STEP-0613f` established the real co-gap blocker is the **frontend**
`ERR:E-SH-IR-CONSTRUCT-VALUE` from `selfhost/parser.sico ::
construct_field_value_ir` (not an emitter arm). Python was confirmed available
(this slice's precursor finding), unblocking the census re-freeze that gates the
repair. This slice attempted the repair + ledger re-freeze as ONE coordinated
slice (STEP-0605 precedent).

**Outcome: the repair design was substantially written (434 lines) but is NOT
landed.** The worktree was restored to the clean pre-attempt state. Honest
boundary record; no gate flipped.

## 2. Repair design (grounded, preserved for reuse)

The dispatched sub-agent produced a complete, coherent design and wrote it into
`parser.sico` before the attempt was halted. The design (preserved verbatim at
`/tmp/parser.sico.broken-step0613h`, 434 insertions):

- **New helper `construct_intrinsic_parts`** — lowers a `sico.*` intrinsic call
  appearing as a construct field value (the 3+-segment case that `I64.literal`
  already handled for the 2-segment case).
- **`construct_field_call_count` / `construct_field_call_ir`** — extended with an
  intrinsic branch so a `sico.*` callee (not just a user-declared function) is
  recognized as a call-shaped field value. Previously these resolved the callee
  only via `function_declaration_index`, so host intrinsics fell through to
  `ERR:E-SH-IR-CONSTRUCT-VALUE`.
- **`construct_local_index` / `construct_local_kind` / `construct_expression_kind`
  / `construct_expression_parts`** — support for a **bare let-bound identifier**
  as a construct field value (the second confirmed gap shape from the STEP-0613f
  bisect, case B4).

This matches the design grounding in `STEP-0613f` §5 (oracle lowers construct
field values through a general expression resolver, `lower.rs:3836-3841`).

## 3. What blocked landing it (honest)

The Rust build compiled (`BUILD_EXIT=0`), but the selfhost `.sico` is compiled at
**runtime** by the probe, which surfaced a cascade of runtime `.sico` errors in
the new code:

1. `unsupported cell redeclared` — Sico cells are function-scoped, so
   sibling-branch `let`s of the same name collide. The sub-agent **fixed** these
   (`let`→`set`) across the three new functions.
2. `unsupported stdlib intrinsic arity` — one of the new `sico.*` calls in the
   inserted code had a wrong argument count. The sub-agent was locating the exact
   call when it ran out of budget.

Two process errors made this worse, both the coordinator's responsibility:

- The coordinator **killed the sub-agent mid-edit** (mistaking 30 min of deep
  design investigation for being stuck) while it was actively landing the fix,
  leaving `parser.sico` in a non-building intermediate state (call sites not yet
  updated to the new signatures).
- A recovery resume then died on a max-tokens error mid-cascade.

With the session turn budget nearly spent and the error cascade not yet closed,
the disciplined choice was to **restore the clean state** rather than leave a
half-broken gate or gamble the last turns.

## 4. Recovery (verified)

- `parser.sico` restored from the clean pre-attempt backup
  (`/tmp/sico_triage_backup/parser.sico.step0613`, sha `2649984bb09380ea…`), which
  preserves the STEP-0613c span fix (`gw_ok_wrap_rhs_packed`) and the original
  4-arg `construct_field_call_count`. `construct_intrinsic_parts` is gone (grep 0).
- Rebuild: `BUILD_EXIT=0`.
- Pins re-verified green after restore: `sico_run_emits_the_minimal_multiblock_
  run_component_byte_exactly`, `sico_is_utf8_gated_run_lowers_byte_exactly`,
  `sico_compiler_emits_the_run_instruction_segment_byte_exactly`,
  `sico_codegen_guest_emits_component_bytes_byte_exactly` (4 passed).
- `git diff --check`: clean. HEAD unchanged `e93f59a`. Nothing committed/pushed.
- The 434-line design is preserved at `/tmp/parser.sico.broken-step0613h` for a
  fresh session.

## 5. Next (fresh session, well-budgeted)

The repair is **design-complete but verification-incomplete**. A fresh session
should:

1. Re-apply the preserved 434-line design (or re-derive from §2).
2. Pre-empt the two known runtime error classes **before** probing: (a) no
   sibling-branch duplicate `let` names (cells are function-scoped); (b) verify
   every `sico.*` call's arity against the stdlib signature.
3. Fix the intrinsic-arity error the sub-agent was locating.
4. Probe `m22_w3a_cogap_refusal_identity --ignored` → F1/F2/F3 must become
   `frontend: ok`.
5. Ledger re-freeze (`census-m22-match-sites.py --update` + emitter-vocabulary
   `--update`), then keep the 7 G1 pins + fail-closed pin green.

This is a large, single-writer-hot-file slice (`parser.sico`, 387 ledger sites)
and should be one coordinated slice with the ledger re-freeze, given a full
session budget.

## 6. Validation

- Restore build: `BUILD_EXIT=0`.
- 4 key pins green post-restore (multiblock run / `is_utf8` gate / echo
  instruction segment / fail-closed `IDENTITY_SOURCE`).
- `cargo fmt --check` (runner workspace): clean. `git diff --check`: clean.
- **No gate flipped by this slice.** The construct repair is NOT landed. The
  probe still reports F1/F2/F3 as `frontend: REFUSED … ERR:E-SH-IR-CONSTRUCT-VALUE`
  (unchanged from STEP-0613f, since the repair was reverted).

## 7. Files

- `selfhost/parser.sico` — restored to clean `2649984b` (no net change from the
  pre-attempt state).
- `/tmp/parser.sico.broken-step0613h` — preserved 434-line design (NOT in repo).
- `docs/steps/STEP-0613h-m22-w3a-construct-field-value-repair-attempt-boundary.md` — this record.
- `docs/steps/README.md` — STEP index row.
