# Script Profile Program/Adapter composition prototype

This completed STEP-0076 prototype exercises the frozen `ScriptInput -> result<ScriptOutput, ScriptError>` value shape through Wasmtime 46.0.1's Component API on both direct and composed paths. It covers Unicode arguments, arbitrary binary input, separated output channels, guest exit values, structured Script errors, a 1 MiB roundtrip and an 8 MiB input-limit refusal.

Each fixture mode is a self-contained Program Component: a hard-coded guest Core Wasm module implementing `run`, lifted to the frozen Script signature with guest `memory` + `realloc`. The Script types are named through a local `sico:script/types@0.1.0` types-only instance import, which requires no host-side Linker definitions. Exporting an imported host function is rejected by Wasmtime 46.0.1, so the earlier host-func simulation was replaced by real guests.

The deterministic `sico:script/adapter@0.1.0` candidate imports the Program function, canonical-lowers it into private Adapter memory, calls it through a core trampoline and canonical-lifts the trampoline as its own `run`. An outer Component explicitly instantiates and wires Program + Adapter. This avoids unsupported function re-export while testing real nested Component composition and a second Canonical ABI boundary.

Run it with:

```powershell
cargo run --release --manifest-path .\prototypes\script-profile\Cargo.toml --locked
```

The output is a machine-readable JSON summary. The fixture cases are in [`cases.json`](./cases.json).

On this Windows host the pinned GNU toolchain cannot link the Wasmtime library (its `dlltool.exe` needs a standalone GNU `as.exe`, which is not installed). The verified run used the installed MSVC toolchain from a VS 2022 x64 Developer PowerShell instead:

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'
cargo run --release --manifest-path .\prototypes\script-profile\Cargo.toml --locked
```

## Runtime evidence

Direct-path raw results and encoding findings are recorded in [`script-profile-prototype-v0.md`](../../docs/reports/script-profile-prototype-v0.md). The composition decision, raw JSON, 20-run comparison and memory sample are recorded in [`script-profile-composition-v0.md`](../../docs/reports/script-profile-composition-v0.md).

## Evidence boundary

The generated Components import only a types-only instance and run hard-coded guests. They validate direct and nested composed execution, Component type checking, deterministic Adapter identity and real Canonical ABI transport across Program and Adapter memory. They do **not** provide compiler-generated guests, randomized ABI roundtrips, WASI CLI adaptation, package manifests, caches or production Runtime limits. Those remain STEP-0077–0084 work.
