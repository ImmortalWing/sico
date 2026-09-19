# STEP-0221: M22 S6 — straight-line mutable `set` through local cells

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0220
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Support `set` reassignment in otherwise straight-line function bodies in
the Sico-written frontend, matching the Rust general-CFG lowering where
every `let` becomes a mutable local cell.

## 2. Contract and mechanism

- Rust routes bodies containing `set` (with no `if`/`while`/`match`) to
  `lower_general`: a single entry block whose block and function range is
  the whole declaration, one locals-table entry per `let` (range = the
  name token), a `write_local` per `let`/`set` (range = the full source
  line), and a fresh `read_local` per cell read (range = the name token).
  Value ids stay sequential across consts, reads, writes and operations.
- The Sico frontend now dispatches such bodies to
  `general_straight_function_ir` before the plain-SSA straight path.
  `general_expression` emits the declared expression subset with cell
  semantics: atoms (constants, parameters, cells), fixed-width
  literals, bit/checked operations (reusing the SSA operand checks) and
  typed user calls with per-argument validation.
- `argument_values`-style textual id resolution is not valid in the cell
  path, so the general path carries its own emission; `general_error`
  shaped results travel as a four-entry `List[Text]` (value, count,
  kind, json) because the script ABI refuses record-typed returns.
  `word_span_offset` learned to cross `<nl>` tokens by finding the
  newline byte; existing same-line walks are unaffected.
- Refusals stay typed: `set` without a prior `let` or targeting a
  parameter, duplicate cell names, `set` type mismatches, unresolved
  names, and non-scalar shapes outside the declared subset. `if`/`while`
  bodies keep the `ERR:E-SH-IR-CONTROL` refusal ahead of the dispatch.

## 3. Executable evidence

Seven new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`: set feeding a parameter-operand bit operation, a literal
operand bit operation with a later read, a user call inside `set`,
alias lets plus a second cell, a Result-typed cell reassigned by a
checked operation, a bool cell with a string cell (unused-local shape)
and the cumulative prior corpus. Five new typed refusals are pinned
with Rust refusing each on the general path.

The cumulative positive differential set is now fifty-nine programs.
The selfhost parser suite stays 2/2 green on the real Windows x64
runner; the Rust oracle suite is 15/15.

Reproducible validators:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --locked --offline -p sico-ir --all-targets -- -D warnings
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

General-CFG control flow (`if`/`while` regions, loop `break`/`continue`,
fall-through match arms) remains a typed refusal and is the next slice;
name shadowing of a parameter by a `let` resolves parameter-first in the
Sico frontend while Rust prefers the cell (both frontends lower the
declared subset byte-identically; shadowing fixtures are out of scope).
This step is not codegen, bootstrap closure or any UI support claim.
