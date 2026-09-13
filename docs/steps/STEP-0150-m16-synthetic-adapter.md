# STEP-0150: M16 synthetic adapter and fail-closed corpus

> - status: complete (contract-verified; platform runtime claims: none)
> - phase: M16 plan §5.4 (synthetic adapter after the accepted contract set — RFC-0040/ADR-0013/threat model accepted 2026-09-10)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - artifacts: new workspace crate [`crates/sico-automation-host/`](../../crates/sico-automation-host/) (capability core, policy, audit, synthetic backend); [`tests/corpus.rs`](../../crates/sico-automation-host/tests/corpus.rs) (15 fixtures)

## 1. What was done

The Host side of RFC-0040 implemented against a simulated surface:
surface registry (policy-exposed only, revocable identities with
revision counters), capture with size/rate/total ceilings, the
preview/commit token path (single-use, surface+revision+action-digest+
expiry bound, nonce-separated so identical previews never collide),
keyboard as a separate grant with declared sensitive-field refusal
(F-1) and the printable-alphabet gate (new `InvalidText` — RFC-0040
amendment A1, recorded), dry-run as withheld minting (ADR-0013 §4,
guest-invisible), the append-only redacted audit stream with the JSONL
export form (F-2), and Host-owned emergency stop draining every token
and grant. A Windows/GDI adapter joins later per ADR-0013 as amended
(A1: v0 ships GDI PrintWindow capture).

## 2. Validation

`corpus.rs` freezes every RFC-0040 §4 refusal row as an asserted
fixture (15 green): commit without/expired/reused token, stale
revision, surface drift (identity invalidation drains tokens),
duplicate-commit semantics, capture↛pointer, pointer↛keyboard,
sensitive-field refusal, non-printable refusal, rate/budget limit+1
(window slides), dry-run invisibility, emergency stop, audit
append-only + redaction. Evidence class: `contract-verified` — never
advertised as runtime support.

## 3. Declared limits

- E9xxx guest-facing diagnostics await the automation guest compiler
  surface (this step is the Host side; the WIT world is not yet linked
  into `sico build`).
- Long-running handle/RSS budgets and the real Windows loop are the
  next STEPs (M16 plan §5.5); nothing here is platform evidence.
