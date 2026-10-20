# STEP-0315: M22 22-D S5 slice 1 — seven declared Core-Wasm codegen families byte-equal

> - status: implementation complete locally; independent CI pending (gitcode pushed, GitHub unreachable from this host)
> - phase: M22 compiler self-host, execution card 22-D (S5 widening)
> - date: 2026-09-29
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

Card 22-D requires S5 to advance by an independent codegen byte corpus, not
formatter percentages. The existing S5 seam accepted exactly one shape
(a nonnegative `Int` constant function). This slice widens the
`--emit-core-hex` seam to seven declared statement families, each accepted
only when the whole source byte-equals a rebuilt canonical form: `I64`/
`U64.literal(n)` constants (i64 backend template), `true`/`false` Bool
constants (i32 template), single-scalar identity (`value: I64|U64|Bool`),
two-parameter identity returning either parameter (`a: T, b: T`),
`I64|U64.less_than(a, b)` (arithmetic vs logical comparison opcodes
`0x53`/`0x54`), and `I64|U64.checked_add(a, b)` (the full STEP-0148
checked-add template with the widened `[tag, ok, error]` Direct-ABI
result). Emission is through one shared `wasm_module_hex` builder
(header, type section from declared content, export, code section) so
every family shares the exact section layout the oracle produces; the
legacy `constant_core_hex` template is byte-identical to before (the
original constant fixture still passes untouched). Byte-count correctness
is enforced structurally: type-section length comes from
`hex_text_bytes` (hex chars / 2), and body lengths are pinned per family
(the constant family keeps the computed `positive_sleb_length` because
the LEB encoding is variable-length). Out-of-subset sources still fail
closed with `unsupported codegen source shape`; seven pinned refusal
fixtures prove no silent fallback. No language, WIT or Host contract
changes.

## Evidence and verdict

- 14 byte-equal fixtures: for each, the Rust oracle
  (`lower_core` + `sico_codegen_wasm::compile`) and the guest compiler
  agree byte-for-byte, and each artifact passes the wasmparser
  validator (`sico_compiler_encodes_declared_scalar_core_wasm_families_byte_exactly`).
- The pre-existing constant-fixture test still passes, proving the
  shared-builder refactor is byte-stable for the inherited shape.
- 7 typed refusal fixtures (wrong parameter name, return-type mismatch,
  three parameters, undeclared intrinsic `greater_than`, `checked_sub`,
  out-of-range U64 literal, undeclared `I64.add`) all refuse with
  `unsupported codegen source shape` and the valid identity fixture
  recovers afterwards in a fresh Store.
- `selfhost/codegen-corpus-v0.json` (schema `sico.m22.codegen-corpus.v0`)
  pins all 21 fixtures by SHA-256; `tools/validate-step-0315.ps1` checks
  the markers, the 14-entry count, and the inherited validator chain.
- The formatter canary remains 30/30 (100%); the S5 seam is measured by
  its own corpus per card 22-D, not by formatter coverage. R3 does not
  apply to S5 widening (implementation card 22-D, not a W2 S3/S4
  frontier step).
- Independent CI was not run for this slice at authoring time.
  M22 remains **NO-GO**: S5 is one widened slice of the RFC-0011 subset
  (statements, remaining fixed ops, while/branch/match constructs are
  still typed-refused), S6 and S7 are untouched.

| Check | Result |
|---|---|
| 14-fixture byte-equal codegen differential (`selfhost_compiler.rs`) | green, Windows x64 GNU real runner |
| inherited constant fixture (`sico_compiler_encodes_general_nonnegative_int_constant_core_wasm`) | green, byte-stable |
| 7 typed refusal fixtures + recovery | green |
| `tools/validate-step-0315.ps1` | exit 0 with the inherited STEP-0314→…→0262 chain green |
| `tools/report-m22-canary.ps1` | 30/30, full-source exit 0 (unchanged) |
| runner `cargo fmt -- --check` | exit 0 |
| independent CI | pending; GitHub unreachable from this host (2026-09-29), snapshot pushed to gitcode `origin` |

## Residual scope (not claimed)

S5 closes only when the whole RFC-0011 deterministic subset the Script
profile uses is byte-equal on its corpus and every out-of-subset shape is
typed-refused. Still refused after this slice: multi-statement bodies,
`let`/cell writes, while/if/match constructs, `checked_sub/mul/div`,
`EqualFixed`, bit operations, calls between guests functions, `Int`
(non-constant) semantics, and everything above the scalar subset. The
`.sapp`/M7 packaging, `A == B == C` bootstrap and budget measurements
(S6) have not been started.
