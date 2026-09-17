# STEP-0199: M22 S6 — return-expression atom and call shape

> - status: complete
> - phase: M22 S6 expression parsing continuation after STEP-0197/0198
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. Objective

Replace the first purely aggregate statement signal with an expression
shape that can feed later typed IR emission. For every `return` in a
function body, preserve an atomic integer/bool/name or a dotted call head
and its top-level argument count.

## 2. Mechanism

- The Sico lexer proxy now retains `.` and emits a canonical `<nl>` line
  marker. This makes `I64.checked_add(...)` a recoverable path and bounds
  each return expression to its source line.
- `expression_shape` emits `int:N`, `bool:true|false`, `name:X`, or
  `call:PATH/ARITY`. `call_arity` tracks parenthesis depth, so commas in a
  nested call do not change the outer arity.
- `return_shapes` stops at the current `end function` and preserves return
  order. The fixed function shape now appends `expr=[...]`.

## 3. Executable evidence

The controlled two-function corpus contains integer returns and:

```sico
return I64.checked_add(I64.literal(1), b)
```

Real `sico build --profile script-v0` plus real Wasmtime runner execution
produces a byte-exact shape whose relevant suffix is:

```text
expr=[call:I64.checked_add/2]
```

The same run preserves `int:1,int:0` in source order for `main`. Runner
integration result: 1/1 green on Windows x64.

## 4. Support boundary

This is an `internal-fixture` parser shape, not typed IR. It does not yet
emit recursive argument nodes, types, bindings, source ranges, string
atoms, operators, or typed parse refusals; string/comment awareness also
remains incomplete in the transition lexer. The next slice must turn a
bounded accepted expression subset into canonical typed operations and
submit it to the Rust IR verifier. Unsupported forms must become typed
refusals rather than the transitional `unsupported` shape before any
compiler-support claim.
