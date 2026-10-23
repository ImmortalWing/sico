# M22 Compiler self-host track

> Status: in progress / NO-GO after STEP-0485 exit re-review (2026-10-20, [audit](../reports/m22-exit-audit-2026-10-20.md)): gates 1/4/5/7 GO, gate 2 PARTIAL, gates 3/6 NO-GO. W2 line since STEP-0352 reached census 149/149 byte-exact + full-source parser.sico self-compile lowering (exit 0, byte-equal) via STEP-0484 arena treadmill; S5 component-codegen wave opened at STEP-0486, whose first slice (STEP-0487) emits a real script Component byte-equal to the Rust backend, and whose second slice (STEP-0488) locates the blocker in front of the IR-driven path: the guest frontend still cannot lower Script-profile shapes (record projection / variant / record construct) — STEP-0491 closes the first of those three (declared-record field projection now lowers to the canonical `project` op byte-equal to the Rust oracle; variant and record construct still refused) and STEP-0492 generalizes that projection from a hardcoded `ScriptInput` base to any declared record, and STEP-0493 closes the named-return-type precondition for record construction / variants (the guest now lowers `returns <declared record>` to the canonical `{"kind":"named",...}` return type byte-equal to the Rust oracle, and the malformed empty-kind IR that named return types used to emit is now a typed refusal), and STEP-0494 closes the record construct itself (`ScriptOutput(stdout: ..., ...)` lowers to the canonical `construct` op with a per-field instruction ledger, byte-equal to the Rust oracle), and STEP-0495 closes the third and last Script-profile frontend shape — the built-in `Result` variants `ok(...)` / `error(...)` lower to the canonical `variant` op (payload lowered first, the variant referencing those result ids, `return_type` always in declared `ok`/`error` order), byte-equal to the Rust oracle, with the payload type checked against the declared arm so a mismatched payload stays a typed refusal instead of a malformed IR; STEP-0495 also registers a newly measured capacity boundary — the whole-module canonical IR grew to 8,608,383 B and no longer fits one Script v0 channel (RFC-0029 freezes a channel at 8 MiB), so the guest compiler gained an `--emit-ir-split` driver mode that halves the IR across stdout and stderr for the full-source census. The three STEP-0488 frontend gaps (projection / construct / variant) are therefore closed; STEP-0496 opens the **guest IR ingestion** half of S5 — `selfhost/codegen.sico` gains a byte-oriented canonical-IR scanner plus a `run` emission-plan extractor, `selfhost/compiler.sico` gains the `--emit-ir-plan` driver mode, and the plan text the guest derives from the scanned IR is byte-identical to the plan the Rust oracle derives from the same `sico_ir::Module` (the oracle-side generator lives in the test file, not in the oracle crate), with four typed refusal identities (`ERR:E-SH-CG-IR-PLAN` / `-TYPE` / `-OP` / `-TERMINATOR`) for the shapes outside the plan subset and a new reproducible probe `tools/probe-m22-ir-plan-frontier.ps1`; what remains on gate 3 is run-body emission, the 37-corpus / formatter / full-source canary ladder and the ADR-0015 `A == B == C` + `.sapp` chain. STEP-0497 is a CI repair slice on this line, not a gate advance: it closes the M23-session CI red point (`sico-app-cli::run_executes_a_script_package_end_to_end`, `forcing trap when growing memory to 2214592512 bytes`) by aligning the Script package's memory limit with the guest linear memory every script component declares (33,792 Wasm pages = 2,214,592,512 B, emitted with minimum == maximum) — the binding constraint was the **package manifest's** 64 MiB (inherited from `RuntimeLimits::default()`), not the Host's 2 GiB grant, so raising only the Host grant left the case red; the fix keeps the emitted bytes unchanged (no `MEMORY_PAGES` change, no snapshot regeneration), raises the package limit to the declared memory, raises the Host grant to 2,560 MiB (the same value the in-process runner path enforces) and turns "ceiling below the declared minimum" into a typed refusal at both the package and the runtime layer, with four runtime-binary-free invariant guards (codegen byte-level / package / runtime / runner) and a matching RFC-0029 STEP-0497 amendment that separates the 2 GiB arena ceiling from the declared linear memory and registers the "stricter package/Host limit" clause as unimplementable for script components; the independent CI run 39524849306 for commit 7e5ad3c concluded `success`, so the handed-over red point is green on an independent runner and STEP-0497 is closed; STEP-0499 opens the emission half — a guest-side slot / local-declaration plan (`cgs_run_function` / `cgs_flat_hex` / `cgs_record_hex` / `cgs_local_hex` in `selfhost/codegen.sico`, the `--emit-locals-hex` driver mode in `selfhost/compiler.sico`) whose bytes are byte-identical to the local declaration section the Rust backend actually emits, and the check reads the oracle's production output (the component `compile_script_program` really produces) instead of a re-implementation of the plan generator; the emission is split in two precisely so that a mismatch in the next slice can be attributed to the slot plan or to the instruction emission; the independent CI run 39535461745 (head_sha ee2317f) concluded `success` with all 12 inner steps green, so the slice is closed on an independent runner and not only on this workstation, and the environment-sensitive budget assert it registered passes there; STEP-0501's independent CI run 39549599204 (head_sha a730ac6) concluded `success` with all 12 inner steps green as well, so the measurement slice is closed too; STEP-0502 then emits the instruction segment -- the guest derives the run body's instruction segment (dispatcher shell, per-block dispatch heads, block instructions) from the ingested IR, and the bytes are identical to the segment the Rust backend actually emits (182 hex), checked against the oracle's production output; the guest gains 28 new functions and a `--emit-run-instr-hex` driver mode, two typed refusal identities cover the shapes outside the subset, and the slice records four measured guest-frontend constraints (the `record` keyword may not name a parameter, `let` has no block scope inside a function, `U64.equal` yields a Bool, and a "not found" sentinel may not double as a total) plus two self-inflicted defects the byte comparison caught; the next slices are pre-cut: STEP-0503 (guest emits the Script result-area return, which needs five frozen component-level facts), STEP-0504 (the derived run body replaces STEP-0487's frozen hex). gate 3 is untouched and remains NO-GO. STEP-0502 was committed as `ee71719` and pushed to `github/codex/m22-w1-ci`; its independent CI run 39556951856 (run #24, head_sha ee71719) concluded `success` with all 12 inner steps green, so the instruction-segment slice is closed on an independent runner as well; STEP-0503 emits the Script result-area return and the dispatcher epilogue -- the guest now derives the whole run body (360 hex = 180 B) from the ingested IR and it is byte-identical to the body the Rust backend actually emits, checked against the oracle's production output; the five component-level facts (allocator index 1, result area 32 B, payload offset 8, the `ScriptOutput` WIT field offsets and record order) are carried by the guest as frozen facts, while the discriminant slot, the payload field slots and the scratch pair are re-derived from the IR and cross-checked against those facts, refusing a mismatch instead of silently preferring one side; the `error` arm and the oracle's dual-payload shape are refused because the reference form never triggers them and no measured truth exists for them; STEP-0503 was committed as `7cf2ece` and pushed to `github/codex/m22-w1-ci`; its independent CI run 39572167889 (run #25, head_sha 7cf2ece) concluded `success` with all 12 inner steps green, so the result-area slice is closed on an independent runner as well; the next slice is pre-cut: STEP-0504 (the derived run body replaces STEP-0487's frozen hex, which makes the whole reference component byte-equal). STEP-0504 now closes that slice: the frozen run-body constant is deleted from the guest source and replaced by `cg_leb_hex(group count) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`, so the guest's emitted Component is byte-identical to the Rust backend's production output for the reference form (the component is 981 B; the core module is 530 B with no data section; the `run` body is 223 B of it and is the only IR-dependent part; the other 758 B are still the frozen component skeleton and the three frozen helper bodies); the proof that the emission follows the IR rather than replaying a constant is a byte-level differential: a second reference form differing only in `I64.literal(0)` vs `I64.literal(7)` must produce a different Component, and the two differ in exactly one byte -- the SLEB immediate of the `i64.const` carrying `exit_code` at component byte 567 -- while both are byte-equal to the backend and both pass `wasmparser`; the guest additionally refuses a module with a second function, because the frozen function section (run/alloc/realloc/post) cannot hold the extra core function the backend would emit, and silently dropping it would produce a plausible-looking but semantically wrong Component (ADR-0016 decision 6). gate 3 is still untouched and remains NO-GO: what the slice establishes is the content leg "guest Component == backend Component" for one shape, not ADR-0015's three-stage `A == B == C` runner execution, and the 37-corpus ladder, the formatter and full-source canaries, and the `.sapp` chain are all still open. STEP-0504's local validation battery is 11/11 `BATTERY_GREEN` (fmt runner 00:04 / one-key CI 12 steps CI GREEN 05:08 / 0245 00:56 / 0262 02:17 / frontend frontier probe 18/18 02:08 / IR plan frontier probe 8/8 00:25 / 0124 00:00 / 0131 00:01 / 0156 00:00 / whitespace 00:00 / line-ending audit 00:01) with the firmware suite at 115/115, re-measured on the corrected tree; the slice also records two self-inflicted defects (a CRLF working copy of `selfhost/codegen.sico`, the only such file in the tree, and an exit-code plumbing bug in the battery script that swallowed the line-ending audit's failure while still printing `BATTERY_GREEN`), and the owner scoped the next slice on 2026-10-21 as STEP-0505 = widen the component emission to the non-reference forms, starting with the `error` arm, which is the only one of the three candidate widenings that yields a measured truth from a single new fixture. STEP-0505 took that truth: the backend really emits for the `error` arm, so the `ScriptError` result-area offsets are measured (`code` at 8 as one byte, `message` at 12 and 16) and the error arm's result segment is 59 B with a 17-group local declaration section; the slice's real finding is that the guest stops earlier than expected -- in the IR construction, not in the codegen -- because the Rust frontend lowers an enum case as an empty-payload `variant` op and the guest's construct lowering has no such operand shape, so the next slice splits into a frontend knife and a codegen knife. STEP-0504 was committed as `579645b` and pushed to `github/codex/m22-w1-ci`; its independent CI run 39585258425 (run #26, head_sha 579645b) concluded `success` with all 12 inner steps green, so the derived-component slice is closed on an independent runner as well -- an independent verdict proves that slice, not gate 3. STEP-0505 then measured the error arm: the backend really emits for it, so the `ScriptError` result-area offsets are measured rather than derived, and the guest turns out to stop earlier than expected -- in the IR construction, because the Rust frontend lowers an enum case as an empty-payload `variant` op and the guest construct lowering has no such operand shape; the local battery is 11/11 BATTERY_GREEN and the firmware suite 116/116. the shapes outside the subset (`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-FLAT`); STEP-0501 measures the remaining truth rather than implementing it: the `run` body the Rust backend actually emits is read from the production component, decoded byte by byte into a reproducible emission table (dispatcher shell / block dispatch / project / const_i64 / construct / variant / Script result-area return / epilogue), and pinned by a new firmware test together with an unambiguous cut marker (`i32.const 32` + `call 1`) that separates the instruction segment (182 hex) from the result-area return plus epilogue (178 hex). The measurement also settles a fact gate 3 depends on: the reference Script profile emits no data section, so the run body is the only IR-dependent part of the component; the slot table it yields is byte-identical to STEP-0499's local declaration plan. The next slices are pre-cut: STEP-0502 (guest emits the instruction segment, no component-level dependency), STEP-0503 (guest emits the Script result-area return, which needs five frozen component-level facts), STEP-0504 (the derived run body replaces STEP-0487's frozen hex). gate 3 is untouched and remains NO-GO. S1/S2 declared subsets complete; S3/S4/S5 partial; S6 not entered; S7 reviewed NO-GO. STEP-0300 independent CI succeeded on f2fd7fc (run 36212509946), and the 8bee75b snapshot containing STEP-0301–0305 succeeded (run 36366432845). STEP-0306–0309 are local only and advance the formatter canary to 29/30, with `main` / SCICOND next and R3 consecutive stall count 0. ADR-0017 Option A and RFC-0047 are accepted. The parallel origin/dev W1/W2 history remains unmerged; neither branch CI nor this review proves M22 completion. See [2026-09-28 exit review](../reports/m22-exit-audit-2026-09-28.md).

## 1. Objective

Rebuild the compiler frontend and language tooling in Sico itself, in two
declared levels:

- **L1 — tool self-host**: a Sico-written formatter and checker subset
  that reproduces the Rust tools' outputs byte-exactly on the frozen
  corpora.
- **L2 — compiler self-host**: a Sico-written compiler for the Script
  profile v0 subset (lex → parse → check → lower → codegen) that
  compiles the frozen corpus byte-identically to the Rust backend and
  then compiles **its own source** (bootstrap closure), verified by the
  existing deterministic-backend oracle (RFC-0011).

The Rust runner, Host providers, DAP and MCP server stay native by
architecture (authority, least-privilege, isolated FFI). Self-host is a
compiler/tooling milestone, not a de-Rust goal; the Rust implementation
remains the permanent differential oracle and is not retired.

## 2. Entry gate

| # | Condition | Status (measured 2026-09-14) |
|---|---|---|
| 1 | M19 CI/release engineering green | **satisfied** — STEP-0168; run-ci 10-step green at v0.1.0 |
| 2 | Byte-level differential oracle exists | **satisfied** — RFC-0011 deterministic backend + frozen snapshot corpora + byte-exact formatter |
| 3 | Language prereq RFC accepted and implemented: byte/text access, collection extension (M21 stdlib batch 2 — shared work, not duplicated) | **satisfied** — RFC-0045 + STEP-0174 (2026-09-14) |
| 4 | v1 batch 2 decisions that the self-host frontend consumes (bare-literal typing, error propagation) accepted | **satisfied** — RFC-0046 + STEP-0175 (2026-09-14); for-loops explicitly NOT required (while suffices), landed anyway |
| 5 | Bootstrap architecture ADR accepted (differential harness, artifact packaging through the M7 trust chain, budget re-measurement policy) | **satisfied** — ADR-0015 / STEP-0198 (2026-09-17): `A == B == C`, Rust oracle retained, canonical `.sapp`, frozen evidence/budget record |
| 6 | M18 combined portfolio still runs without core patches | **satisfied (4/5)** — STEP-0165 |

Entry work (condition 3/4) is the same M21 batch-2 track; M22 does not
fork it.

## 3. Slices

Each slice has a bounded exit test; no slice may widen the declared
language surface without an accepted RFC.

- **S0 — language prereqs**: byte/text access + collection extension
  landed with content-asserting corpora (the M21 batch-2 slice; M22
  consumes it).
- **S1 — L1 formatter**: Sico-written formatter for the Script profile;
  differential vs `sico format`: byte-exact on `syntax-candidates/`,
  `semantic-cases/` and `tests/end-to-end/` fixtures, plus idempotence
  (`format(format(x)) == format(x)`).
- **S2 — L1 checker subset**: Sico-written checker producing the declared
  diagnostic subset (outline + the E1xxx identities the corpus freezes);
  differential on the same corpus. The Rust checker remains the oracle;
  Sico v0 must match refusals exactly or produce a declared, disjoint
  subset (typed, no silent divergence).
- **S3 — L2 lexer + parser**: token/AST as declared data shapes
  (struct-of-arrays is an accepted transition shape while List[record]
  stays outside the executable set); equivalence proxy = canonical
  formatter output round-trip (S1 consumes it), not raw spans.
- **S4 — L2 semantics + lowering**: emit the typed IR; **the Rust IR
  verifier is the oracle** (the verifier is NOT rewritten in Sico);
  IR compared via its canonical serialization on the corpus.
- **S5 — L2 codegen**: the RFC-0011 deterministic backend subset for the
  Script profile; component bytes compared byte-exactly against Rust
  output on the corpus.
- **S6 — bootstrap closure**: the Sico compiler compiles its own source.
  Gate: Rust-compiled artifact A and Sico-compiled artifact B are
  byte-identical (or a declared, ADR-approved equivalence class); B
  re-passes the full corpus; B is packaged as a `.sapp` through the M7
  trust chain and executed via `sico-app`; fuel/timeout/wall-time of the
  self-compile measured against the runner budgets (budget record, no
  SLA claim).
- **S7 — exit audit**: evidence pack, budget tables, dual-implementation
  register, explicit GO/NO-GO.

### 3.1 Historical slice baseline (measured 2026-09-23, through STEP-0262)

| Slice | Status | Closed evidence | Remaining bounded work |
|---|---|---|---|
| S0 prereqs | complete | STEP-0174 / STEP-0175 | — |
| S1 formatter (L1) | complete | frozen corpus byte-exact + idempotent; `formatter.sico` runs on its own sources through the real runner | — |
| S2 checker subset (L1) | complete for the declared subset | frozen diagnostic partition 116 lexical / 34 identity / 65 accepted / 0 unsupported | widening the subset requires its own declared-contract update (no silent growth) |
| S3 lexer + parser (L2) | partial | `lexer.sico` / `tokens.sico` / `declaration_parser.sico` differentials green; `parser.sico` exercised through driver paths (recursive return-expression trees; verifier-accepted scalar IR + noncanonical refusals) | full declared-shape coverage of `parser.sico` (12,202 lines, the largest selfhost unit) on the frozen corpus |
| S4 semantics + lowering (L2) | partial | local real-runner formatter prefix byte-exact through `normalize_source` (29 of 30 functions); typed refusal-first discipline with re-pinned canary each step; STEP-0306–0309 independent CI pending | `main`, then the remaining selfhost sources lower byte-exactly |
| S5 codegen (L2) | partial | STEP-0315/0322: bounded Core-Wasm seam (nonnegative `Int`/`I64`/`U64`/`Bool` constants, identity pairs, `less_than`, `checked_add`, `checked_sub`) byte-equal to Rust codegen, every other shape typed-refused; STEP-0487 adds the first real script Component emission (981 B, byte-equal to the Rust backend, frozen minimal source shape only); STEP-0488 locates the next blocker: the guest frontend cannot yet lower Script-profile shapes (record projection / `ok(...)` variant / record construct), so the IR-driven emission path has no input yet; STEP-0491 closes the first of the three — a declared-record field projection (`input.stdin` on a `ScriptInput` parameter) lowers to the canonical `project` op byte-equal to the Rust oracle, with typed refusal for the unresolved variants, and STEP-0492 generalizes it from a hardcoded `ScriptInput` base to any declared record; STEP-0493 closes the named-return-type precondition — `returns <declared record>` lowers to the canonical `{"kind":"named",...}` return type byte-equal to the Rust oracle (straight pass-through of the named parameter), and the malformed empty-kind IR that such functions used to emit is now a typed refusal; STEP-0494 closes the record construct itself — `ScriptOutput(stdout: ..., ...)` lowers to the canonical `construct` op with a per-field instruction ledger (each field value first lowered into its own instruction, the construct referencing those ids), byte-equal to the Rust oracle, via a dedicated top-level helper family rather than a rewrite of the deep `scalar_ir` return analysis — so the IR-driven path now has all three Script-profile input shapes; STEP-0495 closes the third one — the built-in `Result` variants `ok(...)` / `error(...)` lower to the canonical `variant` op (payload lowered first, the variant referencing those result ids; `return_type` always in declared `ok`/`error` order regardless of which arm was constructed), byte-equal to the Rust oracle, with the payload type checked against the declared arm so a mismatched payload stays `ERR:E-SH-IR-VARIANT-TYPE` and an unsupported payload shape stays `ERR:E-SH-IR-VARIANT-PAYLOAD` instead of producing a malformed IR; STEP-0495 also registers a measured capacity boundary (the whole-module canonical IR reached 8,608,383 B and no longer fits one Script v0 8 MiB channel, hence the new `--emit-ir-split` driver mode that halves the IR across stdout and stderr); STEP-0496 opens the guest IR-ingestion half — a byte-oriented canonical-IR scanner and a `run` emission-plan extractor in `selfhost/codegen.sico`, the `--emit-ir-plan` driver mode in `selfhost/compiler.sico`, and a plan text that is byte-identical to the one the Rust oracle derives from the same `sico_ir::Module` (oracle-side generator in the test file only), with four typed refusal identities outside the plan subset and a new reproducible probe `tools/probe-m22-ir-plan-frontier.ps1`; STEP-0497 is a CI repair slice (not a codegen step): it closes the M23-session CI red point by aligning the Script package's memory limit with the guest linear memory every script component declares (33,792 pages = 2,214,592,512 B, minimum == maximum), the binding constraint having been the package manifest's 64 MiB rather than the Host grant, keeps the emitted bytes unchanged, and adds four runtime-binary-free invariant guards plus the matching RFC-0029 amendment — the independent CI run 39524849306 for commit 7e5ad3c concluded `success`, so the handed-over red point is green on an independent runner and the slice is closed; STEP-0499 opens the emission half — the guest derives the `run` slot / local-declaration plan (`cgs_run_function` / `cgs_flat_hex` / `cgs_record_hex` / `cgs_local_hex` plus the `--emit-locals-hex` driver mode) and the resulting bytes are byte-identical to the local declaration section the Rust backend actually emits, checked against the oracle's production output (the component `compile_script_program` really produces) rather than a re-implementation of the plan generator, with two typed refusal identities (`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-FLAT`); the plan is carried directly as local-declaration bytes so the next slice can attribute any mismatch to the slot plan or to the instruction emission; the independent CI run 39535461745 (head_sha ee2317f) concluded `success` with all 12 inner steps green, so the slice is closed on an independent runner and not only on this workstation; STEP-0501's independent CI run 39549599204 (head_sha a730ac6) concluded `success` as well, so the measurement slice is closed too; STEP-0501 measured the remaining truth instead of implementing it -- the `run` body the backend actually emits is read from the production component, decoded byte by byte into a reproducible emission table (dispatcher shell / block dispatch / project / const_i64 / construct / variant / Script result-area return / epilogue) and pinned by `oracle_run_body_emission_table_is_pinned`, with an unambiguous cut marker (`i32.const 32` + `call 1`) separating the instruction segment (182 hex) from the result-area return plus epilogue (178 hex); the measurement also settles that the reference Script profile emits **no data section**, so the run body is the component's only IR-dependent part, and its slot table is byte-identical to STEP-0499's local declaration plan (i64 slots 8/13/19); STEP-0502 emits the instruction segment -- the guest derives the run body's instruction segment (dispatcher shell / per-block dispatch heads / block instructions) from the ingested IR, and the bytes are byte-identical to the segment the Rust backend actually emits (182 hex = 91 B), checked against the oracle's production output; `selfhost/codegen.sico` gains 28 new functions (arithmetic / decimal / encoding / layout / slot / record / emission) and `selfhost/compiler.sico` the `--emit-run-instr-hex` driver mode; two typed refusal identities cover the shapes outside the subset (`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-EMIT`); the slice records four measured guest-frontend constraints (the `record` keyword may not name a parameter, `let` has no block scope inside a function, `U64.equal` yields a Bool, and a "not found" sentinel may not double as a total) plus two self-inflicted defects the byte comparison caught (an inverted mismatch flag, a broken "not found" sentinel); the next slices are pre-cut: STEP-0503 (guest emits the Script result-area return, needs five frozen component-level facts: allocator index 1, result area 32 B, payload offset 8, the two Script records' WIT field offsets), STEP-0504 (the derived run body replaces STEP-0487's frozen hex); run-body byte emission is closed for the reference form, whole-component equality is still open; STEP-0503 emits the Script result-area return and the dispatcher epilogue -- the guest derives the run body's result-area return (the 32-byte area allocation, the discriminant store, the frozen `ScriptOutput` fields copied into the area, the returned area pointer) and the six-byte dispatcher epilogue from the ingested IR, and the whole run body the guest now emits (360 hex = 180 B) is byte-identical to the body the Rust backend actually emits, checked against the oracle's production output; the component-level facts (allocator index 1, result area 32 B, payload offset 8, the `ScriptOutput` WIT field offsets) are carried by the guest as frozen facts, while the discriminant slot, the payload field slots and the scratch pair are re-derived from the IR and cross-checked against those facts, refusing a mismatch instead of silently preferring one side; the `error` arm and the oracle's dual-payload (`has_ok && has_error`) shape are refused, because the reference form never triggers them and no measured truth therefore exists for them; the slice also records one self-inflicted defect the byte comparison caught (a `call` written as a bare immediate instead of an opcode plus an immediate) and repairs two prose inaccuracies in the STEP-0501 record (the epilogue is six bytes, and `i32.store8`'s memarg is two LEBs); STEP-0502's independent CI run 39556951856 (head_sha ee71719) concluded `success` with all 12 inner steps green, so the instruction-segment slice is closed on an independent runner as well, and the environment-sensitive budget assert it registered passes there; STEP-0503's independent CI run 39572167889 (head_sha 7cf2ece) concluded `success` with all 12 inner steps green as well, so the result-area slice is closed on an independent runner too; STEP-0504 closes the derived-component slice: the frozen run-body constant is deleted from `selfhost/codegen.sico` and replaced by `cg_leb_hex(group count) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`, `cg_core_module_hex` / `cg_component_hex` / `component_hex_for_ir` now take the IR, and `--emit-component-hex` runs lex -> IR -> emit instead of comparing the source against a frozen fixture (the existing typed refusal is unchanged); the guest's emitted Component is byte-identical to the Rust backend's production output for the reference form (component 981 B, core module 530 B with no data section, `run` body 223 B, the other 758 B still the frozen skeleton and the three frozen helper bodies); the emission is proven to follow the IR by a byte-level differential -- a second reference form differing only in `I64.literal(0)` vs `I64.literal(7)` yields Components that differ in exactly one byte, the SLEB immediate of the `i64.const` carrying `exit_code` at component byte 567 -- and a module with a second function is typed-refused because the frozen function section cannot hold the extra core function; the next slice is pre-cut: STEP-0505 (extend the emission beyond the reference form -- the `error` arm, whose `ScriptError` result-area offsets have no measured truth yet, or multi-function / data-section modules), which the owner scoped on 2026-10-21 as "widen to the non-reference forms, starting with the `error` arm": the `error` arm is the only one of the three that yields a measured truth from a single new fixture, because a Script-profile `run` returning `error(...)` really does make the backend emit, so the `ScriptError` result-area offsets can be promoted from "frozen derivation" to "measured"; the local validation battery is 11/11 `BATTERY_GREEN` (fmt runner 00:04 / one-key CI 12 steps CI GREEN 05:08 / 0245 00:56 / 0262 02:17 / frontend frontier probe 18/18 02:08 / IR plan frontier probe 8/8 00:25 / 0124 00:00 / 0131 00:01 / 0156 00:00 / whitespace 00:00 / line-ending audit 00:01) and the firmware suite is 115/115, re-measured on the corrected tree; the slice also records two self-inflicted defects -- a CRLF working copy of `selfhost/codegen.sico` (the only such file in the tree; `.gitattributes` normalises it on commit, so the committed blob was unaffected but the local evidence bytes were not the bytes CI would check out) and an exit-code plumbing bug in the battery script that swallowed the line-ending audit's failure while still printing `BATTERY_GREEN`; STEP-0505 opens the non-reference-form wave by measuring the `error` arm: the Rust backend really does emit a Component for a Script-profile `run` that returns `error(...)` (939 B, accepted by the real validator), so the `ScriptError` result-area offsets are now measured rather than derived -- `code` at result-area offset 8 as a single byte (`i32.store8`), `message` at 12 (pointer) and 16 (length); the error arm's result segment is 59 B (the `ok` arm's is 89 B), its local declaration section is 17 groups / 18 slots against the `ok` arm's 21 groups / 22 slots, and it carries no 64-bit field at all; the slice also locates where the guest currently stops, which is not in the codegen but in the IR construction: the guest's `construct` field-value lowering accepts only `const_string`, a field projection and `I64`/`U64` literals, while the Rust frontend lowers an enum case as an empty-payload `variant` op (`{"op":"variant","data":{"name":"ScriptErrorCode.InvalidInput","payload":[]}}`) -- the guest therefore refuses the arm with `ERR:E-SH-IR-CONSTRUCT-VALUE` before any codegen runs, and the next slice is pre-cut into a frontend knife (add the enum-case operand shape) and a codegen knife (the `error` branch of the result-area return plus the error arm's slot plan); STEP-0504 was committed as `579645b` and pushed to `github/codex/m22-w1-ci`, and its independent CI run 39585258425 (run #26, head_sha 579645b) concluded `success` with all 12 inner steps green, so the derived-component slice is closed on an independent runner as well; the error-arm measurement slice lands with a local battery of 11/11 BATTERY_GREEN and a firmware suite of 116/116, and it records one self-inflicted defect -- a scripted constant backfill that broke rustfmt formatting, caught by the very first battery step | RFC-0011 deterministic backend subset over the frozen corpus, driven by the guest's canonical IR |; STEP-0506 lands the error-arm component slice locally: the guest emits the whole 939 B error-arm Component byte-equal to the backend's production output (new pins `sico_compiler_lowers_the_enum_case_operand_as_a_variant_byte_exactly` and `sico_codegen_guest_emits_the_error_arm_component_byte_exactly`), with the static data segment and the arena base derived from the IR instead of frozen; the firmware suite is 106/106 and the local battery 11/11, independent CI has not run, and gate 3 stays NO-GO |; STEP-0507 is prepared (entry gate satisfied, implementation not started): the multi-function module's component-level truth is measured from the Rust backend's production output (a two-function fixture's Component is 1017 B; the types/functions/exports sections, the per-function local-layout rule and the non-`run` direct return form are all pinned), and the guest frontend is measured to lower a two-function module's IR byte-equal to the Rust IR, so that slice needs no frontend knife — the call-in-expression-position shape is a separate knife |; STEP-0507 lands the multi-function component slice locally: the guest emits the whole 1017 B two-function Component byte-equal to the backend's production output (new pin `sico_codegen_guest_emits_the_multi_function_component_byte_exactly`), with the types/functions/ exports/code sections, the static data segment and the allocator core index (the module function count) all derived from the IR instead of frozen; the self-hosting shape frontier advanced with it (21 codegen functions lower byte-equal through the guest frontend, new pin `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`); the firmware suite is 108/108, independent CI has not run, and gate 3 stays NO-GO; STEP-0508 lands the call-in-expression-position frontend slice locally: a call in a construct field value lowers to the canonical `call` op byte-equal to the Rust oracle (new pin `sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly`), with the argument id base taken from the field's own result id and the construct's own id returned to its callers instead of re-derived; the slice also records and fixes one self-inflicted defect -- a comment written inside a function body, which the guest's lexer (it does not skip `//` comments) turns into an unclassifiable statement and which therefore regressed the full `parser.sico` self-host (the whole `selfhost/parser.sico` comment discipline is now: comments live between functions and carry none of the 17 words the guest's structural scans count); the shape frontier did not advance (8 new or rewritten functions lower byte-equal through the guest frontend), the firmware suite is 109/109, the component layer is still typed-refused (`cgs_op_hex` does not know the `call` operand, so the 1025 B whole-Component byte-equality is the next slice), independent CI has not run, and gate 3 stays NO-GO; STEP-0509 lands the call-in-expression-position codegen slice locally: the guest emits the whole 1025 B Component of the call-in-a-field fixture byte-equal to the backend's production output (new pin `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly`, validated by the real Wasm validator), because the guest now emits the `call` operand (the argument slots, the call itself and the result slots, with the target resolved to the module function list's 0-based position of the callee's declaration id); the slice also measured that the guest's existing slot, local-declaration and construct-field derivations already agree with the backend on the new IR, so the operand emission was the only missing piece; the three new or reshaped codegen functions lower byte-equal through the guest frontend (new pin `sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly`), the firmware suite is 111/111, the component layer still covers one fixture only (a call in a return or statement position is still refused by the frontend) -- **corrected by STEP-0510's measurement (2026-10-22)**: that refusal claim was too narrow, a user call used directly as a return expression has always lowered byte-exactly (18 measured shapes; `selfhost/codegen.sico`'s own `cgs_alloc_index` is that shape and is still green in the STEP-0507 pin); the shape that is still open is a host call (`sico.*`) in an expression position of a straight-line function body; STEP-0510 lands the call-position line's closing slice locally: a user call used directly as a **return expression** lowers byte-exactly (measured, not asserted -- IR 2742 B == 2742 B and the whole component 1075 B == 1075 B against the backend's production output, validated by the real Wasm validator; the control variant 2613 B / 1067 B is byte-equal too), so the guest source needed **no** change -- the slice pins the shape and **corrects a record that contradicted the evidence**: the claim that a call in a return position is refused by the frontend (it entered the records at STEP-0507 §7.2 and was copied into five synced documents) is too narrow, because 18 measured shapes lower and `selfhost/codegen.sico`'s own `cgs_alloc_index` is that shape and is still green in the STEP-0507 pin; the shape that is still open is a **host call (`sico.*`) in an expression position of a straight-line function body**, which is path-dependent (refused in a straight-line body, lowered as soon as the body carries a `while` or an `if`) and is the next slice; two new pins plus three new entries in the frontend frontier probe (18 -> 21), the firmware suite is 113/113, the guest sources are untouched, independent CI has not run, and gate 3 stays NO-GO; STEP-0511 takes the truth for the next gap instead of guessing it: a **host call (`sico.*`) in a straight-line function body** -- the backend really emits a Component for it (1025 B, accepted by the real validator) and the canonical IR shape is one `intrinsic` operation (`{"name":"sico.bytes.length","arguments":[0]}`, result kind `u64`), while the guest frontend refuses the shape today with a typed `ERR:E-SH-IR-EXPRESSION`; the slice pins that truth in the firmware, registers the shape in the frontend frontier probe (21 -> 22), and **locates the patch point** -- the missing piece is not a missing machine (the guest already has `intrinsic_instruction_json` and the single-argument `gw_intrinsic_call_packed`) but a missing wiring in `scalar_ir`'s return-expression chain, plus `cgs_op_hex` not knowing the `intrinsic` operand; three measured variants (one-argument call, three-argument call, let binding) all emit on the backend and all refuse in the guest, so the gap is a whole class; the guest sources are untouched, the firmware suite is 114/114, independent CI has not run, and gate 3 stays NO-GO; STEP-0512 lands the frontend knife for that gap: a **host call (`sico.*`) in a straight-line function body** now lowers to the canonical `intrinsic` operation **byte for byte equal to the Rust frontend's canonical IR** (2215 B == 2215 B), through a dedicated top-level guest route (`direct_host_call_return_ir`, +121 lines) wired into `scalar_ir`'s route ladder (+24 lines, pure insertion, no existing line reordered) that reuses the machines the guest already has (`parameters_ir` / `binding_parameter_index` / `binding_parameter_kind` / `return_expression_start` / `return_expression_end` / `intrinsic_instruction_json` / `gw_find_code_text`) -- so STEP-0511's "the missing piece is a missing wiring, not a missing machine" reading holds; the supported surface is exactly one shape (one `Bytes` parameter, declared `U64` return, the return statement is the body's only statement) and every other shape keeps its previous typed refusal (the three-argument `sico.bytes.at` and the let-binding position still refuse with `ERR:E-SH-IR-EXPRESSION`); the new route's 21-function closure lowers byte-exactly through the guest itself, so the self-hosting shape frontier did not move; the STEP-0511 oracle pin flipped its guest half to the byte-exact comparison it promised, the frontend frontier probe's `host-call-in-straight-line-body` expectation flipped explicitly from `refused` to `lowered`, and the firmware suite is 117/117; the component layer still refuses (the guest's `cgs_op_hex` does not know the `intrinsic` operand), so the 1025 B whole-component equality is the next slice (STEP-0513), independent CI has not run, and gate 3 stays NO-GO; STEP-0513 lands that codegen knife: the `intrinsic` operand reaches the emitter and the guest's **whole component for the host-call fixture equals the backend's production output byte for byte** (1025 B, accepted by the real validator). The truth is read from the oracle's core-module code section -- the 36 B `size` body is locals `03 017e 017f 027f`, the dispatcher shell, **`2001 ad 2102`** (`local.get 1` / `i64.extend_i32_u` / `local.set 2`) and the direct return -- so the **length slot is the second slot of the argument's (pointer, length) pair** and the **result slot is the instruction's own flat slot**; the slot derivation, the local declaration section, the dispatcher shell and the direct return were all already byte-proven, so this slice's whole change is "how the `intrinsic` operand becomes Wasm bytes" (a new `cgs_intrinsic_hex` plus one dispatch branch in `cgs_op_hex`, pure insertion). Every gate fails closed: only `sico.bytes.length` over exactly one `bytes` argument with a one-slot result is claimed, everything else is `ERR:E-SH-CG-IR-EMIT`. The new codegen function lowers byte-exactly through the guest itself, so the self-hosting shape frontier did not move; the IR plan layer already knew the `intrinsic` operand, so both frontier probes are untouched (22/22 and 8/8); the firmware suite is 119/119; STEP-0515 adds the second position of the host-call line — a `sico.bytes.length` in a `let` binding of a straight-line body lowers to the canonical `intrinsic` IR byte-equal to the Rust frontend (2215 B) and the whole Component byte-equal to the Rust backend (1025 B), with the neighbouring `let` shapes still typed-refused `ERR:E-SH-IR-EXPRESSION`; the firmware suite is 122/122; independent CI has not run, and gate 3 stays NO-GO |
| S6 bootstrap closure | not entered | — | ADR-0015 contract: `A == B == C`, `.sapp` via the M7 trust chain, runner-executed self-compile, budget re-measurement |
| S7 exit audit | reviewed / NO-GO | STEP-0310 reviewed all seven exit gates against current executed evidence | repeat full audit only after S3/S4/S5/S6 and final regression close |

Current delta: RFC-0047 record types have since landed through STEP-0275;
STEP-0276 absorbed the Map-parameter WIP and repaired `records` table
emission. STEP-0278 retired the three legacy S3 units named in the historical
S3 row. STEP-0280 replaced the L1 E7002 keyword fingerprint and added a
SHA-frozen five-case W1 delta corpus covering renamed parameters and
zero-indent function bodies. Its real-runner test passed along with the
original 215-case partition locally. STEP-0221/0245/0261/0262 canaries
passed locally after their frozen-records/frontier expectations were updated.
That was the STEP-0280 local snapshot; STEP-0292 subsequently closed W1 on an
independent CI-tested SHA. The current M22 verdict is the
[2026-09-28 exit review](../reports/m22-exit-audit-2026-09-28.md); the
[M14–M26 audit](../reports/m14-m26-milestone-audit-2026-09-24.md) remains
historical planning evidence.

### 3.2 Execution queue (planned ordering; no STEP numbers reserved)

The queue is the planned convergence order from the current typed frontier,
and since STEP-0264 it runs **under the R-phase convergence gate** (see the
plan status header and [M22–M26 route replan v1](./M22-M26-route-replan-v1.md)
§3): R0 architecture decision, R1 canary/budget instrumentation, then the
items below as R2 convergence content, with the R3 stop-loss counter
evaluated after each eligible W2 S3/S4 implementation STEP. Each item lands
under the standing discipline:
observe the typed refusal,
lower to byte-exactness, re-pin the canary at the next typed refusal, keep
the refusal-recovery and local-bounds regressions green, and never emit a
partial lowering that is unverifiable or semantically different. Each
implementation STEP records before/after canary coverage and the refusal
frontier. A stalled implementation STEP **does count** toward the four-STEP
stop-loss window; documentation/measurement-only STEPs do not. Pending CI
leaves a STEP unadjudicated until its runner evidence arrives. The W2 canary begins with byte-exact
`formatter.sico` functions / 30; after 30/30 it uses byte-exact differential
progress across the remaining frozen selfhost source set. S5 is judged by
its independent byte-equal codegen corpus. A four-STEP W2 stall declares
Route B under [the replan](./M22-M26-route-replan-v1.md) §3.

1. **W1 exit and W2 baseline** — STEP-0292 obtained independent CI success
   for STEP-0278/0280 on the isolated `e8495f9` branch snapshot; W1 is GO
   at that revision. The records-surface W2 baseline itself is frozen
   (STEP-0282, execution card 22-B): [`m22-w2-baseline-2026-09-25.json`](../reports/m22-w2-baseline-2026-09-25.json)
   pins the executed 22/30 formatter prefix, the `nearest_match` /
   `ERR:E-SH-IR-GWPACK-OTHER` starting frontier, the four-size fuel ladder and the
   8-source / 16,390-line tracked selfhost tree by SHA256; stack
   high-water is recorded as unmeasured. STEP-0293 is the first W2 S3/S4
   implementation STEP: its local runner result moves the frontier to
   `ERR:E-SH-IR-CALL-TARGET` with 22/30 functions, and independent CI
   succeeded on repaired commit `cb371c5`. This starts the R3 counter with zero consecutive
   stagnant steps. STEP-0294 is the second implementation STEP: the bounded
   `sico.map.get[Text,U64]` match subject passes local byte-exact differential
   and refusal regressions, moving the frontier to `ERR:E-SH-IR-STATEMENT`
   at 22/30; independent CI succeeded on `5a6d0bb`, giving S2 GO with the
   consecutive-stall count still zero. STEP-0295 locally lowers nested `if`
   with a conditional `return` in the match `ok` arm; the formatter prefix
   through `nearest_match` is byte-exact against Rust, moving the canary to
   23/30 and the typed frontier to `set_nearest_match` / `STATEMENT`.
   STEP-0296 locally routes a while-less if/match shape through the general
   machine and lowers a nested `sico.text.length(Text)` comparison operand
   byte-exactly. The canary remains 23/30; the `set_nearest_match` refusal
   moves from `STATEMENT` to `CALL-SHAPE` at the `bytes.slice` match subject.
   STEP-0297 locally lowers the nested `bytes.slice` match subject byte-exactly;
   STEP-0298 locally lowers the `sico.map.put[Text,U64]` return and both
   returning match arms, advancing the canary to 24/30. STEP-0299 locally
   lowers the nested `u64.to_text` key in `map.get`, advancing compilation
   coverage to 27/30 with a separately byte-exact `direct_close` prefix.
   STEP-0300 locally preserves the parent if region across a nested while,
   advancing coverage to 28/30 with a byte-exact `opener_close` prefix;
   STEP-0301 locally lowers `map.empty[Text,U64]()` RHS; the canary remains
   28/30 and `normalize_source` moves to `GWSKIP`. STEP-0302 locally admits
   while loops in else and post-if join regions with two real-runner
   byte-exact IR differentials; the canary remains 28/30 and the next
   typed frontier is `CALL-ARGUMENT`. STEP-0303 locally reads a
   `Map[Text,U64]` local before its `map.get` key with byte-exact SSA
   order, moving the next frontier to `EXPRESSION` while the canary
   remains 28/30. STEP-0304 locally lowers a user call as the left operand
   of a fixed comparison and repairs SSA numbering after returning a
   parameter, moving the next refusal into nested `text.concat` arguments.
   STEP-0305 locally lowers two Text-returning user calls as `text.concat`
   arguments and moves the refusal deeper into the nested trim argument.
   STEP-0306 locally lowers that nested trim/user-call argument and moves
   the refusal to the output `list.append` RHS / GWPACK-OTHER.
   STEP-0307 locally lowers the Text `list.append` RHS and moves the
   refusal to `map.put` with a fixed-literal value / CALL-SHAPE.
   STEP-0308 locally lowers that fixed-literal `map.put` value and proves
   the `normalize_source` prefix byte-exact when only its final join
   return is replaced by a literal; the real source next refuses at
   `text.join` inside the final `text.concat` / CALL-ARGUMENT.
   STEP-0309 locally lowers that `text.join` expression; the actual
   formatter prefix through `normalize_source` is byte-exact and the
   canary advances to 29/30, with `main` / SCICOND next.
   R3 consecutive-stall count remains zero. Independent CI for
   S3 succeeded on `f7288b7` (run 36166445685), and S4 succeeded on `904b86a`
   (run 36168754671), giving each a GO on the isolated branch; STEP-0297
   independent CI succeeded on `ffcdd3f` (run 36209299222), and STEP-0298
   succeeded on `5ba175f` (run 36210200271). STEP-0299 independent CI run
   36212164872 succeeded on exact SHA `f856bb1`, and STEP-0300 run
   36212509946 succeeded on exact SHA `f2fd7fc`; run 36366432845 succeeded on `8bee75b`, which contains STEP-0301–0305. Independent CI for STEP-0306–0309 and the STEP-0310 audit snapshot succeeded on exact SHA `9c94e7d` (run 36406396074, adjudicated 2026-09-29), so those STEPs carry formal progress credit. On 2026-09-29 the owner designated this branch the authoritative M22 working line, to be merged into `dev` after M22 completes; the parallel `origin/dev` history is not otherwise adjudicated by this branch's CI.
   STEP-0317 (2026-09-30, validation economics under the same owner directive
   「规划好这些优化，完成M22」) landed the differential-loop cost reductions
   recorded in §5. STEP-0318 (2026-09-30) lands the first lex_words-region
   family: `checked_arithmetic_set_match_ir`, a whole-function shape for
   `let cell = T.literal(k)` + `T.checked_add` match whose ok arm is
   `set cell = bind` — six byte-equal variants, eleven typed refusals with
   recovery, suite 53/53. No parser.sico function flips green (5/115 and the
   `lex_words`/CALL-TARGET frontier unchanged), so under the §3.2 discipline
   this implementation STEP counts as R3 stall 1/4; the next slices should
   move the frontier (lex_words let-RHS host-call family first).
   STEP-0319 (2026-09-30) extends the canary metric to per-function
   differential coverage over the frozen sources (rearranged subsets, exact
   function text): `next_index`, `word_at` and `valid_escape_byte` flip
   green through existing families, parser coverage 5/115 → 8/115, canary
   +3 and the R3 consecutive-stall count resets to zero. The contiguous
   prefix frontier stays at `lex_words`/CALL-TARGET; measured next-slice
   inputs: `is_digit_token`/`is_string_token` refuse EXPRESSION (missing
   text.encode/text.length/bytes.at let-RHS shapes) and
   `line_end`/`matching_close` refuse CALL-TARGET (missing user-call RHS).
   STEP-0320 (2026-09-30) flips `line_end` itself (while + if + user-call
   consumer, callees reordered first): parser coverage 8/115 → 9/115, R3
   stays zero. Probe census of its downstream pins the next family:
   user-call RHS/argument-position shapes (six CALL-TARGET refusals plus
   return_shapes' CALL-ARGUMENT) and the text.encode/bytes.at let-RHS
   host-call family (json_string_token's EXPRESSION).
2. **Formatter tail from the current frontier** — `main`.
   The `Map[Text,U64]` region through `match_arm_levels`, `close_code`,
   `direct_close` and `opener_close` passed the real-runner
   byte differential. `normalize_source` passed at STEP-0309, and `item` through `repeat_indent` also passed; do not
   re-implement those regions or treat their historical refusals as current.
   Each remaining region lands as its own byte-exact prefix differential. Exit test:
   the complete `formatter.sico` lowers byte-exactly against the Rust
   oracle with the full suite green.
3. **Remaining selfhost sources** — inventory the current tracked
   `selfhost/*.sico` set at the start of W2; lower each remaining source's
   frozen corpus byte-exactly. `parser.sico` is a large bounded unit;
   split its declared shapes by corpus and refusal frontier before editing.
   STEP-0278 retired `lexer.sico`, `tokens.sico` and
   `declaration_parser.sico`; they are historical evidence, not queue
   items. Exit test: every current selfhost source's frozen differential
   is byte-exact and the `E-SH-IR-*` refusal set shrinks only by declared
   regions.
4. **S5 widening** — extend the Core-Wasm seam from nonnegative constants
   to the RFC-0011 deterministic subset. Exit test: emitted bytes equal
   the Rust backend on the frozen corpus; out-of-subset shapes stay
   typed-refused, never silently fallen back.
5. **S6 bootstrap closure** — per ADR-0015: `A == B == C` on the frozen
   corpus, `.sapp` packaging through the M7 trust chain, runner-executed
   self-compile, and the fuel/timeout/wall-time budget record (measured
   with the STEP-0254 reused harness for iteration cost; the record itself
   remains the only performance claim surface).
6. **S7 exit audit** — evidence pack, budget tables, dual-implementation
   register, explicit GO/NO-GO.

### 3.3 S5/S6 闭合阶梯（owner 规划片，2026-10-22；STEP-0514）

> - 本节是 M22 剩余工作的**执行阶梯**，由 owner 于 2026-10-22 指示"只负责规划、尽量细化、执行另开会话"而产生，随后 owner 追加"全权拍板"授权 ⇒ **本节与 §3.3.2 的决策登记（D4–D15）均为已生效的规划决定，执行会话直接遵守，不再回头请示**。它**不预留任何 STEP 号**（沿用 §3.2 的取号规则：开工时按 `docs/steps/` 当前最大号顺取，取号前核对 `docs/steps/README.md` 与 [`handoff-m22.md`](../handoff-m22.md)；M24 占 STEP-0330–0337、M23 已占 STEP-0489/0490，平行会话占用段不得侵占），只给出 slice 标识、依赖、入口门槛、出口判据与禁止事项。
> - 证据等级：全部 `internal-fixture`；运行时主张只来自真实 runner（本机 Windows x64 GNU + Wasmtime）。`contract-verified` 与 runtime 支持继续分离。
> - 本节**不改** §4 出口门、**不改** ADR-0015 合同、**不产生**任何新的能力主张。**唯一触及的合同是 [ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)**（组件级 canary 次序按"先广度后尺度"重排，owner 2026-10-22 授权）——它只改**次序**，不改门、不改 `A == B == C`、不改证据等级。测量片不得被读成任何 canary 梯级的完成。

#### 3.3.1 剩余工作的量化基线（2026-10-22 实测）

| 项 | 实测值 | 来源 |
|---|---|---|
| ADR-0015 的 `A = R(S)` 组件总量 | **1,024,613 B** | `target/tmp/step0506/compiler.component.wasm`（Rust 后端**生产输出**） |
| ├ core module | 1,024,161 B | component section 1 |
| ├ **code 段** | **1,003,250 B** | core section 10 |
| ├ **data 段**（字面量池） | **16,487 B** | core section 11 |
| ├ type 段（函数签名） | 3,685 B | core section 1 |
| ├ function / memory / global / export | 711 B | core sections 3/5/6/7 |
| 组件级 type/import/alias/canon/export | 419 B | component sections 7/10/6/8/11 |
| 冻结源集 S | 9 文件 / **29,088 行 / 1,299,939 B** | `selfhost/*.sico` |
| S 内被调用的宿主内建 | **20 个族**（`text.concat` 2,882 次最多） | `selfhost/*.sico` 全文统计 |
| guest 已闭合的内建 | **1 个**（`sico.bytes.length`，单实参 `Bytes`，**2 个位置**：return 位置 + **let 绑定位置**） | STEP-0512 §5.1 / STEP-0515 §5.1 |
| guest 已发射过的最大组件 | **1,025 B** | STEP-0513 新钉 `sico_codegen_guest_emits_the_host_call_component_byte_exactly` |
| 冻结语料 | **37 accepted / 178 refused**（共 215 条） | `selfhost/script-build-corpus-v0.json` |
| 178 条拒绝的诊断分布 | LEXICAL **116** / SCRIPT_ABI **28** / 散码 **34**（约 20 个不同 E 码） | 同上 |
| 当前门状态 | 门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO | [`m22-exit-audit-2026-10-20`](../reports/m22-exit-audit-2026-10-20.md) |

**落差结论**：固定点要求 guest 发射 **~1,024,613 B** 的组件，而它发射过的最大组件是 **1,025 B** ⇒ **约 1000 倍**。有利证据：前端已在 IR 层逐字节降完整份 `parser.sico`（census 149/149 + full-source exit 0），Rust 后端也已能产出并在 runner 里运行那 1 MB 组件 ⇒ **唯一未被验证的是 guest 侧组件发射在该量级上的行为**（含吞吐、峰值内存与燃料）。

#### 3.3.2 决策登记（owner 授权 2026-10-22 + 本轮拍板）

**A. owner 授权的三项（2026-10-22 会话，owner 后续追加"全权拍板"授权）**

1. **允许插入一个纯测量片并前置**：在组件级语料梯级之前插入 S5-W1（规模可行性测量，只测量、不实现、不推进任何门）。理由：把可能推翻里程碑的唯一未知量前置，避免在最末梯级才撞穿。
2. **授权在合格跑之前重新基线化限额**：ADR-0015 决策 8 允许限额在合格跑前固定；任何上调必须新开一次 attempt 并完整记录，不得事后追认。
3. **`origin/dev` 冲突历史的裁决时点**：**推迟到 S5-W7（门 6 收尾）统一裁决**。**触发条件**：S5-W7.1 的提交快照就绪。

**B. 本轮拍板（D4–D15；owner 已授权全权决策，以下为已生效的规划决定，执行会话直接遵守）**

| 编号 | 决定 | 理由 / 约束 |
|---|---|---|
| **D4** | **组件级 canary 次序冻结为** `formatter.sico 组件级 → 37 条 accepted 语料组件级差分 → 完整 bundle 组件级 → ADR-0015 harness` = **先广度后尺度**；登记为 **[ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)**，STEP-0486 §2.4 已加对齐注记（原句保留） | `formatter.sico` 的 IR 面已字节全等 ⇒ 其组件级失配可单独归因于 codegen；bundle 同时含尺度与全表面，是最贵梯级故最后。**阻断纪律不变**（早梯级 refusal/trap 阻断后梯级完成声明） |
| **D5** | **W1 探针组件不进 `selfhost/`**，放 runner 测试侧的独立 fixture（建议 `runner/sico-runner/tests/fixtures/`），由 Rust 后端单独编译 | 放进 `selfhost/` 会改变冻结源集 S ⇒ 扰动 `A = R(S)` 的基线字节；探针测的是 runtime/限额行为，本就在问题域外。**探针组件不得进入 A/B/C 链** |
| **D6** | **W1 判读阈值**（见 §3.3.4 表） | 无阈值则判读靠感觉；量化后"可行/临界/不可行"可复现 |
| **D7** | **不可行分支的方案优先级**：②组件输出改**二进制通道**（不经 hex） > ①guest codegen 改**流式/分块发射** > ④提高限额 | 先降表示成本，再改算法，最后才加限额（限额是最后手段且需 host 内存支撑） |
| **D8** | **限额重基线候选值**：guest arena **4 GiB** / host cap **5120 MiB** / fuel **1.6T** / timeout **180 s** | 相对现值（2 GiB / 2560 MiB / 800B / 120 s）统一取 2×；**上限不得无证据上调**，每次上调 = 新 attempt 并记录 |
| **D9** | **W2 期间禁止并行开族**：`selfhost/parser.sico` 与 `selfhost/codegen.sico` 是共享热点，同一时间只允许一个族改动这两个文件；W3 起才允许按族并行（且改动面必须分离） | 并行会引入字节冲突与自举形状前沿互扰（STEP-0507/0508/0512 记录的"宿主调用吃宿主调用"与闭包前沿问题均与共享文件改动耦合） |
| **D10** | **族降级规则（T-R3-b 具体化）**：某族 3 片未闭合时可降级为"最小子集形"，但**降级后若 S 内该族仍有未覆盖的调用点形状 ⇒ 不得降级，必须闭合**；降级必须登记进"支持面登记表" | 固定点要求 guest 能编译 S 自身 ⇒ 降级不得制造新的 S 缺口 |
| **D11** | **语料一致性工具面**：新增 `tools/probe-m22-corpus-parity.ps1`（**只读**冻结语料），冻结期望落 `selfhost/corpus-parity-v0.json`（随语料冻结，不写死在探针里）；三元组 = `(verdict, diagnostic id, span)` | 探针不得改冻结语料；期望值必须可版本化 |
| **D12** | **harness 的 S 喂入**：单块 **≤ 64 KiB**、总量 **≤ 2 MiB**、超限 **fail-closed**（含 limit+1 语料）；`A` 的产出固化为独立脚本 + SHA 记录 | ADR-0015 决策 6 要求"源进有界输入、产物出有界输出"；S ≈ 1.30 MB ⇒ 必须显式定界 |
| **D13** | **门 4 耗量口径**：若 Runtime ABI **没有** fuel 实际耗量读数出口 ⇒ 登记为**上界**（限额取值）而非读数，并显式标注"上界，非读数"；**不阻塞 GO** | 诚实降级优于伪造读数；ADR-0015 决策 8 的测量项可部分满足并明确缺口 |
| **D14** | **`origin/dev` 裁决方式** = **不 rebase、不 merge**；M22 的完成判据**不包含**与 `origin/dev` 的合并；冲突历史登记为 **M25 集成议题**；M22 交付 = `codex/m22-w2` + `codex/m22-w1-ci` 的完整提交 + 独立 CI 证据 | 历史重写风险高、收益低；仓库既有纪律本就是"不得覆盖、不得隐式合并" |
| **D15** | **新交付物：支持面登记表**（门 7 之前必须产出，建议 `docs/reports/m22-support-surface-*.md`）= 族 × (名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / **未覆盖形状**) | M22 GO 时必须能说清"支持哪些形状、不支持哪些"，否则"声明子集"限制无法向使用者交付 |

#### 3.3.3 阶梯总览

| slice | 名称 | 依赖 | 出口判据（摘要） | 门映射 |
|---|---|---|---|---|
| S5-W1 | 1 MB 级发射的规模可行性测量（纯测量） | — | 给出 1 MiB 输出的 (wall, fuel, 峰值内存) 与增长曲线；判读为可行/临界/不可行三档之一 | 不推进任何门 |
| S5-W2 | 20 个宿主内建族的逐字节编解码 | W1 判读为可行或临界 | 每族：canonical IR 逐字节等同 **且** 整组件逐字节等同 **且** 过真实 `wasmparser::Validator` | 门 2 主体 |
| S5-W3 | **组件级广度 rung**：`formatter.sico` 组件级 → 37 条 accepted 语料组件级差分 | W2 覆盖对应族 | `formatter.sico` 组件逐字节等同；每条语料 == Rust 生产输出 == 冻结 `component_sha256` | 门 2 |
| S5-W4 | 178 条拒绝的一致性 | W2/W3 稳定 | (verdict, diagnostic id, span) 三元组逐条一致 | 门 2 |
| S5-W5 | **组件级尺度 rung**：完整 self-host bundle（5 文件）+ 9 文件全集组件级全等 | W2/W3/W4 | bundle 组件 ≈ 1,024,613 B 逐字节等同 | 门 2 收尾 |
| S5-W6 | ADR-0015 自举闭环（`A == B == C` + `.sapp` + 预算 + 证据清单） | W5 | 三阶段字节全等 + M7 包链 + hostile 语料 fail-closed + 预算实测 | 门 3 |
| S5-W7 | 门 6/7 收尾（快照 + 独立 CI + 全量回归 + 复审 + 冲突裁决 + 门 4 补强） | W6 | 全量 M0–M21 绿 + 独立 CI 绿 + 新出口审计逐门裁决 | 门 6/7 |

**canary 梯级映射（[ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)，先广度后尺度）**：

| canary 梯级 | 本阶梯位置 | 当前状态 |
|---|---|---|
| 六个既有标量 IR 形状 | W2 之前已闭合（STEP-0222–0240 线） | 已闭合 |
| `formatter.sico` 组件级 | **W3-a** | 未开始（IR 面 30/30 CLOSED，组件面未做） |
| 37 条 accepted Script 语料组件级 | **W3-b** | 未开始 |
| 完整 self-host source bundle 组件级 | **W5** | 未开始（≈1.02 MB） |
| ADR-0015 闭合 harness | **W6** | 未开始 |

**关键路径**：`W1 → W2(T1/T2/T3…) → W5 → W6 → W7`。W3 与 W4 在 W2 到位后可与 W2 后段并行（但同树并行会话改动面必须互不重叠，见 §3.3.12）。

#### 3.3.4 S5-W1：1 MB 级发射的规模可行性测量片（纯测量）

**入口门槛**：owner 决策 1 已给出（2026-10-22）；本片**不推进任何门**、**不闭合任何 canary 梯级**、**不放宽任何 typed 契约**。

**测量项**（全部在真实 runner 内、以 guest 侧执行；Rust 侧只作对照）：

> **探针落点（D5）**：测量用的 guest 程序**不进 `selfhost/`**——放进 `selfhost/` 会改变冻结源集 S，扰动 `A = R(S)` 的基线字节。探针放在 runner 测试侧的独立 fixture（建议 `runner/sico-runner/tests/fixtures/`），由 Rust 后端单独编译成**独立组件**；该组件**不得进入 A/B/C 链**，也不得进入自举形状前沿。测量必须使用**与合格跑相同的限额**（arena 2 GiB / fuel 800B / timeout 120 s）。

| 编号 | 测量 | 阶梯 / 参数 | 记录量 |
|---|---|---|---|
| M1 | guest 字节拼接吞吐 | N ∈ {4 KiB, 32 KiB, 256 KiB, 1 MiB, 2 MiB, 4 MiB}；两种策略：**S1** 增量 `sico.text.concat`、**S2** 分块 `sico.arena.push_chunk` + 末段拼装 | wall time / fuel / 峰值 guest 内存 / 输出字节数；**增长曲线形态**（线性 vs 二次） |
| M2 | 通用 bytes→hex 编码吞吐 | 同上阶梯 | 每 MiB 的编码耗时与峰值内存 |
| M3 | 大对象同时驻留 | canonical IR JSON 文本（1 MB 级样本）+ 2 MB hex 缓冲同时在驻 | 峰值 guest 内存 |
| M4 | Rust 侧对照 | Rust 后端构建 `A` 组件（1,024,613 B） | host 侧 wall time（**不作 runtime 支持主张**） |
| M5 | 目标结构折算 | 把 A 的分段（code 1,003,250 / data 16,487 / type 3,685 / …）折算成 guest 侧必须物化的中间量 | 中间量清单与上界 |

**判读三档（出口判据；阈值按 D6 固定，执行会话不得自行放宽）**：

| 档 | 判据 | 动作 |
|---|---|---|
| **可行** | 1 MiB 输出在 (arena ≤ 2 GiB, fuel ≤ 800B, wall ≤ 120 s) 内达成，**且** 256 KiB→1 MiB 的 wall time 增长 **≤ 6×**（近线性判据） | 按本阶梯继续；**不得改动限额** |
| **临界** | 需要 arena > 2 GiB **或** fuel > 800B **或** wall > 120 s，但增长曲线仍近线性（**≤ 16×**，分块策略可压到 **≤ 8×**） | 触发 RFC-0029 第四次修订 + 限额重基线（候选值见 D8：arena 4 GiB / host cap 5120 MiB / fuel 1.6T / timeout 180 s），作为**新的 attempt** 记录，失败后不得回退追认 |
| **不可行** | 增长 **> 16×**（二次增长）且**两种策略都压不住** | **立即停止 W2/W3 的顺序推进**，转入 ADR / RFC 修订流，并把结论升级给 owner（**不得继续堆实现片**） |

**不可行分支的方案优先级（D7）**：① 组件输出改**二进制通道**（不经 hex） → ② guest codegen 改**流式/分块发射**（不物化整串） → ③ 降低 guest 侧中间表示成本 → ④ 提高限额（**最后手段**，需 host 内存支撑且必须新开 attempt）。先降表示成本，再改算法，最后才加限额。

**禁止事项**：① 不得为测量放宽任何 typed 契约；② 测量读数不得写成 runtime 支持或性能主张（ADR-0015 决策 8：预算记录是观察值，不是 SLA）；③ 测量路径必须与生产路径**分离**（独立驱动模式 + 独立文件），不得让测量代码进入自举形状前沿或 `--emit-component-hex` 生产路径；④ 本片结论**不得**被读成任何梯级完成。

**方法学先例**：STEP-0501 / STEP-0505 / STEP-0511（只测量不实现，读 oracle **生产输出**，oracle 生产代码零改动）。

**W1 判读结果（STEP-0516，2026-10-23 实测）**：**判读 = 可行**。1 MiB 字节组装在合格限额内达成（线性策略 66–78 ms 多轮观测 / fuel 上界 24.1 M / 内存下限 = 声明值 2,214,592,512 B），256 KiB→1 MiB 增长 **1.8×–2.3× ≤ 6×**（墙钟机器相关 ⇒ 按区间记录）⇒ **T-R3-a 止损不触发**，W2/W3 顺序推进继续。**但判读附带一条实施约束**：可达性**强烈依赖组装策略**——增量拼接（O(n²)）在 **128 KiB** 即耗尽 arena，分块 `push_chunk`（chunk 表每次重建 ⇒ O(n²)）在 **256 KiB** 耗尽，而一次成型的线性组装在 **4 MiB** 仍余量充足 ⇒ **W2 的 codegen 刀必须按线性组装设计，不得沿用现有增量拼接形状**（D7 的「只分块」路线实测不足；且 guest 现有 `cg_leb_hex`/`cg_hex_byte`/`cg_section_hex` **就是**增量拼接形状 ⇒ 这是对现有代码的直接诊断）。未测项：M2（runtime 无 hex 内建）、M3、M4 留后续片。

#### 3.3.5 S5-W2：20 个宿主内建族的逐字节编解码

**排序依据**：按 **S 内调用点降序**（＝固定点优先级：guest 必须能编译 S 自身），不是按实现难度。

| 序 | 内建 | S 内调用点 | 备注 |
|---|---|---|---|
| T1 | `sico.text.concat` | 2,882 | **最高杠杆**：guest 自己拼字节靠它；与 M1 二次复杂度假设直接相关 |
| T2 | `sico.text.length` | **318 → 实测 322**（STEP-0528 订正：S 在计划成文后的增长） | 与已闭合的 `bytes.length` 同族，扩参型即可复用机器；**STEP-0528 拍 1 真值已测**：条件位已能逐字节降、语句位不能；**STEP-0529 前端刀（F）已落地**：语句位（`let` / `set` / 单 `return`）**已能逐字节降**（5 个形状）；**STEP-0530 codegen 刀（C）已落地**：**组件层已逐字节发射**（**10 个形状**，helper 索引 = `模块函数数 + 3 + 名字排序位`，helper body = locals 组 + 51 B 代码）⇒ **T2-c1 闭合**；**嵌套调用实参（T2-g2）仍开**（前端层）⇒ **族未闭合** |
| T3 | `sico.text.starts_with` | 316 | |
| T4 | `sico.list.length` | 249 | 容器族入口 |
| T5 | `sico.bytes.length` | 246 | **已闭合 2 个形状**（return 位置 = STEP-0512；**let 绑定位置** = STEP-0515；均单实参 `Bytes`）；本族扩到多实参/其它实参类型 |
| T6 | `sico.text.encode` | 215 | |
| T7 | `sico.list.append` | 179 | |
| T8 | `sico.text.split_lines` | 164 | |
| T9 | `sico.bytes.at` | 86 | **三实参**（STEP-0511 已测得组件真值 1,053 B） |
| T10 | `sico.list.empty` | 42 | |
| T11 | `sico.bytes.utf` | 25 | |
| T12 | `sico.bytes.slice` | 24 | |
| T13 | `sico.bytes.equal` | 22 | |
| T14 | `sico.arena.push_chunk` | 17 | **运行时族**：与 M1 的分块策略直接相关 |
| T15 | `sico.list.get` | 14 | |
| T16 | `sico.text.trim` | 10 | |
| T17 | `sico.text.ends_with` | 8 | |
| T18 | `sico.text.join` | 7 | |
| T19 | `sico.map.put` | 4 | |
| T20 | `sico.map.get` | 4 | |

**每族四拍（固定模板，不得跳拍）**：

1. **真值测量片（T）**：从 oracle **生产输出**取真值（canonical IR + 整组件字节 + core module code 段反汇编），产出规则表、子集边界与下一刀切点；guest 生产代码零改动。
2. **前端刀（F）**：`selfhost/parser.sico` 认该形状 ⇒ canonical IR 逐字节等同；**纯插入、零既有行重排**；新增函数必须自证"经 guest 自己逐字节降低 == Rust 前端"（自举形状前沿钉）。
3. **codegen 刀（C）**：`selfhost/codegen.sico` 认该算子 ⇒ 整组件逐字节等同 + 过真实 `wasmparser::Validator`；闸门**全 fail-closed**（名字 / 实参个数 / 实参类型 / 结果槽数四道），不静默取近似。
4. **收口片（X）**：钉住 + 纠偏 + 前沿探针期望值随之前移 + 固件电池 + 独立 CI（提交后）+ 诚实收窄段。

**每族必须显式登记的子集边界**：支持的名字集合、实参个数、实参类型、结果槽数；边界外一律 typed 拒绝（`ERR:E-SH-IR-*` / `ERR:E-SH-CG-IR-EMIT`）。

**族级出口判据**：`canonical IR` 逐字节等同 **且** `整组件` 逐字节等同 **且** `wasmparser::Validator` 通过 **且** 前沿探针期望值已翻转 **且** 固件电池绿 **且** 独立 CI 绿。**六项缺一不算闭合**（**原写「五项」是计数笔误** —— 上面列的是**六条**判据；STEP-0527 收口时订正，判据**内容**未变）。

**T1 拍 1 结论（STEP-0517，2026-10-23 实测；纯测量，不推进任何出口门）**：**T1 = `sico.text.concat`**（S 内 2,882 个调用点，最高杠杆）。**规则表（10 个变体）**：let 绑定位置（形参 ×2 / cell ×2）⇒ guest **逐字节** == Rust canonical IR（**2295 B**）；**return 位置 + 体内另有 `let`** ⇒ **2295 B 逐字节**；**return 位置 + `let`+`set`** ⇒ **2630 B 逐字节**；**return 位置 + 体内只有一条 `return`**（形参 ×2 / 字面量 ×2 / 形参+字面量 / 嵌套 concat）⇒ **typed 拒绝 `ERR:E-SH-IR-EXPRESSION`**；oracle **生产组件** = **1094 B**（let 绑定 / return+let）/ **1114 B**（return+let+set）/ 1124 / 1133 / 1113 B。**关键发现**：**guest 早就实现了 T1 的 return 位置下降**——`selfhost/parser.sico:9974` 的 `straight_cell_function_ir` **本来就有** return 位置的 concat 分支，只要函数体**除 return 外还有一条语句**，同一个 `return sico.text.concat(a, b)` 就**逐字节等同**；**缺口是可达性不是能力**——路由梯（`:20579`–`:20583`）把整个 `straight_cell_*` 家族门控在 `body_statement_count = max(let_count, set_count) > 0` 上 ⇒ **体内只有一条 return 时该分支根本不被调用**。⇒ **T1 前端刀（F）的工作量被压缩成「开一条可达路径」**：**不新造 concat 机器**（`straight_text_concat_packed` `:8929` / `gw_text_concat_packed` `:5808` 两台**既有**机器已能处理 N 个实参与嵌套调用），只须为「体内只有一条 return、且该 return 是 `sico.text.concat`」开一条**专用顶层路由**（与 `direct_host_call_return_ir` / `direct_host_call_let_ir` 同族，**纯插入、零既有行重排**）。**codegen 刀（C）尚未开工**：`cgs_intrinsic_hex` 只认 `sico.bytes.length` ⇒ `--emit-component-hex` 仍 fail-closed 拒绝；**与 STEP-0513 不同**，`sico.text.concat` 必须**发射一个 helper 函数体**（core func 5：arena 分配 + 两段 `MemoryCopy`）+ 调用点，**线性拷贝**（正落在 STEP-0516 的「线性组装」约束内）。**T1 子集边界（D15 登记）**：名字 = 仅 `sico.text.concat`；实参个数 = 恰好 **2**（**元数边界在语义层**：1 个 / 3 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity` ⇒ guest **不得**自造元数检查）；实参类型 = 两个 `Text`；结果槽数 = **2**（pointer, length）；**语料覆盖 0 条**。**方法学教训（写进本节）**：在 guest 这类「同一函数有多个入口路径」的前端里，**「某形状被拒绝」不等于「该形状没实现」**——**必须用「加一条无关语句」这类最小扰动去探测可达性**，否则会把「可达性缺口」误报成「能力缺口」，把下一刀的工作量估错一个数量级。

**T1 拍 2 结论（STEP-0518，2026-10-23 实测；实现片，**族仍未闭合**）**：`selfhost/parser.sico` 新增**专用顶层路由** `direct_text_concat_return_ir`（**+146 行**）+ `scalar_ir` 路由梯接线（**+24 行**）= **+170 行 / −0 行**、**纯插入、零既有行重排**（自证 `removed_lines=0`）。**支持面 = 恰好一个形状**（**2 个 `Text` 形参**、声明返回 `Text`、函数体**唯一语句**是 `return sico.text.concat(<Text 形参>, <Text 形参>)`），**闸门全 `SKIP`** ⇒ **既有拒绝身份不变**。**出口达成**：guest canonical IR **2295 B == 2295 B**（与 Rust 前端**逐字节相同**；intrinsic range **374..396**、terminator `{"kind":"return","data":2}`）；**自举形状前沿**：新路由闭包 **20** 个函数逐字节等同 ⇒ **未推进形状边界**（**新路由自己的代码里不能出现 `return sico.text.concat(...)`** —— 那是它自己要开启的形状 ⇒ 循环依赖 ⇒ 末尾改用 `let` 累加 + `return` 局部变量）。**未闭合**：STEP-0517 的**四个**未闭合形状里**一个**被认领，其余**三个**（两个字面量实参 / 混合实参对 / 嵌套宿主调用）**仍** typed 拒绝 `ERR:E-SH-IR-EXPRESSION`；**组件层仍 fail-closed 拒绝** ⇒ **T1 族 2/4 拍完成、尚未闭合**（族级出口判据**五项缺一不算闭合**）。**方法学教训（写进本节）**：**自证脚本本身也会说谎**——纯插入审计脚本第一版把 diff 方向写反，报告「删了 170 行」（恰是本片新增的行）⇒ **任何自证结论都必须与「本片声称做了什么」交叉核对**。

**T1 拍 3 结论（STEP-0519，2026-10-23 实测；实现片，**族仍未闭合**）**：`selfhost/codegen.sico` 认 `sico.text.concat` 这个算子 ⇒ **整组件与 Rust 后端生产输出逐字节等同（1094 B）** + oracle 与 guest 两侧**都过**真实 `wasmparser::Validator`。**发射真值从 oracle 生产输出解析而来**（`target/tmp/step0519/core-sections.txt`）：type 段 6 条 ⇒ **type 5 = concat helper** = `60047f7f7f7f027f7f`；function 段 6 条且 **`type_index` = 自身下标**；**export 段不变**；code 段 6 个 body ⇒ helper body **42 B** = `01027f`（1 组 × 2 个 i32 局部）+ `200120036a2205`（总长 `a_len + b_len` → `local.tee 5`）+ `10 <alloc>`（arena 分配）+ `21 04`（结果 pointer）+ 两段 `MemoryCopy` + `200420050b`（返回 `(ptr, len)`）；**helper 的 core function index = 模块函数数 + 3**（后端 `helper_count = import_count + module.functions.len()`，`used_helpers` 是**按名字排序**的集合 ⇒ `sico.text.concat` 恒为 `i = 0`；Script profile 的 `import_count = 0`）。**落点** = **新增 4 个结构化扫描函数**（`cgs_instruction_is_text_concat` / `cgs_block_has_text_concat` / `cgs_function_has_text_concat` / `cgs_module_has_text_concat`）+ **新增 `cg_concat_helper_body_hex`** + **搬家 `cgs_function_count`**（函数体逐字节不变）+ **重塑 `cgs_intrinsic_hex` / `cgs_core_types_hex` / `cgs_core_funcs_hex` / `cgs_core_code_hex`** ⇒ 文件 3744 → **3979 行**（**+235 行**）。**四道闸门全 fail-closed**（名字 / 实参个数 = 恰好 2 / 实参类型 = 两个 `string` / 结果槽数 = 恰好 2）⇒ `ERR:E-SH-CG-IR-EMIT`。**「模块用没用到 concat」= 结构化扫描**（`functions` → `blocks` → `instructions` → `operation.op == "intrinsic"` → `operation.data.name == "sico.text.concat"`），**不是子串搜索**；每级一个函数、**每函数只有一个 `while`**（flag 模式 + early return，**不用嵌套 `continue`**）⇒ 三段条目条件化 ⇒ **不用 concat 的模块保持三条目**（既有 host-call 组件 **1025 B 逐字节不变**）。**出口达成**：整组件 **1094 B == 1094 B**；**helper 索引是推导的**（三函数模块 ⇒ 索引 6，组件仍逐字节等同）⇒ 钉；**自举形状前沿**：本片 10 个 codegen 函数逐字节等同 ⇒ **未推进形状边界**。**未闭合**：**T1 族 3/4 拍完成**——缺**收口片 X**（钉住 + 纠偏 + 电池 + **独立 CI**）⇒ 按族级出口判据**五项缺一不算闭合**；guest codegen **只认一个 helper 型内建**（其它 helper 型内建仍 fail-closed）；STEP-0517 的**三个**前端未闭合形状仍 typed 拒绝；**语料覆盖仍 0 条**。**方法学教训（写进本节）**：①**guest 前端的形状规则是逐路由的**——「直线函数体里 `let` 必须全部排在任何 `set` 之前」（实测 `ERR:E-SH-IR-CALL-TYPE`）**不能**从「`if` 分支内部同样的写法可降」外推 ⇒ **写新 guest 代码前要先按路由试最小变体**，而不是照抄邻近函数的写法；②**反汇编工具本身也要被交叉验证**——准备片的 code body 反汇编**丢了 locals 段**（helper body 一度被读成 39 B 而非 42 B），靠**与 guest 里已冻结的常量比对**才发现 ⇒ **拿工具去和一个已知正确的常量对，是最便宜的自检**；③**「callee 必须先声明」这条规则会逼出函数搬家**——本片为了让 concat 分支拿到模块函数数，把 `cgs_function_count` **原样上移**（函数体逐字节不变）⇒ **codegen 刀不是纯插入**，这是它的真实成本。
 **W2 共享前置结论（STEP-0520，2026-10-23 实测；实现片，**族仍未闭合**）**：**cell 支持是 W2 全部 20 个族的共享前置**，必须先于任何族的 codegen 刀。**为什么（D10 实测）**：S 内 `sico.text.concat` 调用点 **2,857** 个里 **2,843（99.5%）** 落在**带 cell 的函数**里 ⇒ **T1 族不得降级**；且 `let` 绑定一旦被 `set` 赋值，Rust 前端就把它物化成 **cell** ⇒ **任何**族都会撞同一堵墙。**后端真值（生产代码 + oracle 生产输出，非设计）** = `LocalLayout::plan`（`crates/sico-codegen-wasm/src/lib.rs:1812`）槽位顺序 **形参（不声明）→ 各指令结果（每槽一组；unit 结果 0 槽 0 组）→ cells（每 cell 的 flat 类型一组）→ dispatcher（1×i32）→ Script scratch（2×i32）**；`WriteLocal` / `ReadLocal` = 逐槽 `local.get(源)` + `local.set(cell)`；oracle 组件反汇编 ⇒ `join` 的 locals = **5 组 (1,i32) + 1 组 (2,i32)**，槽位 `a`=0,1 `b`=2,3 concat 结果=4,5 **cell x=6,7** dispatcher=**8** scratch=9,10。**落点** = `selfhost/codegen.sico` **新增 8 个函数**（`cgs_is_unit_ty` / `cgs_cell_flat_hex` / `cgs_cell_ty` / `cgs_cell_width` / `cgs_cell_slots` / `cgs_cell_slot` / `cgs_write_local_hex` / `cgs_read_local_hex`）+ **重塑 4 个** ⇒ 文件 3979 → **4277 行**（**+298 行**），函数数 141 → 149。**四道闸门全 fail-closed**：cell 类型 ∈ {bool, i64, u64, string, bytes}（**独立窄函数** `cgs_cell_flat_hex` 与后端 `lower_local_types` **逐类型对齐** —— 通用 `cgs_flat_hex` 在 `Result[I64|U64, NumericError]` 上与后端**差一槽**，故**不可复用**）/ cell 下标存在 / 源宽 == cell 宽 ⇒ `ERR:E-SH-CG-IR-FLAT` / `ERR:E-SH-CG-IR-EMIT`。**出口达成**：参考形状 canonical IR **2630 B == 2630 B**（逐字节）+ 组件 **1114 B == 1114 B**（逐字节）+ oracle 与 guest 两侧**都过**真实 `wasmparser::Validator`；**cell 读**形状（2 写 2 读）组件 **1138 B** 逐字节等同；**I64 cell** 形状组件 **1116 B** 逐字节等同；既有 4 个无 cell 形状仍 **1094 B** 逐字节等同（不退化）；**自举形状前沿**：本片 **12** 个 codegen 函数逐字节等同 ⇒ **未推进形状边界**。**未闭合**：**T1 族仍未闭合**（缺**收口片 X** 与**独立 CI**）——本片**不是**族级收口；**当前可达的 cell 子集是 Text 与 I64**（`Bytes` / `List[Text]` 被**前端**挡在前 `ERR:E-SH-IR-EXPRESSION`；**记录型 cell 的前端 IR 与 Rust 前端不一致** —— 是**前端**差异，本片**没有**认领）；guest codegen 仍只认两个内建；**语料覆盖仍 0 条**；**整文件自举仍未关闭**。**方法学教训（写进本节）**：①**Bool 只能进 `if` 的条件位** —— `U64.equal(<Bool>, 0)` 会被前端拒 `E2001`（`equal` 是定宽数值方法，Sico **没有** `Bool.equal`）；②**同一函数内 `let` 名字必须唯一** —— 前端把一般 CFG 里的每个 `let` 都物化成 **cell**，重名即 `cell redeclared`；③**自证脚本的「还原区间」必须以「改动插入点」为边界**，而不是以「当前文件的锚点」为边界 —— 本片的足迹审计错了三轮（少 7 行 / 多 4 行 / 少 1 行），**靠独立见证**（把重建文件喂给 STEP-0519 的审计脚本，要求给出与 STEP-0519 当时**记录值完全相同**的 3745 / 3980 / +235）才发现 ⇒ **自证脚本本身也会说谎**，任何自证结论都必须与「本片声称做了什么」交叉核对。

**T1 拍 4 前置结论之二（STEP-0522，2026-10-23 实测；实现片，**族仍未闭合**）**：**G-b 刀 = 转义字面量解码 + 空字面量（G-e）⇒ 组件与 Rust 后端生产输出逐字节等同**。**真值（生产输出，非设计）**：Rust 后端（`sico build --profile script-v0`）对 `"tab\there"` + `"nl\nhere"` 的静态 data 段 = **15 B** `74616209686572656e6c0a68657265`（`tab<TAB>here` + `nl<LF>here`）⇒ **后端放的是解码后的字节**，不是源码里的两个字符 `\t`；**改前 guest 对同一 fixture 是 typed 拒绝**（`ERR:E-SH-CG-IR-EMIT`），**改后逐字节相同**（1197 B）。**核心改动 = 字面量的身份换成它的解码后字节**（hex 字节对）：`cgs_literal_text` → `cgs_literal_hex`、`cgs_data_offset` 改为按**字节**去重与累计、`cgs_data_hex` 直接用 hex ⇒ 与后端 `script_data`（`value.as_bytes()` 做 key）**逐字节同构**（否则「同一字节两种拼法」会被当成两个字面量 ⇒ 偏移错 ⇒ **静默错字节**）。**落点** = **新增 5 个**（`cgs_is_const_string` / `cgs_literal_hex` / `cgj_string_hex` / `cgj_escape_hex` / `cgs_hex_bytes`）+ **重塑 3 个** + **删 2 个**（`cgs_literal_text` / 死代码 `cgs_text_bytes_hex`）⇒ 文件 4303 → **4412 行**（**+109 行**），函数数 149 → **152**。**顺带闭合 G-e**（普查时新发现）：空字面量 `""` 此前也被拒 ⇒ 后端是「进 `data_offsets`、贡献 0 字节、指针 = 当前 data 长度」，guest 改后**完全一致**。**S 内普查（文本计数 2,943；STEP-0521 记 2,941，差的 2 个是本片新增的 concat）**：按**操作数**计 5,888 个 = 名字 3,308 / 调用或成员 1,004 / **带转义的字面量 952**（**G-b** 硬阻塞） / 普通字面量 619 / **空字面量 5**（**G-e** 硬阻塞）⇒ **G-d** 仍是硬阻塞 ⇒ 按 **D10 不得降级**。**自举形状前沿反而被拉回**：`cgs_literal_text` 曾在前沿外（`ERR:E-SH-IR-CALL-ARITY`）⇒ 搬到**字节域**后不再需要任何 text 形状的宿主调用 ⇒ 8 个新增/改动函数**全部**逐字节降低 ⇒ **未推进形状边界**。**实测约束（写进本节）**：`U64.checked_div` 这条路由**不在** guest 前端的路由梯里（`ERR:E-SH-IR-CALL-TARGET`）⇒ 字节数按 2 步长计数，**不引入除法**。**撞到并修掉 3 条自造缺陷**：①`\u0000` 只消费了 2 个字节（4 个 `0` 被当字面量 ⇒ guest data 段多 `30 30 30 30`）⇒ **差分测出来**（两侧都 1429 B 但字节不同）⇒ 改成连 4 个数字一起消费；②形参名用了保留字 `end` ⇒ Rust 前端拒 `unsupported atomic expression` ⇒ 改名 `close`；③哨兵歧义（空字面量 vs 解码失败）⇒ 改用 `ERR:E-SH-CG-IR-EMIT` 标记解码失败。**出口**：**4 条新钉** + **STEP-0521 真值表钉按新实测更新**（L / O / Q ⇒ 逐字节等同 **12 → 15** 行、组件层 typed 拒绝 **3 → 0** 行；`GuestVerdict::CodegenRefused` 变体随之无行使用 ⇒ 删除，否则 `dead_code` 会让 clippy `-D warnings` 变红）+ 固件 **141 → 145** + 前沿探针 **31 → 34** + 本机电池 **16/16** `BATTERY_GREEN`。**本片结论**：**T1 族仍不能收口** —— 收口片 X 必须等 **G-d**（前端可达性）做完；下一刀 = **G-d**，之后才是 **X**（真值表复跑 + 登记表定稿 + 独立 CI），之后才允许开 **T2 族**。
**T1 拍 4 前置结论之三（STEP-0523，2026-10-23 实测；实现片，**族仍未闭合**）**：**G-d 刀（第一刀）= 单语句 `return` 的通用操作数 ⇒ 前端路由复用 `let` 绑定路由的操作数降低器**。**真值（生产输出，非设计）**：改前 7 个「函数体只有一条 `return`、表达式是 `sico.text.concat`」的形状**全部**在前端被拒（`ERR:E-SH-IR-EXPRESSION`），而 Rust 后端**全部**能降（组件 1124 / 1113 / 1112 / 1177 / 1177 / 1208 / 1260 B）⇒ **改后 6 个形状逐字节等同**。**落点** = `selfhost/parser.sico` **重塑 1 个函数** `direct_text_concat_return_ir`（22342 → **22304 行**，**−38 行**；函数数 174 → 174）⇒ 路由**不再自己抄一份操作数规则**，改为复用 `let` 绑定路由一直在用的 `straight_text_concat_packed` ⇒ 「同一批操作数、两个路由给出不同答案」这个结构性差异消失（H / I / J / R 与 K / N / M 走**同一份规则**）。**失败语义保持不变**（降低器的 typed 身份 ⇒ `SKIP` ⇒ 仍是原来的 `ERR:E-SH-IR-EXPRESSION`）⇒ **没有偷偷换身份**。**自举形状前沿没有推进**（重塑后的路由与 `straight_text_concat_packed` 改前改后都在前沿内）⇒ 新形状是**复用**既有形状得到的。**本片最重要的发现 = G-d 其实是两个缺口**：S 内那 10 个调用点逐个定位后，剩下的 4 个**全部**在 `if` 块里，而它们的拒绝**不在前端** —— 前端**已经**逐字节降下来了，拒的是**组件发射**（guest 的发射器只支持单块直线代码 ⇒ **任何** `if` 都撞上它）⇒ 拆成 **G-d1**（前端，T1 专属，✅ 闭合，6 / 10）与 **G-d2**（**发射端分支缺口**，⏳ 未闭合，4 / 10）；**G-f**（调用型操作数带**两个实参**）仍在前端 ⇒ S 内 1 个调用点（**⚠️ 口径订正（STEP-0524）**：这是 **G-d 子集**的计数 —— **全 S** 的这种形状（G-f 面）实测是 **117** 个调用点，见 STEP-0524 §5）。**订正一条过期事实**：STEP-0518 的「guest 前端拒前向引用的用户函数调用」**实测不成立** ⇒ 闭包清单改成**从文件自动计算**。**出口**：**2 条新钉** + **真值表扩到 24 行**（H / I / J / R 翻成「逐字节等同」，新增 6 行，引入**第三种判定** `GuestVerdict::CodegenRefused`）+ **前沿探针 34 → 38** + 固件 **145 → 147** + 本机电池 **17/17** `BATTERY_GREEN`。**本片结论**：**T1 族仍不能收口** —— 剩下的是 **G-d2**（发射端分支支持，**不是 T1 专属**）与 **G-f**（两实参调用）⇒ 二者落地后才有收口片 X
**T1 拍 4 前置结论之四（STEP-0524，2026-10-23 实测；实现片，**族仍未闭合**）**：**G-f 刀 = 调用型操作数带多个实参 ⇒ 前端逐字节等同**。**真值（生产输出，非设计）**：S 的真实形状 `sico.text.concat(at(items, U64.literal(1)), sico.text.concat(".", at(items, U64.literal(2))))` 的 Rust canonical IR = **3513 B**、生产组件 = **1229 B**；改前 guest 对它 typed 拒绝（`ERR:E-SH-IR-EXPRESSION`）⇒ 改后**逐字节相同**；另测 3 个变体（两个形参 **3036/1185**、三个实参 **3189/1193**、两个字面量实参 **3120/1211**）**全部逐字节等同**。**Rust 前端把 `U64.literal(1)` 降成 `const_u64`（`{"op":"const_u64","data":1}`，range 只覆盖那个数字）** —— 这条是从 oracle 的 canonical IR 读出来的，**不是**设计。**落点 = 纯插入**：`selfhost/parser.sico` **新增 1 个函数** `stc_call_operand_packed`（**138 行**）并把 `straight_text_concat_packed` 调用分支的 `else`（原来直接 `ERR:E-SH-IR-CALL-ARITY`）**换成一次分派**（+17 行）⇒ 文件 22304 → **22458 行**（**+154**），函数数 174 → **175**；**单实参路径一个字节都没动** ⇒ `let` 路由的 6 行与 1 实参调用行**逐字节不变**（真值表实测）——**这个降低器是全族热点**，所以只加路径、不改旧路：最坏退回「1 个实参照旧、多个实参照旧被拒」，**不会连累既有形状**。**失败语义不变**（typed 身份透传 ⇒ 路由转 `SKIP` ⇒ 仍是 `ERR:E-SH-IR-EXPRESSION`；**实参个数 0 仍 `ERR:E-SH-IR-CALL-ARITY`**，不静默放宽）。**自举形状前沿没有推进**（新函数与共用降低器改前改后都在前沿内）。**撞到并修掉 2 条自造缺陷**：①实参区间是**左闭右开**，我拿字面量 `)` 的下标直接和区间末端比较 ⇒ 改成「`)` 的**下一个**词 == 区间末端」；②**臂内嵌套 `match` 不被 guest 的通用 CFG 路由支持**（最小复现 `ERR:E-SH-IR-STATEMENT`）⇒ 尾部改成「计数器 + 纯 `return`」⇒ 顺带把 `straight_text_concat_packed` 那份**手工维护**的闭包清单改成**自动计算**（`parser_closure_source`）。**出口**：**2 条新钉** + **真值表 24 → 29 行**（Z 行翻成「逐字节等同」；新增 3 行逐字节等同 + 1 行 `I64.literal(-n)` 实参仍 typed 拒绝 + **1 行「实参本身是嵌套调用」仍 typed 拒绝（BF 行，Rust canonical IR 3635 / 生产组件 1267）**）+ **前沿探针 38 → 39** + 固件 **147 → 149**。**S 内 G-f 普查（新增证据）**：G-f 面 = `sico.text.concat` 的操作数是**用户函数调用**、被调函数返回 `Text`、实参 ≥ 2 个 ⇒ S 内 **117** 个调用点（其中 **95** 个的实参全部落在新路径认领的集合内、**22** 个带**嵌套调用实参**）⇒ **订正** STEP-0523 记的「S 内 1 个调用点」（那是 **G-d 子集**口径）（**95** 个的实参全部落在新路径认领的集合内，**22** 个带**嵌套调用实参**）⇒ **订正** STEP-0523 记的「S 内 1 个调用点」（那是 **G-d 子集**口径）。**未认领**：`I64.literal(-n)` 实参（S 内**不存在**）、**嵌套调用实参**（S 内 **22** 个调用点 ⇒ **新登记 G-f2**）。**本片结论**：**T1 族仍不能收口** —— 剩下的是 **G-d2**（发射端分支支持，**不是 T1 专属**）与 **G-f2**（前端形状缺口，S 内 22 个调用点）⇒ 二者落地后才有收口片 X

**T1 拍 4 前置结论之五（STEP-0525，2026-10-23 实测；实现片，**族仍未闭合**）**：**G-f2 刀 = 调用型操作数的实参本身是嵌套调用 ⇒ 前端逐字节等同**。**真值（生产输出，非设计）**：5 个 G-f2 形状（内层调用 1 / 0 / 3 个实参、内层返回 `U64`、两层嵌套）的 Rust canonical IR = **3635 / 3559 / 4005 / 3618 / 3767** B、Rust 生产组件 = **1267 / 1261 / 1296 / 1250 / 1281** B；改前 guest **全部** typed 拒绝（`ERR:E-SH-IR-EXPRESSION`）⇒ 改后**全部逐字节相同**；既有 4 个 G-f 形状**逐字节不变**，`I64.literal(-n)` 实参**仍按设计拒**。**值槽规则（从 oracle 的 canonical IR 读出）** = 值 id **后序分配**（先给实参、再给调用自身的结果 id）、调用指令 range = **[被调名偏移, 闭括号偏移 + 1)**、`FunctionId` **1 基** ⇒ 这三条是「入口门槛②」要求的口径。**落点 = 内联，不新增函数**：`selfhost/parser.sico` 把嵌套调用的逻辑**内联进** `stc_call_operand_packed`（+56 / −4 行）⇒ 文件 22458 → **22510 行**、函数数 175 → **175**。**撞到并修掉 3 条自造缺陷**：①通用调用路由的**内建白名单**里没有 `sico.text.concat` ⇒ 「**内建调用的实参本身是调用**」撞 `ERR:E-SH-IR-CALL-TARGET`；②通用 CFG 路由要求**函数体最后一块终结**，helper 尾部是 `if/else` ⇒ `ERR:E-SH-IR-RETURN-COUNT`；①②合起来 ⇒ **新形状必须落在既有可降形状的延长线上** ⇒ 逻辑内联进**已被证明可降**的共用降低器，它**递归调用自己**（实测 guest canonical IR 与 Rust oracle **逐字节相同**，86,035 B）；③`scco_consumed` 统计「**实参个数**」而不是「**占用的 id 数**」 ⇒ 改成累加 id 数（既有形状行为完全不变）。**S 内 G-f2 普查**：**22** 个调用点（`codegen.sico` 10 + `parser.sico` 12）、内层调用实参数 0×9 / 1×1 / 2×8 / 3×4、内层返回种类 text×15 / u64×7、内层实参形态 形参 13 / 局部 12 / `U64.literal` 4、**22/22 全部可认领** ⇒ **一层嵌套即可覆盖全部 S 站点** ⇒ **G-f + G-f2 之后，G-f 面（117 个调用点）在 S 内不再有未认领的形状**。**出口**：**1 条新钉**（自举形状前沿）+ **真值表 29 → 33 行**（**BF** 行翻成逐字节等同 + 新增 **BG / BH / BI / BJ**）+ **前沿探针 39 → 41** + 固件 **149 → 150**（**150 passed / 0 failed**）。**本片结论**：**T1 族仍不能收口** —— 剩下的是 **G-d2**（发射端分支支持，**不是 T1 专属**，已作独立条目登记）⇒ 它落地后才有收口片 X
**独立条目登记：G-d2 = 发射端分支支持（**不属于 T1 族**）—— ✅ 已闭合（STEP-0526）**：STEP-0523 逐个探过 S 内那 4 个仍被拒的调用点之后确认，它们的拒绝**不在前端** —— 前端**已经**逐字节降下来了，拒的是**组件发射**：guest 的发射器（`selfhost/codegen.sico`）**只支持单块直线代码**（只认 `return` 终结符），而 Rust 的 IR 终结符有 **5 种**（`Return` / `Branch{condition,then,else}` / `Jump` / `Match{values,arms}` / `Unreachable`）⇒ **任何** `if` 都撞上它（实测：`if` 块里只返回一个字面量、连 concat 都没有，照样被拒）。**为什么它是独立条目而不是 T1 的某一拍**：①它**不是**「再补一个操作数形状」——它要的是**发射器的一个新特性**（多块发射 + Wasm `block`/`if`/`else`/`br`/`end` + **分支值槽** + `LocalLayout::plan` 在多块下的口径）；②它**不专属于 T1** —— **任何**用到分支的族都会撞上它；③把它挂在 T1 族名下会让**族级出口判据失真**。**入口门槛**：①多块发射的 IR 计划（`cgpr_` 目前的 `inst`/`op`/`term` 线性脚本要能表达多块）；②Wasm 侧的控制流与分支值槽；③**先量 oracle**：Rust 后端对同一 `if` 形状的**生产组件**与 core module code 段反汇编（这是真值，不是设计）。**出口判据**：某个 `if` 形状的**整组件与 Rust 后端逐字节等同** + 过真实 `wasmparser::Validator` + **自举形状前沿不推进** + 固件绿 + 独立 CI 绿。**归属**：M22 S5 的**发射端能力**（不是 W2 的某一族拍）⇒ 排在 **T1 收口片 X 之前**（否则 T1 族仍不能闭合）。**现状（STEP-0526）**：**G-d2 已闭合** —— 发射端逐块发终结符（`return` / `branch` / `jump` / `unreachable`）并补上 `equal_fixed` 算子 ⇒ **S 内两个真实形状整组件与 Rust 后端逐字节相同**（1266 B / 1316 B），族级真值表的组件层 typed 拒绝归零 ⇒ **下一片 = T1 收口片 X**。

**T1 拍 4 前置结论之六（STEP-0526，2026-10-24 实测；实现片，**族仍未闭合**）**：**G-d2 刀 = 发射端分支支持（独立条目落地）⇒ 整组件与 Rust 后端逐字节等同**。**真值（生产输出，非设计）**：**S 内两个真实形状**（`cgs_zero_hex` 的 2 个调用点、`declaration_name` 的 2 个调用点）的 Rust canonical IR = **3753 / 4387** B、Rust 生产组件 = **1266 / 1316** B ⇒ 改前 guest 是**组件层 typed 拒绝**（前端早已逐字节降下来）⇒ 改后**整组件逐字节相同**；另 4 个分支形状（AA 行 **1198** B、`if`/`else` **1100** B、`while` **1116** B、两个 `if` **1178** B）**全部逐字节相同**。**落点** = `selfhost/codegen.sico` **+142 / −31 行**（4413 → **4524 行**，函数数 152 → **156**）：**新增 5 个**（`cgs_flat_return_hex` / `cgs_branch_hex` / `cgs_jump_hex` / `cgs_block_terminator_hex` / `cgs_equal_fixed_hex`）、**重塑 3 个**（`cgs_function_instr_hex` 多一个 `terms` 标志、`cgs_function_body_hex` 的尾、`cgs_op_hex` 多一支）⇒ **单块函数的字节不变**（既有 33 行真值表**一个字节都没动**）。**终结符编码（从后端生产输出反汇编读出）** = `return` = 各槽 `local.get` + `0f`；`branch` = `local.get cond; 0440; i32.const then; local.set disp; 05; i32.const else; local.set disp; 0b; 0c01`；`jump` = `i32.const target; local.set disp; 0c01`；`unreachable` = `00`。**撞到并修掉 2 条自造缺陷**：①第一版把终结符放进**共用的** `cgs_function_instr_hex` ⇒ `run` 的**指令段**从 182 B 长成 350 B ⇒ **破坏 STEP-0502 的契约**（`--emit-run-instr-hex` = 到结果区返回**之前**为止）⇒ 固件抓红 ⇒ 改为**一个循环 + 一个 `terms: U64` 标志**按**调用方**分开；②嵌套 `if` 的形状仍被拒 ⇒ 定位为**前端**缺口（`ERR:E-SH-IR-STATEMENT`）⇒ 差分脚本加第三种判定 `frontend-refused`，**两类拒绝分开钉**。**自举形状前沿没有推进**：本片 **8** 个新增/重塑函数**全部**能被 guest 前端逐字节降低（新钉）⇒ 新形状是**复用**既有可降形状得到的。**出口**：**3 条新钉** + **族级真值表 33 → 36 行**（**AA 行翻成「逐字节等同」** + 新增 BK / BL / BN 三行；**组件层 typed 拒绝 0 行** ⇒ `GuestVerdict::CodegenRefused` 变体无行使用 ⇒ 按 STEP-0522 的既有做法删除，其行为改由「分支 `run`」的独立钉承载）+ **固件 150 → 153**（**153 passed / 0 failed**）。**顺带登记 G-d3**（发射端缺 `equal_fixed` 算子 ⇒ 连**分支的条件**都发不出来）—— **不是控制流缺口，是算子覆盖缺口** ⇒ 同片闭合。**本片结论**：**T1 族仍未闭合**，但**未闭合项已不再是 G-d2** —— 剩下的是**族外**的（整文件自举：`List[Text]` 形参 ⇒ `ERR:E-SH-IR-RECORD-TYPE`；宿主调用内建只认两个；嵌套 `if` 的前端缺口；**语料覆盖 0 条**）⇒ **下一片 = T1 收口片 X**（真值表复跑 + D15 定稿 + 前沿探针 + 电池 + 独立 CI 回填）。

**T1 拍 4 前置结论之七（STEP-0527，2026-10-24 实测；**收口片 X = 验证片**，**族闭合**）**：**T1 族按族级出口判据六项全过 ⇒ 闭合**。**本片性质** = 计划 §3.3.5 的**收口片 X**（钉住 + 纠偏 + 前沿探针期望值前移 + 固件电池 + 独立 CI 回填 + 诚实收窄）⇒ **不新增实现**（`selfhost/*` 与 `crates/*` **零改动**），只做对齐。**六项判据逐项核对**：①`canonical IR` 逐字节等同 = **35** 个形状（真值表钉住 **36 行** + 数据偏移 **2 行**）；②整组件逐字节等同（**组件层 typed 拒绝 0 行** —— AA 行由 STEP-0526 翻转）；③`wasmparser::Validator` 通过（oracle 与 guest 两侧）；④前沿探针期望值已翻转（**41** 条用例全部与期望一致：**30** 条 `lowered` + **11** 条故意钉住的 typed 拒绝）；⑤固件电池绿（固件 **153 / 153** + 本机 **BATTERY_GREEN 18/18**）；⑥独立 CI 绿（run #31 / #32 / #33，**裁决由下一片回填**）。**D10 复核（不得降级）**：S 内 T1 族的 **10** 个调用点**全部闭合** —— **6 个非分支调用点**（STEP-0518 / 0519 / 0523；**其中** 2 个多实参调用形状 ⇒ STEP-0524、2 个嵌套调用实参形状 ⇒ STEP-0525，**是那 6 个的子集、不另计**）+ **4 个 `if` 块里的调用点**（STEP-0526）⇒ **6 + 4 = 10 / 10** ⇒ **S 内不再有未覆盖的 T1 调用点形状** ⇒ **不需要降级**。**缺口登记**：G-b / G-e（STEP-0522）、G-d1（STEP-0523）、G-f（STEP-0524）、G-f2（STEP-0525）、**G-d2 / G-d3**（STEP-0526）**全部闭合** ⇒ **0 类未闭合缺口**。**诚实收窄（关键）**：**T1 族闭合 ≠ M22 闭合** —— M22 门 1 / 4 / 5 / 7 GO、门 2 PARTIAL、**门 3 / 6 NO-GO**、**整体仍 NO-GO**；**整文件自举仍未闭合**（`List[Text]` 形参 ⇒ `ERR:E-SH-IR-RECORD-TYPE`、宿主调用内建只认两个、嵌套 `if` 前端缺口 `ERR:E-SH-IR-STATEMENT`、`match` 终结符未实现、`run` 多块 fail closed）；**差分语料覆盖仍 0 条**（T1 族的形状一个都没进 `selfhost/corpus-v0.json` / `corpus-w1` ⇒ 族行为**只由族级真值表钉住**）。**方法学结论（写进本节）**：**「族闭合」与「固定点达成」是两个判断** —— 前者问「这个族声明的支持面，guest 是不是逐字节做到了」；后者问「guest 能不能编译整个冻结源集 S」。T1 闭合之后，S 的固定点仍差着**别的族**（宿主调用内建、`List[Text]` 形参、嵌套 `if` 前端形状、record 泛型、cell 的记录类型 …）⇒ **下一族 = T2 族**（`sico.text.length`，S 内 318 处）。

**T2 拍 1 结论（STEP-0528，2026-10-24 实测；**真值测量片 T**，**族未闭合**）**：**T2 = `sico.text.length`** ⇒ **条件位已经能逐字节降，语句位不能**。**真值（生产输出，非设计）**：S 内 **322** 个 `sico.text.length(` 调用点（**订正本节族表记的 318** —— S 在计划成文后的增长）；**条件位 3 个形状已逐字节等同**（`if` 条件 / `while` 条件 / 分支块内的语句：canonical IR = **2991 / 3446 / 3437** B，Rust 生产组件 = **1167 / 1187 / 1187** B）；**语句位 7 个形状仍 typed 拒绝** `ERR:E-SH-IR-EXPRESSION`（canonical IR = 2212 / 2212 / 2252 / 2212 / 2656 / 2369 / 2765 B）；**元数 2 行由 Rust 前端自己拒** `stdlib intrinsic arity`（区间 358..376 / 365..387 ⇒ **元数边界在语义层**，guest **不得**自造元数检查）；**组件层全部 typed 拒绝**。**关键发现 = 路由分家，不是能力缺失**：通用宿主调用路由 `gw_intrinsic_call_packed`（`selfhost/parser.sico:7001`）**已经认** `sico.text.length`（实参可为形参 / 局部 cell），而**直线路由家族** `straight_cell_function_ir`（`:9707` / `:10289`）**只认** `sico.text.concat` ⇒ 语句位够不着 ⇒ **下一刀 = 让直线路由家族认这个形状**（**覆盖 S 内 320 / 322 个调用点**），**不新造机器**（复用 `gw_intrinsic_call_packed`）⇒ 这与 STEP-0517 在 T1 上得到的结论同族：「**某形状被拒绝 ≠ 该形状没实现**」。**组件层真值（core module 反汇编）**：调用点 = `local.get pointer; local.get length; call <helper>; 0xad (i64.extend_i32_u); local.set result`；helper = core function 索引 **`helper_count + 3 + 名字排序位`**（`helper_count = import_count + module.functions.len()`；运行时 helper 占 0/1/2；**内建 helper 按名字排序** ⇒ `sico.text.concat` 恒为 0、`sico.text.length` 为 1），body **51 B**（locals 组 `2 × i32`），**数的是 UTF-8 码点**（非续字节）⇒ **codegen 刀要同时处理两个内建 helper 的索引**（今天 guest 只处理 `concat` 一个）。**出口**：**4 条新钉**（条件位逐字节等同 / 语句位保持 typed 拒绝 / 元数由语义层拒 / 发射端拒绝 + oracle 组件 1167 B 钉住）+ **前沿探针 41 → 46** + 固件 **153 → 157**。**撞到并修掉 2 条自造缺陷**：①第一轮测量对 **IR** 用 fixture、对**组件**用「CLI 变体」⇒ 两个不同模块 ⇒ 数字差 **3** B（函数名长度）⇒ 改用临时探针量**固件真正会构造的那个 fixture 模块**；②元数探针只记「RUST_REFUSED」而**没记是谁拒的** ⇒ 补记**语义层身份**与源码区间并钉住。**缺口登记（新建 [T2 支持面登记表](../../reports/m22-support-surface-t2-text-length.md)）**：**T2-g1**（语句位）、**T2-g2**（实参本身是嵌套调用，S 内 **2** 个调用点）、**T2-c1**（发射端不认识该算子）⇒ **三项全部未闭合** ⇒ **T2 族未闭合**；**语料覆盖 0 条**。**下一刀**：T2 **前端刀（F）**（让直线路由家族认 `sico.text.length`，纯插入、零既有行重排）⇒ 然后 **codegen 刀（C）**（`cgs_intrinsic_hex` 四道闸门 + 发射 51 B helper + 两个内建 helper 的名字排序索引）⇒ 然后 **收口片（X）**。
**T1 族新登记：G-f2 = 调用型操作数的**实参本身是嵌套调用**（前端形状缺口，**不是**发射端）**：STEP-0524 的 S 内普查（脚本 `target/tmp/step0524/census_gf.py`）测出 **G-f 面 = 117** 个调用点（操作数是用户函数调用、被调函数返回 `Text`、实参 ≥ 2 个），其中 **95** 个的实参全部落在 `stc_call_operand_packed` 认领的集合内（形参 / 局部 / 字符串字面量 / `U64.literal(n)`），剩下 **22** 个带**嵌套调用实参**（`f(g(x), y)`）⇒ 实参分类落到 `ERR:E-SH-IR-CALL-ARGUMENT` ⇒ 路由转 `SKIP` ⇒ 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`。**真值（生产输出，非设计）**：该形状的 Rust canonical IR = **3635** B、Rust 生产组件 = **1267** B ⇒ **Rust 两个 oracle 都能降，guest 不能** ⇒ 是**真缺口**（已钉进族级真值表 **BF** 行）。**入口门槛**：①把「实参是调用」变成**可识别的实参形态**（需要能确定被调函数的返回类型与实参的值槽）；②**值槽编号**必须与 Rust 前端一致（实参值 id 的分配顺序）⇒ 先量 oracle 的 canonical IR 再动代码。**出口判据**：该形状的 **guest IR 与 Rust canonical IR 逐字节相同** + **guest 组件与 Rust 后端逐字节相同** + 过真实 `wasmparser::Validator` + **自举形状前沿不推进** + 固件绿 + 独立 CI 绿。**归属**：**T1 族的前端** ⇒ 与 **G-d2** 一起，**两者都落地后**才有收口片 X。**现状（STEP-0525）**：**G-f2 已闭合** —— 逻辑**内联进** `stc_call_operand_packed`（**不新增函数**，因为单独写的 helper **guest 前端降不下**：通用调用路由的内建白名单里没有 `sico.text.concat`，且函数体尾部是 `if/else` ⇒ 通用 CFG 路由判 `ERR:E-SH-IR-RETURN-COUNT`）⇒ 剩下的只有 **G-d2**（发射端，**独立条目**）⇒ 它落地后才有收口片 X。

**T1 拍 4 前置结论（STEP-0521，2026-10-23 实测；实现片，**族仍未闭合**）**：**族级真值表 + D15 支持面登记表 + 缺口定位 + 一处静默偏移修正**。**为什么先做它**：前三拍（真值 T / 前端刀 F / codegen 刀 C）+ 共享前置（cell）都只钉**一个形状** ⇒ 「族支持到哪一步」从来只是一串逐片声明，不是**一张表**；D15 要求每族闭合时登记「族 × 子集边界 × 语料覆盖 × 未覆盖形状」⇒ **先把族级事实钉死再谈收口**。**族级真值表（20 行，四轴实测）**：**逐字节等同 12 行** / **组件层 typed 拒绝 3 行**（字面量含 JSON 转义）/ **前端 typed 拒绝 4 行**（函数体只有一条 `return` 且操作数不是两个形参）/ **修正前静默错字节 1 行**（两个函数各带一个字面量 ⇒ 组件**同长但字节不同**，数据指针差 1）⇒ 整张表**钉进固件**。**方法学复现**：「某形状被拒绝」≠「该形状没实现」—— 被拒的是**路由门**（`body_statement_count = max(let_count, set_count) > 0`），加一条无关语句后**同样两个操作数**就逐字节降下来 ⇒ 前沿探针新增 3 条用例（28 → **31**）全判 `lowered`。**修正的到底是什么**：后端 `script_data` 收**整个模块**的字面量并对重复字面量**去重**，guest 的 `cgs_data_offset` 却只扫**当前函数** ⇒ 偏移偏小 ⇒ **静默发出错误字节**（fail-closed 纪律唯一不允许的结果）⇒ 修正后与 `cgs_data_hex` **同构**，同时修掉重复字面量被误拒。**自举形状前沿反而被拉回**：修正前这两个函数带**嵌套宿主调用** ⇒ guest 报 `ERR:E-SH-IR-CALL-SHAPE`（在前沿之外），按 STEP-0515 的既有修法拆成临时变量后**现在都能被 guest 逐字节降低**（`cgs_literal_text` 仍在前沿外，`ERR:E-SH-IR-CALL-ARITY`）。**S 内普查（文本计数 2,941）**：**1,576** 个调用点的操作数是字面量 ⇒ **G-b**（转义字面量）是固定点硬阻塞；**10** 个落在没有 `let`/`set` 的函数体里 ⇒ **G-d**（单语句 `return` 的非形参操作数）是硬阻塞 ⇒ 按 **D10 不得降级**。**本片结论**：**T1 族不能收口** —— 收口片 X 必须等 **G-b** 与 **G-d** 两刀做完；下一刀 = **G-b**（guest 反转义 JSON 字符串，且把 `cgs_literal_text` 拉回前沿内），再 = **G-d**（前端可达性），之后才是 **X**（真值表复跑 + 登记表定稿 + 独立 CI），之后才允许开 **T2 族**。

**串行规则（D9）**：`selfhost/parser.sico` 与 `selfhost/codegen.sico` 是**共享热点** ⇒ **W2 期间禁止并行开族**，同一时间只允许一个族改动这两个文件；W3 起才允许按族并行，且并行双方的改动面必须分离（以**改动文件交集**判定，不以 STEP 号判定）。

**降级规则（D10，与止损 T-R3-b 配套）**：某族连续 3 片未闭合时可降级为"最小子集形"，但**降级后若 S 内该族仍存在未被覆盖的调用点形状 ⇒ 不得降级，必须继续闭合**（固定点要求 guest 能编译 S 自身）。每次降级必须登记进**支持面登记表**（D15）并写明未覆盖形状。

**支持面登记（D15）**：每族闭合时同步更新"族 × (名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状)"表；门 7 之前汇总为 `docs/reports/m22-support-surface-*.md`。

#### 3.3.6 S5-W3：组件级广度 rung（`formatter.sico` 组件级 → 37 条 accepted 语料组件级差分）

**W3-a：`formatter.sico` 组件级**（对应 canary 梯级 2）

`formatter.sico` 的 **IR 面已字节全等**（2026-10-20 出口审计记 canary 30/30 CLOSED）⇒ 它的组件级失配可**单独归因于 codegen**，是 codegen 正确性最干净的单点测试，也是全组件规模的第一次演练（794 行 / 22.6 KB）。

- **出口判据**：guest 对 `formatter.sico` 发射的组件 == Rust 后端**生产输出**，逐字节相同，过真实 `wasmparser::Validator`。
- **纪律**：若此处失配 ⇒ 回到 W2 找 codegen 缺口，**不得**用"改 fixture"或"改期望值"绕过。

**W3-b：37 条 accepted 语料的组件级差分**（对应 canary 梯级 3）

冻结事实：`selfhost/script-build-corpus-v0.json` 已冻结每条的 `component_sha256` 与 `component_reproducible`。

| 组 | 内容 | 条数 | 难点 |
|---|---|---|---|
| G1 | 端到端家族（bit-ops、block-solver、capability-state-machine、checked-mul-div、control-flow-counter、error-propagation、for-loop×3、infix-equality、list-i64-family、list-i64-sort、list-numeric、map-keys-traversal、map-set-frequency、prng-xorshift、recursion×2、stdlib-batch2、stream-transform、modules-report/main） | 20 | `block-solver.sico` 1,280 行 = 单条最大；覆盖 for/while/match/record/list/map/checked 算术 |
| G2 | 脚本 ABI 家族（script-echo、script-args-echo、script-file-transform、script-fs-escape、script-http-request、script-http2-request、script-json-filter、script-reject、script-stream×4、script-task×3、script-word-count） | 17 | 拉进 stream / task / http / fs 宿主能力面 ⇒ **难度最高，排最后** |

**出口判据（每条）**：guest 发射组件 == Rust **生产输出** == 冻结 `component_sha256`；过 `wasmparser::Validator`；字节差 = 0。可按组推进，但**组内必须逐条钉住**，不得以"组内抽样"代替全量。

#### 3.3.7 S5-W4：178 条拒绝的一致性

判据 = **三元组逐条一致**：`(verdict, diagnostic id, span)`。

| 组 | 条数 | 要求 |
|---|---|---|
| R1 LEXICAL | 116 | guest 词法器给出的诊断 ID 与 source span 与 Rust 完全一致 |
| R2 SCRIPT_ABI | 28 | 脚本 profile 的入口签名 / 资源 / 能力拒绝一致 |
| R3 散码 | 34（约 20 个不同 E 码） | 逐码对齐；每码至少一条正例 + 一条 limit+1 |

**工具面（D11，已拍板）**：新增 `tools/probe-m22-corpus-parity.ps1` —— **只读**冻结语料，逐条跑 guest 并输出三元组，与 Rust 侧输出对拍；冻结期望落 `selfhost/corpus-parity-v0.json`（随语料冻结，**不在探针里写死**期望值）。**探针不得修改冻结语料**；期望值文件必须可版本化、可 diff。

#### 3.3.8 S5-W5：组件级尺度 rung（完整 self-host bundle 的组件级全等）

对应 canary 梯级 4（`formatter.sico` 组件级已前移至 W3-a）。

1. 5 文件 bundle（`compiler.sico` + `compiler_lexer.sico` + `compiler_parser.sico` + `parser.sico` + `codegen.sico`）组件级 ⇒ 目标 ≈ **1,024,613 B** 逐字节等同。**这是 S5-W1 的真正验收点。**
2. 9 文件全集（含 `checker.sico` / `compiler_semantics.sico` / `formatter.sico` / `parser_driver.sico`）组件级。
3. **必要条件（不得跳过）**：guest 必须能**整体降低 `codegen.sico` 与 `compiler.sico`**——STEP-0513 §5.2 实测仍因 `codegen.sico` 的 `List[Text]` 类型形状触发 `ERR:E-SH-IR-RECORD-TYPE`。这不是可选项：固定点要求 guest 能编译 S，而 S 包含这两个文件（D3 已定为**固定点的必要条件**）。
4. **执行拓扑（承接 ADR-0015 决策 1–3）**：`A = R(S)` 由 Rust 前端 + 独立 IR verifier + RFC-0011 后端产出（能力已存在于 `runner/sico-runner/tests/support/selfhost_common.rs::cached_component`，须固化为**独立脚本 + SHA 记录**，D12）；`B = run(A, S)`、`C = run(B, S)` 在真实 runner 内执行，**S 以有界数据喂入**（单块 ≤ 64 KiB、总量 ≤ 2 MiB、超限 fail-closed）。
5. **不得**把 W5 的组件字节等同当作门 3 通过——门 3 还需要 `B` 与 `C` 的**执行**证据（W6）。

#### 3.3.9 S5-W6：ADR-0015 自举闭环（门 3）

按 ADR-0015 决策逐条映射：

| ADR-0015 条款 | 本阶梯的可执行动作 |
|---|---|
| 1 `A = R(S)` | Rust 前端 + 独立 IR verifier + RFC-0011 后端产出组件 A（能力已存在于 `runner/sico-runner/tests/support/selfhost_common.rs::cached_component`；需固化为正式证据面） |
| 2 `B = run(A, S)` | 真实 runner 内执行 A，S 作为**有界数据**输入，输出组件 B |
| 3 `C = run(B, S)` | 同输入同限额执行 B，输出组件 C |
| 4 `A == B == C` 字节全等 + 每个 artifact 过 Component 校验；不符即 typed bootstrap failure（带 stage + digest），**不提升任何 artifact** | 固定点比对与 digest 记录 |
| 5 语料双跑（R 与 B）字节一致 + typed 拒绝一致；Rust IR verifier 仍是永久 oracle | 复用 S5-W3 / S5-W4 的产物 |
| 6 M7 打包：RFC-0015 canonical `.sapp` 字节；开发执行用 RFC-0016 显式本地信任，发布演练用 ADR-0006 角色分离元数据；guest **无**环境文件系统 / 网络 / 凭据 / UI / capture / input 权限（源进有界输入、产物出有界输出） | `.sapp` 打包链 + `sico-app` 执行自编译 |
| 7 证据清单固定：源路径 + SHA-256、编译器 revision、工具链/runner 版本、target triple、限额、语料身份、A/B/C 摘要、包摘要、签名者类别；未知字段 / 重复路径 / 路径穿越 / 截断 / 尾随数据 / 溢出 / 每个 limit+1 全部 fail-closed | hostile manifest / limit+1 语料 |
| 8 预算实测：`B(S)` 的 cold/warm wall time、runner fuel、峰值 guest 内存、hostcall 次数与 fuel、输入/输出字节、超时/取消结果；**限额在合格跑之前固定**，失败后不得上调（owner 决策 2 允许**事前**重基线，但必须新开 attempt 并记录） | 预算表（观察值，**非 SLA**）+ M19 编译吞吐对照（见 W7）；**耗量读数口径（D13）**：若 Runtime ABI 无 fuel 实际耗量读数出口 ⇒ 登记为**上界**（限额取值）并显式标注"上界，非读数"，**不阻塞 GO** |
| 9 晋级是加法式的：Rust 编译器保持默认与永久差分 oracle；选用自举编译器需显式 CLI/profile 选择；last-known-good A 与 Rust 构建路径保持可恢复 | 选择路径与回退路径的显式测试 |

#### 3.3.10 S5-W7：门 6/7 收尾

1. **提交快照**：把 STEP-0352–0513 及后续全部工作提交并推送（owner 授权后），快照必须含独立 CI workflow 可跑的完整状态。
2. **独立 CI**：在提交快照上跑 `m22-w1-ci.yml`（Windows GNU），记录 run id 与结论。
3. **全量 M0–M21 回归**：`tools/run-ci.ps1` 全绿 + runner workspace 全量 + 固件电池 + 各前沿探针。
4. **新出口审计**：按 §4 七门逐项给出 GO/NO-GO + 证据表 + 剩余工作清单；**必须重审**（现行的 2026-10-20 审计早于 S5-W2 起的全部推进）。
5. **`origin/dev` 冲突历史裁决**（owner 决策 3 的触发点；**裁决方式见 D14**）：**不 rebase、不 merge**；M22 的完成判据**不包含**与 `origin/dev` 的合并；冲突历史登记为 **M25 集成议题**；M22 交付 = `codex/m22-w2` + `codex/m22-w1-ci` 的完整提交 + 独立 CI 证据。
6. **门 4 补强**（不阻塞 GO）：燃料**实际耗量**的 ABI 读数（或按 D13 登记为上界并标注）；M19 编译吞吐对照实测。
7. **支持面登记表（D15）**：汇总族 × 子集边界 × 语料覆盖 × 未覆盖形状，落 `docs/reports/m22-support-surface-*.md`。
8. **同步**：STATUS / ROADMAP / 本计划 / handoff / steps-README 同步更新；出口审计报告落到 `docs/reports/`。

#### 3.3.11 文档差异登记（**已于 2026-10-22 全部裁决关闭**）

| 编号 | 差异 | 裁决与状态 |
|---|---|---|
| D1 | ADR-0016 决策 6 的 canary 次序 = *formatter.sico → 完整 self-host bundle → 37 条 accepted 语料 → 确定性 Component codegen → ADR-0015 harness*；STEP-0486 §2.4 的表述 = *37 语料 → formatter → 全源 bundle → harness* | **✅ 已关闭（ADR-0016 A1）**：组件级次序冻结为 **formatter.sico 组件级 → 37 条 accepted 语料组件级差分 → 完整 bundle 组件级 → ADR-0015 harness**（先广度后尺度）；STEP-0486 §2.4 已加**对齐注记**（原句保留以便追溯）。**阻断纪律不变**。此前"未决期间不得宣称任何梯级闭合"的限制**随本裁决解除** |
| D2 | 2026-09-28 审计记 formatter 29/30；2026-10-20 审计记 30/30 CLOSED | **✅ 已关闭（取更晚且有证据的口径）**：30/30 CLOSED 为当前 IR 面口径；**组件面未闭合**（属 W3-a），两者不矛盾 |
| D3 | STEP-0513 §5.2：整文件自举未关闭（`codegen.sico` 的 `List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`） | **✅ 已关闭**：定为**固定点的必要条件**，写入 S5-W5 第 3 项，**不得降级为可选项**；未覆盖形状须进支持面登记表（D15） |

#### 3.3.12 止损、并行与预算纪律

1. **R3 既有规则继续适用**：自 W2 起连续 4 个实现片若 canary 零增长且拒绝前沿不动 ⇒ 登记止损并触发 M23 Route B（文档/测量片不计入）。
2. **新增触发器 T-R3-a**：S5-W1 判读为"不可行" ⇒ **立即停止 W2/W3 的顺序推进**，转入 ADR / RFC 修订流并升级给 owner。
3. **新增触发器 T-R3-b**：S5-W2 任一内建族在 **3 片内**无法闭合 ⇒ 该族降级为"最小子集形"并登记，优先推进覆盖 S 调用点的族，避免长尾陷阱。
4. **限额纪律**：任何 arena / fuel / timeout 的上调必须**事前**进行、以新 attempt 记录，并写入门 4 预算表；失败后上调即作废该次 attempt。
5. **并行会话纪律**：同树存在并行 M23 会话（M23 已占 STEP-0489/0490；M24 在独立 worktree 占 STEP-0330–0337）。改动面必须互不重叠：M22 线触 `selfhost/*.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、`docs/plans/M22-*`、`docs/handoff-m22.md` 与 M22 的 STEP 文档；M23 触 `docs/ROADMAP.md` / `docs/plans/M23-*` 与 M23 普查工具。**冲突面以改动文件交集判定，不以 STEP 号判定。**
6. **每片强制纪律（沿用本线既有实践）**：真值必须来自 oracle **生产输出**（不是复刻）；oracle 生产代码零改动；typed 契约只可收紧不可放宽；新代码零注释（注释纪律校验）；纯 LF、零 BOM（行尾审计）；固件电池 + 前沿探针 + 独立 CI 三件套；诚实收窄段必须写明未覆盖形状。

#### 3.3.13 规模估计与关键路径

| slice | 片数估计 | 备注 |
|---|---|---|
| S5-W1 | 1–2 | 纯测量 |
| S5-W2 | 40–60 | 20 族 × 2–3 片（复杂族 4 拍，简单族可合并 T+F 或 C+X） |
| S5-W3 | 21–42 | W3-a `formatter.sico` 组件级 1–2 + W3-b 37 条语料 20–40（逐条钉 = 上界；按组合并可压缩，但组内仍须逐条钉） |
| S5-W4 | 3–6 | 三个诊断组 + 新增 `tools/probe-m22-corpus-parity.ps1` 与冻结期望文件（D11） |
| S5-W5 | 3–5 | 5 文件 bundle（≈1.02 MB）/ 9 文件全集 / 整文件自举前提 / `A` 产出固化 |
| S5-W6 | 6–8 | ADR-0015 九条款 + hostile 语料 + 预算 + `.sapp` 链 |
| S5-W7 | 4–6 | 快照 / CI / 回归 / 复审 / 冲突裁决（D14）/ 门 4 补强（D13）/ 支持面登记表（D15） |
| **合计** | **≈ 75–125** | 关键路径 = W1 → W2 → W3 → W5 → W6 → W7 |

**串行代价（D9 的直接后果）**：W2 的 20 族因共享 `selfhost/parser.sico` / `selfhost/codegen.sico` 而**必须串行** ⇒ 片数不等于可并行度；W2 是整条关键路径上**最不可压缩**的一段（≈40–60 片）。若 owner 希望压缩墙钟时间，正确的压缩点是"减少每族的片数"（合并 T+F 或 C+X），**不是**并行改共享文件。

**估计的诚实边界**：本表是**规划估计，不是承诺**。最大不确定源有三处：① W1 可能把 W2 的实施方式整体改写（流式发射 vs 现有 hex 拼接）；② W3 的 G2 组会拉进 stream/task/http/fs 宿主能力面，其宽度未知；③ W4 的 116 条 LEXICAL 一致性依赖 guest 词法器的 span 语义与 Rust 逐条对齐，可能出现成片的系统性偏差（而非逐条个例）。

## 4. Exit gates

1. L1: formatter differential byte-exact + idempotent on the frozen
   corpus; checker subset differential green with a declared match/subset
   relation.
2. L2: Sico compiler compiles the frozen corpus; every artifact
   byte-equal to the Rust compiler's.
3. Bootstrap closure demonstrated (S6), including the M7-packaged,
   runner-executed self-compile.
4. Performance: self-compile wall-time, fuel and hostcall budgets
   measured and recorded; regression against the M19 compile-throughput
   budget is registered, not hidden.
5. Dual-implementation register: the Rust compiler stays the oracle;
   neither implementation is deleted or demoted silently.
6. Full M0–M21 regression green + explicit audit.
7. Every claim maps to executed runner evidence (`contract-verified`
   stays distinct from runtime support).

## 5. Evidence classes

STEP-0254 reuses the prepared compiler only inside the differential harness.
Every fixture still executes with a fresh Store and the original limits.
Measured serial suite time is 66.99s versus STEP-0253's 801.46s; this is test
harness overhead reduction, not guest self-compilation performance evidence.

STEP-0317 extends the harness economics (owner directive 2026-09-29): a
shared on-disk component cache keyed by the selfhost source texts plus a
compiler-crate fingerprint (kill switch `SICO_SELFHOST_COMPONENT_CACHE=0`),
opt-level 1 for runner dev dependencies, and a lazily-grown pool of
engine-isolated prepared programs replacing the global mutex. On this host
the filtered single-test invocation drops 23.59s → 1.82s warm (4.55s cold),
the differential suite 57.05s → 12.98s serial (9.35s pool-parallel, 51/51),
and the full runner suite 227.8s → 102.7s. Still harness overhead reduction
only: fixture limits, fresh-Store isolation and the byte-exact oracle are
unchanged, and none of it is gate-4 self-compilation performance evidence.

All M22 evidence is `internal-fixture`. Differential corpus runs are
runtime evidence when executed through a real runner; corpus presence
alone stays `contract-verified`. No external-pilot or production claims
arise from self-hosting.

Validation-host note: STEP-0254 used the pinned Windows MSVC cache. STEP-0280
subsequently provisioned the local Windows GNU path and ran the real runner
with `1.98.0-x86_64-pc-windows-gnu` plus the repository-local
`target/tooling/msys2-binutils/mingw64/bin` gcc/dlltool. Reproduce with the
commands in [the M22 handoff](../handoff-m22.md) §4 and report the actual
toolchain/runner used; toolchain setup is not a product support claim.

## 6. Non-goals

- Runner, Host providers, DAP, MCP server or desktop/web hosts in Sico.
- Rewriting the Rust IR verifier, package trust chain or registry.
- Retiring the Rust compiler/toolchain, or any "no Rust" claim.
- Performance-parity claims between the Sico-written and Rust
  implementations.
- Self-hosting profiles beyond Script profile v0 in the first closure
  (scalar profile follows only as a later, separately gated slice).
- Any language-surface change outside accepted RFCs (RFC-0033 gate).

## 7. Risks

- `List[record]` is outside the executable set: S3 starts
  struct-of-arrays; an accepted extension RFC may replace it later.
- Compile throughput regression is expected; budgets are re-measured at
  S6 and registered per gate 4.
- Language-surface churn invalidates the frontend: S3+ starts only after
  conditions 3–4 of the entry gate are closed.
- Deep recursion in a Sico-written parser consumes the bounded recursion
  budget; parser depth is budgeted per corpus (limit+1 fixtures).
- Host-toolchain reproducibility: the GNU evidence path depends on
  msys2-binutils plus a real C compiler; STEP-0280's local provisioning
  passed, but a different host must first verify both tools and pin the
  matching toolchain (see §5), or a rebuild can fail at `ring`/
  `windows-sys` before reaching the differential test.

## 8. v1 batch 3 candidates — recorded language friction (owner session directive 2026-09-14)

Measured friction that RFC-0044/0045/0046 did not close. These are
**candidates for a future v1 batch 3 RFC**, not commitments; each needs the
RFC-0033 gate (explicit desugar, source-map identity, formatter idempotence,
typed diagnostics) before implementation. The binding context: the M22
self-host frontend and AI-generated Sico consume exactly this surface, so
every item below is generation-noise cost multiplied by corpus size.

1. **Infix comparison and logical operators do not exist.** The only infix
   operator is `==`, and only in `if` condition positions (RFC-0044). The
   lexer punctuation table carries exactly `== <= ( ) [ ] : , . @ + - =`:
   `<`, `>`, `>=`, `!=`, `&&`, `||`, `!` are all absent. Consequences in
   executable code today: comparison is `I64.less_than(...)` (a
   `Result[Bool, NumericError]` match ceremony), logical conjunction is
   nested `if`s, arithmetic is `checked_add` with a full match per
   operation. RFC-0044 itself was accepted because measured test-point
   friction (Web/UI apps rewriting `==` as `match`) proved this class of
   friction real; the same friction necessarily repeats on the remaining
   operators. A batch-3 RFC must decide the smallest closed operator set
   and its desugar shapes, not grow the punctuation table ad hoc.
2. **No expression-position conditions.** `match` freezes the all-return
   statement shape (STEP-0083): `let x = match …` and `let x = if …` do
   not exist, so every conditional value requires a mutable cell, an
   all-return match, and a join read. This is the single largest
   structural verbosity tax in application code and directly multiplies
   AI-generation noise.
3. **Bare literals are arbitrary-precision `Int`; fixed-width code is
   literal-ceremony.** Every fixed-width constant is `I64.literal(…)`,
   and negative constants can only be written `I64.literal(-7)`. RFC-0046
   D3 kept this deliberately (NUM-001 oracle preserved) — it is an
   accepted cost, not debt. It is also, by measurement of generated-code
   review, the largest single AI-generation-noise cost in the current
   surface. **Re-measurement policy:** before any batch-3 bare-literal
   proposal, re-weigh the cost on a real generated-code corpus (AI output
   reviewed per STEP-0175-era DX audits), not on recollection; the D3
   decision stands until that measurement lands.

Items 1–2 are blocked on the same evidence bar as RFC-0044 (a measured
consumer); item 3 is blocked on the re-measurement corpus. None of these
may be implemented inside M22 slices without their own accepted RFC
(RFC-0033 gate; §3 slice rule).
