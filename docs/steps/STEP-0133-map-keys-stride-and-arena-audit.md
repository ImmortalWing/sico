# STEP-0133: arena allocation audit and the `map.keys` stride defect

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4; owns handoff item
>   `docs/reports/step-0131-defect-forensics-and-handoff.md` §2.6)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); STEP-0131 collection runtime
>   representation (`crates/sico-codegen-wasm/src/stdlib.rs`)

## 1. Scope

The STEP-0131 forensics report handed off §2.6: *enumerate every
`alloc(count * w)` arena site and prove non-zero headroom or non-escape.*
This STEP performs that audit over every `$alloc` caller in
`sico-codegen-wasm` (stdlib.rs, json.rs, lib.rs) and fixes the one real
defect it exposed.

## 2. Defect found and fixed: `map.keys` returned the raw entries table

- **Symptom (baseline reproduction)**: a probe putting `"alpha"->1`,
  `"beta"->2` into a `Map[Text,I64]` and joining `map.keys` printed
  `alpha|` — element 1 was an empty garbage pair. `set.to_list` on the same
  probe printed `gamma|delta` correctly. Baseline artifact:
  pre-fix `probe.component.wasm` run through `sico-runner` (2026-09-06).
- **Root cause**: `emit_collection_to_list` returned the map's entries
  table unchanged as a `List[Text]` table. A map entry is 16 bytes
  (8-byte key slot + 8-byte value slot) while `List[Text]` elements stride
  8 bytes (`list.get` computes `table + index * 8`). Element 0 reads the
  first key correctly by accident; element `i >= 1` reads the value slot of
  entry `i - 1` as a `(ptr, len)` pair. A set entry IS one 8-byte key slot,
  so the passthrough was (and is) exact for `set.to_list` — the helper was
  shared between both operations and only correct for one.
- **Why STEP-0131 missed it**: the frozen corpus exercised `map.keys` never
  at all — the traversal helpers had zero end-to-end coverage, the same
  blind spot as defect 1.1 (element contents untested).
- **Fix**: `map.keys` now compacts the key pairs into a fresh arena table
  (`alloc(count * 8)`, stride-16 loads into stride-8 stores). `set.to_list`
  keeps the zero-copy passthrough, now with the stride argument written
  down in the doc comment.
- **Verification**: the same probe rebuilt post-fix prints
  `alpha|beta/gamma|delta`; the new e2e corpus below pins it.

## 3. Audit table: every `$alloc` site

`$alloc` itself (`lib.rs::emit_alloc`) rounds the heap top up to 8,
advances by `size` with wrap and `ARENA_LIMIT` checks that trap, and for
`size == 0` returns the aligned heap top **without advancing** — so a
zero-size block is only safe when it is never stored to and never read
beyond length 0. Verdicts:

| site | size expression | zero possible? | stored when zero? | verdict |
| --- | --- | --- | --- | --- |
| `text/bytes.concat` | `a_len + b_len` | yes | no (two 0-byte `memory.copy`) | proven empty-safe |
| `u64/i64.to_text` scratch | `24` | no | — | safe |
| `u64/i64.to_text` out | `len >= 1` (zero value returns early) | no | — | safe |
| `list.append` | `(count + 1) * 8` | no | — | safe (headroom) |
| `text.join` empty-list path | `0` | yes | no (returns `(ptr, 0)`) | proven empty-safe |
| `text.join` main path | `total` | yes (all-empty elements, empty separator) | writes exactly `total` bytes | proven empty-safe |
| `split_lines` / `split_words` | `count * 8` | yes (`count == 0`) | no (fill loop and tail/final-token stores are guarded) | proven empty-safe |
| `map.put` / `set.add` | `(count + 1) * entry_width` | no | — | safe (STEP-0131 fix) |
| `map.keys` (this STEP) | `count * 8` | yes (`count == 0`) | no (copy loop exits immediately) | proven empty-safe |
| `set.to_list` | no allocation | — | — | zero-copy, stride exact |
| `json.quote` | `out_len >= 2` (surrounding quotes) | no | — | safe |
| `json.find` / `json.scan` | no allocation | — | — | safe |

Aggregate producers with no other `List[Text]` source exist: the language
has no general `List[Text]` literal expression (list literals appear only
in `collect_tasks`, a different task-handle form), and host-provided lists
(`input.arguments`) are lifted by the canonical ABI outside the arena
bump allocator. No further `alloc(count * w)` sites exist.

## 4. End-to-end evidence

`tests/end-to-end/map-keys-traversal.sico` — guest-side assertions with
distinct failure messages, printing `traversal-ok` on success:

- `map.keys[Text,I64]` over a 3-entry map: exact join order
  (`starts_with("alpha|beta|gamma")`) and exact length (16);
- content round-trip: the materialized key at index 1 is read back with
  `list.get` and used for `map.has`/`map.get` content lookups (exercises
  the byte-content key comparison from STEP-0131 defect 1.3);
- an interleaved `text.concat` arena allocation between `map.keys` and its
  use (regression guard for the defect-1.2 aliasing class);
- empty-map `keys` (count 0) and singleton-map `keys` (index 0 only, the
  case that accidentally worked pre-fix);
- `set.to_list[Text]` over a 3-element set with the same order, length,
  and round-trip assertions.

Runner coverage: `runner/sico-runner/tests/map_set.rs` gained
`map_keys_and_set_to_list_preserve_element_contents` (deterministic
result, repeated-run isolation).

## 5. Residuals (handoff items still open)

- §2.5 differential/property testing against a Python oracle remains open;
  this STEP only closed the coverage hole for traversal-helper contents.
- §2.7 parallel-load flake class (`--test-threads=1` workaround) is
  untouched by this STEP.
- §2.1–§2.4 (recursion budget, unknown-callee check diagnostic, first-class
  tasks, unbounded `Int`) are feature/RFC work, not bugs.

## 6. Validation

- Baseline reproduction: pre-fix `probe.component.wasm` prints
  `alpha|/gamma|delta`; post-fix artifact prints `alpha|beta/gamma|delta`.
- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test map_set -- --test-threads=1`: 4/4 green.
- Root workspace `cargo test --locked --offline --workspace --all-targets --all-features` green (frozen core/flow-lowering snapshots byte-identical — no frozen shape contains collection helpers).
- `cargo fmt --all -- --check` clean; `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` clean.
- `tools/validate-step-0131.ps1` green (matrix surface unchanged — the fix
  makes the already-declared `map.keys` row true); `git diff --check` clean.
