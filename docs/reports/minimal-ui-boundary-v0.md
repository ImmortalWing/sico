# Minimal UI WIT and renderer boundary v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M5

## Result

GO for STEP-0050. The Host accepts one strict typed UI tree, produces a renderer-neutral preorder plan with escaped text/accessibility order, and gates UI events by node type, payload, queue and rolling-rate limits. The WIT package parses with the pinned 0.253 toolchain.

## Security evidence

- markup-like text becomes escaped text and is never evaluated；
- duplicate/invalid IDs, nested/deep windows, control characters and missing interactive accessibility labels are rejected；
- button/input event kinds cannot be swapped；
- 121st event in one second and 257th queued event are rejected；
- unknown schema/node variants fail closed through strict serde/WIT contracts。

## Boundary

The render plan is platform neutral and is not itself a native window implementation. STEP-0051 must map it to Windows controls without reintroducing HTML/script interpretation or weakening ceilings.
