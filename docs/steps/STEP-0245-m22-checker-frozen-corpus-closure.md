# STEP-0245: M22 checker frozen-corpus closure

> - status: complete / S2 GO (declared identity subset)
> - phase: M22 S2 checker
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU

## Objective

Close the declared L1 checker subset without overstating full Rust diagnostic
rendering. On the frozen 215-source corpus the Sico checker must preserve every
Rust acceptance and return the exact frozen diagnostic identity for every
refusal. Accepted-source outline metadata remains the byte-exact output of the
Sico declaration parser established by STEP-0216 and STEP-0219.

## Declared match relation

- 65 Rust-accepted sources are accepted by the Sico checker.
- 116 lexical refusals return `LEXICAL`.
- 34 parser, semantic, effect, ownership, task, stream, component, revision,
  and module refusals return the same frozen identity as the Rust oracle.
- Unsupported or silently divergent frozen cases: zero.
- Full rendered text/JSON, labels, help and source spans are not claimed by
  this subset; the runner transports the identity as typed `invalid-input`.
- Outline kind/name/range/detail remains a separate byte-exact 99-source
  differential through `declaration_parser.sico`; this step composes that
  established evidence with the checker identity matrix rather than claiming
  the checker emits the Rust JSON diagnostic document.

## Changes

- Complete enum/match and Result mapping checks for E3001–E3104.
- Add declared effect/capability, affine resource, future/task, stream,
  component boundary, revision guard and standalone module checks for
  E4001–E8010.
- Track task consumption through `await` and `collect_tasks`, including
  multiple awaits on one source line, so accepted structured-concurrency
  fixtures remain accepted.
- Extend the real-runner test to assert the complete frozen partition:
  116 lexical refusals, 34 exact semantic/module identities, 65 accepts and
  zero unsupported cases.

## Validation

```powershell
.\tools\validate-step-0245.ps1
```

The validator runs the Rust semantic oracle, frozen bundle integrity, the
real-runner checker/compiler/declaration-parser suites, all-target runner
clippy, STEP uniqueness and whitespace checks.

## Residuals

S2 is GO only for the declared identity/outline subset above. Full rendered
diagnostic parity is outside that v0 subset. M22 overall remains NO-GO: S3 and
S4 are partial, S5 has no general deterministic Component encoder, and S6
A=B=C self-compilation has not started.
