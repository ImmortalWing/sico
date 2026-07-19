# M9 plan: Streaming, async and interactive scripting

> - status: in-progress; STEP-0085–0089 complete, STEP-0090 next
> - created: 2026-07-17
> - phase: M9
> - entry requirement: M8 Script Profile GO with stable Script WIT/manifest/runner identities
> - execution boundary: M9 may prototype during M8 only when it does not alter an unaccepted M8 contract; no M9 support claim before its own exit audit

## 1. Outcome

Extend the bounded M8 batch profile into a streaming and interactive automation environment. M9 adds backpressured stdin/stdout, source-level Task/Future/Stream execution, scoped HTTP through a Component provider, a persistent development runner, watch mode, a state-bounded REPL and an explicitly reviewed top-level script syntax.

The target flows are:

```powershell
Get-Content large.ndjson | sico run transform.sico > result.ndjson
sico run fetch.sico --allow-net api.example.com:443
sico watch task.sico -- input.json
sico repl
```

## 2. Non-goals

- unrestricted child process or shell execution;
- implicit network, environment or filesystem inheritance;
- production public registry deployment;
- mobile UI completion;
- transparent distributed execution;
- claiming arbitrary long-running service reliability from a local scripting runner.

Any future process capability must be supplied by a separately versioned plugin/Host policy and is not part of M9 core.

## 3. Entry gates

- M8 aggregate ABI and Script runner are stable enough to version rather than rewrite.
- M8 has measured buffered I/O limits and representative JSON/file pilots.
- Runtime faults are structured, not parsed from stderr text.
- Store teardown proves no state or capability leakage between runs.
- The accepted M8 audit identifies streaming/async as the next bottleneck rather than compiler correctness debt.

## 4. Proposed execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0085 | streaming Script WIT/resource/cancellation RFC | complete: RFC-0030 proposed; `sico:script/streams@0.1.0` WIT parses; ownership/EOF/backpressure/cancel matrix fixed |
| STEP-0086 | resource Canonical ABI and stream codegen | complete: streams intrinsics + runner resources; exact drop (stale traps 125); mixing rule; 1/16/256 MiB passthrough at 10.8 MiB peak RSS; `script-streaming-v0` |
| STEP-0087 | Task/Future/Stream source backend | complete: sequential executor (spawn eager/await identity/task group); E5101/E5102 typed including indirect escape; cancelled edge 123; `script-task-sequential-v0` |
| STEP-0088 | async runner and streaming stdio | complete: worker-thread cancellable IO; blocked read/pump/write cancel 120–134 ms incl. spawn+timer; bounded queues; 256 MiB exact; `script-async-runner-v0` |
| STEP-0089 | scoped HTTP Component provider | complete: exact ASCII host/IPv4 + port grants; real `sico run` loopback POST; no redirect/TLS; 8 KiB URL, 64 KiB headers, 8 MiB body; timeout/cancel; `scoped-http-provider-v0` |
| STEP-0090 | persistent development runner and watch mode | Engine/Linker/cache reuse, per-run new Store, file-change coalescing and crash isolation |
| STEP-0091 | bounded REPL session model | deterministic cell IDs, recompile/replay policy, state/resource limits and reset/export behavior |
| STEP-0092 | top-level script syntax decision | parser/formatter/HIR/LSP/AI evidence and desugaring to a normal entry; explicit accept/reject RFC |
| STEP-0093 | editor/debug/AI execution integration | run/watch/REPL protocol, cancellation, source maps, bounded logs and no shell interpolation |
| STEP-0094 | M9 security, performance and exit audit | large-stream throughput/RSS, HTTP denial tests, cancellation races, persistent-runner isolation and platform matrix |

STEP numbers are reserved by this plan but individual STEP records are created only when their work begins.

## 5. Streaming model

M9 replaces whole-buffer transport for large data with affine resources:

```text
InputStream.read(max-bytes) -> result<bytes, stream-error>
OutputStream.write(bytes) -> result<unit, stream-error>
OutputStream.flush() -> result<unit, stream-error>
close(resource)
```

Reads and writes must be bounded, preserve backpressure and expose EOF separately from error. A resource cannot be copied, used after close, borrowed beyond its scope or retained across Store teardown. Buffered M8 entry remains supported as a compatibility profile.

## 6. Async and cancellation model

- Every spawned Task belongs to a lexical task scope.
- A scope cannot return with live child tasks unless the contract explicitly joins or cancels them.
- Future completion, timeout, cancellation and Host failure race to one terminal result.
- Stream production has an explicit capacity and cannot silently become unbounded.
- Cancellation propagates from CLI/Host through runner to guest tasks and blocked host calls.
- HTTP and file operations use the same cancellation and resource accounting model.

## 7. HTTP provider

HTTP is a versioned Component/Host provider, not a compiler intrinsic. The permission decision names exact endpoints or an auditable pattern. M9 must define redirects, DNS, proxies, TLS errors, body/header limits, upload streaming, credentials and logging redaction before enabling the provider.

Default policy remains no network. `--allow-net api.example.com:443` does not authorize other hosts, ports, local addresses, redirects or listener sockets.

## 8. Persistent runner, watch and REPL

The persistent process may reuse Engine, Linker, adapter and compiled artifacts, but each execution receives a new Store, WASI context, capability set and resource table. A crash or trap in one run cannot poison later runs.

Watch mode coalesces filesystem events and reruns only after a successful new compilation. REPL cells are recorded as bounded source history and recompiled/replayed deterministically; M9 does not require a dynamic object VM. Reset discards the Store and cell state.

Top-level syntax is not presumed. STEP-0092 must compare explicit `main`, a labeled `script:` block and unrestricted top-level statements using parser recovery, formatter stability, LSP and AI-generation evidence.

## 9. Performance gates

M9 must measure at least 1 MiB, 16 MiB and 256 MiB passthrough; slow consumer backpressure; HTTP small/large bodies; cancellation latency; repeated watch runs; REPL growth; and peak RSS.

Initial goals, subject to an accepted M8 baseline, are:

- streaming memory remains bounded independently of total input size;
- persistent warm rerun median at most 20 ms for a minimal cached script;
- cancellation reaches a blocked guest/host operation within 100 ms on the measured desktop runner;
- no output or log queue grows without a configured bound;
- 256 MiB passthrough completes without whole-input/whole-output buffering.

These are planning targets, not current evidence.

## 10. M9 exit gate

M9 is GO only when streaming resources, structured async cancellation, scoped HTTP, persistent-runner isolation, watch and REPL limits are independently verified; top-level syntax is either accepted with evidence or explicitly rejected; and the complete M0–M8 regression remains green. Platform support is claimed only for runners with actual execution evidence.
