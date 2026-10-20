# M23 Language v1 batch 3 — expression ergonomics

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved; contract drafting and kickoff measurement may proceed before either M22 exit; implementation uses Route A after M22 S7 GO or Route B after an M22 R3 stop-loss declaration under the current W2 S3/S4 canary (STEP-0264, clarified by the M14–M26 audit)

## 1. Objective

Close the language-surface friction that RFC-0044/0045/0046 measured but
did not remove, so that ordinary application logic (and the M22
self-host frontend that consumes the same surface) stops paying the
recorded ceremony tax. Batch 3 is per-item, RFC-0033-gated: no item
lands without its own accepted RFC carrying the four-part evidence
(explicit desugar, source-map identity, formatter idempotence, typed
diagnostics).

Items, exactly as recorded in the M22 plan §8:

1. **Infix comparison and logical operators.** The only infix operator
   is `==` and only in `if` conditions (RFC-0044). `<`, `<=`, `>`,
   `>=`, `!=`, `&&`, `||`, `!` do not exist; comparison is
   `I64.less_than(...)` match ceremony, conjunction is nested `if`s.
   The smallest closed operator set and its desugar shapes must be
   decided by the RFC, not grown ad hoc. Measured consumer: the
   `selfhost/*.sico` sources themselves (countable ceremony per line of
   real compiler code).
2. **Expression-position conditions.** `match`/`if` freeze the
   all-return statement shape (STEP-0083): `let x = match …` and
   `let x = if …` do not exist. This is the single largest structural
   verbosity tax recorded; the RFC decides the smallest let-bound form.
3. **Bare-literal re-measurement (policy item, not a grammar
   commitment).** RFC-0046 D3 stands until the recorded re-measurement
   lands: re-weigh bare-literal ceremony on a real generated-code
   corpus (AI output reviewed per the STEP-0175-era DX audit method),
   not on recollection. The RFC either records "decision stands" with
   the new measurement, or proposes the smallest typed-literal change
   with full RFC-0033 evidence.
4. **`text.chars`** (queued by RFC-0046 §2): decide with a consumer
   measurement whether the dedicated intrinsic beats the composed
   `text.length`/`char_at` loop; land only with the same evidence bar.

## 2. Entry gate

- **Dual-exit M22 gate (STEP-0264, 2026-09-24; supersedes the single
  "waits for M22 closure" condition).** Implementation may start under
  either route:
  - **Route A — M22 S7 exit audit GO.** Batch 3 implements on a proven
    closure; the self-host re-baseline decision (exit gate 5) follows.
  - **Route B — M22 R3 stop-loss declared** (M22 plan R-phase: 4
    consecutive W2 S3/S4 implementation STEPs with zero current-phase
    canary growth and an unmoved refusal frontier; see
    [`M22–M26 route replan v1`](./M22-M26-route-replan-v1.md) §3–§4).
    Batch 3 implements immediately — its measured consumers already
    exist (§8) and the richer surface is the registered re-baseline
    target for a restarted M22 after this milestone's exit audit. The
    prior M22 stop-loss verdict stays in the dual-implementation
    register; M22 re-entry re-runs S1/S2 differentials on the new
    surface under the unchanged ADR-0015 contract.
  - The original churn rationale survives in both routes: language
    churn must not invalidate a mid-flight bootstrap closure, so Route
    B fires only after convergence is declared absent, never during a
    closure attempt. Only RFC/contract drafting and measured-consumer
    evidence proceed while M22 is mid-flight regardless of route.
  - **Owner parallel directive (2026-09-30, registered):** 「现在有个
    代理在完成M22，你来完成M23」 authorizes batch-3 implementation in
    parallel with the mid-flight M22 line. The dual-gate's churn concern
    is neutralized by the RFC-0048 §4/§9 and RFC-0049 §4/§8 append-only
    proofs (every new form widens today's refusal surface; the selfhost
    sources consume none of it; the M22 differential stays byte-stable —
    re-verified by the STEP-0329 shared-tree record). The selfhost
    re-baseline remains a separately registered decision (§9) and the
    M22 critical path is untouched.
- Per-item accepted RFC (RFC-0033 gate): measured consumer, explicit
  desugar, source-map identity, formatter idempotence, typed
  diagnostics for pre-existing identifiers that used the new keywords/
  operators.
- No reserved STEP numbers: implementation STEPs are allocated only
  when an RFC is accepted and the exit test is bounded.

## 3. Exit gates

1. Every accepted batch-3 item executable end-to-end: e2e byte-exact
   corpus through the real runner, including limit+1 refusals.
2. Formatter idempotence and canonical output on every new shape;
   frozen snapshots additive-only (existing corpus bytes unchanged).
3. Machine support matrix rows + validator extension; undeclared
   check/build/run gap count stays zero for the application profile.
4. Typed diagnostics with stable codes for every new refusal class;
   action-hint catalog extended (M21 §3.3 contract).
5. Self-host re-baseline registered: the Sico frontend (parser/checker/
   formatter slices) either consumes the batch-3 surface or a declared,
   disjoint subset — recorded in the M22 dual-implementation register,
   not silently divergent.
6. M0–M21 and every landed M22 slice regression green + explicit exit
   audit with per-gate evidence. Route B does not imply M22 S6/S7 GO;
   the stopped M22 attempt remains a registered NO-GO.
7. AI-generation-noise claims (if any) only from owner-credentialed
   live-model runs; offline measurement is labeled offline.

## 4. Non-goals

- No new IR operations (desugars to existing CFG/match/intrinsic
  machinery only).
- No generics, macros, operator overloading, or implicit conversions.
- No bare-literal grammar change without the recorded re-measurement.
- No churn to frozen profiles or byte-frozen corpora (append-only).
- No claim that batch 3 makes the language "v1-final" — that verdict
  belongs to M25's freeze.

## 5. Risks

- The self-host frontend re-baseline is real work; if batch 3 lands a
  wide operator set, the Sico parser slices must re-freeze. Mitigation:
  the RFC fixes the smallest closed set before implementation.
- `&&`/`||` short-circuit semantics vs the all-return match desugar:
  the RFC must prove the desugar preserves evaluation order and
  side-effect visibility, or restrict v0 to non-effect operands.
- Diagnostics churn: new operator errors must not renumber existing
  stable codes.

## 6. Evidence classes

`internal-fixture` for all corpora; AI-noise reductions are
`blocked-external-evidence` until owner credentials enable live-model
re-measurement. No platform claims arise from this milestone.

## 7. Kickoff requirement

The kickoff inventory is a measurement STEP in the STEP-0129/0142 pattern:
probe the current surface (operator/keyword absence, ceremony counts on the
selfhost corpus), the RFC set, and the machine matrix extension before any
grammar change. It may run while M22 proceeds. **Done: STEP-0284**
(execution card 23-A) froze [`m23-ceremony-census-2026-09-25.json`](../reports/m23-ceremony-census-2026-09-25.json)
via `tools/census-m23-ceremony.ps1` — deterministic, recomputable textual
counts over the 8-source selfhost tree and the 51-file application corpus;
the AI-generated corpus and runtime call/allocation counters are recorded
gaps, not estimates. The first implementation
STEP additionally requires one of the §2 M22 routes and the relevant
accepted per-item RFC.

## 8. Per-item work protocol (refined 2026-09-22; no STEP numbers reserved)

Every item follows the same four-phase protocol. Phase names are working
labels for planning, not allocated STEP identifiers.

### 8.1 Item 1 — infix comparison and logical operators

> Status 2026-09-30 (STEP-0328): **implemented** under the owner
> parallel directive (§2). [`RFC-0048`](../rfc/RFC-0048-language-v1-batch3-infix-comparison-logical-and.md)
> (accepted by the STEP-0323 adjudication) landed: lexer pair scan,
> condition-position desugar onto `EqualFixed`/`LessFixed` with branch
> inversion and atom-restricted swap forms (E2025), short-circuit `&&`
> between comparison conditions in if/while, E2024 message generalized,
> formatter one-space canonical, matrix rows + validator + e2e and
> refusal corpora, limit+1 nesting evidence. `||`/`!` out of v0 as
> drafted. The `<=` check-green/build-refused debt is closed (accepted
> in-shape; narrower named refusal out-of-shape).

- **Measured consumer (inventory phase).** Enumerate the current tracked
  `selfhost/*.sico` sources and line counts in the inventory STEP (the
  historical eleven-file/14,018-line snapshot is no longer the current
  denominator after STEP-0278), then add the frozen application corpora.
  Count and freeze a ceremony table: (a) `Result[Bool, NumericError]` match blocks
  whose only purpose is one comparison (`less_than`/`greater_than`/…),
  (b) nested-`if` conjunction/disjunction sites two or more levels deep,
  (c) `checked_add`/`checked_sub` match blocks per arithmetic site. Report
  density per 1,000 lines. No operator decision is argued from
  recollection.
- **RFC proof obligations.** Decide the smallest closed operator set
  (evaluation candidates: `< <= > >= !=` and `&& || !`; the RFC decides,
  the punctuation table is not grown ad hoc); give the explicit desugar to
  existing match/intrinsic machinery with **no new IR operations**;
  prove evaluation-order and short-circuit behavior for `&&`/`||` — v0 may
  restrict to side-effect-free operands, and any restriction must be
  enforced by a typed refusal, not a silent fallback; source-map identity
  (operator spans map onto the desugared region, diagnostics point at the
  operator); formatter idempotence with a canonical spacing table;
  new typed diagnostic codes for every new refusal class, append-only (no
  renumbering of existing stable codes); compatibility audit for
  pre-existing identifiers against the new tokens.
- **Bounded exit test.** New-shape corpus byte-exact end-to-end through
  the real runner including limit+1 refusals; machine support matrix row +
  validator; frozen snapshots unchanged (append-only); self-host
  re-baseline entry recorded (§9).

### 8.2 Item 2 — expression-position conditions

> Status 2026-09-30 (STEP-0329): **implemented** under the owner
> parallel directive (§2). [`RFC-0049`](../rfc/RFC-0049-language-v1-batch3-let-bound-if-match.md)
> (accepted by the STEP-0323 adjudication) landed: parser openers for the
> two let-bound forms, check-side E2026/E2027/E2001 arm rules with
> join-time binding visibility, lowering through the landed cell +
> branch-writes + join-read machinery (statement-match payload spill
> reused verbatim), formatter canonical shape (else:/case: at header
> depth), matrix rows + validator + e2e and refusal corpora, limit+1
> nesting evidence. Annotated/nested/multi-statement forms refused as
> drafted.

- **Measured consumer.** Count the mutable-cell + all-return match +
  join-read sites that exist only to bind one conditional value, in the
  selfhost sources and the M14 solver corpus; freeze the count.
- **RFC proof obligations.** Choose the smallest let-bound statement form
  only (`let x = if …` / `let x = match …`); general nested expression
  conditions stay out of v0; the desugar is the already-supported
  cell + branches + join read with no new IR operation; formatter
  canonical shape and idempotence; typed refusals for unsupported operand
  shapes.
- **Bounded exit test.** Same bar as 8.1 (corpus, matrix, snapshots,
  re-baseline entry).

### 8.3 Item 3 — bare-literal re-measurement (policy item)

- **Corpus.** Owner-reviewed AI-generated Sico from the STEP-0175-era DX
  audit method, plus the selfhost sources; the corpus is frozen before
  counting.
- **Metric.** Literal-ceremony tokens (`I64.literal(...)` and friends) per
  expression and per KB, alongside the same counts for a hypothetical
  typed-literal form.
- **Output.** A decision record appended to the RFC-0046 outcome: either
  "D3 stands" with the new measurement, or the smallest typed-literal RFC
  carrying the full RFC-0033 evidence. No grammar change may precede this
  record.
- **Status 2026-09-25 (card 23-A/23-B).** Selfhost-side measurement is
  frozen (STEP-0284: 2,169 literal-constructor calls, 3.199/KB). The
  owner-reviewed AI-generated corpus remains the unmet leg; the decision
  record is therefore **deferred**, the item stays open, and no grammar
  change is made.

### 8.4 Item 4 — `text.chars` (queued by RFC-0046 §2)

- **Measured consumer.** Compare the composed
  `text.length`/`char_at` loop against the candidate intrinsic on the
  frozen word-scanning corpora (formatter `word_kind` region, checker
  scans) — call count, allocation count and byte output.
- **Decision rule.** The intrinsic lands only if measurably better on the
  frozen corpora **and** it carries the same RFC-0033 evidence bar;
  otherwise the composed form is recorded as final and the item closes.
- **Status 2026-09-25 (card 23-A/23-B): closed per the recorded rule.**
  The runtime leg was never demonstrated: the runner CLI exposes no
  call/allocation counters (recorded in STEP-0284), so "measurably
  better" cannot be shown. Frozen demand: 153 composed
  `text.length`/`char_at` call sites, 0 in the formatter `word_kind`
  region. The composed form is recorded as final and the item closes; a
  reopened item would need new runtime measurement evidence and the full
  RFC-0033 bar.

## 9. Sequencing and dependency map (refined 2026-09-22)

- One kickoff inventory covers all four probes (§7); after that the items
  are independent — each may proceed or close on its own RFC without
  waiting for the others.
- Ordering constraint: **implementation** of an item waits for either
  M22 S7 GO (Route A) or the recorded R3 stop-loss (Route B), plus its
  accepted RFC (§2). Kickoff measurement and RFC drafting may run while
  M22 is active. Route B re-baselines M22 only after this milestone's
  exit audit.
- M22 interaction per accepted item: an entry in the M22
  dual-implementation register plus an explicit self-host re-baseline
  decision — the Sico frontend either consumes the new surface or a
  declared, disjoint subset is recorded; a wide operator set forces the
  parser slices to re-freeze, which is exactly why the RFC must fix the
  smallest closed set first (§5 risk 1).
- **Dual-implementation register entry (2026-09-30, STEP-0328/0329):**
  the Sico selfhost frontend declares a **disjoint subset** of the
  batch-3 surface: the selfhost lexer/parser/checker/formatter sources
  keep the intrinsic-call style (0 infix-operator occurrences, 0 let-
  bound forms) and do not consume `< <= > >= != &&` or `let x =
  if/match`; no selfhost slice re-freezes and the M22 differential
  (corpus-v0.json stays frozen at 215 entries; post-freeze corpus files
  are excluded per the RFC-0047 record precedent) is byte-stable. A
  future re-baseline that adopts the batch-3 surface re-opens S1/S2
  differentials under the unchanged ADR-0015 contract.
- Items 1 and 2 share one diagnostic-namespace decision (append-only
  codes); landing them under one RFC-set review keeps the code registry
  coherent, but they remain separately gated items.

## 10. Residual closure (2026-10-20, STEP-0489)

The 2026-09-30 exit audit closed items 1/2/4 and left three residuals. This
section records their current state; it does not change any exit gate.

| Residual | State after STEP-0489 | Depends on M22? |
|---|---|---|
| `diagnostics/catalog.json` + `semantic-case-map.json` full protocol extension (E2021–E2027, E2031) | **closed** — the eight implemented-but-unregistered semantic codes are now registered with catalog templates, and a new implementation-level corpus proves every one of them against the real compiler (`sico check --json`: code, key, argument set and rendered message byte-identical to the catalog) | no |
| item 3 bare-literal re-measurement (owner-review AI corpus leg) | **still deferred-external, now with a live leg executed** (2026-10-21) — the selfhost leg is re-measured at a pinned commit (literal-constructor density **3.199 → 3.055 per KB**, i.e. the explicit-width ceremony did not get denser even though the absolute count grew 2,169 → 3,236), and the AI leg ran for real: 100 billed DeepSeek calls (M13 identity) over the 10 frozen A0 generation tasks, USD 0.0272 → a 100-file / 22,091-byte corpus that contains **0 literal-constructor calls**. That zero is a **surface-syntax artifact** (A0 is an M1-era retained control syntax with no explicit-width literal form and no compiler), so the AI leg is **not commensurable** with the selfhost leg; the measured type facts (`Int` accepts bare literals, `I64` does not) narrow the open question to a context-inferred width literal for `I64`/`U64`. The **owner review of the corpus is still ahead of the freeze**, and no D3 record has been written | no |
| STEP-0328/0329 independent CI adjudication | **exercised — red at an M22-owned point** (2026-10-21) — GitHub Actions is reachable again; two runs (`39454624507` on `2531427`, `39457030463` on `6808569`) failed at exactly one test, `sico-app-cli::run_executes_a_script_package_end_to_end`, with `forcing trap when growing memory to 2214592512 bytes`. The cause is a constant mismatch in the **M22** track: the codegen declares the guest memory minimum as `MEMORY_PAGES = 33792` pages (2,214,592,512 B) while the `sico-runtime` host ceiling is 2 GiB = 32,768 pages, introduced by `c50b3fd` (STEP-0484) — the in-process runner path is unaffected because `RunnerLimits` allows 2,560 MiB, which is why only the `sico-app` launch seam is red. Reproduced deterministically on this host with the repo-pinned wasmtime (case A reproduces the CI message byte-for-byte; the pre-`c50b3fd` shape and an equal-minimum module both pass), so it is a contract mismatch, not a runner-memory budget problem. See [`ci-arena-ceiling-attribution-2026-10-21.md`](../reports/ci-arena-ceiling-attribution-2026-10-21.md). Every other CI step is green in both runs and the M23 code footprint (`crates/sico-semantics` diagnostics text + `tools/`) is not implicated, but the gate itself is still red, so this residual stays open on an M22-owned defect | no |

Two catalog-scope boundaries are now recorded explicitly in
[`diagnostics/README.md`](../../diagnostics/README.md) instead of being left
implicit:

- **E8xxx stays outside the catalog.** RFC-0001 reserves `E8000–E8999`; the
  RFC-0039 module/package codes are CLI link-channel identities, not
  source-level compiler diagnostics, and registering them would change a
  reserved partition (new protocol major version + RFC).
- **E1014–E1016 remain unregistered.** They are syntax codes whose case maps
  (`syntax-mutation-map.json`, `top-level-syntax-map.json`) are bound to the
  frozen mutation and candidate corpora; extending those maps is a separate
  bounded step, not a silent table growth inside this one.

The self-host interaction is unchanged: the batch-3 surface stays a declared
disjoint subset (§9), so the M22 differential corpus stays byte-stable and
this step introduces no M22 re-baseline obligation.

### 10.1 STEP-0490 — item 3 preparation (2026-10-21)

| Item | State |
|---|---|
| selfhost leg re-measured | **done** at a pinned commit (2531427; 9/9 selfhost blobs byte-identical to the commit): literal-constructor calls 2,169 → 3,236, but density **3.199 → 3.055 per KB**, comparison-predicate density 59.671 → 54.166 per 1,000 lines. A second, independent measurement tool agrees exactly (3,236) and also prices the hypothetical typed-literal form (42,042 bytes, 3.88% of the corpus) |
| AI corpus source | **frozen** — the 10 A0 `generation` tasks of `sico-ai-eval-v1` (`tools/prepare-ai-eval.ps1 -Category generation -Syntax A0`); B/C are alternative candidate syntaxes and are excluded from a current-Sico ceremony count |
| DeepSeek adapter | **versioned** — `tools/ai-eval/deepseek_generation_corpus.py` (standard library only, key read from `DEEPSEEK_API_KEY` and never persisted), M13-identical identity/parameters, cost computed from provider-reported tokens at the published rates, hard USD 10 cap; the M13 adapter was lost with `target/` and is not repeated here |
| provider credential | **used** — the owner supplied the DeepSeek key on 2026-10-21 for this leg; it was read from the environment only and never written to disk or into the repository |
| offline proof | **done** — fixture mode produces the run file (SHA), the corpus candidates, the per-file SHA manifest and the cost line; `--dry-run` prints the full call plan; a local-mock pre-flight additionally exercised the **live HTTP path** (request shape, response parsing, the 429 backoff branch, corpus + manifest + run-file writing, cost accounting): 14/14 checks, 21 HTTP requests for 20 calls |
| live AI leg | **executed** (2026-10-21) — 100 billed calls (10 A0 generation tasks × 10 repetitions), M13-identical identity (`deepseek-chat`, temp 0, top_p 1, 4 in-flight), **USD 0.0272**, 75,830 in / 6,116 out provider-reported tokens, 0 failed calls. Corpus: 100 files / 22,091 B / 1,190 lines / 400 expression lines / 350 call sites → **0 literal-constructor calls (0.000/KB)**. Evidence: [`ai-eval-item3-live-model-2026-10-21.md`](../../docs/reports/ai-eval-item3-live-model-2026-10-21.md) |
| why the AI leg reads zero | **measured, not assumed** — the A0 candidate syntax writes integer literals plainly (`return keep_size(Bytes(16))`) and defines no explicit-width literal form; A0 is an M1-era *retained control* syntax that no compiler can execute (`syntax-candidates/README.md`), and the current front-end rejects all 100 files with `E1013` (the frozen prompts ask for declaration-only programs; the current language requires an explicit `main`). The zero is therefore a **surface-syntax artifact**: the AI leg as frozen is **not commensurable** with the selfhost leg and can neither corroborate nor refute its 3.055/KB |
| type-system mechanism | **measured** (`sico check --json`) — bare `0` is **accepted** where an `Int` is expected; `return 0` in an `I64` context is `E2002` (`expected I64, found Int; convert explicitly`); `I64.literal(0)` in an `Int` context is `E2001`. `Int` and `I64` have no implicit conversion, so the constructor is forced wherever an `I64`/`U64` value is needed — the selfhost's density reflects its width-typed code, not a tax on all integer use. The narrowest open question is therefore a **context-inferred width literal for `I64`/`U64`**, not a general "bare literal" change |
| corpus statistics | **14 distinct programs**, not 100 samples: at temperature 0 the provider is near-deterministic, so the 10 repetitions per task yield 1–2 outputs each. This is the M13 identity the owner required, but it caps the statistical value of the leg and must not be reported as 100 independent samples |
| owner review of the corpus | **pending** — the corpus is frozen only after the owner reviews it (plan §8.3) |
| D3 decision record | **not written** — no grammar change may precede the recorded measurement, and the AI leg as executed is degenerate for the metric, so the decision needs the owner's explicit call between "D3 stands" with the selfhost measurement plus the AI leg's negative result, and a minimal typed-literal RFC carrying the full RFC-0033 evidence |
