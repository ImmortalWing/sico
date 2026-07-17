# Script Profile v0 contract baseline

> - status: complete-contract / prototype-pending
> - date: 2026-07-17
> - related step: STEP-0075

## Summary

STEP-0075 registered M8/M9 and froze the candidate Script Profile boundary needed by STEP-0076. It does not claim aggregate compiler codegen, adapter composition, an embedded runner, args/stdin execution or a standard library.

## Frozen candidate decisions

- entry: `sico:script/program@0.1.0.run(script-input) -> result<script-output, script-error>`;
- Text: strict UTF-8; Bytes: arbitrary octets;
- dynamic fixed-width values: explicit checked `I64/U64`; arbitrary-precision `Int` is not weakened;
- guest exit: 0–119; M8 tool/fault exits: 120–127 as specified by RFC-0029;
- limits: 1,024 args, 64 KiB per arg, 1 MiB total args, 8 MiB each stdin/stdout/stderr, 64 KiB error text, 64 MiB profile memory before stricter limits;
- baseline verified capabilities: `script.args`, `script.stdio`;
- explicit scoped capabilities: `environment.read`, `storage.read`, `storage.write`;
- M8 excludes network, process execution, streaming, async, REPL and top-level syntax;
- source cache: SHA-256 domain `SICO-SCRIPT-SOURCE-CACHE-V0\0` plus fixed-order, length-prefixed identity fields and exact digests;
- preferred architecture: Program Component + exact adapter + strict package + structured runner; direct Program export invocation is the fallback.

## Existing measured baseline

On the 2026-07-17 Windows release environment with Wasmtime 46.0.1 and a 114-byte minimal Component, after two warmups:

| Stage | Samples | Median | P95 |
|---|---:|---:|---:|
| `sico` process/version | 15 | 14.97 ms | 17.58 ms |
| compile + Component write | 15 | 32.89 ms | 48.90 ms |
| package write | 15 | 24.68 ms | 27.61 ms |
| verify + external Runtime | 15 | 65.38 ms | 95.37 ms |
| complete `sico-app dev` | 15 | 104.42 ms | 141.96 ms |
| raw Wasmtime `cache=n` | 20 | 37.48 ms | 53.69 ms |

The component was 114 bytes and the package 695 bytes. A separate in-process compiler loop built 30,000 tiny programs in 1,583.831 ms after sources were loaded, showing that process/filesystem/Runtime boundaries dominate this tiny case. These measurements are non-SLA planning evidence; the original interactive shell output is not yet stored as an immutable report artifact.

## STEP-0076 gate

The prototype must provide machine-readable cases for Unicode/non-Unicode argument handling, empty/binary/1 MiB stdin, separated stdout/stderr, guest exit, typed error, every frozen bound, malformed Component, wrong WIT/adapter identity and top-level import closure.

GO thresholds:

- cold P95 below 200 ms;
- warm P95 below 120 ms;
- correct 1 MiB binary roundtrip;
- bounded measured memory;
- exact Wasmtime 46.0.1 execution;
- no undeclared imports or default env/fs/network;
- deterministic prototype artifacts.

Failure activates the direct-runner fallback or returns RFC-0029/ADR-0009 to revision. It must not be hidden by changing the threshold after measurement.

## Validation completed

- the Script WIT draft parses through the repository `wit-parser` contract test;
- Rust formatting is canonical;
- the existing 23-package module boundary remains valid;
- planning document local links and repository diff whitespace were checked.

## Remaining evidence

No Program/Adapter behavior, composition, rich Canonical ABI, embedded Wasmtime runner, cache implementation or Script standard library has been implemented by STEP-0075. Those claims start at STEP-0076 and later M8 steps.
