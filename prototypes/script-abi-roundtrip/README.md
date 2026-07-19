# Script ABI roundtrip harness

STEP-0079 evidence harness. Drives compiler-generated `sico:script/program@0.1.0` Program Components through Wasmtime 46.0.1:

- 10,000 seeded boundary roundtrips over one live instance (Unicode arguments, arbitrary binary stdin, separated channels, guest exit values);
- a repeated-call cleanup oracle (512 × 1 MiB plus 8 × 4 MiB calls on one instance — impossible without working `cabi_post_run` arena cleanup);
- a boundary-size case (1,024 arguments, 8 MiB stdin);
- six malicious-memory fixtures (invalid result/enum discriminants, invalid UTF-8, out-of-bounds and overflowing pointers, misaligned result area) that must fail closed;
- deterministic component identity (byte-identical rebuilds, SHA-256 reported).

Run it with the MSVC toolchain (the pinned GNU toolchain cannot link the Wasmtime crate on this host):

```powershell
tools\validate-step-0079.ps1
```

or manually:

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'
cargo run --release --manifest-path .\prototypes\script-abi-roundtrip\Cargo.toml --locked --offline
```

The output is a machine-readable JSON summary, also written to `target/evidence/step-0079-script-abi-roundtrip.json` by the validation script.

## Evidence boundary

The guests are compiler-generated from verified IR, but the harness constructs that IR directly; source-level Script syntax, adapter composition, manifests, caches and runner policy remain STEP-0080–0082 work. Argument list element content is verified through the independent host helper (`sico-runtime::canonical`, 10,000 seeded buffer roundtrips) and through the STEP-0076 hard-coded guests; compiler-generated guests reflect argument content only after list element access lowering lands with the standard library (STEP-0083).
