# STEP-0173: M22 compiler self-host track planning

> - status: complete (plan recorded; implementation not started)
> - phase: M22 planning (owner session directive「规划自举里程碑」2026-09-14, following the M18–M21 planning pattern)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: [`M22 plan`](../plans/M22-compiler-self-host.md), ROADMAP M22 section, plan/docs indexes, `validate-step-0124.ps1` scope M14–M22

## 1. What was done

The self-host milestone was planned at two declared levels, grounded in
the machine support matrix rather than aspiration:

- **L1 tool self-host**: Sico-written formatter/checker subset,
  byte-exact differential against the Rust tools on the frozen corpora
  (`syntax-candidates/`, `semantic-cases/`, `tests/end-to-end/`),
  plus formatter idempotence.
- **L2 compiler self-host**: Sico-written Script-profile compiler
  (lex→parse→check→lower→codegen) whose corpus artifacts are byte-equal
  to the Rust backend (RFC-0011 oracle), closing with **bootstrap**:
  the Sico compiler compiles its own source, A≡B, repackaged through the
  M7 trust chain and executed by the real runner.

Slices S0–S7 are bounded with per-slice exit tests; entry gate ties the
language prereqs to the M21 batch-2 track (byte/text access, collection
extension — shared work, explicitly not forked) and requires a bootstrap
architecture ADR only before S6, so L1 can start as soon as the language
prereqs land. Non-goals freeze the architecture boundary: the Rust
runner/Host/verifier stay native and become the permanent differential
oracle — self-host makes no de-Rust claim.

## 2. Validation

Document + validator scope change only; no compiled code touched.

- `validate-step-0124.ps1` extended to M14–M22 (plan file, roadmap
  heading order, both index links, unreserved-STEP invariant): OK
  (`milestones=M14-M22 …,compiler-self-host`).
- `git diff --check`: clean.
- ROADMAP references no STEP number without an execution record
  (validator-enforced).

## 3. Honest scheduling note

No STEP numbers are reserved for S0–S7 (planning contract). At the
current cadence the entry prereqs are 1–3 STEPs, L1 is 2–4, L2 is 5–8;
the binding gates are owner acceptance of the byte/text + collection
RFCs (language contract) and the bootstrap ADR — not implementation
effort.
