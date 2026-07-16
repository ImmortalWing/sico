# STEP-0072: user manual and documentation information architecture

> - status: complete
> - phase: release documentation support
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Turn the root README into an executable user entry, preserve its former development/status content as a dedicated development handbook, and split detailed user guidance into navigable topic manuals backed by the real CLI contract.

## 2. Scope

Included: installation, quick start, language/codegen boundary, all six CLI commands, source/package run, cache, capabilities, development signing/trust, generic LSP integration, AI tooling, troubleshooting, limitations, development handbook migration, indexes, local-link validation and machine-readable documentation contract.

Excluded: binary distribution, editor extensions, production signing/service setup, source debugging, unimplemented platform instructions and changing language/runtime behavior.

## 3. Changes

1. Replaced the root README with a user manual and shortest verified `42` workflow.
2. Moved contributor architecture, repository layout, quality commands and evidence rules into `docs/development/README.md`.
3. Added a topic-indexed `docs/user-guide/` with ten detailed manuals plus its README.
4. Derived command syntax and trust behavior from `sico --help`, RFC-0019, RFC-0027, the AI tooling protocol and CLI integration tests.
5. Kept `examples/` labeled as design/regression history and used the end-to-end scalar source for the user workflow.
6. Added `sico.user-manual-contract.v0` and a drift validator.

## 4. Validation

The documented release flow was executed with the release binary:

```text
sico 0.0.1
check=ok
format-check=ok
outline=main
source-run=42
package-build=ok
inspect=unsigned-development
unsigned-package-run=42
signed-inspect=development-trusted
signed-package-run=42
```

The validator checks eleven indexed manual files, six real CLI commands, version alignment, security/evidence language, local links and PowerShell syntax.

Expected result:

`STEP_0072_OK version=0.0.1 manuals=11 commands=6 quick_start=42 unsigned_dev=verified signed_dev=verified lsp=bounded ai=compiler-backed production=external-gated platforms=honest`

## 5. Links

- [user entry](../../README.md)
- [user manual index](../user-guide/README.md)
- [development handbook](../development/README.md)
- [machine contract](../../tests/tooling/user-manual-contract.json)
