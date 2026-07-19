# Typed cancellation bridge v0

> - status: verified Windows Runtime slice
> - date: 2026-07-19
> - scope: STEP-0098 only

## Result

Sico now has one typed terminal-winner path for completion, failure, timeout and cancellation. Windows console control, timer requests and canonical client/debugger requests feed the same `CancelToken`; the first Runtime-observed committed outcome is final. `ObservedRun` retains the winning cancellation source for later event emission.

Actual Windows process-group evidence covers a Component blocked inside stream stdin plus an idle persistent watch session. Both receive `CTRL_BREAK_EVENT`, exit 123 and are awaited by the fixture. Client-file evidence covers a busy one-shot and generation-bound watch run; stale generation identity does not cancel.

## Bounds and authority

- request document: canonical JSON, 4,096 bytes maximum, 4,097 refused;
- exact run/generation matching before token mutation;
- causes limited to `client` and `debugger` at the client protocol boundary;
- 5 ms bounded polling and one remembered request payload to prevent replay;
- no guest filesystem or signal capability is added;
- process kill remains external termination, never forged into exit 123.

M9's bounded worker model remains unchanged. Blocked OS worker threads are process-bounded fallback resources; cancellation does not wait for them, and one-shot process exit reclaims them. The M9 STEP-0088/0089 validators remain the evidence for blocked write and HTTP wait behavior.

```text
STEP_0098_OK signal=windows-console-real client=one-shot+watch busy-read-pump-write-http=123 races=8/8 terminal=1 double-cancel=idempotent request=4096/4097-refused external-kill=not-cancelled authority=unchanged m11-adr=unlocked next=STEP-0099+STEP-0103
```

Linux native signal evidence remains for STEP-0102; this report makes no cross-platform M10 GO claim.
