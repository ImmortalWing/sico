# M22 interim evidence audit

> - verdict: **NO-GO / implementation in progress**
> - date: 2026-09-20 (updated through STEP-0243)
> - trigger: owner directive "先完成自举，再完善 GUI/原生界面库"
> - evidence class: internal-fixture

## Executive finding

The authoritative M22 plan requires full-corpus L1 differentials, a real AST,
canonical typed IR across the frozen compiler subset, Sico-native deterministic
Component codegen, A=B=C self-compilation, corpus replay, M7 packaging and
budgets. STEP-0214–0221 have closed the source corpus, S1 formatter, lossless
lexer, accepted declaration metadata, native Script build oracle, and the
compiler frontend module integration. Executable evidence still does not
satisfy the lowering, general codegen or bootstrap gates.

The STEP-0213 correction remains binding: old “S1-S5 landed” shorthand was an
overstatement. Later executable evidence may close an individual row, but a
bounded seam is never promoted automatically to a later slice or M22 GO.

## Gate table

| Plan gate | Executable evidence | Verdict |
|---|---|---|
| L1 formatter byte-exact + idempotent on frozen corpora | STEP-0217/0218: 99/99 accepted byte-exact and idempotent; 116/116 typed lexical refusal through real runner | **GO (S1)** |
| L1 checker declared diagnostic differential | STEP-0242: real-runner 215-source lexical subset is exact (116 `LEXICAL` + 99 non-lexical); STEP-0243: E1001–E1016 frozen syntax identities 16/16 through the integrated parser, with the 215-source no-false-positive regression retained; 34 semantic/module Rust refusals remain unmatched | **partial** |
| Lexer/token/AST parser | STEP-0215: lossless tokens 215/215; STEP-0216/0219: accepted declaration shape/metadata 99/99 via `declaration_parser.sico`; STEP-0221: lexer/parser linked into compiler on its own 3 sources; the primary lowering parser additionally carries the STEP-0193–0212 expression subset, but general statement/recovery/refusal AST remains absent | **partial** |
| Canonical typed IR + Rust verifier differential | STEP-0193–0212 plus STEP-0222–0240: 110 positive fixtures cover recursive expression trees, parameters, constants, typed user calls, fixed-width operations, I64/U64 checked arithmetic matches with recursive operands, a loop-driven straight environment with cross-statement SSA, all Rust lower_core straight-line return shapes, set/mutation cell-mode lowering (write_local/read_local/locals table) extended to operation/call right-hand sides and nested call arguments, and structured if/else plus while loops with break/continue — all byte-identical to the Rust paths; the full compiler corpus remains outside the executed subset | **partial** |
| Sico-native deterministic codegen | STEP-0221 retains one fixed constant-return Core Wasm byte seam; no general encoder or Component wrapper | **partial** |
| Frozen corpus artifact equality | STEP-0220 freezes native oracle: 37 reproducible Script Components + 178 no-artifact refusals; guest has not reproduced them | **NO-GO (oracle ready)** |
| A=B=C self-compilation | Sico compiler does not compile `selfhost/compiler.sico` | **NO-GO** |
| M7 `.sapp` verify/execute | no self-host compiler package | **NO-GO** |
| wall/fuel/memory/output measurement | no closure samples | **NO-GO** |
| M0-M21 regression + exit audit | not run for a closure candidate | **NO-GO** |

## Measured implementation boundary

- `selfhost/formatter.sico`: S1 complete on the frozen 215-source
  accept/refuse corpus; this does not supply checker semantics.
- `selfhost/checker.sico`: imports the integrated lexer/parser, exactly matches
  the frozen lexical partition and E1001–E1016 syntax identities; its remaining
  header report is not canonical and it does not yet implement the 34
  semantic/module refusals.
- `selfhost/tokens.sico`: lossless lexer parity on 215/215 bounded line
  segments. `selfhost/declaration_parser.sico` matches accepted top-level
  declaration metadata; `selfhost/parser.sico` is the primary lowering parser
  advanced through STEP-0240. Neither supplies a general statement/recovery
  tree for the full frozen corpus.
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
   the fixed STEP-0221 byte vector is only its transport seam.
4. Reproduce the STEP-0220 37-accept/178-refuse matrix from the guest, then
   execute ADR-0015 A=B=C, M7 package verify/execute and the
   five-sample budget protocol.
5. Run M0-M21 regression and issue the S7 exit audit.

M23 remains entry-gated on an eventual M22 GO. The registered M23–M25 plans do
not change this verdict.
