# STEP-0626 — M22 W3-a: whole-`formatter.sico` Component emission closed byte-exactly (19.0 s / 486.2e9 fuel) + deep audit of hidden bugs and optimization directions

- Date: 2026-11-18
- Branch: `dev` / `codex/m22-w2-step0613`
- Status: **implementation + measurement + deep optimization audit complete locally (W3-a whole-`formatter.sico` Component byte-equality achieved within standard budget; MS-081..084 `fixed_literal_const_segment` flipped from `FuelExhausted` to `ir-equal+component-equal` in 12.40 s)**; M22 overall remains **NO-GO** (pending W3-b/W4/W5/W6/W7 and independent CI).
- Evidence level: `internal-fixture` (real Wasmtime v46.0.1 runner on Windows x64 GNU, byte-for-byte comparison against Rust production CLI `sico build --profile script-v0`, verified by `wasmparser::Validator`).

---

## 1. Summary of Landed Fixes & Breakthroughs

1. **Corrected the STEP-0625 `ERR:E-SH-CG-IR-FLAT` root-cause diagnosis and closed the local-cell flattening gap (`selfhost/codegen.sico:1714-1719`)**:
   - `STEP-0625` inferred that `ERR:E-SH-CG-IR-FLAT` on whole-`formatter.sico` was caused by missing `sico.map.*` instruction emission arms. Static inspection proved that `ERR:E-SH-CG-IR-FLAT` is only returned during local-declaration layout (`cgs_function_local_hex:1744,1764` and `cgs_flat_return_hex:5738`), and `cgs_flat_hex` (`:1517`) already flattened `"map"` to `"017f017f"`.
   - The true blocker was `cgs_cell_flat_hex` (`:1667-1724`), which accepted `bool`, `i64`, `u64`, `string`, `bytes`, and `result`, but omitted `"list"` and `"map"` local cells (used by mutable `let`/`set` variables in `formatter.sico::line_tokens` and `formatter.sico::normalize_source`). Delegating `"list"` and `"map"` in `cgs_cell_flat_hex` to `cgs_flat_hex(bytes, ty)` mirrors the Rust backend's `lower_local_types` (`crates/sico-codegen-wasm/src/lib.rs:2930-2960`).

2. **Fixed the `74098aa` Wasm section-mismatch bug for `sico.map.empty[Text,U64]` and landed `sico.map.get[Text,U64]` + `sico.map.put[Text,U64]` (`selfhost/codegen.sico`)**:
   - Commit `74098aa` added `sico.map.empty[Text,U64]` to `cgs_core_code_hex` (`code` section) without registering it in `cgs_core_types_hex` (`type` section) or `cgs_core_funcs_hex` (`function` section), which would emit mismatched Wasm section counts and shift helper type indices. Fixed both sections.
   - Added `cg_map_get_text_u64_helper_body_hex` (`:323-327`) and `cg_map_put_text_u64_helper_body_hex` (`:334-341`), replicating the Rust backend's `emit_map_get` and `emit_map_put` (`crates/sico-codegen-wasm/src/stdlib.rs:1182-1394`) byte-for-byte, and wired `sico.map.get[Text,U64]` and `sico.map.put[Text,U64]` across `cgs_intrinsic_bit`, `cgs_helper_index`, `cgs_collection_call_hex`, `cgs_intrinsic_hex`, `cgs_core_types_hex`, `cgs_core_funcs_hex`, `cgs_core_code_hex`, and `tools/census-m22-emitter-vocabulary.py`.

3. **Eliminated $O(S \times I^2)$ index-rescanning in `cgs_data_offset`, `cgs_data_hex`, and `cgs_function_local_hex` (`selfhost/codegen.sico`)**:
   - Converted the 3 nested `cgj_array_item` loops (`functions`, `blocks`, `instructions`) in `cgs_data_offset` (`:2957-3005`), `cgs_data_hex` (`:5270-5311`), and `cgs_function_local_hex` (`:1739-1787`) to `cgj_array_first` / `cgj_array_next` cursors, and made `cgs_data_offset` return immediately upon matching the target literal `hex` instead of continuing to walk the remainder of the module.

4. **Measured Breakthroughs**:
   - **Whole-`formatter.sico` Component emission (W3-a) closed locally**:
     - Under the standard budget (`800e9` fuel / `120 s` / `2560 MiB`), both `A:main` and `B:run` variants of `selfhost/formatter.sico` (827 lines, 30 functions) now emit **28,459 bytes** (`56,918` hex chars) in **19.0 s**, consuming **`486,209,428,145` fuel** (`60.8%` of the `800e9` budget, down **−59.0%** from `1.186e12` in `STEP-0618`), **100% byte-for-byte equal** to the Rust production compiler output (`byte_equal = yes`), and passing `wasmparser::Validator::validate_all` (`validator = pass`).
     - Pinned by the permanent non-ignored test `sico_compiler_emits_the_whole_formatter_component_byte_exactly` in `runner/sico-runner/tests/selfhost_compiler.rs`.
   - **`parser.sico:fixed_literal_const_segment` (`MS-081..084`) flipped from `FuelExhausted` to `ir-equal+component-equal`**:
     - Previously exhausted `>800e9` fuel in `STEP-0605`, `STEP-0606`, and `STEP-0611`; now finishes in **12.40 s** with `verdict = "ir-equal+component-equal"` (`component_sha256 = 9ab5f5e51e0d0714c98ba5086f7f524a48be21a54de574a295e2bdf122256b7f`).
   - **Ledger & Census Hygiene**:
     - Updated `selfhost/match-component-coverage-v1.json` (`LEDGER_CURRENT sites=405`) and `selfhost/emitter-vocabulary-v1.json` (`EMITTER_VOCABULARY_CURRENT sites=342`).
     - Cleaned up 7 compiler/Clippy warnings in `runner/sico-runner/tests/selfhost_formatter_component_measurement.rs` so `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` is clean.

---

## 2. Deep Audit of Hidden Problems & Detailed Optimization Directions

### 2.1 Category A — M22 Guest Emitter (`selfhost/codegen.sico`) Hidden Superlinear Bottlenecks

1. **[A1 — CRITICAL] `cgs_helper_index` recomputes `cgs_intrinsic_mask(bytes)` and `cgs_function_count(bytes)` on EVERY intrinsic call instruction ($O(M \times I)$ full-module rescan)**
   - **Location**: `selfhost/codegen.sico:3463-3485` (`cgs_helper_index`), called from `cgs_collection_call_hex:3783`, `cgs_list_length_u64_hex:3857`, `cgs_intrinsic_hex:3993,4054`, `cgs_is_utf8_hex:4191`, `cgs_text_trim_hex:4259`, `cgs_bytes_equal_hex:6449`, `cgs_starts_with_hex:6530`, `cgs_u64_to_text_hex:6590`.
   - **Hidden Bug**: `STEP-0625` hoisted `cgs_intrinsic_mask(bytes)` only out of the 3 module-section builders (`cgs_core_types_hex`, `cgs_core_funcs_hex`, `cgs_core_code_hex`), missing that `cgs_helper_index` calls `cgs_function_count(bytes)` and `cgs_intrinsic_mask(bytes)` (walking every function, block, and instruction of the entire module!) on **every single intrinsic instruction** (`sico.text.concat`, `sico.text.length`, `sico.list.append`, `sico.list.get`, `sico.map.get`, `sico.map.put`, `sico.u64.to_text`, etc.). In `parser.sico` closures with $I \approx 15,000$ instructions and $M \approx 4,000$ intrinsic calls, this causes $4,000$ full-module walks!
   - **Optimization Direction**: Compute `fn_count = cgs_function_count(bytes)` and `mask = cgs_intrinsic_mask(bytes)` once per module (or once per function in `cgs_function_instr_hex`, e.g., appended to `vindex` or passed as a precomputed `(fn_count, mask)` pair to `cgs_helper_index_from(fn_count, mask, wanted)`). This reduces every `cgs_helper_index` call from $O(\text{module})$ to $O(1)$ bitwise operations and is the single largest remaining fuel lever for the 300+ `parser.sico` closures.

2. **[A2 — HIGH] $O(F^2)$ full-module JSON array rescanning in `cgs_core_types_hex` and `cgs_core_code_hex`**
   - **Location**: `selfhost/codegen.sico:5560` (`cgs_core_types_hex` -> `cgs_core_type_hex:5450`) and `selfhost/codegen.sico:7075` (`cgs_core_code_hex` -> `cgs_function_body_hex:6993`), both calling `cgs_function_at_index(bytes, index)` (`:5353` -> `cgj_array_item(bytes, cfai_functions, index)`).
   - **Hidden Bug**: For a module with $F = 160$ functions and 1 MiB of IR, `cgs_function_at_index` rescans the `"functions"` JSON array from byte 0 on each index `0..F-1` in both `cgs_core_types_hex` and `cgs_core_code_hex` ($2 \times \frac{F(F-1)}{2}$ full-function skips).
   - **Optimization Direction**: Replace the index loop `0..count` in `cgs_core_types_hex` and `cgs_core_code_hex` with a `cgj_array_first` / `cgj_array_next` cursor over `"functions"`, passing the function byte offset `fn_off` directly to `cgs_core_type_at_hex(bytes, fn_off)` and `cgs_function_body_at_hex(bytes, fn_off)`.

3. **[A3 — HIGH] $O(C_{\text{calls}} \times F)$ function-table rescan on every user function call in `cgs_call_hex`**
   - **Location**: `selfhost/codegen.sico:3119` (`cgs_call_hex` -> `cgs_function_position(bytes, cc_target)` at `:5367`).
   - **Hidden Bug**: Every user `call` instruction walks the `"functions"` array from function 0 to find the 1-based position of `cc_target`.
   - **Optimization Direction**: Precompute a module-wide `findex: List[U64]` (or verify dense canonical function IDs `1..F` once per module in `cg_core_module_hex` just like `cgs_value_ids_refusal` does for values, then compute `cc_index = cgs_sub(cgs_decimal_u64(cc_target), U64.literal(1))` in $O(1)$).

4. **[A4 — HIGH] $O(K \times (I + L^2))$ per-instruction local-cell and dispatcher-slot rescans**
   - **Location**: `selfhost/codegen.sico:2251-2375` (`cgs_cell_slots`, `cgs_dispatcher_slot`, `cgs_cell_slot`), `:4299` (`cgs_write_local_hex`), `:4335` (`cgs_read_local_hex`), `:5831` (`cgs_branch_hex`), `:5857` (`cgs_jump_hex`), `:3861` (`cgs_list_length_u64_hex`), `:5133,5168` (`cgs_result_return_hex`).
   - **Hidden Bug**:
     - Inside `cgs_dispatcher_slot` (`:2326`), `cgs_cell_slots` (`:2260`), and `cgs_cell_slot` (`:2359`), the loops over `"locals"` call `cgs_cell_width(bytes, fn, cell_index)` -> `cgs_cell_ty` -> `cgj_array_item(bytes, cct_locals, index)`, discarding the loop cursor and rescanning `"locals"` from index 0 on every cell ($O(L^2)$).
     - Every `read_local` and `write_local` instruction calls `cgs_cell_slot(bytes, fn, local)`, which recomputes `cgs_dispatcher_slot(bytes, fn)` ($O(I + L^2)$) and `cgs_cell_slots(bytes, fn)` ($O(L^2)$) from scratch.
     - `cgs_branch_hex` and `cgs_jump_hex` recompute `cgs_dispatcher_slot(bytes, fn)` on every basic block even though their caller `cgs_block_terminator_hex` (`:6957`) already holds `dispatcher: U64`.
   - **Optimization Direction**:
     - Replace `cgs_cell_width(bytes, fn, index)` inside cursor loops with direct `cgs_cell_flat_hex(bytes, cgj_member(bytes, cell_off, "ty"))`.
     - Thread the already-computed `dispatcher: U64` from `cgs_block_terminator_hex` into `cgs_branch_hex`, `cgs_jump_hex`, and `cgs_result_return_hex`.
     - Precompute `cindex: List[U64]` once per function in `cgs_function_instr_hex` (`2k -> cell_slot`, `2k+1 -> cell_width`) so `cgs_write_local_hex` and `cgs_read_local_hex` become $O(1)$.

5. **[A5 — MEDIUM] Remaining index-based `cgj_array_item` loops in `selfhost/codegen.sico`**
   - **Location**: `cgs_instruction_for_value` (`:4737`), `cgs_result_arm_ty` (`:5007`), `cgs_match_condition_hex` (`:6861`), `cgs_match_hex` (`:6912`), `cgs_construct_hex` (`:2601`), `cgs_variant_hex` (`:2690,2732`), `cgs_record_field_offset` (`:2391`), `cgs_record_field_slots` (`:2425`).
   - **Optimization Direction**: Convert all remaining `cgj_array_item` loops to `cgj_array_first` / `cgj_array_next` cursors; in particular, `cgs_instruction_for_value` and `cgs_result_arm_ty` can also be indexed in `vindex` (storing the instruction offset alongside `(slot, ty)` as a stride-3 `vindex`).

---

### 2.2 Category B — M22 Guest Frontend (`selfhost/parser.sico`) Structural Bottlenecks

1. **[B1 — HIGH] Specialized lowering-machine fragmentation ("Self-Hosting Shape Frontier")**
   - **Location**: `selfhost/parser.sico` (`straight_*`, `fixed_*`, `gw_*`, `record_variant_function_ir`).
   - **Problem**: Functions without a `while` loop are routed to narrow straight-line/guard-chain lowerers rather than the general `gw_*` machine. As a result, valid Sico constructs that work inside a `while` function (such as `match sico.map.get[Text,U64](...)` or mutable `List`/`Map` cells or inline `//` comments inside function bodies) fail with `ERR:E-SH-IR-STATEMENT` or `ERR:E-SH-IR-CALL-SHAPE` when placed in a loop-free function.
   - **Optimization Direction**: Allow loop-free functions that are declined by the fast straight-line routes (`"SKIP"`) to fall through to the general block/statement machine (`gw_*` with 0 loops) before emitting `ERR:E-SH-IR-STATEMENT`.

2. **[B2 — MEDIUM] 24 `parser.sico:scalar_ir` sites hitting the 8 MiB stdout cap**
   - **Location**: `runner/sico-runner/src/lib.rs:3599` vs `selfhost/parser.sico:scalar_ir` closures (~1 MiB source -> >8 MiB canonical IR JSON).
   - **Optimization Direction**: Measure the exact IR JSON byte size of `scalar_ir` and either compact the closure extraction (excluding unreachable helper functions) or adjudicate a typed IR binary/streaming emission path per `AGENTS.md §10.1.3`.

---

### 2.3 Category C — Rust Compiler, Runtime & Runner Hidden Bugs on `dev`

1. **[C1 — HIGH] Fail-open user-import world emission in `crates/sico-codegen-wasm/src/canonical.rs:1366-1370`**
   - **Problem**: In `wrap_script_component`, when `user_interface_instance(import)` returns `None`, the loop executes `continue;` (silently dropping the component-level import) despite the comment stating `refuse instead of emitting a wrong world`, while `script_transport_imports` (`crates/sico-codegen-wasm/src/lib.rs:1530`) unconditionally emits the core import. Because the A7 value-set gate (`lib.rs:1188`) only checks parameter types and not return types (`canonical.rs:500`), an unsupported user-import return type produces a broken component instead of a typed `CodegenError`.
   - **Fix Direction**: Return `Result<Vec<u8>, CodegenError>` from `wrap_script_component` (or gate user-import return types in `lib.rs:1188`) so unsupported signatures fail closed.

2. **[C2 — HIGH] Unbounded heap memory leak via `.leak()` / `Box::leak` in `crates/sico-codegen-wasm/src/canonical.rs:432,1365`**
   - **Problem**: `user_interface_instance` (`:432`) calls `format!("p{index}").leak()` and `wrap_script_component` (`:1365`) calls `Box::leak(import.name.clone().into_boxed_str())` on every component build. In long-lived processes (`sico-language-server`, `sico-runner` test binaries, or daemon services), every compilation permanently leaks heap allocations.
   - **Fix Direction**: `wasm_encoder::InstanceType` / `ComponentBuilder` accept `&str` lifetimes tied to the local call frame or an arena `Vec<String>` owned by the function; pre-allocate parameter names `"p0".."pN"` in a local `Vec<String>` and borrow `&str` without `Box::leak`.

3. **[C3 — HIGH] Resource slot leak in `runner/sico-runner/src/http2.rs:623` (`bodies_live` never decremented on `finish`)**
   - **Problem**: `upload-pipe.finish` removes the upload handle via `table.delete(handle)`, which does not invoke the Wasmtime resource destructor (`:350/:366`). Consequently, `http2.bodies_live` is never decremented on successful uploads, and after `MAX_LIVE_BODIES` (16) uploads in a Store, all subsequent `open-upload` calls permanently fail with `HttpError::Limit`.
   - **Fix Direction**: Decrement `http2.bodies_live` on the `finish` path when `table.delete(handle)` succeeds.

4. **[C4 — HIGH] `in_flight` request counter leak and double-decrement in `crates/sico-http-provider/src/engine.rs:151-156,708,918-933`**
   - **Problem**:
     - `HttpEngine::execute()` increments `in_flight` via `self.begin_request()?` (`:151`) and then calls `self.execute_inner(request)` without calling `self.end_request()` when `execute_inner` returns `Err`. After 16 failed requests (`MAX_IN_FLIGHT = 16`), the engine permanently rejects all subsequent requests with `HttpError::Limit`.
     - In `UploadPipe::finish()`, `worker_in_flight` (a clone of `self.in_flight`) is passed into the returned `ResponseBody` (`:708`), while `UploadPipe::drop` (`:926`) ALSO calls `release_slot` when `done.is_some()`, decrementing `in_flight` twice for a single request.
   - **Fix Direction**: Use an RAII guard for `in_flight` in `execute()`, and transfer ownership of the in-flight slot to `ResponseBody` upon successful `UploadPipe::finish()` so `UploadPipe::drop` only releases the slot when aborted before producing a `ResponseBody`.

5. **[C5 — MEDIUM] Cross-branch divergence between `dev` and `origin/codex/m24-vision-ui-v2`**
   - **Problem**: `origin/codex/m24-vision-ui-v2` (`615c6ef..6274ae6`) diverged from `dev` (`615c6ef..74098aa`): `dev` carries `crates/sico-ui-renderer-d2d` (`STEP-0577..0607`) and M22 `STEP-0562..0626` on Rust `1.98.0`, whereas `origin/codex/m24-vision-ui-v2` carries `crates/sico-ui-d2d`, `guests/image-codec` JPEG encode/decode, `RFC-0056` (`sico.bytes.read_u32_le`), `ADR-0020` (DirectML), native file picker `picker.rs`, and colliding `STEP-0552..0561` numbers on Rust `1.100.0`.
   - **Fix Direction**: Port `guests/image-codec` JPEG support, `RFC-0056`, `ADR-0020`, the native file picker, and the C1/C3/C4 fixes onto `dev` (`sico-ui-renderer-d2d`) under fresh sequential STEP numbers (`STEP-0627+`), then retire `origin/codex/m24-vision-ui-v2`.
