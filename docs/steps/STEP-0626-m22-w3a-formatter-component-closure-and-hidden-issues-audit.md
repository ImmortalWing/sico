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
   - **Topology**: Both branches share merge-base `615c6ef` (immediately after `95b80cd Merge branch 'codex/m24-vision-ui' into dev`, `STEP-0330..0337`). From `615c6ef`, two parallel M24 tracks diverged:
     - **`dev` track (`af6dcb7`..`250d5c3`, `STEP-0338` + `STEP-0577..0626`)**: Landed `crates/sico-ui-renderer-d2d` (offscreen DIB + Direct2D renderer + `WinShell` + `WindowedUiBackend` + `layout.rs` hit-testing), `runner/sico-runner/src/ui.rs` (`D2dUiRenderer` + `route_click_event`), `STEP-0578` provenance runtime budget enforcement, `STEP-0587` provider crash isolation, `STEP-0588/0593/0596` M7 registry roster/codec consumers, and `STEP-0589/0607` M24 exit audit refresh (`gate 3` and `gate 6` GO, overall NO-GO) — plus the entire M22 `STEP-0338..0626` self-host compiler history.
     - **`codex/m24-vision-ui-v2` track (`a142cc3..6274ae6`, 15 commits, `STEP-0552..0561` — colliding STEP numbers with M22)**: Landed a competing Direct2D crate `crates/sico-ui-d2d` + `runner/sico-runner/src/ui_d2d.rs` (`STEP-0552..0553`), source-level 13-function `sico.ui.*` stdlib (`STEP-0554..0556`, flattening `poll-event` to `list<string>`), native file open/save dialog `wit/script-path-picker-v0/path-picker.wit` + `runner/sico-runner/src/picker.rs` + `crates/sico-ui-d2d/src/picker.rs` (`STEP-0557`), baseline JPEG encode/decode in `guests/image-codec/src/lib.rs` (`jpeg-decoder 0.3.2` + `jpeg-encoder 0.7.1`, `c528408`/`c2f6818`), `RFC-0056` (`sico.bytes.read_u32_le`), `ADR-0020` (DirectML), Rust toolchain `1.98.0 -> 1.100.0` (`STEP-0559`), and provenance budget runtime (`STEP-0560..0561`).
   - **Why direct `git merge` is unsafe**: Merging `codex/m24-vision-ui-v2` directly into `dev` would introduce duplicate Direct2D crates (`sico-ui-d2d` vs `sico-ui-renderer-d2d`), incompatible `wit/script-ui-v1/ui.wit` `poll-event` signatures (`list<string>` vs `option<ui-event>`), duplicate STEP numbers (`STEP-0552..0561`), and several Win32/Direct2D safety bugs in `sico-ui-d2d` (see Category D below).
   - **Fix Direction**: Keep `dev` (`crates/sico-ui-renderer-d2d`) as the single canonical base, selectively port `guests/image-codec` JPEG support, `RFC-0056`, `ADR-0020`, source-level `sico.ui.*` lowering (adapted to `dev`'s `ui.wit`), the native `path-picker@1.0.0` capability (with the `CoTaskMemFree` fix in D4), and the C1/C3/C4 fixes under fresh sequential STEP numbers (`STEP-0627+`).

---

### 2.4 Category D — M24 Native UI, Win32 Shell & Image Codec Hidden Bugs (`dev` vs `codex/m24-vision-ui-v2`)

1. **[D1 — HIGH (`dev`)] GDI `HDC` and `HBITMAP` resource leak on error paths in `render_frame`**
   - **Location**: `crates/sico-ui-renderer-d2d/src/render/win.rs:238-299` (`render_frame`).
   - **Problem**: Unlike `WinShell`'s `Dib` struct (`crates/sico-ui-renderer-d2d/src/shell/win.rs:138-155`) which implements `Drop`, `render_frame` creates raw `dc = CreateCompatibleDC(HDC(0))` (`:238`) and `bitmap = CreateDIBSection(...)` (`:242`) without a RAII guard and without checking `dc.0 == 0`. If `CreateDIBSection` (`:243`), `CreateDCRenderTarget` (`:259`), `target.cast()` (`:262`), `target.BindDC()` (`:272`), `EndDraw()` (`:287`), or `byte_len` overflow (`:292`) returns `Err` via `?`, `SelectObject(dc, previous)`, `DeleteObject(HGDIOBJ(bitmap.0))`, and `DeleteDC(dc)` are skipped, permanently leaking GDI handles (Windows enforces a strict 10,000 GDI handle process limit) and DIB section memory.
   - **Fix Direction**: Check `dc.0 == 0` after `CreateCompatibleDC` and wrap `(dc, bitmap, previous)` in a local RAII `OffscreenDibGuard` whose `Drop` restores `previous`, deletes `bitmap`, and deletes `dc` on all return paths.

2. **[D2 — MEDIUM (`dev`)] Dual diverging UI layout engines between visual rendering and click hit-testing**
   - **Location**: `crates/sico-ui-renderer-d2d/src/render/win.rs:306-461` (`render::win::layout`, using live DirectWrite `CreateTextLayout` metrics and `0x0` container own-boxes) vs `crates/sico-ui-renderer-d2d/src/layout.rs:197-350` (`layout::compute`, using token-proxy character widths and full container bounding boxes) wired in `runner/sico-runner/src/ui.rs:304-316` (`route_click_event`).
   - **Problem**: Visual rendering (`render_frame`) and pointer hit-testing (`route_click_event`) compute control positions with two different layout algorithms. In rows (`NodeKind::Row`) where buttons or text nodes precede another control, DirectWrite proportional glyph widths diverge from the token-proxy width formula in `layout.rs`, causing clicks near control boundaries to hit-test against a shifted rectangle relative to the pixels actually drawn on screen.
   - **Fix Direction**: Unify rendering and hit-testing onto a single layout pass (and re-freeze the 4 frame digests in `docs/evidence/m24/ui-frame-corpus.json` once authorized per `RFC-0052-A2` item C-C).

3. **[D3 — MEDIUM (`dev`)] `WindowedUiBackend::poll_event` silently discards `WindowEvent::Paint` and `WindowEvent::Resized` without re-presenting**
   - **Location**: `runner/sico-runner/src/ui.rs:277-282, 388-425` (`classify_window_event` and `D2dUiRenderer::poll_event`) and `crates/sico-ui-renderer-d2d/src/shell/win.rs:239-278` (`WM_SIZE` / `WM_PAINT`).
   - **Problem**: In `shell/win.rs:250-278`, `WM_PAINT` calls `BeginPaint`/`EndPaint` (validating the window region without blitting the held `Dib`) and queues `WindowEvent::Paint`. Then `D2dUiRenderer::poll_event` (`runner/sico-runner/src/ui.rs:280`) classifies `WindowEvent::Paint` and `WindowEvent::Resized` as `None` (with a comment claiming "it drives a re-render and never crosses the boundary"), **but never actually triggers a re-blit or re-render**! Consequently, if an OS window is uncovered or resized while the guest is polling events, the validated client area goes blank until the guest explicitly calls `run()` again.
   - **Fix Direction**: In `WinShell`'s `WM_PAINT` handler (or when `WindowEvent::Paint` / `Resized` is pumped in `WindowedUiBackend::poll_event`), automatically re-blit the held `Dib` (or re-render at the new client size) before continuing the pump loop.

4. **[D4 — HIGH (`codex/m24-vision-ui-v2`)] Win32 COM memory leak in `IShellItem::GetDisplayName` in native file picker**
   - **Location**: `crates/sico-ui-d2d/src/picker.rs:250-259` (`run_dialog` on `codex/m24-vision-ui-v2`).
   - **Problem**: `shell_item.GetDisplayName(SIGDN_FILESYSPATH)` allocates a UTF-16 string buffer via the COM task allocator (`CoTaskMemAlloc`) and returns a raw `PWSTR`. `picker.rs` converts it via `path.to_string()` and returns without calling `CoTaskMemFree(Some(path.0.cast()))`, leaking the COM buffer on every file selection (and also if `path.to_string()` returns `Err`).
   - **Fix Direction**: When porting `picker.rs` to `dev`, copy `path.to_string()` into a local `result` and unconditionally call `unsafe { CoTaskMemFree(Some(path.0.cast())) }` before returning `result`.

5. **[D5 — HIGH (`codex/m24-vision-ui-v2`)] `WaitMessage()` wakeup race / deadlock, re-entrant `RefCell` hazard, and Win32/Direct2D cleanup leaks in `crates/sico-ui-d2d/src/d2d.rs`**
   - **Location**: `crates/sico-ui-d2d/src/d2d.rs:117, 133-137, 180-245, 250-310, 339, 356-388, 571-609` (on `codex/m24-vision-ui-v2`).
   - **Problem**:
     1. **Wakeup lost -> deadlock (`:212-243`)**: `ui_thread` drains `commands.try_recv()`, then drains `PeekMessageW(..., PM_REMOVE)` (which consumes `WAKE_MESSAGE`), and then calls `WaitMessage()`. If a caller thread calls `send()` (`commands.send` + `PostThreadMessageW(WAKE_MESSAGE)`) between `commands.try_recv()` returning `Empty` and `PeekMessageW` finishing, `PeekMessageW` eats `WAKE_MESSAGE` while `command` remains in `commands`, and `WaitMessage()` sleeps indefinitely, deadlocking `open_window`/`draw_frame`!
     2. **`RefCell` held across `CreateWindowExW` (`:251-294`) & `hwnd` leak (`:303`)**: `create_window` holds `STATE.borrow_mut()` while calling `CreateWindowExW` (which synchronously invokes `window_proc` on the same thread), and if `CreateHwndRenderTarget` fails at `:303`, the newly created `hwnd` is leaked without `DestroyWindow`.
     3. **`BeginPaint`/`DestroyWindow`/`EndDraw`/`CoUninitialize` violations (`:193, :339, :361, :372-386, :573-609`)**: `WM_PAINT` calls `EndPaint` unconditionally without checking `BeginPaint`'s return value (violating `AGENTS.md §7`); `paint()` and `render_offscreen()` skip `EndDraw()` when `paint_frame()?` fails; `ui_thread` (`:193`) and `render_offscreen` (`:573`) return without calling `CoUninitialize()`.
     4. **Unbounded event channel (`:117`)**: `let (event_tx, event_rx) = channel()` uses an unbounded channel instead of the `RFC-0052 §3` bounded coalescing queue.
   - **Fix Direction**: Do **not** merge `crates/sico-ui-d2d` from `codex/m24-vision-ui-v2`; keep `dev`'s `crates/sico-ui-renderer-d2d` (which already uses a clean channel-driven worker thread without `WaitMessage`/`PostThreadMessageW` races, enforces bounded `MAX_QUEUE_EVENTS` coalescing, and pins `!Send + !Sync` + `BeginPaint`/`DestroyWindow` invariants per `STEP-0585`).

6. **[D6 — MEDIUM (`codex/m24-vision-ui-v2`)] `guests/image-codec/src/lib.rs` JPEG MCU buffer budget on small subsampled JPEGs**
   - **Location**: `guests/image-codec/src/lib.rs:291-293` (on `codex/m24-vision-ui-v2`).
   - **Problem**: `decode_jpeg_inner` sets `decoder.set_max_decoding_buffer_size((pixels * 8).min(u32::MAX))` (`:292`). For small odd-dimensioned baseline JPEGs (e.g. `1×1` or `3×3` with `2×2` chroma subsampling `4:2:0` where `jpeg-decoder` allocates full `16×16` MCU blocks during IDCT upsampling), `16 * 16 * 3 = 768` bytes exceeds `pixels * 8 = 8` bytes, causing `jpeg-decoder` to reject valid small subsampled JPEGs as `"oversized"`.
   - **Fix Direction**: When porting `guests/image-codec` JPEG support onto `dev`, set the `jpeg-decoder` buffer ceiling to `(pixels.saturating_mul(8) + 4096).min(MAX_DECODE_BUFFER_BYTES)` (accounting for MCU block padding up to `16×16` while still strictly bounded by `MAX_PIXELS`).
