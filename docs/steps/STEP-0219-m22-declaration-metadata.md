# STEP-0219: M22 accepted declaration metadata parity

> - status: complete (accepted declaration metadata; S3 remains partial)
> - phase: M22 S3 parser/AST
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/declaration_parser.sico`, `runner/sico-runner/tests/selfhost_declaration_parser.rs`, `tools/validate-step-0219.ps1`

## 1. Result

The Sico parser now has a `--metadata` mode that emits every accepted top-level
declaration's kind, normalized name, half-open UTF-8 byte range and structured
detail. It tracks original LF/CRLF widths rather than deriving offsets from
normalized lines.

The real runner compares that canonical stream with public Rust `ModuleAst`
declarations on all 99 STEP-0214 formatter-accepted sources. Results are
**99/99 byte-exact**, alongside the existing 99/99 declaration-shape test.
Current frozen entries all carry `DeclarationDetail::None`; package-use and
versioned-interface detail variants therefore remain outside this evidence.

## 2. Honest boundary

This closes accepted top-level declaration metadata, not full S3. Expression
and statement syntax nodes, parser recovery nodes, lexical/parser refusal kinds,
related ranges and recovery anchors remain open. No checker, lowering, codegen,
bootstrap or package claim follows from this STEP.
