# STEP-0207: M22 S6 — cross-function call IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0206
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Generalize the Sico-written lowering from single-function modules to
modules with multiple function definitions. The bounded slice targets
direct calls between defined functions so declaration-order function ids,
module-wide name resolution and separated caller/callee parameter tables
are proven before constant or nested call arguments widen the argument
surface.

## 2. Contract and mechanism

- Functions keep source declaration order; ids are 1-based in that order
  (`FunctionId(1)` = first definition), matching the Rust per-file id
  base assignment.
- Call targets resolve by name across every definition in the module
  (forward references included). The first definition of a name wins;
  a repeated name is the typed refusal `ERR:E-SH-IR-FUNCTION-DUPLICATE`.
- Arity is checked against the callee's parameter list. Each argument
  must still be one of the caller's parameters; its kind is checked
  against the callee's parameter kind at that call position. Target,
  shape, argument, arity and type failures remain distinct stable codes.
- The call result starts at the caller's parameter count and carries the
  callee's return type; the module-wide return-type check still requires
  it to equal the caller's declared return type.
- Per-function structure is unchanged and now enforced per definition:
  exactly one `return` statement, `let`/`set`/`if`/`while`/`match`-free
  bodies, scalar parameter kinds and return types.
- Each function emits one block whose range and terminator range equal
  the function's own span: from its `function` keyword through the
  newline after `end function`. Parameter name/type search now starts at
  the function's own parenthesis instead of the first parenthesis in the
  file, so second and later definitions get correct ranges.

## 3. Executable evidence

Two new positive differentials:

```sico
function identity(value: Int) returns Int:
  return value
end function

function main(value: Int) returns Int:
  return identity(value)
end function
```

emits `call function:1, arguments [0]`, result `ValueId(1)`, and the
reordered cross-call `sub(y, x)` against `sub(left, right)` emits
`arguments [1, 0]`. Both deserialize, pass the independent verifier,
round-trip canonically and are byte-identical to Rust `lower_core`. The
cumulative positive differential set is now thirteen programs, including
the STEP-0206 self-recursive call.

Four new typed refusals are aligned with the Rust semantic gate that
`lower_core` runs internally: unknown callee
(`ERR:E-SH-IR-CALL-TARGET`), cross-function arity
(`ERR:E-SH-IR-CALL-ARITY`), cross-function parameter type
(`ERR:E-SH-IR-CALL-TYPE`) and duplicate definitions
(`ERR:E-SH-IR-FUNCTION-DUPLICATE`). Rust refuses all four; the guest
returns typed `invalid-input` with the matching stable code and no
partial IR. The complete selfhost parser suite remains 2/2 green on the
real Windows x64 runner.

## 4. Residuals

Constant or nested call arguments, fixed-width operations, general
statements and blocks inside function bodies, and full-corpus lowering
remain open. This step proves multi-definition modules and cross-function
call construction, not general call support or bootstrap closure.
