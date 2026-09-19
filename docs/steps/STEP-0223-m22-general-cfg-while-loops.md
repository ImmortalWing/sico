# STEP-0223: M22 S6 — general-CFG `while`/`break`/`continue` loops

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0222
> - completed: 2026-09-19
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Support `while` loops with `break`/`continue` in straight-line function
bodies in the Sico-written frontend, matching the Rust general-CFG
lowering: loop header blocks carrying the condition branch, body blocks
with back edges, after blocks, and break/continue jump targets.

## 2. Contract and mechanism

- Rust `lower_general` lowers a `while` by creating a header block,
  sealing the current block with `Jump(header)`, emitting the (Bool)
  condition into the header, branching header -> body/after, running the
  body, sealing `Jump(header)` (the back edge — only when the body does
  not end in `return`/`break`/`continue`), and continuing in `after`.
  `break`/`continue` seal `Jump(after)`/`Jump(header)` against the
  innermost loop frame; outside a loop they are unsupported.
- The Sico frontend extends the general path: frames now carry a kind
  (`if`/`while`) with per-kind fields; the `else`/`end if`/`end while`
  boundaries validate the frame kind; `break`/`continue` scan the frame
  stack for the nearest loop. Region extents come from the structural
  keyword scan (loop depth counts nested `if`/`while`/`for`/`match`).
  Block creation order (header, body, after) matches Rust, so the
  first-target stamp order renumbers identically.
- Dispatch: bodies containing `while`, `break` or `continue` (besides
  `set`/`if`) route to the general path; `for` and `match` bodies stay
  typed refusals. Statements after `break`/`continue`/`return` in the
  same region remain `ERR:E-SH-IR-STATEMENT` refusals (Rust lowers them
  into a fresh dead block — declared disjoint subset). Non-Bool loop
  conditions, `break`/`continue` outside loops, and unterminated regions
  stay typed refusals.
- Fuel budget (M22 gate 4 registration): the word-oriented frontend
  costs ~1M fuel per general-CFG statement on the default 10M runner
  budget (measured minimums: trivial SSA fixture 0.7M, straight-cell
  fixture 3.7M, single-while 5.1M, nested-while 10.2M). The differential
  harness raises its budget to 64M (recorded in the test) so fixtures
  run to completion; the regression is registered per the M22 plan §3
  slice rule and re-measured at the S6 bootstrap gate. Two
  implementation-level mitigations landed en route: an amortized
  line-span cursor (replacing the per-line function-anchored walk) and
  `is_word` switched from double `text.encode` to `text.compare`.

## 3. Executable evidence

Five new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`: a counting loop (header branch, body back edge, after
read), `break` (jump to after, suppressed back edge), `continue` (jump
to header), nested loops (seven blocks, interleaved ids with the inner
after continuing the outer body), and a loop with an `if`+`break`
guard. Three new typed refusals are pinned with Rust refusing each:
`break` outside a loop, `continue` outside a loop, and a non-Bool loop
condition. One standalone declared-subset refusal pins statements after
`break` in the same region (Rust lowers them into a dead block).

The cumulative positive differential set is now sixty-nine programs.
The selfhost parser suite stays 2/2 green on the real Windows x64
runner; the Rust oracle suite is 17/17.

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

`for` loops, fall-through match arms and infix `==` conditions remain
outside the frontend subset (typed refusals / declared omissions). The
frontend fuel constant (~1M per general-CFG statement) is registered as
a budget item and must be re-measured at the S6 bootstrap closure and
codegen slices. This step is not codegen, bootstrap closure or any UI
support claim.
