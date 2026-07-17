# Script Profile direct-runner prototype

This STEP-0076 prototype exercises the frozen `ScriptInput -> result<ScriptOutput, ScriptError>` value shape through Wasmtime 46.0.1's dynamic Component API. It covers Unicode arguments, arbitrary binary input, separated output channels, guest exit values, structured Script errors, a 1 MiB roundtrip and an 8 MiB input-limit refusal.

Run it with:

```powershell
cargo run --release --manifest-path .\prototypes\script-profile\Cargo.toml --locked
```

The output is a machine-readable JSON summary. The fixture cases are in [`cases.json`](./cases.json). `cargo check` succeeds with the pinned Rust 1.97/Wasmtime dependencies. Executing the binary on the current Windows GNU host additionally requires a complete MinGW assembler/linker installation; the installed Rust toolchain omits `as.exe`, so runtime results are not yet claimed from this machine.

## Evidence boundary

The generated Component imports a rich typed `host-run` function and re-exports it as `run`. It is an executable test candidate for the direct in-process runner fallback, Component type checking and value transport. It does **not** yet provide runtime evidence, nor does it validate compiler-generated guest memory, Canonical ABI allocation, WASI CLI adapter composition, package manifests, caches or production Runtime limits. STEP-0076 remains in progress until execution and the separate Program/Adapter composition experiment produce a GO/fallback decision.
