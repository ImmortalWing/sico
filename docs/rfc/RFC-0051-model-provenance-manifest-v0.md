# RFC-0051: M17 gate 4 — model/package provenance manifest v0

> - status: **draft — owner acceptance required before any implementation STEP**
> - date: 2026-09-25
> - phase: M24 §8.2 (M17 gate 4 model/package provenance RFC)
> - depends: RFC-0041 (image data contract), RFC-0015/0024/0026 (M7 digest, signed registry, dependency-lock discipline), ADR-0012 (live-model budget record), M24 plan §8.2, STEP-0287 inventory

## 1. Summary

Freeze the asset-manifest schema that lets a **fixture model** satisfy
M17 gate 4 end to end — digest-verified load, typed refusals, budget
enforcement, typed cancellation, malicious-asset refusal — while keeping
every live-model claim externally gated exactly as M13/M17 recorded them
(ADR-0012 budget gap B-repair 0.833 stays a recorded product fact).

## 2. Manifest schema (`sico.m17.provenance.v0`, carried inside the package)

```text
asset:
  name: Text                      // package-unique
  kind: enum { model, table }     // v0: fixture models and lookup tables only
  media: Text                     // e.g. "application/x-sico-fixture-model"
  sha256: Text                    // lowercase hex, M7 discipline reuse — no parallel trust path
  size: U64                       // bytes; must match the payload exactly
  provenance:
    origin: Text                  // where the bytes came from (URL or "in-repo generated")
    license: Text                 // SPDX identifier or "in-repo"
    created: Text                 // ISO-8601 date
    generator: Text               // tool/family that produced it ("hand-authored fixture" for v0)
  budgets:                        // all enforced by typed refusal, all required
    max_memory_bytes: U64
    max_wall_ms: U64
    max_hostcalls: U64
  interface: Text                 // the package function names that may load this asset
```

- Manifests are **closed**: unknown fields refuse at load; missing
  required fields refuse; `size`/`sha256` mismatches refuse (limit+1:
  oversized `size` and oversized payload are distinct refusals).
- Digest verification happens at **package install** (M7, existing) and
  again at **asset load** (guest-visible, typed) — the two share one
  algorithm and one hex form; no second trust path is introduced.

## 3. Evidence set (fixture model; the gate's satisfaction bar)

On the frozen fixture corpus, through the real runner:

1. digest-verified load succeeds and yields the declared interface;
2. bad digest / truncated payload / oversized payload (+1 over `size`)
   each produce their single typed refusal, no partial load;
3. budget enforcement: a run exceeding any declared budget gets the
   typed limit refusal (hostcall-count and wall-time brackets included);
4. cancellation mid-inference: typed outcome, no orphaned work (no
   partial writes, no leaked Host-side state);
5. malicious-asset probes: signature-confused bytes, manifest/payload
   mismatch, path-like names, unknown fields — all refused.

## 4. Honesty gates

- A fixture model satisfying this gate flips **gate 4 only**; M17 overall
  GO still requires gate 2 (externally owner-gated) and the roster
  packages (§8.3) on their own corpora.
- No live-model, no quality, and no speed claim arises from this RFC or
  its implementation; provenance records what a model **is**, never what
  it achieves (those claims stay behind the M13/ADR-0012 external gates).
- Non-goals (v0): downloading at runtime, multi-file models, quantization
  metadata, GPU residency declarations, license enforcement beyond
  recording, any inference acceleration.
