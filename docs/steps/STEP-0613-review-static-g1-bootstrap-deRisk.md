# STEP-0613 review — static de-risk of the G1 guest bootstrap statement-frontier (READ-ONLY)

- Date: 2026-11-09
- Track: M22 compiler self-host, goal **W3-a**
- Slice type: **read-only static review** — zero source modifications, no cargo/build/test, no commit/push.
- Review target: the **unverified** G1 implementation on branch `codex/m22-w2-step0613` in `D:/SorftWare/sico-m22-w3a`
  (worktree HEAD `e93f59a` + the working-tree diff). Host has **no Rust toolchain** (STEP-0613 §7), so this
  review is **pure static reasoning** over frozen oracle source + the diff; every byte claim is traced to
  `file:line`, nothing is hand-invented.
- Files in scope (git status `M`/`??`, nothing else touched):
  - `crates/sico-codegen-wasm/src/lib.rs` (+6/−0, O1)
  - `selfhost/codegen.sico` (net −31, G1)
  - `runner/sico-runner/tests/selfhost_compiler.rs` (1 flipped pin + 1 new pin)
  - `docs/steps/STEP-0613-m22-w3a-o1-oracle-hygiene-g1-guest-multiblock-run.md` (author intent)

## Verdict table

| # | Risk point | Verdict | One-line reason |
|---|---|---|---|
| 1 | O1 oracle correctness (`helper_dependencies` + `is_utf8`) | **CORRECT** | mirrors existing entries; cannot change any existing frozen component (only un-refuses previously-refused modules) |
| 2 | Multi-block `run` byte-structure vs oracle | **CORRECT (structure)** | per-block terminator + per-return-point result area + `000b0b000b` epilogue matches `compile_function`; single-block is byte-identical |
| 3 | Bootstrap statement frontier | **LIKELY-CORRECT, UNPROVEN** | new shapes ≤ proven peers (no match-in-match, simple call args, delegation kept); only a cargo run confirms |
| 4 | Test-flip / new-pin soundness | **SUSPECT → BUG (guest assertion)** | the fixture uses `is_utf8` without a direct `utf8_decode` call; the guest emits `call <utf8_decode index>` but does **not** emit the `utf8_decode` helper (type/function/body) — pins will fail on the guest side; the typed-refusal contrast is sound |

**Recommendation: FIX-FIRST.** The G1 structural emission change and the O1 oracle change are sound, but
as written the two new pins **cannot pass**: the guest lacks the oracle's O1-equivalent
(`is_utf8 → utf8_decode` helper-dependency closure), so the `IS_UTF8_GATE_FIXTURE_SOURCE`-based pins fail on
the guest side. The specific fix (one small change, 4 sites) and the cargo-only confirmations are in §5.

---

## 1. Point 1 — O1 oracle correctness: CORRECT

### 1.1 The entry mirrors the existing declaration style

The added entry (`crates/sico-codegen-wasm/src/lib.rs:546`):

```rust
"sico.bytes.is_utf8" => &["sico.bytes.utf8_decode"],
```

is textually and structurally identical in kind to the existing entries:

- `"sico.bytes.equal" => &["sico.bytes.equal"]` (lib.rs:540)
- `"sico.text.char_at" => &["sico.text.char_loc"]` (lib.rs:548) — an intrinsic whose *runtime* helper
  differs from its own name, the same shape as `is_utf8 → utf8_decode`
- `"sico.text.leading_spaces" => &[]` (lib.rs:553) — the documented precedent that an **inline** arm may
  declare **no** helper

So the declaration style is consistent with the surrounding code. The comment block (lib.rs:541-545)
accurately describes the phantom refusal (see §1.3).

### 1.2 `helper_dependencies` only affects helper presence/ordering — not any existing component

`helper_dependencies` has exactly **one** consumer: the `used_helpers` BTreeSet built at
lib.rs:835-846 (`for instruction in ... if let Operation::Intrinsic { name, .. } → for helper in
helper_dependencies(name) { used_helpers.insert(helper) }`). That set is the sole input to the `helpers`
map (lib.rs:847-857, `used_helpers.into_iter().enumerate()` → `helper_count + 3 + index`), which drives
(a) the helper function indices (and hence every `call <helper>` byte), and (b) which helper
type/function/body entries the module carries. It does **not** affect data segments, module-function
ordering, or any non-helper byte.

Adding a new `(intrinsic → helper)` edge therefore can only *add* a helper name to `used_helpers` for
modules that use that intrinsic. It never removes or reorders an existing member.

### 1.3 No existing frozen component depends on `is_utf8` being absent

Before O1, `helper_dependencies("sico.bytes.is_utf8")` fell to the default arm (lib.rs:557-560):
`INTRINSIC_HELPERS.iter().find(|h| **h == name).map_or(&[], |h| std::slice::from_ref(h))`. `is_utf8` is
**not** in `INTRINSIC_HELPERS` (lib.rs:566-588; the list contains `utf8_decode` at :571 but not
`is_utf8`), so the default arm returned `&[]`.

Consequences for any module **M**:

- **M does not use `is_utf8`**: `helper_dependencies` is never called for that name; `used_helpers`
  unchanged → byte-identical. ✓
- **M uses `is_utf8` and calls `utf8_decode` directly**: `used_helpers` already contains `utf8_decode`
  (from the direct call); the O1 edge adds nothing → set unchanged → byte-identical. ✓
- **M uses `is_utf8` and does *not* call `utf8_decode` directly**: the inline `is_utf8` arm
  (lib.rs:4505-4514) executes `body.instruction(Instruction::Call(helper("sico.bytes.utf8_decode")?))`
  (lib.rs:4512), and `helper()` is `emit.helpers.get(name).copied().ok_or_else(|| unsupported(...,
  "stdlib helper layout"))` (lib.rs:4414-4419). With `utf8_decode ∉ used_helpers`, `helpers` has no
  entry → **compile-time refusal** `Unsupported { function, "stdlib helper layout" }` → **no component is
  ever produced**. This is precisely the phantom refusal STEP-0612 §2 identified. After O1 the module
  emits.

So the only modules whose output changes are modules that **previously refused** (no frozen artifact
exists for them). A frozen artifact cannot depend on `is_utf8` being absent, because no module that
needed `is_utf8`-only ever produced a frozen component. **O1 cannot change any existing frozen
Component's bytes.** ✓

**Verdict: CORRECT.** (Note: this is a statement about the *oracle* side only. The guest's equivalent of
the O1 dependency closure is **missing** — see §4, which is the blocking finding.)

---

## 2. Point 2 — multi-block `run` byte-structure vs oracle: CORRECT (structure)

### 2.1 Oracle ground truth (the frozen multi-block shape)

`compile_function` (lib.rs:1684-1754) lowers **every** module function (incl. `run`) identically:

```
i32.const <entry>          41 <leb>            (lib.rs:1698-1701)
local.set <dispatcher>     21 <leb>            (lib.rs:1702)
block  (empty)             02 40               (lib.rs:1703)
loop   (empty)             03 40               (lib.rs:1704)
per block b:
  local.get <dispatcher>   20 <leb>            (lib.rs:1720)
  i32.const <b.id>         41 <leb>            (lib.rs:1721-1724)
  i32.eq                   46                  (lib.rs:1725)
  if   (empty)             04 40               (lib.rs:1726)
  <block instructions>                         (lib.rs:1728)
  <terminator>                                 (lib.rs:1730)
  end                      0b                  (lib.rs:1740)
unreachable                00                  (lib.rs:1750)
end                        0b                  (lib.rs:1751)
end                        0b                  (lib.rs:1752)
unreachable                00                  (lib.rs:1753)
end                        0b                  (lib.rs:1754)
```

i.e. epilogue `000b0b000b`. `compile_terminator` (lib.rs:2269-2328) implements `Return`/`Jump`/`Branch`
(the multi-block core, lib.rs:2285-2300)/`Match`/`Unreachable`. `emit_return` (lib.rs:3313-3325) routes a
`run` `Return(value)` to `emit_script_return` (lib.rs:3321-3324); `emit_script_return`
(lib.rs:3379-3470) emits, **per return point**, one result-area sequence: `i32.const <result_size>`
(lib.rs:3411-3414) `call <alloc>` (lib.rs:3415) `local.set <return_area>` (lib.rs:3416)
`local.get <return_area>` `local.get <tag>` `i32.store8 0 0` (lib.rs:3417-3423) + result fields
(lib.rs:3425-3466) + `local.get <return_area>` + `return` `0f` (lib.rs:3467-3468), all sharing the one
scratch pair `context.scratch` (lib.rs:3387-3389).

### 2.2 Guest G1 layout, term by term

`cgs_run_instr_hex` (selfhost/codegen.sico:4121-4128) now calls
`cgs_function_instr_hex(bytes, crih_run, U64.literal(1))` (codegen.sico:4126; was `0`).

`cgs_function_instr_hex` (codegen.sico:4135-4194) emits, per block: the dispatch head
`20 <disp> 41 <id> 460440` (codegen.sico:4167-4169), the block instructions (4175-4182), and — when
`terms == 1` — the terminator via `cgs_block_terminator_hex` + `0b` (codegen.sico:4183-4190, `0b` at
:4189). The shell `41 <entry> 21 <disp> 02400340` is at codegen.sico:4157-4159.

`cgs_block_terminator_hex` (codegen.sico:6576-6603) dispatches:
- `return` → if `fn` is `run` → `cgs_result_return_hex(bytes, fn, cbth_value, vindex)` (codegen.sico:6583-6587);
  else `cgs_flat_return_hex` (6588). The `run` `return` goes to the result-area path.
- `branch` → `cgs_branch_hex` (6590-6592), which emits `20 <cond> 0440 41<then> 21<disp> 05 41<else>
  21<disp> 0b 0c01` (codegen.sico:5429-5464) — identical to `compile_terminator`'s `Branch`
  (lib.rs:2290-2299).
- `jump`/`match`/`unreachable` → existing paths (6593-6601).

`cgs_result_return_hex` (codegen.sico:4756-4847) emits for the ok arm (4767-4801):
`41 20` (`i32.const 32`, codegen.sico:4792 via `cgs_i32_const_hex(cgs_frozen_result_area_size())` with
:4277-4279 `=32`) `10 <alloc>` (`cgs_alloc_call_hex`, :4792) `21 <area>` (`cgs_local_set_hex`, :4792)
`20 <area>` `20 <tag>` `3a0000` (4793-4795) `<result fields>` (4796) `20 <area>` `0f` (4797-4798) — the
same byte order as `emit_script_return` (lib.rs:3411-3468). The error arm (4802-4842) is the analogous
sequence. The area/copy scratch slots are `dispatcher+1`/`dispatcher+2` (codegen.sico:4790-4791,
4829-4830), the same convention the (previously green) single-block path used, and the oracle's shared
`context.scratch` pair — reuse is safe because only one return path executes.

`cgs_run_body_hex` (codegen.sico:4853-4859) appends the epilogue `000b0b000b` (codegen.sico:4858), which
is exactly `compile_function`'s tail `00 0b 0b 00 0b` (lib.rs:1750-1754).

### 2.3 The minimal `if`+two-arm-`return` `run` (the fixture) structurally matches

Fixture `IS_UTF8_GATE_FIXTURE_SOURCE` (selfhost_compiler.rs:3943) lowers to: block 0 = the `is_utf8`
condition + `branch` terminator (then=block 1, else=block 2), block 1 = `return ok(ScriptOutput(...))`,
block 2 = `return error(ScriptError(...))`. Guest emits, in order:

```
41<0> 21<disp> 02400340                     shell
20<disp> 41<0> 460440 <is_utf8 instr>       block 0 head + instruction
<branch bytes: 20<cond> 0440 ... 0c01> 0b   block 0 terminator + close-if
20<disp> 41<1> 460440 <construct ok>        block 1 head + instruction
<result-area ok seq ending 0f> 0b           block 1 terminator + close-if
20<disp> 41<2> 460440 <construct err>       block 2 head + instruction
<result-area err seq ending 0f> 0b           block 2 terminator + close-if
000b0b000b                                  epilogue
```

which is term-for-term the oracle's `compile_function` output for the same IR (§2.1). **Structurally
CORRECT.**

### 2.4 Single-block `run` stays byte-identical (the author's claim holds)

- **Old** (HEAD) single-block: `cgs_run_instr_hex` with `terms=0` → no per-block terminator, no close-if
  in the instruction segment; `cgs_run_body_hex` then appended
  `sico.text.concat(crbh_return, "0b000b0b000b")` where `crbh_return` was the old
  `cgs_result_return_hex(bytes, run)` (block-0 result area, ending `0f`). Net layout:
  `... <instr0> <resultseq> 0b 00 0b 0b 00 0b` — the `0b` is the first byte of the old epilogue string,
  closing block 0's dispatcher `if`.
- **New** single-block: `terms=1` → the per-block `0b` close is emitted inside
  `cgs_function_instr_hex` (codegen.sico:4189) right after `<resultseq>`; `cgs_run_body_hex` appends
  `000b0b000b`. Net layout: `... <instr0> <resultseq> 0b 00 0b 0b 00 0b` — **identical byte sequence**.

The value passed to the result area is the same in both (old `cgs_run_return_value` read block 0's
`return` terminator `data`; new `cbth_value` reads the same block's terminator `data`, codegen.sico:6584),
and `vindex` is `cgs_value_index(bytes, run)` in both (old computed it inside `cgs_result_return_hex`;
new gets it from `cgs_function_instr_hex:4141`, same `run`). The only behavioral deltas are (a) the
multi-block refusal is gone, and (b) `cgs_value_ids_refusal` now runs once (inside
`cgs_function_instr_hex:4137`) instead of twice — a pure pure-function call, no byte effect. **Confirmed
byte-identity for single-block.** ✓

### 2.5 Caveat (carries into §4)

§2.2-2.4 show the *emission structure* is right. But the **fixture-level byte parity** that the two new
pins assert additionally depends on the helper section agreeing, and that is **not** the case for this
fixture (see §4). So: structure CORRECT; the specific fixture will not byte-match until the guest-side
O1-equivalent is added.

---

## 3. Point 3 — bootstrap statement-frontier: LIKELY-CORRECT, UNPROVEN

The self-host pins that guard the reshaped functions are
`sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly` (selfhost_compiler.rs:3469-3512;
covers `cgs_result_return_hex` :3497, `cgs_run_instr_hex` :3501) and
`sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly` (selfhost_compiler.rs:3515-3551;
covers `cgs_block_terminator_hex` :3538, `cgs_run_body_hex` :3541), plus the whole-compiler
`compile_guest()` bootstrap (selfhost_compiler.rs:187-190) that every `run_guest*` call depends on.
These pins extract each function's current source (`codegen_function_text` :3354, `codegen_stub` :3404
reads the **current** signature) and lower it through the guest frontend.

The measured frontier rules (STEP-0507 comment, selfhost_compiler.rs:3476-3479): a user call may not sit
directly in a return expression; a call argument may not be another call; a host call may not take a host
call as its argument. These are **conservative slogans** — the real frontier allows more (see below), so
I judged the new shapes by **direct comparison to already-green peers**, which is the stronger evidence.

### 3.1 `cgs_result_return_hex` (new 4-arg) — within frontier

The new body (codegen.sico:4756-4847) is a **strict subset** of the old body (which the step0507 pin
covered and was green at HEAD, STEP-0610 §4): the only delta is it *removes* the `cgs_value_ids_refusal`
call, the `cgs_value_index` call, the `cgs_run_return_value` call and the multi-block check, and treats
`crrh_value`/`vindex` as parameters instead of `let`-locals. No new call shape is introduced. Its
deepest shape — `sico.text.concat(cgs_i32_const_hex(...), sico.text.concat(cgs_alloc_call_hex(...),
cgs_local_set_hex(...)))` at codegen.sico:4792 — is **unchanged** and was green. `List[U64]` parameters
are already proven (`cgs_branch_hex`, codegen.sico:5429, step0526-green). No `match` in a `match` branch
was added. **LIKELY-CORRECT.**

### 3.2 `cgs_block_terminator_hex` (new nested `if`) — within frontier

The new shape (codegen.sico:6583-6588):

```
if cg_is_word(cbth_kind, "return"):
    let cbth_value = cgj_value_text(bytes, cgj_member(bytes, cbth_term, "data"))
    if cg_is_word(cbth_name, "run"):
      return cgs_result_return_hex(bytes, fn, cbth_value, vindex)
    end if
    return cgs_flat_return_hex(bytes, fn, cbth_value, vindex)
end if
```

- **Depth**: `if` inside an `if`'s then-branch (depth 2), with `return <user_call>` at depth 2 and at
  depth 1. The already-green `cgs_result_field_hex` (codegen.sico:4598-4611) has `if` inside `if` inside
  `if` (depth 3) with `return <user_call>` at depth 3 and depth 2, and **call arguments that are
  themselves user calls** (`cgs_alloc_index(bytes)` as an argument to `cgs_result_list_hex`,
  codegen.sico:4604). The new shape is strictly shallower and has only simple-variable call arguments
  (`bytes, fn, cbth_value, vindex`). ✓
- **No `match` in a `match` branch**: the new code adds no `match` at all. ✓
- **Call args are not calls**: `cgs_result_return_hex(bytes, fn, cbth_value, vindex)` and
  `cgs_flat_return_hex(bytes, fn, cbth_value, vindex)` take only params/locals. ✓
- **Delegation, not inlining**: `cgs_result_return_hex` is a separate function, exactly the STEP-0610
  §3.2 lesson (inlining pushed `cgs_intrinsic_hex` over the frontier with `ERR:E-SH-IR-STATEMENT`; the
  delegated `cgs_is_utf8_hex` at codegen.sico:3868 restored green). The run `return` is delegated, not
  inlined. ✓
- **Return count unchanged**: the deleted blanket `if run: return "ERR:..."` is replaced by the nested
  `return cgs_result_return_hex(...)`; net `return` statements in the function stay the same (7→7). The
  `ERR:E-SH-IR-RETURN-COUNT` gate (parser.sico:10839/12599) is therefore not newly tripped. ✓

**LIKELY-CORRECT** — but this is the #1 risk and only a cargo run of the step0507/step0526/step0513 pins
confirms it. If it reds, the most likely culprit is this nested-dispatch shape (STEP-0613 §7.2).

### 3.3 `cgs_run_instr_hex` / `cgs_run_body_hex` — trivially within frontier

`cgs_run_instr_hex` (4121-4128) changed only `U64.literal(0)` → `U64.literal(1)` (4126) — same shape.
`cgs_run_body_hex` (4853-4859) *simplified* (dropped a nested `sico.text.concat(sico.text.concat(...))`
to a single concat; dropped the `cgs_result_return_hex` call) — strictly simpler than the old (green)
shape. ✓

**Verdict: LIKELY-CORRECT (unproven).** Note also that `compile_guest()` (selfhost_compiler.rs:187-190)
rebuilds the whole guest compiler from `parser.sico`+`codegen.sico`+`compiler.sico`, so if any reshaped
function is over the frontier, **every** `run_guest*` test in the file panics, not just the two pins —
a broad, easily-visible failure surface.

---

## 4. Point 4 — test-flip / new-pin soundness: SUSPECT → BUG on the guest assertion

### 4.1 The two pins (assertions)

Flipped pin `sico_is_utf8_gated_run_lowers_byte_exactly` (selfhost_compiler.rs:3946-3986): asserts
(a) oracle `compile_script_program(&module)` emits (`.expect`), (b) guest `--emit-component-hex` output
`==` oracle bytes, (c) `wasmparser::Validator` passes. New pin
`sico_run_emits_the_minimal_multiblock_run_component_byte_exactly` (selfhost_compiler.rs:3989-…): same
component equality + frontend IR equality + typed-refusal contrast (non-`Result` run, :4029).

### 4.2 Oracle assertions are sound

(a) is sound **only because of O1**: the fixture uses `is_utf8` and no direct `utf8_decode`; without the
O1 entry the oracle refused (`stdlib helper layout`, §1.3); with it `used_helpers = {utf8_decode}`
(lib.rs:835-846), the `is_utf8` arm's `helper("sico.bytes.utf8_decode")?` (lib.rs:4512) succeeds, and the
multi-block `run` is already supported (lib.rs:1684-1754). ✓

### 4.3 Guest assertions are NOT sound — the guest lacks the O1-equivalent (the blocking finding)

The guest's used-helper model is **directly-used intrinsic helpers only**: every helper-section gate is
`cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")` (a walk of the module's instructions,
codegen.sico:3221-3243):

- type section: `cgs_core_types_hex` — `cctsh_utf8` at codegen.sico:5189, gated `if ... == 1` at :5195
- function section: `cgs_core_funcs_hex` — `ccfsh_utf8` at :5308, gated at :5314
- code section: `cgs_core_code_hex` — `cccsh_utf8` at :6651, gated at :6751

There is **no** `is_utf8 → utf8_decode` closure anywhere in the guest (all `utf8_decode` references:
:181, :713, :752, :3268, :3271, :3806, :3911, :5189, :5308, :6651 — none maps `is_utf8` to it).

For the fixture: the module contains **no** `sico.bytes.utf8_decode` instruction → all three gates are 0
→ the guest emits **no** `utf8_decode` type entry, **no** function entry, **no** body. But the inline
`is_utf8` arm emits `call <utf8_decode index>`: `cgs_is_utf8_hex` (codegen.sico:3911) computes
`cgs_helper_index(bytes, "sico.bytes.utf8_decode")`, which returns `base + rank` unconditionally for
`wanted == "sico.bytes.utf8_decode"` (codegen.sico:3268-3269) — it does **not** require the helper to be
used. For the fixture (`cgs_function_count = 1`, no `bytes.equal`, no other helper): base = 1+3 = 4,
rank = 0 → returns **4**; the `ciuh_helper == 0` guard (codegen.sico:3912-3914) does not fire.

Result: the guest emits `... call 4 ...` (matching the oracle's `call 4`, since the oracle's helper index
is also `helper_count + 3 + 0 = 0 + 1 + 3 + 0 = 4`, lib.rs:797-800/852) **but defines only functions
0-3** (`run` + alloc/realloc/post). Function 4 does not exist in the guest output → the guest component
is (a) **shorter** than the oracle's (missing the `utf8_decode` type/function/body entries) and (b)
**invalid** (`call 4` → undefined function index). The pinned assertions `assert_eq!(actual, expected)`
(selfhost_compiler.rs:3982) and `validate_all` (:3985) **fail**. The new pin fails the same way.

> So the author's claim (STEP-0613 §4.2/§4.3, "expect 逐字节等同 oracle") holds for the *emission
> structure* but **not** for the *helper section* of this fixture. The guest needs the same one-direction
> dependency closure the oracle got in O1.

**This is a real gap, not a hypothesis**: it is fully traceable in code (the four gates above are
unconditional on `is_utf8`), and it is not masked by any guard. It is a **pre-existing latent asymmetry
of the STEP-0610 `is_utf8` arm that G1 exposes** (before G1 the fixture refused at the multi-block check,
so the arm never reached component emission; STEP-0610 §5 recorded exactly that the arm was "unreachable
end-to-end").

### 4.4 Typed-refusal contrast remains sound (unaffected by O1)

`out_of_profile` (selfhost_compiler.rs:4029) is `run() returns I64` with an `if`-`return`-`return` body.
- Oracle refuses at the Script-entry signature check
  (`compile_script_program_parts`, lib.rs:644-652: `run` must be
  `(input: ScriptInput) -> Result[ScriptOutput, ScriptError]`; this fixture is `() -> I64`) → `Err` —
  **before** any helper logic, so O1 is irrelevant. ✓
- Guest refuses in `cgs_result_return_hex`: the `return I64` value is not produced by a `variant`
  instruction → `cg_is_word(cgs_op_name(...), "variant")` is false → `return "ERR:E-SH-CG-IR-EMIT"`
  (codegen.sico:4766 → else :4843-4845) → the `--emit-component-hex` path returns a `Domain` outcome
  (the same mapping the STEP-0610 pin already asserted for an emitter refusal). ✓

Both sides stay fail-closed; O1 does not weaken the oracle side (it refuses at the signature gate, not at
the helper layout). **Sound.**

### 4.5 Other pins plausibly affected (cargo-only)

The single-block `run` pins `sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly`
(STEP-0504) and `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly` (STEP-0509) use
ECHO fixtures with **no** `is_utf8` → §2.4 byte-identity applies → expected to stay green (unproven).

---

## 5. Recommendation: FIX-FIRST (do not land G1 as-is)

**Blocking (must fix before the two pins can pass):** add the guest-side `is_utf8 → utf8_decode`
helper-dependency closure. Concretely, the helper-section gates and the rank logic must treat
`is_utf8` as implying `utf8_decode` is used:

- `cgs_core_types_hex` — gate at codegen.sico:5189/5195
- `cgs_core_funcs_hex` — gate at codegen.sico:5308/5314
- `cgs_core_code_hex` — gate at codegen.sico:6651/6657/6751
- `cgs_helper_index` — rank increment for `utf8_decode` at codegen.sico:3271 (so a module using
  `is_utf8` **and** a later helper still numbers that helper correctly)

i.e. replace `cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode")` with
`(cgs_module_has_intrinsic(bytes, "sico.bytes.utf8_decode") == 1 or
cgs_module_has_intrinsic(bytes, "sico.bytes.is_utf8") == 1)` at those four sites (and keep the
`cgs_helper_index` return at :3268-3269 consistent — it already returns `base + rank` for
`utf8_decode`, which is correct once the rank counts it as used).

This mirrors the oracle's `helper_dependencies` O1 edge exactly (lib.rs:546). Alternatively (larger,
not recommended here): give the guest a real `helper_dependencies`-style table so this class of
asymmetry cannot recur.

**Must be confirmed by cargo once a toolchain exists (each is not provable statically):**

1. `step0507` / `step0526` / `step0513` self-host pins stay green (bootstrap statement-frontier, §3).
2. The two new pins turn green after the §5 fix (helper section parity for the fixture).
3. The single-block `run` pins (STEP-0504/0509) stay green (no regression).
4. `sico_compiler_lowers_full_formatter_byte_exactly` and the whole-`compile_guest()` bootstrap still
   build (the reshaped functions must not push the compiler over the frontier — a red here breaks every
   `run_guest*` test, §3.3).
5. `wasmparser::Validator` accepts the guest fixture output.

**Out of scope / noted, not a blocker:** the guest's helper index base is
`cgs_function_count + 3` (codegen.sico:3260) and ignores `import_count`, whereas the oracle's is
`import_count + fn_count + 3` (lib.rs:797-800) — a **pre-existing** discrepancy for modules with
fs/stream/http/user imports (not G1; the fixture and the formatter have no imports). Also, the
`emitter-vocabulary-v1.json` source sha is stale (STEP-0613 §7.4) — a known bound cost, not a correctness
issue.

---

## 6. Honest boundary of this review

- **Proved by reading code:** O1 cannot regress any frozen component (§1); the multi-block emission
  structure matches the oracle term-for-term and single-block is byte-identical (§2); the guest lacks the
  O1-equivalent and will emit a `call` to a non-emitted helper for the fixture (§4.3); the contrast test
  is sound (§4.4).
- **Hypothesis, only a cargo run settles:** the bootstrap statement-frontier (§3) — the new nested `if`
  in `cgs_block_terminator_hex` is within the envelope of the proven `cgs_result_field_hex`, but the
  frontier is measured, not derivable, and STEP-0610 §3.2 showed it can break on a shape that "looks"
  fine.
- **No source file was modified; no cargo/build/test was run; nothing committed or pushed.** M22 overall
  remains NO-GO; no gate flipped.
