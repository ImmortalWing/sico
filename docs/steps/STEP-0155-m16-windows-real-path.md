# STEP-0155: M16 Windows real path — fixture loop closed with raw evidence

> - status: complete (gate 1 substance achieved; full exit audit still open)
> - phase: M16 plan §5.4/§5.5 (fixture app + real Windows loop per accepted ADR-0013+A1)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - artifacts: new crate [`crates/sico-automation-fixture/`](../../crates/sico-automation-fixture/); Win32 adapter `crates/sico-automation-host/src/win32.rs`; driver `crates/sico-automation-host/examples/real_loop.rs`; raw evidence [`docs/evidence/m16/`](../evidence/m16/)

## 1. What was done

- **Fixture app** (§5.4): `sico-automation-fixture` — a deterministic
  480x320 window (class `SicoFixture`, increment/reset buttons at fixed
  client rects, `count=<n>` frame, no timers, no hardening). Pure
  windows-sys; the only unsafe home besides the adapter.
- **Win32 adapter** (ADR-0013 as amended): class-bound enumeration
  (`FindWindowW` on `SicoFixture` — same-title foreign windows are
  invisible), client geometry, GDI `PrintWindow`
  (`PW_CLIENTONLY|PW_RENDERFULLCONTENT`, top-down BGRA8 DIB; A1's v0
  capture), and commit via one atomic `SendInput` sequence (absolute
  move + press + release) after commit-time foreground promotion
  (synthesized ALT unlock for the foreground lock) — the bounded action
  of ADR-0013 §3.
- **Real loop** (`examples/real_loop.rs`): observe → capture (digest) →
  preview (Host mint; dry-run withholds) → execute-one → capture →
  verify. Raw JSONL per run into `target/evidence/step-m16-real-loop/`,
  committed copies under `docs/evidence/m16/`.

## 2. Raw evidence (committed)

- `real-loop-commit-1.jsonl`: before `5360715f…` → after `b3be5a2c…`
  (one committed click, frame changed, verify matched).
- `real-loop-commit-2.jsonl`: before `b3be5a2c…` → after `f7ec8d7b…`
  (chained: the after-state of run 1 is exactly the before-state of
  run 2 — the fixture counter genuinely persisted and advanced).
- The dry-run record (mint refused `Permission`, zero input events) is
  part of the committed corpus from STEP-0150's session log.

## 3. Gates addressed

- Gate 1 substance: the loop closed on the real Windows desktop against
  the controlled fixture app with raw logs (this STEP; full M16 exit
  audit still requires the drift/stale/revocation corpora on the real
  adapter, cancellation-recovery, long-run budgets, and gate 7
  regression — next STEPs).
- Gates 2–4 synthetic halves: STEP-0150; their real-adapter halves are
  follow-up work reusing this driver.

## 4. Declared limits

- Windows x64 only (ADR-0013); other platforms unsupported.
- `SendInput` moves the real cursor for one bounded action per run;
  dry-run (default) emits zero input events.
- The 0.1–1 frame-digest equality is the verify oracle at this stage
  (deterministic fixture); region-digest tolerance per RFC-0040 §2
  arrives with the CV chain (M17).
