# STEP-0202: M22 S6 — typed parameter and direct SSA return IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0201
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. Objective

Extend the first Rust-identical IR slice from a zero-parameter integer
constant to one typed `Int` parameter and name resolution, preserving
canonical parameter/Value IDs and exact source spans.

## 2. Mechanism

- `--emit-ir` accepts zero or one parameter. The one-parameter form must
  be exactly `name: Int`; larger arity or another type is a stable typed
  refusal.
- The parameter identifier byte range is recovered from the function
  signature and emitted in the IR parameter record.
- A single-token return is classified as a canonical integer constant or
  as the declared parameter binding. Unknown names fail closed.
- Constant instructions start after parameter ValueIds. A direct
  parameter return emits no instruction and points the return terminator
  at parameter `ValueId(0)`.

## 3. Oracle correction

The planned shorthand said “parameter/Copy”. The first Rust differential
contradicted that expectation: `lower_core` represents
`return value` as an empty instruction list plus
`return data: 0`, directly reusing the parameter SSA value. The Sico
frontend now emits that exact shape; no redundant `Copy` was added merely
to match the plan wording. STEP-0201's residual text is corrected in the
same change.

## 4. Executable evidence

The same real-built Sico guest now lowers both:

```sico
function answer() returns Int:
  return 7
end function
```

and:

```sico
function identity(value: Int) returns Int:
  return value
end function
```

For each source, the test deserializes the guest bytes to
`sico_ir::Module`, runs the independent verifier, checks canonical
reserialization, and requires byte-for-byte equality with Rust
`lower_core`. The identity case pins parameter range `18..23`, empty
instructions, and terminator `ValueId(0)`. The negative-bare typed refusal
from STEP-0201 remains covered. Combined selfhost parser suite: 2/2 green
on the real Windows x64 runner.

## 5. Residuals

Bool/Text constants, more parameters, calls, fixed-width operations,
general token spans, blocks/control flow and multiple functions remain
open. Every added shape must remain Rust-lowering byte-identical before
codegen work can claim coverage.
