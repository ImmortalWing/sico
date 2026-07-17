# Script Profile direct-runner prototype

This STEP-0076 prototype exercises the frozen `ScriptInput -> result<ScriptOutput, ScriptError>` value shape through Wasmtime 46.0.1's Component API with real guest Core Wasm and Canonical ABI lift/lower. It covers Unicode arguments, arbitrary binary input, separated output channels, guest exit values, structured Script errors, a 1 MiB roundtrip and an 8 MiB input-limit refusal.

Each fixture mode is a self-contained Program Component: a hard-coded guest Core Wasm module implementing `run`, lifted to the frozen Script signature with guest `memory` + `realloc`. The Script types are named through a local `sico:script/types@0.1.0` types-only instance import, which requires no host-side Linker definitions. Exporting an imported host function is rejected by Wasmtime 46.0.1, so the earlier host-func simulation was replaced by real guests.

Run it with:

```powershell
cargo run --release --manifest-path .\prototypes\script-profile\Cargo.toml --locked
```

The output is a machine-readable JSON summary. The fixture cases are in [`cases.json`](./cases.json).

On this Windows host the pinned `1.97.1-x86_64-pc-windows-gnu` toolchain cannot link the Wasmtime library (its `dlltool.exe` needs a standalone GNU `as.exe`, which is not installed). The verified run used the installed MSVC toolchain instead:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-msvc'
cargo run --release --manifest-path .\prototypes\script-profile\Cargo.toml --locked
```

## Runtime evidence

Runtime results (6/6 cases, cold/warm latency, memory sample, environment metadata and the exact Component encoding rules) are recorded in [`docs/reports/script-profile-prototype-v0.md`](../../docs/reports/script-profile-prototype-v0.md).

## Evidence boundary

The generated Components import only a types-only instance and run hard-coded guests; they validate the direct in-process runner path, Component type checking, real Canonical ABI transport and guest memory/realloc behavior. They do **not** yet provide compiler-generated guests, randomized ABI roundtrips, WASI CLI adapter composition, package manifests, caches or production Runtime limits. STEP-0076 remains in progress until the Program/Adapter composition experiment produces the GO/fallback decision.
