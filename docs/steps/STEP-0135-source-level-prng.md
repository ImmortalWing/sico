# STEP-0135: source-level seeded PRNG

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); profile item 7 (deterministic
>   pseudo-randomness as a source-level helper — never ambient randomness)

## 1. What was implemented

Profile item 7 needs no compiler surface at all: with STEP-0132 bit
operations a seeded PRNG is expressible as ordinary Sico source. This STEP
proves that claim end to end and pins the sequence so the proof cannot
silently rot.

- `tests/end-to-end/prng-xorshift.sico`: a xorshift64 generator
  (`x ^= x << 13; x ^= x >> 7; x ^= x << 17` on `U64` via
  `bit_xor`/`shl`/`shr`) iterated 8 times from the canonical seed
  88172645463325252 inside a `while` loop, with every output compared
  guest-side against a pinned reference vector.
- Reference vector generated once from the equivalent Python
  implementation (mask-to-64-bit after each shift):

  ```
  8748534153485358512, 3040900993826735515, 3453997556048239312,
  16431732851926010853, 8204724074003728306, 17801246309558322749,
  7041795614029497201, 16736801589742238903
  ```

- No ambient randomness is added to any surface; the runner and Host stay
  deterministic. The same helper shape is what the M14 solver port will
  embed for its seeded rollout (RFC-0038 §3.1).
- Matrix: `tests/language-matrix/application-profile-v0.json` gained the
  `source-prng-xorshift64` executable row.

## 2. End-to-end evidence

Fixture compiles with `sico build --profile script-v0` and runs through
the real runner printing `prng-ok`. Covered by
`runner/sico-runner/tests/prng_xorshift.rs` (pinned sequence +
repeated-run isolation).

## 3. Residuals

- Range reduction (`next % n`) for the solver will compose
  `checked_div`/`checked_mul`/`checked_sub` (STEP-0134) — no remainder
  intrinsic is planned for the profile.
- xorshift64 is a non-cryptographic PRNG; the profile forbids ambient or
  cryptographic randomness claims for it.

## 4. Validation

- Fixture build+run via real CLI/runner: `prng-ok`.
- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test prng_xorshift -- --test-threads=1`: 2/2 green.
- Root workspace full `cargo test --locked --offline --workspace --all-targets --all-features` green (331 passed, 0 failed); `cargo fmt`/`clippy -D warnings` clean.
- `tools/validate-step-0131.ps1` green; `git diff --check` clean.
