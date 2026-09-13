# STEP-0148: RFC-0039 A6 repair — checked-Result function seam

> - status: complete
> - phase: M15 prerequisite track (RFC-0039 A6 dedicated defect step)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - artifacts: [`tests/checked_result_seam.rs`](../../crates/sico-cli/tests/checked_result_seam.rs); updated `tests/wasm/artifacts.hex`

## 1. What was done

The STEP-0143 recorded defect — a script-profile user function returning
`Result[I64|U64, NumericError]` declared the full flat core width
[tag, ok, error] while its body emitted the packed two-slot value form,
producing invalid WebAssembly — is repaired by aligning both seams:

- `emit_return` (Direct/script path) widens the packed value form into
  the declared full flat width, zeroing the losing side.
- The call site re-packs the full flat width into the value form
  (error tag zero-extended from the joined error slot).
- `lower_result_type` (Direct) declares the same full flat width, so the
  M3-era Direct backend and the script profile now agree (the backend
  snapshot changed accordingly — append/patch discipline, justification:
  the A6 repair; the module now passes real Wasm validation where the
  frozen bytes encoded the defect).
- The STEP-0143 typed refusal is removed; the corpus replaces it:
  `checked_result_seam.rs` runs an imported checked-Result function
  through the real runner across both the ok and the NumericError path
  (`ok-42|overflow`, byte-exact); the modules_command fixture flipped
  from build-refusal to build-success.

## 2. Validation

- `checked_result_seam` (1, byte-exact through the real runner, both
  result paths); `modules_command` 14/14; `backend` 13/13 including the
  updated deterministic snapshots; full workspace + runner suites green.
