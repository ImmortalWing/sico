# STEP-0254: M22 differential harness PreparedProgram reuse

> - status: complete / validation infrastructure; M22 NO-GO unchanged
> - phase: M22 support (self-host differential iteration cost)
> - completed: 2026-09-22
> - evidence: internal-fixture, Windows x64 real runner (pinned rustc 1.98.0 MSVC host, matching this machine's cached runner artifacts), serial in-process measurement

## Objective

Remove the dominant wall-clock cost of the `selfhost_compiler` differential
suite: every fixture was rebuilding a `Runner` and re-linking the same
compiler Component. Reuse one native compilation in-process while proving
that no guest execution state can leak between fixtures.

## Changes

- `run_guest_with_args` now holds the prepared compiler in a process-wide
  `OnceLock<Mutex<PreparedProgram>>`. Only the native compilation
  (Component link) is shared; `PreparedProgram::run` still allocates a fresh
  Store, fresh resources and fresh fuel for every fixture.
- Runs are serialized through the mutex because timeout watchdogs advance
  the shared Engine's epoch; concurrent watchdogs on one Engine could
  otherwise poison each other's deadlines.
- Per-fixture limits are unchanged: fuel 5,000,000,000, 30 s timeout,
  512 MiB memory, and byte-exact comparison against the Rust oracle.
- `sico_compiler_refuses_an_invalid_parameter_shape_with_typed_identity`
  gained a recovery assertion: after the typed
  `ERR:E-SH-IR-PARAMETER-TYPE` domain refusal, the same prepared compiler
  compiles the valid identity source byte-exactly in a fresh Store. This
  pins that reuse cannot surface residual guest state after a refusal.
- STATUS, ROADMAP, the M22 plan evidence note and the steps index already
  carry this record; this document closes the previously dangling index
  link and adds the dedicated validator.

## Executed validation

- Full `selfhost_compiler` suite: 13/13 green through the real runner,
  including the refusal-recovery assertion.
- Serial suite wall-clock: the 66.99 s figure recorded with the change
  (versus 801.46 s for the same 13 tests immediately before, the
  STEP-0253 baseline, roughly one twelfth) was measured in the authoring
  session; the clean re-run on this machine measured 79.60 s for the 13
  tests plus a one-time 37.85 s rebuild of the edited test crate. This is
  test-harness overhead reduction only; it is not guest self-compilation
  performance evidence and does not enter any M22 exit gate.
- `tools/validate-step-0254.ps1` and `git diff --check` pass.

## Honest residuals

- Language support boundary is unchanged: the formatter canary still stops
  at `item`'s `sico.list.get` result match with the typed
  `E-SH-IR-CALL-TARGET` refusal (exit 122); no new lowering is claimed.
- Toolchain environment finding, recorded not fixed here: this host can no
  longer complete the documented GNU flow — `ring` needs a real gcc (the
  rustup self-contained gcc is linker-only) and the GNU link path wants
  dlltool from `target/tooling/msys2-binutils`, which was removed from
  this machine after 2026-09-20. The cached runner artifacts (and this
  validation) were built with the pinned `1.98.0-x86_64-pc-windows-msvc`
  toolchain, whose cc/link auto-discovery works via the installed
  VS 2022. Re-provisioning msys2-binutils (and a C compiler) or repairing
  the toolchain assumption in the validators remains open for the GNU
  evidence path.
- S5 general deterministic Component codegen, S6 `A == B == C` bootstrap
  and the M22 exit audit remain open; this step does not advance them.
- `compiler_entry_uses_lossless_module_lexer_on_its_own_sources` still
  prepares its own program instance; only the shared differential path is
  reused, which is the measured cost.
