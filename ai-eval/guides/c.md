# Sico candidate C guide

- `opaque Name = Base` declares a nominal type.
- Records and enums use braces. Fields and enum variants require trailing commas, for example `name: Text,`. A short enum may stay on one line: `enum Color { Red, Green, Blue }`. A variant payload is named: `Invalid(value: Text)`.
- Functions use `fn name(parameters) -> ReturnType { ... }`.
- Every function result is returned with `return`.
- Generic types use `Type[A, B]`, for example `Result[Int, ParseError]` and `List[Int]`.
- Match is an expression: `match value { Pattern: value, ... }` with trailing commas on arms. Patterns always qualify the variant with its enum name, for example `Color.Red: "red",`. A payload arm binds the payload name, for example `ParseError.Invalid(value): ...`.
- A successful Result is `Ok(value)`; explicit error mapping uses `.map_error(fn(problem) => match problem { ... })`.
- Record and variant payload construction uses `Type(field: value)` or `Variant(value)`.
- Capability, resource, and interface declarations use braces. Their members are bare signatures such as `fn write(text: Text) -> Result[Unit, ConsoleError]` with no body and no braces of their own.
- Async functions add `async` before `fn`; deterministic cleanup is `using value { ... }` and structured concurrency is `task group { ... }`. A spawned task is bound with `let name = spawn call(...)` and observed with `await`.
- Exported effectful functions add `export`; `effects [...]` and `capabilities [...]` each appear on their own line between the signature and the function body brace.
- Resource methods write `borrow self` for a borrow and `self` for a consuming receiver.
- Use the exact declaration and member order requested by the task.

## Canonical layout

Output is compared byte-for-byte against a canonical reference, so layout matters:

- indent with exactly 2 spaces per level; never use tabs;
- exactly one blank line between top-level declarations; no blank lines elsewhere;
- keep the trailing commas shown above on multi-line fields, variants and match arms;
- no comments and no trailing whitespace; the file ends with a single newline.

## Complete example

```sico
record Point {
  x: Int,
  y: Int,
}

fn origin() -> Point {
  return Point(x: 0, y: 0)
}

capability Clock {
  fn now() -> Int
}

export fn tick(clock: Clock) -> Int
  effects [clock.now]
  capabilities [clock]
{
  return clock.now()
}
```
