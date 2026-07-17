# STEP-0078: General executable control and function codegen

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - owners: autonomous-agent

## 1. Objective

Extend the verified backend from entry-block scalar expressions to general executable functions and structured control flow, using STEP-0077 dynamic `Bool/I64/U64` storage without reintroducing compile-time-constant assumptions.

## 2. Context

STEP-0077 proved dynamic parameters, locals, fixed-width results and checked numeric failures. The backend still refuses `Call`, entry `Jump`/`Match`, non-return branch blocks and all data operations. Arbitrary-precision `Int` remains constant-only until its Runtime representation is separately implemented.

## 3. Scope

Included:

- direct calls between verified IR functions;
- dynamic `Bool/I64/U64` arguments and results across calls;
- general verified branch/jump/match CFG emission, including loop/back-edge policy;
- Core Wasm representation for `Construct`, `Project` and `Variant` sufficient for internal control/data flow;
- verifier-driven refusal for CFG or value transport that cannot yet be represented;
- deterministic source/IR/backend tests and real Wasmtime execution.

Excluded:

- arbitrary-precision dynamic `Int`;
- public Text/Bytes/List/record/Result Canonical ABI, allocation and randomized boundary roundtrips (STEP-0079);
- Script adapter packaging, manifest v1, runner, CLI and standard library;
- async/resource execution.

## 4. First actions

1. Inventory currently lowerable `Call`, `Branch`, `Jump`, `Match`, `Construct`, `Project` and `Variant` shapes and verifier dominance constraints.
2. Freeze a reducible-CFG or dispatcher-loop lowering strategy and explicit refusals for unsupported graphs.
3. Define internal aggregate slot/layout rules without claiming Component ABI support.
4. Add mutation and runtime oracles before widening backend dispatch.
5. Execute multi-function, nested branch, loop and enum-match fixtures in Wasmtime 46.0.1.

## 5. Exit gate

- no backend decision depends on a value being a compile-time constant unless the IR operation explicitly requires it;
- direct calls and representative control graphs execute with exact Wasmtime results;
- malformed or unsupported graphs fail with typed backend errors, never malformed Wasm;
- STEP-0077 numeric behavior and the full workspace regression remain green;
- aggregate Component ABI remains honestly deferred to STEP-0079.

## 6. Links

- [`STEP-0077`](./STEP-0077-fixed-width-dynamic-scalars.md)
- [`fixed-width evidence`](../reports/fixed-width-dynamic-scalars-v0.md)
- [`M8 plan`](../plans/M8-script-profile.md)
- [`RFC-0008`](../rfc/RFC-0008-typed-sico-ir-contract-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`general executable codegen evidence`](../reports/general-executable-codegen-v0.md)
- [`STEP-0079`](./STEP-0079-script-aggregate-canonical-abi.md)

## 7. Completion record

- Direct calls use an explicit canonical `FunctionId -> Wasm function index` map; dynamic `Bool/I64/U64` values cross supported call boundaries without constant folding.
- Every verified block is emitted through a deterministic dispatcher loop. `Jump`, `Branch`, `Match`, non-return intermediate blocks and back-edges share one lowering path.
- Internal records flatten named fields into Wasm locals. `Construct` now preserves field names in verified IR, and `Project` copies the selected field slots.
- Internal variants use deterministic module-local tags plus flattened payload slots. Match supports Boolean, wildcard and variant-tag decisions; payload binding/inspection remains a typed refusal.
- Aggregate parameters, results and call transport remain refused because they are public/lift-lower ABI work for STEP-0079, not an internal-local representation claim.
- `tools/validate-step-0078.ps1` executes seven exact cases in Wasmtime 46.0.1 and proves the back-edge policy with a bounded-fuel trap.
