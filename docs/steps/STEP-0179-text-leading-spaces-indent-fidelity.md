# STEP-0179: text.leading_spaces + Sico formatter byte-exact differential

> - status: complete — **S1.5 closed: the Sico formatter now matches
> `sico format` byte-for-byte on the simple corpus**
> - phase: M22 S1.5 (the primitive slice that closes the STEP-0178 gap)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/formatter.sico`, `crates/sico-{ir,semantics,codegen-wasm}/src/lib.rs`, this STEP record

## 1. What was done

**`sico.text.leading_spaces(Text) -> U64`** — the leading-whitespace
measurement primitive RFC-0045 D1 queued. Emitted inline (no helper): a
branch-free scan for the first byte that is neither space (0x20) nor tab
(0x09), returning the count as one i64 U64 slot. Registered across the IR
signature table, the semantics check table, and the codegen emission arm
(`INTRINSIC_HELPERS` entry + `helper_dependencies`).

**Indentation-faithful Sico formatter** — `selfhost/formatter.sico` now
preserves the measured leading whitespace (halved to canonical two-space
steps, matching the Rust formatter's behaviour), trims trailing
whitespace, and re-emits each line. The differential against `sico
format` is **byte-exact** on the probe corpus:

```text
$ diff <(sico run selfhost/formatter.sico < in.txt) <(sico format in.txt)
SICO_FMT_MATCHES_RUST
```

## 2. Validation

- `sico.text.leading_spaces` returns 4 for `"    return 1"`, 0 for
  top-level lines (debug-instrumented and confirmed during development).
- The formatter's `normalize_line` + `emit_indent` chain produces
  byte-identical output to `sico format` on the three-line probe
  (`function f() returns Int:` / `    return 1` / `end function`).
- fmt + both clippy workspaces green; `git diff --check` clean. Full
  `run-ci.ps1` regression: see the STEP-0179 closure note in STATUS.

## 3. Honest register

- The probe corpus is three lines; the full frozen-corpus differential
  (`syntax-candidates/`, `semantic-cases/`) across every construct is the
  L1 exit gate and lands with the S2 checker slice (the formatter's
  coverage grows with the corpus the self-host frontend can parse).
- `U64.greater_than_or_equal` is not a registered predicate; the
  formatter composes `U64.checked_sub` + match (the `>= 2` test) instead
  — a faithful composition, no language gap.
