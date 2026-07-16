# M7 clean-room pilot release drill

> - status: complete-local
> - step: STEP-0069
> - date: 2026-07-16
> - evidence: repository-authored clean-room; not independent third-party or production

## Outcome

The standalone pilot under `pilots/third-party-component` consumed public crate APIs without changing compiler or Runtime implementations. Two source revisions passed LSP diagnostics/index, structured AI inspection and strict companion UI validation, then completed source → IR → Component → signed `.sapp` → signed local registry → Host install/upgrade/open → Wasmtime execution. Revision 1.1.0 returned `42`.

The drill rejected four security failures: downloaded-byte tamper, unapproved Host downgrade, checkpoint tamper and stable-channel sequence replay. The machine corpus freezes 24 workflow/evidence cases.

## Measurement

One Windows debug run with Rust 1.97.0 and Wasmtime 46.0.1 measured:

| Segment | Time |
|---|---:|
| build two packages | 16 ms |
| publish/discover/download/checkpoint | 461 ms |
| Host install/upgrade/open/refusals | 253 ms |
| Wasmtime execution | 2,957 ms |
| total | 3,704 ms |

This is a reproducibility baseline, not an SLA or cross-platform benchmark. The source record is `tests/performance/m7-pilot-windows-debug.json`.

## Evidence boundary

The fixture proves that the repository's documented public surfaces compose locally. It does not prove independent developer usability, legal publisher identity, production key custody, a public registry, live-model value, Android/Harmony delivery or Linux-native behavior. Development fixture keys are disposable and cannot become production keys.

## Reproduce

```powershell
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
cargo test --locked --offline -p sico-third-party-pilot -- --nocapture
pwsh -NoProfile -File .\tools\validate-step-0069.ps1
```

Expected invariant: `releases=2 result=42 runtime=true security_rejections=4`.
