# STEP-0205: M22 S6 — multi-scalar parameter IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0204
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Replace the one-`Int`-parameter transition shape with canonical function
signatures for multiple scalar parameters, preserving parameter order,
types, source identity and direct SSA returns. This is the structural
prerequisite for user-function call lowering.

## 2. Contract and mechanism

- The accepted parameter type subset is `Int`, `Bool`, `Text`, `I64` and
  `U64`, mapped to IR kinds `int`, `bool`, `string`, `i64` and `u64`.
- Parameters are parsed left-to-right from the function signature. IDs
  are contiguous from zero, matching Rust `lower_core`; constants begin
  at `parameters.len()`.
- Because the transition lexer intentionally stores token text rather
  than full spans, parameter name ranges are recovered monotonically
  from the signature bytes. The scan advances past each parameter type,
  preventing a later name from matching text in an earlier declaration.
- A returned parameter resolves by name to its declared ID and type. No
  `copy` instruction is emitted; the block terminator directly references
  the parameter SSA value, matching the Rust lowering contract.
- Unsupported parameter types return
  `ERR:E-SH-IR-PARAMETER-TYPE`; they never produce partial IR.

## 3. Executable evidence

The real-runner differential adds:

```sico
function choose(count: Int, ready: Bool, label: Text, delta: I64, bits: U64) returns Text:
  return label
end function
```

The guest module deserializes, passes the independent verifier, round-trips
canonically and is byte-identical to Rust `lower_core`. This proves all
five scalar parameter kinds, five ordered parameter ranges and a nonzero
direct SSA return (`ValueId(2)`) in one closed fixture. The cumulative
positive differential set is now ten programs.

`Bytes` is deliberately outside this slice and returns typed
`invalid-input` with `ERR:E-SH-IR-PARAMETER-TYPE`. The complete selfhost
parser suite remains 2/2 green on the real Windows x64 runner.

## 4. Residuals

Multiple functions, user calls, fixed-width operations, general blocks
and full-corpus lowering remain open. STEP-0205 establishes the signature
and argument-value foundation but does not claim call support or bootstrap
closure.
