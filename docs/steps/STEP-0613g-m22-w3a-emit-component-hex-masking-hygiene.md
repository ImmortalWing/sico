# STEP-0613g — M22 W3-a: `--emit-component-hex` masking hygiene — surface the specific frontend error code

- Date: 2026-11-09
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (dispatched sub-agent implemented; coordinator verified)
- Status: implementation complete + verified locally
- Evidence level: internal-fixture (deterministic MSVC build + guest driver pins)

## 1. Why this slice

`STEP-0613f` established that the driver message
`"unsupported codegen source shape"` had been **masking a frontend refusal** —
the co-gap fixtures are refused in the frontend with `ERR:E-SH-IR-CONSTRUCT-VALUE`,
but `--emit-component-hex` collapsed any `cch_ir` beginning with `ERR:` into the
generic message. That masking is why the frontend gap stayed invisible across
every prior driver run and had to be recovered with a `component_refusal_for_ir`
probe.

This slice removes the masking at its source so the **next** frontend gap is
visible immediately in normal driver runs, without a probe.

## 2. Change

`selfhost/compiler.sico`, one line, inside the frontend `ERR:` guard of the
`--emit-component-hex` branch (`compiler.sico:881`):

```diff
       let cch_ir = parser.scalar_ir(cch_words, input.stdin)
       if sico.text.starts_with(cch_ir, "ERR:"):
-        return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "unsupported codegen source shape"))
+        return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: cch_ir))
       end if
```

Scope discipline:

- The **emitter** empty-length guard (`component_hex_for_ir` returning `""`,
  `compiler.sico:885`) is **unchanged** — it is a genuine generic emitter
  refusal (`cch_ir` is valid IR but the shape is outside the frozen emitter), so
  it correctly keeps the generic message.
- This mirrors the sibling `--emit-ir-split` branch, which already surfaced the
  raw code via `message: split_ir`.
- `parser.sico` and `codegen.sico` were **not** touched.

## 3. Effect (verified)

After the change the co-gap diagnostic probe
(`m22_w3a_cogap_refusal_identity`, run with `--ignored`) reports, for F1/F2/F3:

```
outcome = Domain code="invalid-input" message="ERR:E-SH-IR-CONSTRUCT-VALUE" (typed refusal)
```

The specific frontend code now flows through the normal `--emit-component-hex`
driver path instead of the generic mask. The bisect section (B0–B6) still
reports the same identities via `component_refusal_for_ir`, consistent with
`STEP-0613f`. Log: `target/tmp/step0613f/cogap-refusal-identity.log`.

## 4. Fail-closed safety check

A concern was whether any test pins the old masked message on the frontend
`ERR:` path. The relevant fail-closed test
`sico_codegen_guest_emits_component_bytes_byte_exactly`
(`selfhost_compiler.rs:3036-3042`) asserts `IDENTITY_SOURCE` (a plain
`identity(value: Int)` function) still returns the generic message. Verified:
`IDENTITY_SOURCE` is accepted by the frontend (no `ERR:` code) and refused only
by the emitter's empty-length guard, which is unchanged — so the test still
passes. **Coordinator independently re-ran it: PASS.** No pin asserted the old
masked frontend message; none broke.

## 5. Validation (MSVC, `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-msvc`)

- Build: `build_g1.bat` → `BUILD_EXIT=0`.
- 7 G1 capability pins: all green (multiblock run / `is_utf8` gate / echo
  instruction segment / run body / `step0507` / `step0513` / `step0526`).
- `sico_codegen_guest_emits_component_bytes_byte_exactly` (fail-closed,
  `IDENTITY_SOURCE`): PASS (coordinator re-run).
- `cargo fmt --check` (runner workspace): clean.
- `git diff --check`: clean.
- Ledger scope: the edit only risks the 3 `compiler.sico` match sites
  (MS-003/004/005), all in `lex_words`/`add_u64` near lines 56–120, far from the
  edit at `:881`; an in-place message change shifts no line numbers or heads, and
  `frozen_match_site_ledger` is already red at MS-024 (test stops at first
  mismatch), so this cannot make it more red. Known-pre-existing, unrelated.

## 6. Not done here / next

- This is masking hygiene only. The **underlying** frontend gap
  (`ERR:E-SH-IR-CONSTRUCT-VALUE` from `parser.sico:construct_field_value_ir`) is
  **not** fixed — it is now merely *visible*. Fixing it is the STEP-0613f §8
  coordinated slice (construct field-value lowering + ledger re-freeze, needs
  Python, owner-scoped).
- `compiler.sico` change shifts no frozen corpus coverage (`corpus-v0.json` does
  not track selfhost files). No ledger re-freeze needed for this slice.

## 7. Files

- `selfhost/compiler.sico` — one-line masking fix (`:881`).
- `docs/steps/STEP-0613g-m22-w3a-emit-component-hex-masking-hygiene.md` — this record.
- `docs/steps/README.md` — STEP index row.
