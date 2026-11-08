# STEP-0608 — M22 W3-a whole-`formatter.sico` Component measurement (measurement-first)

- Date: 2026-11-08
- Track: M22 compiler self-host, goal **W3-a**（`AGENTS.md` §10.7 / `docs/plans/M22-compiler-self-host.md:314`）
- Slice type: **MEASUREMENT / diagnostic only** — no frozen artifact touched, no gate flipped, no parity claimed, nothing committed or pushed.
- Worktree HEAD at start: `90e37db`（`M22 match dispatcher slot cache: byte-identical pure speedup (3-7% fuel)`）.

## 1. Goal and honest scope

W3-a asks: the guest-compiled `selfhost/formatter.sico` Component must be **byte-equal** to the Rust production backend Component and pass `wasmparser::Validator`.

The **IR face is already green**: `sico_compiler_lowers_full_formatter_byte_exactly`（`runner/sico-runner/tests/selfhost_compiler.rs:1094`）compares `run_guest(FORMATTER_SOURCE)` canonical IR vs the Rust oracle for the WHOLE file. The **component face was not started**.

This slice **measures** the component face and answers one question: *does the guest component emitter currently accept the whole `formatter.sico`, and if not, what is the first named gap?* It does **not** implement any emitter change and does **not** claim W3-a.

## 2. Method

New measurement probe (the only file this slice adds):

- `runner/sico-runner/tests/selfhost_formatter_component_measurement.rs` — an `#[ignore]` test `step0608_measure_whole_formatter_component_emission`. Run with:
  ```powershell
  $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
  $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
  & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml `
    --test selfhost_formatter_component_measurement -- --ignored --exact --nocapture `
    step0608_measure_whole_formatter_component_emission
  ```

It measures two variants and a bisect ladder:

- **Variant A** — the real `selfhost/formatter.sico`（entry `main`）, oracle Component built through the real production CLI `sico build --profile script-v0`（mirrors `selfhost_formatter.rs::compile_formatter`, which performs the Script-profile `main`→`run` mapping in `prepare_script_module` before `compile_script_program`）.
- **Variant B** — the same source with the entry declaration renamed `main`→`run`, to isolate whether the entry-name mapping is the blocker; oracle lowers the already-`run` module straight through `sico_codegen_wasm::compile_script_program`.
- **Refusal driver** — an injection-style fixture (`refusal.sico` + sibling `codegen.sico`) that reads canonical IR from stdin and answers with the emitter's own `codegen.component_refusal_for_ir`, which **names** the first refusal instead of collapsing it into the CLI's generic `unsupported codegen source shape`. This is the accurate oracle for "which shape does the emitter refuse".
- **Bisect ladder** — single-feature `run`-body fixtures (ok-only / `bytes.is_utf8` if / `utf8_decode`+`text.encode` / error nested construct), each fed through the refusal driver.

Guest budget is the standard harness budget, **not raised**: fuel `800e9`, timeout `120s`, memory `2560 MiB`, hostcall_fuel `8 GiB`. Raw log: `target/tmp/step0608/formatter-component-measurement.log`.

## 3. Raw measurements

| quantity | value |
|---|---|
| `formatter.sico` source | 22639 bytes / 827 lines |
| Rust-oracle Component (A, `main` entry, via CLI) | **28459 bytes** |
| Rust-oracle Component (B, `run` entry, via backend) | **28459 bytes** |
| `oracle_size_delta_B_minus_A` | 0 |
| Guest `--emit-component-hex` (A and B) | typed refusal, 3.48 s wall |
| Guest fuel consumed (A/B whole-file, via `--emit-component-hex`) | ~2.19e8 of 800e9（**~0.03%**；远未触燃料墙） |
| Emitter refusal identity (whole file, both entries) | **`ERR:E-SH-CG-IR-EMIT`**（component-body emission gate） |

Per-variant detail (all under budget; none exhausted fuel; none timed out):

- `[A:main]` outcome = `Domain { invalid-input, "unsupported codegen source shape" }`, wall 3.484 s → byte_equal n/a (refused).
- `[B:run]` outcome = `Domain { invalid-input, "unsupported codegen source shape" }`, wall 3.484 s → byte_equal n/a (refused).

Control + bisect chain through the refusal driver (the decisive evidence):

| fixture | emitter verdict |
|---|---|
| echo `run` + Script records | **no-refusal**（emits; component_hex_chars=4） |
| echo records + `I64` helper + `run` | **no-refusal**（emits） |
| whole-formatter, `run` entry | **refusal `ERR:E-SH-CG-IR-EMIT`**（3.5 s） |
| whole-formatter, `main` entry | **refusal `ERR:E-SH-CG-IR-EMIT`**（3.4 s） |
| bisect: `run` ok-only | **no-refusal**（emits） |
| bisect: `run` + `bytes.is_utf8` if | **refusal `ERR:E-SH-CG-IR-EMIT`** |
| bisect: `run` + `utf8_decode` + `text.encode` | **refusal `ERR:E-SH-CG-IR-EMIT`** |
| bisect: `run` + error nested construct only | **no-refusal**（emits） |

## 4. Findings

1. **W3-a is not close — it is blocked at the first construct of the formatter entry body.** The guest component emitter refuses the whole `formatter.sico` with `ERR:E-SH-CG-IR-EMIT` under the standard budget. No byte comparison and no `wasmparser::Validator` run is possible yet, because the guest produces no component bytes.
2. **It is NOT the fuel wall.** Whole-file emission burns ~2.19e8 of 800e9 fuel (~0.03%) and completes in ~3.5 s with a typed refusal — the same order as the small fixtures. The 331-site `parser.sico` fuel wall is orthogonal and does not block `formatter.sico`（as the task background predicted）.
3. **It is NOT the `main`→`run` entry-name mapping.** Renaming the entry to `run` still refuses identically, and both oracle Components are the same 28459 bytes. The CLI's `prepare_script_module` rename is not the blocker for the component face.
4. **The named gap is `sico.bytes.is_utf8` used as an `if`-condition in the entry body.** The bisect isolates it cleanly: `run` ok-only emits; adding `if sico.bytes.is_utf8(input.stdin):` flips it to `ERR:E-SH-CG-IR-EMIT`; `utf8_decode`+`text.encode`（same `is_utf8` gate）also refuses; a bare nested `error(...)` construct emits. The formatter `main` is gated **entirely** on `if sico.bytes.is_utf8(input.stdin):`, so this one shape blocks whole-file component emission.
5. **The emitter is otherwise healthy on Script-profile scaffolding.** Records (`ScriptInput`/`ScriptOutput`/`ScriptError`), a preceding `I64` helper function, the `ok` arm, and a nested `error(...)` construct all emit through the refusal driver. The gap is the intrinsic-in-condition shape, not Script ABI or multi-function structure.

## 5. Honest scope / what was NOT run

- **No component byte-equality and no validator result for the whole formatter** — the guest refuses before emitting, so there are no guest component bytes to compare. Byte-equality and `wasmparser::Validator` remain **unmeasured** for W3-a.
- **No emitter/frontend/codegen change was made.** `selfhost/parser.sico`, `selfhost/codegen.sico`, `selfhost/formatter.sico` and every frozen artifact are untouched (verified: `git diff --stat` on all three is empty). Per the binding discipline, an emitter change is required to close the gap and is left to a follow-up implementation slice **after owner/coordinator review**.
- **Two bisect fixtures could not reach the emitter at all** — they were refused by the guest **frontend** IR lowering, not the component emitter: `run`+user-call-in-if-condition → `ERR:E-SH-IR-CALL-TARGET`/`ERR:E-SH-IR-CALL-SHAPE`, and the `formatter run only`+records fixture → `ERR:E-SH-IR-VARIANT-PAYLOAD`. These are artifacts of my hand-written minimal fixtures (arity/shape not matching the Script profile), **not** statements about the whole formatter. The whole-file IR lowering is already green on the existing test, so the frontend is not the W3-a blocker.
- **Not run**: full `selfhost_compiler`/`selfhost_formatter` suites, `run-ci.ps1`, the M22 battery, `git diff --check` beyond the new file. This is a single-probe measurement slice; only the new `#[ignore]` test was compiled and run (`--exact`).
- **Concurrent worktree note**: `runner/sico-runner/tests/selfhost_compiler.rs` shows a `TEMP-STEP0609-PROBE` addition from the concurrent STEP-0609 agent. That is **not** this slice's change and was left untouched.

## 6. Recommendation for the follow-up implementation slice

Close the `sico.bytes.is_utf8`-as-`if`-condition gap in the guest component emitter (the `ERR:E-SH-CG-IR-EMIT` arm), then re-run this probe: the expected next blocker (if any) will surface as a new named identity, and once emission completes the probe already compares bytes and runs `wasmparser::Validator`. Because the whole-file fuel cost is ~0.03% of budget, once the shape gap closes the byte-equality comparison should be reachable without touching any limit (owner D-B: no limit raises).

## 7. Gate status

**No gate flipped. M22 overall remains NO-GO.** W3-a remains open; this slice only measures and names the first gap.
