# M16 Native Automation Host — threat model (pre-RFC)

> - status: accepted (owner session directive "完成M15-17", 2026-09-10; F-1/F-2 ruled below)
> - date: 2026-09-08
> - rule: every threat maps to a §3 capability boundary or a §6 non-goal of the M16 plan **and** to a testable refusal or corpus case. Threats without a test are open findings.

## 1. Scope and assets

Assets: the user's desktop session (windows, pixels, input stream), the user's credentials and attention, the Host's audit integrity, and the fixture-app boundary. Out of scope: kernel-level attackers, attackers with pre-existing code execution outside the Sico Host, and physical access.

## 2. Adversaries and abuse paths

| # | Adversary / path | Consequence if open | Boundary / refusal | Test |
|---|---|---|---|---|
| T1 | Hidden automation of the user's own session (guest drives the pointer with no visible affordance) | user cannot distinguish own actions from automation | dry-run default is Host-enforced; preview/commit tokens single-use and expiring; audit events per committed input | corpus: commit without preview token → typed refusal; expired token → refusal; audit-event presence per action |
| T2 | Credential/OTP harvesting via capture+input chains (focus a password field, screenshot, read pixels) | credential theft | capture grants are surface-scoped and rate-limited; sensitive-field refusal policy for keyboard; Host-side redaction heuristics are policy, not guest logic | corpus: capture of a declared sensitive-region fails closed; keystroke budget exhaustion is typed |
| T3 | Clipboard/IME snooping via keyboard grants | indirect secret capture | `input.keyboard` is a separate grant, printable/control distinction, per-window budget; no clipboard capability exists in v0 at all | authority-mutation corpus: clipboard/IME access attempts → typed refusal (absence is the contract) |
| T4 | Review-fatigue abuse (guest floods preview dialogs to train click-through) | preview becomes rubber stamp | preview/commit tokens are bound to an observation revision; one committed action per token; Host rate-limits token issuance | corpus: N tokens within the rate window → typed limit outcome |
| T5 | Capture exfiltration (frames streamed to a covert channel) | screen surveillance | capture inherits NO network authority; exfiltration requires a separately granted M12 endpoint — visible in grants, auditable | corpus: capture+http composition is refused unless both grants exist; audit shows the association |
| T6 | Audit tampering from the guest side | unaccountable actions | audit stream is a Host-owned resource; guests can append nothing and read nothing | corpus: guest attempts to write/read the audit stream → typed refusal |
| T7 | Emergency-stop suppression (guest keeps executing through stop) | unstoppable automation | emergency stop is Host-owned; guest code cannot observe around or defer it; teardown closes capture/input handles deterministically | corpus: stop during in-flight action → no further input, handles closed, outcome typed |
| T8 | Stale-permission replay (token minted for surface A replayed after revocation or geometry change) | input into the wrong window | tokens bind surface identity + observation revision + action digest + expiry | corpus: replay after revocation/geometry drift → typed stale rejection |
| T9 | Fixture-app hardening pressure (anti-cheat style evasion demanded by apps) | Sico becomes an evasion tool | §6 non-goal: no bypass; the platform ADR draws the Win32-vs-UIA line explicitly so "read the accessibility tree" cannot creep in | non-goal assertion in the RFC's refusal corpus |
| T10 | Scope widening via update (a package swaps the automation guest for an aggressive variant) | grant boundaries bypassed by version drift | grants bind at authorize time to the component identity; updates re-authorize (M7 boundary) | carried M7 corpus; no new mechanism |

## 3. Open findings (ruled at acceptance, 2026-09-10)

- **F-1 (sensitive-field policy) — ruled: declared region classes.** The
  operator's Host policy explicitly declares which surfaces (window
  identity patterns) and which regions of them are sensitive; keyboard
  input into a sensitive-declared surface is a typed `sensitive-field`
  refusal (RFC-0040 `keyboard-error`). Heuristic detection is out of v0:
  heuristics produce silent false negatives, and the house rule prefers
  deterministic, declared policy over invisible guessing. Declared
  policy fails loudly and is auditable; the corpus case is a
  policy-fixture keyboard call that must refuse.
- **F-2 (audit retention) — ruled: Host-local append-only JSONL per run.**
  Audit events append to one JSONL file per run under the Host storage
  root; retention keeps the most recent 32 runs (Host-local policy
  constant, operator-configurable upward only); no network export path
  exists in v0 and the stream stays guest-invisible (T6 boundary). The
  JSONL file itself is the export format, consumable by operator
  tooling only. Both rulings are Host policy, not guest-visible surface,
  exactly as the threat model anticipated.
