# STEP-0191: token-emission defect RESOLVED — root cause was an application bug, not the compiler

> - status: complete — **the "Int"→"nt" defect is fixed; root cause was an
> application-layer `is_ident_byte` bug (uppercase misclassified), NOT a
> compiler-lowering defect. STEP-0185–0190's lowering hypothesis is
> retracted**
> - phase: M22 S4 resolution (corrects STEP-0185–0190)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/tokens.sico`, `runner/sico-runner/tests/selfhost_tokens.rs`, this STEP record

## 1. The actual root cause

STEP-0185 through STEP-0190 progressively isolated the token-emission
defect (`Int` → `nt`) and concluded it was a compiler-lowering bug in the
while-loop cell scheduling. **That conclusion was wrong.** The actual root
cause is an application-layer bug in `is_ident_byte`:

```sico
# BUGGY (STEP-0183):
if I64.less_than(b, I64.literal(97)): return false   # 'a' = 97
if I64.less_than(b, I64.literal(123)): return true    # 'z' = 122
if I64.less_than(b, I64.literal(65)): return false    # 'A' = 65  (unreachable!)
if I64.less_than(b, I64.literal(91)): return true     # 'Z' = 90  (unreachable!)
```

Uppercase letters (65–90) are **below 97**, so the first guard `b < 97`
returns `false` for every uppercase letter. The uppercase checks below it
are **dead code**. The token `'Int'` starts with `'I'` = 73, which
`is_ident_byte` misclassified as non-identifier — so the `Int` run never
started, `word_start` was never set to 20, and the slice fell through to
the next token's coordinates (25, 2 = `nt`).

The fix reorders the guards so uppercase is checked before the lowercase
range:

```sico
# FIXED:
if I64.less_than(b, 65): return false    # below 'A'
if I64.less_than(b, 91): return true     # 'A'..'Z'
if I64.less_than(b, 97): return false    # between 'Z' and 'a'
if I64.less_than(b, 123): return true    # 'a'..'z'
```

## 2. Result

`selfhost/tokens.sico` on the probe source now emits
`function,main,returns,Int,return,end,function` — **all seven tokens
byte-exact**. The e2e (`selfhost_tokens.rs`) asserts the full byte-exact
stream.

## 3. Why STEP-0185–0190 were misled

The defect appeared only on the token after an uppercase-starting word,
and the simpler probes (`ab,cd`, `oneif`, `nested`, …) all used lowercase
or single-words that dodged the uppercase path. The `[25,2]:nt`
instrumentation was read as a cell-scheduling displacement when it was
actually the slice running on a `word_start` that was never set for `Int`
(because `Int` was never detected as a run). The wasm dump (STEP-0190)
showed correct `local 85` scheduling — because the scheduling **was**
correct; the application never told it to start the `Int` run.

## 4. Honest retraction

STEP-0185, 0186, 0187, 0188, 0189, and 0190 each concluded "compiler
lowering bug" at increasing confidence. **All six are retracted.** The
defect was application-layer. This is itself a valuable lesson in the
self-host methodology: the differential oracle correctly flagged a real
defect, but the layer attribution (compiler vs application) required the
per-byte classification instrumentation (`@24:73`) to resolve — and even
then, the misleading simpler probes delayed the correct attribution. The
M22 track's value (finding the defect) stands; the layer attribution is
corrected here.

## 5. Residuals

- Token **spans** (the parallel `List[I64]` offsets) are now trustworthy;
  the struct-of-arrays token triple (kinds + spans) is complete and S5
  (parser) is unblocked.
- No compiler change was needed; the frozen snapshots are untouched.
