# M23 Vision/model runtime and native AI-loop closure

> Status: planned by owner directive "完成M22，规划M23-24" (2026-09-16); entry requires M22 GO and the named platform/model inputs; no STEP numbers reserved

## 1. Objective

Close the implementation debt deliberately left by M17/M18: a deterministic
vision package roster, an isolated and budgeted model-provider path, and one
real Windows observe → plan → preview → execute-one → verify/stop application
that composes M14, M16 and M17 without patching compiler, Runtime or Host cores.

M23 is not a new language-feature milestone. Image/CV/model behavior remains in
versioned packages or least-privilege providers; deterministic vision remains
the default when it solves the task.

## 2. Entry gate

1. M22 is GO with reproducible A=B=C bootstrap evidence and its M7-packaged
   compiler artifact.
2. M16 Windows capture/input loop remains green on the pinned fixture.
3. M17 RFC-0041/RFC-0043 image contract and reference package remain green.
4. The owner supplies or explicitly declines the acceleration SDK/runtime and
   model assets/credentials used for live evidence. Contract work may proceed
   without them, but cannot create a platform/model support claim.
5. No unresolved security regression exists in M7, M11, M12 or M16.

## 3. Work tracks

### 3.1 Deterministic CV package completion

- Complete the accepted image package roster: bounded resize/color conversion,
  template match, connected regions/contours and grid extraction.
- Freeze canonical input/output, tie-breaking, overflow, memory and limit+1
  behavior in a package RFC amendment before implementation.
- Run byte-exact reference corpora and the block-game recognition corpus with no
  model dependency.

### 3.2 Optional acceleration provider

- Accept a cross-component ADR naming the provider boundary, CPU reference
  fallback, native FFI isolation crate, cancellation and crash containment.
- Require result equivalence within a named numeric tolerance; unsupported
  devices fall back or typed-refuse, never silently change algorithms.
- Record each claimed OS/architecture/device only from an executed native run.

### 3.3 Model package and inference contract

- Accept the deferred M17 model RFC: model/weights digest, format/version,
  provenance, license metadata, input normalization, deterministic controls,
  CPU/GPU/memory/time/token budgets and cancellation.
- Treat weights and metadata as hostile inputs. Unknown fields, truncation,
  trailing bytes, overflow, tensor-shape mismatch and limit+1 all fail closed.
- Keep model inference explicit in source/package authority; it may not replace
  the deterministic path implicitly.

### 3.4 Native application closure

- Compose the block-game fixture through M16 capture, deterministic M17
  recognition, the M14 Sico solver, preview, one authorized input action and
  post-action verification.
- Preserve separate capture/input grants, one-action atomicity, stale/duplicate
  frame rejection, drift detection, emergency stop and chained audit digest.
- Package and execute the application only from an M19-style release bundle.

### 3.5 Evidence and tooling

- Add reproducible one-command validators, raw budget tables and machine support
  matrix entries.
- Run live-model evidence only when the exact model/version/prompts/corpus,
  token/cost budget and raw outputs can be retained.

## 4. Exit gates

1. Deterministic CV roster passes byte-exact fixed and hostile corpora.
2. Reference and accelerated results match the accepted tolerance on every
   claimed device; provider cancellation/exhaustion/crash stays isolated.
3. Model loader/inference rejects malformed assets and enforces all declared
   budgets; provenance and digest bind every accepted run.
4. The Windows native block-game loop completes capture → recognition → Sico
   plan → preview → one action → verify/stop with injected stale, drift,
   timeout, cancellation and permission-loss cases.
5. A clean release-bundle consumer runs without compiler/Runtime/Host patches.
6. M0-M22 regression and M23 validators are green.
7. An explicit GO/NO-GO audit distinguishes `internal-fixture`,
   `clean-room-consumer`, `external-pilot` and `production` evidence.

## 5. Non-goals

- Core language image/model syntax.
- A general desktop agent, unrestricted desktop capture, clipboard, credentials,
  process control, background keyboard access or multiple speculative actions.
- CAPTCHA/authentication bypass, anti-cheat evasion or covert monitoring.
- macOS/Linux/Android acceleration claims without native evidence.
- Treating synthetic/offline evaluation as live-model evidence.

## 6. Expected evidence

- Versioned RFC/ADR and support-matrix entries.
- Deterministic CV content corpus plus malformed/limit+1 corpus.
- Provider parity, cancellation, resource and crash-isolation results.
- Block-game chained capture/action/verification evidence.
- Release-bundle install/run record and explicit M23 exit audit.
