# STEP-0067: Language server, editor and debugging workflow

> - status: complete
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Provide a real local language-server/editor boundary over existing compiler contracts, plus truthful check/run/debug workflow behavior.

## 2. Scope

Included: stdio LSP framing; lifecycle; full document sync; push/pull diagnostics; UTF-16 conversion; symbols, completion, hover, unique definition and bounded references; canonical formatting; shell-free check/run argv; explicit debug refusal; protocol/resource limits; corpus, RFC, review and validator.

Excluded: editor-specific extension packaging, incremental edits, semantic tokens, rename, workspace watchers, persistent cache, network transport and source-level DAP.

## 3. Plan

1. Freeze bounded protocol and compiler ownership. — complete
2. Implement `sico-lsp` framing and lifecycle. — complete
3. Adapt diagnostics, Semantic Index and formatter. — complete
4. Implement deterministic editor command records. — complete
5. Review ambiguity, coordinates, versions and resource limits. — complete
6. Add corpus, mutation tests, RFC, report and regression. — complete

## 4. Changes and validation

Added the `sico-language-server` workspace crate and `sico-lsp` binary. Eight tests pass, including 512/512 malformed/oversized frame mutations. The machine catalog contains 24 unique cases. Full-file changes are versioned; documents and source bytes are bounded; UTF-8 compiler spans roundtrip through UTF-16 editor positions; navigation refuses ambiguous names; formatting calls `sico-format`; check/run never invoke a shell; debug is explicitly unavailable rather than falsely advertised.

Result: `STEP_0067_OK tests=8 mutations=512 cases=24 lsp=stdio positions=utf16 diagnostics=compiler index=compiler format=canonical shell=false debug=explicit-unavailable external_side_effects=0 next=STEP-0068`.

## 5. Risks

The current Semantic Index has definitions but not reference facts, so v0 references are conservative lexical occurrences gated by a unique compiler symbol. Runtime lacks source debugging hooks; DAP remains unimplemented. No editor marketplace artifact or real user telemetry is included.

## 6. Links

- [RFC-0027](../rfc/RFC-0027-language-server-editor-protocol-v0.md)
- [review](../reports/language-server-editor-workflow-v0.md)
