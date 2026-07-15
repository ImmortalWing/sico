# RFC-0021: Android Intent and JNI contract v0

> - status: accepted
> - date: 2026-07-16
> - target phase: M6

## Contract

The Android boundary is a typed request/response protocol, not arbitrary JNI calls. Kotlin supplies canonical UTF-8 JSON plus copied package bytes through fixed operations: `probe`, `ingest`, `permission`, `lifecycle`, `ui-model`, `ui-event`, `run`, and `cancel`. Every request has schema, request ID, operation and bounded payload. Every response has the same request ID, `ok|error`, stable code and bounded data.

JNI catches Rust panics and maps them to `NATIVE_PANIC`; Kotlin exceptions map to `PLATFORM_ERROR`. Neither stack trace nor Java object reference enters durable state. URIs are consumed by Kotlin/ContentResolver and never passed to Rust as host paths. Rust receives exact bytes or an immutable staging path beneath the Host root.

## Limits

- JSON request/response: 64 KiB;
- package stream: M4 package ceiling;
- one URI and one active guest per immutable app identity;
- 256 queued open/UI events, 120 UI events per second;
- no raw pointers, object handles, reflection, shell strings or WebView content in the protocol.

## Versioning

Unknown schema, operation, field, enum or response code fails closed. `sico.android.bridge.v0` remains internal until Android runner evidence and M6 GO.

