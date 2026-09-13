# STEP-0171: M21 planning — developer experience, stdlib batch 2, ecosystem activation

> - status: complete (planning only; no implementation claims)
> - phase: roadmap planning (owner directive "完成M19-M20，然后再规划M21", 2026-09-13)
> - completed: 2026-09-13
> - owners: autonomous-agent
> - artifacts: [`M21 plan`](../plans/M21-developer-experience-and-stdlib.md); validator extended to M14–M21; both index links; ROADMAP section

## 1. What was done

M21 planned as the "make it enjoyable" milestone after the M14–M20
completeness arc, grounded entirely in measured friction:

1. **Stdlib batch 2** (§3.1): byte/text access intrinsics — the oldest
   measured gap, whose workaround (the ASCII-mask op) is retired by it —
   plus sort/min/max/format and map.values/entries, each with
   content-asserting corpora.
2. **Language v1 batch 2** (§3.2, RFC-gated): the bare-literal typing
   decision (with the NUM-001 trade-off attached), for-loops,
   error-propagation shorthand, reserved-word migration.
3. **DX quality** (§3.3): action hints on every stable diagnostic,
   bounded LSP completion/hover, `sico explain`.
4. **Ecosystem activation** (§3.4): the standard library published as
   packages through the M19 registry path; an independent-author
   consumer (external-pilot class, owner-gated); live-model DX
   measurement (owner-gated credentials).

Entry gates measured: M19 CI/release GO ✓ (STEP-0168), M18 4/5 ✓, v1
decisions = entry work, live-model = owner-gated.

## 2. Validation

`validate-step-0124.ps1` extended to M14–M21 (plan file, roadmap
heading, both-index linkage) — green. Document-only step.
