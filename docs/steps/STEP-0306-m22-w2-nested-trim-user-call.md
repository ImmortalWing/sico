# STEP-0306: M22 W2 slice 14 — nested trim argument in text.concat

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 14
> - date: 2026-09-28
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0305 left `normalize_source` at `ERR:E-SH-IR-CALL-ARGUMENT` inside
`sico.text.trim(item(tokens, add(comment_index, U64.literal(1))))`, itself
the second argument of `sico.text.concat`. ADR-0017 Option A and RFC-0047
remain accepted. The W1 independent CI gate was satisfied on `e8495f9`;
the W2 baseline is STEP-0282. This slice changes only `selfhost/parser.sico`,
the corresponding runner differential, validator and M22 status documents.
Its exit test is a byte-exact Rust IR differential for this nested expression,
with a typed wrong-return refusal and recovery in a fresh Store. It does not
enter S5, S6 or S7.

The bounded lowering recognizes one declared Text-returning user call as the
sole argument to `sico.text.trim`, when trim is one `sico.text.concat`
argument. It validates the complete delimiters and the declared return
kind, emits nested arguments, user call and trim in evaluation order, and
checks SSA ID arithmetic. Other expressions remain typed-refused.

## Evidence and verdict

- The real-runner fixture exercises `trim(item(tokens, add(comment_index,
  U64.literal(1))))` inside concat and compares the entire canonical IR
  byte-for-byte with Rust. A wrong return kind yields
  `ERR:E-SH-IR-CALL-TYPE`; the valid fixture succeeds after that refusal.
- A temporary diagnostic probe, removed from the delivered code, located
  the next `normalize_source` refusal at the `output` binding. The stable
  code is `ERR:E-SH-IR-GWPACK-OTHER`, caused by the later `sico.list.append`
  RHS. The formatter canary remains 28/30, full-source exit 122. Since the
  refusal site advanced, the R3 consecutive-stall count remains **0**.
- `tools/validate-step-0306.ps1` and a fresh
  `tools/report-m22-canary.ps1` are the reproducible local checks. Root and
  runner formatting checks, `tools/validate-step-0124.ps1 -SelfTest`, and
  `git diff --check` cover formatting and planning links. Exact run results
  are recorded below after execution.

| Check | Result |
|---|---|
| `tools/validate-step-0306.ps1` | exit 0; 38/38 compiler tests, 2/2 parser tests, 1/1 local-bounds test, inherited STEP-0262/0302–0305 checks green |
| `tools/report-m22-canary.ps1` | exit 0; 28/30, `normalize_source` / `ERR:E-SH-IR-GWPACK-OTHER`, full-source exit 122; 5,000,000,000 fuel cap |
| root/runner `cargo fmt --all -- --check` | both exit 0 |
| `tools/validate-step-0124.ps1 -SelfTest`; `git diff --check` | both exit 0; negative roadmap cases 5/5 |
| independent exact-SHA CI | not run; no commit/push authorized |

M22 remains **NO-GO**. The `normalize_source` tail, `main`, remaining
selfhost sources, S5 byte-equal codegen, S6 `A == B == C` bootstrap and S7
exit audit are open. No Component self-bootstrap or package claim follows
from this IR fixture.
