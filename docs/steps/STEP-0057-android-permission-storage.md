# STEP-0057: Android permissions and isolated storage mapping

> - status: complete
> - phase: M6
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## Evidence

- exact mapping for five M4 capabilities；unknown defaults deny；
- `INTERNET` is normal manifest permission; no dangerous runtime permissions in v0；
- URI grants are separate, ephemeral Host ingress；
- app+signer and capability drift change authority keys；
- app-private storage remains under shared Host identity；
- 4 tests + two Android target checks。

```text
STEP_0057_OK capabilities=5 manifest=INTERNET runtime_permissions=0 uri_grants=ephemeral-separated storage=app-private-app-signer drift=re-prompt tests=4 android_checks=2 next=STEP-0058
```

