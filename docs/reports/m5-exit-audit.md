# M5 Desktop Host exit audit

> - status: complete
> - date: 2026-07-16
> - phase: M5

GO: M5 complete; M6 entry gate satisfied; next STEP-0054.

## Requirement audit

| Gate | Evidence | Result |
|---|---|---|
| immutable signed install/open | app+signer identity、exact package digest、atomic copy、reverify-on-open | proven |
| permission integrity | capability fingerprint、deny/once/persistent、corruption fail closed、uninstall cleanup | proven |
| lifecycle isolation | one child per identity、bounded open queue、timeout/cancel/crash recovery、exact cleanup | proven |
| bounded native UI | 7 typed nodes、escaping、accessibility order、node/depth/text/event ceilings、Windows Forms probe | proven on Windows |
| Windows shell flow | install/open/Wasmtime `42`/uninstall、HKCU explicit association、icon、zip smoke | runtime-verified |
| desktop platform parity | same extension/content/identity/open/trust contract and generated artifacts | Windows runtime; macOS/Linux contract only |
| representative app | signed `examples/desktop/hello-desktop.sico` plus validated typed UI companion | proven on Windows |
| security/property | 10,240 new identity/capability/UI/open inputs plus threat and integration corpora | proven |
| startup baseline | release signed reinstall/reverify/open/Wasmtime; 3 x 20; median mean 32.344 ms; no SLA | measured |
| regression | M0–M4 exit chain and full Rust workspace validation | proven by exit validator |

## Honest gaps

macOS and Linux have generated, corpus-checked association artifacts but no runner, native registration or Runtime execution evidence. UI WIT has no compiler-facing Sico binding yet; the representative UI remains a strict companion model. The Windows zip is not a production-signed installer. Production publisher identity, update/revocation and public registry remain M7.

## M6 authorization

M6 may begin at STEP-0054 because the portable identity, package, permission, lifecycle and typed UI contracts are frozen and Windows runtime evidence exists. M6 must obtain Android emulator/device evidence before claiming Android completion.

