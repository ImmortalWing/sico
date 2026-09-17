# M22 Compiler self-host track

> Status: **NO-GO / implementation in progress** (STEP-0200 interim audit corrected the earlier “S1-S5 landed” shorthand: STEP-0178–0199 are bounded probes/seams, not full slice exits); STEP-0201 freezes the complete 215-source corpus and lands the strict ADR-0015 source-bundle decoder; STEP-0202 closes lossless lexer parity on all 215 sources through bounded line segments; STEP-0203/0206 match accepted semantic `ModuleAst` declaration shape and metadata on 99/99 sources but leave expression/statement and refusal AST work open; STEP-0204/0205 close S1 formatter output/idempotence/refusal on all 215 sources; STEP-0207 freezes the reproducible Script build oracle at 37 accept/178 refuse; STEP-0208 links the lossless lexer and declaration parser into the actual modular compiler Component on its own sources; S2 checker, expression/statement lowering, guest artifact parity, A=B=C, M7 package and budgets remain open

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
| 5 | Bootstrap architecture ADR accepted (differential harness, artifact packaging through the M7 trust chain, budget re-measurement policy) | **satisfied** — ADR-0015 accepted by owner directive "完成M22，规划M23-24" on 2026-09-16; STEP-0196 |
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
  (`format(format(x)) == format(x)`). **Complete:** STEP-0204 closes all
  99 Rust-accepted outputs and their idempotence through the real runner;
  STEP-0205 closes the 116-source typed lexical-refusal gate without path
  or manifest allowlists.
- **S2 — L1 checker subset**: Sico-written checker producing the declared
  diagnostic subset (outline + the E1xxx identities the corpus freezes);
  differential on the same corpus. The Rust checker remains the oracle;
  Sico v0 must match refusals exactly or produce a declared, disjoint
  subset (typed, no silent divergence).
- **S3 — L2 lexer + parser**: token/AST as declared data shapes
  (struct-of-arrays is an accepted transition shape while List[record]
  stays outside the executable set); equivalence proxy = canonical
  formatter output round-trip (S1 consumes it), not raw spans.
  STEP-0208 additionally proves the lexer/declaration parser modules are
  linked and called by the compiler entry on the compiler's own sources;
  expression/statement and refusal AST remain open.
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

All M22 evidence is `internal-fixture`. Differential corpus runs are
runtime evidence when executed through a real runner; corpus presence
alone stays `contract-verified`. No external-pilot or production claims
arise from self-hosting.

STEP-0207 fixes the L2 native oracle population at 37 Script v0 Component
acceptances and 178 no-artifact refusals. Guest parity must use these exact
entries and complete artifact bytes; formatter/checker acceptance is not a
substitute for build acceptance.

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

## 9. Step allocation

Completed implementation slices are recorded above, but future work is
allocated only when its entry gate and bounded exit test are ready: **no STEP numbers reserved**
for the remaining checker, AST, IR, codegen, bootstrap,
package, budget or audit work.
