# STEP-0084: performance, security, platform and M8 exit audit

> - status: complete
> - phase: M8
> - started: 2026-07-18
> - completed: 2026-07-18
> - owners: autonomous-agent

## 1. Objective

Audit M8 against the section-11 exit gate with measured evidence: rerun the performance matrix on the final STEP-0083 pipeline, record Windows platform evidence in full, attempt Linux runner evidence through WSL (honest deferral if the environment fails), and produce an honest GO/NO-GO decision that distinguishes measured evidence from goals.

## 2. Included

- the §11 exit gate item-by-item, mapped to concrete artifacts and validators;
- `sico run` cold (cache-miss) and warm (cache-hit) latency matrix on the final pipeline;
- composed cold/warm comparison against the STEP-0076 baseline method where reusable;
- security re-verification: fail-closed cache/capability/malformed-component evidence reruns;
- Linux runner evidence via WSL when the toolchain can be established;
- the M8 audit report and GO/NO-GO.

## 3. Excluded

- new features, M9 streaming/async work;
- public deployment or production signing;
- invented SLAs: every number records method, iteration count and environment.

## 4. Exit gate

- all eight §11 items verified with named evidence artifacts;
- benchmark matrix published with method and environment;
- full workspace regression and all step validators 0077–0083 green;
- audit report states GO or NO-GO with honest limits.

## 5. Links

- [`M8 plan`](../plans/M8-script-profile.md) (§11 exit gate)
- [`STEP-0076 composition evidence`](../reports/script-profile-composition-v0.md)
- [`STEP-0083 stdlib evidence`](../reports/script-standard-library-v0.md)
