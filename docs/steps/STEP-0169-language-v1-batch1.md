# STEP-0169: M20 language v1 batch 1 — infix equality conditions (RFC-0044)

> - status: complete
> - phase: M20 §3.1 language v1 (first batch from the measured friction log)
> - completed: 2026-09-13
> - owners: autonomous-agent
> - artifacts: [`RFC-0044`](../rfc/RFC-0044-language-v1-batch1.md) (accepted), `tests/end-to-end/infix-equality.sico` + `.test.json`, matrix entry `infix-equality-conditions`

## 1. What was done

Infix `==` conditions lower directly (RFC-0044): `left == right` in an
`if` condition emits `Operation::EqualFixed` — the exact comparison the
typed `I64.equal`/`U64.equal` dispatch produces — for same-width fixed
operands. The frozen revision-guard path now declines non-Revision
equality with a fallback to the general CFG instead of refusing, which
is what makes the common `if a == b: return ...` shape lowerable. Mixed
widths and non-fixed operands keep the typed refusal (narrowed).

## 2. The bare-literal decision point (measured, recorded)

The original batch also planned "bare integer literals are I64". The
implementation was attempted and REVERTED with measured evidence: bare
literals are unbounded `Int` in the frozen M2 semantics, and the NUM-001
arbitrary-precision oracle (`999999999999999999999999 + 1`, SEM-021)
depends on that — bare=I64 breaks the oracle's fold and would kill the
unbounded-Int story that the type system still carries. The decision
(bare = I64 with explicit-Int literals, vs keep Int + typed-let syntax)
is now RFC-0044's recorded v1 open question with the NUM-001 evidence
attached. During the attempt, three adjacent real defects were found and
fixed: the `I64.literal` special case now tolerates I64-typed literal
arguments, and the revision-if path gained an honest fallback instead of
a dead-end refusal.

## 3. Validation

- `infix-equality` e2e: 1/1 byte-exact via the real runner (typed
  literal equality, `text.length(...) == U64.literal(0)`, equality in
  checked-arithmetic match arms).
- Matrix: executable entry `infix-equality-conditions` (check/build/run).
- Full semantic/IR suites green after the reverts (NUM-001 and the 58
  semantic oracles unchanged).
