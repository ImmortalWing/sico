# STEP-0129: M14 inventory, application profile RFC and support matrix

> - status: complete (RFC-0038 accepted by owner 2026-09-05)
> - phase: M14 (first step per M14 plan §7: inventory/RFC before code changes)
> - started: 2026-09-05
> - completed: 2026-09-05
> - owners: autonomous-agent
> - artifacts: [`RFC-0038`](../rfc/RFC-0038-application-profile-v0.md)

## 1. What was done

The pre-code-change inventory M14 plan §7 requires: every accepted source
construct classified against real check/build/run behavior, a
machine-readable gap list, and the RFC freezing the application profile
and the exit corpus.

## 2. Key measured findings (probes + source evidence)

Live probes through `sico check/build/run` (2026-09-05, sico 0.0.2-dev,
Windows x64):

1. **recursion passes check** (a self-recursive `sum_to` over `I64` is
   accepted by syntax + semantics) but is unusable today: fixed-width
   arithmetic must go through `I64.checked_add/checked_sub` (returning
   `Result[I64, NumericError]`), and unwrapping those results requires
   nested match — which build refuses.
2. **the single biggest blocker**: `unsupported non-return match arm` —
   every match arm must be a `return`, so mid-function branching,
   accumulation across branches, and loop-shaped recursion are all
   impossible. `while`/`for`/`if` are not in the grammar at all.
3. the codegen handles exactly the straight-line Operation set
   (`Call/Construct/Variant/Project/Intrinsic/CheckedAdd/CheckedSub/
   EqualFixed/LessFixed` + task-scope ops) with 126 typed-refusal sites;
   the stdlib is 36 intrinsics (text/bytes/json/fs/http/streams).
4. `I64.less`/`I64.sub`/`I64.add` are correctly refused (`unsupported
   call target`) — the fixed intrinsic surface is exactly
   `checked_add/checked_sub/equal/less_than`.

Classification, application profile and exit corpus are frozen in
RFC-0038 §1–§3.

## 3. Probe reproduction

```text
sico check  <probe>   # recursion accepted: "check ok"
sico build  <probe>   # "unsupported non-return match arm" / "unsupported call target I64.less"
```
Probes live in the session temp directory (not committed); the refusal
classes above are durable compiler behavior, re-verified in the STEP-0130
fixtures that will land with the first gap-closing STEP.

## 4. Acceptance-project decision (recorded)

`案例项目/俄罗斯方块消除` stays the primary M14 oracle; RFC-0038 §3.1
freezes the strengthened corpus (JSON-boundary fixtures against the Python
oracle, source-level seeded PRNG, typed max-nodes exhaustion, bitboard
variant). Alternatives evaluated and rejected with reasons in RFC-0038.

## 5. Validation

Document-only step: inventory evidence gathered from compiler sources and
live probes; no production code changed. `git diff --check` clean. The
RFC proceeds to owner acceptance before any STEP-0130 code change.
