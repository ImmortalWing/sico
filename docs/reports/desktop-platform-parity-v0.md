# Desktop platform adapter parity v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M5

## Result

GO for STEP-0052. One machine-readable contract and six generated artifacts preserve `.sapp` extension, MIME/content identity, single-path delivery, no-shell behavior and shared-core trust ownership across Windows, macOS and Linux.

## Evidence labels

| Platform | Evidence | Current proof |
|---|---|---|
| Windows | runtime-verified | STEP-0051 signed install/open/Wasmtime/uninstall plus native probe |
| macOS | contract-verified | parsed string/artifact corpus for UTI and `CFBundleDocumentTypes`; no runner |
| Linux | contract-verified | parsed string/artifact corpus for desktop/shared-MIME/mimeapps; no runner |

No macOS or Linux target is installed in this workspace. This report therefore does not claim native build, registration or runtime execution on those platforms.

