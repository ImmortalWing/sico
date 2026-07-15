# ADR-0004: Desktop Host identity, lifecycle and platform boundary v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - scope: M5 Desktop Host
> - supersedes: -
> - superseded-by: -

## Context

M4 can verify, trust, authorize and limit `.sapp`, but a Desktop Host adds mutable shell paths, persistent installs, permission decisions, concurrent opens, windows and process lifecycle. Using only `app.id` as identity would let another development signer inherit storage or permission records; executing the originally selected path would leave a TOCTOU window.

Platform association mechanisms also differ. Windows exposes per-user file type registration under `HKEY_CURRENT_USER\Software\Classes`; macOS declares supported document UTIs through bundle `CFBundleDocumentTypes`; Linux desktops use MIME desktop entries and `mimeapps.list`. These mechanisms are adapters, not new trust roots.

Official references accessed 2026-07-16:

- [Microsoft File Types](https://learn.microsoft.com/en-us/windows/win32/shell/fa-file-types)
- [Apple CFBundleDocumentTypes](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundledocumenttypes)
- [freedesktop MIME Applications 1.0.1](https://specifications.freedesktop.org/mime-apps/latest-single/)

## Decision

### Immutable identities

- `AppIdentityKey = SHA-256("SICO-DESKTOP-APP-ID-V0\\0" || app-id || NUL || trust-identity)`；
- `RevisionDigest = SHA-256(exact canonical .sapp bytes)`；
- `CapabilityFingerprint = SHA-256(canonical sorted authorized capabilities)`；
- persistent install requires a cryptographically valid locally trusted development signer；unsigned development packages may run ephemerally but cannot create persistent app/permission/association state。

The install store is `root/apps/<AppIdentityKey>/revisions/<RevisionDigest>/app.sapp`. Metadata is canonical JSON installed with create-new temporary + atomic rename. Every open rereads the installed package, verifies its digest, strict package/signature trust and capability closure, then passes only `AuthorizedPackage` onward.

### Lifecycle

v0 uses one supervised guest per immutable app identity. A second open becomes a bounded host event; it never creates an unsupervised duplicate. States are `Installed → Starting → Running/Background → Closing → Exited|Crashed|TimedOut|Cancelled`. Terminal transitions audit temporary files and storage. Graceful close is bounded; expiry kills the guest process tree.

### Platform evidence

Shared Rust core owns install/trust/permission/lifecycle semantics. Platform adapters may only translate shell/open/window/dialog operations. Windows is the M5 measured platform. macOS/Linux can be marked `contract-verified` or `compile-verified` only when their adapter/corpus is checked; neither label equals runtime-verified.

Association registration advertises Open With support but does not silently become the user's default. Shell commands use an executable path and `%1`/platform equivalent as distinct arguments; package paths are never concatenated into a shell string.

## Alternatives

1. App ID alone: rejected because signer collision inherits authority.
2. Package digest alone: rejected because every revision loses stable app/storage identity.
3. Execute the selected external path after verification: rejected due to TOCTOU.
4. One process per open: rejected because it creates unbounded duplicate guests and ambiguous permission/lifecycle ownership.

## Consequences

Updates retain app identity only for the same signer but have distinct revision digest. Capability changes require a new permission fingerprint. Production publisher identity and migration/revocation remain M7 decisions; this ADR does not promote development trust into production identity.

## Links

- [`STEP-0046`](../steps/STEP-0046-desktop-host-threat-lifecycle-contract.md)
- [`M5 plan`](../plans/M5-desktop-host.md)
- [`threat matrix`](../../tests/desktop-host/threat-matrix.json)
