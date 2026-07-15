# Package CLI and source cache v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M4

## Result

GO for STEP-0044. Default `build` now emits verified `.sapp`; `inspect` verifies without execution; package `run` requires explicit development trust and passes through capability authorization plus Runtime limits. M3 raw Component remains available only through `--raw-component` for regression evidence.

## Evidence

- 8 CLI integration tests cover 54 semantic oracles plus package build/inspect/run/cache contracts；
- same source/config produces byte-identical `.sapp` from file and stdin；
- deterministic development signature is independently inspected and then trusted with an explicit public-key file；
- package execution without a trust choice exits 2；
- cache entry is strict-verified on hit, and deliberate corruption exits 2 before Runtime launch；
- non-empty scalar arguments exit 2 rather than being discarded；
- tampered package inspection fails before execution；
- real Wasmtime 46.0.1 returns the expected scalar result after package authorization。

## Honest boundary

source cache currently stores a canonical package and intentionally fails closed on corruption; it does not auto-repair the audit artifact. Runtime stdin and application args are not implemented for scalar `main()`. Production signing, publisher identity, registry and update remain outside M4.
