# RFC-0020: Desktop UI and permission contract v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target phase: M5
> - supersedes: -
> - superseded-by: -

## Summary

Desktop permission UI consumes only verified capability closure, keys decisions by immutable app/revision capability identity, and exposes a bounded typed UI model rather than guest HTML, scripts or shell strings.

## Permission semantics

For each requested capability, the host decision is `deny`, `allow-once` or `allow-persistent`. `allow-once` is memory-only and expires at terminal lifecycle state. Persistent records use canonical versioned JSON and include app identity key, signer identity, capability fingerprint, decision set and creation revision. Unknown schema, unknown capability, missing entry, identity mismatch or fingerprint drift fails closed and requires a new prompt.

The prompt displays app ID/version, signer fingerprint, package digest and exact sorted capability names from `AuthorizedPackage`. It never trusts raw manifest text when closure verification failed. Denial occurs before storage creation or guest process launch.

## Minimal UI model

M5 UI v0 is a deterministic tree of typed nodes: `window`, `column`, `row`, `text`, `button`, `input` and `status`. Text remains text; there is no HTML/CSS/JavaScript evaluation. Stable limits are 1 window, 1,024 nodes, depth 32, 64 KiB total UTF-8 text, 4 KiB per text value, 256 queued events and 120 accepted events/second.

Node IDs are non-empty ASCII `[A-Za-z0-9._-]`, unique and at most 64 bytes. Events reference an existing interactive node and carry at most 4 KiB UTF-8. Accessibility label is required for interactive nodes; traversal order is deterministic preorder.

## Platform mapping

Windows may initially map permission and representative UI through OS-native dialog/control adapters. macOS/Linux adapters must preserve the same model, limits, event identities and accessibility order. Platform-specific visuals are non-normative; capability decision and lifecycle results are normative.

## Security

Control characters other than newline/tab in display text are rejected. Markup-like content is displayed escaped as text. Guest-supplied paths, command fragments, URLs and event payloads are never evaluated by a shell. Oversized/deep/duplicate/event-flood input is rejected before renderer mutation.

## Compatibility

Schema names are `sico.desktop.permission.v0` and `sico.ui.model.v0`. New node or capability semantics require a new RFC/schema version; readers reject unknown variants rather than guessing.

## Links

- [`STEP-0046`](../steps/STEP-0046-desktop-host-threat-lifecycle-contract.md)
- [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
