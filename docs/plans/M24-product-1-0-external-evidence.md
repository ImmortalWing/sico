# M24 Product 1.0 and external-evidence closure

> Status: planned by owner directive "完成M22，规划M23-24" (2026-09-16); entry requires M23 GO plus owner-supplied platform, identity and pilot inputs; no STEP numbers reserved

## 1. Objective

Turn the internally proven compiler, Runtime, Hosts, packages and representative
applications into an honestly supportable Sico 1.0 release. M24 closes, or
explicitly defers by named external gate, the remaining cross-platform,
production-origin, independent-consumer and live-model evidence gaps. It adds no
feature merely to improve a completion percentage.

## 2. Entry gate

1. M22 compiler bootstrap is GO and the packaged compiler is reproducible.
2. M23 vision/model/native-loop audit is GO for its declared Windows surface.
3. M19 release/registry rehearsals and one-command CI remain green.
4. The owner supplies each platform runner/device, production domain/TLS and
   publisher identity, external-pilot participant and live-model credential to
   be claimed. Missing inputs stay explicit external gates.
5. A release-candidate support matrix names every intended OS/architecture,
   Host/provider and evidence class before execution starts.

## 3. Work tracks

### 3.1 Language/toolchain 1.0 freeze

- Freeze the supported syntax, standard packages, canonical IR/Component/package
  schemas, diagnostics and migration policy.
- Run check/build/run refusal matrices with no undeclared gap.
- Publish deterministic bootstrap/rebuild instructions and the permanent Rust ↔
  Sico oracle policy.

### 3.2 Platform closure

- Execute native parity suites on every claimed Windows, Linux and macOS target.
- Re-enter Android only with a buildable Host, runner and authorized device;
  otherwise publish an explicit 1.0 deferral rather than a documentation claim.
- Measure install, startup, Runtime, Host-provider and update behavior on clean
  machines; simulators and contract tests remain separately labeled.

### 3.3 Production distribution

- Run signed install → execute → upgrade → rollback/uninstall from release
  bundles on clean machines.
- Deploy the registry origin only with owner-controlled domain, TLS, production
  keys, backup/recovery and audit retention.
- Produce reproducible release artifacts, SBOM/provenance, vulnerability review
  and key-rotation/revocation rehearsal.

### 3.4 Independent adoption and operations

- Recruit at least two independent consumers: one package/Component author and
  one representative application/pilot operator.
- Preserve their unedited setup logs, defects, time-to-first-run and patch
  requirements; same-author clean-room fixtures do not satisfy this gate.
- Define support, compatibility, incident, rollback and deprecation procedures
  with measurable response ownership, without inventing an SLA.

### 3.5 Final evidence audit

- Re-run live-model tooling on the pinned corpus when authorized and record
  exact model/version/prompts/cost/raw outputs.
- Audit every AGENT_GOAL §13 claim back to an executed test, native run,
  external pilot or production record.
- Publish one release-candidate GO/NO-GO report with every residual blocker and
  owner.

## 4. Exit gates

1. Language/toolchain 1.0 contract, migration guide and check/build/run matrix
   are frozen and green.
2. Every supported platform has native runner evidence; every absent platform
   is explicitly excluded or deferred.
3. Reproducible signed release plus clean-machine install/upgrade/uninstall and
   recovery rehearsal pass.
4. Production registry/deployment evidence exists, or the release is explicitly
   labeled non-production with the missing owner inputs named.
5. Two independent consumers complete bounded workflows without core patches;
   defects and remediation evidence are retained.
6. Authorized live-model budgets pass or are reported as measured NO-GO.
7. M0-M23 regression is green and the final §13 audit issues an explicit Sico
   1.0 GO/NO-GO verdict.

## 5. Non-goals

- Inflating platform, production or adoption claims from internal fixtures.
- New language syntax, Host authority or model capability unrelated to a failed
  release-candidate gate.
- Silent compatibility breaks or removal of the Rust differential oracle.
- Manufacturing external identities, credentials, devices, domains or pilots.

## 6. Expected evidence

- 1.0 support matrix, compatibility policy and migration guide.
- Native per-platform run records and clean-machine lifecycle evidence.
- Reproducible artifacts, SBOM/provenance and recovery/key-rotation logs.
- Independent-consumer raw logs and issue/remediation register.
- Live-model evidence pack when authorized.
- Final AGENT_GOAL §13 traceability table and Sico 1.0 audit.
