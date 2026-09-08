# STEP-0142: M15 kickoff inventory, source-modules/user-WIT RFC and consumer scope freeze

> - status: complete (inventory + RFC draft); RFC-0039 acceptance pending owner
> - phase: M15 (first step per M15 plan §7: inventory/RFC before code changes)
> - started: 2026-09-07
> - completed: 2026-09-07
> - owners: autonomous-agent
> - artifacts: [`RFC-0039`](../rfc/RFC-0039-source-modules-package-resolution-user-wit-v0.md) (draft)

## 1. What was done

The pre-code-change inventory M15 plan §7 requires, mirroring the STEP-0129
pattern: every module/package/WIT-relevant surface classified against real
`check/build/outline` behavior via live probes, the draft RFC freezing the
prerequisite-track contract (source modules, package resolution, user WIT
imports), and the bounded feature list for the clean-room consumer that
closes M15 entry condition 3 (owner decision 2026-09-07: condition kept,
closed by evidence, not re-scoped).

## 2. Key measured findings (probes + source evidence)

Live probes through `sico check/build/outline` (2026-09-07, sico 0.0.2-dev
rebuilt from the working tree, Windows x64):

1. **Single-file compilation unit**: `check [OPTIONS] <FILE|->`; a second
   path is a clap `unexpected argument` error. No file links another.
2. **No module syntax**: `module`/`use`/`import` are not keywords; top-level
   `module helpers` and `use sico.component` each yield
   `E1013 top level accepts declarations only` (RFC-0033 refusal reused).
3. **`interface` declarations are silently dropped**: `check ok` on
   `interface Formatter: function format(...) returns ... end interface`;
   outline lists it; semantics matches `DeclarationKind::Interface => {}`
   (sico-semantics lib.rs:286); codegen ignores it. The
   `wit-safe-interface.sico` oracle demands syntax acceptance — accepting
   the syntax while giving it no meaning is an undeclared behavior gap.
4. **Call-target resolution is codegen-time**: `double(2)` with no `double`
   in the file passes `check`; `build` refuses
   `unsupported call target ScriptOutput at bytes …` (exit 2).
5. **Other parse-accepted surface classified**: `capability` blocks (check
   ok; consumed only as `Type::Capability` naming at lowering),
   `export function` prefix (accepted, no distinct semantic identity),
   `using` at top level (E1013; it is a function-body block).
6. **Fixed WIT world**: `sico:script/program@0.1.0` + buffered
   `http@0.2.0 request` emission (STEP-0136); no user-WIT mechanism.
7. **Packages carry built Components**: `sico-app` builds `.sapp` from a
   WebAssembly Component; no source-level dependency resolution exists.
8. Scale: 42 literal `sico.*` intrinsic names + `sico.map/set` generic
   families; codegen refusal mentions ~159 (126 measured sites at
   STEP-0129, grown by M14 slices).

Full classification and the frozen proposal are in RFC-0039 §1–§2.

## 3. Probe reproduction

```text
sico check wit-interface.sico   # check ok — interface silently dropped
sico check module-use.sico      # 2x E1013 (module / use at top level)
sico check uses-lib.sico        # check ok — undefined double(2) accepted
sico build  uses-lib.sico       # unsupported call target … (exit 2)
sico check uses-lib.sico lib.sico  # clap: unexpected argument
sico outline cap2.sico          # capability + function listed
```

Probes live in the session temp directory (`target/probes/step-0142/`,
not committed); the refusal classes are durable compiler behavior and the
RFC's exit corpus (§4) re-verifies each as a committed fixture.

## 4. RFC decision points left to the owner

- **D0**: accept RFC-0039 as drafted (or amend §2 before acceptance).
- **D1**: `export function` today parses with no semantic identity — typed
  refusal (recommended; a decorative marker contradicts the no-silent-
  discard rule) or declared no-op marker until an exports RFC.

## 5. Clean-room consumer: bounded feature list (frozen)

Purpose: M15 plan §3.0 track exit test and M15 entry-condition-3 evidence,
delivered at `clean-room-consumer` class. Location: `pilots/log-analyzer/`
(separate code path outside `tests/`/`examples/`; mirrors the
`pilots/third-party-component` precedent). Development is docs-driven:
no compiler/Runtime patches, no fixture internals; violations fail the
deliverable. Evidence record states the same-author limitation honestly
(independent code-path pressure, not independent third-party validation).

**Application**: CSV log analyzer, `sico test`-driven, byte-exact output.

| Item | Frozen v1 scope |
|---|---|
| Own modules | `main` (entry/IO), `parse` (CSV subset parser), `report` (aggregation + rendering) — exercises the multi-module surface |
| Consumed packages | `csv@1` (line/document parse) and `table-stats@1` (column aggregates) via user WIT imports under the lock — exercises package resolution + user WIT |
| Input contract | UTF-8 CSV ≤ 1 MiB, header row + rows; quoted fields; CRLF/LF |
| Behavior | per-column `count` / `min` / `max` / `mean` (I64 fixed-point ×1000) over a selected column; deterministic Text report |
| Typed failures | malformed row (with row index), unknown column, empty input, oversize input — all fail closed, exit codes fixed |
| Tests | ≥ 8 `sico test` fixture pairs (happy/edge/error), all byte-exact |
| Evidence | raw run logs, lock + package digests recorded, workspace-diff proving no compiler changes |

The packages themselves are small prebuilt Components produced by the
§3.0 implementation track as ordinary internal artifacts; the *application*
is what is clean-room.

## 6. Validation

Document-only step: inventory evidence from compiler sources and live
probes; no production code changed. `tools/validate-step-0124.ps1` green;
`git diff --check` clean. RFC-0039 proceeds to owner acceptance before any
implementation STEP; D1 is resolved at acceptance time.
