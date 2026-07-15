# ADR-0003: Hashed per-app storage and explicit WASI grants

> - status: accepted
> - date: 2026-07-16
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -

## Context

应用 ID、显示名称、资源路径和 public key 都来自 package，不能直接成为宿主目录。Wasmtime CLI 默认可以继承 stdio，但不会自动预开放宿主目录或网络；M4 必须只从 `AuthorizedPackage` 产生 flags。

## Decision

- storage directory name is lowercase SHA-256 of `SICO-STORAGE-ID-V0\0 || app-id || 0 || trust-identity`；
- signed/unsigned and different development keys never share a namespace；
- base/root must canonicalize to direct parent/child and may not be symlink or Windows reparse point；
- tree audit never follows links, rejects special files, and checks persistent byte quota before/after execution；
- guest receives only one `HOST::/data` preopen when `storage.read-write` is actually granted；
- clock/random/network flags are emitted only for their exact granted capability；network inheritance, UDP and DNS lookup remain off in v0；
- `log.write` has no Wasmtime CLI adapter and is rejected rather than silently linked.

The current external-process boundary can enforce persistent storage quota at launch/return and bounds transient execution with STEP-0043 timeout/fuel. It does not claim a filesystem-level byte-by-byte disk quota; Hosts that expose long-running writable storage must add an in-process quota adapter before widening this boundary.

## Consequences

App-controlled strings never join host paths; cross-app roots differ deterministically. Extra host grants do not create flags. Windows junction/reparse attacks are covered explicitly. Storage identity changes with trust identity, preventing an unsigned/dev-key package from inheriting another identity's data.

## Links

- [`STEP-0042`](../steps/STEP-0042-wasi-capability-host-isolated-storage.md)
- [`RFC-0017`](../rfc/RFC-0017-capability-closure-permission-v0.md)
