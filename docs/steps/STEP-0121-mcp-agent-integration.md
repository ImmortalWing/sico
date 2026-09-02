# STEP-0121: agent-framework integration layer (MCP)

> - status: complete
> - phase: M13 (parallel support track)
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Objective

Expose the four frozen `sico.ai-tool.v0` operations (inspect / validate_fix / plan_execution / summarize_execution) as MCP tools over stdio through a new `sico-mcp-server` crate, fail-closed exactly like the existing tool, with JSON Schema tool registrations, contract cases (AIT-025+), and module-boundary coverage — adding no new external dependency.

## 2. Context and evidence

- [`M13 plan`](../plans/M13-ai-tooling-closure.md) STEP-0121 deliverables/exit evidence.
- `sico-ai-tools::execute_value` is the single frozen protocol entry: bounded, pure (no fs write / process / network / model calls), typed failures; 512 protocol-mutation corpus proves fail-closed behavior.
- [`ADR-0007`](../adr/ADR-0007-openjdk-style-modular-monorepo.md): every new package needs a module assignment and boundary check.
- MCP is a JSON-RPC 2.0-based protocol over stdio (2025-06-18 schema generation); a server needs `initialize`, `tools/list`, `tools/call`.

## 3. Scope

In scope:

- `crates/sico-mcp-server`: stdio JSON-RPC 2.0 MCP server binary + library, mapping MCP `tools/call` to `sico_ai_tools::execute_value`, JSON Schema registrations for the four tools, protocol-version negotiation, bounded frames;
- module-boundary contract update (new package in the `tooling` module), workspace membership;
- AIT-025+ contract cases; `validate-step-0121.ps1`.

Out of scope:

- any behavior change to the underlying `sico.ai-tool.v0` protocol or its frozen budget/digest semantics;
- network/SSE MCP transports, OAuth, remote servers;
- model calls in either direction.

## 4. Options and decision

### 4.1 MCP implementation strategy

- **Candidate A (chosen): hand-rolled JSON-RPC 2.0/MCP mapping over `execute_value`, zero new dependencies.** The MCP surface we need (initialize handshake, tools/list with JSON Schema, tools/call) is a thin envelope; `serde_json` already in the workspace covers parsing. The security story stays exactly the frozen one — one pure function call per request, bounded input, typed refusals — and the module-boundary validator can hold the crate to `tooling`-only dependencies.
- Candidate B: adopt the official `rmcp`/MCP SDK crate. Rejected for this step: it drags a runtime + async stack into a deliberately synchronous, dependency-minimal tooling module and makes the fail-closed story rest on an unaudited third-party envelope instead of the 512-mutation-tested pure function. Revisit only if a future step needs transports the hand-rolled loop cannot express.
- Candidate C: expose the tool via the existing LSP instead. Rejected: MCP is the plan's deliverable and the agent-framework lingua franca; LSP already has its own STEP-0073-era boundary.

### 4.2 What `tools/call` returns

Chosen: each MCP tool maps 1:1 to one `sico.ai-tool.v0` request; the tool's `arguments` object is transported verbatim as the request `input`, with `schema`/`protocol_version`/`request_id`/`budget` filled by the server (bounded server-side defaults, never widened by the client). The MCP response embeds the exact `sico.ai-tool.response.v0` JSON as text content — no reinterpretation, so digests and diagnostics stay byte-comparable with the direct tool.

## 5. Plan

1. This step doc.
2. `crates/sico-mcp-server` (lib + `sico-mcp-server` bin): envelope, schemas, mapping, tests (initialize/tools list/4 tools/call roundtrips, oversize/unknown-tool/bad-envelope refusals, 512-mutation corpus through the MCP transport).
3. Module-boundary contract + AIT-025+ cases + registries.
4. `tools/validate-step-0121.ps1`, full regression, commit.

## 6. Changes

- `crates/sico-mcp-server` (new, 26th workspace package, `tooling` module): stdio JSON-RPC 2.0 MCP server with zero new external dependencies (serde_json only). Methods: `initialize` (protocol `2025-06-18`, server identity), `ping`, `tools/list` (the four frozen tools with JSON Schema `inputSchema` registrations), `tools/call` (1:1 mapping onto `sico_ai_tools::execute_value`; the client controls only `operation` name and `input` payload — `schema`/`protocol_version`/`request_id`/`budget` are filled server-side and never widened). The `sico.ai-tool.response.v0` JSON is embedded verbatim as MCP text content with `isError` mirroring `ok`. Notifications stay silent; unknown methods/arguments are typed JSON-RPC errors; the binary is a bounded line-framing loop.
- `tests/architecture/module-boundaries.json`: `sico-mcp-server` assigned to `tooling`; module-boundary validator green at 26 packages.
- `ai-eval/tooling-cases.json`: AIT-025–AIT-029 added (transport mapping, budget non-widening, 512-mutation through MCP, real stdio session roundtrip, verbatim response embedding).
- `tools/validate-step-0068.ps1`: corpus count 24→29 and test-count invariant 8→12 (ai-tools gained its own protocol tests since 0068 froze the number; the oracle's substance is unchanged and reruns green).
- `tools/validate-step-0121.ps1` (new): fmt/clippy, MCP crate tests, ai-tools regression, module boundaries (26), 0068 oracle, AIT case presence, fail-closed invariants, dependency-minimality guard (rejects tokio/rmcp/reqwest/hyper).

## 7. Validation

Executed 2026-09-02 on Windows x64 GNU, Rust 1.98.0:

- `validate-step-0121.ps1` green: `STEP_0121_OK mcp-stdio=4-tools schema=registered mutations-512=fail-closed-through-mcp session=roundtrip packages=26 ai-tools=regression-green cases=29`.
- MCP crate tests 7/7, including the full frozen 512-mutation corpus replayed through the MCP transport (all typed-refused — the envelope adds no bypass on client-controllable fields) and a real stdio session performing initialize → tools/list → inspect (diagnoses a missing `end function`) → validate_fix (clean fix accepted; stale-digest non-fix typed-refused).
- Module boundaries green at 26 packages; STEP-0068 oracle rerun green; the underlying `sico.ai-tool.v0` protocol, its budgets and its 512-mutation direct corpus are byte-identical to STEP-0068's freeze.

## 10. Audit links

- [`M13 plan`](../plans/M13-ai-tooling-closure.md)
- [`ADR-0007`](../adr/ADR-0007-openjdk-style-modular-monorepo.md)
- [`STEP-0068`](./STEP-0068-ai-tooling-measured-evaluation.md), [`STEP-0119`](./STEP-0119-ai-generation-quality-baseline.md)
