# STEP-0138: offline block-game solver — the Sico acceptance oracle port

> - status: complete
> - phase: M14 (RFC-0038 §3.1 primary acceptance application)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); M14 plan §4 application 1

## 1. What was implemented

The offline block-game solver now exists as pure Sico source and matches
the fixed Python oracle byte-for-byte on the frozen corpus:

- **Oracle (fixed comparison, Python only)**:
  `案例项目/俄罗斯方块消除/oracle/oracle.py` — an integer-arithmetic
  mirror of `gamebot/solver.py`'s algorithmic spread: 8x8 bitboard,
  piece placement, simultaneous row/column clearing, target-gem counting,
  depth-first tray search with duplicate-shape skipping, node budget with
  typed limit outcome, integer leaf scoring (cleared-line/target rewards,
  occupancy penalty, largest-empty-region flood fill, row/column
  fill-potential). Weights are the Python solver's weights scaled x100
  and floored to integers; every division is by a constant with both
  sides truncating identically.
- **Corpus**: `oracle/corpus.json` — four frozen fixtures
  (trivial / medium / hard / limit) with expected outputs extracted from
  the oracle. `medium` and `limit` exhaust the 200,000-node budget and
  carry `"limit":true` **with their best-so-far plan** (the merge before
  the limit cut), proving the typed-limit path keeps partial evidence
  (exit gate 4).
- **Sico port**: `tests/end-to-end/block-solver.sico` (~900 lines) — the
  entire algorithm in the application profile: `U64` bitboards with
  STEP-0132 bit operations, general recursion (`visit`/`try_slot`),
  STEP-0130 `while`/`if`/`set` control flow, STEP-0134 checked
  arithmetic exclusively (no wrapping primitive is used anywhere; the
  SWAR popcount deliberately avoids the classic wrapping multiply),
  records-free scalar core (packed score/placed/nodes/limit in one I64,
  unpacked with checked div/mul/sub), and a second deterministic pass
  that reconstructs the winning move sequence by exact-score matching.
  Input is flat JSON over the frozen stdlib (`json.is_valid/has/get`,
  `bytes.slice/utf8_decode`, `text.contains/concat/length`,
  `i64/u64.to_text`); malformed input fails with typed `InvalidInput`.
- **Runner test**: `runner/sico-runner/tests/block_solver.rs` compiles
  the fixture through the real CLI and asserts stdout byte-equality with
  the frozen expected outputs for all four fixtures (raised fuel/timeout
  budgets recorded below).

## 2. Evidence

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test block_solver -- --test-threads=1`: 1/1 green — all four fixtures byte-exact vs the oracle.
- Search equivalence was verified at the node-accounting level: for the
  `limit` fixture both sides report **204,077 enumerated nodes** at the
  budget cut and identical best-so-far score 98561.

## 3. Defects found during porting (all fixed, kept as lessons)

1. `bridge64` (U64→I64 bridge) saturated at 63 and missed 64 — the
   pot4 accumulator legitimately exceeds 63; per-row counts (≤8) are
   bridged instead, and 64 is handled explicitly.
2. The SWAR popcount's classic wrapping multiply is **unsound under
   checked arithmetic** (mulU saturates on the guaranteed overflow);
   replaced with 8 shift-adds whose byte sums provably stay below 2^64.
3. Origin enumeration was off-by-one (`8-h` instead of `9-h`), silently
   dropping the last row/column.
4. The tray parser's `;`/`#`/`.` branches read the board loop's stale
   character cell — piece masks were always empty.
5. The fourth tray piece's end-flush branch was missing, leaving `m3 = 0`
   (empty pieces placed everywhere, corrupting the budget accounting).

## 4. Residuals

- The seeded rollout path of `solver.py` (future sampling with a source
  PRNG) is expressed by STEP-0135's xorshift64 fixture at profile level;
  wiring it into the solver's candidate scoring is future work and is
  not part of the frozen corpus.
- Runner budgets for the corpus: fuel 2×10^11, timeout 600 s (the
  `medium` fixture dominates at ~200k nodes; wall time ~8 s on the
  Windows dev host, unoptimized build).

## 5. Validation

- `cargo fmt`/`clippy -D warnings` clean; root workspace tests green;
  runner workspace serial suite green; `tools/validate-step-0131.ps1`
  green with the extended exit-corpus; `git diff --check` clean.
