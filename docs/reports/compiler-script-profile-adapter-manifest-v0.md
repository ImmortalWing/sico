# Compiler Script profile, adapter composition and manifest v1 evidence

> - date: 2026-07-17
> - step: STEP-0080
> - status: complete
> - runtime: Wasmtime 46.0.1 (`823d1b8f2`, 2026-06-24), WASI `0.2.12`

## Implemented boundary

`sico build --profile script-v0` compiles one source file with explicit `ScriptInput`/`ScriptOutput`/`ScriptError`/`ScriptErrorCode` declarations and exactly one `main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]` into a deterministic `sico:script/program@0.1.0` Program Component (STEP-0079 ABI). Declaration shapes are validated against the frozen ABI before lowering; wrong shapes, wrong signatures and unknown profiles fail closed. The source surface gained `error(...)` construction and profile-owned `Bytes`/`List[Text]` types with user-type shadowing preserved (`newtype Bytes` keeps working); source `main` maps to the boundary `run`, and source `snake_case`/`CamelCase` names map to WIT `kebab-case` fields/cases.

The versioned Script Adapter (`sico:script/adapter@0.1.0`, SHA-256 `185ebf1d393cb2b93c1d023f89d19d36c323c64f858a2fef1092f6a0283502c7`, 2,198 bytes) is the only Script component importing the broad WASI CLI interfaces (`wasi:cli/*@0.2.12`, `wasi:io/*@0.2.12`). It reads WASI arguments (skipping `argv[0]`), reads stdin in 64 KiB chunks to the 8 MiB bound treating `stream-error::closed` as clean EOF, invokes the lowered Program `run`, writes the channels and maps the Script result to WASI 0.2 exit semantics. `sico-app compose` wires Program + Adapter + WASI into one command Component exporting `wasi:cli/run@0.2.12`; the import closure of the composed Component is exactly `sico:script/types@0.1.0` plus the six WASI capability instances and `wasi:io/error@0.2.12` (types-only).

Manifest v1 (`sico.sapp.manifest.v1`, `sico-app pack --script`) is a strict schema (`deny_unknown_fields`) recording the exact profile/world/entry, language semantics (`sico.ir.v0`), Script WIT package `sico:script@0.1.0` with its SHA-256 (`00c6905bc06c47a8abea5e285b762bb8643c2b55608ecb04e8f2d8e9b0863192`) and the adapter identity with its digest. `authorize` maps the composed command imports exactly to `script.args`/`script.stdio`; unknown imports, versions, capabilities or tampered identities fail closed, and v0 manifests cannot carry a script identity (and vice versa).

## Exact-engine evidence

`tools/validate-step-0080.ps1` records `STEP_0080_OK`:

- `script-echo.sico` and `script-reject.sico` build to deterministic Program Components;
- composed under the versioned adapter, `printf 'step-0080-payload' | wasmtime run echo.command.wasm alpha beta` writes the payload to both channels with exit 0, and `reject.command.wasm` writes `rejected` to stderr with exit 1;
- `sico-app pack --script` produces a v1 package whose inspect output carries the exact script identity and whose import/capability closure authorizes only with `script.args` + `script.stdio` grants (`HostDenied` otherwise).

Component Model encoding findings verified along the way: interface imports are instances (no `#function` names); resources cannot be declared inside instance types and plain-name `SubResource` type imports need a linker implementation at runtime, so stream resources come from the WASI instances; aggregate types (variant/record) must be introduced with a name and functions must reference the export index; resource methods are exported as `[method]type.name`; WASI 0.2.12 stdin EOF is `stream-error::closed`; hostcall fuel is per-element and must be budgeted per call.

## Honest limits

- WASI 0.2 exit semantics are `result<_, _>`: the composed command path collapses guest exit values to 0/1. Exact `0..=119` guest codes and the `120..=127` tool mapping are direct-runner contracts (STEP-0081/0082), not adapter claims.
- The adapter performs one bounded stdin read pass and whole-channel writes; streaming/backpressure is M9.
- `sico-app run` argument/stdin wiring for script packages is STEP-0082; STEP-0080 evidence executes the composed command through the Wasmtime CLI.
- STEP-0077–0079 evidence and the full workspace regression remain green.

## Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
cargo test --offline --locked -p sico-codegen-wasm -p sico-runtime -p sico-cli -p sico-package -p sico-app-cli
.\tools\validate-step-0080.ps1
```
