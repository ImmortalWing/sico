# Desktop Host lifecycle and supervision v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M5

## Result

GO for STEP-0049. The shared Host owns one direct OS child per immutable app identity, passes program/arguments without a constructed shell command, bounds duplicate-open events, classifies terminal outcomes, kills timed-out/cancelled process trees on Windows and cleans only registered Host-owned files.

## Evidence

- second launch for the same identity returns `AlreadyRunning`；
- open queue accepts exactly 256 entries and refuses overflow；
- timeout kills the guest and a crash does not prevent a healthy relaunch；
- success/crash/timeout/cancel are distinct terminal outcomes；
- cleanup refuses an outside file and preserves it while deleting the registered inside file；
- per-guest permission session is cleared before removal。

## Boundary

The process adapter cannot deliver an in-guest graceful close event yet, so close is bounded termination. UI/event transport in STEP-0050/0051 may add graceful protocol before the same deadline but cannot remove forced cleanup.
