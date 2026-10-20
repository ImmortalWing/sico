# STEP-0294: M22 W2 S3/S4 — Map[Text,U64] get as a match subject

> - status: complete / 22-C S2 GO on the isolated branch after independent CI
> - phase: M22 compiler self-host, execution card 22-C S2
> - date: 2026-09-25
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and scope

STEP-0293's repaired `cb371c5` passed independent Windows GNU CI, giving
22-C S1 GO on the isolated `codex/m22-w1-ci` branch. The starting formatter
canary was 22/30, `nearest_match` / `ERR:E-SH-IR-CALL-TARGET`.

This S2 slice lowers a bounded `sico.map.get[Text,U64]` match subject in the
existing general-while machine. Its declared shape is a `Map[Text,U64]`
parameter and a single Text key atom (literal, parameter or local). The
subject emits the canonical `Result[U64, NumericError]` intrinsic, and the
`ok` projection and local carry `U64`. The `bytes.slice` match path retains
its prior `Bytes` result shape. Nested key expressions, local Map operands
and match-arm control-flow expansion are outside this slice and remain typed
refusals.

The implementation exposed a pre-existing malformed Map type string in three
parameter/signature lookup helpers: the outer JSON object lacked its closing
brace. All three now emit the canonical complete type. No source-language,
WIT, Host or authority contract changes.

## Local exit evidence

- The new real-runner differential compares literal, parameter and local Text-key
  `map.get` match subjects with the Rust oracle byte for byte. Extra argument
  and extra generic type (limit+1) mutations refuse with
  `ERR:E-SH-IR-CALL-SHAPE`; wrong Map value and key types refuse with
  `ERR:E-SH-IR-CALL-TYPE`. A valid input after the refusal sequence verifies
  recovery in a fresh Store.
- `tools/report-m22-canary.ps1`: **22/30**, first uncovered function
  `nearest_match`, frontier **`ERR:E-SH-IR-STATEMENT`**, full-source exit 122.
  The refusal moves from `CALL-TARGET` to the match-arm control-flow layer;
  byte-exact coverage remains 22/30.
- `tools/validate-step-0262.ps1`: compiler 23/23, parser 2/2, local bounds
  1/1, 22-function IR snapshot and current frontier passed. The busiest
  function remains `general_while_function_ir` at **240 locals** (the first
  draft reached 241 and failed the headroom test; its extra local was removed
  before this full rerun).
- `tools/validate-step-0261.ps1`: compiler 23/23, parser 2/2, local bounds
  1/1, 17-function historical IR snapshot and current frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 23 and runner clippy passed.

## Independent CI and gate accounting

The local shape exit passed and the typed frontier moved, so the R3
consecutive-stall count remains **0**. The final implementation and
literal/parameter/local key differential were pushed as
`5a6d0bbad59c9d014aab2cf5612b39b02e408fbd`. [GitHub Actions run
36147743130](https://github.com/ImmortalWing/sico/actions/runs/36147743130)
completed **success** on a fresh Windows GNU runner at 2026-09-25 14:47:10
UTC. Its `Run repository CI on a fresh runner` step succeeded. The earlier
`5a802ff` run also succeeded but did not include the parameter-key assertion
and is not used for the final verdict.

**22-C S2 GO on the isolated branch.** M22 remains NO-GO. STEP-0291's S3
control-flow slice is next: nested `if` and `return`
inside the `ok` arm of `nearest_match`, with byte-exact prefix differential.
The parallel `origin/dev` W1/W2 history remains unmerged; branch CI does not
adjudicate that history.
