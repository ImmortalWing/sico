# STEP-0197: M22 S6 — deterministic function-body statement shape

> - status: complete
> - phase: M22 S6 lowering continuation after STEP-0195
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. Objective

Extend the exact `fn:name/arity` summary from STEP-0195 with the first
bounded function-body shape needed before expression parsing: counts of
`for`, `while`, `if`, `match`, `let`, `set`, and `return` starters,
stopping at that declaration's `end function`.

## 2. Mechanism

- `count_until_end_function` walks the existing token stream for one
  target keyword. `end function` closes the function; the second word of
  other terminator pairs (`end if`, `end while`, `end match`, `end for`)
  is skipped so terminators are not counted as starters.
- `find_body_end` advances `summarize` directly to the next declaration.
- `fn_entry` emits a fixed-order canonical form:
  `fn:name/arity{for=N,while=N,if=N,match=N,let=N,set=N,return=N};`.
  Its construction is deliberately incremental rather than one deeply
  nested concat expression; the first real build rejected the nested
  draft with `UnexpectedDelimiter(RightParen)`.

## 3. Executable evidence

The runner corpus contains two functions. `main` includes nested
for/if/while/match blocks, two lets, two sets and two returns; `add`
has arity two and one return. Real `sico build --profile script-v0`
plus real `sico-runner` execution produced the following byte-exact
output (1/1 test green on Windows x64):

```text
fn:main/0{for=1,while=1,if=1,match=1,let=2,set=2,return=2};fn:add/2{for=0,while=0,if=0,match=0,let=0,set=0,return=1};
```

## 4. Support boundary and residuals

This is an `internal-fixture` lexical shape, not an AST or typed IR
claim. It does not yet distinguish a keyword-shaped token in every
possible expression/string/comment context, validate block nesting,
bind names, parse expressions, or emit verifier-accepted IR. Those
remain the next S6 slices. Bootstrap closure also remains blocked on the
plan-required bootstrap architecture ADR, followed by codegen,
RFC-0011 byte comparison, M7 packaging, and budget measurement.
