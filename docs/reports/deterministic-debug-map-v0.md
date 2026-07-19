# Deterministic compiler debug map v0

> - status: verified compiler artifact
> - date: 2026-07-19
> - scope: STEP-0096 only

## Result

Sico now emits an identity-bound debug triplet on explicit `sico build --debug-info`. The Component contains only a compact canonical link; the bounded canonical map and identity remain sidecars. Rebuilding the same source with the same compiler produces identical bytes for all three files, while ordinary builds remain free of the link and preserve their prior semantics.

The code offset is derived from the emitted Core Wasm function body, not estimated from IR order. Each IR operation and block terminator captures its local encoder byte interval; the completed Core module is parsed to obtain the absolute function-body range. Script-generated allocator and intrinsic-helper bodies are marked generated with no source span.

## Integrity properties

- exact source bytes, compiler executable, pre-link Component, map and final Component are SHA-256 bound;
- the final Component digest is outside the embedded link, avoiding a circular hash;
- map and identity JSON are compact canonical serde output with unknown fields denied;
- function coordinates are qualified by `core_module`;
- source documents, module/function bindings, mapping order, overlap, integer limits and the 16 MiB map ceiling fail closed;
- artifact mix-and-match, truncation, duplicate/missing links and source-document substitution fail verification;
- display URIs never authorize file access and absolute paths are not emitted by default.

## Integration policy

The new `sico-observability` crate is a data-only contract layer. It can be shared by compiler, Runtime and tooling without making the compiler depend on Runtime or giving tooling new execution capability.

Debug sidecars are developer/runtime-observation inputs, not `.sapp` resources. Packaging preserves a linked Component exactly but excludes map/identity bytes; later Runtime/debug launch must receive the sidecars explicitly and verify the full chain. The normal execution cache and debug-disabled build path remain unchanged, so no cache entry gains hidden source metadata.

## Evidence

`tools/validate-step-0096.ps1` runs four package test suites, the STEP-0095 contract regression, the 25-package module-boundary validator and two real CLI build classes. Its observed sample contained 4 mappings/1 function for `answer.sico`, and 28 mappings/8 functions including 7 generated mappings for `script-word-count.sico`.

```text
STEP_0096_OK packages=4 deterministic_triplet=true maps=4/28 functions=1/8 synthetic=7 package_roundtrip=true module_packages=25 plain_debug_link=false authority=unchanged next=STEP-0097
```

No Runtime source frame or debugger behavior is claimed by this report.
