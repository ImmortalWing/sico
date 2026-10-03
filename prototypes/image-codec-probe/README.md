# M24 image-codec feasibility probe (measurement only)

This prototype answers one question the owner deferred to evidence before
committing the M24 codec work:

> Can the RFC-0050 codec implementation home stay a **pure package
> Component** — a Rust crate compiled to `wasm32-unknown-unknown`, wrapped by
> the repository's own `sico_codegen_wasm::package_builder::build_package_component`,
> executed through the existing package path — so that RFC-0050's "zero
> compiler/Runtime change" claim survives? Or must it fall back to a host
> provider (which would contradict RFC-0050 §1 and M24 §3 gate 5)?

It is a bounded probe under the 24-A execution card. It moves **no gate** and
claims **no capability**. Nothing here is runtime/platform support.

## Verdict: `package-route-feasible`

Recorded in `results/probe-report.json` after a clean run on this host
(Windows x64, `1.98.0-x86_64-pc-windows-gnu`, wasmtime `=47.0.2`):

| Check | Result |
| --- | --- |
| Guest core module rebuilds byte-identical after `cargo clean` | **true** |
| `build_package_component` output byte-identical on rebuild | **true** |
| Exported instance `sico:user/image-codec-probe@1.0.0` | present |
| Exported functions `decode-png`, `encode-png` | present |
| 9 decode cases vs an **independent** Python/zlib PNG oracle | **9/9 pass** |
| Encode byte-stable across two calls | true |
| Encode chunk set frozen to `{IHDR, IDAT, IEND}` | true |
| Round trip `decode(encode(pixels))` reproduces the payload | true |
| 3 encode refusals typed (`oversized`, `oversized`, `corrupt-stream`) | 3/3 pass |

So the pure-package route is demonstrated end to end. **Decision: the M24
codec implementation home stays a package; RFC-0050's original text is not
amended to a host provider.** The earlier owner answer ("host provider +
crates.io codec") is superseded by this measurement, exactly as the
"probe first, then decide" option anticipated.

## What was actually proven

- A hand-written Canonical ABI shim (`guest/src/lib.rs`) that matches the
  in-repo emitter (`emit_csv_error`) lifts and lowers correctly under
  wasmtime's component model: `Bytes` params flatten to `(i32 ptr, i32 len)`,
  `U64` to `i64`, and an aggregate `Result[Bytes, Text]` returns one `i32`
  pointer to a callee-allocated 12-byte return area (disc byte at 0, payload
  ptr at +4, len at +8).
- The `png = "=0.18.1"` crate links into a `wasm32-unknown-unknown` cdylib
  with `panic = "abort"`, `lto`, `codegen-units = 1`, `strip`, and produces a
  byte-reproducible core module.
- Decode output shape is the RFC-0050 §1 flattening required by the v0
  user-WIT value set (records/enums are refused with E8017): a 16-byte LE
  header (width u64, height u64) followed by RFC-0041 BGRA8 pixels; the error
  case rides a case-name `Text`, following the HTTP case-name precedent.

## Contract gaps found by measurement (for the real 24-A STEP)

1. **Encode-side buffer/dimension mismatch has no `CodecError` case.** RFC-0050's
   closed error set has no name for "pixel buffer length != width*height*4" on
   encode. The probe maps it to `corrupt-stream` and flags it in a source
   comment. The real contract must either add a case or explicitly reuse
   `corrupt-stream`.
2. **The gnu target needs MinGW binutils on PATH.** `windows-sys` (a wasmtime
   dependency) uses `raw-dylib`, so `cargo build` for
   `x86_64-pc-windows-gnu` fails with `error calling dlltool` unless
   `as.exe`/`dlltool.exe` are reachable — the same precondition
   `tools/run-ci.ps1` and STEP-0196 document for `ring`. This is an
   environment precondition, not a defect in the package route. The MSVC host
   variant is not usable here either (Git's `link.exe` shadows the real one and
   no VS Build Tools are installed).

## Layout

- `guest/` — the probe codec compiled to `wasm32-unknown-unknown` (excluded
  from the root workspace via `Cargo.toml [workspace] exclude`).
- `host/` — `codec-probe.exe`: wraps the core module with
  `build_package_component`, exercises it through wasmtime's dynamic component
  API (the same shape as `runner/sico-runner/tests/vision_package.rs`), and
  writes `results/probe-report.json`.
- `tools/make-fixtures.py` — independent PNG oracle (Python `zlib` + manual
  chunk/CRC framing); shares no code with the `png` crate the guest links.
- `tools/probe.ps1` — the reproducible runner: fixtures -> build -> clean ->
  rebuild (reproducibility) -> run -> verdict.

## Reproduce

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"   # probe.ps1 locates this itself
powershell -NoProfile -ExecutionPolicy Bypass -File `
  prototypes\image-codec-probe\tools\probe.ps1 -RepositoryRoot (Get-Location).Path
```

`probe.ps1` locates MinGW binutils and Python from `target\tooling\` (falling
back to the sibling main checkout when run from an external worktree). It ends
by printing `PROBE VERDICT: package-route-feasible` and the two evidence lines
(core reproducible, codec-probe verdict).
