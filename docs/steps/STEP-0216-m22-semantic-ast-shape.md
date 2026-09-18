# STEP-0216: M22 accepted-corpus semantic AST declaration shape

> - status: complete (accepted semantic-AST slice; S3 remains partial)
> - phase: M22 S3 parser/AST
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/declaration_parser.sico`, `runner/sico-runner/tests/selfhost_declaration_parser.rs`, `tools/validate-step-0216.ps1`

## 1. Result

The Sico parser no longer summarizes a punctuation-free word stream as one
`fn:name/arity` probe. It walks bounded source lines, tracks nested labeled
blocks, and emits the ordered top-level declaration tree consumed by the Rust
parser's semantic `ModuleAst` layer. It recognizes `Newtype`, `Record`, `Enum`,
`Capability`, `Resource`, `Interface`, `Function`, `Module` and `Use`, including
`async`/`export function`, dotted module imports and package-use names.

The real runner executes the Sico parser on every STEP-0214 source accepted by
the Rust parser/formatter. Its output is compared to the permanent Rust
`ModuleAst::shape()` oracle: **99/99 byte-exact**.

## 2. Honest boundary

This is the accepted semantic-declaration AST slice, not complete S3 closure.
Declaration ranges/details, full expression/statement syntax nodes and the 116
formatter-refused cases' parse-error identities are not emitted yet. Therefore
STEP-0213's “real bounded AST” row remains partial, and no formatter, lowering,
codegen or bootstrap claim follows from this STEP.

The next parser slice must add exact declaration metadata and refusal identity,
then expose enough syntax-node structure for S1 formatter round-trip.
