# M19 Production engineering and release readiness

> Status: partial GO per STEP-0168 (gates 1/2/4/7 GO; signed clean-host install, budgets and documentation partial); historical plan refined 2026-09-13; M25 owns release-candidate residual closure; no STEP numbers reserved

## 1. Objective

Turn the GO'd milestones into a *deliverable* product: continuous
integration that makes every claim reproducible, versioned release
artifacts with installers, a deployable registry origin, performance
budgets where none exist, and a documentation set that matches real
behavior. M19 does not add user-facing features; it makes everything
M8–M18 built survivable outside this repository.

## 2. Entry gate

| # | Condition | Status (measured 2026-09-13) |
|---|---|---|
| 1 | M15–M17 exit audits explicit (GO or honest NO-GO registers) | **satisfied** — M15 GO 7/7 (STEP-0158 + 0162), M16 GO 7/7 (STEP-0159), M17 NO-GO register with gate-1 GO and completion path (STEP-0160/0166) |
| 2 | M18 internal-fixture portfolio runs without core patches | **satisfied (4/5)** — STEP-0164/0165: API agent, streaming tool, Web/UI app, package consumer all green; block-game pilot honestly deferred (M17 full GO pending) |
| 3 | No unresolved compiler-defect registers (A6-class seams) | **satisfied** — A6 repaired (STEP-0148); the five vision-package defects found in STEP-0166 were fixed in the same step; register clear |
| 4 | Registry/deployment external inputs (domain, TLS, identity) remain owner-gated | unchanged — owner-supplied; M19 prepares but does not claim public deployment |

## 3. Required workstreams

### 3.1 CI and reproducible verification

- A pipeline definition (one command per check) running, concretely:
  `cargo fmt --all -- --check`; `cargo clippy --locked --offline
  --workspace --all-targets --all-features -- -D warnings` (both
  workspaces); both workspaces' full test suites;
  `tools/validate-module-boundaries.ps1`; `tools/validate-step-0124.ps1`
  (M14–M20 planning contract); `tools/validate-step-0131.ps1`
  (application-profile matrix); `tools/validate-step-0156.ps1`
  (cross-host matrix + UI corpus); the fixture generators re-run and
  compared (`generate_pilot_packages`, `generate_web_harness`,
  `generate_ui_corpus`, `generate_web_ui_pilot` — all #[ignore], invoked
  explicitly); and the wasm artifact snapshot check
  (`tests/wasm/artifacts.hex`).
- CI must run the cross-host matrix (M15) headless-browser step and the
  M16 synthetic corpus; real-desktop steps stay local-evidence and are
  recorded, not faked.
- Flaky-test policy, grounded in the two ACTUALLY observed env-sensitive
  tests (`debug_pause_terminate_leaves_no_stranded_tasks_or_workers`
  RSS growth and `prepared_program_reruns_are_isolated_and_survive_a_trap`
  warm median — both fail under heavy parallel load and pass solo, M14- and
  M16-class): budget asserts get explicit solo-retry classification in CI
  config, recorded as such — never silently retried to green.

### 3.2 Release engineering

- Versioned release bundle: `sico` CLI, `sico-runner`, `sico-app`
  (desktop host where applicable), digest manifest, build provenance
  (toolchain, lock hashes) — plus the package-signing tool path so the
  signed pilot packages regenerate exactly as the
  `generate_pilot_packages` / `generate_tetris_vision_package` pattern
  does today (M18 pilots must install from this bundle, per the M18
  refinement).
- Reproducible-build check: two clean-room builds of the same tree
  produce byte-identical artifacts (deterministic codegen already
  proven; extend to packaging).
- Install/upgrade/uninstall scripts for Windows with signature
  verification riding the M5/M7 development trust path.

### 3.3 Registry origin and distribution (owner-gated)

- The M7 local registry server promoted to a deployable configuration
  (config review, health endpoint, retention, backup), deployable the
  moment owner inputs (domain/TLS/identity) arrive.
- Update-path end-to-end rehearsal against a private origin: publish →
  discover → download → verify → authorize → run.

### 3.4 Performance budgets

- Convert "no SLA" markers into recorded budgets: compile throughput,
  run warm/cold latency, runner RSS ceilings, package verify time per
  MiB — measured on pinned hardware, stored as versioned fixtures with
  deviation alarms (the M16 budget-corpus pattern generalized).

### 3.5 Documentation completeness

- User manuals regenerated from actual behavior (`--help` snapshots,
  exit-code tables, limits tables); ops runbook for registry origin and
  release bundle; every RFC/ADR cross-linked from the docs index.

## 4. Non-goals

- New language surface, new host capabilities, or new platform claims.
- Public deployment claims (owner inputs remain external gates).
- Performance SLAs beyond the recorded budgets.
- Rewriting M18 applications to satisfy release constraints (feedback
  flows the other way).

## 5. Exit gates

1. CI green on a clean machine from a fresh clone: every validator and
   both workspaces, including the browser matrix step.
2. Two independent builds produce byte-identical release bundles with
   verifiable manifests.
3. Install → run → upgrade → uninstall rehearsal passes on a clean
   Windows VM/box with signature verification enabled.
4. Registry origin deployable configuration reviewed and rehearsed
   end-to-end against a private origin (public deployment remains
   owner-gated).
5. Performance budgets recorded for every "no SLA" marker and wired
   into CI as advisory alarms.
6. Documentation audit: every manual section maps to executable
   behavior; stale sections removed or marked.
7. Full M0–M18 regression green and an explicit exit audit.

## 6. Dependencies and parallelism

- §3.1/§3.2/§3.4/§3.5 can start immediately after the entry gate.
- §3.3 is ready-to-deploy work blocked only by owner inputs.
- M18 pilots and M19 workstreams interleave: a pilot that cannot
  install/upgrade through the release bundle exposes an M19 gap.

## 7. Sequencing rule

No implementation STEP is reserved by this plan. First STEP: CI
definition + reproducible-build rehearsal (the smallest end-to-end
proof). Each subsequent STEP closes one workstream item and reruns the
cumulative CI; regression stays green throughout.

## 8. Execution sequence (work items; STEP numbers allocated at kickoff)

1. CI pipeline definition + one-command local runner; wire the seven
   validators and both workspaces; solo-retry classification for the two
   documented budget flakes.
2. Reproducible-build rehearsal: two clean builds → byte-identical
   bundles; digest manifest + provenance record.
3. Install/upgrade/uninstall rehearsal on a clean Windows environment
   with signature verification (M5/M7 development trust path).
4. Registry-origin deployable configuration + private-origin end-to-end
   rehearsal (publish → discover → download → verify → authorize → run).
5. Budget fixtures: compile throughput, warm/cold run latency, runner
   RSS, package-verify per MiB — replacing the remaining "no SLA"
   markers in STATUS §3, wired as CI advisory alarms.
6. Documentation audit: manuals regenerated from `--help` snapshots,
   exit-code and limits tables; ops runbook for origin + bundle.
7. M19 exit audit (gates 1–7 explicit).
