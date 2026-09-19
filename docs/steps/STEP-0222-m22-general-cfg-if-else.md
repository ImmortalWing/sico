# STEP-0222: M22 S6 — general-CFG `if`/`else` regions

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0221
> - completed: 2026-09-19
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Support `if`/`else` regions in straight-line function bodies in the
Sico-written frontend, matching the Rust general-CFG lowering: branch
terminators, block creation/stamping order, empty-else trailing blocks
and nested-if block renumbering.

## 2. Contract and mechanism

- Rust `lower_general` lowers an `if` by emitting the condition into the
  current block, creating then/else/join blocks (all ranged to the `if`
  line), sealing a `Branch`, running the arms, sealing `Jump(join)`, and
  continuing in the join. An `else` arm without a body never becomes
  current: it trails the stamped blocks in creation order carrying only
  its `Jump`. Block ids are assigned by first-target order, so nested
  ifs interleave ids by processing order; the verifier's canonical-id
  rule is satisfied because emission order equals stamp order.
- The Sico frontend reimplements exactly this in
  `general_straight_function_ir`: nine parallel `List[Text]` block
  registers (instruction JSON, terminator kind/data, range, stamp),
  a continuation frame stack for open regions, an `else`/`end if`
  boundary handler, and a materialization pass that orders stamped
  blocks by stamp sequence and unstamped blocks by creation order before
  renumbering terminator targets. Region extents are found by a
  structural keyword scan (`if`/`while`/`for`/`match` depth with
  `else`/`end` boundaries), matching indentation-based extents on
  consistently indented sources.
- `general_expression` gains `I64/U64.equal` and `less_than` as
  `EqualFixed`/`LessFixed` comparisons yielding `Bool` (condition
  surface only in this step; the SSA return path still refuses them).
  A non-Bool condition, an unterminated region, a stray `else`/`end if`
  and a statement after a `return` in the same region remain typed
  refusals (`ERR:E-SH-IR-CONTROL`/`ERR:E-SH-IR-STATEMENT`).
- Functions whose bodies contain `set` or `if` (and no `while`/`for`)
  route to the general path; `for` bodies gain an explicit
  `ERR:E-SH-IR-CONTROL` dispatch refusal. Empty `locals` are omitted
  from the emitted JSON to match the Rust `skip_serializing_if` shape —
  this also corrects the prior cell-path emission for locals-free
  functions.

## 3. Executable evidence

Five new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`: an open `if` (empty else trailing as block 3 with a jump
to the renumbered join), an `if`/`else` whose both arms return (join
materializes `Unreachable`), `if`/`else` with cell writes and a
`LessFixed` condition plus trailing join code, a nested `if` inside an
`else` arm (seven blocks, interleaved ids), and a branch-scoped `let`
whose cell leaks to the join. Three new typed refusals are pinned with
Rust refusing each: a non-Bool condition, an unterminated region and a
mixed-width `equal` comparison. A standalone declared-subset refusal
pins `while` bodies (`Rust lowers them; the Sico subset refuses).

The cumulative positive differential set is now sixty-four programs.
The selfhost parser suite stays 2/2 green on the real Windows x64
runner; the Rust oracle suite is 16/16.

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

`while`/`for` loops, `break`/`continue`, fall-through match arms and
infix `==` conditions remain outside the frontend subset (typed
refusals or declared omissions); parenthesised conditions are refused.
Loop regions are the next lowering slice, then codegen (RFC-0011) and
the ADR-0015 bootstrap harness. This step is not codegen, bootstrap
closure or any UI support claim.
