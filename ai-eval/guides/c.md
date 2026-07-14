# Sico candidate C guide

- `opaque Name = Base` declares a nominal type.
- Records and enums use braces. Fields and enum variants require trailing commas.
- Functions use `fn name(parameters) -> ReturnType { ... }`.
- Every function result is returned with `return`.
- Generic types use `Type[A, B]`.
- Match is an expression: `match value { Pattern: value, ... }`.
- A successful Result is `Ok(value)`; explicit error mapping uses `.map_error(fn(problem) => match problem { ... })`.
- Record and variant payload construction uses `Type(field: value)` or `Variant(value)`.
- Use the exact declaration and member order requested by the task.
