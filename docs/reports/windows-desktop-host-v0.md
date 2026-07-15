# Windows Desktop Host v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M5

## Result

GO for STEP-0051. On Windows, a development-signed `.sapp` is installed by digest, reopened through the full trust/identity/capability path, executed by Wasmtime 46.0.1, and uninstalled with its persistent permission records. The representative test returns `42`.

## Native evidence

- Windows Forms loads successfully in a non-interactive STA probe;
- the permission prompt exposes deny, allow once and persistent allow choices;
- typed UI JSON is validated before a native preview can be created;
- the release zip contains the host executable, `.ico` and operator README;
- file association is HKCU-only and requires explicit `association-apply`.

## Security boundary

Automated validation inspects but does not mutate registry associations. The open command passes `%1` as one quoted argument, never through `cmd.exe` or guest-controlled PowerShell. This package remains a development smoke distribution; production publisher identity and signed installation belong to a later release phase.

