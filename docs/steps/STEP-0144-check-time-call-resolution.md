# STEP-0144: Check-time call-target resolution (RFC-0039 §2.2)

> - status: complete
> - phase: M15 prerequisite track (second implementation slice under accepted RFC-0039)
> - started: 2026-09-08
> - completed: 2026-09-08
> - owners: autonomous-agent
> - artifacts: [`RFC-0039`](../rfc/RFC-0039-source-modules-package-resolution-user-wit-v0.md) §2.2

## 1. What was done

The declared check-surface tightening of RFC-0039 §2.2: every call target
must resolve at check time — local, imported (qualified), an exposed
intrinsic, a constructor, or a variant — and an unbound callee is now the
typed diagnostic `E2031 UNRESOLVED_CALL_TARGET` with the callee identity,
instead of a silent `Unknown` that deferred to the build backend's
"unsupported call target" refusal (the STEP-0142 finding F4).

## 2. Implementation map

- `sico-semantics`:
  - `Model.imported_functions` — the analyzed file's qualified
    `module.item` import surface; consulted after local functions.
  - `exported_functions(source)` — one module file's exported top-level
    function signatures for cross-module resolution.
  - `analyze_with_modules(source, modules)` — analysis with the import
    surface injected (`analyze` = empty surface; LSP/index/AI tools keep
    their per-file behavior).
  - `infer_call` — imported qualified calls type-check their arguments
    against the exported signature; the resolution fallback emits E2031.
  - Match scrutinees are now inferred like every other expression (the
    STEP-0142 probe's blind spot: If/While conditions were inferred but
    match scrutinees never were, so scrutinee callees escaped check).
- `sico-ir` (`lower_core_modules`): each file's semantic gate runs
  `analyze_with_modules` with the import surface (every import's exported
  functions), so the lowering gate and the check pass agree exactly — the
  CLI's "semantic gate failed unexpectedly" guard stays an internal-
  consistency backstop, not a user path.
- `sico-cli` (`modules.rs::analyze_set`): check/build/run/test analyze the
  assembled set once, entry first, imports in discovery order, each with
  its own direct-import surface; diagnostics render with the owning file's
  name.

## 3. Behavior changes (declared)

- Undefined bare calls (`missing(1)`) and unimported qualified references
  (`math_util.twice` without a `use` edge) now fail at check with E2031;
  previously they passed check and failed at build. Valid programs are
  unaffected — the class of programs that changed behavior is exactly the
  one that could only fail later.
- Imported calls type-check arguments at check time against the module's
  exported signature (E2001 for mismatches), with the diagnostic
  attributed to the calling file.
- Match scrutinees participate in check-time resolution for the first
  time.

## 4. Evidence

- `crates/sico-cli/tests/modules_command.rs` 14/14: the three new §2.2
  tests (bare-call E2031, unimported-qualified E2031, cross-module
  argument type mismatch E2001) plus the pre-existing module suite.
- Root workspace `--all-targets --all-features` green; runner workspace
  (serial) green; fmt + clippy `-D warnings` clean.
- Matrix updated: `unresolved-call-target` refusal entry; the
  `http2-streaming-upload-from-source` entry's refusal now lands at check
  (E2031); STEP-0143's "unimported qualified reference keeps the F4
  build-time refusal" limitation is superseded by this slice.

## 5. Limitations (declared)

- The import surface passes a module's whole exported function set once
  any `use` edge exists (per-item strictness is future tightening).
- LSP/semantic-index analysis stays single-file; cross-module resolution
  is a CLI compilation-set behavior by design for this slice.
