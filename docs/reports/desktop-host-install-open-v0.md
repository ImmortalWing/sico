# Desktop Host install/open core v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M5

## Result

GO for STEP-0047. `sico-host-core` consumes M4 `TrustedPackage`/`AuthorizedPackage`, installs exact signed bytes under app+trust/revision hashes and repeats digest/trust/identity/capability verification on every open.

## Evidence

- unsigned/untrusted persistent install refused；
- same app ID with different signer receives a different app identity；
- numeric version downgrade refused by default；
- exact existing revision is idempotent；
- package tamper after install is refused before authorization；
- metadata is strict canonical JSON in a hash-only, link-safe direct-child store；
- install uses a process-unique staging directory and directory rename, so partial revision is never a valid target。

## Boundary

Non-numeric version ordering is not guessed; M5 callers must opt in to an explicit downgrade when ordering is not comparable. Persistent install requires development signer trust and does not imply production publisher identity.
