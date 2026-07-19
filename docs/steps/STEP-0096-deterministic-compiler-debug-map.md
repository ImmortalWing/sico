# STEP-0096: deterministic compiler debug-map artifact

> - status: complete
> - phase: M10
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Implement the compiler-owned part of RFC-0035: deterministic source-to-Core-Wasm mappings, a canonical sidecar, a compact digest link in the final Component and an exact build identity. Keep debug-disabled builds byte-compatible and do not add Runtime, DAP or execution authority.

## 2. Decision and contract clarification

The accepted non-circular chain is now executable:

```text
source + compiler identity
  -> pre-debug Component code digest
  -> canonical debug-map digest
  -> sico.debug-link.v0 custom section
  -> final Component digest
  -> canonical debug identity
```

Implementation of Script Components exposed one ambiguity in the STEP-0095 map shape: function index `0` can exist in the guest, transport and adapter Core modules. RFC-0035, the JSON Schema, fixtures and validator now require `core_module` on both function and mapping rows. This is a clarification needed to make the frozen v0 coordinates unambiguous, not a new debugger feature or authority.

## 3. Changes

- Added authority-free [`sico-observability`](../../crates/sico-observability/src/lib.rs), which owns strict canonical data validation and Component link verification but cannot launch processes, access Host providers or execute guests.
- `sico-codegen-wasm` records byte ranges while emitting each verified IR operation and block terminator, resolves them to Component-file absolute offsets for the embedded guest Core module with `wasmparser`, and emits source or explicit generated mappings.
- Script alloc/realloc/post-return and used intrinsic helper functions receive synthetic `generated=true`, `source=null` rows instead of false user locations.
- `sico build --debug-info` installs an all-or-none triplet:
  - `<output>`: linked Component;
  - `<output>.debug-map.json`;
  - `<output>.debug-identity.json`.
- Output paths are not embedded as authority. Display URIs are bounded `workspace://` or `sico-source://` observations; source identity is the exact byte digest.
- Existing builds without `--debug-info` remain unchanged and contain no Sico debug-link section.
- `.sapp` packaging accepts and preserves the linked Component byte-for-byte while deliberately excluding developer sidecars from the package. A future debugger must provide and verify the exact sidecar; package transport does not silently absorb source metadata.
- The machine module contract now contains a separate `observability` module. Compiler, Runtime and tooling may depend on this data-only module; it has no dependency back into those authority-bearing layers.

## 4. Mapping evidence

The tests cover:

- real parsed numeric source with exact UTF-8 spans;
- calls, dispatcher control flow, matches, jumps and a back edge with independently assigned source ranges;
- Script record/result lowering and intrinsic helpers through `script-word-count.sico`;
- generated arena and helper functions with no forged source span;
- deterministic repeat builds and valid Component parsing;
- malformed JSON, truncation, stale/mixed artifact sets, duplicate links, non-canonical JSON, reordered/duplicate contract fixtures and source-document mismatch;
- atomic refusal when any target sidecar already exists;
- packaging roundtrip without embedding sidecar bytes.

Debug maps describe addressable Core Wasm instructions. Canonical-lift declarations that do not own a Core instruction range are not assigned invented offsets.

## 5. Validation

Executed from repository root:

```powershell
./tools/validate-step-0096.ps1
```

Result:

```text
STEP_0096_OK packages=4 deterministic_triplet=true maps=4/28 functions=1/8 synthetic=7 package_roundtrip=true module_packages=25 plain_debug_link=false authority=unchanged next=STEP-0097
```

Targeted Clippy also passed with warnings denied:

```powershell
$env:RUSTUP_TOOLCHAIN='1.97.0-x86_64-pc-windows-gnu'
cargo clippy --offline -p sico-observability -p sico-codegen-wasm -p sico-cli -p sico-package --all-targets --all-features -- -D warnings
```

## 6. Honest boundary and next action

This step proves compiler artifact production and validation only. It does not prove Runtime stack walking, trap/source lookup, typed fault JSON, signals, execution events or any DAP request. `sico.debug` remains refused.

The next executable step is STEP-0097 structured Runtime faults and source frames. It must consume the exact bound map and fail closed when the link, map, source document or Component is absent/stale.

## 7. Audit links

- [`implementation report`](../reports/deterministic-debug-map-v0.md)
- [`M10 plan`](../plans/M10-runtime-observability-debugging.md)
- [`RFC-0035`](../rfc/RFC-0035-runtime-observability-debug-v0.md)
- [`STEP-0096 validator`](../../tools/validate-step-0096.ps1)
