# STEP-0195: M22 S6 — exact parameter arity (`fn:add/2`)

> - status: complete — **the arity walk now counts the full parameter
>   list; `function add(a: I64, b: I64)` yields `fn:add/2`, the exact
>   IR-signature arity**
> - phase: M22 S6 (lower slice, continued from STEP-0194)
> - completed: 2026-09-16
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. What was done

STEP-0194's residual was fixed: the parameter count is now exact for the
declared shape. The rewritten `count_params` walks the word region between
the function name and `returns` and counts completed `name: Type` word
pairs, so `function main()` yields `fn:main/0` (unchanged) and
`function add(a: I64, b: I64)` yields `fn:add/2`.

Mechanism correction recorded per honesty rules: STEP-0194 described the
walk as comma-counting, but `lex_words` drops all punctuation — the comma
and `(` probes could never fire and the count was always zero. The arity
therefore derives from the punctuation-free word stream, where each
parameter contributes exactly two words (name, type). A parameter whose
type is itself generic (`xs: List[Text]`) contributes three words and is
**outside this slice's declared shape** — registered here, not silently
approximated; the full typed IR slice will carry parameter types as
structured data and removes this assumption.

Also fixed in the same change: `selfhost_parser.rs` gave both tests the
same `compile_parser()` temp directory (PID-keyed), so parallel test
threads raced on `create_dir`/`remove_dir_all`; the directory is now
tagged per test. Both tests pass under the default parallel harness.

## 2. Validation

- `selfhost_parser.rs` 2/2 (`fn:main/0`, `fn:add/2`), byte-exact, via the
  real runner (component built by `sico build --profile script-v0`).
- Full runner workspace: all suites green; the two `runner.rs` budget
  asserts (RSS-growth, warm-median) failed once under the parallel
  harness and pass serially — the known environment flake, unrelated to
  this change.
- run-ci: see STATUS.md for the recorded result.

## 3. Residuals

- The full typed IR (expression trees, blocks, parameter types as
  structured data) and the Rust-verifier differential remain the S6
  slices toward the bootstrap closure.
- Generic-typed parameters are uncounted by the current word-pair arity
  walk; resolved by the structured-IR slice above, not by another
  word-stream heuristic.
