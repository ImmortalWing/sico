# STEP-0177: M21 exit audit — DX quality, stdlib batch 2, ecosystem activation

> - status: complete — **audit verdict: GO（6/6 门，两项外部门控诚实登记）**
> - phase: M21 exit audit (M21 plan §5)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: [`M21 plan`](../plans/M21-developer-experience-and-stdlib.md), STEP-0174/0175/0176 execution records, this audit

## 1. Per-gate verdicts (M21 plan §5)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | Stdlib batch 2 with content-asserting corpora; byte/text gap closed (tetris ASCII-mask retired or tracked) | **GO** | STEP-0174: 9 intrinsics, `stdlib_batch2.rs` 3/3 byte-exact. ASCII-mask retirement tracked to RFC-0046 D4 list-element extension (now landed in STEP-0176) |
| 2 | v1 batch 2 RFCs accepted and landed (bare-literal, for-loops, error propagation) | **GO** | RFC-0046 accepted; STEP-0175 for-loops + `?` desugar landed; STEP-0176 list family landed; bare-literal D3 recorded as no-grammar-change |
| 3 | Diagnostics: every stable code has an action hint; AI repair measurement on v1 surface | **GO（提示）/ 外部门控（AI 测量）** | 16 E1xxx codes carry action hints (`sico_diagnostics::action_hint` + fixture test); `sico explain <code>` CLI landed. AI repair re-measurement is owner-credential-gated (M13) |
| 4 | LSP completion/hover against pilot workspaces | **GO** | `sico-language-server` completion/hover/definition/references green (9/9); semantic-index driven; intrinsic symbols surface via the index |
| 5 | Standard library published as packages and consumed via registry | **GO** | `registry_rehearsal_publish_discover_download_verify` 1/1 (publish→channel→discover→download→verify); `packages_resolve.rs` 3/3 (consumer-side resolution + execution); csv/table-stats packages consumed end-to-end |
| 6 | Full M0–M20 regression green; exit audit explicit | **GO** | `tools/run-ci.ps1` 10/10 CI GREEN on this tree (fmt, both clippies, both test suites, module boundaries, planning contract, application-profile matrix, cross-host matrix + UI corpus, whitespace) |

## 2. What M21 actually delivered

- **Stdlib batch 2** (STEP-0174): 9 intrinsics as guest-side emitted helpers,
  content-asserting corpus; the tetris reader's byte-level access gap is
  closed by `sico.bytes.at`/`text.char_at`; `split_words`'s STEP-0083-era
  final-token defect was found and fixed by the new corpus.
- **Language v1 batch 2** (STEP-0175/0176): for-loops (general-CFG
  desugar), `?` error propagation (match/return desugar), bare-literal
  decision recorded; `List[I64]`/`List[U64]` family with signed/unsigned
  ordering comparators. STEP-0148 frozen shapes byte-identical throughout.
- **DX quality** (this step): action-hint catalog for all 16 stable syntax
  codes + `sico explain` CLI; LSP completion/hover verified against the
  pilot index.

## 3. Honest external gates (unchanged, not M21 failures)

- AI repair / live-model DX measurement: owner credentials (M13 register).
- A second clean-room consumer with an independent author: recruiting is
  owner-gated per the M21 plan.

## 4. Residuals (next work)

- M22 S1: Sico-written formatter (differential vs `sico format` on the
  frozen corpora) — the first self-host slice, now unblocked by byte/text
  access.
- `text.chars` intrinsic (batch 3) rather than implicit decoding.
- `map.entries` (pair records in lists) stays proposed.
