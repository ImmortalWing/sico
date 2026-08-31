# Sico candidate B guide

- `newtype Name from Base` declares a nominal type.
- Records use `record Name:`, one `field name: Type` line per member, and `end record`.
- Enums use `enum Name:`, variants prefixed by `case`, and `end enum`. A variant payload is named: `case Invalid(value: Text)`.
- Functions use `function name(parameters) returns ReturnType:`, an indented body, and `end function`.
- Every function result is returned with `return`.
- Generic types use `Type[A, B]`, for example `Result[Int, ParseError]` and `List[Int]`.
- Match uses `match value:` and `end match`. Each arm is `case EnumName.Variant:` on its own line with the arm body on the following indented lines. Patterns always qualify the variant with its enum name; a payload arm binds the payload name, for example `case ParseError.Invalid(value):`.
- Lambdas use `function (parameters) returns Type:` and `end function`.
- A successful Result is `ok(value)`; explicit error mapping uses `.map_error(...)`.
- Capability, resource, and interface blocks use `capability Name: ... end capability`, `resource Name: ... end resource`, and `interface Name: ... end interface`. Their members are bare signatures such as `function write(text: Text) returns Result[Unit, ConsoleError]` with no body.
- Async functions add `async` before `function`; deterministic cleanup is `using value: ... end using` and structured concurrency is `task group: ... end task`. A spawned task is bound with `let name = spawn call(...)` and observed with `await`.
- Exported effectful functions add `export`; `effects:` and `capabilities:` each introduce an indented list with one entry per line before executable statements.
- Resource methods write `borrow self` for a borrow and `self` for a consuming receiver.
- Use the exact declaration and member order requested by the task.

## Canonical layout

Output is compared byte-for-byte against a canonical reference, so layout matters:

- indent with exactly 2 spaces per level; never use tabs;
- exactly one blank line between top-level declarations; no blank lines elsewhere;
- never place a body on the same line as its declaration header;
- no comments and no trailing whitespace; the file ends with a single newline.

## Complete example

```sico
record Point:
  field x: Int
  field y: Int
end record

function origin() returns Point:
  return Point(x: 0, y: 0)
end function

capability Clock:
  function now() returns Int
end capability

export function tick(clock: Clock) returns Int:
  effects:
    clock.now
  capabilities:
    clock
  return clock.now()
end function
```
