# STEP-0139: streaming transform and capability-backed state machine

> - status: complete
> - phase: M14 (RFC-0038 §3.2 companion acceptance applications 2 and 3)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); M14 plan §4 applications 2–3

## 1. Streaming transform (`tests/end-to-end/stream-transform.sico`)

A stdin→stdout line-numbering transform written in pure Sico source:

- **Bounded memory**: reads fixed 4 KiB chunks (`sico.stream.read`), so
  guest memory per iteration is the chunk plus its numbered line table,
  independent of the total input size. The runner test drives 1 MiB
  (256 × 4096-byte lines) through the default runner budgets.
- **Typed partial failure**: a non-UTF-8 chunk fails with
  `DomainError("bad utf8 at chunk N")` — exit 122, class `domain-error` —
  after earlier chunks were already emitted, demonstrating typed partial
  failure rather than a trap.
- **Cancellation**: every stream read/write routes through the host's
  `send_cancellable`/`recv_cancellable` (RFC-0030), so the epoch/CancelToken
  machinery cancels mid-pump with the typed `Cancelled` outcome (covered by
  the existing typed-cancellation fixtures; the same host path).
- **Deterministic**: no ambient state; output is a pure function of input.
- **Capability discipline**: the pump passes its output-stream handle down
  explicitly. The first draft created `sico.stream.stdout()` per chunk and
  was correctly refused by the host's live-stream ceiling
  (`resource-limit: too many live stream handles`) at chunk ~62 — the
  least-privilege cap caught the leak, and the fixture now reuses one
  handle.

## 2. Capability-backed state machine
(`tests/end-to-end/capability-state-machine.sico`)

A persisted three-state machine (`sealed` / `open` / `burned`):

- **Explicit external effect**: the state document persists through the
  granted scoped `sico.fs.write` capability; without a write root every
  write fails closed.
- **Revision guard**: the document carries a unary revision counter; every
  command carries the caller's expected revision, and a length mismatch is
  a typed `stale revision` refusal before any transition.
- **Typed transitions**: `open` (sealed→open), `seal` (open→sealed),
  `burn` (open→burned); illegal transitions are typed `DomainError`s and
  leave the document untouched.
- **Deterministic recovery**: a missing, non-UTF-8, invalid-JSON, or
  field-incomplete document deterministically recovers to the initial
  state and then applies the command against the recovered state.
- Host note: the frozen `fs.exists` surface reports a missing file as a
  typed read error; the fixture treats that error as "absent" and
  recovers.

## 3. Frontend defect fixed: string escape decoding

The lexer accepts `\" \\ \n \r \t \0` escapes, but `unquote`
(`sico-ir/src/lower.rs`) decoded only `\"` and `\\` — `\n` in any source
string silently stayed as two literal characters. `unquote` now decodes
the full lexer-accepted set. No frozen fixture used the affected escapes
(frozen snapshots are unchanged).

## 4. Defects found and fixed in existing helpers (e2e firsts)

Both streaming-app defects are pre-existing STEP-0083-era helpers whose
element contents had never been e2e-verified — the exact blind spot class
the STEP-0131 forensics predicted:

1. **`text.split_lines` continue-branch depth**: the non-LF branch's
   `br 0` targeted the `if` label instead of the loop, so every input
   byte stored a (one-byte) line and wrote past the table into the arena
   (silent corruption). Fixed to `br 1`. Symptom: `"alpha\nbravo\n"`
   split into `a`,`p`,`a`,`b`.
2. **`split_lines` inverted guards**: the final-LF count decrement and
   both CR-strip guards tested `len == 0` where `len != 0` was required,
   so trailing-LF documents over-counted lines by one and CR stripping
   never fired.

Both are covered by the new e2e corpus: the transform's byte-exact 1 MiB
round-trip exercises multi-line chunks and the line/table contents
directly.

## 5. Validation

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test stream_transform -- --test-threads=1`: 2/2 green (1 MiB round-trip; typed partial failure).
- `--test capability_state_machine`: 2/2 green (transitions/guard/recovery; determinism).
- Root workspace tests green; runner workspace serial suite green; fmt/clippy `-D warnings` clean; `validate-step-0131.ps1`/`validate-step-0138.ps1` green; `git diff --check` clean.
