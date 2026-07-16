# Language server and editor workflow v0 review

> - status: complete
> - step: STEP-0067
> - date: 2026-07-16

## Result

`sico-language-server` and the `sico-lsp` stdio binary implement a real bounded LSP subset. Eight tests cover lifecycle, framing, document synchronization, compiler diagnostics, UTF-16 conversion, Semantic Index-backed symbols/completion/navigation, canonical formatting, deterministic editor commands and explicit debug refusal. All 512 malformed or oversized frame mutations fail closed; the 24-case catalog is unique and mechanically validated.

## Review findings

- Protocol input is capped at 1 MiB body/document, 8 KiB headers, 128 documents, 8 MiB total source and 4 KiB file URI.
- Full-document changes require strictly increasing versions; rejected changes do not mutate state.
- Same-stem files receive distinct full-URI module identities.
- Ambiguous unqualified definitions, hover and references return no result instead of choosing an arbitrary symbol.
- Diagnostics and symbols are compiler products; formatting is parse-success-only canonical output.
- Check/run results are argv records with `shell: false`; the server has no process or network side effect.
- Source debugging remains an explicit typed refusal because Runtime pause/step/inspect hooks do not exist.

## Residual risk

Reference locations are bounded lexical matches after compiler identity validation, not compiler-produced reference facts. Index rebuilding is intentionally simple and bounded but has no latency SLA. No VS Code/JetBrains extension, workspace watcher, incremental text sync, localization, DAP, remote transport or persistent cache is claimed.

## Evidence

`STEP_0067_OK tests=8 mutations=512 cases=24 lsp=stdio positions=utf16 diagnostics=compiler index=compiler format=canonical shell=false debug=explicit-unavailable external_side_effects=0 next=STEP-0068`

## Links

- [RFC-0027](../rfc/RFC-0027-language-server-editor-protocol-v0.md)
- [STEP-0067](../steps/STEP-0067-language-server-editor-workflow.md)
