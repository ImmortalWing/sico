# Top-level Script syntax decision evidence

> - date: 2026-07-19
> - step: STEP-0092
> - status: verified decision and implementation evidence

## Finding

Before STEP-0092, both `stdout.write(...)` at root and a `script:` block produced a successful parser/check result while contributing no AST declaration or HIR. That was a fail-open diagnostic defect even though the backend did not execute the discarded text.

RFC-0033 resolves the design question by retaining explicit typed `main` and rejecting both shorthand candidates. The implementation also closes the silent-drop path.

## Evidence

The final `tools/validate-step-0092.ps1` run proved:

- explicit `main` checks successfully and formats canonically;
- unrestricted root statements produce one stable E1013;
- a complete or unterminated `script:` block produces one E1013 and recovers at its close or the next declaration;
- rejected candidates stop before semantic analysis and HIR lowering;
- formatter returns no source for rejected candidates;
- LSP diagnostics and AI inspect return `SYNTAX_UNEXPECTED_TOP_LEVEL`;
- the accepted 54-source B corpus and its 12 recovery mutations remain green.

## Decision limits

No new keyword, SyntaxKind, module initializer, hidden entry, capability grant or cache identity was introduced. `sico eval` and `sico repl` keep their independent in-memory expression contracts.
