# STEP-0195: M22 S6 — exact IR signatures (punctuation tokens, depth-counted arity, `end function` context)

> - status: complete — **the parser emits exact IR signatures on a
> two-function corpus: `fn:main/0;fn:add/2;`, closing the STEP-0194
> residual**
> - phase: M22 S6 (lower slice, continued from STEP-0194)
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. Objective

Fix the registered STEP-0194 residual: the param count under-reported
multi-parameter functions (`fn:add/0` instead of `fn:add/2`), so the
parser's declaration summary is not yet the exact function identity the
typed IR carries.

## 2. Root cause — registered correction to STEP-0194

STEP-0194 registered the defect as "comma counting under-counts by one".
The executable evidence is different and stronger: `parser.sico`'s
`lex_words` only captured ident-classified byte runs, so **punctuation
never entered the token stream at all** — `count_params` never saw `(`,
`,` or `)`, `seen_open` stayed 0, and every signature returned 0. The
comma under-count theory was unreachable because there were no commas to
count. Recorded here per the documentation-vs-evidence rule; STEP-0194's
`fn:main/0` output was correct only because the empty parameter list and
the missing tokens produced the same answer.

## 3. Changes

- `lex_words` now emits single-character punctuation tokens `(` `)` `,`
  `:` `[` `]` (byte-exact 1-byte words) and classifies digits as
  identifier bytes, so `I64` stays one word and integer literals enter
  the stream. Two-character operators (`==` `<=`) are still NOT emitted
  as single tokens — declared pending the expression slices.
- `count_params` is a depth-state walk: inside a depth-1 `(`…`)` span it
  counts top-level commas and any non-separator token; an empty list
  yields 0, a non-empty list yields commas+1. No `returns`-scan fallback
  (a missing `)` now yields 0 — typed parse refusals come with the
  checker slices).
- `summarize` tracks the previous word: `function` after `end` is the
  terminator pair, not a new declaration. Without this, the two-function
  corpus produced a phantom `fn:function/…` entry between the two real
  signatures. Entries are now `;`-separated (`fn:main/0;fn:add/2;`).

## 4. Validation

- `runner/sico-runner/tests/selfhost_parser.rs` (Windows x64, real
  `sico build --profile script-v0` + real `sico-runner` Wasmtime
  execution): corpus extended to two functions; expected
  `fn:main/0;fn:add/2;` byte-exact — 1/1 ok.
- Internal trace registered: `main()` → 0 (open+close, no tokens);
  `add(a: I64, b: I64)` → 1 comma + non-empty → 2.

## 5. Environment register (fresh host)

This step was executed on a rebuilt host: the previous Windows profile
(owning `E:/github/sico`) no longer exists. Rebuilt from scratch:
rustup + pinned `1.98.0-x86_64-pc-windows-gnu` (minimal + clippy/rustfmt),
`cargo fetch --locked` both workspaces, wasmtime v46.0.1 already present
under `target/tooling` (sha-pinned by `tools/ensure-wasmtime.ps1`), and
the MSYS2 binutils at `target/tooling/msys2-binutils/mingw64/bin`
prepending to `PATH` for the GNU `dlltool` (the mechanism
`tools/validate-step-0034.ps1` already automates; `run-ci.ps1` assumes
it on `PATH`). Registered finding, not fixed here: `run-ci.ps1` runs
`test (workspace)` before `test (runner)`, but
`crates/sico-cli/tests/packages_resolve.rs` falls back to
`runner/sico-runner/target/debug/sico-runner.exe` — on a truly fresh
clone those two tests fail until the runner workspace has built once
(observed; exit 121 `cannot read component`). The documented
console-control tests (`runner_cli_observes_real_windows_console_control`,
`runner_watch_observes_console_control_while_idle`) require a real
Windows console and fail in a non-TTY harness shell; prior audits
recorded the same environment sensitivity with isolated re-runs green.

## 6. Residuals (the S6 completion boundary, unchanged)

- Full **lowering to typed IR**: expressions, blocks, control flow —
  verified against the Rust IR verifier (the M22 plan's oracle).
- **Codegen**: wasm emission from the IR, byte-exact vs the Rust backend
  on the frozen corpus (RFC-0011).
- Bootstrap closure (S6's exit): the Sico compiler compiles its own
  source; the bootstrap architecture ADR is the plan-level prerequisite.
- Selfhost index note: `docs/steps/README.md`'s table stopped at
  STEP-0130 (pre-existing); STATUS.md remains the live tracker.
