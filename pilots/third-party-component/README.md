# Third-party-style Component and release pilot

> - evidence: repository-authored clean-room
> - expected Runtime result: `42`
> - production/public/mobile claims: none

This crate is deliberately outside compiler/runtime implementation directories and consumes only public crate APIs. It represents the strongest local substitute for an external adopter: two standalone source revisions, a UI model and one manifest drive editor, AI inspect, compiler, Component, package, publisher, registry, checkpoint, Host and Runtime boundaries without modifying those implementations.

## Reproduce

```powershell
$env:SICO_TEST_WASMTIME = & ../../tools/ensure-wasmtime.ps1
cargo test --locked --offline -p sico-third-party-pilot -- --nocapture
```

The release drill builds versions 1.0.0 and 1.1.0, signs local development packages, publishes signed local release/channel metadata, verifies a chained checkpoint, discovers and downloads exact bytes, installs/upgrades through Host storage and runs the latest revision through the selected Wasmtime binary. Tamper, replay and unapproved downgrade attempts must fail.

`ui.json` is a strict companion UI contract because compiler-facing UI bindings remain outside the current source subset. The test validates it through the public Host UI model.

## External handoff

An actual independent pilot must fork/copy this directory without privileged repository knowledge, record its own organization/person, commit, tool versions, raw commands, failures and feedback, and return an immutable evidence bundle. Production identity, namespace, service and signing material must be owner-provided; development fixture keys in this test are never reusable.

This local pilot must not be relabeled as third-party evidence. The gate is recorded in [`pilot.json`](./pilot.json) and the [M7 exit audit](../../docs/reports/m7-exit-audit.md).
