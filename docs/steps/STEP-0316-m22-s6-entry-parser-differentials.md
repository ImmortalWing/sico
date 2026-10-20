# STEP-0316: M22 22-D S6 entry — parser.sico W2 differentials begin (three typed-frontier slices)

> - status: implementation complete locally; independent CI pending (gitcode pushed, GitHub unreachable from this host)
> - phase: M22 compiler self-host, execution card 22-D (S6 entry inventory + first parser.sico S3/S4 slices)
> - date: 2026-09-29
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

With the formatter canary closed at 30/30 (STEP-0314), card 22-D re-bases
S3/S4 progress on the remaining selfhost sources and S6 starts with a
measured inventory. This STEP lands three slices that unblock the first
parser.sico functions, plus the S6 entry inventory itself:

1. **Control-if decline, no-else guard form** — a whileless
   `if <compare>: return <literal>` without `else` followed by a trailing
   `return` previously refused `ERR:E-SH-IR-STATEMENT` from
   `control_if_function_ir`'s `end if` handler (ret-form requires an
   else arm) and that error is not a decline, so the general-while
   fallback never ran. The handler now returns
   `ERR:E-SH-IR-EXPRESSION` (its established SKIP signal) when
   `sif_has_else` is false, and the general machine lowers the whole
   function byte-exactly (`is_ident_byte`'s five-guard shape).
2. **Control-if decline, ret-form with trailing return** — the dual-guard
   form `if: return / else: return / end if / return <final>` hit the
   same non-decline refusal at the final return (ret-form forbids a
   final arm). The final-return handler now declines with
   `ERR:E-SH-IR-EXPRESSION` when `sif_form` is `ret`, routing to the
   general machine.
3. **`list.get` match subject with canonical literal index** —
   `list_get_match_ir` required the second operand to be the declared
   `U64` parameter. The frozen all-return match now also accepts
   `sico.list.get(<param>, U64.literal(<magnitude>))` with a one-parameter
   signature: it validates the literal shape with typed refusals
   (non-magnitude → `CALL-SHAPE`, unknown receiver member → `CALL-TARGET`,
   trailing operands → `CALL-SHAPE`, literal form under a two-parameter
   signature → `PARAMETERS`), emits the digit-span `const_u64` first
   (oracle SSA: const = param count, subject = +1), and offsets every
   downstream id. The parameter-index form is unchanged; the byte layout
   was verified against the Rust oracle before any test was written.

**S6 entry inventory (measured 2026-09-29, guest compiler vs each
selfhost source, binary-searched function prefix, 5×10⁹ fuel):**
formatter 30/30 (closed); parser 5/115, frontier `lex_words` /
`ERR:E-SH-IR-CALL-TARGET` (statement-position match with `set` arms in
the entry region is the next gap); compiler_lexer 15/19, frontier
`token_finish`; compiler_parser 0/32, frontier `lossless_tokens`;
parser_driver 0/2, frontier `invalid_input`; checker 0/1, frontier
`main`. The bisection boundary is the next function's start line with
the `module` header stripped; `word_at`'s list.get family already
lowers, but sits behind `lex_words` in file order. ADR-0015's
`A == B == C` stages cannot run until these sources lower, so S6 is
**entered, not closed**; the stage-B probe (guest compiling
`compiler.sico`) currently refuses at `ERR:E-SH-IR-STATEMENT` and is
recorded, not hidden. No language, WIT or Host contract changes.

## Evidence and verdict

- The parser prefix through `is_declaration_start` (module header
  stripped) emits Rust canonical IR byte-for-byte
  (`sico_compiler_lowers_parser_is_ident_byte_prefix_byte_exactly`).
- Guard forms: single no-else guard (U64), dual-guard with else, and the
  Bool guard chain all compile byte-exactly and route through the
  general machine (`sico_compiler_lowers_control_if_decline_routes_to_general_machine`).
- `list.get` literal-index and parameter-index matches are byte-exact;
  three typed refusals (literal form with two parameters, negative
  literal, trailing operand) recover afterwards
  (`sico_compiler_lowers_list_get_literal_index_match_byte_exactly`).
- Differential suite 51/51; `selfhost_parser` 2/2; local bounds
  unchanged (`general_while_function_ir` 240/256); formatter canary
  stays 30/30 with full-source exit 0.
- `tools/validate-step-0316.ps1` checks the three test markers, both
  parser.sico markers, and the inherited STEP-0315→…→0262 chain.
- S5/S6/S7 verdicts: S5 slice 1 landed in STEP-0315 (seven families,
  14 fixtures); this STEP does not widen `--emit-core-hex` further. S6
  is entered with the inventory above; `.sapp`/M7 packaging, `A == B ==
  C` and budget measurement have not run. S7 has no new audit. M22
  remains **NO-GO**.
- Independent CI was not run for this STEP at authoring time.

| Check | Result |
|---|---|
| `selfhost_compiler.rs` differential suite | 51 passed, Windows x64 GNU real runner |
| `selfhost_parser.rs` | 2 passed |
| `tools/validate-step-0316.ps1` | exit 0 with the inherited chain green |
| `tools/report-m22-canary.ps1` | 30/30, full-source exit 0 (unchanged) |
| runner `cargo fmt -- --check` | exit 0 |
| independent CI | pending; GitHub unreachable from this host (2026-09-29), snapshot pushed to gitcode `origin` |

## Next frontier pointers

parser.sico `lex_words` needs statement-position `match` with `set`
arms in the entry region (general-machine match currently refuses
entry; the all-return checked-arithmetic route refuses non-return
arms). compiler_lexer `token_finish` and compiler_parser
`lossless_tokens` face the same family plus first-use gaps recorded in
the inventory above. Each becomes its own W2-style STEP with a
byte-exact differential and typed refusal/recovery regressions.
