# STEP-0147: RFC-0039 §2.3/§2.4 — package resolution and user-WIT imports

> - status: complete
> - phase: M15 prerequisite track (RFC-0039 §2.3/§2.4; owner session directive "完成M15-17" 2026-09-10 accepted the drafted contract set: RFC-0040, ADR-0013, RFC-0041, M16 threat model F-1/F-2)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - artifacts: amendments A7/A7b/A8 in [`RFC-0039`](../rfc/RFC-0039-source-modules-package-resolution-user-wit-v0.md); matrix + validator; [`tests/packages_resolve.rs`](../../crates/sico-cli/tests/packages_resolve.rs), [`tests/packages_command.rs`](../../crates/sico-cli/tests/packages_command.rs), runner `tests/package_builder.rs`

## 1. What was done

The remaining RFC-0039 slices landed end to end:

1. **Parser**: `use pkg <name> version <n> [expose <interface>]` (E1015
   message extended) and `interface <name> version <n>:` (new E1016
   `SYNTAX_INVALID_INTERFACE_VERSION`); `DeclarationDetail` carries the
   package/interface-version payload through HIR.
2. **Semantics**: versioned interfaces register with shape + value-set
   gate (E8017 `WIT_UNSUPPORTED_TYPE`, amendment A7 flat subset — scalars,
   `List[Text]`, `Result[ok, Text]`); the version-less corpus form is a
   declared no-boundary registration (no longer a silent drop). D1's
   `export function` refusal (E8019, A7b) fires only for script-profile
   programs (sources with `main`); the M2-era boundary oracles stay
   accepted. Package functions bind qualified `<package>.<function>`
   calls at check time.
3. **IR**: `PackageImport` lowering; package calls lower to
   `Operation::Intrinsic` with the full import name
   `sico:user/<interface-kebab>@<major>.0.0.<function-kebab>`; signatures
   ride `Module.import_signatures` (serde-transparent); the verifier
   consults them after the fixed intrinsic table. `boundary_kebab`
   (shared sico-ir helper) maps source names to boundary kebab-case
   (injective; extern names require kebab).
4. **Codegen**: one import instance per user interface (types interned),
   canon-lowered through the shared transport memory; generic
   `emit_user_call` — params flatten directly, results always via a
   caller-allocated return area (the fs/stream/http convention). Two
   latent shape bugs found and fixed by the new corpus: the user-import
   arm of `build_script_core`'s local `import_count` and of the
   alloc-forwarding predicate omitted the user-import terms (run's core
   signature and the global were off by one) — both caught by wasmtime
   validation of the first end-to-end component.
5. **Package producer** (`sico-codegen-wasm::package_builder`): fully
   programmatic pure-component builder (core module with bump allocator +
   canonical lifts + instance export) plus `csv@1` (RFC-4180-subset
   `parse-line`) and `table_stats@1` (`aggregate(values: List[Text],
   op) -> Result[List[Text], Text]` — count/min/max/mean, decimal parse
   inside the package with `malformed-number@<index>`). Every component
   stamps the `sico:user-interface` custom section: identity + SHA-256 of
   the canonical interface specification text (A8's E8016 gate).
6. **CLI**: `sico-lock.json` (`sico:source-lock:v0`) resolution with the
   fail-closed gate — E8012 unknown package, E8013 missing/corrupt lock
   or artifact (digest pinned), E8014 version mismatch, E8015
   >16-package bound, E8016 stamp mismatch, E8017 value set, E8018
   module-name collision, E8020 unknown exposed interface, E8021 impure
   package; source-level gates fire before artifact reads. Package
   components join the run cache identity (artifact digest) and reach the
   runner as `--package` paths (content-addressed cache entries).
7. **Runner**: `PackageBinary` preparation (pure-component enforcement,
   exported-identity + arity freeze), per-Store package instantiation
   (limiter widens by exactly the prepared package count — set at Store
   construction because wasmtime caches the memories() limit), and the
   dynamic `Val` bridge (`func_new` two-level export lookup: instance
   identity, then function; arity re-checked at link time).

## 2. Validation

- `packages_resolve` (3): signed-package consumer through the lock runs
  byte-exact via the real Component Runtime — csv parse ("alpha|be,ta|
  gam\"ma"), two-package composition ("count=3 mean=4000"), and the
  tampered-artifact digest refusal (E8013).
- `packages_command` (1 test, 8 identities): every E80xx refusal above.
- `package_builder` runner tests (4): content contracts (quoted CSV,
  malformed/overflow/empty-column/unknown-op), export structure,
  determinism.
- Full workspace + runner suites green (see STATUS); matrix +
  `validate-step-0131.ps1` extended and green; `fmt` clean; clippy on
  touched crates clean.

## 3. Declared limits (honesty)

- The E8016 gate is the producer stamp, not a structural component-type
  walker (A8; runner re-checks arities). The stamp's trust chain —
  owner-curated lock → artifact digest → producer stamp → declared
  interface — is explicit.
- `sico app authorize`/composed-command distribution of package-bearing
  programs is out of v0 (dev-run path only); capability mapping for
  `sico:user/*` is vacuous (packages are pure).
- `sico:script/http@0.2.0` is still missing from
  `capabilities_for_imports` (pre-existing, untouched).
