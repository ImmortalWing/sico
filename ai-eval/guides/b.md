# Sico candidate B guide

- `newtype Name from Base` declares a nominal type.
- Records use `record Name:`, fields, and `end record`.
- Enums use `enum Name:`, variants prefixed by `case`, and `end enum`.
- Functions use `function name(parameters) returns ReturnType:`, an indented body, and `end function`.
- Every function result is returned with `return`.
- Generic types use `Type[A, B]`.
- Match uses `match value:`, arms written as `case Pattern:` followed by a returned value, and `end match`.
- Lambdas use `function(parameters) returns Type:` and `end function`.
- A successful Result is `ok(value)`; explicit error mapping uses `.map_error(...)`.
- Capability, resource, and interface blocks use `capability Name: ... end capability`, `resource Name: ... end resource`, and `interface Name: ... end interface`.
- Async functions add `async` before `function`; deterministic cleanup is `using value: ... end using` and structured concurrency is `task group: ... end task`.
- Exported effectful functions add `export`; `effects:` and `capabilities:` each introduce an indented list before executable statements.
- Resource methods write `borrow self` for a borrow and `self` for a consuming receiver.
- Use the exact declaration and member order requested by the task.
