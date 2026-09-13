# RFC-0039: Source modules, package resolution and user WIT bindings v0

> - status: accepted
> - accepted: 2026-09-08 (owner, session directive "开始" on the presented D0/D1 decision points; D1 resolved to the typed refusal)
> - date: 2026-09-07
> - phase: M15 prerequisite track (M15 plan §3.0; the M14 §3.3 carry-over the STEP-0141 scope-honesty note left unclaimed)
> - depends: RFC-0012 (Component/WIT boundary), RFC-0013 (async/resource mapping), RFC-0015/0024/0025/0026 (package format, registry, update, lock), RFC-0033 (top-level syntax decision), RFC-0038 (application profile)
> - implemented (partial): §2.1 modules + §2.5 D1 refusal + the E8xxx module-link diagnostics landed in STEP-0143; §2.2 full check-time resolution landed in STEP-0144; §2.3 packages + §2.4 user WIT (with the A7/A8 amendments) land in STEP-0147

## Accepted amendments (2026-09-08, at acceptance/implementation time; A7/A8 added 2026-09-10 at STEP-0147)

- A7 (§2.4): the v0 user-WIT value set is the **flat subset**: parameters
  are `Bool`, `I64`, `U64`, `Text`, `Bytes`, or `List[Text]`; returns may
  additionally be `Unit` or `Result[ok, Text]` with
  `ok ∈ {Unit, Bool, Text, List[Text]}`. Nested compositions, `Option`,
  records/enums across the boundary, numeric lists (`List[I64]` et al —
  the guest local layout materialises `List[Text]` only), maps, async,
  and resource shapes stay outside v0 with the typed
  `E-wit-unsupported-type` refusal (E8017). Rationale: every remaining
  shape flattens to trivial core slots (all `i32` for variant payloads)
  under the fs/stream/http return-area convention, which keeps the
  canonical-ABI surface exactly as proven by the frozen M8/M12 corpora.
  Boundary names are kebab-case: source `Csv`/`parse_line` render as
  `csv`/`parse-line` (deterministic, injective; component extern names
  require kebab).
- A7b (§2.5 D1 refinement, 2026-09-10): the E8019 `export function`
  refusal fires only for **script-profile programs** (sources defining
  the `main` entry). The M2-era effects/revision oracles accept
  export-prefixed boundary-style functions without `main`, and accepted
  oracles outrank a blanket refusal; the refinement keeps D1's intent —
  no silent user exports in script programs — while leaving the
  check-only corpus untouched.
- A8 (§2.3): the v0 source lock is `sico-lock.json`
  (`sico:source-lock:v0`), a deterministic **projection** of the RFC-0026
  graph: per package exactly `{name, version, path, sha256}` over the
  *signed* package artifact (deny-unknown-fields, explicit paths, no
  acquisition). Shape validation (E8016) is producer-stamped: the
  package builder embeds the interface identity and the SHA-256 of the
  canonical interface specification text in a `sico:user-interface`
  custom section; the CLI compares the stamp against the source-declared
  interface, and the runner re-checks arities fail-closed at link time.
  The trust chain — owner-curated lock → exact artifact digest →
  producer-stamped digest → declared interface — is explicit; a full
  structural component-type walker stays future work and its absence is
  declared here rather than hidden.

- A1 (§2.1): the grammar has no brace delimiters, so `use <module>.{a, b}`
  becomes one `use <module>.<item>` declaration per line.
- A2 (§2.1): the CLI entry file is the program root and does not declare a
  module (E8010 otherwise); every file pulled in via `use` must declare a
  matching `module <name>` and its basename must equal the module name.
- A3 (§2.1): references to imported items are written qualified
  (`module.item`); the `use` declaration declares the dependency edge and
  is verified (module exists, item exported, no cycle, within bounds). A
  qualified reference without a matching import keeps today's build-time
  "unsupported call target" refusal; moving it to check time is §2.2's
  follow-up slice.
- A4 (§2.1): type imports are refused in v0 (E8011); imports carry
  functions only. Cross-module type/constructor sharing is a later slice.
- A5: module-link diagnostics use the reserved E8xxx family (E8001–E8011
  as implemented in STEP-0143); parser shapes are E1014/E1015.
- A6 (new defect record): the script profile's ABI seam for user
  functions returning `Result[I64|U64, NumericError]` was found broken
  while landing STEP-0143 (packed value layout vs full flat declared
  width → invalid WebAssembly; no frozen corpus ever exercised it). The
  shape is now a typed build refusal; the repair is a dedicated defect
  step outside this RFC's slices.

## Summary

Close the source/backend gap that M15's compiler-facing bindings depend on but M14 did not deliver: source modules with explicit imports, versioned package resolution through the M7 trust boundary, and source-level user WIT imports. The RFC also gives the already-parse-accepted `interface` declaration real semantics — today it is silently dropped (§1 finding F3), which is the exact "silent discard" class RFC-0033 excluded for statements.

Everything not frozen here keeps a typed refusal with a stable identity. No build scripts, no native FFI, no ambient downloads, no cyclic dependencies.

## 1. Measured inventory (2026-09-07, sico 0.0.2-dev, Windows x64)

Live probes through `sico check/build/outline` (reproduction in STEP-0142 §2); source evidence in `sico-lexer`, `sico-parser`, `sico-semantics`, `sico-codegen-wasm`, `wit/`.

| # | Finding | Evidence |
|---|---|---|
| F1 | **The compilation unit is exactly one file.** `check/build/run` accept a single `<FILE|->`; a second path is a clap error. HIR's `Module` is the file; nothing links files. | probe: `sico check a.sico b.sico` → `unexpected argument` |
| F2 | **No module/namespace syntax.** `module`, `use`, `import` are not keywords; a top-level `module helpers` / `use sico.component` line yields `E1013 top level accepts declarations only` (the RFC-0033 refusal, reused because the line lexes as non-declaration tokens). | probe: `module-use.sico` → 2× E1013 |
| F3 | **`interface` declarations parse, appear in outline, and are silently dropped by semantics.** `sico-semantics` matches `DeclarationKind::Interface => {}`; `check ok` on an interface-only declaration; codegen ignores it. The `syntax-candidates/b/component-call/valid/wit-safe-interface.sico` oracle says *accept* — accepting the syntax is per-oracle, but accepting it *as meaning nothing* is an undeclared behavior gap of the same class M14 gate 1 polices. | probe: `wit-interface.sico` → `check ok`; outline lists `interface Formatter`; semantics lib.rs:286 |
| F4 | **Call-target resolution happens at codegen, not check.** A call to an undefined function or constructor passes `check` and is refused at `build` (`unsupported call target ScriptOutput at bytes …`, exit 2). Cross-file references therefore fail late and without import-aware spans. | probes: `uses-lib.sico` check ok → build refusal |
| F5 | **Other parse-accepted declaration surface:** `capability` blocks (stored in the semantic model, consumed only for `Type::Capability` naming at lowering), `export function` prefix (no distinct semantic identity), `using` blocks (function-body block kind; E1013 at top level). These are M4/M5-era surface; each must be classified by this RFC, not left ambiguous. | probes: `cap2.sico`, `export2.sico`, `using2.sico` |
| F6 | **The emitted world is fixed.** `sico:script/program@0.1.0` (types, fs-read/fs-write, streams, http; `run` export) plus the buffered `sico:script/http@0.2.0 request` import emitted when `sico.http2.request` is used (STEP-0136). There is no user-defined WIT mechanism anywhere in the pipeline. | `wit/script-profile-v0/world.wit`, `wit/script-http2-v0/http2.wit`, codegen flags |
| F7 | **Packages carry built Components, not source.** `sico-app` builds `.sapp` from a WebAssembly Component; registry/signing/lock (RFC-0024/0025/0026) operate on artifacts. Source-level dependency declaration and resolution do not exist. | `crates/sico-app-cli` subcommands |
| F8 | **The M5-era component design history exists as oracles, not commitments.** `examples/component-plugin/` (`interface X version N`, `component Y implements X@N`, `Component<T@N>`) and `syntax-candidates/b/component-call/` (COMP cases) sketch a past candidate grammar in a different dialect. This RFC must reconcile or explicitly supersede them; examples remain design history per the repository rules. | corpus files |
| F9 | Scale markers: 42 literal `sico.*` intrinsic names plus the `sico.map/set` generic families; the codegen typed-refusal table measured at 126 sites in STEP-0129 has grown with the M14 slices (~159 refusal mentions today). | grep counts 2026-09-07 |

## 2. Proposal

### 2.1 Source modules

- One file is one module. Every module file starts with an explicit `module <name>` declaration; the file basename must equal `<name>` (deterministic discovery, no path magic, no directory-implicit modules in v0).
- `use <module>.<item>` (or `use <module>.{item, …}`) imports items into scope under their own name; references stay qualified (`helpers.double`) in v0 — no re-export, no glob, no renaming.
- The compilation unit of `sico check/build/run/test` becomes the transitive closure of the entry file's `use` graph, resolved from the entry file's directory. Cycles are a typed error (`E-module-cycle`). The use-graph depth and module count are bounded (limit+1 tested).
- `module` and `use` become reserved keywords. This is a versioned grammar change satisfying the RFC-0033 reconsideration gate: explicit desugaring is none (declarations map directly), source-map identity, formatter idempotence, outline/LSP symbols and AI-tooling evidence are exit-corpus items (§4), and pre-existing source that used `module`/`use` as identifiers receives a typed diagnostic with span, not silent reinterpretation. The E1013 refusal for stray top-level non-declarations is unchanged otherwise.

### 2.2 Check-time name resolution (a declared check-surface tightening)

Call targets resolve at check: every callee/constructor must be bound (local, imported, or an exposed intrinsic) or `check` fails with a stable diagnostic. Today's F4 late failure ("undefined name passes check, build refuses") becomes a check-time error with import-aware spans. This is a declared tightening of the check contract: valid programs are unaffected; the class of programs that change behavior is exactly the one that could only fail later. The application-profile matrix gains entries for this.

### 2.3 Package resolution

- A source dependency is declared as `use pkg <name> version <requirement>`; resolution reads the M7 lock file (RFC-0026) and the vendor/registry trust configuration (RFC-0024/0025). A dependency is a **signed prebuilt Component** — never source, never a build script. Compilation validates the component's exports against the user WIT interface the source declares (§2.4) and refuses shape mismatch with a typed identity.
- Plain `sico run/test` and `sico-app` compose read the same lock; no second resolution mechanism is introduced. No automatic download in v0: missing packages are a typed `E-package-missing`; acquisition is an explicit, owner-configured step.
- Bounds: package count per program and total declared import size are limited (limit+1 tested).

### 2.4 User WIT imports

- An `interface` declaration becomes a user WIT interface with identity `sico:user/<interface>@<version>` (version spelled on the declaration, `interface Formatter version 1` — the M5-oracle spelling, now given semantics). The interface body maps 1:1 to WIT (functions with named params and result types); it is not a second ABI.
- Source imports a package's interface as an instance: `use pkg <name> version <req> expose <Interface>` binds `Interface` methods for qualified calls. Calls lower to Component-level imports of the resolved component; values cross the boundary under the RFC-0012 mapping restricted to the v0 value set: `Bool`, `I64`, `U64`, `Text`, `Bytes`, `List[T]`, `Option[T]`, `Result[T, E]`, and records/enums of the above. Everything else — resources, async, streams, variants with payload across the boundary, handles — is refused with `E-wit-unsupported-type` (RFC-0013's async/resource machinery stays internal, as today).
- The imported component receives **no ambient authority**: grants remain run-level, default-deny, exactly as `sico-app` authorize composes them today. An import can only narrow, never widen (architecture boundary rule).
- **User-interface exports are refused in v0** (`E-export-user-interface`): the Script `run` export remains the only export. `examples/component-plugin` stays design history; superseding it is future RFC work. (The M15 exit test needs imports only.)

### 2.5 Classification of the other parse-accepted surface (F5)

- `capability` blocks: stay accepted as pure type-naming surface (their only consumer), now *declared* in the matrix as such.
- `export function`: accepted today with no distinct semantic identity. v0 decision: keep accepted, declared as a no-op marker pending the exports RFC — or give it a typed refusal. **Decision point D1 for the owner** (recommendation: typed refusal; a decorative marker contradicts the no-silent-discard rule).
- `using` blocks: unchanged (function-body surface, out of scope here).

### 2.6 Typed refusal classes (stable identities, limit+1 each)

`E-module-declaration-missing`, `E-module-file-mismatch`, `E-module-cycle`, `E-unknown-module`, `E-unknown-item`, `E-unknown-package`, `E-package-missing`, `E-package-version`, `E-package-limit`, `E-wit-shape-mismatch`, `E-wit-unsupported-type`, `E-duplicate-name`, `E-use-depth-limit`, `E-export-user-interface`. Each lands in the machine-readable matrix and the refusal corpus.

## 3. Compatibility

- Single-module programs (everything that exists today) compile byte-identically: the frozen lowering snapshots are additive-only; any IR-op addition follows the append/patch snapshot discipline with justification; the frozen capability counts (20 lowered / 5 refused) are re-measured and re-frozen by the implementing STEP.
- The grammar version bumps for `module`/`use` reservation; old sources using them as identifiers get typed diagnostics with spans.

## 4. Exit corpus (binding when accepted)

1. Multi-module e2e: ≥3 modules with cross-calls, deterministic byte-exact output through `sico run` and `sico test`.
2. Package consumer: source imports a user WIT interface from a versioned, signed package via the lock; runs through a real Component Runtime.
3. Refusal corpus: one fixture per §2.6 identity, validator-checked.
4. limit+1: use-graph depth, module count, package count, interface size.
5. Matrix: bindings profile added to `tests/language-matrix/` + validator extension.
6. Tooling evidence: formatter idempotence on multi-module files, outline/LSP symbol coverage, AI-inspect on modules (bounded claims only).
7. Clean-room consumer: the STEP-0142 §5 frozen application (log analyzer) delivered at `clean-room-consumer` class — this doubles as M15 entry-condition-3 evidence per the owner decision of 2026-09-07.

## 5. Non-goals (v0)

- User-interface exports from Script programs; `component implements` syntax.
- Glob imports, re-exports, renaming, module-level visibility modifiers beyond import-or-not.
- Cyclic dependencies, source dependencies, build scripts, ambient downloads.
- WIT resources, async, streams or handles across user boundaries.
- Changes to the fixed `sico:script` world (fs/streams/http surfaces) — those evolve by their own versioned RFCs.

## 6. Reconsideration gates

Exports, richer imports, or visibility design change this RFC only through a new accepted version. Implementation STEPs may not reinterpret §2; gaps found during implementation return to the owner as RFC amendments.
