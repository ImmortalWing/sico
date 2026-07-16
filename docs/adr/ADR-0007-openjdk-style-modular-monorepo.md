# ADR-0007: OpenJDK-style modular monorepo and command ownership

> - status: accepted
> - date: 2026-07-16
> - owners: repository-owner
> - supersedes: repository-split proposal
> - archived baseline: `codex/archive-v0.0.1-development-history`

## Context

Sico v0.0.1 placed the language frontend, compiler backend, `.sapp` package model, Runtime, Desktop Host, Mobile Host core, ecosystem and tools in one Rust workspace. The crate boundaries were useful, but `sico-cli` crossed the product boundary: one executable compiled source, created and inspected application packages, selected trust policy and launched Wasmtime.

OpenJDK is the chosen organizational reference. Compiler, virtual machine, libraries, tools and operating-system adapters advance in one repository, while source modules and build rules enforce boundaries. Separate commands do not require separate source repositories.

## Decision

Sico remains one repository and one Rust workspace. Repository separation is rejected while language, Component ABI, `.sapp` and Runtime compatibility still advance together.

The workspace has eight ownership modules recorded in `tests/architecture/module-boundaries.json`: language, compiler, tooling, application package, Runtime, Host, ecosystem and integration. Normal dependency edges are machine checked; dev-dependencies may cross boundaries only to provide conformance fixtures.

Command ownership is separated:

- `sico` owns language-only `check`, `format`, `outline` and Component `build`;
- `sico-app` owns `.sapp` `pack`, `inspect` and `run`;
- `sico-desktop-host` owns installation, open, lifecycle and platform association.

`sico build` emits a raw WebAssembly Component. It does not sign, authorize or execute an application. `sico-app` consumes a prebuilt Component or `.sapp`; it has no normal dependency on compiler crates and does not compile source implicitly.

Platform-independent behavior stays in `sico-host-core`. Operating-system adapters remain inside the Host module and may not become language dependencies. Windows runtime evidence does not make the language module Windows-specific.

## Consequences

Compiler and Runtime changes can still be atomic and tested as one revision. Installed products can be packaged separately. Users now perform an explicit compile, package and run pipeline, which makes trust and execution boundaries visible.

The v0.0.1 command surface remains reproducible from the archive branch. Main is the modular development line and may require a later version before release.

## Validation

Run:

```powershell
pwsh -NoProfile -File tools/validate-module-boundaries.ps1
```

The validator loads Cargo metadata, assigns every workspace package exactly once, rejects forbidden normal dependencies and checks the language/application CLI split.

## References

- [OpenJDK JEP 201: Modular Source Code](https://openjdk.org/jeps/201)
- [OpenJDK JEP 296: Consolidate the JDK Forest into a Single Repository](https://openjdk.org/jeps/296)
- [`ADR-0002`](./ADR-0002-runtime-platform-baseline.md)
- [`ADR-0004`](./ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
