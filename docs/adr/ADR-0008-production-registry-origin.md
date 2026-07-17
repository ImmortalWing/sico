# ADR-0008: Production registry origin boundary

> - status: accepted
> - date: 2026-07-17
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -

## Context

RFC-0024 defines immutable signed registry records and content-addressed `.sapp` blobs, but the implementation is only reachable through `LocalRegistry` in repository tests. A public deployment needs a network read boundary. The current Windows runner has no container engine and no owner-provided domain, certificate, cloud account or production signing identity.

The registry transport is deliberately untrusted under ADR-0006. Therefore the origin must not become a new signing or authorization root.

## Decision drivers

- no private signing material in the serving process;
- immutable, cacheable reads and no directory listing;
- bounded resource use and fail-closed path handling;
- deployable without Docker while remaining container-friendly;
- TLS and public edge policy selected independently by the operator;
- exact separation between locally verified deployment capability and public production evidence.

## Considered options

### Put signing and publication in one public API

Rejected for the first production slice. It couples Internet request handling to key custody and makes authentication/database design a prerequisite for serving immutable content.

### Require an object-storage vendor

Deferred. Static object storage is compatible with the layout, but choosing a vendor now would require an owner account, region, legal terms and credentials.

### Native read-only origin behind a reverse proxy

Accepted. `sico-registry` serves an existing registry tree over bounded HTTP/1.1. It binds loopback by default. Non-loopback cleartext binding requires an explicit unsafe-at-the-edge acknowledgement. TLS, rate limiting and Internet exposure belong to the reverse proxy or private service mesh.

## Decision

1. The origin serves only `GET` and `HEAD`; mutation methods fail with `405`.
2. `/healthz` reports process liveness and `/readyz` requires a readable registry root.
3. `/v1/<relative-path>` maps to a regular file below the canonical registry root. Dot segments, encoded paths, queries, directory listing and symlink escape fail closed.
4. Requests, concurrent connections and response file size are bounded by operator-visible settings.
5. Registry records and blobs receive immutable caching headers. Clients still verify signatures, digests, lengths and `.sapp` structure locally.
6. The origin does not terminate TLS, parse publisher keys, sign records or infer trust from transport identity.
7. Public production status requires separate evidence for domain/TLS, edge policy, monitoring, backup/restore and owner-controlled publication.

## Consequences

The service can be built and smoke-tested in this repository without external authority. Operators can run it directly, as a Windows service, under systemd or inside a container. A public deployment still needs an edge and production materials, but changing providers does not change registry trust semantics.

The trade-off is that publication remains an offline/admin operation in this step. That is intentional: a compromised read origin cannot mint a valid release.

## Validation

Acceptance requires HTTP tests for health/readiness, exact GET/HEAD bytes, method refusal, traversal/encoding refusal, missing files, connection/request bounds and explicit non-loopback consent; an operator-bundle smoke test must use the release binary.

## Revisit conditions

- a provider requires a different health or static-object contract;
- authenticated remote publication becomes an owner-approved requirement;
- HTTP/2 or HTTP/3 at the origin provides measured value beyond the reverse proxy;
- production evidence shows the native origin is not operationally adequate.

## Links

- [STEP-0074](../steps/STEP-0074-production-deployment-origin-and-ux.md)
- [ADR-0006](./ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0024](../rfc/RFC-0024-signed-registry-metadata-v0.md)
