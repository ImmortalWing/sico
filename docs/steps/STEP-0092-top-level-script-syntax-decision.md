# STEP-0092: top-level Script syntax decision

> - status: complete
> - phase: M9
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## Objective

Compare explicit `main`, unrestricted top-level statements and a labeled `script:` block across parser recovery, formatting, HIR/source maps, LSP and AI tooling, then record an explicit accept/reject decision.

## Delivered

- RFC-0033 accepts the existing explicit typed `main` and rejects both shorthand candidates for M9.
- Parser hardening closes a silent-drop defect where root statements and `script:` blocks previously passed `check` but never entered AST/HIR.
- Stable `E1013 / SYNTAX_UNEXPECTED_TOP_LEVEL` points users to an explicit `main` and recovers at a close or next declaration.
- Formatter refuses invalid root execution without output; HIR and semantic analysis cannot consume it.
- LSP and AI inspect expose the same typed identity; `script` remains an ordinary identifier.

## Exit evidence

`tools/validate-step-0092.ps1` is green across parser, diagnostics, formatter, HIR, CLI, LSP and AI-tool tests. Both rejected candidates produce exactly one E1013, stop before semantic analysis and cannot be formatted; explicit `main` remains accepted and canonical.

Report: [`top-level-script-syntax-decision-v0`](../reports/top-level-script-syntax-decision-v0.md).

## Honest limits

This decision does not claim live-model quality evidence or add declaration-capable REPL cells. Future shorthand requires a new RFC and cannot change this grammar implicitly.
