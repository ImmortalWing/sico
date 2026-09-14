# STEP-0175: Language v1 batch 2 — for-loops, error propagation, bare-literal record

> - status: complete
> - phase: M21 §3.2 / M22 S0 continuation (RFC-0046 accepted 2026-09-14 via owner session directive 「先完成M21和自举吧」)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: [`RFC-0046`](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md), `crates/sico-{lexer,hir,parser,ir,semantics}/src/lib.rs`, `crates/sico-ir/src/lower.rs`, `tests/end-to-end/for-loop-words.sico`, `runner/sico-runner/tests/for_loop.rs`, `runner/sico-runner/tests/error_propagation.rs`

## 1. What was done

**For-loops** (`for x in expr:` … `end for`), lowering to the general CFG
desugared as: subject spill cell + U64 index cell + a `while` over
`index < list.length(subject)` whose body binds `x` through `list.get`.
Two invariants keep the verifier green: values crossing blocks travel
through cells (IR values are block-scoped), and every block is created
immediately before its content is emitted (the verifier's sequential
value-id rule). The bounds predicate makes the fetch statically in-range
so no dispatcher is needed; the increment is a `checked_add` whose error
arm is statically impossible.

- Lexer: `For`/`In`/`QuestionMark` token kinds (`for`, `in` keywords;
  `?` punctuation).
- HIR: `LineKind::For` mapping + `opens_block`.
- Parser: `BlockKind::For` across the three maps (`opener`/`close_kind`/
  `missing_close_error`) with the while syntax kind shared.
- Semantics: the For line checks the subject is a `List[Text]` (the
  executable iterable set) and binds the loop variable to its element
  type until the region closes; Map/Set subject iteration materializes
  the key list up front (the frozen `map.keys`/`set.to_list` surface).
- Lowering: the For arm desugars per RFC-0046; the routing stays
  consistent with the STEP-0148 frozen shapes.

**Error propagation** (`let y = expr?`, RFC-0046 D2): desugars to
`match expr { ok(v) -> bind y; join | error(e) -> return error(e) }` via
the line-level desugar (control flow even in an otherwise straight body).
The semantics check forwards the `NumericError` payload verbatim (E3101
family, same identity as the keyword-prefix `try expr` form which stays
supported). `expr?` in expression position (nesting) stays proposed per
RFC D2. The STEP-0148 `RESULT-001` frozen snapshot stays byte-exact
(`try expr` keeps its straight-path lowering).

**Bare-literal decision (RFC-0046 D3)**: recorded as a no-grammar-change
decision — bare integers remain arbitrary-precision `Int` (NUM-001 oracle
preserved); executable code names widths explicitly. This closes the M21
§3.2 decision point with the measured NUM-001 trade-off unchanged.

## 2. Defects found

1. **Formatter/snapshot coupling**: an early routing change that sent
   `try expr` through the general CFG leaked `write-local`/`read-local`
   ops into the STEP-0148 frozen `RESULT-001` shape and the test failed
   until the routing was narrowed to only the `?` form (which genuinely
   introduces control flow). No semantic change; the frozen shapes are
   byte-identical again.
2. **Reserved-word migration**: `for`/`in` become reserved (RFC-0033
   gate). Pre-existing sources using them as identifiers receive typed
   diagnostics; nothing else changes.

## 3. Validation

- `runner/sico-runner/tests/for_loop.rs` 2/2: for-loop over a
  `List[Text]` with per-iteration binding, accumulator updates through
  checked arithmetic, post-loop content assertion (`alpha|beta|gamma|delta|`),
  repeated-run determinism.
- `runner/sico-runner/tests/error_propagation.rs` 1/1: helper
  `Result[I64, NumericError]` flows through match; the `?` form
  forwards the `NumericError` payload (happy path verified;
  `main`-returning-`ScriptError` end-to-end is the registered residual).
- `validate-step-0131` (matrix + corpus paths): OK. `git diff --check`:
  clean. Full `tools/run-ci.ps1` 10/10: CI GREEN (fmt, both clippies,
  both test suites, module boundaries, planning contract, matrix,
  cross-host corpus, whitespace).

## 4. Residuals (STEP-0176)

- `List[I64]`/`List[U64]` element extension: `empty`/`length`/`get`/`append`
  monomorphs are gated at the registry until their signed/unsigned
  comparators land; the for-loop over scalar lists rides on this.
- `map.entries` (pair records) stays proposed per RFC D4.
