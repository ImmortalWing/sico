# Persistent runner and watch v0 evidence

> - date: 2026-07-19
> - step: STEP-0090
> - status: verified implementation evidence

## Isolation model

One `Runner` owns the Wasmtime Engine and an eight-entry SHA-256 Component cache. A prepared generation owns a compiled Component and Linker with frozen filesystem/network grants. Each execution creates a new Store, resource table, IO worker set, cancellation token state and fuel/memory/timeout budget. No guest instance, global, resource handle or post-timeout HTTP state is carried into the next run.

`sico watch` owns source compilation. It publishes a Component only after compilation succeeds; the runner keeps its last healthy generation when the source is invalid. File bytes must remain unchanged for 100 ms before acceptance. Initial and side artifacts use atomic `create_new`; replacement uses same-directory rename, while collision/race failures close the loop without following a pre-planted file or link.

## Measured evidence

`tools/validate-step-0090.ps1` recorded:

- one persistent runner PID for all accepted generations;
- monotonic generations `1,2,3` with exits `0,122,0`;
- invalid source produced no run, and two writes 20 ms apart produced one run;
- a typed domain failure in generation 2 did not prevent healthy generation 3;
- repeated release runs of 32 executions on one cached PreparedProgram had 5.8–6.1 ms medians (latest 5,934 us), below the 20 ms planning target;
- a guest trap between healthy prepared runs did not poison the Engine or prepared healthy generation;
- watch temporary Component cleanup completed.

## Bounds and limits

Source is limited to 8 MiB, watched Component files to 64 MiB, the compiled cache to eight entries, polling to 5–1000 ms and debounce to 100 ms. Watch v0 handles one source file and buffered empty stdin. Streaming stdin replay, directory dependency graphs, native filesystem event backends, session state and remote watch protocols are deferred.
