# STEP-0069: third-party-style pilot and M7 exit audit

> - status: complete-local / blocked-external-evidence
> - phase: M7
> - started: 2026-07-16
> - completed locally: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Close every repository-local M7 workflow through a standalone two-release application, then issue an evidence-based milestone/project decision without upgrading local fixtures to external proof.

## 2. Scope

Included: public LSP and AI inspection; UI contract; source-to-Component build; development-signed `.sapp`; production-policy fixture; signed local namespace/release/channel/checkpoint; discover/download; Host install/upgrade/open; selected Wasmtime result; security refusal paths; non-SLA performance; full regression; completion audit.

Excluded: independent participant actions, legal production identity, production keys, public services, provider credentials/cost, Android/Harmony/Linux native runners and unsupported source debugging.

## 3. Completed work

1. Added `pilots/third-party-component` outside historical `examples/` and limited it to public crate APIs.
2. Exercised two revisions, versions 1.0.0 and 1.1.0, through eleven local stages.
3. Executed Wasmtime 46.0.1 and verified result `42`.
4. Rejected package tamper, downgrade, checkpoint tamper and channel replay.
5. Added 24 machine cases and a Windows debug reproducibility baseline.
6. Audited every M7/project requirement and preserved seven external evidence gates.
7. Reran format, Clippy, workspace tests and STEP-0063–0069 validators.

## 4. Validation

The pilot integration target has four tests. The release drill invariant is:

`PILOT_RELEASE_METRICS releases=2 result=42 runtime=true security_rejections=4`

The step validator emits:

`STEP_0069_LOCAL_OK tests=4 stages=11 releases=2 result=42 runtime=true security_rejections=4 cases=24 third_party=external-gated production=external-gated live_model=external-gated mobile=external-gated linux_runner=external-gated project=blocked-external-evidence`

## 5. Exit decision

Repository-local STEP-0069 is complete. M7 and project product completion remain NO-GO because independent third-party, production and target-platform evidence is external and absent. This is a closed audit state, not a claim that the product gates passed.

## 6. Links

- [pilot](../../pilots/third-party-component/README.md)
- [release drill](../reports/m7-pilot-release-drill.md)
- [M7 exit audit](../reports/m7-exit-audit.md)
- [project completion audit](../reports/project-completion-audit.md)
