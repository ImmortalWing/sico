# pilots/log-analyzer — clean-room consumer (M15 §3.0 exit test)

CSV log analyzer delivered at `clean-room-consumer` class per the M15 plan
§3.0 owner decision (2026-09-07): it closes M15 entry condition 3 by
evidence. It is developed against the published language documentation and
the frozen RFC-0039 package/WIT surface — no compiler or Runtime patches,
no fixture internals. **Same-author limitation:** this class proves
independent code-path pressure, not independent third-party validation.

## Frozen v1 scope (STEP-0142 §5)

- Own modules: `main` (entry), `parse` (CSV row/field handling), `report`
  (aggregation rendering), `app` (pipeline composition).
- Consumed packages: `csv@1` (`parse-line`) and `table_stats@1`
  (`aggregate`) via `use pkg … expose …` under `sico-lock.json`.
- Input: UTF-8 CSV on stdin, ≤ 1 MiB, header row + data rows; quoted
  fields with `""` escapes; CRLF or LF.
- Behavior: per-column `count` / `min` / `max` / `mean` (I64 fixed-point
  ×1000, floor) over the column named in `arguments[0]`; deterministic
  text report on stdout.
- Typed failures (fail closed, fixed exit codes):

| condition | exit |
|---|---|
| malformed row (short row or bad CSV, row index in message) | 10 |
| unknown column / missing column argument | 11 |
| empty input | 12 |
| input over 1 MiB | 13 |

All other errors use the Script profile's own exit semantics.

## Layout

- `main.sico` — CLI entry, delegates to `app.run`.
- `app.sico`, `parse.sico`, `report.sico` — the program modules.
- `t01…t08.sico` + `t01…t08.test.json` — `sico test` fixture pairs
  (byte-exact stdout, fixed exits).
- `sico-lock.json` + `packages/` — the lock projection and the two signed
  package artifacts with their digests (recorded in `evidence/`).

## Running

```text
sico test .                 # 9 fixture pairs, all byte-exact
sico run main.sico -- latency < samples/basic.csv
```
