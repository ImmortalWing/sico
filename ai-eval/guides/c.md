# Sico candidate C guide

- `opaque Name = Base` declares a nominal type.
- Records and enums use braces. Fields and enum variants require trailing commas.
- Functions use `fn name(parameters) -> ReturnType { ... }`.
- Every function result is returned with `return`.
- Generic types use `Type[A, B]`.
- Match is an expression: `match value { Pattern: value, ... }`.
- A successful Result is `Ok(value)`; explicit error mapping uses `.map_error(fn(problem) => match problem { ... })`.
- Record and variant payload construction uses `Type(field: value)` or `Variant(value)`.
- Capability, resource, and interface declarations use braces.
- Async functions add `async` before `fn`; deterministic cleanup is `using value { ... }` and structured concurrency is `task group { ... }`.
- Exported effectful functions add `export`; `effects [...]` and `capabilities [...]` occur between the signature and function body brace.
- Resource methods write `borrow self` for a borrow and `self` for a consuming receiver.
- Use the exact declaration and member order requested by the task.
