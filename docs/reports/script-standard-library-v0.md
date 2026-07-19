# Minimal useful script standard library evidence

> - date: 2026-07-18
> - step: STEP-0083
> - status: complete

## Implemented boundary

The Script v0 standard library is a closed, versioned intrinsic registry pinned identically in three layers — IR (`sico_ir::intrinsic_signature`), semantics (`infer_stdlib_call`) and the backend (`emit_intrinsic`). Unknown names are rejected; every signature is frozen.

Pure-computation intrinsics lower to deterministic helper core functions emitted into the program module, reusing the STEP-0079 bounded arena and checked arithmetic (`crates/sico-codegen-wasm/src/stdlib.rs`, `src/json.rs`):

- text: `length`, `concat`, `trim`, `contains`, `starts_with`, `split_lines`, `split_words`, `replace`, `join`, `encode`;
- bytes: `length`, `concat`, `slice` (checked, `Result[Bytes, NumericError]`), `is_utf8`, `utf8_decode` (passthrough guarded by `is_utf8`; a strict DFA validator backs the check);
- list (List\[Text\]): `length`, `get` (checked), `append`;
- `sico.u64.to_text` / `sico.i64.to_text`;
- JSON: `is_valid`, `has`, `get`, `quote` — a bounded guest-side scanner (depth ≤ 32, strict string escapes, strict numbers). Number-precision contract: JSON numbers are only scanned, never evaluated, so no precision is lost; `get` returns the raw JSON token text (a string value keeps its quotes).

Language machinery added for real consumption of these intrinsics:

- Script helper (non-`run`) functions accept and return aggregates through the flattened Canonical ABI (`flat_ir_types`), so helpers like `lookup(document: Text) -> Result[ScriptOutput, ScriptError]` compose.
- Script `Result` values occupy the full flat width (tag + ok payload + error payload); the inactive side is zero-filled at construction, and the `run` boundary selects the payload record from the runtime tag.
- `case ok(x)` / `case error(x)` match payload binding lowers to a `Project` over synthetic `ok`/`error` fields; IR values stay block-scoped, so the match subject must be a function parameter (the verifier still rejects cross-block references; non-parameter subjects keep the typed `unsupported match payload binding` refusal).

## Scoped filesystem channel

`sico.fs` is a capability, split exactly along RFC-0029 `storage.read` / `storage.write`:

- WIT (`wit/script-profile-v0/world.wit`): `sico:script/fs-read@0.1.0` (`read: func(string) -> result<list<u8>, string>`, `exists: func(string) -> result<bool, string>`) and `sico:script/fs-write@0.1.0` (`write: func(string, list<u8>) -> result<_, string>`), imported by the `program` world.
- Codegen: used `sico.fs.*` intrinsics become Component imports lowered through a per-program transport module that owns the 64 MiB memory and the single bump allocator (`heap_base` above the static data segment). Programs without fs intrinsics keep the previous self-contained layout byte-identically (snapshots unchanged).
- Runner (`runner/sico-runner`): both instances are always linked so fs components launch; every call re-checks `FsGrants` and fails closed with a typed message. Roots are canonicalized once at startup (`--fs-read-root`/`--fs-write-root`, repeatable); guest paths must be relative, ≤ 4 KiB, without parent components, drive/verbatim prefixes, colons or backslashes, and canonical containment is verified against the granted root (symlink escapes denied). Files and payloads stay inside the 8 MiB channel budget. `sico run` forwards the same flags.
- Package closure: `sico:script/fs-read@…` maps to `storage.read`, `sico:script/fs-write@…` to `storage.write` in `capabilities_for_imports`; both are supported grant names.

Deviation from the step-0083 draft architecture: two interfaces (`fs-read`/`fs-write`) instead of one `sico:script/fs@0.1.0`, so the RFC-0029 read/write capability split is expressible at import granularity. `sico.fs.list` was cut from v0 (the four pilots do not need it; the interface version allows adding it later).

## Evidence

`tools/validate-step-0083.ps1`:

- args echo: `sico run script-args-echo.sico -- alpha beta 雪` → stdout `alpha beta 雪`, exit 0;
- word count: seven words over two lines → `7`, padded single word → `1`, invalid UTF-8 stdin → domain `invalid-input` exit 122;
- JSON filter: `{"name":"sico","n":42}` → `"sico"` exit 0 (also with trailing CRLF stdin — `is_valid` skips surrounding whitespace and short-circuits a failed scan); `{"other":1}` → domain `missing name field` 122; `{bad` → domain `malformed JSON` 122;
- file transform: read root `input.txt` → write root `output.txt` containing `words: 5`, stdout `transformed`, exit 0;
- fs refusals (all typed domain errors, exit 122, process survives): no grants → `storage.read not granted`; read-only grant → `storage.write not granted`; missing file → `io: not found below any granted read root`; `../secret.txt` read → `storage.read: path escapes the granted scope`; `../escape.txt` write → `storage.write: path escapes the granted scope` (no file created);
- full workspace regression: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace --all-targets --all-features` (67 suites) green; `sico-runner` release test suite (MSVC) 6/6 green; STEP-0079–0082 validators still pass.

## Honest limits

- `sico.json.get` returns the raw token text; typed number/boolean accessors are later work (the number contract is scan-only).
- Multiple read roots resolve in grant order (first containing root); writes go to the first write root. No `sico.fs.list`, no streaming file IO (M9).
- `case ok(x)` binding requires the match subject to be a function parameter; nested match statements inside one arm and multi-statement arms stay refused (helpers flatten the control flow, as the pilots show).
