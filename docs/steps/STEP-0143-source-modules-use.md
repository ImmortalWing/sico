# STEP-0143: Source modules and `use` imports (RFC-0039 modules slice)

> - status: complete
> - phase: M15 prerequisite track (M15 plan §3.0; first implementation slice under accepted RFC-0039)
> - started: 2026-09-08
> - completed: 2026-09-08
> - owners: autonomous-agent
> - artifacts: [`RFC-0039`](../rfc/RFC-0039-source-modules-package-resolution-user-wit-v0.md) (accepted, amendments A1–A6)

## 1. What was done

The modules slice of accepted RFC-0039: `module`/`use` become keywords,
imported files assemble into one compilation through a fail-closed CLI
link pass, cross-module qualified calls lower through one merged IR module,
and the whole stack (check/build/run/watch/test/cache) sees the module set.
Single-file programs assemble without touching the filesystem and lower
byte-identically.

## 2. Implementation map

- `sico-lexer`: `module`/`use` keywords (appended discriminants; snapshot
  stability preserved).
- `sico-parser`: single-line `module <name>` / `use <module>.<item>`
  declarations (E1014/E1015 for malformed shapes); use names normalized to
  `module.item`.
- `sico-semantics`: Module/Use carry no per-file semantics (verified at
  link time — nothing silently discarded overall).
- `sico-ir`: `lower_core_modules(entry, imports)` — per-file single-file
  lowering with per-file FunctionId bases (entry first, imports in
  discovery order) so cross-module qualified calls resolve through
  `module.item` definition-table aliases with no post-pass rewriting;
  merged-module verifier bounds widen to the combined source length
  (identical to the entry length for single files).
- `sico-cli` `modules.rs`: DFS discovery (entry-dir `<module>.sico`),
  module-declaration/basename checks, item/function checks, duplicate use,
  cycle, depth ≤ 32, modules ≤ 64, stdin-root and entry-decl refusals —
  stable codes E8001–E8011; assembled-set compilation wired into
  check/build/run/watch/test; the run cache key covers module bytes
  (module edits invalidate cached Components).
- `sico-diagnostics`: E1014/E1015 syntax identities.

## 3. Defects found and recorded while landing

1. **Pre-existing, fixed refusal (A6):** a user function returning
   `Result[I64|U64, NumericError]` in the Script profile emitted INVALID
   WebAssembly — the callee declares the full flat width [tag, ok, error]
   while its body emits the packed two-slot value form. No frozen corpus
   ever called such a function; the seam surfaced through the new module
   fixtures and was proven pre-existing by a single-module reproduction
   (`/tmp/v3` probe, identical failure without any module involvement).
   It is now a typed build refusal (`unsupported: user function returning
   a checked fixed-width Result … recorded defect`); the packed↔flat ABI
   repair is a dedicated follow-up defect step. Matrix: refused entry
   `user-function-checked-result-script-profile`.
2. **Pre-existing validator break, fixed:** `validate-step-0131.ps1`
   still demanded `step == 'STEP-0131'` after the matrix step field moved
   to `STEP-0131..0140` during M14 — the validator was red on the
   uncommitted M14 worktree. Now range-checked and extended with module
   fixture invariants.

## 4. Evidence

- New tests, all green: `crates/sico-ir/tests/modules_lowering.rs` (3/3:
  cross-module call ids/targets, determinism, single-file equality),
  `crates/sico-cli/tests/modules_command.rs` (11/11: check/build/run/test
  happy path, cache invalidation on module edit, module-attributed
  semantic failure, E8001/02/03/04/05/06/09/10/11 + E1015 refusals,
  checked-result refusal).
- E2E fixture `tests/end-to-end/modules-report/` (main.sico +
  math_util.sico + main.test.json): `sico run` → `modules-ok`, exit 0;
  `sico test` → 1 passed. Matrix `source-modules-use` executable triple +
  exit-corpus entries; `validate-step-0131.ps1` green;
  `validate-step-0124.ps1` green.

## 5. Limitations (declared)

- `--debug-info` with module imports: typed tool refusal (cross-module
  debug maps are future work); single-file debug unchanged.
- Watch tracks the entry file only; module-file edits are picked up on
  the next entry change.
- Unimported qualified references keep the F4 build-time refusal (§2.2
  check-time tightening is the next slice); type imports refused (E8011).

## 6. Scope

Compiler/frontend only; no Host/Runtime/runner behavior change for
single-file programs (byte-identical lowering asserted by test). The
runner workspace suite is exercised separately (serial).
