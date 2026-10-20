# RFC-0047: Record types — minimal closed set for self-host re-baseline and application ergonomics

> - status: accepted (owner directive 2026-09-24 「接受」, same-day draft)
> - date: 2026-09-24
> - phase: M22 R0 outcome (ADR-0017 Option A); serves M22 re-baseline, M14 application profile and M23 ergonomics
> - depends: RFC-0033 (syntax decision gate), RFC-0008 (typed IR), RFC-0011 (deterministic backend), ADR-0016 (SOA invariants), ADR-0017 (R0 decision)

## 1. Summary

One language decision with three surfaces, closing the structural-data gap
that the M22 restart assessment (STEP-0265) and quality review (P0-2) measured
as the dominant self-host maintenance cost:

1. **Record declarations** — nominal product types with named, typed,
   immutable fields.
2. **Record literals and field access** — named construction with existing
   punctuation only (no lexer-table growth), dot field access.
3. **`List[record]` executable** — typed record lists, monomorphized like
   the M14 map/set family, lowering to the same parallel-array machinery
   SOA uses today.

Explicitly out (proposed, not frozen): mutable fields/field-update syntax,
positional literals, pattern matching on records, methods, generics beyond
`List[T]` monomorphization, `Map`/`Set` with record keys/values, records in
the Script WIT surface.

## 2. Motivation and measured consumers

- **Consumer 1 — the self-host corpus.** STEP-0265 §2 inventory: the S3/S4
  layer is 16,931 lines of which `parser.sico` alone is 12,739 (75%); the
  S2 semantic table simulates one struct with 18 parallel `List[Text>`
  columns and a 507-line single function; review P2 documents the
  duplication and fingerprint-rule debts. The later frozen EC-4 census
  (STEP-0270) measured 360 direct list-ceremony lines in 16,931 selfhost
  lines (2.13% ceiling proxy); it does not support the early estimate of
  dramatic line-count reduction. The consumer case is typed invariants and
  long-term maintenance, not source-size savings.
- **Consumer 2 — the application profile.** RFC-0038's application corpus
  (M14 end-to-end sources) and the M22 plan §8 friction list (AI-generation
  ceremony per line) measure record-less cost on ordinary application code;
  the same census method (STEP-0175-era DX audit) applies.

No operator decision is argued from recollection: both census tables are
frozen before implementation lands (EC-4).

## 3. Decision points (to be resolved at acceptance)

- **D1 — declaration syntax.** Block form matching the existing
  `function`/`if`/`while` style:
  ```
  record Point:
      x: I64
      y: I64
  end record
  ```
  Fields: one per line, `name: Type`, canonical order = declaration order.
  Field types in v0: `I64`, `U64`, `Bool`, `Text`, `Bytes`, other named
  records (nesting depth bounded by the existing block-depth budget), and
  `List[T]` where `T` is any v0 field type. No default values.
- **D2 — literal and access syntax.** Named literal with existing
  punctuation only: `Point(x: I64.literal(1), y: I64.literal(2))`
  (keywords `(` `)` `:` `,` already lex; the lexer table gains **no**
  punctuation). Field access: `p.x`. The dot is today used by qualified
  names (`sico.list.length`); resolution rule: a dot following a value of
  record type in operand position is field access, in callee/leading
  position remains a qualified name; a record type and a module sharing a
  field-name spelling in the same scope is a typed refusal (E2xxx family),
  never silent disambiguation.
- **D3 — typing rules.** Records are **nominal**: two declarations with
  identical fields are different types. Records are values with copy
  semantics (copy-on-write, the List/Map/Set precedent). Fields are
  immutable in v0; whole-record replacement via existing `set`;
  per-field `set p.x = …` and `with`-update syntax are proposed for v1.
  Equality `==` on records is refused in v0 (no consumer; field-wise
  comparison composes explicitly).
- **D4 — IR and lowering (honest non-desugar).** Records are **not**
  desugarable to scalar IR: RFC-0008 gains a record type
  (`{"kind":"record","data":{"name":…,"fields":[{"name":…,"ty":…},…]}}`
  canonical form, field order = declaration order, nominal identity by
  declaration). The verifier gains field-access and literal-shape rules;
  canonical IR JSON is append-only. Lowering maps each record value to its
  field sequence; **a `List[record]` lowers to exactly the parallel-array
  representation ADR-0016 mandates today** — element storage is unchanged,
  so guest runtime cost at the List boundary is unchanged; only the
  authored surface and the semantic-layer invariants change. Codegen
  (RFC-0011) emits records as their canonical field sequence.
- **D5 — `List[record]`.** Executable with `sico.list.length/get/append`
  monomorphized per record type (the `sico.list.get[Point]` spelling
  precedent from RFC-0046 D4); `for x in list` iterates records by value.
  `List[List[record]]` and deeper nestings are allowed within the
  block-depth budget.
- **D6 — limits.** Field count per record, record literal arity, and
  nesting depth reuse the existing block/arity budgets; every limit has a
  limit+1 typed refusal. Records in the Script profile v0 WIT surface are
  refused (guest ABI stays scalar/list-of-scalar).
- **D7 — diagnostics.** New append-only codes (no renumbering): record
  redeclaration, unknown field, field type mismatch, literal
  missing/extra/duplicate field, record-in-WIT refusal, nominal-mismatch
  assignment. Action hints per the M21 §3.3 catalog convention.
- **D8 — formatter.** Canonical shape: declaration block as written above;
  literal fields in declaration order with the canonical spacing table;
  idempotence corpus entries for every new shape.

## 4. Grammar (exact)

```
RecordDecl := "record" IDENT ":" Newline FieldLine+ "end record"
FieldLine  := IDENT ":" Type
Literal    := IDENT "(" FieldAssign ("," FieldAssign)* ")"
FieldAssign:= IDENT ":" Expression
Access     := Expression "." IDENT
```

`record` becomes a reserved keyword (RFC-0033 reconsideration gate: explicit
desugar story, source-map identity, formatter idempotence, typed
diagnostics for pre-existing identifiers named `record`).

## 5. Source-map identity

Every new node carries its source span: the declaration block maps to its
`record … end record` extent; a literal maps to its `IDENT(…)` extent with
per-field sub-spans; an access maps the dot and field identifier. The
desugar boundary is the record literal → field-sequence evaluation: source
maps point at the literal, not at synthesized temporaries. Frozen lowering
snapshots are additive-only; previously accepted programs compile
byte-identically.

## 6. Compatibility and migration

- Additive surface: every currently accepted program keeps its bytes;
  every currently refused program stays refused (records only add accepts).
- Identifiers named `record` in existing sources receive typed diagnostics
  with spans and an action hint (rename or quote per the RFC-0001 catalog).
- The SOA invariants of ADR-0016 stay in force until the M22 re-baseline
  lands; the re-baseline re-runs the S1/S2 differentials and the canary
  (STEP-0266 tooling) on the records surface and re-pins the baseline
  before the R3 counter restarts (ADR-0017 §Validation).

## 7. Exit corpus (binding when accepted)

- **EC-1 syntax:** declarations (empty refused, one-field, many-fields,
  nested record fields, list-of-record fields), literals (named, nested,
  in `let`/`return`/call-argument positions), access chains; limit+1 for
  field count, literal arity, nesting depth; `record` as identifier
  (typed refusal with span).
- **EC-2 semantics:** nominal inequality (same-shape declarations are
  distinct), immutability (field `set` refused), copy-on-write aliasing
  behavior, `List[record]` length/get/append, `for` iteration, monadic
  interaction with `Result` (records as ok/error payloads).
- **EC-3 lowering/codegen:** canonical IR JSON byte-exact vs the Rust
  oracle; RFC-0011 codegen byte-exact on the frozen corpus; refusals for
  WIT-surface records and every out-of-subset shape.
- **EC-4 measured consumers:** the frozen column-shape census tables for
  the selfhost corpus and the application-profile corpus (§2), with the
  projected authored-size reduction recorded before implementation.
- **EC-5 machine matrix:** new support-matrix rows + validator extension;
  the undeclared check/build/run gap count stays zero.

## 8. Evidence classes

All corpus evidence is `internal-fixture`; AI-generation-noise claims are
`blocked-external-evidence` until owner-credentialed live-model
re-measurement (M23 §6 rule carries). No platform claim arises from this
RFC.

## 9. Risks

- **Trusted-kernel surface.** Records touch parser, HIR, semantics, IR,
  verifier, codegen and formatter in the Rust oracle; the EC-1..EC-5
  chain and the dual-implementation register are the mitigation.
- **Scope bleed.** The minimal set is deliberately small; every "just add
  X" (methods, patterns, mutable fields) re-enters through its own RFC.
- **Re-baseline slippage.** The re-convergence on records could underperform
  the STEP-0266 SOA baseline; ADR-0017's revisit condition covers this with
  an explicit continue/rollback decision after three R-reviews.
