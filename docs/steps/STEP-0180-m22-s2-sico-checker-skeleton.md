# STEP-0180: M22 S2 — Sico checker skeleton and differential harness

> - status: complete — **S2 opened with an honest partial: execution +
> well-formed differential green; the declaration-shape negative is the
> registered coverage gap**
> - phase: M22 S2 (tool self-host; the checker slice of L1)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/checker.sico`, `runner/sico-runner/tests/selfhost_checker.rs`, this STEP record

## 1. What was done

The second Sico-written tool exists and runs end to end:
`selfhost/checker.sico` reads a program from stdin, walks its lines, and
checks that declaration lines (`record`/`enum`/`function`/`module`/
`interface`/`capability`/`resource`/`use`/`export`) either end with `:`
or carry a `(` (call-shaped). It emits `check ok` when nothing looks
off, else a per-line report. Differential against `sico check`:

```text
well-formed:  Sico checker → "check ok";  sico check → "check ok"
missing colon: Sico checker → "check ok" (GAP);  sico check → E1013 refusal
```

The well-formed program is **byte-exact in verdict** (both accept). The
missing-colon negative exposes the concrete coverage gap: the Sico
checker accepts a `function` declaration whose header lacks the trailing
`:`, where the Rust checker's parser refuses it (E1013 family). The gap
is the declaration **colon-shape check** — the Sico walk needs to verify
the header's final non-space token is `:` (a last-token primitive, or a
"does the trimmed line end with X" walk composed over `text.length` +
`char_at`).

## 2. Validation

- `runner/sico-runner/tests/selfhost_checker.rs` 2/2: the checker compiles
  and runs; the well-formed differential matches (`check ok` both sides);
  the malformed case runs deterministically and is registered as the gap.
- No STEP-0148 frozen snapshot touched (new `selfhost/` program).

## 3. Honest coverage register

- The Sico checker's verdict is a **subset** of the Rust checker's (it
  accepts some programs the Rust one refuses). The M22 L1 exit gate is
  byte-exact differential across the frozen corpus; that gate is not yet
  met — the colon-shape primitive is the next slice (STEP-0181-sized),
  and the full-corpus differential lands with it.

## 4. Residuals

- Last-token / ends-with primitive (`text.ends_with` or composed
  `length` + `char_at`) closes the colon-shape gap.
- The fuller checker (name resolution, type shapes) is S4+ territory —
  this slice proves the tool runs and the harness measures real drift.
