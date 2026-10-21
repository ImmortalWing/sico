# M22 Compiler self-host track

> Status: in progress / NO-GO after STEP-0485 exit re-review (2026-10-20, [audit](../reports/m22-exit-audit-2026-10-20.md)): gates 1/4/5/7 GO, gate 2 PARTIAL, gates 3/6 NO-GO. W2 line since STEP-0352 reached census 149/149 byte-exact + full-source parser.sico self-compile lowering (exit 0, byte-equal) via STEP-0484 arena treadmill; S5 component-codegen wave opened at STEP-0486, whose first slice (STEP-0487) emits a real script Component byte-equal to the Rust backend, and whose second slice (STEP-0488) locates the blocker in front of the IR-driven path: the guest frontend still cannot lower Script-profile shapes (record projection / variant / record construct) — STEP-0491 closes the first of those three (declared-record field projection now lowers to the canonical `project` op byte-equal to the Rust oracle; variant and record construct still refused) and STEP-0492 generalizes that projection from a hardcoded `ScriptInput` base to any declared record, and STEP-0493 closes the named-return-type precondition for record construction / variants (the guest now lowers `returns <declared record>` to the canonical `{"kind":"named",...}` return type byte-equal to the Rust oracle, and the malformed empty-kind IR that named return types used to emit is now a typed refusal), and STEP-0494 closes the record construct itself (`ScriptOutput(stdout: ..., ...)` lowers to the canonical `construct` op with a per-field instruction ledger, byte-equal to the Rust oracle), and STEP-0495 closes the third and last Script-profile frontend shape — the built-in `Result` variants `ok(...)` / `error(...)` lower to the canonical `variant` op (payload lowered first, the variant referencing those result ids, `return_type` always in declared `ok`/`error` order), byte-equal to the Rust oracle, with the payload type checked against the declared arm so a mismatched payload stays a typed refusal instead of a malformed IR; STEP-0495 also registers a newly measured capacity boundary — the whole-module canonical IR grew to 8,608,383 B and no longer fits one Script v0 channel (RFC-0029 freezes a channel at 8 MiB), so the guest compiler gained an `--emit-ir-split` driver mode that halves the IR across stdout and stderr for the full-source census. The three STEP-0488 frontend gaps (projection / construct / variant) are therefore closed; STEP-0496 opens the **guest IR ingestion** half of S5 — `selfhost/codegen.sico` gains a byte-oriented canonical-IR scanner plus a `run` emission-plan extractor, `selfhost/compiler.sico` gains the `--emit-ir-plan` driver mode, and the plan text the guest derives from the scanned IR is byte-identical to the plan the Rust oracle derives from the same `sico_ir::Module` (the oracle-side generator lives in the test file, not in the oracle crate), with four typed refusal identities (`ERR:E-SH-CG-IR-PLAN` / `-TYPE` / `-OP` / `-TERMINATOR`) for the shapes outside the plan subset and a new reproducible probe `tools/probe-m22-ir-plan-frontier.ps1`; what remains on gate 3 is run-body emission, the 37-corpus / formatter / full-source canary ladder and the ADR-0015 `A == B == C` + `.sapp` chain. STEP-0497 is a CI repair slice on this line, not a gate advance: it closes the M23-session CI red point (`sico-app-cli::run_executes_a_script_package_end_to_end`, `forcing trap when growing memory to 2214592512 bytes`) by aligning the Script package's memory limit with the guest linear memory every script component declares (33,792 Wasm pages = 2,214,592,512 B, emitted with minimum == maximum) — the binding constraint was the **package manifest's** 64 MiB (inherited from `RuntimeLimits::default()`), not the Host's 2 GiB grant, so raising only the Host grant left the case red; the fix keeps the emitted bytes unchanged (no `MEMORY_PAGES` change, no snapshot regeneration), raises the package limit to the declared memory, raises the Host grant to 2,560 MiB (the same value the in-process runner path enforces) and turns "ceiling below the declared minimum" into a typed refusal at both the package and the runtime layer, with four runtime-binary-free invariant guards (codegen byte-level / package / runtime / runner) and a matching RFC-0029 STEP-0497 amendment that separates the 2 GiB arena ceiling from the declared linear memory and registers the "stricter package/Host limit" clause as unimplementable for script components; the independent CI run 39524849306 for commit 7e5ad3c concluded `success`, so the handed-over red point is green on an independent runner and STEP-0497 is closed; STEP-0499 opens the emission half — a guest-side slot / local-declaration plan (`cgs_run_function` / `cgs_flat_hex` / `cgs_record_hex` / `cgs_local_hex` in `selfhost/codegen.sico`, the `--emit-locals-hex` driver mode in `selfhost/compiler.sico`) whose bytes are byte-identical to the local declaration section the Rust backend actually emits, and the check reads the oracle's production output (the component `compile_script_program` really produces) instead of a re-implementation of the plan generator; the emission is split in two precisely so that a mismatch in the next slice can be attributed to the slot plan or to the instruction emission; the independent CI run 39535461745 (head_sha ee2317f) concluded `success` with all 12 inner steps green, so the slice is closed on an independent runner and not only on this workstation, and the environment-sensitive budget assert it registered passes there; STEP-0501's independent CI run 39549599204 (head_sha a730ac6) concluded `success` with all 12 inner steps green as well, so the measurement slice is closed too; STEP-0502 then emits the instruction segment -- the guest derives the run body's instruction segment (dispatcher shell, per-block dispatch heads, block instructions) from the ingested IR, and the bytes are identical to the segment the Rust backend actually emits (182 hex), checked against the oracle's production output; the guest gains 28 new functions and a `--emit-run-instr-hex` driver mode, two typed refusal identities cover the shapes outside the subset, and the slice records four measured guest-frontend constraints (the `record` keyword may not name a parameter, `let` has no block scope inside a function, `U64.equal` yields a Bool, and a "not found" sentinel may not double as a total) plus two self-inflicted defects the byte comparison caught; the next slices are pre-cut: STEP-0503 (guest emits the Script result-area return, which needs five frozen component-level facts), STEP-0504 (the derived run body replaces STEP-0487's frozen hex). gate 3 is untouched and remains NO-GO. the shapes outside the subset (`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-FLAT`); STEP-0501 measures the remaining truth rather than implementing it: the `run` body the Rust backend actually emits is read from the production component, decoded byte by byte into a reproducible emission table (dispatcher shell / block dispatch / project / const_i64 / construct / variant / Script result-area return / epilogue), and pinned by a new firmware test together with an unambiguous cut marker (`i32.const 32` + `call 1`) that separates the instruction segment (182 hex) from the result-area return plus epilogue (178 hex). The measurement also settles a fact gate 3 depends on: the reference Script profile emits no data section, so the run body is the only IR-dependent part of the component; the slot table it yields is byte-identical to STEP-0499's local declaration plan. The next slices are pre-cut: STEP-0502 (guest emits the instruction segment, no component-level dependency), STEP-0503 (guest emits the Script result-area return, which needs five frozen component-level facts), STEP-0504 (the derived run body replaces STEP-0487's frozen hex). gate 3 is untouched and remains NO-GO. S1/S2 declared subsets complete; S3/S4/S5 partial; S6 not entered; S7 reviewed NO-GO. STEP-0300 independent CI succeeded on f2fd7fc (run 36212509946), and the 8bee75b snapshot containing STEP-0301–0305 succeeded (run 36366432845). STEP-0306–0309 are local only and advance the formatter canary to 29/30, with `main` / SCICOND next and R3 consecutive stall count 0. ADR-0017 Option A and RFC-0047 are accepted. The parallel origin/dev W1/W2 history remains unmerged; neither branch CI nor this review proves M22 completion. See [2026-09-28 exit review](../reports/m22-exit-audit-2026-09-28.md).

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
| S5 codegen (L2) | partial | STEP-0315/0322: bounded Core-Wasm seam (nonnegative `Int`/`I64`/`U64`/`Bool` constants, identity pairs, `less_than`, `checked_add`, `checked_sub`) byte-equal to Rust codegen, every other shape typed-refused; STEP-0487 adds the first real script Component emission (981 B, byte-equal to the Rust backend, frozen minimal source shape only); STEP-0488 locates the next blocker: the guest frontend cannot yet lower Script-profile shapes (record projection / `ok(...)` variant / record construct), so the IR-driven emission path has no input yet; STEP-0491 closes the first of the three — a declared-record field projection (`input.stdin` on a `ScriptInput` parameter) lowers to the canonical `project` op byte-equal to the Rust oracle, with typed refusal for the unresolved variants, and STEP-0492 generalizes it from a hardcoded `ScriptInput` base to any declared record; STEP-0493 closes the named-return-type precondition — `returns <declared record>` lowers to the canonical `{"kind":"named",...}` return type byte-equal to the Rust oracle (straight pass-through of the named parameter), and the malformed empty-kind IR that such functions used to emit is now a typed refusal; STEP-0494 closes the record construct itself — `ScriptOutput(stdout: ..., ...)` lowers to the canonical `construct` op with a per-field instruction ledger (each field value first lowered into its own instruction, the construct referencing those ids), byte-equal to the Rust oracle, via a dedicated top-level helper family rather than a rewrite of the deep `scalar_ir` return analysis — so the IR-driven path now has all three Script-profile input shapes; STEP-0495 closes the third one — the built-in `Result` variants `ok(...)` / `error(...)` lower to the canonical `variant` op (payload lowered first, the variant referencing those result ids; `return_type` always in declared `ok`/`error` order regardless of which arm was constructed), byte-equal to the Rust oracle, with the payload type checked against the declared arm so a mismatched payload stays `ERR:E-SH-IR-VARIANT-TYPE` and an unsupported payload shape stays `ERR:E-SH-IR-VARIANT-PAYLOAD` instead of producing a malformed IR; STEP-0495 also registers a measured capacity boundary (the whole-module canonical IR reached 8,608,383 B and no longer fits one Script v0 8 MiB channel, hence the new `--emit-ir-split` driver mode that halves the IR across stdout and stderr); STEP-0496 opens the guest IR-ingestion half — a byte-oriented canonical-IR scanner and a `run` emission-plan extractor in `selfhost/codegen.sico`, the `--emit-ir-plan` driver mode in `selfhost/compiler.sico`, and a plan text that is byte-identical to the one the Rust oracle derives from the same `sico_ir::Module` (oracle-side generator in the test file only), with four typed refusal identities outside the plan subset and a new reproducible probe `tools/probe-m22-ir-plan-frontier.ps1`; STEP-0497 is a CI repair slice (not a codegen step): it closes the M23-session CI red point by aligning the Script package's memory limit with the guest linear memory every script component declares (33,792 pages = 2,214,592,512 B, minimum == maximum), the binding constraint having been the package manifest's 64 MiB rather than the Host grant, keeps the emitted bytes unchanged, and adds four runtime-binary-free invariant guards plus the matching RFC-0029 amendment — the independent CI run 39524849306 for commit 7e5ad3c concluded `success`, so the handed-over red point is green on an independent runner and the slice is closed; STEP-0499 opens the emission half — the guest derives the `run` slot / local-declaration plan (`cgs_run_function` / `cgs_flat_hex` / `cgs_record_hex` / `cgs_local_hex` plus the `--emit-locals-hex` driver mode) and the resulting bytes are byte-identical to the local declaration section the Rust backend actually emits, checked against the oracle's production output (the component `compile_script_program` really produces) rather than a re-implementation of the plan generator, with two typed refusal identities (`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-FLAT`); the plan is carried directly as local-declaration bytes so the next slice can attribute any mismatch to the slot plan or to the instruction emission; the independent CI run 39535461745 (head_sha ee2317f) concluded `success` with all 12 inner steps green, so the slice is closed on an independent runner and not only on this workstation; STEP-0501's independent CI run 39549599204 (head_sha a730ac6) concluded `success` as well, so the measurement slice is closed too; STEP-0501 measured the remaining truth instead of implementing it -- the `run` body the backend actually emits is read from the production component, decoded byte by byte into a reproducible emission table (dispatcher shell / block dispatch / project / const_i64 / construct / variant / Script result-area return / epilogue) and pinned by `oracle_run_body_emission_table_is_pinned`, with an unambiguous cut marker (`i32.const 32` + `call 1`) separating the instruction segment (182 hex) from the result-area return plus epilogue (178 hex); the measurement also settles that the reference Script profile emits **no data section**, so the run body is the component's only IR-dependent part, and its slot table is byte-identical to STEP-0499's local declaration plan (i64 slots 8/13/19); STEP-0502 emits the instruction segment -- the guest derives the run body's instruction segment (dispatcher shell / per-block dispatch heads / block instructions) from the ingested IR, and the bytes are byte-identical to the segment the Rust backend actually emits (182 hex = 91 B), checked against the oracle's production output; `selfhost/codegen.sico` gains 28 new functions (arithmetic / decimal / encoding / layout / slot / record / emission) and `selfhost/compiler.sico` the `--emit-run-instr-hex` driver mode; two typed refusal identities cover the shapes outside the subset (`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-EMIT`); the slice records four measured guest-frontend constraints (the `record` keyword may not name a parameter, `let` has no block scope inside a function, `U64.equal` yields a Bool, and a "not found" sentinel may not double as a total) plus two self-inflicted defects the byte comparison caught (an inverted mismatch flag, a broken "not found" sentinel); the next slices are pre-cut: STEP-0503 (guest emits the Script result-area return, needs five frozen component-level facts: allocator index 1, result area 32 B, payload offset 8, the two Script records' WIT field offsets), STEP-0504 (the derived run body replaces STEP-0487's frozen hex); run-body byte emission is still open | RFC-0011 deterministic backend subset over the frozen corpus, driven by the guest's canonical IR |
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
