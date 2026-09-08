# STEP-0131 defect forensics and open hard problems (handoff for model upgrade)

> - date: 2026-09-06
> - purpose: forensic record of defects that took repeated attempts to
>   localize during STEP-0131, plus the currently open hard problems. All
>   defects listed under §1 are FIXED and covered by tests; §2 lists what
>   remains genuinely open. A stronger reviewer should treat §1 as an audit
>   target (verify the fixes, try to break them) and §2 as an ownership
>   handoff.

## 1. Fixed defects (STEP-0131)

### 1.1 `text.split_words` inverted close condition (pre-existing)

- **Symptom**: every element of a `split_words` list had length 0 through
  `list.get`/`text.join`; word content printed empty; map keys degenerated
  to pointer-like distinct garbage. `list.length` (count) was correct, so
  every pre-existing test/fixture passed — only element *contents* were
  broken, and no fixture had ever read them.
- **Why it was hard**: five candidate layers (split fill, list.get,
  Project/fields maps, join, text.length) were each statically traced
  end-to-end against the emitted Wasm disassembly and all looked correct.
  The defect was pre-existing, so diffing against HEAD found nothing. It
  was localized only by stashing all changes and reproducing on the
  baseline binary — proving it predated STEP-0131 — after which a
  byte-level re-read of the fill pass found it.
- **Root cause**: in `emit_split_words` (sico-codegen-wasm stdlib.rs), the
  fill-pass close condition was `LocalGet(7) I32Eqz` — i.e. "close when
  in_token == 0" instead of "close when in_token != 0". The in_token=1
  branch therefore never stored anything, and the final-token path was
  also suppressed (`Eqz` false). The table was allocated (count was
  correct) but never filled: a fresh arena page reads as zeros, so every
  element was `(ptr=0, len=0)`.
- **Fix**: drop the `Eqz` so the close branch fires when `in_token != 0`.
- **Verification**: `map-set-frequency` fixture (dynamic keys), plus
  regression: word-count still 3 for "alpha beta beta"; join total
  non-zero. Lesson: **element contents of stdlib collections had no e2e
  coverage at all** — the frozen corpus only exercised counts. A stronger
  model should add differential tests (Sico text/list helpers vs a Python
  oracle) rather than trust static tracing.

### 1.2 Zero-size arena block aliasing (new code, STEP-0131)

- **Symptom**: `put` into an empty map, followed by any other arena
  allocation (e.g. `text.concat`), made later `has`/`get` miss. Removing
  the intermediate allocation made it hit.
- **Why it was hard**: every collection helper was re-verified byte-by-byte
  and looked correct; the failure depended on *allocation order*, not on
  the collection code. Asymmetric probes (put dynamic → has literal vs
  put literal → has dynamic) finally isolated it.
- **Root cause**: copy-on-write tables allocated `count * entry_width`,
  which is **0 bytes for an empty map**. `alloc(0)` returns the current
  heap pointer without advancing it; the entry written "into" the table
  was then overwritten by the next allocation landing at the same address.
- **Fix**: mutation helpers always allocate `(count + 1) * entry_width`.
- **Lesson**: a bump allocator plus value semantics means **zero-size
  allocations are dangling stores**. Any new aggregate producer must
  allocate headroom or prove emptiness. Worth an audit of every
  `alloc(count * w)` site.

### 1.3 Key comparison by pointer identity

- **Symptom**: `put("k", …)` then `has(k2)` where `k2` had equal *content*
  but a different address missed; `has("k")` (same static address) hit.
- **Root cause**: the first helper draft compared `(ptr, len)` pairs
  structurally. Static literals dedupe into one data-segment address, so
  pointer equality silently worked for literal tests and broke only for
  dynamic Text.
- **Fix**: pair keys compare by length then byte content (a small Wasm
  byte loop with explicit verdict locals).
- **Lesson**: value semantics require **content** equality for every
  aggregate key. Any future key type (records, fixed arrays) must define
  equality structurally, not by layout.

### 1.4 Verifier's block-scoped value model vs cross-block bindings

- **Symptom**: IR verifier raised `UndefinedValue` for code the lowering
  considered obviously correct (values produced in one block, read in a
  later block after a loop/match join).
- **Root cause**: a deliberate model conflict — IR values are block-scoped
  (wasm locals are planned per value, per block), and the verifier's
  `available` map resets per block. STEP-0130 had already solved this for
  reassigned variables via mutable local cells, but only for `set`
  targets; plain `let` bindings and match payload bindings still produced
  block-local values.
- **Fix**: in general-CFG lowering, every `let` lowers to a mutable local
  cell (straight-line shapes keep plain bindings so frozen snapshots stay
  byte-identical), and match payload bindings spill into
  compiler-generated `#match<name>` cells. The verifier's model was NOT
  relaxed — the invariant "values never cross blocks" is load-bearing for
  codegen.
- **Lesson**: when the verifier rejects compiler output, first ask whether
  the *lowering* violates a deliberate invariant; relaxing the verifier to
  make a broken lowering pass would have produced miscompiled code.

### 1.5 Wasm ABI/layout traps in the emitted helpers (all new code)

Each of these produced validator or runtime errors that looked like the
previous bug and cost a build cycle each:

- `Text` keys occupy **two** i32 params (`ptr, len`), not one — helper
  signatures must be derived from an `element_param_count` table, never
  hand-counted.
- Wasm `If` arms must be stack-neutral under `BlockType::Empty`: a
  count-selection `If/Else` yielding a value needs a scratch local
  (store in both arms, load after).
- `br_if` depth arithmetic: labels are per enclosing block/loop/if; the
  byte-compare loop's exhaustion branch and the verdict branch needed
  different depths after the verdict moved into a local. A wrong depth can
  still validate (br is stack-polymorphic) and only fails at runtime.
- Local declarations are grouped by type in index order: `(1, I32),
  (1, I64), (1, I32)` — a scratch local shared by i32 code paths must not
  be declared in an i64 group.
- `Result[Text|Bytes, NumericError]` needed a dedicated 4-slot local form
  (tag, ptr, len, error tag) matching what `list.get`/`map.get` emission
  writes; the packed 2-slot form stays special-cased for
  `Result[I64|U64, NumericError]`.

### 1.6 Known flake class (not fixed, documented)

`runner_cli_observes_real_windows_console_control`,
`runner_watch_accepts_generation_bound_client_cancellation`,
`debug_pause_terminate_leaves_no_stranded_tasks_or_workers`,
`dap_hundred_sequential_sessions_have_bounded_rss_handles_and_no_poisoning`
fail intermittently under full parallel load and pass serially / in
isolation (metrics flat: 98→98 handles). Same class as the STEP-0125
watch-cancel race. Serial run is the documented workaround
(`cargo test --test runner -- --test-threads=1`).

## 2. Open hard problems (ownership handoff)

1. **Recursion with typed exhaustion** (RFC-0038 profile item 2). Needs a
   fuel/stack budget in the runner, a typed limit outcome, and a lowering
   decision (direct calls vs explicit trampoline). Deep recursion currently
   remains a typed build refusal; `check` accepts it — a declared gap.
2. **`sico check` is silent on unknown callees.** Unknown names pass check
   and refuse at build (`unsupported call target`). For the profile gate
   ("no undeclared check/build mismatch") these rows are *declared* in the
   matrix, but a stricter check-time diagnostic (unresolved-name E-code)
   would tighten the gate. Requires a semantic-case corpus change.
3. **First-class `Task`/`Future`/`Stream` values** and **parallelism > 1**
   (M9/M11 deliberately deferred). Needs scheduler design beyond
   sequential-v1; all refusals are typed and declared.
4. **Unbounded `Int` at the ABI boundary** — check-only; needs an RFC
   freezing its runtime form (bigint or refusal).
5. **Differential/property testing for stdlib helpers.** Defect 1.1
   survived because element contents had no coverage. Minimum bar:
   split/join/append/get/length and map/set ops vs a Python oracle over a
   frozen corpus; stretch: coverage-guided fuzz of the text helpers.
   (Partially addressed by STEP-0133: traversal-helper element contents now
   have guest-asserted e2e coverage; the Python-oracle differential corpus
   remains open.)
6. **Arena allocator audit** (lesson 1.2): enumerate every
   `alloc(count * w)` and prove non-zero headroom or non-escape.
   **Closed by STEP-0133** (`docs/steps/STEP-0133-map-keys-stride-and-arena-audit.md`):
   every `$alloc` site enumerated with verdicts; the audit exposed and fixed
   a real defect — `map.keys` returned the 16-byte-stride map entries table
   as an 8-byte-stride `List[Text]` table.
7. **Parallel-load flake class** (1.6): root-cause the console-control and
   DAP RSS tests to be parallel-safe, or encode serial execution as a
   first-class harness rule with a marker.

## 3. Reproduction pointers

- Defects fixed in: `crates/sico-codegen-wasm/src/stdlib.rs`
  (split close condition, collection helpers), `crates/sico-ir/src/lower.rs`
  (let/payload cells), `crates/sico-codegen-wasm/src/canonical.rs`
  (flat layouts), `crates/sico-codegen-wasm/src/lib.rs` (helper registry).
- e2e evidence: `tests/end-to-end/map-set-frequency.sico` +
  `runner/sico-runner/tests/map_set.rs`.
- Baseline comparison technique (stash + rebuild + rerun) was the single
  most effective localization tool for 1.1; recommend it before any deep
  static trace of pre-existing helpers.
