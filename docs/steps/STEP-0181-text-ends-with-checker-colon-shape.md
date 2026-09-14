# STEP-0181: text.ends_with + checker colon-shape check — negative differential green

> - status: complete — **the Sico checker now matches the Rust checker's
> refusal on the missing-colon negative case**
> - phase: M22 S2 continuation (closes the STEP-0180 coverage gap)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/checker.sico`, `crates/sico-{ir,semantics,codegen-wasm}/src/lib.rs`, `runner/sico-runner/tests/selfhost_checker.rs`, this STEP record

## 1. What was done

**`sico.text.ends_with(Text, Text) -> Bool`** — the suffix-comparison
primitive the checker's colon-shape check needed. Emitted as a reverse-scan
helper (`emit_ends_with`): needle-longer-than-haystack returns 0, else the
cursor walks both strings from the end. Registered across the IR signature
table, the semantics check table, the codegen emission arm, and
`INTRINSIC_HELPERS` + `helper_dependencies`.

**Checker colon-shape check** — `selfhost/checker.sico` now verifies that
`function` headers and the block-opener declarations (`record`/`enum`/
`module`/`interface`) end with `:`, using `sico.text.ends_with`. The
missing-colon negative case now reports `line 0: function main() returns
Int`, matching the Rust checker's refusal verdict on the same input (the
Rust side refuses with E1013; the Sico side flags the same line — the
verdicts agree the program is not well-formed).

## 2. Validation

- `runner/sico-runner/tests/selfhost_checker.rs` 2/2: well-formed
  differential green (`check ok` both sides); negative differential green
  (Sico checker reports the missing-colon line; Rust checker refuses).
- fmt + both clippy workspaces green; `git diff --check` clean. Full
  `run-ci.ps1` regression: CI GREEN.

## 3. Honest register

- The Sico checker's diagnostic text (`line 0: ...`) differs from the
  Rust checker's E1013 rendering — the verdicts agree (not well-formed),
  the rendering differs. Byte-exact diagnostic rendering parity is a later
  slice (the E1xxx text formatter in Sico is S4+ territory).
- The full frozen-corpus differential (every `syntax-candidates/` and
  `semantic-cases/` file) remains the L1 exit gate; this slice closes one
  concrete gap on the path to it.
