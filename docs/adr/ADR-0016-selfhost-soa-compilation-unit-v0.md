# ADR-0016: Self-host SOA frontend and compilation-unit boundary v0

> - status: accepted (owner directive “完成M22”, 2026-09-20)
> - date: 2026-09-20
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -
> - depends: RFC-0039, RFC-0046, ADR-0015, M22 plan

## Context

M22 already accepts struct-of-arrays (SOA) as the transition representation
while `List[record]` remains outside the executable language set.  The earlier
implementation nevertheless split lowering into a standalone `parser.sico`
program and kept three exact-source IR templates in `compiler.sico`.  It also
treated every checker input as a CLI entry, so the self-host path had no way to
distinguish an entry source from a legal imported `module` source.

The representation and compilation-unit boundary must be frozen before S3/S4
can converge without changing the language surface or weakening ADR-0015.

## Decision

1. M22 v0 uses SOA throughout its Sico-written token, declaration, semantic,
   lowering and codegen state.  Every logical row has parallel arrays with
   identical lengths; a mismatch, missing row, overflow or limit+1 input is a
   typed refusal.  No array lookup may silently substitute a semantic value.
2. `selfhost/parser.sico` is the single reusable lowering module.  A thin
   `parser_driver.sico` owns only the Script ABI used by the real-runner test.
   `compiler.sico` imports the same `lex_words` and `scalar_ir` functions; it
   must not retain exact-source IR templates as a fallback.
3. S2 remains a single-entry checker contract: a standalone source declaring
   `module` returns E8010.  L2 compilation receives a bounded, ordered source
   bundle with exactly one entry and zero or more imported modules.  Imported
   files must declare the matching module identity and pass RFC-0039 duplicate,
   cycle, traversal, depth and count gates.  Entry and module roles are data in
   the compilation unit, not inferred from source spelling.
4. Frozen source identity is computed from canonical UTF-8 LF bytes.  Host
   checkout CRLF materialization never changes a manifest digest.  Invalid
   UTF-8 and non-CRLF carriage returns fail closed.
5. The Rust parser, IR verifier and deterministic backend remain independent
   permanent oracles.  This ADR neither permits a Rust compile hostcall nor
   weakens ADR-0015 `A == B == C` Component-byte closure.
6. The canary order is fixed: the six existing scalar IR shapes, then
   `formatter.sico`, the complete self-host source bundle, the 37 accepted
   Script corpus entries, deterministic Component codegen, and finally the
   ADR-0015 bootstrap harness.  A typed refusal or trap at an earlier canary
   blocks later completion claims.

## Consequences

SOA remains verbose but is now a deliberate bounded architecture rather than a
temporary convention.  The parser and compiler share one lowering path, so a
shape cannot pass a parser-only suite while the compiler silently uses a
template.  Adding `List[record]` later remains a separate RFC and is not an M22
prerequisite.

M22 remains NO-GO until S5 Component bytes, S6 bootstrap/package/budget
evidence, and S7 regression audit pass.  This decision closes an architecture
ambiguity; it does not itself close those gates.

## Validation

STEP-0246 validates canonical manifest regeneration, source SHA checks, the
module/driver split, the unified compiler lowering path, exact scalar IR
baselines and the absence of the former exact-source fallback.

## Links

- [`M22 plan`](../plans/M22-compiler-self-host.md)
- [`ADR-0015`](./ADR-0015-compiler-bootstrap-closure-v0.md)
- [`STEP-0246`](../steps/STEP-0246-m22-soa-convergence-baseline.md)

