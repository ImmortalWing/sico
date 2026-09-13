# End-to-end fixtures: two declared program shapes

The directory holds both executable profiles of the source language.
They are not interchangeable, and `sico build` / `sico run` accept
different classes by design (RFC-0038 application profiles; RFC-0044
script composition):

- **Scalar-profile build fixtures** (`main()` with a scalar return):
  `answer.sico`, `truth.sico`, `unit.sico`. `sico build` (default
  profile) accepts them and emits deterministic components; they are
  the backend determinism/debug-map fixtures. `sico run` and
  `sico build --profile script-v0` refuse them with the typed
  script-profile refusal — asserted by `crates/sico-cli/tests/cli.rs`
  (`build_emits_a_deterministic_component`, the script-v0 refusal
  case), not by accident.
- **Script-profile fixtures** (`record ScriptInput` +
  `main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]`):
  everything else, including `checked-mul-div.sico`,
  `bit-ops-bitboard.sico` and the `script-*.sico` family. These run
  through `sico run` / `sico-runner` and are the executable e2e corpus.

`answer.sico` (`return 40 + 2`) is kept in its scalar shape on
purpose: it exercises the scalar build/backend path that the script
profile does not cover. Updating it to the script shape would delete
that coverage; a modern script-shape "answer" already exists in the
release smoke (`checked-add-script.sico`, `tools/release-windows.ps1`).
