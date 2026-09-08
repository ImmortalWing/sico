# STEP-0131: dynamic collections (insertion-ordered Map/Set)

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-05
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); profile items 4 (dynamic
>   List/Map/Set traversal and construction with deterministic order), 5
>   (fixed-width comparisons), 6 (user types across function and collection
>   boundaries)

## 1. What was implemented

The RFC-0038 application profile's collection slice, end to end:

- **IR types**: `Type::Map { key, value }` and `Type::Set(element)`
  (insertion-ordered map/set; functional copy-on-write; first-insertion key
  order; a re-put keeps the original entry position and overwrites the value).
- **Canonical intrinsics**: `sico.map.empty/put/get/has/length/keys` and
  `sico.set.empty/add/has/length/to_list`, each spelled with an explicit
  element-type suffix — `sico.map.put[Text,I64](m, k, v)`,
  `sico.set.add[Text](s, k)` — parsed by `collection_intrinsic` in `sico-ir`
  (closed registry: 11 operations × 5-element instantiations). The registry
  accepts every well-formed instantiation; the check-time executable surface
  is restricted (see refusals below) and mirrors the registry parser in
  `sico-semantics` (the language module may not depend on the compiler
  module; the duplication is documented at both sites).
- **Lowering**: `map.empty`/`set.empty` are zero-argument intrinsics;
  suffix composition needs no synthesis — the source spells the canonical
  name directly and `intrinsic_signature` resolves it for the verifier.
- **Semantics**: `infer_collection_call` types every canonical instantiation
  and enforces the v0 executable surface: `map.get` materializes only
  fixed-width values (`Result[I64|U64, NumericError]` two-slot checked
  form), traversal helpers (`map.keys`, `set.to_list`) materialize only
  `List[Text]` (their key material doubles as the list table); everything
  else falls through to the unknown-callee diagnostic — no undeclared
  check/build gap.
- **Codegen**: `Map`/`Set` values are the `(entries pointer, count)` pair —
  identical shape to a `List[Text]` table. Entry slots are 8-byte keys plus,
  for maps, 8-byte values (`Text`/`Bytes` as canonical `(ptr, len)` pairs,
  scalars as `i64`). Each monomorphized helper allocates room for one MORE
  entry than the source so an empty map/set never starts from a zero-size
  arena block (a zero-size block would be overwritten by the next
  allocation). `Result[Text|Bytes, NumericError]` gained a four-slot local
  form (tag, ptr, len, error tag) so `list.get`/`map.get` results can spill
  into cells. Helpers compare keys by BYTE CONTENT (Text/Bytes), not pointer
  identity — dynamic Text built at runtime never shares addresses even when
  its contents match.
- **Discovery fix (pre-existing stdlib defect)**: `text.split_words` closed
  tokens under an INVERTED in-token condition (`Eqz` on `in_token`), so its
  output table was never filled — element lengths always read as 0 through
  `list.get`/`text.join`. Element contents were never e2e-executable before
  STEP-0131 (the `Result[Text, NumericError]` spill cell was refused at
  build), so the defect was invisible to the existing corpus (which only
  exercises `list.length`). Fixed by removing the `Eqz`; the map/set fixture
  covers element traversal end to end.
- **General-CFG bindings**: `let` in a general-CFG function now always
  lowers to a mutable local cell (IR values are block-scoped; a binding read
  in a later block must live in a cell, whether or not the source reassigns
  it). Straight-line bodies keep plain bindings, so frozen shapes are
  unchanged. Match payload bindings (`case ok(word)`) also spill into
  compiler-generated cells so arms can read them after the join.

## 2. End-to-end evidence

`tests/end-to-end/map-set-frequency.sico` — a pure-Sico word-frequency
counter: `Map[Text,I64]` counts, `Set[Text]` deduplicates, both inside a
`while` loop driven by `list.get` payloads unwrapped through nested
`match`. Compiles with `sico build --profile script-v0` and runs through
the real runner; the guest asserts distinct-key count, `map.has` hit/miss,
`map.get` value, `set.has` membership and `set.length`, printing the fixed
marker `map-set-ok`. Covered by `runner/sico-runner/tests/map_set.rs`
(3 tests: deterministic result, repeated-run isolation, typed refusal for
the off-surface `map.get[Text,Text]` instantiation at build).

## 3. Machine-readable matrix

`tests/language-matrix/application-profile-v0.json` records the
check/build/run classification per construct (RFC-0038 §1/§2), the frozen
instantiation surface and the declared refusals. Validated on every change
by `tools/validate-step-0131.ps1`, which checks the fixture paths and runs
a registry probe asserting every canonical instantiation resolves in
`sico_ir::intrinsic_signature` while off-surface spellings stay closed.

## 4. Residuals (declared, per the RFC matrix)

- `map.keys`/`set.to_list` materialize only `List[Text]` (Text keys);
  other key elements parse at the registry but are refused at build (typed
  unsupported-call) until non-Text traversal layouts exist (RFC-0038
  proposed list). The matrix records these rows as check-accepted,
  build-refused — a declared mismatch, not an undeclared gap.
- `map.get` with non-fixed-width values (`Text`/`Bytes`/`Bool`) is likewise
  refused at build (two-slot checked-Result form is the only materializable
  miss shape today); matrix row declared accordingly.
- Record/enum keys and values stay proposed (RFC-0038).
- Discovered during fixture work and fixed in this STEP: split_words
  inverted close-condition (above). The defect predates STEP-0131; the fix
  is exercised by the new fixture.

## 5. Validation

`cargo test` root workspace: all green (30 suites; includes the 12 frozen
core-lowering snapshot cases byte-identical, and the new registry probe).
Runner workspace: 37 lib + 1 dap + 3 control-flow + 6 http2 + 37 runner
integration (serial, per the documented process-global-test constraint) +
3 map_set — all green. Under full parallel load four system-level runner
tests (DAP RSS, console control, watch cancel) flaked once and all passed
serially and in isolation; same flake class as STEP-0125-followup,
unrelated to collections.
`cargo fmt` clean; module-boundary validator green.
