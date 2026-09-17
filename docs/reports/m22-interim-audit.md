# M22 interim evidence audit

> - verdict: **NO-GO / implementation in progress**
> - date: 2026-09-17 (updated through STEP-0208)
> - trigger: owner directive "先完成自举，再完善 GUI/原生界面库"
> - evidence class: internal-fixture

## Executive finding

The authoritative M22 plan requires full-corpus L1 differentials, a real AST,
canonical typed IR across the frozen compiler subset, Sico-native deterministic
Component codegen, A=B=C self-compilation, corpus replay, M7 packaging and
budgets. STEP-0201–0208 have closed the source corpus, S1 formatter, lossless
lexer, accepted declaration metadata, native Script build oracle, and the
compiler frontend module integration. Executable evidence still does not
satisfy the lowering, general codegen or bootstrap gates.

The STEP-0200 correction remains binding: old “S1-S5 landed” shorthand was an
overstatement. Later executable evidence may close an individual row, but a
bounded seam is never promoted automatically to a later slice or M22 GO.

## Gate table

| Plan gate | Executable evidence | Verdict |
|---|---|---|
| L1 formatter byte-exact + idempotent on frozen corpora | STEP-0204/0205: 99/99 accepted byte-exact and idempotent; 116/116 typed lexical refusal through real runner | **GO (S1)** |
| L1 checker declared diagnostic differential | `selfhost_checker`: one accept + one missing-colon refusal | **NO-GO** |
| Lexer/token/AST parser | STEP-0202: lossless tokens 215/215; STEP-0203/0206: accepted declaration shape/metadata 99/99; STEP-0208: lexer/parser linked into compiler on its own 3 sources; expression/statement and refusal AST absent | **partial** |
| Canonical typed IR + Rust verifier differential | STEP-0197: six accepted scalar shapes + one refusal | **partial** |
| Sico-native deterministic codegen | STEP-0199: one fixed constant-return Core Wasm byte seam; no general encoder or Component wrapper | **partial** |
| Frozen corpus artifact equality | STEP-0207 freezes native oracle: 37 reproducible Script Components + 178 no-artifact refusals; guest has not reproduced them | **NO-GO (oracle ready)** |
| A=B=C self-compilation | Sico compiler does not compile `selfhost/compiler.sico` | **NO-GO** |
| M7 `.sapp` verify/execute | no self-host compiler package | **NO-GO** |
| wall/fuel/memory/output measurement | no closure samples | **NO-GO** |
| M0-M21 regression + exit audit | not run for a closure candidate | **NO-GO** |

## Measured implementation boundary

- `selfhost/formatter.sico`: S1 complete on the frozen 215-source
  accept/refuse corpus; this does not supply checker semantics.
- `selfhost/checker.sico`: still a header-colon checker, not the Rust semantic
  subset.
- `selfhost/tokens.sico`: lossless lexer parity on 215/215 bounded line
  segments. `selfhost/parser.sico` matches accepted top-level declaration
  metadata, not expression/statement/recovery trees.
- `selfhost/compiler.sico` now links `compiler_lexer.sico` and
  `compiler_parser.sico`; the actual compiler Component consumes their token
  and declaration streams on its own sources. Its lowering remains bounded to
  scalar identity, constant and `AddInt` canonical IR; codegen remains the
  fixed `answer() -> 42` Core Wasm transport.
- `selfhost/script-build-corpus-v0.json` is the strict native target: 37
  byte-reproducible accepts and 178 no-artifact refusals.
- Rust verifier/backend remain correctly independent and reject any attempted
  substitution of semantic equivalence for byte equality.

## Required closure sequence

1. Complete S2 checker diagnostics and extend the integrated parser from
   declaration metadata to expression/statement and refusal AST identities.
2. Implement semantic/type tables and canonical IR for every construct used by
   the compiler source and frozen corpus; verify/reserialize natively.
3. Implement a general deterministic Core Wasm + Component encoder in Sico;
   the fixed STEP-0199 byte vector is only its transport seam.
4. Reproduce the STEP-0207 37-accept/178-refuse matrix from the guest, then
   execute ADR-0015 A=B=C, M7 package verify/execute and the
   five-sample budget protocol.
5. Run M0-M21 regression and issue the S7 exit audit.

M23 remains entry-gated on an eventual M22 GO. Planning M23/M24 in STEP-0198
does not change this verdict.
