# STEP-0295: M22 W2 S3 — nested if join in a match arm

> - status: complete / 22-C S3 GO on the isolated branch after independent CI
> - phase: M22 compiler self-host, execution card 22-C S3
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0294 gave S2 GO on the isolated `codex/m22-w1-ci` branch. The formatter
canary was 22/30; `nearest_match` refused with `ERR:E-SH-IR-STATEMENT`.
STEP-0291 predeclared the next shape: `match sico.map.get[Text,U64]` inside a
while body, with a conditional `return` inside the `ok` arm and a self-set in
the `error` arm. No language, WIT, Host or authority contract changes.

The existing general-while lowering created a join block for the nested `if`,
then attached `end match` to the *entry* of the `ok` arm. The `if` join remained
unterminated (real-runner diagnostic at block 5). The match frame now records
the current `ok` tail when `case error` opens; `end match` connects that tail
to the match join. The original `ok` entry remains the match arm target.
The change adds no new local to the 240-local general-while function.

## Evidence and limits

- The real-runner differential compares the formatter prefix through
  `nearest_match` with the Rust canonical IR byte for byte. A statement after
  the conditional `return` is a typed `ERR:E-SH-IR-STATEMENT` refusal, followed
  by successful compilation of the valid prefix in a fresh Store.
- `tools/report-m22-canary.ps1`: **23/30** formatter functions, first uncovered
  `set_nearest_match`, `ERR:E-SH-IR-STATEMENT`, full-source exit 122.
- `tools/validate-step-0262.ps1`: compiler 24/24, parser 2/2, local bounds
  1/1, historical 22-function snapshot and the 23rd function prefix passed.
  The prefix through `set_nearest_match` remains a typed refusal.
- `tools/validate-step-0261.ps1`: compiler 24/24, parser 2/2, local bounds
  1/1, historical 17-function snapshot and the current full-source typed
  frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 24 and runner clippy passed.
- Root `cargo fmt --all -- --check`, `tools/validate-step-0124.ps1` and
  `git diff --check` passed.
- The next `set_nearest_match` shape and the remaining selfhost sources are
  not covered by this STEP. S5 byte-equal codegen, S6 bootstrap and S7 audit
  remain open. M22 remains **NO-GO**.

The canary numerator grew by one, so the R3 consecutive-stall count remains
**0**. The exact implementation and evidence commit
`f7288b753a7444fa61d23bb88c52a215564bdd8f` passed [GitHub Actions run
36166445685](https://github.com/ImmortalWing/sico/actions/runs/36166445685)
on the fresh Windows GNU runner at 2026-09-25 17:38:31 UTC. Its repository
CI step succeeded. **22-C S3 GO on the isolated branch.** M22 remains NO-GO.
