# RFC-0038: M14 application profile and support matrix v0

> - status: draft (STEP-0129)
> - date: 2026-09-05
> - phase: M14 kickoff (M14 plan §7 requires this RFC before code changes)

## Summary

Freeze the M14 application profile: the machine-readable check/build/run
support matrix for every accepted source construct, the classification of
each as `executable` / `profile-restricted` / `proposed`, and the frozen
exit corpus. The defining M14 gate is that the application profile has
**no undeclared check/build/run mismatch**; this RFC is the declaration.

## 1. Measured inventory (2026-09-05, sico 0.0.2-dev)

Evidence: compiler sources (`sico-ir` Operation/lowering tables, 126
typed-refusal sites in `sico-codegen-wasm`), the 36-intrinsic stdlib
table, and live probes run through `sico check/build/run`
(reproduction commands in STEP-0129 §3).

### 1.1 Executable today (check = build = run green)

| construct | notes |
|---|---|
| records, enums, variant construction | user types cross function boundaries |
| functions + non-recursive calls | `call target after verification` path |
| `match` where **every arm is a `return`** | the only control flow; arms may not fall through to code after the match (probe: `unsupported non-return match arm`) |
| `Result` / `Option` | including checked-arithmetic `Result[I64, NumericError]` unwrapping via match |
| `List[T]` with `sico.list.*` | get/append/length |
| fixed-width arithmetic | `I64/U64.checked_add/checked_sub/equal/less_than` lower to `CheckedAdd/CheckedSub/EqualFixed/LessFixed` |
| 36 stdlib intrinsics | text/bytes/json/fs/http@0.1.0/streams |
| HTTP `http@0.1.0` buffered; `http@0.2.0` | 0.1.0 source-emitted; 0.2.0 guest-visible at Component level only (source emission = M14 work) |
| Task scopes (M11 sequential-v1) | parallelism-1; first-class Task/Future/Stream values refused |

### 1.2 Check-accepted, build-refused (the M14 gap list)

| gap | evidence |
|---|---|
| recursion | self-call passes `sico check`; useless today because arms must return and deep recursion needs loops/mutable state anyway — remains a typed refusal class to re-verify after the profile lands |
| non-return `match` arms | probe: `unsupported non-return match arm` — blocks mid-function branching, the single biggest algorithmic blocker |
| unbounded `Int` | `unbounded Int parameter` at the ABI boundary |
| non-scalar component values | `non-scalar Component value` |
| first-class `Task`/`Future`/`Stream` | `async_unsupported` refusals |

### 1.3 Not in the grammar (proposed / absent)

`while`/`for` loops, `if` statements (match-on-Bool is the only spelling),
`break`/`continue`, mutable local reassignment, `Map`/`Set` types,
closures. None may be claimed by user documentation.

## 2. Application profile (frozen by this RFC)

The **application-ready profile** is the set of constructs that must be
executable (check = build = run, with typed limit+1 outcomes) by the M14
exit audit:

1. general iteration: bounded loops (`while`/`for`) with explicit
   termination and cancellation points, or evidence-backed rejection with
   an equivalent iteration model;
2. tail/general recursion with a runtime stack/fuel bound and a typed
   exhaustion outcome;
3. mid-function control flow: match arms that continue (not only return),
   `if` statements, and `break`/`continue` inside loops;
4. dynamic `List`/`Map`/`Set` traversal and construction with
   deterministic iteration order (insertion order for Map/Set keys);
5. fixed-width arithmetic (checked `+ - * /`, comparisons, bit
   and/or/xor/shift) suitable for bitboards, plus `Int` staying
   check-only unless a later RFC freezes its runtime form;
6. user records/enums/Result/Option freely across function and collection
   boundaries;
7. deterministic pseudo-randomness as a source-level helper (seeded PRNG
   in Sico source — never ambient randomness);
8. source-level emission of `http@0.2.0` imports beside `0.1.0`
   compatibility.

Everything outside this profile stays exactly as classified in §1 —
executable, restricted, or proposed — with no silent widening.

## 3. Exit corpus (frozen by this RFC)

### 3.1 Primary oracle: offline block-game solver (unchanged, corpus strengthened)

`案例项目/俄罗斯方块消除` remains the representative acceptance
application. Evaluated alternatives (JSON parser/serializer, mini
interpreter, A*/Dijkstra pathfinding, Huffman compression, regex engine)
all cover a strict subset of the solver's algorithmic spread
(bounded search, flood-fill graph traversal, candidate enumeration,
weighted heuristic scoring, seeded rollout) or lack an existing byte-exact
oracle; the solver already ships a Python oracle with tests
(`gamebot/` + `tests/`), and M16/M17/M18 build on this same case, so
replacing it would break the acceptance chain. "更全面" is achieved by
strengthening the frozen corpus, not by switching projects:

- **fixture boards**: a frozen set of JSON board configurations
  (trivial / medium / hard) with expected best-plan scores byte-comparable
  against the Python oracle (oracle corpus extracted from `tests/` and
  committed under `案例项目/俄罗斯方块消除/oracle/`);
- **typed boundary**: solver consumes JSON input and emits JSON plan
  output through the Component boundary (plan §4 already allows this) —
  exercises serialization plus algorithms;
- **deterministic rollout**: the seeded future-sampling path
  (`random_seed`/Laplace prior in `solver.py`) re-implemented in Sico
  source with a source-level PRNG, proving reproducibility without
  ambient randomness;
- **limit+1**: `max_nodes` exhaustion must produce a typed limit outcome
  (exit gate 4) rather than a trap;
- **bitboard variant**: one fixture set solved with bit-packed boards to
  exercise the bit-operation slice of the profile.

### 3.2 Companion applications (unchanged from M14 plan §4)

- streaming transform (bounded memory independent of input size,
  cancellation, typed partial failure);
- capability-backed state machine (explicit external effect, revision
  guard, deterministic recovery).

## 4. Sequencing

Each post-RFC STEP closes one measured §1.2/§1.3 gap in the order the
acceptance corpus demands (loops/branching first, collections, bit ops,
PRNG, http@0.2.0 emission), rerunning the cumulative oracle corpus and
the M0–M13 regression each time. The matrix in §1 becomes a
machine-readable fixture validated on every change.
