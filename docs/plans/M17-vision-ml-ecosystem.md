# M17 Vision and model package ecosystem

> Status: planned; owner-approved 2026-09-04; no STEP numbers reserved

## 1. Objective

Provide deterministic image-processing packages and optional bounded model inference through the versioned package/Component ecosystem. Vision is not core syntax and model inference is not an implicit dependency.

## 2. Entry gate

- M14 is GO.
- M16 has at least an accepted read-only capture prototype and frozen image ownership/size limits.
- M7 package identity, dependency lock, update and provider trust can carry binary implementations and model assets.
- Native/GPU providers require an ADR and actual runner evidence.

## 3. Layers

### 3.1 Portable data contract

- Image, pixel format, color space, stride, orientation and bounded region types.
- Owned/borrowed resource rules and exact allocation limits.
- Canonical serialization only where copying is explicitly affordable.

### 3.2 Deterministic vision packages

- Color conversion and thresholding.
- Resize/crop and bounded convolution.
- Template, grid and contour primitives.
- Reference implementations with fixed corpora and explicit numeric tolerances.

### 3.3 Optional accelerated providers

- OpenCV or equivalent behind a versioned Component/Host provider.
- No provider may receive broader filesystem, network, screen or input authority than its caller grants.
- Provider crash, cancellation and memory exhaustion remain isolated.

### 3.4 Model inference packages

- Model/weight digest, provenance and format/version identity.
- Explicit CPU/GPU/memory/time and output-size budgets.
- Deterministic or tolerance-labelled results and a non-model fallback when the application permits it.
- No automatic download, telemetry or ambient credential access.

## 4. Acceptance corpus

- Synthetic and captured grid boards across scale, color and compression variants.
- Hostile dimensions/stride/metadata and decompression-bomb cases.
- Reference vs accelerated result comparison.
- Model corruption, unsupported operator, cancellation and out-of-budget cases.

## 5. Non-goals

- Adding image, OpenCV, GPU or model primitives to core language syntax.
- Giving vision packages ambient screen, filesystem, network or input authority.
- Making a model mandatory when deterministic geometry/color algorithms satisfy the task.
- Claiming cross-platform acceleration without hardware-specific runner evidence.

## 6. Exit gates

1. Portable baseline completes the fixed corpus without model inference.
2. Accelerated paths match reference results within frozen tolerances.
3. Package consumers do not modify compiler/Runtime core.
4. Model and provider authority remains explicit and least-privilege.
5. Resource, crash and malicious-asset corpora fail closed.
6. Performance claims identify exact hardware/provider/version.
7. Full M0–M16 regression is green and the exit audit is explicit.
