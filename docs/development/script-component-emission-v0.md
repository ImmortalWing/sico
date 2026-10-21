# Script Component emission v0 (guest-mirror spec)

> - status: frozen reference for the STEP-0486 S5 component-codegen wave (gate 3)
> - date: 2026-10-20
> - source of truth: [`crates/sico-codegen-wasm/src/lib.rs`](../../crates/sico-codegen-wasm/src/lib.rs) (`compile_script_program` → `build_script_core` → `compile_function` → `emit_script_return`) and [`canonical.rs`](../../crates/sico-codegen-wasm/src/canonical.rs); the layout table stays [`script-canonical-abi-layout-v0.md`](./script-canonical-abi-layout-v0.md)
> - implementation: this document is the *readable freeze* of the emission algorithm the guest-side emitter (`selfhost/codegen.sico`) must reproduce byte-for-byte. The Rust backend stays the only byte oracle (RFC-0011); any disagreement between this document and the Rust output is a defect in this document.

## 1. Scope

This spec covers the **deterministic emission** of a `sico:script/program@0.1.0` Component for the Script profile entry `run: func(input: script-input) -> result<script-output, script-error>`:

- the local-slot plan of a function,
- the instruction sequence per IR operation,
- the Script result lowering into the Canonical ABI return area,
- the fixed component/core-module scaffolding.

It does not cover the intrinsic helper table, the transport (fs/streams/http) modules, or user imports; those are separate wave slices. Every shape outside the described subset must be **typed-refused** by the guest, never silently approximated.

> **Guest-side caveat (STEP-0491, 2026-10-21)**: this document freezes the **Rust-side** emission algorithm. The guest side has a second, independent constraint the Rust side does not have: the guest's own frontend must be able to **lower the guest's own source** of this emitter. Measured on 2026-10-21, that lowering has a *shape frontier* — a `match` nested deep inside a loop body, a `set` whose right-hand side is a nested intrinsic call at that depth, and a tail-region `match` whose subject operands are cells assigned only inside the loop body each make the guest refuse its own source with `ERR:E-SH-IR-CALL-SHAPE`. Before writing the guest emitter, pick a shape that the guest can already lower (see the frontier table in [`../handoff-m22.md` §4](../handoff-m22.md)); the Rust algorithm below stays authoritative for *what* to emit, not for *how the Sico source may be written*.

> **IR-input status (STEP-0496, 2026-10-21)**: the guest now **ingests** the IR in-process. STEP-0496 added a byte-oriented canonical-IR scanner and a `run` emission-plan extractor to `selfhost/codegen.sico` (driver mode `--emit-ir-plan`): the plan text the guest derives from the scanned IR is **byte-identical** to the plan the Rust oracle derives from the same `sico_ir::Module` (the oracle-side generator lives in the fixture test file, not in the oracle crate), and every shape outside the plan subset is typed-refused (`ERR:E-SH-CG-IR-PLAN` / `-TYPE` / `-OP` / `-TERMINATOR`). The plan text is a **verifiable intermediate**, not the emission target: the layout below stays authoritative for *what* to emit, and the emitter may work directly on the scanned IR. What is still missing is the **emission** side — the guest-side emitter must reproduce the layout below byte-for-byte from the ingested IR. Measured fuel cost: the plan mode needs more than the runner CLI's default 10M budget on the Script-profile fixture (10M < cap ≤ 20M), so the emission slice must budget for a byte-oriented scan whose cost grows with the IR size (see [STEP-0496](../steps/STEP-0496-m22-w2-s5-ir-ingestion-plan.md)).

## 2. Component and core-module layout

Component (section ids per `wasm_encoder::ComponentSectionId`):

| order | id | payload |
|---:|---:|---|
| 1 | 7 | script types instance type (`script_types_instance`) |
| 2 | 10 | import of `sico:script/types@0.1.0` (types only) |
| 3 | 6 | alias exports `script-input` / `script-result` |
| 4 | 7 | `run` function type `(input: script-input) -> result<script-output, script-error>` |
| 5 | 1 | the core module (below) |
| 6 | 2 | core instance |
| 7 | 6 | alias exports `memory`, `cabi_realloc`, `cabi_post_run`, `run` |
| 8 | 8 | canonical function lift of `run` |
| 9 | 11 | export `run` |

Core module: `type` section (one type per function, then `alloc`, `cabi_realloc`, `cabi_post_run`), `function` section, `memory` section (one bounded arena memory), `global` section (arena bump pointer, perm-region base, scratch-mode flag), `export` section (`memory`, `run`, `cabi_realloc`, `cabi_post_run`), `code` section (every function body in function-index order).

Every section payload length is a real unsigned LEB128 of the payload byte count; section ids are single bytes.

## 3. Local-slot plan (`LocalLayout::plan`)

1. **Parameters** — each parameter's type is flattened with `flat_ir_types`; the slots are assigned in declaration order, starting at 0. For the Script entry: `ScriptInput` → `arguments(ptr,len)`, `stdin(ptr,len)` = 4 × i32 (slots 0..3).
2. **Instructions** — in IR order, each instruction's result takes `flat` slots (in flat order) from the running counter.
3. **Function cells** — declared locals take their flat slots next.
4. **Dispatcher** — one extra i32 slot after all value slots; it holds the currently executing block id.
5. **Scratch** — two extra i32 slots `(return_area, copy_destination)`, present only in the Script profile.

The `local` declaration section lists the non-parameter locals: one group per slot `(1, ty)` **except** the scratch pair, which is one group `(2, i32)`. The Rust writer does not merge adjacent groups.

Flat rules (`flat_ir_types`): `bool` → 1 × i32; `i64`/`u64` → 1 × i64; `string`/`bytes`/`list`/`map`/`set` → 2 × i32 `(ptr,len)`; `result` → 1 × i32 tag followed by the ok payload then the error payload; a named record → its fields' flats concatenated in declaration order.

Record layouts: `ScriptAbi::record(name)` **prefers** the layout derived from the module's own `records` table (`register_user_records`: flat = field flats concatenated, byte offset = `slot_start × 8`, size = `flat × 8`, align 8) and falls back to the frozen WIT layout only when the module's declaration is not flattenable (for example `ScriptError.code` is an enum and therefore not flattenable, so `ScriptError` always uses the WIT layout).

## 4. Function body shape (`compile_function`)

Every function body is a dispatcher:

```
i32.const <entry block id> ; local.set <dispatcher>
block ; loop
  local.get <dispatcher> ; i32.const <block id> ; i32.eq ; if
    <block instructions> <terminator>
  end
  unreachable
end ; end
unreachable ; end
```

## 5. Instruction emission

| IR operation | emission |
|---|---|
| `project {base, field}` | for each slot of the base's field: `local.get <base slot>` ; `local.set <result slot>` |
| `construct {fields}` | for each field in IR order: copy the source value's slots into the result's contiguous slot range |
| `const_i64` | `i64.const <value>` ; `local.set <slot>` |
| `variant {name, payload}` | `i32.const <tag>` ; `local.set <tag slot>`; then zero-fill the inactive head region (`error` variants right-align their payload), copy the payload slots, then zero-fill the inactive tail region. Zero-fill uses `i32.const 0` or `i64.const 0` according to the slot type. |

Tag values: `ok` = 0, `error` = 1 (variant tag table, declaration order).

## 6. Script result lowering (`emit_script_return`)

Only the Script entry `run` uses this path. The result area holds the `Result` tag at byte 0 and the payload at `result_payload_offset`; the area size is `result_size`.

Frozen facts (from `wit/script-profile-v0/world.wit`):

| record | fields (declaration order) | byte offsets | size / align |
|---|---|---|---|
| `ScriptInput` | `arguments` list\<string> (0,4), `stdin` list\<u8> (8,12) | 0, 8 | 16 / 4 |
| `ScriptOutput` | `stdout` (0,4), `stderr` (8,12), `exit_code` s64 (16) | 0, 8, 16 | 24 / 8 |
| `ScriptError` | `code` enum (0), `message` string (4,8) | 0, 4 | 12 / 4 |

Derived: `result_payload_offset = align_to(1, max(align)) = 8`; `result_size = align_to(8 + max(24, 12), 8) = 32`.

Emission sequence:

```
i32.const 32 ; call <alloc> ; local.set <return_area>
local.get <return_area> ; local.get <tag slot> ; i32.store8 offset=0 align=0
; per field of the lowered record, in declaration order:
;   (ptr,len) field:
;     local.get <len> ; call <alloc> ; local.set <copy_destination>
;     local.get <copy_destination> ; local.get <ptr> ; local.get <len> ; memory.copy
;     local.get <return_area> ; local.get <copy_destination> ; i32.store offset=(8+byte_offset) align=2
;     local.get <return_area> ; local.get <len>             ; i32.store offset=(8+byte_offset+4) align=2
;   s64 field:
;     local.get <return_area> ; local.get <slot> ; i64.store offset=(8+byte_offset) align=3
;   i32/enum field:
;     local.get <return_area> ; local.get <slot> ; i32.store8 offset=(8+byte_offset) align=0
local.get <return_area> ; return
```

Payload bytes are copied into the bounded arena so result buffers never alias caller storage. The record to lower is chosen by payload shape: the output record when the returned value's field map contains every output field name, else the error record, else a typed refusal. Field names are normalized kebab-case → snake_case (`exit-code` → `exit_code`).

## 7. Byte targets used by the wave

The frozen minimal echo fixture (`record ScriptInput` / `record ScriptOutput` / `enum ScriptErrorCode` / `record ScriptError` + `run` returning `ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))`) is the reference shape:

| artifact | size |
|---|---:|
| Component | 981 B |
| core module | 530 B |
| code section | 403 B |
| `run` body | 223 B |
| `alloc` / `cabi_realloc` / `cabi_post_run` bodies | 84 / 72 / 32 B |

The `run` body for that fixture is a single block with five instructions (`project`, `project`, `const_i64`, `construct`, `variant`) and a `return` terminator; its slot plan is 4 parameter slots + 19 instruction slots, dispatcher 23, scratch 24/25, and 21 local declaration groups.
