# RFC-0025: Secure update and recovery v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: local update client v0
> - supersedes: -
> - superseded-by: -

## Summary

Define a fail-closed local update transaction, monotonic trusted-state journal, append-only advisory chain and explicit retained-revision recovery contract over STEP-0063/0064 trust records.

## Problem

Individually valid signatures do not prevent replay, freeze, mix-and-match, partial activation or unsafe automatic rollback. A working revision can be destroyed if downloading and activation share a mutable path. Restoring an older package can also cross application identity or silently restore obsolete permissions.

## Goals and non-goals

Goals: one consistent signed transaction; current online metadata; non-decreasing policy/checkpoint/channel/release trust; durable last-trusted time; staged verification; create-new activation commits; retained last-known-good bytes; signed advisories; exact recovery decisions; capability re-evaluation.

Non-goals: public update service, OS installer integration, delta transfer, production key custody, network availability, globally witnessed transparency, Android/Harmony activation or unattended local-state repair.

## Update snapshot

`sico.update.snapshot.v0` is registry-authority threshold signed over a distinct domain. It binds the exact production identity, pinned publisher policy version/digest, snapshot/checkpoint/channel/release sequences, namespace/checkpoint/channel/release record digests, package digest/length, capability fingerprint and latest advisory sequence/digest. It has a bounded validity interval.

The registry snapshot supplies consistency and freshness, not code authority. The namespace event, channel and release keep their STEP-0064 signatures; the package remains publisher-authorized by the release role and is strictly verified locally.

The first trusted transaction starts all four repository sequences at one. Later snapshots strictly advance snapshot sequence; checkpoint, channel, release and advisory sequences never decrease. Reusing a release sequence with different release/package bytes is forbidden. v0 keeps the caller-pinned publisher policy exact during update activation; a policy change must first pass the STEP-0063 transition ceremony and a future explicit client policy-advance interface rather than being inferred from a higher number.

## Consistent transaction and activation

The client verifies, in order: fixed update time; snapshot; checkpoint chain; namespace grant; channel; release; advisory chain; every cross-record digest/sequence; package length/SHA-256; strict `.sapp`; manifest app ID/version; Component-import/source-effect/manifest capability closure; signed capability fingerprint.

Candidate package bytes are written to a staging path and read back before immutable revision persistence. Activation is the creation of the next contiguous `states/N.json` commit record. Revision and metadata files are written first with create-new immutable semantics. A corrupt or interrupted stage cannot create a state generation; stale staging bytes may be replaced on an explicit retry. Previous revision and state generations remain retained.

Each state is canonical JSON with a domain-separated checksum and carries exact production identity, app ID, policy/checkpoint/snapshot/channel/release/advisory trust head, active release/package, capability fingerprint and last trusted time. Startup validates every contiguous generation and every referenced retained `.sapp`. The checksum detects accidental corruption, but the local state root is an OS-protected trust input; an attacker able to restore the entire directory from backup is outside this unkeyed v0 journal and requires an OS secure counter in a later adapter.

## Freeze and rollback

The new snapshot and all online registry records must be current at one caller-supplied fixed update time. A time lower than `last_trusted_time` fails closed. Replayed, lower, forked or same-sequence/different-digest data is rejected before state commit.

No failed update automatically activates a prior revision. `sico.update.recovery-authorization.v0` requires the publisher recovery threshold and binds current package, exact retained release/package, target capability fingerprint, next activation sequence, reason, audit reference and expiry. Alternatively, an explicit local policy must name the same identity/current/target digests, expiry and a domain-separated confirmation digest.

Recovery creates a new state generation while preserving the latest policy/checkpoint/snapshot/channel/release/advisory trust sequences. It never deletes the newer revision. Production identity and manifest app ID cannot change. Capability fingerprint drift always returns `permissions_must_be_reconfirmed`, including rollback to a previously installed package.

## Security advisories

`sico.update.advisory.v0` is signed by the offline revocation threshold and chained by exact predecessor digest and sequence. It includes a bounded advisory ID, inert affected-range text, non-empty sorted exact release and package digest sets, severity, inert description and audit reference. Update snapshots retain the highest advisory sequence/digest, so later discovery cannot silently lower the client's known advisory head.

This makes advisories independently addressable and unambiguous. It does not prove that an uncontacted server has disclosed every issue or provide a public transparency witness.

## Positive and negative cases

Positive: two consistent releases; checkpoint/channel/release progression; advisory advancement; capability re-prompt; corrupt-stage retry; restart verification; signed recovery; exact explicit local recovery.

Negative: snapshot/root/checkpoint/channel/release/advisory replay; expired metadata; clock rollback; mixed release/package; checkpoint omission; package corruption; app-ID drift; capability mismatch; partial activation; state gap/checksum/revision corruption; ambiguous/forked advisory; wrong recovery role; cross-identity/target recovery; unknown field and byte mutation.

## Alternatives

- Mutable `active.json`: rejected because cross-platform replace semantics and crash behavior differ.
- Delete old revision after update: rejected because last-known-good recovery would be impossible.
- Automatically fall back after launch failure: rejected because it bypasses recovery authorization and permission review.
- Compare only SemVer: rejected because version precedence is not a trust sequence or artifact identity.
- Accept any higher publisher policy version: rejected because a registry could manufacture a new policy and become the trust root.
- Treat an unkeyed checksum as anti-rollback hardware: rejected; it detects corruption, not whole-directory restore by a local attacker.

## Validation and acceptance criteria

Acceptance requires all ten assigned threats, the 36-case corpus, two real deterministic `.sapp` revisions, 1,592 signed snapshot mutations, expired/replay/mix/corrupt/state negatives, advisory exactness, signed and local recovery, capability re-evaluation, full workspace regression and zero external operations.

## Links

- [STEP-0065](../steps/STEP-0065-secure-update-rollback.md)
- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0022](./RFC-0022-ecosystem-compatibility-contract-v0.md)
- [RFC-0023](./RFC-0023-production-publisher-policy-v0.md)
- [RFC-0024](./RFC-0024-signed-registry-metadata-v0.md)
- [TUF specification](https://theupdateframework.github.io/specification/latest/)
