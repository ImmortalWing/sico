# STEP-0056: Android package, open, share and deep-link adapter

> - status: complete
> - phase: M6
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## Evidence

- ACTION_VIEW/ACTION_SEND/picker: one content URI + exact MIME + `.sapp` name；
- bounded one-pass stream copy before shared Host verification；
- `sico://open` opens picker only；no embedded network/file URI；
- AndroidManifest/Kotlin adapter contract；
- 4 intent/stream tests and two Android target checks。

```text
STEP_0056_OK actions=view,send,picker deeplink=picker-only uri=content-only stream=copy-once limit=64MiB mime=exact manifest=parsed tests=4 android_checks=2 sdk_compile=unavailable next=STEP-0057
```

