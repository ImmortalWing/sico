# STEP-0141: M14 exit audit

> - status: complete
> - phase: M14 (M14 plan §6 — explicit GO/NO-GO audit)
> - date: 2026-09-06
> - owners: autonomous-agent (auditor)
> - contract: M14 plan `docs/plans/M14-application-ready-language.md` §6;
>   RFC-0038 (accepted 2026-09-05)

## Verdict

**M14 = GO**, with one externally gated item recorded as
`blocked-external-evidence` (exit gate 7, AI regression re-measure) that
does not gate the language/runtime milestone per the M13 precedent
(STEP-0127 promoted M12 to full GO with the M13 (d) item blocked the same
way).

## Exit-gate evidence (M14 plan §6)

1. **No undeclared check/build/run mismatch** — GO. The machine-readable
   matrix `tests/language-matrix/application-profile-v0.json` declares
   every profile construct (executable / refused with typed reasons) and
   is validated by `tools/validate-step-0131.ps1`; STEP-0134–0139 each
   updated it in lockstep. One undeclared *behavior* gap was found and
   fixed during acceptance work (string `\n` escapes silently kept as two
   characters; STEP-0139 §3) — now declared and covered.
2. **All three acceptance applications run from Sico source through a
   real Component Runtime** — GO.
   - offline block-game solver (`block-solver.sico`,
     `runner/sico-runner/tests/block_solver.rs`),
   - streaming transform (`stream-transform.sico`,
     `tests/stream_transform.rs`; 1 MiB through default budgets),
   - capability-backed state machine (`capability-state-machine.sico`,
     `tests/capability_state_machine.rs`; fs capability, revision guard,
     typed transitions, deterministic recovery).
3. **Solver matches the fixed Python oracle corpus within recorded
   budgets** — GO. 4/4 frozen fixtures byte-exact
   (`validate-step-0138.ps1` regenerates the oracle live). Budgets: guest
   node budget 200,000 enumerations (mirrored by the oracle; the limit
   fixture exceeds it and yields the typed partial-evidence outcome);
   runner fuel 2×10^11, timeout 600 s; `medium` dominates at ~8 s wall on
   the Windows dev host. Node accounting equivalence proven at
   204,077/204,077 for the limit fixture.
4. **Limit+1 inputs fail with typed outcomes and leave the Host
   reusable** — GO. `RunOutcome::StackLimit` (STEP-0137) with an explicit
   reuse-after-exhaustion test; fuel exhaustion and memory limits remain
   typed from M10/M11; the solver's node budget produces the typed
   `"limit":true` application outcome with best-so-far evidence.
5. **Module/package/WIT security mutation corpora fail closed** — GO by
   carried evidence: the M7 security/mutation corpora (STEP-0069/0072)
   and M12 trust boundaries are untouched by M14's additive language
   surface; the M14 additions compose only already-granted capabilities
   (fs scopes, endpoint grants, stream-handle ceilings — the live-stream
   cap demonstrably caught a handle leak during STEP-0139 bring-up).
6. **Windows and Linux native runner evidence for the portable core** —
   GO. Windows x64-gnu: full serial runner suite green except the
   documented environment-sensitive console-control fixture
   (STEP-0131 §1.6 flake class; passes in isolation — reconfirmed
   2026-09-06). Linux x64 (WSL2 Ubuntu-24.04, native): full serial runner
   suite green 2026-09-06 — evidence
   `target/evidence/step-0141/linux/runner-suite.log` (14/14 suites ok,
   0 failures).
7. **AI generation, repair and comprehension regression measured under
   the M13 protocol** — `blocked-external-evidence`. The authoritative
   protocol requires owner-supplied live-model credentials (STEP-0128
   measured 0.9095 on the pre-M14 profile). Re-measuring on the M14
   profile is owner-gated and recorded as follow-up; it does not gate
   the language-runtime milestone.
8. **Full M0–M13 regression green + explicit GO/NO-GO** — GO. Root
   workspace (`--all-targets --all-features`) and runner workspace
   (serial) suites green after every STEP-0130–0140 change; fmt and
   clippy `-D warnings` clean; planning-contract validator
   (`validate-step-0124.ps1`, updated so the roadmap may reference
   implemented steps only) green.

## Scope honesty

- The M14 plan §3.3 items (source modules/imports, versioned package
  resolution, compiler-facing user WIT) were **not** part of the frozen
  application profile (RFC-0038 §2) and received no STEP; they remain
  classified exactly as RFC-0038 §1 left them and are not claimed.
- The solver's seeded rollout scoring uses the STEP-0135 PRNG at profile
  level; wiring rollouts into candidate scoring is future work outside
  the frozen corpus.
- The flake class of STEP-0131 §1.6 (console-control / RSS tests under
  parallel load) remains documented, serial-run mitigated.

## Follow-ups recorded

- Owner-gated: live-model M14-profile regression (gate 7).
- `sico test` v1 candidates: provider grants in manifests, streaming
  profile support.
