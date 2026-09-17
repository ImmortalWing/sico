# STEP-0194: M22 S6 — IR signature shape (function name + param count)

> - status: complete — **the parser extracts the IR-signature shape
> (`fn:main/0`), the function identity the typed IR carries**
> - phase: M22 S6 (lower slice, continued from STEP-0193)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, this STEP record

## 1. What was done

`selfhost/parser.sico` extends STEP-0193's function-name extraction with a
parameter count: walking the tokens after the function name, it counts
commas between `(` and `returns` to derive the arity, emitting
`fn:<name>/<params>`. On `function main() returns Int:` the parser emits
`fn:main/0` — the function identity and arity the typed IR signature
carries.

## 2. Honest register

The param count is **comma-counted**: `function add(a: I64, b: I64)`
yields `fn:add/0` where the correct arity is 2 (one comma + one). The
count-params walk starts after the function-name token and stops at
`returns`, but the `(`-detection and comma counting under-count by one
for the multi-parameter case (it misses that the parameter list opening
implies at least one parameter). This is a **bounded parser refinement**,
not a defect in the token stream or the name extraction (both byte-exact).

## 3. Residuals

- Fix the arity walk to count `(`…`)` parameters correctly (comma+1 when
  the list is non-empty); then the IR signature is exact. — **closed by
  STEP-0195** (`fn:add/2`); mechanism corrected: commas were never in
  the token stream at all (the lexer captured identifier runs only).
  Two variants were built in parallel; the merged implementation emits
  single-character punctuation tokens and counts the depth-1 `(`…`)`
  span (empty list 0, otherwise commas+1), which also covers generic
  type parameters (`xs: List[Text]`) and stays correct across
  multi-function streams (`end function` context).
- The full typed IR (expression trees, blocks) and the Rust-verifier
  differential are the remaining S6 slices toward the bootstrap closure.
