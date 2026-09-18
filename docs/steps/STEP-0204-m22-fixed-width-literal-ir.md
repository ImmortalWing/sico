# STEP-0204: M22 S6 — bounded fixed-width literal IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0203
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Extend the verifier-accepted, Rust-identical self-host lowering slice with
the fixed-width literal forms used throughout the compiler's own Sico
source: `I64.literal(...)` and `U64.literal(...)`.

## 2. Contract and mechanism

- The accepted shapes are exactly one canonical decimal argument to
  `I64.literal` or `U64.literal`; only `I64.literal` accepts a leading
  minus token.
- Decimal magnitudes are compared lexically after the existing canonical
  magnitude check. The closed limits are `i64::MIN..=i64::MAX` and
  `0..=u64::MAX`; limit+1 inputs return stable typed refusals before any
  IR bytes are emitted.
- Return types map to canonical IR `i64` and `u64`; operations are
  `const_i64` and `const_u64` with JSON numeric payloads.
- Rust lowering attaches the operation range to the literal argument,
  not to the enclosing conversion call. The Sico frontend reproduces
  that range identity, including the minus token for signed negatives.
- No new Sico language surface or backend behavior is introduced.

## 3. Executable evidence

The cumulative real-runner lowering test now covers nine positive
Rust byte differentials. STEP-0204 adds:

1. `I64.literal(9223372036854775807)`;
2. `I64.literal(-9223372036854775808)`;
3. `U64.literal(18446744073709551615)`.

Each guest output deserializes as `sico_ir::Module`, passes the independent
verifier, round-trips through canonical JSON and is byte-identical to Rust
`lower_core`, including source ranges. Two additional limit+1 cases prove
both Rust refusal and guest `invalid-input` refusal:

- `I64.literal(9223372036854775808)` → `ERR:E-SH-IR-I64-RANGE`;
- `U64.literal(18446744073709551616)` → `ERR:E-SH-IR-U64-RANGE`.

The full selfhost parser suite remains 2/2 green on the real Windows x64
runner.

## 4. Residuals

Multiple parameters/functions, user calls, fixed-width operations, general
blocks and complete token spans remain open. Literal support alone is not
claimed as compiler self-host closure.
