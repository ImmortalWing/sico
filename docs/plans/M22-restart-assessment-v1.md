# M22 restart assessment — keep / refactor / re-author, by layer

> Status: drafted 2026-09-24 as the R0 decision input (M22–M26 route replan v1
> §3, STEP-0264); planning-only — it inventories what exists, judges what a
> restart would throw away, and defines what "re-implement M22" means under
> each R0 option; it freezes no contract and reserves no STEP numbers.

## 1. Question and method

Owner question (2026-09-24): "查看M22已有步骤，是不是需要回退重新开始。
规划好之后重新实现M22." Method: measured inventory of every selfhost layer
(line counts from `wc -l selfhost/*.sico`, 2026-09-24 working tree), the
review's structural findings (`docs/reports/m22-quality-review-2026-09-20.md`),
the post-review STEP trajectory (0260–0262 + uncommitted WIP), and a
per-layer keep/refactor/re-author verdict. No code was modified for this
assessment.

## 2. Asset inventory (measured)

| Layer | Files (lines) | What it is | Verdict |
|---|---|---|---|
| L1 formatter | `formatter.sico` (827) | S1: byte-exact + idempotent on the frozen 215-source corpus; runs on its own sources through the real runner | **Keep.** Closed slice, real differential evidence |
| L1 checker subset | `compiler_lexer.sico` (462), `compiler_parser.sico` (686), `compiler_semantics.sico` (749), `checker.sico` (42) | S2: 215/215 frozen outcomes (116 lexical / 34 identity / 65 accepted / 0 unsupported) | **Keep + harden** (fingerprint rules P2, sha gate) |
| Corpus + harness | `corpus-v0.json`, `script-build-corpus-v0.json`, `tools/validate-step-0245/0261.ps1`, `tools/fixtures/step-0NNN/`, STEP-0254 prepared-compiler harness (66.99 s serial suite) | The oracle, the evidence chain, the iteration loop | **Keep unconditionally.** Restarting would re-buy the most expensive infrastructure |
| S3/S4 lowering machines | `parser.sico` (12,739 — 75% of selfhost) | Lossless token walk + declaration parse + the general lowering machines (straight-line environment, guard chains, general-while with packed frame stack, gw last-resort routing) | **Keep the machines; re-author decision pending R0** (§4) |
| Integrated compiler | `compiler.sico` (430) + `parser_driver.sico` (49) | The Component that links the modular chain; S5 codegen seam | **Keep + widen** (S5 is a declared queue item, not a defect) |
| Legacy S3 lexer/parser | `lexer.sico` (117), `tokens.sico` (459), `declaration_parser.sico` (371) | Pre-integration era units; review P2 flags near-duplication with `compiler_lexer`/`compiler_parser` | **Consolidate** into the modular chain; do not maintain two lexers |
| Rust oracle + differential tests | `runner/sico-runner/tests/selfhost_*.rs` | The other half of every differential | **Keep** — the Rust oracle is never retired (M22 non-goals) |

Working-tree state (preserved per AGENTS.md §1, not committed by this
assessment): `parser.sico` +228/−34 extends `parameters_ir` toward the
`Map[Text,U64]` parameter surface (`nearest_match` frontier,
`E-SH-IR-UNRESOLVED`) with bracket-depth-aware comma counting, plus a
`dump_nm_region_ir` oracle probe in `selfhost_compiler.rs`. This is coherent
next-frontier work in the STEP-0262 line, not scratch; before any restart
wave lands it must be either landed as its own STEP or its value absorbed
into the re-authored source. It is left untouched.

## 3. Verdict: no git-level rollback; a three-layer restart

**Wholesale rollback (abandon STEP-0178→0262 and restart M22 from a clean
tree) is rejected on the evidence:**

1. The closed slices are real. S1 byte-exactness + idempotence and the S2
   215/215 closure are differential facts about the *corpus*, independent of
   how the S3/S4 source layer is organized. Re-deriving them would repeat
   paid work and add re-freeze risk.
2. The infrastructure is the expensive half. Frozen corpora, validators,
   fixtures, the 12× faster prepared-compiler harness, and the Rust oracle
   suite are exactly what any restart — conservative or radical — needs on
   day one.
3. The trajectory has already turned. The review's "shape enumeration
   machine" verdict was measured at `acb5113` (2026-09-20). Since then
   STEP-0260 (general-while convergence), 0261 (packed while-frame stack)
   and 0262 (gw last-resort routing) are *general machines*, not per-shape
   templates — formatter coverage moved 16 → 17 → 22 of 30 functions in
   three STEPs. Restarting now would discard precisely the generalization
   the review demanded.

**What "重新实现 M22" therefore means: re-author the S3/S4 source layer
under an R0 architecture decision, keep everything else, and pay the
consolidation debts.** Two layers genuinely deserve rewrite, and both are
gated on R0 rather than on taste:

- the *data model* of the lowering pipeline (SOA parallel arrays + JSON
  string payloads inside `parser.sico`), and
- the *authored shape* of the selfhost sources themselves (the ceremony tax
  recorded in M22 plan §8 that the 12.7k-line file pays per line).

## 4. What re-implementation means under each R0 option

### Option A — records RFC (`List[record]` becomes executable)

A versioned record/struct type (field access, record literals, typed
record lists) added to the language under the full RFC-0033 gate: explicit
desugar, source-map identity, formatter idempotence, typed diagnostics,
measured consumer. The selfhost sources are then **re-authored** on records
— the parallel-array simulation in `parser.sico` and the 18 parallel
`List[Text]` columns in `compiler_semantics.sico` collapse into typed
shapes. STEP-0270's later EC-4 census measured 360 direct list-ceremony
lines in 16,931 selfhost lines (2.13% ceiling proxy), so a large line-count
reduction is not evidenced. The *lowering machines' logic* is
language-shape-independent: they are re-hosted against the record-based AST,
re-verified byte-exact against the Rust oracle, not re-discovered.

- Cost: the RFC itself is a major language-surface change (parser,
  semantics, IR, verifier, codegen, formatter, diagnostics — the whole
  M3/M14 machinery). This is months-scale, and it is the same feature the
  application profile and AI-generated code want (M14/M23 ergonomics).
- Reuse: corpus, harness, validators, Rust oracle, S1/S2 slices, machine
  logic, S5 seam. Re-convergence order: re-authored sources → canary
  (formatter byte-exact again) → selfhost source set → S5 → S6.
- Trigger condition: records judged the better investment *now* (see §6).

### Option B — SOA freeze ADR

Struct-of-arrays stays; an ADR freezes it as the permanent self-host
architecture with a written spec (column invariants, append-only
discipline, the consolidation debts of §5 as mandatory follow-ups). The
S3/S4 source layer is then **refactored in place**, not re-authored:
extract the general machines from `parser.sico` into modules with named
column contracts, consolidate the legacy lexer/parser units, and keep
lowering under the canary gate (replan §3 R2/R3).

- Cost: days-to-weeks scale; no language change; re-freeze of corpus
  contents is minimal (byte-exactness is against the Rust oracle, and the
  Sico sources are the *input*, not the frozen bytes — only re-baselined
  canary fixtures move).
- Ceiling: the ceremony tax stays until batch 3 lands (M23), so selfhost
  sources keep paying verbosity per line; convergence is slower per
  function than under Option A's re-authored sources would be.

**Assessment recommendation:** Option B is the default-under-uncertainty;
Option A becomes preferred if either (a) R1 budget curves show the SOA
codebase's fuel/line cost superlinear enough to threaten the S6 closure
budget, or (b) the owner judges records valuable for M14/M23 independent of
M22 (they are — the same §8 friction list says so). The decision is the
R0 document's; this assessment supplies the cost table above.

## 5. Consolidation debts to pay under either option

1. **One lexer, one declaration parser.** Retire `lexer.sico` /
   `tokens.sico` / `declaration_parser.sico` in favor of the integrated
   `compiler_lexer` / `compiler_parser` chain (review P2 duplication; the
   `<`-token defect branch in `tokens.sico:431-435` dies with it).
2. **Fingerprint rules.** Replace keyword-fingerprint semantic checks in
   `compiler_semantics.sico` (E7001/E7002, `Stream` literals) with
   structural checks, or record them as declared-subset limitations with a
   rename-safety corpus (review P2).
3. **`selfhost_checker.rs` sha gate.** Validate `source_sha256` like
   `bootstrap_bundle.rs` does (review P0-5 follow-up).
4. **Indentation-as-semantics proxy.** The `same(raw, line)` declaration/
   statement discrimination must gain a 0-indent corpus case or a declared
   refusal (review P2).
5. **Working-tree WIP.** Land or absorb the Map-parameter WIP before the
   first restart wave touches `parser.sico` (§2).

## 6. Sequenced restart plan (the "重新实现" order)

1. **R1 instrumentation first** (replan §3): canary coverage reporting in
   the STEP-0261-style validator; budget probe at 1k/4k/8k lines measuring
   *consumed* fuel, stack high-water, wall time; compilation-unit context
   draft. These numbers are the Option-A-vs-B evidence and are
   decision-independent.
2. **R0 decision** (owner, with §4's cost table + R1 numbers): records RFC
   or SOA freeze ADR. One document, bounded.
3. **W1 — consolidation** (§5 items 1–4): mechanical, corpus-guarded, no
   lowering semantics change; suite + canary must stay green.
4. **W2 — source-layer re-authoring per R0**: Option A → re-author on
   records, re-host machines, re-converge canary; Option B → machine
   extraction + module split under the canary gate. Either way: S1/S2
   slices and Rust oracle untouched; every wave re-pins the canary.
5. **W3 — S5 widening → S6 closure → S7 audit** per ADR-0015 and the M22
   plan exit gates, unchanged.

Stop-loss (replan R3) is armed from W2 S3/S4 onward: 4 consecutive
implementation STEPs with zero current-phase canary growth and an unmoved
refusal frontier declare S6 not demonstrated on the current surface. Stalled
implementation STEPs count; documentation/measurement-only STEPs do not.
After the formatter reaches 30/30, the remaining frozen selfhost source-set
differential is the canary. The accepted Option A decision remains recorded;
the stop-loss triggers M23 Route B, not an unreviewed architecture switch.

## 7. Entry gate / Exit gates / Non-goals

**Entry gate:** STEP-0264 replan accepted; this assessment registered
(STEP-0265); working-tree WIP preserved and dispositioned (§2).

**Exit gates:** every layer in §2 carries an explicit verdict; R0 decision
document exists before any W2 source change; W1 consolidation lands with
suite + canary green; the restart plan in §6 is the binding execution
order.

**Non-goals:** no git history rewrite or commit reversion; no retirement of
the Rust oracle; no language-surface change by this document (Option A's
RFC is a separate contract); no STEP numbers reserved; no performance
claim; S6's A=B=C contract unchanged.

## 8. Risks

- **Sunk-cost defense.** "Keep everything" can itself be the bias. The
  mitigation is the stop-loss: four stalled W2 S3/S4 implementation STEPs
  declare Route B, while the accepted Option A decision remains in the
  register — the assessment defends *assets*, not *trajectory*.
- **R1 measurement gaming.** Fuel numbers depend on input choice; the
  probe pins corpus inputs (1k/4k/8k lines from the frozen set), not
  ad-hoc programs.
- **Option A scope bleed.** Records are an application-profile feature;
  the RFC must stay minimal (records + typed record lists only) or it
  becomes an M14 rewrite wearing an M22 label.
