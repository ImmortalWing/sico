# Sico registry origin deployment

This directory documents the production-capable read origin introduced by STEP-0074. It serves an existing RFC-0024 registry transport tree; it does not sign, authorize, mutate or publish releases.

## Quick start

Build the operator bundle on the verified Windows toolchain:

```powershell
.\tools\package-registry-origin.ps1
```

Extract the resulting `sico-registry-origin-v<VERSION>-x86_64-pc-windows-gnu.zip` on the origin host, create or synchronize the `data/` registry tree, then run:

```powershell
.\sico-registry.exe check --root .\data
.\sico-registry.exe serve --root .\data --listen 127.0.0.1:8787
```

Probe it locally:

```powershell
Invoke-RestMethod http://127.0.0.1:8787/healthz
Invoke-RestMethod http://127.0.0.1:8787/readyz
```

Objects are available below `/v1/` using their exact relative transport path, for example:

```text
http://127.0.0.1:8787/v1/blobs/sha256/<digest>
http://127.0.0.1:8787/v1/records/releases/<digest>.json
```

## Production edge

Keep the origin on loopback or a private service network. Terminate HTTPS at an operator-controlled reverse proxy/load balancer and forward only `GET`/`HEAD` traffic. The edge must provide:

- a real domain and valid TLS certificate;
- request and bandwidth limits;
- access/error metrics and alerting;
- DDoS controls appropriate to the hosting provider;
- immutable cache behavior for `/v1/`;
- `Cache-Control: no-store` for health/readiness;
- backup/restore of the registry tree and append-only publication logs.

The origin refuses a non-loopback listener unless `--allow-non-loopback-http` is supplied. That flag only acknowledges the network topology; it does not enable TLS or make cleartext Internet exposure safe.

## Trust boundary

The serving process must not receive publisher private keys, OIDC tokens or registry authority signing material. Signed metadata and content-addressed blobs are prepared through an owner-approved offline/admin publication flow and synchronized into `data/` atomically.

Clients must continue to verify publisher policy, namespace history, release/channel/checkpoint signatures, digest, byte length and `.sapp` structure. HTTPS authenticates the service connection but is not package authorization.

## Current evidence limit

The repository validates the origin on Windows loopback. Public domain/TLS, production identity, key custody, remote monitoring, backup restore and external availability remain unmeasured until real infrastructure is selected and authorized.
