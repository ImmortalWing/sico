# STEP-0200: M22 S6 — recursive return-expression tree

> - status: complete
> - phase: M22 S6 expression parsing continuation after STEP-0199
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. Objective

Replace STEP-0199's lossy `call:path/arity` summary with the recursive
argument tree needed by typed lowering. Preserve strings as one token,
negative integer spelling, dotted callees, source order, and a hard
expression-depth bound.

## 2. Mechanism

- `lex_words` now uses byte length (not decoded character count), retains
  `-` and `.`, and scans quoted strings including escaped bytes as a
  single token. Newlines inside a string do not terminate an expression.
  An unterminated string yields the transitional `ERR-string` token list.
- `matching_close` pairs parentheses with checked depth arithmetic.
- `expression_shape_depth` recursively splits only depth-zero commas and
  emits `int:`, `bool:`, `name:`, `string:`, or
  `call:path(child,...)`. Parenthesized expressions are stripped. Depth
  above 64 emits `error:depth` instead of recursing without a bound.

## 3. Executable evidence

The real-runner corpus now proves all of the following in one execution:

- `-1` remains `int:-1`;
- `"ready, (nested-looking)"` remains one string atom despite its comma
  and parentheses;
- `I64.checked_add(I64.literal(1), b)` becomes
  `call:I64.checked_add(call:I64.literal(int:1),name:b)`, preserving the
  nested argument tree and order;
- the earlier exact signatures and statement inventory remain stable.

`sico build --profile script-v0` plus the real Wasmtime runner: 1/1
byte-exact test green on Windows x64.

The first build of this slice produced a typed lowering refusal,
`unsupported cell redeclared`, because two loops in one Sico function
used the same local name. Renaming the second loop cell made the real
build green; no compiler workaround was introduced.

## 4. Support boundary

This remains an `internal-fixture` parser tree, not verifier-accepted
typed IR. Source byte ranges are not yet carried alongside tokens;
comments, operators, named arguments, types/bindings and typed diagnostic
objects remain incomplete. The next slice must emit a minimal canonical
IR module for a frozen scalar-return corpus and pass the existing Rust IR
deserializer and independent verifier. Unsupported input must fail typed
and produce no IR artifact.
