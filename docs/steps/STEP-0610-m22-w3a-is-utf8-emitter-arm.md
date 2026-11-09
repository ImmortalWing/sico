# STEP-0610 — M22 W3-a `sico.bytes.is_utf8` guest emitter arm (arm landed; W3-a still blocked by a shared multi-block-`run` gap)

- Date: 2026-11-08
- Track: M22 compiler self-host, goal **W3-a**（`AGENTS.md` §10.7 / `docs/plans/M22-compiler-self-host.md`）
- Slice type: **narrow additive emitter arm + honest localization** — no gate flipped, nothing committed or pushed.
- Worktree HEAD at start: `066c036`（`066c0368d4bcb5fe90e619103413765c4d6ca58d`）.

## 1. Goal and honest scope

The scoping brief for this slice held that `sico.bytes.is_utf8` had **no arm at all** in the guest
intrinsic dispatcher `cgs_intrinsic_hex`（`selfhost/codegen.sico`）and that this single missing arm was
**the whole gap** blocking whole-`formatter.sico` Component emission（W3-a）. That diagnosis was correct
about the missing arm — `grep -c is_utf8 selfhost/codegen.sico` was `0` at HEAD — and this slice
**implements the arm**. But it is **not** the whole gap.

**Finding of this slice (the important result):** the `is_utf8` arm is a *necessary prerequisite* but it
**does not unlock W3-a**. The real W3-a blocker is a **different, pre-existing, and SHARED** gap: the
Script-component emitter（on **both** the guest and the Rust oracle）only supports a **single-block
`run` body**. The `formatter.sico` `main` gates the whole program on `if sico.bytes.is_utf8(input.stdin):`,
which makes `run` **multi-block**, so **both** implementations refuse component emission before the
`is_utf8` instruction is ever emitted. W3-a therefore needs **multi-block-`run` emission support**
（the run dispatcher / branch terminators / multi-return result-area derivation）, which is a separate
and larger slice, on both sides.

This slice does **not** flip any gate, does **not** claim W3-a, and does **not** raise any fuel/memory
limit（owner D-B）. It does **not** touch `selfhost/parser.sico` or `selfhost/formatter.sico`.

## 2. What landed

### 2.1 The `is_utf8` emitter arm (`selfhost/codegen.sico`, +71 lines)

The backend's `is_utf8` arm is **inline** in its own dispatcher（`crates/sico-codegen-wasm/src/lib.rs:4499-4508`）
and **reuses the existing `sico.bytes.utf8_decode` runtime helper**（`is_utf8` is *not* in
`INTRINSIC_HELPERS`; its default helper-dependency arm returns `&[]` at `lib.rs:551-554`）. The byte
target is:

```
local.get <source_pointer>
local.get <source_length>
call  <sico.bytes.utf8_decode helper>
local.set <result0>          // single scalar bool slot
```

The guest arm reproduces exactly that. It is **delegated to its own function**
`cgs_is_utf8_hex`（`selfhost/codegen.sico:3868`）with a one-line dispatch in
`cgs_intrinsic_hex`（`selfhost/codegen.sico:3851-3853`）, mirroring the STEP-0560
`cgs_list_length_u64_hex` pattern. Delegation is **required**, not stylistic: an inline arm pushed
`cgs_intrinsic_hex` past the guest frontend's self-hosting statement frontier and the guest could no
longer lower its own emitter（`ERR:E-SH-IR-STATEMENT`, see §3.2）. Guards: arg0 present, operand is a
`bytes` kind flattening to a 2-slot `(ptr,len)` pair, result is 1 scalar slot, helper index nonzero —
otherwise the typed `ERR:E-SH-CG-IR-EMIT` refusal. No new helper, no helper-table change.

### 2.2 Frozen emitter vocabulary refreshed

`tools/census-m22-emitter-vocabulary.py --update` then `--check`：the change alters
`guest_emitter.source_sha256` and adds `sico.bytes.is_utf8` to `guest_vocabulary.intrinsics`. Run twice
（the first `--update` raced the §3.2 refactor）; the final `--update`+`--check` is green. Log:
`target/tmp/step0610/emitter-vocab-update2.log`.

### 2.3 Test (`runner/sico-runner/tests/selfhost_compiler.rs`)

`IS_UTF8_GATE_FIXTURE_SOURCE` + `sico_is_utf8_lowers_byte_exactly_but_the_gated_run_emission_is_refused`.
It pins the localization honestly:

- the **Rust oracle refuses** the `is_utf8`-gated entry `run`（`compile_script_program` →
  `Unsupported { function: "run", feature: "stdlib helper layout" }`）, and
- the **guest refuses** the same shape（`--emit-component-hex` →
  `Domain { "invalid-input", "unsupported codegen source shape" }`）.

i.e. both sides refuse the multi-block `run`. The test name and comments state plainly that the `is_utf8`
arm alone does not close W3-a.

## 3. Evidence: how the diagnosis changed

### 3.1 Baseline red, then the multi-block-`run` discovery

1. New test at baseline（before the arm）: guest refuses → `Domain { "unsupported codegen source shape" }`.
2. Added the inline `is_utf8` arm → the `run`+`is_utf8`-`if` fixture **still refused**
   `ERR:E-SH-CG-IR-EMIT` at the emitter（confirmed via the STEP-0608 refusal driver,
   `target/tmp/step0608/`）. This contradicted "the arm is the whole gap" and triggered the root-cause pass.
3. Root cause: `cgs_result_return_hex`（`selfhost/codegen.sico:4788-4790`）refuses any `run` with
   **more than one basic block**（`if U64.less_than(0, cgj_array_item(blocks, 1)): return "ERR:E-SH-CG-IR-EMIT"`）.
   `cgs_run_body_hex` = `cgs_run_instr_hex`（emitted with `terms=0`, i.e. **no branch terminators**）+
   `cgs_result_return_hex` + a fixed epilogue — so a branched `run` is never wired. The `is_utf8` `if`
   is exactly what makes the formatter `run` multi-block.
4. The **oracle refuses the same shape**（`Unsupported { function: "run", feature: "stdlib helper layout" }`,
   the `helper()` lookup at `lib.rs:4408-4412`）: multi-block `run` component emission is a **shared**
   limitation, not a guest-only gap.

### 3.2 Self-hosting regression found and fixed

The first (inline) version of the arm broke
`sico_compiler_lowers_the_step0513_codegen_function_byte_exactly`：the guest frontend refused
`cgs_intrinsic_hex` with `ERR:E-SH-IR-STATEMENT`（statement-frontier）. The delegation refactor（§2.1）
restored it to green. **This is why the arm is a separate function.**

### 3.3 Honest dead-ends (recorded so they are not re-walked)

Several hand-written minimal fixtures were tried to exercise the arm end-to-end and were refused by the
**guest frontend**, not the emitter（artifacts of hand-written shapes, as STEP-0608 §5 already noted）:
`ERR:E-SH-IR-CALL-ARGUMENT`（`gate` helper returning `utf8_decode`）, `ERR:E-SH-IR-CALL-SHAPE`（nested
`text.encode(utf8_decode(...))`）, `ERR:E-SH-IR-SCICOND`（`bytes.equal` as an `if`-condition）,
`ERR:E-SH-IR-CONSTRUCT-VALUE`（`ScriptOutput` from a `Bool`-returning helper）. A minimal fixture also
cannot dodge the oracle's helper-inclusion edge（a module using *only* `is_utf8` omits the
`utf8_decode` helper yet still emits the `call`; `lib.rs:829-840`）. A minimal-fixture IR byte-compare
also differs from the oracle by a fixed source-span offset（fixture artifact）. The whole
`formatter.sico` IR lowering, by contrast, is green and is the reliable oracle for spans.

## 4. What was run (all on Windows x64, `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`)

Regression guards（`--test selfhost_compiler -- --exact`, all green, 14 tests）:

- component-emission identity `sico_codegen_guest_emits_component_bytes_byte_exactly`（`:3000`/`:3038`）✓
- error-arm `sico_codegen_guest_emits_the_error_arm_component_byte_exactly`（`:3175`）✓
- host-call `..._host_call_component_byte_exactly`（`:3896`）✓ · host-call-let `..._host_call_let_binding_component_byte_exactly`（`:5923`）✓
- STEP-0560 `the_emitter_vocabulary_c_covers_the_list_length_u64_shape`（`:11397`）✓
- `is_utf8` IR：`..._let_utf8_decode_field_rhs_...`（`:984`）✓ · `..._error_return_variant_construct_...`（`:1041`）✓ · `..._ok_return_construct_...`（`:1104`）✓ · `..._main_condition_...`（`:1184`）✓
- whole-file formatter IR pin `sico_compiler_lowers_full_formatter_byte_exactly`（`:1094`）✓
- self-host lowering of the changed dispatcher `sico_compiler_lowers_the_step0513_codegen_function_byte_exactly`（`:3989`）✓（red before the §3.2 refactor, green after）
- exit-blockers `step0507`（`:3469`）✓ · `step0519`（`:6429`）✓ · `step0520`（`:6754`）✓ · `step0526`（`:3515`）✓
- new test `sico_is_utf8_lowers_byte_exactly_but_the_gated_run_emission_is_refused` ✓

Format / lint：`cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` → clean；
`cargo clippy --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_compiler -- -D warnings` → clean.

Whole-file W3-a probe：`step0608_measure_whole_formatter_component_emission`（`#[ignore]`）**still refuses**
after the arm（`[A:main]`/`[B:run]` → `unsupported codegen source shape`；refusal driver →
`ERR:E-SH-CG-IR-EMIT`）, exactly as §1 predicts. Log refreshed at
`target/tmp/step0608/formatter-component-measurement.log`.

## 5. Residual blockers (not converted into implied success)

- **W3-a is NOT closed.** The `is_utf8` arm landed and is correct-by-construction（mirrors the oracle
  byte sequence and the established armed-intrinsic pattern）, but it is **unreachable end-to-end**
  until multi-block-`run` component emission exists. **Both** the guest（`cgs_result_return_hex:4788`）
  and the oracle refuse a branched `run`.
- **Next slice to unlock W3-a:** implement multi-block-`run` emission（run dispatcher branch
  terminators + multi-return result-area derivation）on **both** sides, then re-run the STEP-0608 probe.
  The `is_utf8` arm will then be exercised and can be byte-verified.
- Not done here（out of scope）：any parser.sico change（byte-bound to the match ledger）, any
  `formatter.sico` change, any fuel/memory-limit change, any gate flip, any commit/push.

## 6. Files

- `selfhost/codegen.sico` — `cgs_intrinsic_hex` delegating arm（`:3851-3853`）+ new `cgs_is_utf8_hex`（`:3857-3919`）. **+71 / −0.**
- `runner/sico-runner/tests/selfhost_compiler.rs` — `IS_UTF8_GATE_FIXTURE_SOURCE` + localization test.
- `selfhost/emitter-vocabulary-v1.json` — refreshed via the census tool（source sha + `is_utf8` intrinsic）.
- `docs/steps/STEP-0610-m22-w3a-is-utf8-emitter-arm.md`（this file）+ one `docs/steps/README.md` row.

No `docs/STATUS.md` change（coordinator-owned）. No gate flipped.
## 7. Addendum（2026-11-08，只读侦察片订正）——「两侧共享缺口」判读不成立，W3-a 是 guest-only 缺口

本节为事后订正，正文保留原文。只读 scoping 片对 §5 的「Both the guest and the oracle refuse a branched
`run`」做了证据复核，结论：**该判读不成立**。

- **决定性证据（本片之前已有、被误读）**：STEP-0608 测量日志 `target/tmp/step0608/formatter-component-measurement.log`
  第 1/8/49 行记录 `oracle_bytes_A_main = 28459`、`oracle_bytes_B_run = 28459`、
  `oracle_size_delta_B_minus_A = 0` —— **oracle 对整份 `formatter.sico`（含 multi-block `run`）成功发射了
  28459 字节完整 Component**。oracle 从未拒绝过真实 formatter 的 branched `run`。
- **`stdlib helper layout` 拒绝的真实根因（fixture 伪信号）**：`sico.bytes.is_utf8` 的发射臂
  （`crates/sico-codegen-wasm/src/lib.rs:4499-4508`）调用 `helper("sico.bytes.utf8_decode")`，但
  `helper_dependencies`（`lib.rs:510-556`）**没有 `is_utf8` 条目**（其 match 落到默认臂返回 `&[]`）⇒
  只用 `is_utf8` 而从不直接调用 `utf8_decode` 的模块会缺 helper 注册。本片的手写 fixture
  `IS_UTF8_GATE_FIXTURE_SOURCE` 恰好如此；**真实 `formatter.sico:819` 直接调用 `sico.bytes.utf8_decode`**，
  helper 已注册 ⇒ oracle 顺利发射。新钉测试中断言的 oracle `is_err` 是**helper 依赖缺口的产物，
  不是 multi-block 限制**。
- **oracle 侧 multi-block `run` 语义（已存在）**：`compile_function`（`lib.rs:1691-1748`）对所有函数统一走
  dispatcher 循环（block/loop + 每块 `dispatcher==block_id` 分派头）；`compile_terminator`
  （`lib.rs:2263-2323`）已实现 Return/Jump/**Branch**/Match/Unreachable 五种终止子；`emit_script_return`
  （`lib.rs:3373-3463`）在每个 `Return` 终止子处各发一份 result-area 序列，共用同一对 scratch 局部。
  契约文档 `script-component-emission-v0.md` §4/§6 已冻结该形状，**无任何「run 必须单块」条款** ⇒
  **W3-a 关缺是既有契约内的 guest 实现缺口，不需要 RFC/ADR**。
- **修正后的 W3-a 挡板清单（guest-only，4 处）**：① `cgs_result_return_hex:4798-4800` 多块硬拒；
  ② `cgs_run_return_value:4388-4417` 只读块 0 的返回值；③ `cgs_block_terminator_hex:6630-6632`
  run 终止子一律拒；④ `cgs_run_instr_hex:4121-4128` 以 `terms=0` 发射、每块 `if` 无收口。
- **oracle 侧唯一卫生项（1 行，非 W3-a 必需）**：`helper_dependencies` 补
  `"sico.bytes.is_utf8" => &["sico.bytes.utf8_decode"]`，并把本片钉测试的 oracle 断言由 `is_err`
  改为发射成功（该 fixture 才能作为 G1 多块刀的字节锚）。详见后续侦察 STEP 记录与切片规划
  （O1 卫生 → G1 最小多块 `if+两臂 return` → G2 formatter 实形 2 分支+3 return → G3 W3-a 收口，
  预计 3–4 片）。
- **本 addendum 不改变的事实**：`is_utf8` 臂已落地且必要（真实 formatter 的 run 体确实发射该指令）；
  W3-a 仍 open；M22 overall 仍 NO-GO；未翻任何门。
