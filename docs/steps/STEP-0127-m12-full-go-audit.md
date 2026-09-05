# STEP-0127: M12 full-GO exit audit refresh

> - status: complete / GO
> - phase: M12
> - started: 2026-09-05
> - completed: 2026-09-05
> - owners: autonomous-agent
> - supersedes: the GO-core verdict of STEP-0118 (same gates, refreshed evidence)

## 1. Gate-by-gate verdict (M12 plan §10)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | versioned WIT/provider contract coexists with HTTP v0 without silent mutation | **GO** | RFC-0037 accepted; STEP-0089 oracle rerun green on Windows and Linux after every provider change; `wit/script-http2-v0/http2.wit` freezes the `0.2.0` surface; real guest Components importing `http@0.2.0` run beside unchanged `0.1.0` clients (STEP-0125) |
| 2 | real HTTPS with valid trust; rejects certificate/hostname failures | **GO** | STEP-0112 probes + STEP-0125/0126 corpus; no `danger()`/custom-verifier/insecure path (validator scan); empty trust roots fail closed |
| 3 | endpoint canonicalization, DNS pinning, private-address policy resist the confusion/rebinding corpus | **GO** | STEP-0113 corpus green on both platforms (48/48 provider tests) |
| 4 | streaming bodies preserve bytes, backpressure, budgets, bounded RSS | **GO** | `BodyReader` strict incremental framing (length/chunked/until-close, CRLF + trailers verified, budget per chunk); chunked download byte-exact through a real guest Component; depth-1 rendezvous backpressure; 64 MiB default budget, 1 GiB Host ceiling |
| 5 | redirects never widen authority or forward secrets cross-origin | **GO** | STEP-0115 engine unchanged; redirect loop re-runs policy per hop; protected headers stripped structurally on origin change |
| 6 | Host secret use exact, opaque, redacted across CLI/events/DAP/AI/cache | **GO** | STEP-0116 secret store; STEP-0125 wires it through the runner: injection host-side only, typed errors only cross the boundary; `--secret-file` keeps values out of argv |
| 7 | cancellation/timeout/retry/pooling one bounded lifecycle per Store | **GO** | STEP-0125 closes the partial: engine in-flight cap atomic (16), per-Store pool (≤32/≤8/15 s idle) dies with the Store, retry idempotent-only within one total deadline, abandoned-Store fail-closed discipline identical to `0.1.0`, runner cancellation reaches streaming reads |
| 8 | stdlib/tooling helpers cannot bypass policy | **GO** | nothing outside the provider crate performs IO; helpers lower to the same boundary |
| 9 | complete M0–M11 regression green | **GO** | `validate-step-0110.ps1` aggregate chain rerun green after STEP-0125/0126 (STEP_0110_OK, gates 10/10, decision GO); `validate-step-0125.ps1` adds workspace tests + STEP-0089 oracle + module boundaries |
| 10 | platform support limited to actual native execution | **GO** | Windows x64 (STEP-0118 + STEP-0125) and Linux x64 native (STEP-0126: provider 48/48, runner 37+6+35, oracle 13/13); no macOS/mobile claims |

Automatic-NO-GO list (custom TLS, ambient credentials, unbounded
queues, cross-origin leaks, Store-crossing reuse): **zero violations**.

## 2. Verdict

**M12: GO** — full GO, upgrading the GO-core of STEP-0118. The contract
(RFC-0037), all five policy/transport layers, and both integration
deliverables are complete with Windows x64 and Linux x64 native runtime
evidence. Source-level `http@0.2.0` import emission remains intentionally
outside M12 and belongs to the M14 application-ready language baseline;
nothing in this audit claims source-language support.

## 3. External-input recheck (2026-09-05)

| Input | State |
|---|---|
| Public deployment / production identity | still absent |
| Third-party pilots | still absent |
| Live-model credentials (M13) | owner-promised, not yet delivered |
| Mobile runners | still absent |
| Linux x64 | available; provider corpus green (STEP-0126) |

## 4. Residual risks (carried from STEP-0118, updated)

- `system-roots` trust mode is not implemented (no native-certs
  dependency); TLS requires explicitly provided pinned roots and fails
  closed otherwise. This is a functional limitation, not an
  insecurity.
- Revocation checking remains explicitly unsupported in v1 (RFC-0037 §3).
- Secret zeroization is bounded by rustls/String semantics; values never
  persist in caches and never cross process boundaries.
- The runner CLI wires `--http-trust-roots`/`--secret-file` through the
  plain-run path; the watch/debug entry points still prepare generations
  with the default (empty-trust) policy, which fails closed for TLS.
- The RSS-bound DAP tests in `tests/runner.rs` are measurement-noisy
  under host parallelism; the flakiness reproduces on the STEP-0118
  commit and is not a regression of STEP-0125/0126 (serial runs green on
  both platforms).

## 5. Validation

`tools/validate-step-0125.ps1` (Windows) and
`tools/validate-step-0126.sh` (Linux, evidence at
`target/evidence/step-0126/linux/validator.log`) both exit green on
2026-09-05, and the full `tools/validate-step-0110.ps1` M0–M11 aggregate
chain was rerun green after the STEP-0125 changes (one real regression
was caught and fixed during this audit: the `http@0.2.0` draft WIT moved
to `wit/script-http2-v0/` because wit-parser rejects two versions of one
package in a single directory). M12 successor numbering (M14–M18 sequence) remains governed
by STEP-0124; no future STEP is implied by this audit.
