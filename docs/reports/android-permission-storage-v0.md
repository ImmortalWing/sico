# Android permission and storage mapping v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M6

## Result

GO for STEP-0057 cross-check. All five M4 capabilities have exact mappings. Only `network.connect` needs the normal manifest `INTERNET` permission; v0 requests no dangerous Android runtime permission. App-private storage, clock, random and logging remain Host scopes.

Content URI read grants are ephemeral Host-ingress authority and never enter guest capability sets. Signer change and capability drift retain the M5 identity/fingerprint invalidation behavior. Kotlin checks any future runtime permission at use time; SDK compilation remains unavailable.

