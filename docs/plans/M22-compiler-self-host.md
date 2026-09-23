# M22 Compiler self-host track

> Status: entry satisfied (conditions 3/4 closed by STEP-0175 on 2026-09-14; condition 5 closed by accepted ADR-0015 / STEP-0198 on 2026-09-17); executable audit through STEP-0261: S1 and the declared S2 identity/outline subset complete; S3/S4/S5 partial; S6 not entered. ADR-0016 freezes SOA and compilation-unit roles; STEP-0246 unifies parser/compiler lowering without exact-source IR fallback; STEP-0247 removes quadratic List-builder exhaustion; STEP-0248 adds typed user-call guard chains, literal arguments and Text returns; STEP-0249 repairs the 256-local build failure, makes the validators host-runnable and re-pins the canary; STEP-0250 opens `Bytes` parameters; STEP-0251 lands `general_while_function_ir` with the canonical lazy-block discipline; STEP-0252 lowers nested `ident_continue` conditions; STEP-0253 adds parent-linked append-only if frames; STEP-0254 reuses the prepared compiler inside the differential harness; STEP-0256 lands `item`/`append_pair`; STEP-0260 lands `line_tokens`; STEP-0261 replaces the single-slot while frame with a packed frame stack (nested/sequential whiles byte-exact through `source_has_lex_error`), adds `sico.list.length`/`sico.text.split_lines` parameter surfaces and nested user-call arguments; STEP-0262 routes while-less statement-bearing functions through the general machine as a dispatcher last resort (bare-cell conditions, entry-region if/return, open-order deferred empty-else blocks, `sico.text.concat` intrinsic RHS with nested concat arguments) and lands `call_left`/`format_code`/`repeat_indent`. The formatter prefix is byte-exact through `repeat_indent` (twenty-two functions); the typed frontier is `nearest_match`'s `Map[Text,U64]` parameter surface (`ERR:E-SH-IR-UNRESOLVED`). Full rendered diagnostic parity is not claimed; no STEP numbers reserved beyond the completed sequence.

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

### 3.1 Slice status (measured 2026-09-23, through STEP-0262)

| Slice | Status | Closed evidence | Remaining bounded work |
|---|---|---|---|
| S0 prereqs | complete | STEP-0174 / STEP-0175 | — |
| S1 formatter (L1) | complete | frozen corpus byte-exact + idempotent; `formatter.sico` runs on its own sources through the real runner | — |
| S2 checker subset (L1) | complete for the declared subset | frozen diagnostic partition 116 lexical / 34 identity / 65 accepted / 0 unsupported | widening the subset requires its own declared-contract update (no silent growth) |
| S3 lexer + parser (L2) | partial | `lexer.sico` / `tokens.sico` / `declaration_parser.sico` differentials green; `parser.sico` exercised through driver paths (recursive return-expression trees; verifier-accepted scalar IR + noncanonical refusals) | full declared-shape coverage of `parser.sico` (12,202 lines, the largest selfhost unit) on the frozen corpus |
| S4 semantics + lowering (L2) | partial | formatter prefix byte-exact through `repeat_indent` (22 of 30 functions; nested whiles via the STEP-0261 frame stack, while-less statement functions via the STEP-0262 last-resort routing); typed refusal-first discipline with re-pinned canary each step | the 8 remaining formatter functions (`nearest_match` … `main`), then the remaining selfhost sources lower byte-exactly |
| S5 codegen (L2) | partial | bounded Core-Wasm seam: nonnegative `Int` constant emission byte-equal to Rust codegen; every other shape typed-refused (`unsupported codegen source shape`) | RFC-0011 deterministic backend subset over the frozen corpus |
| S6 bootstrap closure | not entered | — | ADR-0015 contract: `A == B == C`, `.sapp` via the M7 trust chain, runner-executed self-compile, budget re-measurement |
| S7 exit audit | not entered | — | evidence pack + explicit GO/NO-GO |

### 3.2 Execution queue (planned ordering; no STEP numbers reserved)

The queue is the planned convergence order from the current typed frontier.
Each item lands under the standing discipline: observe the typed refusal,
lower to byte-exactness, re-pin the canary at the next typed refusal, keep
the refusal-recovery and local-bounds regressions green, and never emit a
partial lowering that is unverifiable or semantically different.

1. **`item` region** — extend self-host match lowering to a match subject
   that consumes an intrinsic result (`sico.list.get` on a `List[Text]`
   parameter indexed by `U64`); today this is typed-refused as
   `E-SH-IR-CALL-TARGET`. Exit test: formatter prefix byte-exact through
   `item`; canary re-pinned at the next declared refusal.
2. **Formatter tail in source order** — `append_pair`, `line_tokens`,
   `source_has_lex_error`, `no_space_before`/`no_space_after`, `call_left`,
   `format_code`, `repeat_indent`, the `Map[Text,U64]` region
   (`nearest_match`/`set_nearest_match`/`match_arm_levels`),
   `close_code`/`direct_close`/`opener_close`, `normalize_source`, `main`.
   Each region lands as its own byte-exact prefix differential. Exit test:
   the complete `formatter.sico` lowers byte-exactly against the Rust
   oracle with the full suite green.
3. **Remaining selfhost sources** — lower `checker.sico`, `lexer.sico`,
   `tokens.sico`, `declaration_parser.sico`, `parser.sico`,
   `compiler*.sico` corpora byte-exactly (the S3/S4 convergence completes
   together; `parser.sico` is the bounded largest unit). Exit test: every
   selfhost source's frozen corpus differential is byte-exact and the
   `E-SH-IR-*` refusal set shrinks only by declared regions.
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

All M22 evidence is `internal-fixture`. Differential corpus runs are
runtime evidence when executed through a real runner; corpus presence
alone stays `contract-verified`. No external-pilot or production claims
arise from self-hosting.

Validation-host note (recorded with STEP-0254): this evidence machine's
runner artifacts build under the pinned `1.98.0-x86_64-pc-windows-msvc`
toolchain; the documented GNU flow additionally needs a real gcc for
`ring` (the rustup self-contained gcc is linker-only) and dlltool from
`target/tooling/msys2-binutils`, which this host no longer has.
Validators must pin the toolchain matching the local cache until the GNU
path is re-provisioned; this is an environment repair, not a product
support change.

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
  msys2-binutils plus a C compiler that the evidence host lost after
  2026-09-20; until re-provisioned, every M22 validator must pin the
  toolchain that matches the cached artifacts (see §5), or a full rebuild
  fails at `ring`/`windows-sys` and masks the actual differential result.

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
