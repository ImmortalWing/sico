# RFC-0027: Language server and editor protocol v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: local tooling protocol v0
> - supersedes: -
> - superseded-by: -

## Summary

Define a bounded local Language Server Protocol adapter that translates compiler diagnostics, canonical formatting and Semantic Index facts into editor operations without duplicating language semantics.

## Transport and lifecycle

`sico-lsp` communicates only through stdin/stdout using JSON-RPC 2.0 and ASCII `Content-Length` headers separated by CRLF. Bodies are UTF-8 JSON. A message or one open document is at most 1 MiB, all headers are at most 8 KiB, one session holds at most 128 documents and 8 MiB of source, and file URIs are at most 4 KiB. Duplicate lengths, unsupported encodings, malformed/truncated frames and oversized input fail before dispatch.

The server implements initialize, initialized, shutdown and exit ordering. It advertises UTF-16 positions, full-document synchronization, pull and push diagnostics, symbols, completion, hover, definition, references, whole-document formatting and three editor commands. It does not advertise rename, incremental sync, workspace diagnostics, semantic tokens or source debugging.

## Compiler ownership and coordinates

Parser diagnostics retain RFC-0001 E1xxx identities; semantic diagnostics come from `sico-semantics`; document symbols, completion identity, hover and definition come from `sico-index`; edits come only from `sico-format`. The adapter converts authoritative UTF-8 byte ranges to zero-based UTF-16 positions and rejects positions inside a surrogate pair.

Module identities encode the entire file URI so same-name files in different directories cannot collapse. v0 definition and hover require exactly one unqualified compiler symbol; ambiguity returns no result. References are bounded lexical occurrences, but only after exactly one compiler-owned symbol establishes the queried identity. This conservative rule avoids invented cross-module resolution while the compiler index has no reference facts.

## Run and debug workflow

`sico.check` and `sico.run` return `sico <subcommand> <program>` as an argument array with `shell: false`; the language server never spawns a shell or executes a program itself. Paths with spaces remain one argument and control characters are rejected.

`sico.debug` returns typed error `-32004`. The current Runtime exposes no source pause, step, stack or variable inspection hooks, so advertising DAP breakpoints would be false. A later RFC may add a Debug Adapter only after those Runtime hooks exist and can be tested end to end.

## Compatibility and limitations

The implemented subset follows the framing, JSON-RPC, capability negotiation and UTF-16 coordinate rules of the current Language Server Protocol while keeping a small v0 surface. The server is local and stateless across processes. It rebuilds an at-most-8-MiB in-memory index per request; no watcher, network service, background execution, cache persistence or editor-specific extension is included.

## Validation

Acceptance requires eight Rust tests, 24 machine cases, 512 malformed/oversized frame mutations, deterministic key-order behavior, exact compiler/formatter reuse, full workspace Clippy/regression and no external side effects.

## References

- [Language Server Protocol specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/)
- [Debug Adapter Protocol overview](https://microsoft.github.io/debug-adapter-protocol/overview)
- [RFC-0001 diagnostics](./RFC-0001-diagnostics-protocol-v0.md)
- [RFC-0002 Semantic Index](./RFC-0002-semantic-index-query-v0.md)
- [STEP-0067](../steps/STEP-0067-language-server-editor-workflow.md)
