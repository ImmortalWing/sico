//! In-process Script runner for `sico:script/program@0.1.0` Program
//! Components (STEP-0081).
//!
//! One Store per call, the direct Program export invocation proven by
//! STEP-0076/0079, M8 input bounds enforced before execution, per-call
//! hostcall-fuel budget, fuel for non-termination, an epoch deadline for
//! wall-clock timeout, and a memory ceiling. Every Runtime outcome is a
//! typed value; nothing is recovered from engine error text.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use wasmtime::component::{Component, Linker, Resource, ResourceTable, ResourceType, Val};
use wasmtime::{Config, Engine, ResourceLimiter, Store, UpdateDeadline};

/// RFC-0029 Script v0 bounds enforced by the runner before execution.
pub const MAX_ARGUMENTS: usize = 1_024;
pub const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
pub const MAX_TOTAL_ARGUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_CHANNEL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ERROR_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_GUEST_EXIT: i64 = 119;

/// Script v0 input value.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScriptInput {
    pub arguments: Vec<String>,
    pub stdin: Vec<u8>,
}

/// Host-granted scoped filesystem roots (STEP-0083, RFC-0029 storage.read /
/// storage.write). Roots are canonicalized by the caller before the run;
/// guest paths are relative and resolved strictly inside a granted root.
/// Empty means the capability is not granted and every call fails closed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FsGrants {
    pub read_roots: Vec<PathBuf>,
    pub write_roots: Vec<PathBuf>,
}

/// Reserved STEP-0089 HTTP grants. The runner does not link the draft HTTP
/// interface until the scoped provider and its denial tests are complete.
/// Keeping this data type inert lets the in-progress contract compile without
/// accidentally granting ambient network access.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetGrants {
    pub endpoints: Vec<(String, u16)>,
}

/// Per-call fs bounds: path text and file payload stay inside the Script v0
/// channel budget so fs cannot smuggle past the 8 MiB boundary.
pub const MAX_FS_PATH_BYTES: usize = 4 * 1024;
pub const MAX_FS_FILE_BYTES: usize = MAX_CHANNEL_BYTES;

/// Script v0 output value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i64,
}

/// Runtime limits applied per call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerLimits {
    /// Deterministic non-termination bound (Wasm fuel units).
    pub fuel: u64,
    /// Wall-clock bound enforced through the epoch deadline.
    pub timeout: Duration,
    /// Guest linear-memory ceiling in bytes.
    pub memory_bytes: usize,
    /// Canonical-ABI transport budget (elements are charged per value).
    pub hostcall_fuel: usize,
}

impl Default for RunnerLimits {
    fn default() -> Self {
        Self {
            fuel: 10_000_000,
            timeout: Duration::from_secs(10),
            memory_bytes: 64 * 1024 * 1024,
            hostcall_fuel: 128 << 20,
        }
    }
}

/// Input bound violation detected before any guest execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputViolation {
    ArgumentCount(usize),
    ArgumentBytes(usize),
    TotalArgumentBytes(usize),
    ChannelBytes(usize),
}

/// Typed Runtime outcome; no class is recovered from engine error text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunOutcome {
    /// Guest returned `ScriptOutput` with a valid exit value.
    Output(ScriptOutput),
    /// Guest returned a typed `ScriptError`.
    Domain { code: String, message: String },
    /// Host-side cancellation through [`CancelToken`].
    Cancelled,
    /// Wall-clock deadline exceeded.
    Timeout,
    /// Deterministic fuel exhausted.
    FuelExhausted,
    /// Guest memory growth denied by the configured ceiling.
    MemoryLimit,
    /// Any other guest trap (including malformed results and out-of-range
    /// guest exit values, which are rejected rather than truncated).
    Trap(String),
    /// The component could not be instantiated with the runner's imports.
    Launch(String),
    /// The artifact is not a runnable Program Component.
    Incompatible(String),
}

impl RunOutcome {
    /// RFC-0029 `sico run` exit mapping.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Output(output) => u8::try_from(output.exit_code).unwrap_or(125),
            Self::Domain { .. } => 122,
            Self::Cancelled => 123,
            Self::Timeout => 124,
            Self::FuelExhausted | Self::MemoryLimit | Self::Trap(_) => 125,
            Self::Launch(_) => 126,
            Self::Incompatible(_) => 127,
        }
    }

    /// Stable machine-readable class name for JSON diagnostics.
    #[must_use]
    pub fn class(&self) -> &'static str {
        match self {
            Self::Output(_) => "output",
            Self::Domain { .. } => "domain-error",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::FuelExhausted => "resource-limit.fuel",
            Self::MemoryLimit => "resource-limit.memory",
            Self::Trap(_) => "trap",
            Self::Launch(_) => "launch-failure",
            Self::Incompatible(_) => "incompatible",
        }
    }
}

/// Shared cancellation handle; [`crate::run_program`] consults it at every
/// epoch tick.
#[derive(Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[derive(Debug)]
struct Marker(&'static str);

impl fmt::Display for Marker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for Marker {}

struct RunState {
    memory_ceiling: usize,
    denied: bool,
    table: ResourceTable,
    streams_live: u32,
    cancel: CancelToken,
    io: IoWorkers,
}

/// Bounded worker channels behind the stream host calls (STEP-0088): every
/// OS read/write happens on a dedicated worker thread, so a blocked call is
/// interruptible — the runner waits in a 2 ms cancel-check loop instead of
/// blocking inside the OS call. Queues are bounded (rendezvous depth 1).
struct IoWorkers {
    stdin_request: std::sync::mpsc::SyncSender<u64>,
    stdin_response: std::sync::mpsc::Receiver<Result<Vec<u8>, ()>>,
    output_request: std::sync::mpsc::SyncSender<OutputJob>,
    output_response: std::sync::mpsc::Receiver<Result<(), ()>>,
}

enum OutputJob {
    Write { stderr: bool, bytes: Vec<u8> },
    Flush { stderr: bool },
}

fn spawn_io_workers() -> IoWorkers {
    use std::io::{Read as _, Write as _};
    let (stdin_request, request_rx) = std::sync::mpsc::sync_channel::<u64>(1);
    let (response_tx, stdin_response) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        while let Ok(max) = request_rx.recv() {
            let mut buffer = vec![0_u8; usize::try_from(max).unwrap_or(0)];
            let result = std::io::stdin()
                .lock()
                .read(&mut buffer)
                .map(|count| {
                    buffer.truncate(count);
                    std::mem::take(&mut buffer)
                })
                .map_err(|_| ());
            if response_tx.send(result).is_err() {
                break;
            }
        }
    });
    let (output_request, job_rx) = std::sync::mpsc::sync_channel::<OutputJob>(1);
    let (done_tx, output_response) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        while let Ok(job) = job_rx.recv() {
            let result = match job {
                OutputJob::Write { stderr, bytes } => {
                    if stderr {
                        std::io::stderr().lock().write_all(&bytes)
                    } else {
                        std::io::stdout().lock().write_all(&bytes)
                    }
                }
                OutputJob::Flush { stderr } => {
                    if stderr {
                        std::io::stderr().lock().flush()
                    } else {
                        std::io::stdout().lock().flush()
                    }
                }
            }
            .map_err(|_| ());
            if done_tx.send(result).is_err() {
                break;
            }
        }
    });
    IoWorkers {
        stdin_request,
        stdin_response,
        output_request,
        output_response,
    }
}

/// Waits for one worker response, returning `Cancelled` as soon as the
/// run's token fires instead of blocking behind the OS call.
fn recv_cancellable<T>(
    receiver: &std::sync::mpsc::Receiver<Result<T, ()>>,
    cancel: &CancelToken,
) -> Result<T, HostStreamError> {
    loop {
        if cancel.cancelled.load(Ordering::Acquire) {
            return Err(HostStreamError::Cancelled);
        }
        match receiver.try_recv() {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(())) => return Err(HostStreamError::Io),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(HostStreamError::Io);
            }
        }
    }
}

/// Enqueues one bounded worker job without allowing a full depth-1 queue to
/// hide cancellation. `SyncSender::send` can itself block before the response
/// wait begins, so callers must use this polling form for every stream job.
fn send_cancellable<T>(
    sender: &std::sync::mpsc::SyncSender<T>,
    mut value: T,
    cancel: &CancelToken,
) -> Result<(), HostStreamError> {
    loop {
        if cancel.cancelled.load(Ordering::Acquire) {
            return Err(HostStreamError::Cancelled);
        }
        match sender.try_send(value) {
            Ok(()) => return Ok(()),
            Err(std::sync::mpsc::TrySendError::Full(returned)) => {
                value = returned;
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                return Err(HostStreamError::Io);
            }
        }
    }
}

/// RFC-0030 stream-error enum in WIT declaration order.
#[derive(Clone, Copy, Debug, wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(enum)]
#[repr(u8)]
enum HostStreamError {
    #[component(name = "io")]
    Io,
    #[component(name = "cancelled")]
    Cancelled,
    #[component(name = "closed")]
    Closed,
    #[component(name = "resource-limit")]
    ResourceLimit,
}

/// Host payload behind an `input-stream` handle (RFC-0030).
struct HostInputStream {
    terminal: bool,
}

/// Host payload behind an `output-stream` handle; `stderr` picks the channel.
struct HostOutputStream {
    stderr: bool,
    terminal: bool,
}

/// RFC-0030 per-run bound on live stream handles.
const MAX_LIVE_STREAMS: u32 = 64;
/// RFC-0030 per-call read clamp and write payload bound.
const MAX_STREAM_READ: u64 = 64 * 1024;
const MAX_STREAM_WRITE: usize = 1024 * 1024;
/// Streaming runs top their budgets up per host call: streamed bytes are
/// unbounded by design (RFC-0030), while compute between two calls stays
/// bounded (1M fuel / 64 MiB hostcall transport) and the epoch deadline
/// remains the ultimate non-termination bound.
const STREAM_FUEL_PER_CALL: u64 = 1_000_000;
const STREAM_HOSTCALL_FUEL_PER_CALL: usize = 64 << 20;

fn top_up_stream_budgets(store: &mut wasmtime::StoreContextMut<'_, RunState>) {
    let _ = store.set_fuel(STREAM_FUEL_PER_CALL);
    store.set_hostcall_fuel(STREAM_HOSTCALL_FUEL_PER_CALL);
}

impl ResourceLimiter for RunState {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        let allowed = desired <= self.memory_ceiling;
        if !allowed {
            self.denied = true;
        }
        Ok(allowed)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        self.denied = true;
        Ok(false)
    }

    fn instances(&self) -> usize {
        // Program core plus the STEP-0083 fs transport module.
        2
    }

    fn tables(&self) -> usize {
        0
    }

    fn memories(&self) -> usize {
        1
    }
}

/// Runner engine with component model, fuel and epoch interruption enabled.
pub struct Runner {
    engine: Engine,
}

impl Runner {
    /// Creates a runner; engine setup is shared, every call still receives a
    /// fresh Store, WASI context (none) and resource table.
    ///
    /// # Errors
    ///
    /// Returns an engine configuration error.
    pub fn new() -> Result<Self, wasmtime::Error> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);
        // Streaming pumps are recursive until the source-level loop lands
        // (STEP-0087): 4 MiB stack covers ~8k iterations at 64 KiB chunks
        // (512 MiB streamed) while staying a hard bound.
        config.max_wasm_stack(4 * 1024 * 1024);
        Ok(Self {
            engine: Engine::new(&config)?,
        })
    }

    /// True when the component imports `sico:script/streams@0.1.0` (RFC-0030
    /// mixing rule: the buffered stdin then stays empty). An unreadable
    /// component reports false and fails later at instantiation.
    #[must_use]
    pub fn component_imports_streams(&self, component: &[u8]) -> bool {
        let Ok(component) = Component::new(&self.engine, component) else {
            return false;
        };
        component
            .component_type()
            .imports(&self.engine)
            .any(|(name, _)| name == "sico:script/streams@0.1.0")
    }

    /// Validates input bounds and runs one Program Component to completion.
    ///
    /// # Errors
    ///
    /// Returns [`InputViolation`] without any guest execution.
    pub fn run_program(
        &self,
        component: &[u8],
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        fs: &FsGrants,
    ) -> Result<RunOutcome, InputViolation> {
        check_input(input)?;
        Ok(self.run_unchecked(component, input, limits, cancel, fs))
    }

    fn run_unchecked(
        &self,
        component: &[u8],
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        fs: &FsGrants,
    ) -> RunOutcome {
        let component = match Component::new(&self.engine, component) {
            Ok(component) => component,
            Err(error) => return RunOutcome::Incompatible(format!("{error}")),
        };
        let mut linker = Linker::new(&self.engine);
        if let Err(error) = link_fs(&mut linker, fs) {
            return RunOutcome::Launch(format!("fs host setup failed: {error}"));
        }
        if let Err(error) = link_streams(&mut linker) {
            return RunOutcome::Launch(format!("streams host setup failed: {error}"));
        }
        let mut store = Store::new(
            &self.engine,
            RunState {
                memory_ceiling: limits.memory_bytes,
                denied: false,
                table: ResourceTable::new(),
                streams_live: 0,
                cancel: cancel.clone(),
                io: spawn_io_workers(),
            },
        );
        store.limiter(|state| state);
        if store.set_fuel(limits.fuel).is_err() {
            return RunOutcome::Launch("fuel configuration failed".to_owned());
        }
        let timeout_ticks = ticks_for(limits.timeout);
        // Check the cancellation token on every watchdog tick. A deadline set
        // directly to `timeout_ticks` would postpone cancellation of busy guest
        // code until the full wall-clock timeout elapsed.
        store.set_epoch_deadline(1);
        let cancelled = cancel.cancelled.clone();
        let mut remaining_ticks = timeout_ticks.max(1);
        store.epoch_deadline_callback(move |_| {
            if cancelled.load(Ordering::Acquire) {
                Err(Marker("cancelled").into())
            } else if remaining_ticks <= 1 {
                Err(Marker("timeout").into())
            } else {
                remaining_ticks -= 1;
                Ok(UpdateDeadline::Continue(1))
            }
        });
        // Watchdog: advance the engine epoch until the call finishes.
        let done = Arc::new(AtomicBool::new(false));
        let engine = self.engine.clone();
        let watchdog_done = done.clone();
        let watchdog = std::thread::spawn(move || {
            for _ in 0..timeout_ticks.max(1).saturating_add(1) {
                std::thread::sleep(TICK);
                engine.increment_epoch();
                if watchdog_done.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        let outcome = match linker.instantiate(&mut store, &component) {
            Ok(instance) => match instance.get_func(&mut store, "run") {
                Some(run) => self.invoke(&mut store, &run, input, limits),
                None => RunOutcome::Incompatible("component does not export run".to_owned()),
            },
            Err(error) => RunOutcome::Launch(format!("{error}")),
        };
        done.store(true, Ordering::Relaxed);
        let _ = watchdog.join();
        outcome
    }

    fn invoke(
        &self,
        store: &mut Store<RunState>,
        run: &wasmtime::component::Func,
        input: &ScriptInput,
        limits: &RunnerLimits,
    ) -> RunOutcome {
        // Canonical-ABI transport is charged per element; budget the call
        // from the measured input size on top of the configured floor.
        let needed = input
            .stdin
            .len()
            .saturating_mul(64)
            .saturating_add(64 << 20)
            .max(limits.hostcall_fuel);
        store.set_hostcall_fuel(needed);
        let params = [input_val(input)];
        let mut results = [Val::Result(Ok(None))];
        match run.call(&mut *store, &params, &mut results) {
            Ok(()) => read_result(&results[0]),
            Err(error) => classify_error(store, &error),
        }
    }
}

const TICK: Duration = Duration::from_millis(5);

/// Wires the scoped `sico:script/fs-read/fs-write@0.1.0` host functions.
/// Both instances always exist so a component built with fs imports links;
/// every call re-checks the grants and fails closed with a typed message.
/// The outer `wasmtime::Result` is the host trap channel; the inner
/// `Result<_, String>` is the WIT `result<_, string>` the guest matches on.
type FsReadOutcome = Result<(Result<Vec<u8>, String>,), wasmtime::Error>;
type FsExistsOutcome = Result<(Result<bool, String>,), wasmtime::Error>;
type FsWriteOutcome = Result<(Result<(), String>,), wasmtime::Error>;

fn link_fs(linker: &mut Linker<RunState>, fs: &FsGrants) -> Result<(), wasmtime::Error> {
    let read_roots = fs.read_roots.clone();
    let write_roots = fs.write_roots.clone();
    let mut fs_read = linker.instance("sico:script/fs-read@0.1.0")?;
    let roots = read_roots.clone();
    fs_read.func_wrap("read", move |_store, (path,): (String,)| -> FsReadOutcome {
        let outcome = (|| {
            let resolved = resolve_read(&roots, &path)?;
            let metadata = std::fs::metadata(&resolved).map_err(|error| format!("io: {error}"))?;
            if metadata.len() > MAX_FS_FILE_BYTES as u64 {
                return Err("resource-limit: file exceeds 8 MiB".to_owned());
            }
            std::fs::read(&resolved).map_err(|error| format!("io: {error}"))
        })();
        Ok((outcome,))
    })?;
    fs_read.func_wrap(
        "exists",
        move |_store, (path,): (String,)| -> FsExistsOutcome {
            Ok((resolve_read(&read_roots, &path).map(|resolved| resolved.is_file()),))
        },
    )?;
    let mut fs_write = linker.instance("sico:script/fs-write@0.1.0")?;
    fs_write.func_wrap(
        "write",
        move |_store, (path, content): (String, Vec<u8>)| -> FsWriteOutcome {
            let outcome = (|| {
                if content.len() > MAX_FS_FILE_BYTES {
                    return Err("resource-limit: content exceeds 8 MiB".to_owned());
                }
                let resolved = resolve_write(&write_roots, &path)?;
                std::fs::write(&resolved, &content).map_err(|error| format!("io: {error}"))
            })();
            Ok((outcome,))
        },
    )?;
    Ok(())
}

/// Wires the `sico:script/streams@0.1.0` streaming channel (RFC-0030). The
/// instance always exists; handles live in the per-Store resource table and
/// die with it. v0 is synchronous: cancellation is observed between calls
/// through the epoch deadline, interruption of a blocked host read is the
/// STEP-0088 async-runner item.
fn link_streams(linker: &mut Linker<RunState>) -> Result<(), wasmtime::Error> {
    let mut streams = linker.instance("sico:script/streams@0.1.0")?;
    streams.resource(
        "input-stream",
        ResourceType::host::<HostInputStream>(),
        |mut cx, rep| {
            let state = cx.data_mut();
            state
                .table
                .delete(Resource::<HostInputStream>::new_own(rep))?;
            state.streams_live = state.streams_live.saturating_sub(1);
            Ok(())
        },
    )?;
    streams.resource(
        "output-stream",
        ResourceType::host::<HostOutputStream>(),
        |mut cx, rep| {
            let state = cx.data_mut();
            let stream = state
                .table
                .delete(Resource::<HostOutputStream>::new_own(rep))?;
            // Drop is close. RFC-0030 promises only a best-effort flush here;
            // never block Store teardown behind a full or OS-blocked queue.
            let _ = state.io.output_request.try_send(OutputJob::Flush {
                stderr: stream.stderr,
            });
            state.streams_live = state.streams_live.saturating_sub(1);
            Ok(())
        },
    )?;
    streams.func_wrap(
        "stdin",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         _: ()|
         -> Result<(Resource<HostInputStream>,), wasmtime::Error> {
            if store.data().streams_live >= MAX_LIVE_STREAMS {
                return Err(wasmtime::Error::msg(
                    "resource-limit: too many live stream handles",
                ));
            }
            let resource = store
                .data_mut()
                .table
                .push(HostInputStream { terminal: false })?;
            store.data_mut().streams_live += 1;
            Ok((resource,))
        },
    )?;
    let stdout_ctor = |stderr: bool| {
        move |mut store: wasmtime::StoreContextMut<'_, RunState>,
              _: ()|
              -> Result<(Resource<HostOutputStream>,), wasmtime::Error> {
            if store.data().streams_live >= MAX_LIVE_STREAMS {
                return Err(wasmtime::Error::msg(
                    "resource-limit: too many live stream handles",
                ));
            }
            let resource = store.data_mut().table.push(HostOutputStream {
                stderr,
                terminal: false,
            })?;
            store.data_mut().streams_live += 1;
            Ok((resource,))
        }
    };
    streams.func_wrap("stdout", stdout_ctor(false))?;
    streams.func_wrap("stderr", stdout_ctor(true))?;
    streams.func_wrap(
        "[method]input-stream.read",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         (handle, max): (Resource<HostInputStream>, u64)|
         -> Result<(Result<Vec<u8>, HostStreamError>,), wasmtime::Error> {
            if store
                .data()
                .table
                .get(&handle)
                .map_or(true, |stream| stream.terminal)
            {
                return Ok((Err(HostStreamError::Closed),));
            }
            top_up_stream_budgets(&mut store);
            if max == 0 {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
                return Ok((Err(HostStreamError::ResourceLimit),));
            }
            let max = max.min(MAX_STREAM_READ);
            let cancel = store.data().cancel.clone();
            let outcome = send_cancellable(&store.data().io.stdin_request, max, &cancel)
                .and_then(|()| recv_cancellable(&store.data().io.stdin_response, &cancel));
            if outcome.is_err() {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
            }
            Ok((outcome,))
        },
    )?;
    streams.func_wrap(
        "[method]output-stream.write",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         (handle, bytes): (Resource<HostOutputStream>, Vec<u8>)|
         -> Result<(Result<(), HostStreamError>,), wasmtime::Error> {
            let stderr_channel = match store.data().table.get(&handle) {
                Ok(stream) if !stream.terminal => stream.stderr,
                Ok(_) => return Ok((Err(HostStreamError::Closed),)),
                Err(_) => return Ok((Err(HostStreamError::Closed),)),
            };
            if bytes.len() > MAX_STREAM_WRITE {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
                return Ok((Err(HostStreamError::ResourceLimit),));
            }
            top_up_stream_budgets(&mut store);
            let cancel = store.data().cancel.clone();
            let outcome = send_cancellable(
                &store.data().io.output_request,
                OutputJob::Write {
                    stderr: stderr_channel,
                    bytes,
                },
                &cancel,
            )
            .and_then(|()| recv_cancellable(&store.data().io.output_response, &cancel));
            if outcome.is_err() {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
            }
            Ok((outcome,))
        },
    )?;
    streams.func_wrap(
        "[method]output-stream.flush",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         (handle,): (Resource<HostOutputStream>,)|
         -> Result<(Result<(), HostStreamError>,), wasmtime::Error> {
            let stderr_channel = match store.data().table.get(&handle) {
                Ok(stream) if !stream.terminal => stream.stderr,
                Ok(_) => return Ok((Err(HostStreamError::Closed),)),
                Err(_) => return Ok((Err(HostStreamError::Closed),)),
            };
            top_up_stream_budgets(&mut store);
            let cancel = store.data().cancel.clone();
            let outcome = send_cancellable(
                &store.data().io.output_request,
                OutputJob::Flush {
                    stderr: stderr_channel,
                },
                &cancel,
            )
            .and_then(|()| recv_cancellable(&store.data().io.output_response, &cancel));
            if outcome.is_err() {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
            }
            Ok((outcome,))
        },
    )?;
    streams.func_wrap(
        "pump",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         (input, output): (Resource<HostInputStream>, Resource<HostOutputStream>)|
         -> Result<(Result<u64, HostStreamError>,), wasmtime::Error> {
            if store
                .data()
                .table
                .get(&input)
                .map_or(true, |stream| stream.terminal)
                || store
                    .data()
                    .table
                    .get(&output)
                    .map_or(true, |stream| stream.terminal)
            {
                return Ok((Err(HostStreamError::Closed),));
            }
            let stderr_channel = store.data().table.get(&output).unwrap().stderr;
            top_up_stream_budgets(&mut store);
            let mut total = 0_u64;
            let outcome = loop {
                let cancel = store.data().cancel.clone();
                let chunk = match send_cancellable(
                    &store.data().io.stdin_request,
                    MAX_STREAM_READ,
                    &cancel,
                )
                .and_then(|()| recv_cancellable(&store.data().io.stdin_response, &cancel))
                {
                    Ok(chunk) => chunk,
                    Err(error) => break Err(error),
                };
                if chunk.is_empty() {
                    break Ok(total);
                }
                let moved = chunk.len() as u64;
                if let Err(error) = send_cancellable(
                    &store.data().io.output_request,
                    OutputJob::Write {
                        stderr: stderr_channel,
                        bytes: chunk,
                    },
                    &cancel,
                ) {
                    break Err(error);
                }
                if let Err(error) = recv_cancellable(&store.data().io.output_response, &cancel) {
                    break Err(error);
                }
                total = total.saturating_add(moved);
                // Keep each call's transport bounded even for huge streams.
                let _ = store.set_fuel(STREAM_FUEL_PER_CALL);
                store.set_hostcall_fuel(STREAM_HOSTCALL_FUEL_PER_CALL);
            };
            if outcome.is_err() {
                store.data_mut().table.get_mut(&input)?.terminal = true;
                store.data_mut().table.get_mut(&output)?.terminal = true;
            }
            Ok((outcome,))
        },
    )?;
    Ok(())
}
/// drive/verbatim prefixes, no backslash or colon inside components.
fn scope_path(capability: &str, path: &str) -> Result<PathBuf, String> {
    if path.is_empty() || path.len() > MAX_FS_PATH_BYTES {
        return Err(format!("{capability}: path is empty or exceeds 4 KiB"));
    }
    let mut clean = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            PathComponent::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| format!("{capability}: path is not UTF-8"))?;
                if part.contains([':', '\\']) {
                    return Err(format!("{capability}: path escapes the granted scope"));
                }
                clean.push(part);
            }
            PathComponent::CurDir => {}
            _ => return Err(format!("{capability}: path escapes the granted scope")),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(format!("{capability}: empty path"));
    }
    Ok(clean)
}

/// Resolves a read-side path: the first granted root containing the file;
/// symlink escapes are denied through canonical containment checks.
fn resolve_read(roots: &[PathBuf], path: &str) -> Result<PathBuf, String> {
    if roots.is_empty() {
        return Err("storage.read not granted".to_owned());
    }
    let clean = scope_path("storage.read", path)?;
    for root in roots {
        let candidate = root.join(&clean);
        if let Ok(canonical) = candidate.canonicalize() {
            return if canonical.starts_with(root) {
                Ok(canonical)
            } else {
                Err("storage.read: path escapes the granted scope".to_owned())
            };
        }
    }
    Err("io: not found below any granted read root".to_owned())
}

/// Resolves a write-side path below the first granted write root; the parent
/// directory must already exist inside the root (no symlink escape).
fn resolve_write(roots: &[PathBuf], path: &str) -> Result<PathBuf, String> {
    let Some(root) = roots.first() else {
        return Err("storage.write not granted".to_owned());
    };
    let clean = scope_path("storage.write", path)?;
    let candidate = root.join(&clean);
    let parent = candidate
        .parent()
        .ok_or_else(|| "storage.write: path escapes the granted scope".to_owned())?;
    let canonical = parent
        .canonicalize()
        .map_err(|error| format!("io: {error}"))?;
    if !canonical.starts_with(root) {
        return Err("storage.write: path escapes the granted scope".to_owned());
    }
    Ok(candidate)
}

fn ticks_for(timeout: Duration) -> u64 {
    u64::try_from(timeout.as_millis() / TICK.as_millis().max(1)).unwrap_or(u64::MAX)
}

fn check_input(input: &ScriptInput) -> Result<(), InputViolation> {
    if input.arguments.len() > MAX_ARGUMENTS {
        return Err(InputViolation::ArgumentCount(input.arguments.len()));
    }
    let mut total = 0_usize;
    for argument in &input.arguments {
        if argument.len() > MAX_ARGUMENT_BYTES {
            return Err(InputViolation::ArgumentBytes(argument.len()));
        }
        total = total.saturating_add(argument.len());
    }
    if total > MAX_TOTAL_ARGUMENT_BYTES {
        return Err(InputViolation::TotalArgumentBytes(total));
    }
    if input.stdin.len() > MAX_CHANNEL_BYTES {
        return Err(InputViolation::ChannelBytes(input.stdin.len()));
    }
    Ok(())
}

fn classify_error(store: &Store<RunState>, error: &wasmtime::Error) -> RunOutcome {
    if error
        .downcast_ref::<Marker>()
        .is_some_and(|m| m.0 == "timeout")
    {
        return RunOutcome::Timeout;
    }
    if error
        .downcast_ref::<Marker>()
        .is_some_and(|m| m.0 == "cancelled")
    {
        return RunOutcome::Cancelled;
    }
    if store.data().denied {
        return RunOutcome::MemoryLimit;
    }
    if matches!(store.get_fuel(), Ok(0)) {
        return RunOutcome::FuelExhausted;
    }
    RunOutcome::Trap(format!("{error}"))
}

fn input_val(input: &ScriptInput) -> Val {
    Val::Record(vec![
        (
            "arguments".to_owned(),
            Val::List(input.arguments.iter().cloned().map(Val::String).collect()),
        ),
        (
            "stdin".to_owned(),
            Val::List(input.stdin.iter().copied().map(Val::U8).collect()),
        ),
    ])
}

fn field<'a>(fields: &'a [(String, Val)], name: &str) -> Result<&'a Val, RunOutcome> {
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .ok_or_else(|| RunOutcome::Trap(format!("malformed result: missing field {name}")))
}

fn bytes_field(fields: &[(String, Val)], name: &str) -> Result<Vec<u8>, RunOutcome> {
    let Val::List(values) = field(fields, name)? else {
        return Err(RunOutcome::Trap(format!(
            "malformed result: {name} is not a list"
        )));
    };
    if values.len() > MAX_CHANNEL_BYTES {
        return Err(RunOutcome::Trap(format!(
            "malformed result: {name} exceeds 8 MiB"
        )));
    }
    values
        .iter()
        .map(|value| match value {
            Val::U8(byte) => Ok(*byte),
            _ => Err(RunOutcome::Trap(format!(
                "malformed result: {name} item is not u8"
            ))),
        })
        .collect()
}

fn read_result(value: &Val) -> RunOutcome {
    match read_result_inner(value) {
        Ok(outcome) => outcome,
        Err(outcome) => outcome,
    }
}

fn read_result_inner(value: &Val) -> Result<RunOutcome, RunOutcome> {
    match value {
        Val::Result(Ok(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err(RunOutcome::Trap(
                    "malformed result: output is not a record".to_owned(),
                ));
            };
            let exit_code = match field(fields, "exit-code")? {
                Val::S64(exit) => *exit,
                _ => {
                    return Err(RunOutcome::Trap(
                        "malformed result: exit-code is not s64".to_owned(),
                    ));
                }
            };
            if !(0..=MAX_GUEST_EXIT).contains(&exit_code) {
                return Err(RunOutcome::Trap(format!(
                    "guest exit value {exit_code} is outside 0..={MAX_GUEST_EXIT}"
                )));
            }
            Ok(RunOutcome::Output(ScriptOutput {
                stdout: bytes_field(fields, "stdout")?,
                stderr: bytes_field(fields, "stderr")?,
                exit_code,
            }))
        }
        Val::Result(Err(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err(RunOutcome::Trap(
                    "malformed result: error is not a record".to_owned(),
                ));
            };
            let code = match field(fields, "code")? {
                Val::Enum(code) => code.clone(),
                _ => {
                    return Err(RunOutcome::Trap(
                        "malformed result: code is not an enum".to_owned(),
                    ));
                }
            };
            let message = match field(fields, "message")? {
                Val::String(message) => message.clone(),
                _ => {
                    return Err(RunOutcome::Trap(
                        "malformed result: message is not text".to_owned(),
                    ));
                }
            };
            if message.len() > MAX_ERROR_MESSAGE_BYTES {
                return Err(RunOutcome::Trap(
                    "malformed result: error message exceeds 64 KiB".to_owned(),
                ));
            }
            if code == "cancelled" {
                Ok(RunOutcome::Cancelled)
            } else {
                Ok(RunOutcome::Domain { code, message })
            }
        }
        _ => Err(RunOutcome::Trap(
            "malformed result: invalid result shape".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_worker_queue_observes_later_cancellation() {
        let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
        sender.send(1_u8).unwrap();
        let cancel = CancelToken::new();
        let cancel_thread = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            cancel_thread.cancel();
        });
        let started = std::time::Instant::now();
        assert!(matches!(
            send_cancellable(&sender, 2_u8, &cancel),
            Err(HostStreamError::Cancelled)
        ));
        assert!(started.elapsed() < Duration::from_millis(200));
    }
}
