# STEP-0071: Linux Desktop Host development handbook

> - status: complete
> - phase: M5/M7 support documentation
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Preserve a detailed, executable Linux Desktop Host development handbook without converting generated association contracts or Windows runtime results into Linux support claims.

## 2. Context and evidence

The repository has a portable Rust Desktop Host core and generated Linux desktop/shared-MIME/mimeapps declarations. It has no Linux runner, native build, association installation, GTK permission/UI adapter, desktop-session tests or Runtime evidence. Non-Windows permission and UI functions explicitly return unavailable.

The audit also found two implementation blockers: the platform artifact command is coupled to a canonical Windows `.exe`, and the generated Linux `open %f` command omits three arguments currently required by the CLI. A real file-manager open cannot work until platform-specific generation and XDG configuration defaults are implemented.

Primary references reviewed on 2026-07-16: freedesktop XDG Base Directory, Desktop Entry, MIME Apps and Shared MIME specifications; GTK 4 accessibility and GDK Wayland documentation; XDG Desktop Portal APIs; Rust target support; and Wasmtime platform support.

## 3. Scope

Included: current-state audit, proposed support matrix, Linux project layout, native/cross build, ABI/glibc checks, XDG paths, file association, ingress/TOCTOU, GTK permission/UI, Wayland/X11, portals, single-instance IPC, process supervision, Runtime probes, packaging, CI, evidence, troubleshooting and GO gates.

Excluded: implementing the Linux adapter, selecting supported distributions, installing system packages, mutating the current machine's associations, creating production signing identity, shipping a package or claiming compile/runtime verification.

## 4. Decision

Linux remains `contract-verified-not-runtime-verified`. Proposed v0 starts with x86_64 GNU/glibc, an optional separately proven aarch64 target, GTK 4, Wayland-first plus X11 fallback, per-user XDG installation and an unsandboxed auditable bundle before a separate Flatpak profile. These are handbook recommendations, not an accepted Linux product ADR.

Implementation must first split the Linux artifact generator from Windows `.exe` validation, then add XDG defaults for store/trust/runtime so `open %f` is complete. GUI, IPC, Runtime and packaging follow only after those entrypoint invariants are tested.

## 5. Changes

- Added `docs/platforms/LINUX-DEVELOPMENT.md`.
- Added a machine-readable Linux documentation/current-state contract.
- Added a validator that checks the handbook against current code gaps.
- Updated platform, project, status, handoff and STEP indexes.

STEP-0071 was an out-of-order support documentation step created before the STEP-0063–0069 implementation sequence completed. Those steps kept their allocated meanings and are now closed locally; the handbook still does not upgrade Linux evidence.

## 6. Validation

```powershell
& .\tools\validate-step-0071.ps1
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:__COMPAT_LAYER = 'RunAsInvoker'
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
git diff --check
```

No Linux build, GTK session, file association or Runtime command is claimed as executed. Windows-hosted Rust regression only proves that the existing shared contracts remain intact.

Validation result: `STEP_0071_OK linux=contract-verified-not-runtime-verified links=27 blockers=artifact-generator,xdg-open-config current=STEP-0063 next=STEP-0072`. New-document local links, STEP-0070 regression, `git diff --check`, Rust formatting, Clippy with warnings denied and the complete offline locked workspace test suite passed.

## 7. Risks and follow-ups

- A Linux ADR must freeze distro/glibc/architecture/GTK/session/package support.
- Current generated association artifacts are contract fixtures, not an installable complete integration.
- Flatpak and portal behavior require an independent security and Runtime profile.
- GNOME/KDE, Wayland/X11, x86_64/aarch64 and old/new glibc evidence cannot be inferred from each other.

## 8. Audit links

- [Linux handbook](../platforms/LINUX-DEVELOPMENT.md)
- [platform documentation index](../platforms/README.md)
- [machine-readable contract](../../tests/platform/linux-documentation-contract.json)
- [desktop adapter source](../../crates/sico-desktop-host/src/platform.rs)
- [desktop host source](../../crates/sico-desktop-host/src/lib.rs)
- [STEP-0052](./STEP-0052-desktop-platform-adapters-parity.md)
