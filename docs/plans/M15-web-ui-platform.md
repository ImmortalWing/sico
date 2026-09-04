# M15 Web platform and UI controls

> Status: planned; renumbered from former M14 on 2026-09-04; no STEP numbers reserved

## 1. Objective

Provide a first-class Sico Web/UI path without introducing JavaScript semantics into the language or duplicating Runtime security rules.

## 2. Entry gate

- M14 application-ready language baseline is GO.
- M13 AI tooling covers application-profile source.
- At least one verifiable application/package consumer exists outside compiler implementation fixtures.
- Contract-only exploration may happen earlier; support claims and product implementation may not.

## 3. Required decisions and workstreams

- ADR comparing direct browser Component execution, a controlled webview Host and staged coexistence.
- Compiler-facing UI/WIT bindings over M5 typed UI concepts.
- Deterministic control tree, layout, text, input, focus, event, lifecycle and accessibility contracts.
- DOM/network/storage/event authority that reuses M12 endpoint policy and existing capability closure.
- Escaping and hostile-content corpus; guest strings never become unchecked HTML, script, URL, path or command.
- Browser/Host debugging, source identity, event redaction and resource limits using M10/M11 infrastructure.

## 4. Non-goals

- Treating Sico as JavaScript or HTML syntax sugar.
- Ambient browser cookies, unrestricted DOM ownership or arbitrary script injection.
- Desktop screen automation; that belongs to M16.
- Inferring browser support from a webview prototype.

## 5. Exit gates

1. One business component runs with equivalent core behavior in a declared Web Host and native Host.
2. UI state/event/accessibility behavior is deterministic under the frozen contract.
3. Navigation, network, storage and user-input authority are explicit and least-privilege.
4. Hostile markup/URL/event/size/rate corpora fail closed.
5. Actual-browser performance, lifecycle and crash-isolation evidence exists.
6. Platform claims name exact tested engines and versions.
7. Full M0–M14 regression is green and the exit audit is explicit.
