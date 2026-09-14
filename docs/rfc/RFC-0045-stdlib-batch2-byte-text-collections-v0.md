# RFC-0045: Standard-library batch 2 — byte/text access, collections, formatting v0

> - status: accepted
> - accepted: 2026-09-14 (owner session directive 「先完成M21和自举吧」 on the presented D1–D4 decision points)
> - date: 2026-09-14
> - phase: M21 §3.1 / M22 S0 (shared entry prereq — this RFC is both milestones' first slice)
> - depends: RFC-0038 (application profile), RFC-0044 (language v1 batch 1), STEP-0083 (stdlib helpers), STEP-0131 (collection family)

## Summary

Freeze the second standard-library batch: byte-level access, text
comparison/indexing, deterministic list ordering, map values and template
formatting. All operations are pure, deterministic, guest-side computed
(emitted wasm helpers — no world/import change), and take the frozen
Script profile value set as-is. The tetris board reader's ASCII-mask
workaround (STEP-0166 decision point, M21 exit gate 1) is the named
retirement target.

## 1. Frozen surface

| Intrinsic | Signature | Semantics |
|---|---|---|
| `sico.bytes.at` | `(Bytes, U64, I64) -> I64` | byte at index, or the third argument when `index >= length` (D1 as amended at implementation: total function — the packed `Result[U64, NumericError]` match shape was found unexercised by every frozen corpus and is deferred to the list-element RFC) |
| `sico.bytes.equal` | `(Bytes, Bytes) -> Bool` | content equality (length + byte loop; never pointer identity) |
| `sico.text.compare` | `(Text, Text) -> I64` | byte-order lexicographic: `-1`/`0`/`+1` (UTF-8 byte order = code-point order) |
| `sico.text.char_at` | `(Text, U64) -> Result[Text, NumericError]` | the 1-char `Text` at char index (lead-byte count = `sico.text.length` semantics); `overflow` when out of range |
| `sico.text.format` | `(Text, List[Text]) -> Text` | `{N}` → `args[N]`; `{{`/`}}` → literal braces; any other `{…}` sequence is copied verbatim (D3) |
| `sico.list.sort` | `(List[Text]) -> List[Text]` | ascending byte-order, stable insertion sort |
| `sico.list.min` | `(List[Text], Text) -> Text` | minimum, or the second argument when empty (D2: total function, no error ceremony) |
| `sico.list.max` | `(List[Text], Text) -> Text` | maximum, or the second argument when empty |
| `sico.map.values[Text,Text]` | `(Map[Text,Text]) -> List[Text]` | values in first-insertion key order (same order `sico.map.keys` reports) |

## 2. Decision points (resolved at acceptance)

- **D1 — index-error type (as amended at implementation, 2026-09-14)**:
  `bytes.at` ships as a total function `(Bytes, U64, I64) -> I64` with an
  explicit fallback argument (same shape as D2's `min`/`max`). The
  originally proposed `Result[U64, NumericError]` match shape is the same
  class as the A6 checked-Result seam — matching a fixed-width-ok Result
  was exercised by no frozen corpus and its lowering needs its own
  defect STEP; `bytes.at` must not gate on it. `text.char_at` keeps the
  `Result[Text, NumericError]` shape (the Text-payload 4-slot form is
  proven by `list.get`/`bytes.slice`).
- **D2 — `min`/`max` on empty input**: total functions with an explicit
  default argument instead of a Result. `NumericError` has no honest
  "empty" case (its three cases are frozen by RFC-0038), and a caller
  paying the match ceremony per min/max defeats the DX goal.
- **D3 — malformed `format` templates**: documented deterministic
  pass-through (unknown or garbled `{…}` sequences are copied verbatim).
  This is specified behavior, not a silent discard; no Result ceremony on
  the hottest string API.
- **D4 — `map.values` instantiation**: v0 freezes `[Text,Text]` only.
  `List[I64]` is not an executable type yet, so `[Text,I64]` and friends
  are typed build refusals until the list-element extension RFC lands;
  `entries` (key+value pairs) needs record-in-list and stays proposed
  with a typed refusal.

## 3. Compatibility

- Additive only: no existing intrinsic changes behavior; single-module
  programs compile byte-identically unless they call the new names.
- No world/WIT/import change: every operation is guest-side computed.
- Grammar unchanged: no new syntax (calls use the existing
  `sico.*`/bracket-generic forms).

## 4. Exit corpus (binding when accepted)

1. e2e content test exercising every frozen intrinsic with byte-exact
   stdout (including empty-list defaults, `{}` escapes and verbatim
   pass-through cases).
2. Refusal: `sico.map.values[Text,I64]` fails closed at check/build with
   the typed unsupported-shape error (limit+1 style).
3. Matrix: `stdlib-batch-2` rows in `tests/language-matrix/application-profile-v0.json` + validator extension.
4. Retirement: the tetris board reader consumes `sico.bytes.at` (or the
   corpus documents why the existing shape already satisfies gate 1) —
   the M21 exit-gate-1 item is tracked to its own STEP either way.
5. Determinism: repeated builds byte-identical (snapshot discipline).

## 5. Non-goals

- `List[I64]`/`List[U64]` element extension, records in lists,
  `map.entries` — the follow-up collection RFC (M22 S0 continuation).
- Float/Decimal formatting, locale-aware collation, unicode case tables.
- Sorting maps in place (collections are functional/copy-on-write).
