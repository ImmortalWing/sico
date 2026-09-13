# STEP-0149: M15 clean-room consumer — pilots/log-analyzer

> - status: complete
> - phase: M15 prerequisite track exit test + M15 entry-condition-3 evidence (owner decision 2026-09-07: condition closed by clean-room-consumer evidence, not re-scoped)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - artifacts: [`pilots/log-analyzer/`](../../pilots/log-analyzer/) (12 Sico sources, 9 `sico test` pairs, lock + 2 signed packages); generator `crates/sico-cli/tests/generate_pilot_packages.rs` (#[ignore], explicit)

## 1. What was done

The frozen STEP-0142 §5 application, delivered at clean-room-consumer
class: a CSV log analyzer with own modules (`main` entry, `app`
pipeline, `parse` CSV/column handling, `report` rendering) consuming
`csv@1.parse-line` and `table_stats@1.aggregate` via `use pkg … expose …`
under `sico-lock.json`. Behaviour: per-column count/min/max/mean
(fixed-point ×1000, floor) over the column named in `arguments[0]`;
deterministic text report; typed fail-closed exits (10 malformed row,
11 unknown column, 12 empty input, 13 oversize input > 1 MiB).

No compiler/Runtime patches: the application builds and runs on the
landed STEP-0143/0144/0147 surface exactly as published — modules,
user-WIT interfaces, package resolution through the lock, signed
artifacts, and the real Component Runtime with the package bridge.

Same-author limitation (recorded per the owner decision): this class
proves independent code-path pressure against the published docs, not
independent third-party validation.

## 2. Validation

- `sico test pilots/log-analyzer`: **9/9 byte-exact** (happy basic,
  quoted fields with `""` escapes, CRLF, malformed row at a named index,
  unknown column, empty input, oversize input, negative mean floor
  ([-7,3] → mean_x1000=-2000), non-numeric column → indexed
  malformed-number error).
- Package artifacts regenerate deterministically via the #[ignore]
  generator test; digests recorded in `sico-lock.json`.
- Workspace-diff statement: no file outside `pilots/log-analyzer/` plus
  the STEP-0147 surface was touched for the application itself.

## 3. Declared limits

- M15 entry condition 3 is closed **as evidence for the §3.0 track exit
  test**; the honest class label (clean-room-consumer, same-author)
  travels with every claim.
- The 8 MiB Script stdin bound and the 1 MiB application bound are
  separate; oversize fixtures exercise the application bound.
