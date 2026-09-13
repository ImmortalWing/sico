//! In-process Script runner for `sico:script/program@0.1.0` Program
//! Components (STEP-0081).
//!
//! One Store per call, the direct Program export invocation proven by
//! STEP-0076/0079, M8 input bounds enforced before execution, per-call
//! hostcall-fuel budget, fuel for non-termination, an epoch deadline for
//! wall-clock timeout, and a memory ceiling. Every Runtime outcome is a
//! typed value; nothing is recovered from engine error text.

#![forbid(unsafe_code)]

mod dap;
pub mod http2;
pub mod scheduler;

pub use dap::{RuntimeDapBackend, RuntimeDapConfig};
use scheduler::{ReadinessClass, RunIdentity, SchedulerCore, TerminalKind};

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, TcpStream, ToSocketAddrs};
use std::num::NonZeroUsize;
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use sico_observability::{
    ContractError, DebugMap, EXECUTION_EVENT_SCHEMA, EngineFrame, EventQueue, EventRedactor,
    ExecutionEvent, ExecutionEventPayload, MAX_CAPTURED_CHANNEL_BYTES, RuntimeFault, RuntimeFrame,
    parse_cancel_request, resolve_engine_frame, validate_runtime_fault, verify_debug_artifacts,
};
use wasmparser::{Parser, Payload};
use wasmtime::component::types::ComponentItem;
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

/// Exact per-invocation HTTP endpoint grants (STEP-0089 / RFC-0031).
/// Empty means no network. v0 accepts ASCII DNS names and canonical IPv4
/// literals only; every endpoint includes a non-zero port.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetGrants {
    pub endpoints: Vec<(String, u16)>,
    /// Scheme-aware grants backing `sico:script/http@0.2.0` (RFC-0037):
    /// `https://`, `http://` and `https+private://` spellings, canonicalized
    /// once through the provider's strict authority parser.
    pub secure_endpoints: Vec<SecureEndpoint>,
}

/// One scheme-aware endpoint grant identity (RFC-0037 §2).
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecureEndpoint {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

impl NetGrants {
    /// Adds one exact `host:port` grant after strict v0 validation.
    ///
    /// # Errors
    ///
    /// Returns a bounded diagnostic without resolving or opening a socket.
    pub fn grant(&mut self, endpoint: &str) -> Result<(), String> {
        let parsed = parse_endpoint(endpoint)?;
        if !self.endpoints.contains(&parsed) {
            self.endpoints.push(parsed);
            self.endpoints.sort();
        }
        Ok(())
    }

    /// Adds one scheme-aware `scheme://host[:port]` grant (RFC-0037).
    /// Canonicalization happens exactly once; ambiguous or non-canonical
    /// authorities are refused before any grant exists.
    ///
    /// # Errors
    ///
    /// Returns a bounded diagnostic without resolving or opening a socket.
    pub fn grant_secure(&mut self, endpoint: &str) -> Result<(), String> {
        use sico_http_provider::authority::{Endpoint as ProviderEndpoint, Scheme};
        let canonical: ProviderEndpoint = sico_http_provider::authority::canonicalize_url(endpoint)
            .map_err(|error| format!("authority: {error}"))?;
        let scheme_text = match canonical.scheme {
            Scheme::Http => "http",
            Scheme::HttpPrivate => "http+private",
            Scheme::Https => "https",
            Scheme::HttpsPrivate => "https+private",
        };
        let secure = SecureEndpoint {
            scheme: scheme_text.to_owned(),
            host: canonical.host,
            port: canonical.port,
        };
        if !self.secure_endpoints.contains(&secure) {
            self.secure_endpoints.push(secure.clone());
            self.secure_endpoints.sort();
        }
        Ok(())
    }

    fn allows(&self, host: &str, port: u16) -> bool {
        self.endpoints
            .iter()
            .any(|(allowed_host, allowed_port)| allowed_host == host && *allowed_port == port)
    }

    /// Builds the exact provider endpoint set this Store may contact for
    /// `http@0.2.0`: the 0.1.0 `host:port` grants map to plain `http`
    /// endpoints (0.1.0 grants remain valid), plus every scheme-aware
    /// grant. Empty means default-deny.
    pub(crate) fn http2_endpoint_set(&self) -> Vec<sico_http_provider::authority::Endpoint> {
        use sico_http_provider::authority::{Endpoint as ProviderEndpoint, Scheme};
        let mut set: Vec<ProviderEndpoint> = self
            .endpoints
            .iter()
            .map(|(host, port)| ProviderEndpoint {
                scheme: Scheme::Http,
                host: host.clone(),
                port: *port,
            })
            .collect();
        for secure in &self.secure_endpoints {
            let Some(scheme) = Scheme::parse(&secure.scheme) else {
                continue;
            };
            set.push(ProviderEndpoint {
                scheme,
                host: secure.host.clone(),
                port: secure.port,
            });
        }
        set
    }
}

/// Host-side HTTP policy frozen into one prepared runner generation
/// (RFC-0037 §3/§6): the explicit trust-root set and the Host-owned secret
/// registry. Secret values never enter guest-visible memory.
#[derive(Clone, Default)]
pub struct HttpPolicy {
    /// PEM trust roots for the pinned-roots trust mode. Empty means every
    /// TLS handshake fails closed (no insecure fallback exists).
    pub trust_roots_pem: Vec<u8>,
    /// Host-owned opaque secrets; `None` means an empty registry.
    pub secrets: Option<std::sync::Arc<sico_http_provider::secrets::SecretStore>>,
}

impl HttpPolicy {
    /// Builds a policy trusting exactly `trust_roots_pem`.
    #[must_use]
    pub fn with_trust_roots(trust_roots_pem: Vec<u8>) -> Self {
        Self {
            trust_roots_pem,
            secrets: None,
        }
    }

    /// Attaches a Host-owned secret registry.
    #[must_use]
    pub fn with_secrets(
        mut self,
        secrets: std::sync::Arc<sico_http_provider::secrets::SecretStore>,
    ) -> Self {
        self.secrets = Some(secrets);
        self
    }
}

impl std::fmt::Debug for HttpPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpPolicy")
            .field("trust_roots_bytes", &self.trust_roots_pem.len())
            .field("secrets_registered", &self.secrets.as_ref().is_some())
            .finish()
    }
}

/// Per-call fs bounds: path text and file payload stay inside the Script v0
/// channel budget so fs cannot smuggle past the 8 MiB boundary.
pub const MAX_FS_PATH_BYTES: usize = 4 * 1024;
pub const MAX_FS_FILE_BYTES: usize = MAX_CHANNEL_BYTES;
pub const MAX_HTTP_URL_BYTES: usize = 8 * 1024;
pub const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;
pub const MAX_HTTP_BODY_BYTES: usize = MAX_CHANNEL_BYTES;
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const HTTP_TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

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
    /// Wasm call-stack budget exhausted (deep recursion trips the
    /// `max_wasm_stack` accounting on the big-stack guest worker, never the
    /// native thread stack).
    StackLimit,
    /// Guest memory growth denied by the configured ceiling.
    MemoryLimit,
    /// A named Host provider failed internally rather than returning its
    /// declared guest-visible domain result.
    HostProviderFailure { provider_id: String },
    /// Any other guest trap (including malformed results and out-of-range
    /// guest exit values, which are rejected rather than truncated).
    Trap(String),
    /// The component could not be instantiated with the runner's imports.
    Launch(String),
    /// The artifact is not a runnable Program Component.
    Incompatible(String),
}

/// One typed outcome plus its strict Runtime fault record when execution did
/// not return a successful `ScriptOutput`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedRun {
    pub outcome: RunOutcome,
    pub fault: Option<RuntimeFault>,
    pub cancellation_source: Option<CancellationSource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugStopFrame {
    pub core_module: Option<String>,
    pub module_pc: u32,
    pub locals: Vec<DebugLocalValue>,
}

/// One bounded, read-only scalar captured while the Store is stopped. Core
/// references are never formatted or dereferenced because they could expose
/// engine addresses or unbounded guest graphs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugLocalValue {
    pub name: String,
    pub type_name: &'static str,
    pub value: String,
    pub available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugStop {
    pub reason: &'static str,
    pub frames: Vec<DebugStopFrame>,
}

#[derive(Debug, Default)]
struct DebugControlState {
    pause_requested: bool,
    stopped: Option<DebugStop>,
    resume: bool,
}

/// Thread-safe control plane used by the DAP adapter. A pause request advances
/// the Wasmtime epoch; the guest-debug handler freezes the Store and waits for
/// an explicit continue without exposing mutation or capability APIs.
#[derive(Clone, Debug)]
pub struct RuntimeDebugControl {
    engine: Engine,
    state: Arc<(Mutex<DebugControlState>, Condvar)>,
}

impl RuntimeDebugControl {
    fn new(engine: Engine) -> Self {
        Self {
            engine,
            state: Arc::new((Mutex::new(DebugControlState::default()), Condvar::new())),
        }
    }

    pub fn request_pause(&self) {
        self.state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pause_requested = true;
        self.engine.increment_epoch();
    }

    #[must_use]
    pub fn wait_for_stop(&self, timeout: Duration) -> Option<DebugStop> {
        let (lock, changed) = &*self.state;
        let state = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let state = changed
            .wait_timeout_while(state, timeout, |state| state.stopped.is_none())
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0;
        state.stopped.clone()
    }

    pub fn continue_execution(&self) -> bool {
        let (lock, changed) = &*self.state;
        let mut state = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.stopped.is_none() {
            return false;
        }
        state.resume = true;
        changed.notify_all();
        true
    }

    fn handler(&self) -> RuntimeDebugHandler {
        RuntimeDebugHandler {
            state: self.state.clone(),
        }
    }

    fn release_stop(&self) {
        let (lock, changed) = &*self.state;
        let mut state = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.resume = true;
        changed.notify_all();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DebugStartError {
    Input(InputViolation),
    Runtime(RunOutcome),
    DebugDisabled,
    MissingDebugMap,
    UnboundBreakpoint(DebugBreakpoint),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DebugSessionError {
    Timeout,
    Disconnected,
    Contract(ContractError),
}

/// One owned debug execution. Drop, terminate and disconnect all converge on
/// the same typed cancellation token and release a stopped Store before join.
pub struct RuntimeDebugSession {
    control: RuntimeDebugControl,
    cancel: CancelToken,
    result: std::sync::mpsc::Receiver<Execution>,
    worker: Option<std::thread::JoinHandle<()>>,
    debug_map: Arc<DebugMap>,
    run_id: String,
    generation_id: u64,
}

impl RuntimeDebugSession {
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.worker
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }

    pub fn request_pause(&self) {
        self.control.request_pause();
    }

    #[must_use]
    pub fn wait_for_stop(&self, timeout: Duration) -> Option<DebugStop> {
        self.control.wait_for_stop(timeout)
    }

    pub fn continue_execution(&self) -> bool {
        self.control.continue_execution()
    }

    pub fn terminate(&self) -> bool {
        let accepted = self.cancel.request(CancellationSource::Debugger);
        self.control.release_stop();
        accepted
    }

    /// Waits for the one terminal winner and converts it to the same strict
    /// observed fault envelope used by ordinary execution.
    pub fn finish(mut self, timeout: Duration) -> Result<ObservedRun, DebugSessionError> {
        let execution = self
            .result
            .recv_timeout(timeout)
            .map_err(|error| match error {
                std::sync::mpsc::RecvTimeoutError::Timeout => DebugSessionError::Timeout,
                std::sync::mpsc::RecvTimeoutError::Disconnected => DebugSessionError::Disconnected,
            })?;
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| DebugSessionError::Disconnected)?;
        }
        let fault = runtime_fault(
            &execution.outcome,
            &execution.frames,
            Some(&self.debug_map),
            &self.run_id,
            self.generation_id,
        )
        .map_err(DebugSessionError::Contract)?;
        Ok(ObservedRun {
            outcome: execution.outcome,
            fault,
            cancellation_source: execution.cancellation_source,
        })
    }
}

impl Drop for RuntimeDebugSession {
    fn drop(&mut self) {
        if self.worker.is_none() {
            return;
        }
        let _ = self.cancel.request(CancellationSource::Debugger);
        self.control.release_stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Clone)]
struct RuntimeDebugHandler {
    state: Arc<(Mutex<DebugControlState>, Condvar)>,
}

impl wasmtime::DebugHandler for RuntimeDebugHandler {
    type Data = RunState;

    fn handle(
        &self,
        mut store: wasmtime::StoreContextMut<'_, Self::Data>,
        event: wasmtime::DebugEvent<'_>,
    ) -> impl std::future::Future<Output = ()> + Send {
        let state = self.state.clone();
        async move {
            let reason = match event {
                wasmtime::DebugEvent::Breakpoint => "breakpoint",
                wasmtime::DebugEvent::EpochYield => {
                    if !state
                        .0
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .pause_requested
                    {
                        return;
                    }
                    "pause"
                }
                _ => return,
            };
            let frames = collect_debug_frames(&mut store);
            let (lock, changed) = &*state;
            let mut control = lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            control.pause_requested = false;
            control.stopped = Some(DebugStop { reason, frames });
            control.resume = false;
            changed.notify_all();
            while !control.resume {
                control = changed
                    .wait(control)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            control.stopped = None;
            control.resume = false;
        }
    }
}

fn collect_debug_frames(
    store: &mut wasmtime::StoreContextMut<'_, RunState>,
) -> Vec<DebugStopFrame> {
    let mut result = Vec::new();
    let activations: Vec<_> = store.debug_exit_frames().take(256).collect();
    for frame in activations {
        let mut current = Some(frame);
        while let Some(frame) = current.take() {
            if result.len() == 256 {
                break;
            }
            let module = frame
                .module(&mut *store)
                .ok()
                .flatten()
                .and_then(|module| module.name().map(str::to_owned));
            if let Ok(Some((_, pc))) = frame.wasm_function_index_and_pc(&mut *store) {
                let locals = collect_debug_locals(&frame, &mut *store);
                result.push(DebugStopFrame {
                    core_module: module,
                    module_pc: pc.raw(),
                    locals,
                });
            }
            current = frame.parent(&mut *store).ok().flatten();
        }
    }
    result
}

fn collect_debug_locals(
    frame: &wasmtime::FrameHandle,
    store: &mut wasmtime::StoreContextMut<'_, RunState>,
) -> Vec<DebugLocalValue> {
    let count = frame.num_locals(&mut *store).unwrap_or(0).min(256);
    (0..count)
        .map(|index| {
            let (type_name, value, available) = match frame.local(&mut *store, index) {
                Ok(wasmtime::Val::I32(value)) => ("i32", value.to_string(), true),
                Ok(wasmtime::Val::I64(value)) => ("i64", value.to_string(), true),
                Ok(wasmtime::Val::F32(bits)) => ("f32", f32::from_bits(bits).to_string(), true),
                Ok(wasmtime::Val::F64(bits)) => ("f64", f64::from_bits(bits).to_string(), true),
                Ok(wasmtime::Val::V128(value)) => {
                    ("v128", format!("0x{:032x}", value.as_u128()), true)
                }
                Ok(_) => ("reference", "<unavailable>".to_owned(), false),
                Err(_) => ("unknown", "<unavailable>".to_owned(), false),
            };
            DebugLocalValue {
                name: format!("local{index}"),
                type_name,
                value,
                available,
            }
        })
        .collect()
}

impl ObservedRun {
    /// Converts a typed pre-execution outcome (for example incompatible or
    /// launch failure) into the same strict fault envelope.
    ///
    /// # Errors
    ///
    /// Rejects an invalid run identity or generation bound.
    pub fn from_outcome(
        outcome: RunOutcome,
        run_id: &str,
        generation_id: u64,
    ) -> Result<Self, ContractError> {
        let fault = runtime_fault(&outcome, &[], None, run_id, generation_id)?;
        Ok(Self {
            outcome,
            fault,
            cancellation_source: None,
        })
    }
}

/// Produces the canonical bounded event stream for one completed execution.
/// Lifecycle and terminal records use only typed Runtime state; guest output
/// crosses the mandatory redactor before it enters the queue.
///
/// # Errors
///
/// Rejects invalid identities, sequence overflow, event contract violations,
/// or a terminal record too large for the reserved queue capacity.
pub fn execution_events<R: EventRedactor>(
    observed: &ObservedRun,
    run_id: &str,
    generation_id: u64,
    redactor: &R,
) -> Result<Vec<Vec<u8>>, ContractError> {
    let mut queue = EventQueue::new();
    let mut sequence = 0_u64;
    for kind in ["accepted", "started"] {
        queue.push(base_event(run_id, generation_id, sequence, kind, "launch"))?;
        sequence += 1;
    }
    if let RunOutcome::Output(output) = &observed.outcome {
        sequence = emit_channel(
            &mut queue,
            run_id,
            generation_id,
            sequence,
            "stdout",
            &output.stdout,
            redactor,
        )?;
        sequence = emit_channel(
            &mut queue,
            run_id,
            generation_id,
            sequence,
            "stderr",
            &output.stderr,
            redactor,
        )?;
    }
    if let Some(source) = observed.cancellation_source {
        let cause = match source {
            CancellationSource::Timer => "timeout",
            CancellationSource::Signal => "signal",
            CancellationSource::Client => "client",
            CancellationSource::Debugger => "debugger",
            CancellationSource::Host => "host",
        };
        queue.push(base_event(
            run_id,
            generation_id,
            sequence,
            "cancellation-requested",
            cause,
        ))?;
        sequence += 1;
    }
    let kind = if observed.fault.is_some() {
        "fault"
    } else {
        "terminal"
    };
    let cause = match observed.outcome {
        RunOutcome::Timeout => "timeout",
        RunOutcome::Cancelled => {
            observed
                .cancellation_source
                .map_or("internal", |source| match source {
                    CancellationSource::Timer => "timeout",
                    CancellationSource::Signal => "signal",
                    CancellationSource::Client => "client",
                    CancellationSource::Debugger => "debugger",
                    CancellationSource::Host => "host",
                })
        }
        RunOutcome::HostProviderFailure { .. } => "host",
        RunOutcome::Launch(_) | RunOutcome::Incompatible(_) => "internal",
        _ => "guest",
    };
    let mut terminal = base_event(run_id, generation_id, sequence, kind, cause);
    terminal.payload.terminal_class = Some(observed.outcome.class().to_owned());
    queue.push(terminal)?;
    Ok(queue.drain().collect())
}

fn emit_channel<R: EventRedactor>(
    queue: &mut EventQueue,
    run_id: &str,
    generation_id: u64,
    sequence: u64,
    kind: &str,
    bytes: &[u8],
    redactor: &R,
) -> Result<u64, ContractError> {
    let captured = &bytes[..bytes.len().min(MAX_CAPTURED_CHANNEL_BYTES)];
    let emitted = queue.push_output(
        base_event(run_id, generation_id, sequence, kind, "guest"),
        captured,
        redactor,
    )?;
    let mut next = sequence
        .checked_add(emitted)
        .ok_or(ContractError::Limit("event-sequence"))?;
    if bytes.len() > MAX_CAPTURED_CHANNEL_BYTES {
        let mut truncated = base_event(run_id, generation_id, next, "truncated", "overflow");
        truncated.payload.message = Some(format!("{kind} capture limit reached"));
        queue.push(truncated)?;
        next += 1;
    }
    Ok(next)
}

fn base_event(
    run_id: &str,
    generation_id: u64,
    sequence: u64,
    kind: &str,
    cause: &str,
) -> ExecutionEvent {
    ExecutionEvent {
        schema: EXECUTION_EVENT_SCHEMA.to_owned(),
        run_id: run_id.to_owned(),
        generation_id,
        sequence,
        kind: kind.to_owned(),
        task_id: Some("task-0".to_owned()),
        parent_task_id: None,
        scope_id: Some("scope-0".to_owned()),
        cause: cause.to_owned(),
        payload: ExecutionEventPayload::default(),
    }
}

/// Validation failure before an observed run can produce a trusted record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationError {
    Input(InputViolation),
    Contract(ContractError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CancelRequestError {
    Contract(ContractError),
    IdentityMismatch,
}

/// Applies one canonical, identity-bound client/debugger cancellation request.
///
/// # Errors
///
/// Rejects malformed/oversized requests and requests for any other run or
/// generation before touching the token.
pub fn apply_cancel_request(
    token: &CancelToken,
    bytes: &[u8],
    run_id: &str,
    generation_id: u64,
) -> Result<bool, CancelRequestError> {
    let request = parse_cancel_request(bytes).map_err(CancelRequestError::Contract)?;
    if request.run_id != run_id || request.generation_id != generation_id {
        return Err(CancelRequestError::IdentityMismatch);
    }
    let source = match request.cause.as_str() {
        "client" => CancellationSource::Client,
        "debugger" => CancellationSource::Debugger,
        _ => unreachable!("strict parser accepts only client/debugger"),
    };
    Ok(token.request(source))
}

impl From<InputViolation> for ObservationError {
    fn from(error: InputViolation) -> Self {
        Self::Input(error)
    }
}

impl From<ContractError> for ObservationError {
    fn from(error: ContractError) -> Self {
        Self::Contract(error)
    }
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
            Self::FuelExhausted | Self::MemoryLimit | Self::StackLimit | Self::Trap(_) => 125,
            Self::HostProviderFailure { .. } => 126,
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
            Self::StackLimit => "resource-limit.stack",
            Self::MemoryLimit => "resource-limit.memory",
            Self::HostProviderFailure { .. } => "host-provider-failure",
            Self::Trap(_) => "trap",
            Self::Launch(_) => "launch-failure",
            Self::Incompatible(_) => "incompatible",
        }
    }
}

/// Shared cancellation handle; [`crate::run_program`] consults it at every
/// epoch tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationSource {
    Timer,
    Signal,
    Client,
    Debugger,
    Host,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalDecision {
    pub outcome: RunOutcome,
    pub cancellation_source: Option<CancellationSource>,
}

#[derive(Debug)]
enum TerminalState {
    Running,
    CancellationRequested(CancellationSource),
    Terminal(TerminalDecision),
}

#[derive(Debug)]
struct TerminalArbiter {
    state: Mutex<TerminalState>,
}

impl TerminalArbiter {
    fn new() -> Self {
        Self {
            state: Mutex::new(TerminalState::Running),
        }
    }

    fn request(&self, source: CancellationSource) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        match &*state {
            TerminalState::Running => {
                *state = TerminalState::CancellationRequested(source);
                true
            }
            TerminalState::CancellationRequested(_) | TerminalState::Terminal(_) => false,
        }
    }

    fn commit(&self, outcome: RunOutcome) -> TerminalDecision {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let decision = match &*state {
            TerminalState::Running => TerminalDecision {
                outcome,
                cancellation_source: None,
            },
            TerminalState::CancellationRequested(source) => TerminalDecision {
                outcome: RunOutcome::Cancelled,
                cancellation_source: Some(*source),
            },
            TerminalState::Terminal(decision) => return decision.clone(),
        };
        *state = TerminalState::Terminal(decision.clone());
        decision
    }
}

#[derive(Debug, Default)]
struct CancelState {
    requested: Option<CancellationSource>,
    active: Option<Weak<TerminalArbiter>>,
}

#[derive(Debug, Default)]
struct CancelInner {
    state: Mutex<CancelState>,
}

#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    inner: Arc<CancelInner>,
}

impl CancelToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        let _ = self.request(CancellationSource::Client);
    }

    /// Records one typed cancellation request. The first request observed by
    /// an active run wins; repeated requests are idempotent.
    pub fn request(&self, source: CancellationSource) -> bool {
        let active = {
            let mut state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if state.requested.is_some() {
                return false;
            }
            let active = state.active.as_ref().and_then(Weak::upgrade);
            if active.is_none() {
                state.requested = Some(source);
                return true;
            }
            active
        };
        let accepted = active.is_some_and(|active| active.request(source));
        if accepted {
            self.inner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .requested = Some(source);
        }
        accepted
    }

    #[must_use]
    pub fn requested_source(&self) -> Option<CancellationSource> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .requested
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.requested_source().is_some()
    }

    fn bind(&self, arbiter: &Arc<TerminalArbiter>) -> Result<CancelBinding, RunOutcome> {
        let requested = {
            let mut state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.strong_count() > 0)
            {
                return Err(RunOutcome::Launch(
                    "cancellation token is already bound to an active run".into(),
                ));
            }
            state.active = Some(Arc::downgrade(arbiter));
            state.requested
        };
        if let Some(source) = requested {
            let _ = arbiter.request(source);
        }
        Ok(CancelBinding {
            inner: Arc::downgrade(&self.inner),
            arbiter: Arc::downgrade(arbiter),
        })
    }
}

struct CancelBinding {
    inner: Weak<CancelInner>,
    arbiter: Weak<TerminalArbiter>,
}

impl Drop for CancelBinding {
    fn drop(&mut self) {
        let (Some(inner), Some(arbiter)) = (self.inner.upgrade(), self.arbiter.upgrade()) else {
            return;
        };
        let mut state = inner
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state
            .active
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|active| Arc::ptr_eq(&active, &arbiter))
        {
            state.active = None;
        }
    }
}

static CONSOLE_TOKENS: OnceLock<Mutex<Vec<Weak<CancelInner>>>> = OnceLock::new();
static CONSOLE_HANDLER: OnceLock<Result<(), String>> = OnceLock::new();

/// Registration guard for the process-global console control handler. Dropping
/// it prevents later signals from cancelling the associated run.
pub struct ConsoleCancellation {
    inner: Weak<CancelInner>,
}

/// Installs the safe process-global Ctrl+C handler once and associates the
/// supplied token with console signals for the guard's lifetime.
///
/// # Errors
///
/// Returns the platform handler installation failure without pretending that
/// Ctrl+C can be classified as typed cancellation.
pub fn register_console_cancellation(token: &CancelToken) -> Result<ConsoleCancellation, String> {
    CONSOLE_HANDLER
        .get_or_init(|| {
            ctrlc::set_handler(dispatch_console_signal).map_err(|error| error.to_string())
        })
        .clone()?;
    let registry = CONSOLE_TOKENS.get_or_init(|| Mutex::new(Vec::new()));
    let mut tokens = registry.lock().unwrap_or_else(|error| error.into_inner());
    tokens.retain(|token| token.strong_count() > 0);
    tokens.push(Arc::downgrade(&token.inner));
    Ok(ConsoleCancellation {
        inner: Arc::downgrade(&token.inner),
    })
}

fn dispatch_console_signal() {
    let Some(registry) = CONSOLE_TOKENS.get() else {
        std::process::exit(130);
    };
    let tokens = {
        let mut tokens = registry.lock().unwrap_or_else(|error| error.into_inner());
        tokens.retain(|token| token.strong_count() > 0);
        tokens.clone()
    };
    let mut delivered = false;
    for inner in tokens.into_iter().filter_map(|token| token.upgrade()) {
        delivered = true;
        let _ = CancelToken { inner }.request(CancellationSource::Signal);
    }
    if !delivered {
        // The safe process-global handler cannot be uninstalled. Outside a
        // scoped registration, preserve ordinary console termination instead
        // of swallowing the signal or forging typed exit 123.
        std::process::exit(130);
    }
}

impl Drop for ConsoleCancellation {
    fn drop(&mut self) {
        let Some(registry) = CONSOLE_TOKENS.get() else {
            return;
        };
        let Some(inner) = self.inner.upgrade() else {
            return;
        };
        registry
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .retain(|token| {
                token
                    .upgrade()
                    .is_some_and(|registered| !Arc::ptr_eq(&registered, &inner))
            });
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

#[derive(Debug)]
struct ProviderFault(&'static str);

impl fmt::Display for ProviderFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Host provider failed")
    }
}

impl Error for ProviderFault {}

struct RunState {
    memory_ceiling: usize,
    denied: bool,
    table: ResourceTable,
    streams_live: u32,
    http_abandoned: bool,
    cancel: CancelToken,
    io: IoWorkers,
    /// The run's bounded scheduler core (M11 STEP-0105): root task plus
    /// identity-checked Host-operation completion ingress.
    scheduler: SchedulerCore,
    /// Per-Store `http@0.2.0` state (RFC-0037): engine, pool and
    /// abandonment flag; dies with the Store.
    http2: crate::http2::Http2State,
    /// RFC-0039 §2.4 (STEP-0147): per-Store package instances; indexes are
    /// captured by the bridge closures at link time.
    package_instances: Vec<wasmtime::component::Instance>,
    /// The prepared package count: the limiter's instance/memory ceilings
    /// widen by exactly this amount, bounded at prepare time.
    package_instances_capacity: usize,
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
pub(crate) fn recv_cancellable<T, E>(
    receiver: &std::sync::mpsc::Receiver<Result<T, E>>,
    cancel: &CancelToken,
) -> Result<T, HostStreamError> {
    loop {
        if cancel.is_cancelled() {
            return Err(HostStreamError::Cancelled);
        }
        match receiver.try_recv() {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(_)) => return Err(HostStreamError::Io),
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
pub(crate) fn send_cancellable<T>(
    sender: &std::sync::mpsc::SyncSender<T>,
    mut value: T,
    cancel: &CancelToken,
) -> Result<(), HostStreamError> {
    loop {
        if cancel.is_cancelled() {
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
pub(crate) enum HostStreamError {
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
        // Program core, the STEP-0083 fs transport module, and one
        // instantiation per RFC-0039 §2.3 package (STEP-0147).
        2 + self.package_instances_capacity
    }

    fn tables(&self) -> usize {
        0
    }

    fn memories(&self) -> usize {
        // The program's single memory plus one per pure package component.
        1 + self.package_instances_capacity
    }
}

/// Runner engine with component model, fuel and epoch interruption enabled.
#[derive(Clone)]
pub struct Runner {
    engine: Engine,
    components: ComponentCache,
    debug_enabled: bool,
    /// Process-local monotonic counter backing synthetic run identities on
    /// the unobserved `run` path (M10 callers pass their own identity).
    run_counter: Arc<AtomicU64>,
}

type ComponentCache = Arc<std::sync::Mutex<Vec<([u8; 32], Component)>>>;

/// Compiled Component and linked Host providers reusable across executions.
/// Every [`PreparedProgram::run`] still constructs a fresh Store, resource
/// table, capability state, IO workers and cancellation boundary.
pub struct PreparedProgram {
    runner: Runner,
    component: Component,
    linker: Linker<RunState>,
    streams_component: bool,
    debug_map: Option<Arc<DebugMap>>,
    debug_core_base: Option<u64>,
    /// HTTP 0.2.0 policy frozen into this generation (RFC-0037 §3/§6);
    /// callers must prepare a new generation to change trust or secrets.
    http_policy: HttpPolicy,
    /// RFC-0039 §2.3/§2.4 (STEP-0147): pure package components available to
    /// this generation, matched against `sico:user/...` guest imports.
    packages: Vec<PreparedPackage>,
}

/// One prepared package component (RFC-0039 §2.4, STEP-0147): a compiled
/// pure component plus the exported `sico:user/<interface>@<version>`
/// instance identities with per-function arities for fail-closed link checks.
/// Exported `sico:user/<instance>` functions of one prepared package:
/// instance identity → function name → (param, result) arity pair.
type PackageExports = BTreeMap<String, BTreeMap<String, (usize, usize)>>;

struct PreparedPackage {
    component: Component,
    functions: PackageExports,
}

/// One raw package component handed to preparation (RFC-0039 §2.3,
/// STEP-0147). The bytes must be a signed-and-verified package's inner
/// component; verification is the caller's (CLI) responsibility and the
/// runner re-checks purity and shape fail-closed.
#[derive(Clone, Debug)]
pub struct PackageBinary {
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugBreakpoint {
    pub core_module: String,
    pub module_pc: u32,
    pub source_offset: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceBreakpointBinding {
    pub requested_offset: u64,
    pub breakpoint: Option<DebugBreakpoint>,
}

impl Runner {
    /// Creates a runner; engine setup is shared, every call still receives a
    /// fresh Store, WASI context (none) and resource table.
    ///
    /// # Errors
    ///
    /// Returns an engine configuration error.
    pub fn new() -> Result<Self, wasmtime::Error> {
        Self::new_with_debug(false)
    }

    /// Creates a runner with Wasmtime guest-debug instrumentation enabled.
    /// Ordinary runs remain uninstrumented because this mode has measurable
    /// compilation and execution overhead.
    ///
    /// # Errors
    ///
    /// Returns an engine configuration error.
    pub fn new_debug() -> Result<Self, wasmtime::Error> {
        Self::new_with_debug(true)
    }

    fn new_with_debug(debug: bool) -> Result<Self, wasmtime::Error> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);
        config.generate_address_map(true);
        config.guest_debug(debug);
        config.wasm_backtrace_max_frames(NonZeroUsize::new(256));
        // Streaming pumps are recursive until the source-level loop lands
        // (STEP-0087): 4 MiB stack covers ~8k iterations at 64 KiB chunks
        // (512 MiB streamed) while staying a hard bound.
        config.max_wasm_stack(4 * 1024 * 1024);
        // The Wasmtime `debug` crate feature also enables async support at
        // compile time, so this invariant applies to ordinary engines even
        // when `guest_debug(false)` keeps their code uninstrumented.
        config.async_stack_size(8 * 1024 * 1024);
        Ok(Self {
            engine: Engine::new(&config)?,
            components: Arc::new(std::sync::Mutex::new(Vec::new())),
            debug_enabled: debug,
            run_counter: Arc::new(AtomicU64::new(0)),
        })
    }

    /// True when the component imports `sico:script/streams@0.1.0` (RFC-0030
    /// mixing rule: the buffered stdin then stays empty). An unreadable
    /// component reports false and fails later at instantiation.
    #[must_use]
    pub fn component_imports_streams(&self, component: &[u8]) -> bool {
        let Ok(component) = self.compile_cached(component) else {
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
        self.run_program_with_net(component, input, limits, cancel, fs, &NetGrants::default())
    }

    /// Runs with explicit scoped filesystem and network grants.
    ///
    /// # Errors
    ///
    /// Returns [`InputViolation`] without any guest execution.
    pub fn run_program_with_net(
        &self,
        component: &[u8],
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        fs: &FsGrants,
        net: &NetGrants,
    ) -> Result<RunOutcome, InputViolation> {
        self.run_program_with_policy(
            component,
            input,
            limits,
            cancel,
            fs,
            net,
            &HttpPolicy::default(),
        )
    }

    /// Runs with explicit filesystem/network grants and the HTTP 0.2.0
    /// policy (trust roots + Host secrets).
    ///
    /// # Errors
    ///
    /// Returns [`InputViolation`] without any guest execution.
    #[allow(clippy::too_many_arguments)]
    pub fn run_program_with_policy(
        &self,
        component: &[u8],
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        fs: &FsGrants,
        net: &NetGrants,
        policy: &HttpPolicy,
    ) -> Result<RunOutcome, InputViolation> {
        match self.prepare_program_with_policy(component, fs, net, policy) {
            Ok(prepared) => prepared.run(input, limits, cancel),
            Err(outcome) => Ok(outcome),
        }
    }

    /// Compiles and links a Program once for persistent development reruns.
    /// Provider grants are frozen into this prepared generation; callers must
    /// prepare a new generation to change authority.
    pub fn prepare_program_with_net(
        &self,
        component: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
    ) -> Result<PreparedProgram, RunOutcome> {
        self.prepare_program_inner(component, fs, net, None, &HttpPolicy::default(), &[])
    }

    /// Debug-enabled variant of [`Runner::prepare_program_with_packages`].
    ///
    /// # Errors
    ///
    /// Returns a typed [`RunOutcome::Launch`]/`Incompatible` failure.
    pub fn prepare_program_with_debug_packages(
        &self,
        component: &[u8],
        debug_map: &[u8],
        debug_identity: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
        packages: &[PackageBinary],
    ) -> Result<PreparedProgram, RunOutcome> {
        let (map, _) = verify_debug_artifacts(component, debug_map, debug_identity)
            .map_err(|_| RunOutcome::Incompatible("debug artifact identity mismatch".into()))?;
        let core_base = embedded_guest_core_base(component)
            .ok_or_else(|| RunOutcome::Incompatible("debug guest core is missing".into()))?;
        self.prepare_program_inner(
            component,
            fs,
            net,
            Some((Arc::new(map), core_base)),
            &HttpPolicy::default(),
            packages,
        )
    }

    /// Compiles and links a Program with RFC-0039 §2.3 package components
    /// (STEP-0147). Package bytes must already be signature-verified and
    /// export-validated (CLI responsibility); the runner re-checks purity
    /// and shape fail-closed at link time.
    ///
    /// # Errors
    ///
    /// Returns a typed [`RunOutcome::Launch`]/`Incompatible` failure.
    pub fn prepare_program_with_packages(
        &self,
        component: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
        packages: &[PackageBinary],
    ) -> Result<PreparedProgram, RunOutcome> {
        self.prepare_program_inner(component, fs, net, None, &HttpPolicy::default(), packages)
    }

    /// [`Runner::run_program_with_policy`] with RFC-0039 §2.3 packages
    /// (STEP-0147): the packages join preparation, so the run links its
    /// `sico:user/...` imports against the caller-verified components.
    ///
    /// # Errors
    ///
    /// Returns input violations; launch/compatibility failures surface as
    /// typed [`RunOutcome`] values.
    #[allow(clippy::too_many_arguments)]
    pub fn run_program_with_policy_and_packages(
        &self,
        component: &[u8],
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        fs: &FsGrants,
        net: &NetGrants,
        policy: &HttpPolicy,
        packages: &[PackageBinary],
    ) -> Result<RunOutcome, InputViolation> {
        match self.prepare_program_inner(component, fs, net, None, policy, packages) {
            Ok(prepared) => prepared.run(input, limits, cancel),
            Err(outcome) => Ok(outcome),
        }
    }

    /// [`Runner::prepare_program_with_net`] with RFC-0039 §2.3 packages.
    ///
    /// # Errors
    ///
    /// Returns a typed [`RunOutcome::Launch`]/`Incompatible` failure.
    pub fn prepare_program_with_net_packages(
        &self,
        component: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
        packages: &[PackageBinary],
    ) -> Result<PreparedProgram, RunOutcome> {
        self.prepare_program_inner(component, fs, net, None, &HttpPolicy::default(), packages)
    }

    /// Compiles and links a Program with an explicit HTTP 0.2.0 policy
    /// (pinned trust roots, Host secret registry).
    ///
    /// # Errors
    ///
    /// Returns a typed [`RunOutcome::Launch`]/`Incompatible` failure.
    pub fn prepare_program_with_policy(
        &self,
        component: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
        policy: &HttpPolicy,
    ) -> Result<PreparedProgram, RunOutcome> {
        self.prepare_program_inner(component, fs, net, None, policy, &[])
    }

    /// Compiles and links a debuggable Program with an explicit HTTP 0.2.0
    /// policy; the Component, map and identity must pass the full digest
    /// chain first.
    ///
    /// # Errors
    ///
    /// Returns a typed [`RunOutcome::Launch`]/`Incompatible` failure.
    pub fn prepare_program_with_debug_policy(
        &self,
        component: &[u8],
        debug_map: &[u8],
        debug_identity: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
        policy: &HttpPolicy,
    ) -> Result<PreparedProgram, RunOutcome> {
        let (map, _) = verify_debug_artifacts(component, debug_map, debug_identity)
            .map_err(|_| RunOutcome::Incompatible("debug artifact identity mismatch".into()))?;
        let core_base = embedded_guest_core_base(component)
            .ok_or_else(|| RunOutcome::Incompatible("debug guest core is missing".into()))?;
        self.prepare_program_inner(
            component,
            fs,
            net,
            Some((Arc::new(map), core_base)),
            policy,
            &[],
        )
    }

    /// Compiles and links a Program only after the Component, map and identity
    /// pass the complete digest/source-document chain.
    pub fn prepare_program_with_debug(
        &self,
        component: &[u8],
        debug_map: &[u8],
        debug_identity: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
    ) -> Result<PreparedProgram, RunOutcome> {
        self.prepare_program_with_debug_policy(
            component,
            debug_map,
            debug_identity,
            fs,
            net,
            &HttpPolicy::default(),
        )
    }

    fn prepare_program_inner(
        &self,
        component: &[u8],
        fs: &FsGrants,
        net: &NetGrants,
        debug: Option<(Arc<DebugMap>, u64)>,
        policy: &HttpPolicy,
        packages: &[PackageBinary],
    ) -> Result<PreparedProgram, RunOutcome> {
        let component = match self.compile_cached(component) {
            Ok(component) => component,
            Err(error) => return Err(RunOutcome::Incompatible(format!("{error:#}"))),
        };
        let streams_component = component
            .component_type()
            .imports(&self.engine)
            .any(|(name, _)| name == "sico:script/streams@0.1.0");
        let prepared_packages = packages
            .iter()
            .map(|package| self.prepare_package(&package.bytes))
            .collect::<Result<Vec<_>, RunOutcome>>()?;
        let mut linker = Linker::new(&self.engine);
        if let Err(error) = link_fs(&mut linker, fs) {
            return Err(RunOutcome::Launch(format!("fs host setup failed: {error}")));
        }
        if let Err(error) = link_streams(&mut linker) {
            return Err(RunOutcome::Launch(format!(
                "streams host setup failed: {error}"
            )));
        }
        if let Err(error) = link_http(&mut linker, net) {
            return Err(RunOutcome::Launch(format!(
                "http host setup failed: {error}"
            )));
        }
        if let Err(error) = crate::http2::link_http2(&mut linker, net) {
            return Err(RunOutcome::Launch(format!(
                "http@0.2.0 host setup failed: {error}"
            )));
        }
        link_packages(&mut linker, &component, &prepared_packages)?;
        Ok(PreparedProgram {
            runner: self.clone(),
            component,
            linker,
            streams_component,
            debug_map: debug.as_ref().map(|(map, _)| map.clone()),
            debug_core_base: debug.map(|(_, base)| base),
            http_policy: policy.clone(),
            packages: prepared_packages,
        })
    }

    /// Compiles one package component and freezes its exported
    /// `sico:user/...` instance identities with per-function arities
    /// (RFC-0039 §2.4, STEP-0147). Packages must be pure: any import of
    /// its own is a typed launch failure — an import can only narrow, and
    /// a package has no grants to narrow from.
    pub fn prepare_program_inner_probe(
        &self,
        component: &[u8],
    ) -> Result<PackageExports, RunOutcome> {
        let package = self.prepare_package(component)?;
        Ok(package.functions)
    }

    fn prepare_package(&self, bytes: &[u8]) -> Result<PreparedPackage, RunOutcome> {
        let component = self
            .compile_cached(bytes)
            .map_err(|error| RunOutcome::Incompatible(format!("package: {error:#}")))?;
        let ty = component.component_type();
        if ty.imports(&self.engine).next().is_some() {
            return Err(RunOutcome::Launch(
                "package component must not import (v0 packages are pure)".into(),
            ));
        }
        let mut functions = BTreeMap::new();
        for (name, item) in ty.exports(&self.engine) {
            let item = item.ty;
            let Some(name) = name.strip_prefix("sico:user/") else {
                return Err(RunOutcome::Launch(format!(
                    "package export {name} is outside the sico:user namespace"
                )));
            };
            let ComponentItem::ComponentInstance(instance) = item else {
                return Err(RunOutcome::Launch(format!(
                    "package export sico:user/{name} must be an instance"
                )));
            };
            let mut entry = BTreeMap::new();
            for (function_name, function_item) in instance.exports(&self.engine) {
                let function_item = function_item.ty;
                let ComponentItem::ComponentFunc(function) = function_item else {
                    return Err(RunOutcome::Launch(format!(
                        "package export sico:user/{name}.{function_name} must be a function"
                    )));
                };
                entry.insert(
                    function_name.to_owned(),
                    (function.params().len(), function.results().len()),
                );
            }
            functions.insert(format!("sico:user/{name}"), entry);
        }
        Ok(PreparedPackage {
            component,
            functions,
        })
    }

    fn compile_cached(&self, bytes: &[u8]) -> Result<Component, wasmtime::Error> {
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        let mut components = self
            .components
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, component)) = components.iter().find(|(key, _)| *key == digest) {
            return Ok(component.clone());
        }
        let component = Component::new(&self.engine, bytes)?;
        if components.len() == 8 {
            components.remove(0);
        }
        components.push((digest, component.clone()));
        Ok(component)
    }
}

/// Registers dynamic bridges for every guest `sico:user/<interface>@<version>`
/// import (RFC-0039 §2.4, STEP-0147). Each bridged function forwards
/// dynamically typed `Val`s to the matching package instance's export in
/// the same Store. Fail-closed: an import without a package, a function
/// the package does not export, or an arity mismatch is a typed launch
/// failure — the CLI's structural validation is never silently relaxed.
fn link_packages(
    linker: &mut Linker<RunState>,
    component: &Component,
    packages: &[PreparedPackage],
) -> Result<(), RunOutcome> {
    let engine = linker.engine().clone();
    // Collect the guest's user-WIT import surface as owned data so the
    // type borrows die before the linker registration below.
    struct NeededImport {
        name: String,
        functions: Vec<(String, usize, usize)>,
    }
    let mut needed = Vec::new();
    for (name, item) in component.component_type().imports(&engine) {
        if !name.starts_with("sico:user/") {
            continue;
        }
        let ComponentItem::ComponentInstance(instance_type) = item.ty else {
            return Err(RunOutcome::Launch(format!(
                "guest import {name} must be an instance"
            )));
        };
        let mut functions = Vec::new();
        for (function_name, function_item) in instance_type.exports(&engine) {
            let ComponentItem::ComponentFunc(function_type) = function_item.ty else {
                return Err(RunOutcome::Launch(format!(
                    "guest import {name}.{function_name} must be a function"
                )));
            };
            functions.push((
                function_name.to_owned(),
                function_type.params().len(),
                function_type.results().len(),
            ));
        }
        needed.push(NeededImport {
            name: name.to_owned(),
            functions,
        });
    }
    for import in &needed {
        let instance_name = &import.name;
        let package_exports = packages
            .iter()
            .find(|package| package.functions.contains_key(instance_name))
            .ok_or_else(|| {
                RunOutcome::Launch(format!(
                    "guest imports {instance_name} but no prepared package exports it"
                ))
            })?
            .functions
            .get(instance_name)
            .expect("package matched by its exported identity");
        let instance_slot = packages
            .iter()
            .position(|package| package.functions.contains_key(instance_name))
            .unwrap_or(0);
        let mut linker_instance = linker
            .instance(instance_name)
            .map_err(|error| RunOutcome::Launch(format!("package bridge instance: {error}")))?;
        for (function_name, guest_params, guest_results) in &import.functions {
            let Some(&(params, results)) = package_exports.get(function_name) else {
                return Err(RunOutcome::Launch(format!(
                    "package does not export {instance_name}.{function_name}"
                )));
            };
            if guest_params != &params || guest_results != &results {
                return Err(RunOutcome::Launch(format!(
                    "package export {instance_name}.{function_name} arity mismatch"
                )));
            }
            let bridge_name = function_name.clone();
            let bridge_identity = instance_name.clone();
            linker_instance
                .func_new(function_name, move |mut caller, _ty, params, results| {
                    let instance = caller.data_mut().package_instances[instance_slot];
                    // The package groups its functions under the exported
                    // interface instance; resolve identity, then function.
                    let Some(instance_index) =
                        instance.get_export_index(&mut caller, None, &bridge_identity)
                    else {
                        return Err(wasmtime::Error::msg(format!(
                            "package instance vanished: {bridge_identity}"
                        )));
                    };
                    let Some(func_index) = instance.get_export_index(
                        &mut caller,
                        Some(&instance_index),
                        bridge_name.as_str(),
                    ) else {
                        return Err(wasmtime::Error::msg(format!(
                            "package function vanished: {bridge_name}"
                        )));
                    };
                    let Some(func) = instance.get_func(&mut caller, func_index) else {
                        return Err(wasmtime::Error::msg(format!(
                            "package function vanished: {bridge_name}"
                        )));
                    };
                    func.call(&mut caller, params, results)
                })
                .map_err(|error| {
                    RunOutcome::Launch(format!(
                        "package bridge {instance_name}.{function_name}: {error}"
                    ))
                })?;
        }
    }
    Ok(())
}

impl PreparedProgram {
    /// True when this generation owns the streaming stdin channel.    /// Instantiates every prepared package into the run's Store (pure
    /// components: an empty linker). Instances die with the Store.
    fn instantiate_packages(
        &self,
        store: &mut Store<RunState>,
    ) -> Result<Vec<wasmtime::component::Instance>, RunOutcome> {
        // The limiter widens by exactly the prepared package count before
        // the first package instantiation consumes its instance/memory.
        store.data_mut().package_instances_capacity = self.packages.len();
        let mut instances = Vec::with_capacity(self.packages.len());
        for package in &self.packages {
            let linker = Linker::new(&self.runner.engine);
            let instance = linker
                .instantiate(&mut *store, &package.component)
                .map_err(|error| {
                    RunOutcome::Launch(format!("package instantiation failed: {error}"))
                })?;
            instances.push(instance);
        }
        Ok(instances)
    }

    #[must_use]
    pub fn imports_streams(&self) -> bool {
        self.streams_component
    }

    /// Binds requested UTF-8 source byte offsets to exact guest-debug PCs.
    /// Unmapped/generated rows remain explicitly unbound.
    #[must_use]
    pub fn bind_source_breakpoints(
        &self,
        document_id: &str,
        offsets: &[u64],
    ) -> Vec<SourceBreakpointBinding> {
        let Some(map) = self.debug_map.as_deref() else {
            return offsets
                .iter()
                .map(|offset| SourceBreakpointBinding {
                    requested_offset: *offset,
                    breakpoint: None,
                })
                .collect();
        };
        let Some(core_base) = self.debug_core_base else {
            return Vec::new();
        };
        offsets
            .iter()
            .map(|offset| {
                let breakpoint = map
                    .mappings
                    .iter()
                    .filter(|mapping| {
                        !mapping.generated
                            && mapping.source.as_ref().is_some_and(|span| {
                                span.document_id == document_id
                                    && span.start <= *offset
                                    && *offset < span.end
                            })
                    })
                    .min_by_key(|mapping| mapping.instruction_start)
                    .and_then(|mapping| {
                        let pc = mapping.instruction_start.checked_sub(core_base)?;
                        Some(DebugBreakpoint {
                            core_module: mapping.core_module.clone(),
                            module_pc: u32::try_from(pc).ok()?,
                            source_offset: *offset,
                        })
                    });
                SourceBreakpointBinding {
                    requested_offset: *offset,
                    breakpoint,
                }
            })
            .collect()
    }

    /// Starts one identity-bound Program debug execution with exact core
    /// breakpoints. The Store is fresh and owned by the returned session.
    ///
    /// # Errors
    ///
    /// Rejects ordinary non-debug runners, missing maps, input bounds,
    /// unbound module PCs, instantiation failures and malformed Programs.
    pub fn start_debug(
        &self,
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        breakpoints: &[DebugBreakpoint],
        run_id: &str,
        generation_id: u64,
    ) -> Result<RuntimeDebugSession, DebugStartError> {
        if !self.runner.debug_enabled {
            return Err(DebugStartError::DebugDisabled);
        }
        check_input(input).map_err(DebugStartError::Input)?;
        let debug_map = self
            .debug_map
            .clone()
            .ok_or(DebugStartError::MissingDebugMap)?;
        let arbiter = Arc::new(TerminalArbiter::new());
        let binding = cancel.bind(&arbiter).map_err(DebugStartError::Runtime)?;
        let mut store = Store::new(
            &self.runner.engine,
            RunState {
                memory_ceiling: limits.memory_bytes,
                denied: false,
                table: ResourceTable::new(),
                streams_live: 0,
                http_abandoned: false,
                cancel: cancel.clone(),
                io: spawn_io_workers(),
                scheduler: SchedulerCore::new(RunIdentity {
                    run_id: run_id.to_owned(),
                    generation_id,
                }),
                http2: crate::http2::Http2State::new(&self.http_policy),
                package_instances: Vec::new(),
                package_instances_capacity: self.packages.len(),
            },
        );
        store.limiter(|state| state);
        store.set_fuel(limits.fuel).map_err(|_| {
            DebugStartError::Runtime(RunOutcome::Launch("fuel configuration failed".into()))
        })?;
        let package_instances = self
            .instantiate_packages(&mut store)
            .map_err(DebugStartError::Runtime)?;
        store.data_mut().package_instances = package_instances;
        let instance = self
            .linker
            .instantiate(&mut store, &self.component)
            .map_err(|error| {
                DebugStartError::Runtime(if store.data().denied {
                    RunOutcome::MemoryLimit
                } else {
                    RunOutcome::Launch(format!("{error}"))
                })
            })?;
        let modules = store.debug_all_modules();
        {
            let mut edit = store
                .edit_breakpoints()
                .ok_or(DebugStartError::DebugDisabled)?;
            for breakpoint in breakpoints {
                let module = modules
                    .iter()
                    .find(|module| module.name() == Some(breakpoint.core_module.as_str()))
                    .or_else(|| (modules.len() == 1).then(|| &modules[0]))
                    .ok_or_else(|| DebugStartError::UnboundBreakpoint(breakpoint.clone()))?;
                edit.add_breakpoint(module, wasmtime::ModulePC::new(breakpoint.module_pc))
                    .map_err(|_| DebugStartError::UnboundBreakpoint(breakpoint.clone()))?;
            }
        }
        let run = instance.get_func(&mut store, "run").ok_or_else(|| {
            DebugStartError::Runtime(RunOutcome::Incompatible(
                "component does not export run".into(),
            ))
        })?;
        let control = RuntimeDebugControl::new(self.runner.engine.clone());
        store.set_debug_handler(control.handler());
        let timeout_ticks = ticks_for(limits.timeout).max(1);
        store.set_epoch_deadline(1);
        let deadline_cancel = cancel.clone();
        let deadline_arbiter = arbiter.clone();
        let mut remaining_ticks = timeout_ticks;
        store.epoch_deadline_callback(move |_| {
            if deadline_cancel.is_cancelled() {
                Err(Marker("cancelled").into())
            } else if remaining_ticks <= 1 {
                let _ = deadline_arbiter.commit(RunOutcome::Timeout);
                Err(Marker("timeout").into())
            } else {
                remaining_ticks -= 1;
                Ok(UpdateDeadline::Continue(1))
            }
        });
        let params = [input_val(input)];
        let hostcall_fuel = input
            .stdin
            .len()
            .saturating_mul(64)
            .saturating_add(64 << 20)
            .max(limits.hostcall_fuel);
        store.set_hostcall_fuel(hostcall_fuel);
        let engine = self.runner.engine.clone();
        let cancel_for_result = cancel.clone();
        let (sender, result) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("sico-guest-debug".to_owned())
            .stack_size(GUEST_WORKER_STACK_BYTES)
            .spawn(move || {
                let done = Arc::new(AtomicBool::new(false));
                let watchdog_done = done.clone();
                let watchdog = std::thread::spawn(move || {
                    for _ in 0..timeout_ticks.saturating_add(1) {
                        std::thread::sleep(TICK);
                        engine.increment_epoch();
                        if watchdog_done.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                });
                let mut results = [Val::Result(Ok(None))];
                let mut execution =
                    match runtime_block_on(run.call_async(&mut store, &params, &mut results)) {
                        Ok(()) => Execution::without_frames(read_result(&results[0])),
                        Err(error) => Execution {
                            outcome: classify_error(&store, &error),
                            frames: engine_frames(&error),
                            cancellation_source: None,
                        },
                    };
                let decision = arbiter.commit(execution.outcome);
                execution.outcome = decision.outcome;
                execution.cancellation_source = decision
                    .cancellation_source
                    .or_else(|| cancel_for_result.requested_source());
                // Same teardown discipline as the plain run path (STEP-0108):
                // the debug session cannot strand tasks or operations.
                let terminal = scheduler_terminal(&execution.outcome);
                if let Err(message) = settle_scheduler(store.data_mut(), terminal) {
                    execution.outcome = RunOutcome::Launch(message);
                }
                done.store(true, Ordering::Relaxed);
                let _ = watchdog.join();
                drop(binding);
                let _ = sender.send(execution);
            })
            .expect("debug guest worker spawns");
        Ok(RuntimeDebugSession {
            control,
            cancel: cancel.clone(),
            result,
            worker: Some(worker),
            debug_map,
            run_id: run_id.to_owned(),
            generation_id,
        })
    }

    /// Executes one isolated generation using the cached Component and Linker.
    ///
    /// # Errors
    ///
    /// Returns [`InputViolation`] before creating a Store.
    pub fn run(
        &self,
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
    ) -> Result<RunOutcome, InputViolation> {
        check_input(input)?;
        let run_id = format!(
            "run-{:016x}",
            self.runner.run_counter.fetch_add(1, Ordering::Relaxed)
        );
        let identity = RunIdentity {
            run_id,
            generation_id: 1,
        };
        Ok(self.run_unchecked(input, limits, cancel, identity).outcome)
    }

    /// Executes once and emits a strict fault record from typed outcome state
    /// and engine frame metadata. Source spans are present only when this
    /// program was prepared through [`Runner::prepare_program_with_debug`].
    ///
    /// # Errors
    ///
    /// Rejects input bounds or an invalid run/fault contract before returning.
    pub fn run_observed(
        &self,
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        run_id: &str,
        generation_id: u64,
    ) -> Result<ObservedRun, ObservationError> {
        check_input(input)?;
        let execution = self.run_unchecked(
            input,
            limits,
            cancel,
            RunIdentity {
                run_id: run_id.to_owned(),
                generation_id,
            },
        );
        let fault = runtime_fault(
            &execution.outcome,
            &execution.frames,
            self.debug_map.as_deref(),
            run_id,
            generation_id,
        )?;
        Ok(ObservedRun {
            outcome: execution.outcome,
            fault,
            cancellation_source: execution.cancellation_source,
        })
    }

    fn run_unchecked(
        &self,
        input: &ScriptInput,
        limits: &RunnerLimits,
        cancel: &CancelToken,
        identity: RunIdentity,
    ) -> Execution {
        let arbiter = Arc::new(TerminalArbiter::new());
        let _binding = match cancel.bind(&arbiter) {
            Ok(binding) => binding,
            Err(outcome) => return Execution::without_frames(outcome),
        };
        let mut store = Store::new(
            &self.runner.engine,
            RunState {
                memory_ceiling: limits.memory_bytes,
                denied: false,
                table: ResourceTable::new(),
                streams_live: 0,
                http_abandoned: false,
                cancel: cancel.clone(),
                io: spawn_io_workers(),
                scheduler: SchedulerCore::new(identity),
                http2: crate::http2::Http2State::new(&self.http_policy),
                package_instances: Vec::new(),
                package_instances_capacity: self.packages.len(),
            },
        );
        store.limiter(|state| state);
        if store.set_fuel(limits.fuel).is_err() {
            let decision =
                arbiter.commit(RunOutcome::Launch("fuel configuration failed".to_owned()));
            return Execution::without_frames(decision.outcome);
        }
        let timeout_ticks = ticks_for(limits.timeout);
        // Check the cancellation token on every watchdog tick. A deadline set
        // directly to `timeout_ticks` would postpone cancellation of busy guest
        // code until the full wall-clock timeout elapsed.
        store.set_epoch_deadline(1);
        let cancel = cancel.clone();
        let deadline_arbiter = arbiter.clone();
        let mut remaining_ticks = timeout_ticks.max(1);
        store.epoch_deadline_callback(move |_| {
            if cancel.is_cancelled() {
                Err(Marker("cancelled").into())
            } else if remaining_ticks <= 1 {
                let _ = deadline_arbiter.commit(RunOutcome::Timeout);
                Err(Marker("timeout").into())
            } else {
                remaining_ticks -= 1;
                Ok(UpdateDeadline::Continue(1))
            }
        });
        // Watchdog: advance the engine epoch until the call finishes.
        let done = Arc::new(AtomicBool::new(false));
        let engine = self.runner.engine.clone();
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

        let root = store.data().scheduler.root_task();
        if let Err(fault) = store.data_mut().scheduler.make_runnable(root) {
            return Execution::without_frames(RunOutcome::Launch(format!("scheduler: {fault}")));
        }
        // Instantiate and invoke on a big-stack guest worker so deep
        // recursion trips the typed wasm-stack budget, never the native
        // thread stack (STEP-0137). The watchdog ticks the engine from this
        // thread while the worker runs.
        let guest = std::thread::scope(|scope| {
            let worker = std::thread::Builder::new()
                .name("sico-guest".to_owned())
                .stack_size(GUEST_WORKER_STACK_BYTES)
                .spawn_scoped(scope, move || {
                    let execution = match self
                        .instantiate_packages(&mut store)
                        .map_err(|outcome| wasmtime::Error::msg(format!("{outcome:?}")))
                        .map(|instances| {
                            store.data_mut().package_instances = instances;
                        })
                        .and_then(|()| self.linker.instantiate(&mut store, &self.component))
                    {
                        Ok(instance) => match instance.get_func(&mut store, "run") {
                            Some(run) => self.invoke(&mut store, &run, input, limits),
                            None => Execution::without_frames(RunOutcome::Incompatible(
                                "component does not export run".to_owned(),
                            )),
                        },
                        Err(error) => {
                            let denied = store.data().denied;
                            return (
                                store,
                                Execution {
                                    frames: engine_frames(&error),
                                    cancellation_source: None,
                                    outcome: if denied {
                                        RunOutcome::MemoryLimit
                                    } else {
                                        RunOutcome::Launch(format!("{error}"))
                                    },
                                },
                            );
                        }
                    };
                    (store, execution)
                });
            match worker {
                Ok(worker) => Ok(worker.join()),
                Err(error) => Err(format!("guest worker spawn: {error}")),
            }
        });
        let (mut store, mut execution) = match guest {
            Ok(Ok(pair)) => pair,
            // A panicked worker takes its Store down with it; there is
            // nothing left to settle, so report the trap directly.
            Ok(Err(_)) => {
                return Execution::without_frames(RunOutcome::Trap(
                    "guest worker panicked".to_owned(),
                ));
            }
            Err(message) => {
                return Execution::without_frames(RunOutcome::Launch(message));
            }
        };
        let decision = arbiter.commit(execution.outcome);
        execution.outcome = decision.outcome;
        execution.cancellation_source = decision.cancellation_source;
        // The root task commits its single terminal state from the typed
        // run outcome, then the scheduler proves teardown discipline before
        // the Store drops (all tasks and Host operations terminal, scopes
        // closed in deterministic reverse order).
        let terminal = scheduler_terminal(&execution.outcome);
        let scheduler_outcome = settle_scheduler(store.data_mut(), terminal);
        if let Err(message) = scheduler_outcome {
            execution.outcome = RunOutcome::Launch(message);
        }
        done.store(true, Ordering::Relaxed);
        let _ = watchdog.join();
        execution
    }

    fn invoke(
        &self,
        store: &mut Store<RunState>,
        run: &wasmtime::component::Func,
        input: &ScriptInput,
        limits: &RunnerLimits,
    ) -> Execution {
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
            Ok(()) => Execution::without_frames(read_result(&results[0])),
            Err(error) => Execution {
                outcome: classify_error(store, &error),
                frames: engine_frames(&error),
                cancellation_source: None,
            },
        }
    }
}

/// Maps the arbitrated run outcome to the root task's terminal kind
/// (M11 STEP-0105); shared by the plain and debug run paths.
fn scheduler_terminal(outcome: &RunOutcome) -> TerminalKind {
    match outcome {
        RunOutcome::Output(_) => TerminalKind::Succeeded,
        RunOutcome::Cancelled | RunOutcome::Timeout => TerminalKind::Cancelled,
        RunOutcome::Domain { .. } => TerminalKind::Failed,
        RunOutcome::FuelExhausted
        | RunOutcome::MemoryLimit
        | RunOutcome::StackLimit
        | RunOutcome::HostProviderFailure { .. }
        | RunOutcome::Trap(_)
        | RunOutcome::Launch(_)
        | RunOutcome::Incompatible(_) => TerminalKind::Failed,
    }
}

/// Commits the root task's terminal state (cancellation drives the same
/// downward tree any descendant takes, STEP-0106) and proves teardown
/// discipline before the Store drops. A teardown failure with a provable
/// absorbing state reports the typed deadlock, not a hang.
fn settle_scheduler(state: &mut RunState, terminal: TerminalKind) -> Result<(), String> {
    let committed = match terminal {
        TerminalKind::Cancelled => state.scheduler.cancel_root().map(|_| ()),
        kind => state.scheduler.commit_root(kind),
    };
    committed
        .and_then(|()| state.scheduler.teardown())
        .map(|_| ())
        .map_err(|fault| match state.scheduler.detect_deadlock() {
            Err(deadlock) => format!("scheduler: {deadlock}"),
            Ok(()) => format!("scheduler: {fault}"),
        })
}

fn embedded_guest_core_base(component: &[u8]) -> Option<u64> {
    Parser::new(0)
        .parse_all(component)
        .filter_map(|payload| match payload.ok()? {
            Payload::ModuleSection {
                unchecked_range, ..
            } => u64::try_from(unchecked_range.start).ok(),
            _ => None,
        })
        .last()
}

struct RuntimeThreadWake(std::thread::Thread);

impl std::task::Wake for RuntimeThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

fn runtime_block_on<F: std::future::Future>(future: F) -> F::Output {
    let waker = std::task::Waker::from(Arc::new(RuntimeThreadWake(std::thread::current())));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}

struct Execution {
    outcome: RunOutcome,
    frames: Vec<RawEngineFrame>,
    cancellation_source: Option<CancellationSource>,
}

impl Execution {
    fn without_frames(outcome: RunOutcome) -> Self {
        Self {
            outcome,
            frames: Vec::new(),
            cancellation_source: None,
        }
    }
}

struct RawEngineFrame {
    module: Option<String>,
    component_function: u64,
    instruction_offset: Option<u64>,
}

fn engine_frames(error: &wasmtime::Error) -> Vec<RawEngineFrame> {
    error
        .downcast_ref::<wasmtime::WasmBacktrace>()
        .map_or_else(Vec::new, |trace| {
            trace
                .frames()
                .iter()
                .take(256)
                .map(|frame| RawEngineFrame {
                    module: frame.module().name().map(str::to_owned),
                    component_function: u64::from(frame.func_index()),
                    instruction_offset: frame
                        .module_offset()
                        .and_then(|offset| u64::try_from(offset).ok()),
                })
                .collect()
        })
}

fn runtime_fault(
    outcome: &RunOutcome,
    engine_frames: &[RawEngineFrame],
    debug_map: Option<&DebugMap>,
    run_id: &str,
    generation_id: u64,
) -> Result<Option<RuntimeFault>, ContractError> {
    let (class, code, key, message) = match outcome {
        RunOutcome::Output(_) => return Ok(None),
        RunOutcome::Domain { .. } => (
            "domain-error",
            "runtime.domain-error",
            "runtime_domain_error",
            "guest returned a domain error",
        ),
        RunOutcome::Cancelled => (
            "cancelled",
            "runtime.cancelled",
            "runtime_cancelled",
            "execution was cancelled",
        ),
        RunOutcome::Timeout => (
            "timeout",
            "runtime.timeout",
            "runtime_timeout",
            "execution deadline elapsed",
        ),
        RunOutcome::FuelExhausted => (
            "resource-limit.fuel",
            "runtime.fuel-exhausted",
            "runtime_fuel_exhausted",
            "execution exhausted its fuel budget",
        ),
        RunOutcome::StackLimit => (
            "resource-limit.stack",
            "runtime.stack-limit",
            "runtime_stack_limit",
            "execution exhausted its call-stack budget",
        ),
        RunOutcome::MemoryLimit => (
            "resource-limit.memory",
            "runtime.memory-limit",
            "runtime_memory_limit",
            "execution exceeded its memory limit",
        ),
        RunOutcome::HostProviderFailure { .. } => (
            "host-provider-failure",
            "runtime.host-provider-failure",
            "runtime_host_provider_failure",
            "a Host provider failed internally",
        ),
        RunOutcome::Trap(_) => (
            "trap",
            "runtime.trap",
            "runtime_trap",
            "guest execution trapped",
        ),
        RunOutcome::Launch(_) => (
            "launch-failure",
            "runtime.launch-failure",
            "runtime_launch_failure",
            "component launch failed",
        ),
        RunOutcome::Incompatible(_) => (
            "incompatible-artifact",
            "runtime.incompatible-artifact",
            "runtime_incompatible_artifact",
            "artifact is incompatible with this runner",
        ),
    };
    let frames = engine_frames
        .iter()
        .take(256)
        .map(|frame| {
            let Some(map) = debug_map else {
                return RuntimeFrame {
                    function_id: "runtime.unmapped".into(),
                    source: None,
                    generated: true,
                    unavailable_reason: Some("debug-map-unavailable".into()),
                };
            };
            let known_module = frame.module.as_deref().filter(|module| {
                map.functions
                    .iter()
                    .any(|function| function.core_module == *module)
            });
            resolve_engine_frame(
                map,
                &EngineFrame {
                    core_module: known_module,
                    component_function: frame.component_function,
                    instruction_offset: frame.instruction_offset,
                },
            )
        })
        .collect();
    let fault = RuntimeFault {
        schema: "sico.runtime-fault.v0".into(),
        run_id: run_id.into(),
        generation_id,
        class: class.into(),
        code: code.into(),
        key: key.into(),
        message: message.into(),
        provider_id: match outcome {
            RunOutcome::HostProviderFailure { provider_id } => Some(provider_id.clone()),
            _ => None,
        },
        frames,
    };
    validate_runtime_fault(&fault)?;
    Ok(Some(fault))
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

#[derive(Debug, wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(record)]
struct HostHttpResponse {
    #[component(name = "status")]
    status: i64,
    #[component(name = "body")]
    body: Vec<u8>,
}

type HttpOutcome = Result<(Result<HostHttpResponse, String>,), wasmtime::Error>;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    host: String,
    port: u16,
    authority: String,
    target: String,
    body: Vec<u8>,
}

/// Wires the default-deny `sico:script/http@0.1.0` provider (RFC-0031).
/// Validation and authorization happen before DNS or socket creation. The
/// blocking request runs on a worker so cancellation/timeout can abandon it.
fn link_http(linker: &mut Linker<RunState>, net: &NetGrants) -> Result<(), wasmtime::Error> {
    let grants = net.clone();
    let mut http = linker.instance("sico:script/http@0.1.0")?;
    http.func_wrap(
        "request",
        move |mut store: wasmtime::StoreContextMut<'_, RunState>,
              (method, url, body): (String, String, Vec<u8>)|
              -> HttpOutcome {
            // A timed-out OS operation cannot be synchronously joined. Fail
            // closed for the remainder of this Store so guest recovery logic
            // cannot accumulate abandoned worker threads.
            if store.data().http_abandoned {
                return Ok((Err("resource-limit".to_owned()),));
            }
            let request = match prepare_http_request(&grants, method, url, body) {
                Ok(request) => request,
                Err(error) => return Ok((Err(error),)),
            };
            let cancel = store.data().cancel.clone();
            let operation = match store
                .data_mut()
                .scheduler
                .register_root_operation(ReadinessClass::HostCompletion)
            {
                Ok(operation) => operation,
                Err(fault) => return Err(wasmtime::Error::msg(format!("scheduler: {fault}"))),
            };
            let deadline = Instant::now() + HTTP_TOTAL_TIMEOUT;
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = sender.send(perform_http_request(request, deadline));
            });
            let outcome = wait_http_response(&receiver, &cancel, deadline);
            let payload = outcome.as_ref().map_or(0, |response| response.body.len());
            // A timed-out operation cannot be synchronously joined: classify
            // it as abandoned so a late worker record is stale, not duplicated
            // (ADR-0010 adversarial trace 3).
            let completed = store
                .data_mut()
                .scheduler
                .complete_root_operation(operation, payload)
                .or_else(|fault| {
                    store
                        .data_mut()
                        .scheduler
                        .abandon_operation(operation)
                        .map_err(|_| fault)
                });
            if let Err(fault) = completed {
                return Err(wasmtime::Error::msg(format!("scheduler: {fault}")));
            }
            if matches!(
                outcome.as_ref().map_err(String::as_str),
                Err("timeout" | "cancelled")
            ) {
                store.data_mut().http_abandoned = true;
            }
            Ok((outcome,))
        },
    )?;
    Ok(())
}

fn prepare_http_request(
    grants: &NetGrants,
    method: String,
    url: String,
    body: Vec<u8>,
) -> Result<HttpRequest, String> {
    if !matches!(method.as_str(), "GET" | "POST") {
        return Err("protocol".to_owned());
    }
    if body.len() > MAX_HTTP_BODY_BYTES {
        return Err("resource-limit".to_owned());
    }
    let parsed = parse_http_url(&url)?;
    if !grants.allows(&parsed.host, parsed.port) {
        return Err("denied".to_owned());
    }
    Ok(HttpRequest {
        method,
        host: parsed.host,
        port: parsed.port,
        authority: parsed.authority,
        target: parsed.target,
        body,
    })
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    authority: String,
    target: String,
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl, String> {
    if url.len() > MAX_HTTP_URL_BYTES {
        return Err("resource-limit".to_owned());
    }
    let Some(rest) = url.strip_prefix("http://") else {
        return Err(if url.contains("://") {
            "denied".to_owned()
        } else {
            "protocol".to_owned()
        });
    };
    if rest.is_empty() || rest.contains('#') || rest.contains('@') {
        return Err("protocol".to_owned());
    }
    let split = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority_text = &rest[..split];
    let remainder = &rest[split..];
    let (host, port) = if authority_text.contains(':') {
        parse_endpoint(authority_text)?
    } else {
        (normalize_host(authority_text)?, 80)
    };
    let target = if remainder.is_empty() {
        "/".to_owned()
    } else if remainder.starts_with('?') {
        format!("/{remainder}")
    } else {
        remainder.to_owned()
    };
    if target.bytes().any(|byte| byte <= b' ' || byte == 0x7f) {
        return Err("protocol".to_owned());
    }
    let authority = if port == 80 {
        host.clone()
    } else {
        format!("{host}:{port}")
    };
    Ok(ParsedHttpUrl {
        host,
        port,
        authority,
        target,
    })
}

fn parse_endpoint(endpoint: &str) -> Result<(String, u16), String> {
    if endpoint.len() > 320 || endpoint.matches(':').count() != 1 {
        return Err("endpoint must be an ASCII host:port pair".to_owned());
    }
    let Some((host, port)) = endpoint.rsplit_once(':') else {
        return Err("endpoint must include a port".to_owned());
    };
    if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("endpoint port must be decimal".to_owned());
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| "endpoint port is outside 1..=65535".to_owned())?;
    if port == 0 {
        return Err("endpoint port is outside 1..=65535".to_owned());
    }
    Ok((normalize_host(host)?, port))
}

fn normalize_host(host: &str) -> Result<String, String> {
    if host.is_empty() || host.len() > 253 || !host.is_ascii() {
        return Err("endpoint host must be bounded ASCII".to_owned());
    }
    let host = host.to_ascii_lowercase();
    if host
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        let address = host
            .parse::<Ipv4Addr>()
            .map_err(|_| "endpoint IPv4 literal is not canonical".to_owned())?;
        if address.to_string() != host {
            return Err("endpoint IPv4 literal is not canonical".to_owned());
        }
        return Ok(host);
    }
    if host.ends_with('.')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err("endpoint DNS host is not canonical".to_owned());
    }
    Ok(host)
}

fn wait_http_response(
    receiver: &std::sync::mpsc::Receiver<Result<HostHttpResponse, String>>,
    cancel: &CancelToken,
    deadline: Instant,
) -> Result<HostHttpResponse, String> {
    loop {
        if cancel.is_cancelled() {
            return Err("cancelled".to_owned());
        }
        if Instant::now() >= deadline {
            return Err("timeout".to_owned());
        }
        match receiver.try_recv() {
            Ok(outcome) => return outcome,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err("connect".to_owned());
            }
        }
    }
}

fn perform_http_request(
    request: HttpRequest,
    deadline: Instant,
) -> Result<HostHttpResponse, String> {
    let addresses = (request.host.as_str(), request.port)
        .to_socket_addrs()
        .map_err(|_| "dns".to_owned())?;
    let mut stream = None;
    for address in addresses.take(32) {
        let timeout = remaining(deadline)?.min(HTTP_CONNECT_TIMEOUT);
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(connected) => {
                stream = Some(connected);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => {}
        }
    }
    let mut stream = stream.ok_or_else(|| {
        if Instant::now() >= deadline {
            "timeout".to_owned()
        } else {
            "connect".to_owned()
        }
    })?;
    set_socket_timeout(&stream, deadline)?;
    let header = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nConnection: close\r\nUser-Agent: sico-runner/0.0.2\r\n\r\n",
        request.method,
        request.target,
        request.authority,
        request.body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(&request.body))
        .map_err(classify_socket_error)?;
    read_http_response(&mut stream, deadline)
}

fn read_http_response(
    stream: &mut TcpStream,
    deadline: Instant,
) -> Result<HostHttpResponse, String> {
    let mut bytes = Vec::new();
    let header_end = loop {
        set_socket_timeout(stream, deadline)?;
        let mut chunk = [0_u8; 8 * 1024];
        let count = stream.read(&mut chunk).map_err(classify_socket_error)?;
        if count == 0 {
            return Err("protocol".to_owned());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(index) = find_header_end(&bytes) {
            if index > MAX_HTTP_HEADER_BYTES {
                return Err("resource-limit".to_owned());
            }
            break index;
        }
        if bytes.len() > MAX_HTTP_HEADER_BYTES {
            return Err("resource-limit".to_owned());
        }
    };
    let (status, content_length) = parse_response_head(&bytes[..header_end])?;
    let mut body = bytes[header_end + 4..].to_vec();
    if body.len() > MAX_HTTP_BODY_BYTES {
        return Err("resource-limit".to_owned());
    }
    match content_length {
        Some(expected) => read_sized_body(stream, deadline, &mut body, expected)?,
        None => read_body_to_eof(stream, deadline, &mut body)?,
    }
    Ok(HostHttpResponse { status, body })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_response_head(head: &[u8]) -> Result<(i64, Option<usize>), String> {
    let text = std::str::from_utf8(head).map_err(|_| "protocol".to_owned())?;
    let mut lines = text.split("\r\n");
    let mut status_parts = lines.next().unwrap_or_default().split_ascii_whitespace();
    let version = status_parts.next().unwrap_or_default();
    let status = status_parts
        .next()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| (100..=999).contains(value))
        .ok_or_else(|| "protocol".to_owned())?;
    if !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err("protocol".to_owned());
    }
    let mut content_length = None;
    for line in lines {
        if line.starts_with([' ', '\t']) {
            return Err("protocol".to_owned());
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err("protocol".to_owned());
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err("protocol".to_owned());
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("protocol".to_owned());
        }
        if name.eq_ignore_ascii_case("content-length") {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|_| "protocol".to_owned())?;
            if content_length.replace(parsed).is_some() {
                return Err("protocol".to_owned());
            }
        }
    }
    Ok((status, content_length))
}

fn read_sized_body(
    stream: &mut TcpStream,
    deadline: Instant,
    body: &mut Vec<u8>,
    expected: usize,
) -> Result<(), String> {
    if expected > MAX_HTTP_BODY_BYTES || body.len() > expected {
        return Err(if expected > MAX_HTTP_BODY_BYTES {
            "resource-limit".to_owned()
        } else {
            "protocol".to_owned()
        });
    }
    while body.len() < expected {
        set_socket_timeout(stream, deadline)?;
        let remaining = expected - body.len();
        let mut chunk = vec![0_u8; remaining.min(64 * 1024)];
        let count = stream.read(&mut chunk).map_err(classify_socket_error)?;
        if count == 0 {
            return Err("protocol".to_owned());
        }
        body.extend_from_slice(&chunk[..count]);
    }
    Ok(())
}

fn read_body_to_eof(
    stream: &mut TcpStream,
    deadline: Instant,
    body: &mut Vec<u8>,
) -> Result<(), String> {
    loop {
        set_socket_timeout(stream, deadline)?;
        let mut chunk = [0_u8; 64 * 1024];
        let count = stream.read(&mut chunk).map_err(classify_socket_error)?;
        if count == 0 {
            return Ok(());
        }
        if body.len().saturating_add(count) > MAX_HTTP_BODY_BYTES {
            return Err("resource-limit".to_owned());
        }
        body.extend_from_slice(&chunk[..count]);
    }
}

fn set_socket_timeout(stream: &TcpStream, deadline: Instant) -> Result<(), String> {
    let timeout = remaining(deadline)?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|_| "connect".to_owned())
}

fn remaining(deadline: Instant) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| "timeout".to_owned())
}

fn classify_socket_error(error: std::io::Error) -> String {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        "timeout".to_owned()
    } else {
        "protocol".to_owned()
    }
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
            // Every Host suspension point is a registered scheduler
            // operation; its completion enters the identity-checked ingress
            // (M11 STEP-0105).
            let operation = match store
                .data_mut()
                .scheduler
                .register_root_operation(ReadinessClass::HostCompletion)
            {
                Ok(operation) => operation,
                Err(fault) => return Err(wasmtime::Error::msg(format!("scheduler: {fault}"))),
            };
            let outcome = send_cancellable(&store.data().io.stdin_request, max, &cancel)
                .and_then(|()| recv_cancellable(&store.data().io.stdin_response, &cancel));
            let payload = outcome.as_ref().map_or(0, Vec::len);
            if let Err(fault) = store
                .data_mut()
                .scheduler
                .complete_root_operation(operation, payload)
            {
                return Err(wasmtime::Error::msg(format!("scheduler: {fault}")));
            }
            if outcome.is_err() {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
            }
            Ok((provider_result(outcome, "sico.streams.stdin")?,))
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
            let operation = match store
                .data_mut()
                .scheduler
                .register_root_operation(ReadinessClass::HostCompletion)
            {
                Ok(operation) => operation,
                Err(fault) => return Err(wasmtime::Error::msg(format!("scheduler: {fault}"))),
            };
            let outcome = send_cancellable(
                &store.data().io.output_request,
                OutputJob::Write {
                    stderr: stderr_channel,
                    bytes,
                },
                &cancel,
            )
            .and_then(|()| recv_cancellable(&store.data().io.output_response, &cancel));
            if let Err(fault) = store
                .data_mut()
                .scheduler
                .complete_root_operation(operation, 0)
            {
                return Err(wasmtime::Error::msg(format!("scheduler: {fault}")));
            }
            if outcome.is_err() {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
            }
            let provider = if stderr_channel {
                "sico.streams.stderr"
            } else {
                "sico.streams.stdout"
            };
            Ok((provider_result(outcome, provider)?,))
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
            let operation = match store
                .data_mut()
                .scheduler
                .register_root_operation(ReadinessClass::HostCompletion)
            {
                Ok(operation) => operation,
                Err(fault) => return Err(wasmtime::Error::msg(format!("scheduler: {fault}"))),
            };
            let outcome = send_cancellable(
                &store.data().io.output_request,
                OutputJob::Flush {
                    stderr: stderr_channel,
                },
                &cancel,
            )
            .and_then(|()| recv_cancellable(&store.data().io.output_response, &cancel));
            if let Err(fault) = store
                .data_mut()
                .scheduler
                .complete_root_operation(operation, 0)
            {
                return Err(wasmtime::Error::msg(format!("scheduler: {fault}")));
            }
            if outcome.is_err() {
                store.data_mut().table.get_mut(&handle)?.terminal = true;
            }
            let provider = if stderr_channel {
                "sico.streams.stderr"
            } else {
                "sico.streams.stdout"
            };
            Ok((provider_result(outcome, provider)?,))
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
            Ok((provider_result(outcome, "sico.streams")?,))
        },
    )?;
    Ok(())
}

fn provider_result<T>(
    outcome: Result<T, HostStreamError>,
    provider_id: &'static str,
) -> Result<Result<T, HostStreamError>, wasmtime::Error> {
    match outcome {
        Err(HostStreamError::Io) => Err(ProviderFault(provider_id).into()),
        other => Ok(other),
    }
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

/// Native stack reserved for the guest worker thread. Deep guest recursion
/// must trip the deterministic `max_wasm_stack` accounting (4 MiB) long
/// before the native thread stack is at risk; 16x headroom guarantees the
/// typed `StackLimit` outcome instead of a process-level native overflow
/// (STEP-0137).
const GUEST_WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;

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
    if let Some(provider) = error.downcast_ref::<ProviderFault>() {
        return RunOutcome::HostProviderFailure {
            provider_id: provider.0.to_owned(),
        };
    }
    if store.data().denied {
        return RunOutcome::MemoryLimit;
    }
    if matches!(store.get_fuel(), Ok(0)) {
        return RunOutcome::FuelExhausted;
    }
    if matches!(
        error.downcast_ref::<wasmtime::Trap>(),
        Some(wasmtime::Trap::StackOverflow)
    ) {
        return RunOutcome::StackLimit;
    }
    RunOutcome::Trap(format!(
        "{error}; downcast={:?}; root={}",
        error.downcast_ref::<wasmtime::Trap>(),
        error
            .source()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "<none>".to_owned())
    ))
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
    use std::future::Future;
    use std::task::{Context, Poll, Wake, Waker};

    struct SecretRedactor;

    impl EventRedactor for SecretRedactor {
        fn redact(&self, bytes: &[u8]) -> Vec<u8> {
            bytes
                .windows(6)
                .enumerate()
                .fold(bytes.to_vec(), |mut output, (index, window)| {
                    if window == b"secret" {
                        output[index..index + 6].copy_from_slice(b"******");
                    }
                    output
                })
        }
    }

    #[test]
    fn observed_output_produces_redacted_bounded_terminal_stream() {
        let observed = ObservedRun {
            outcome: RunOutcome::Output(ScriptOutput {
                stdout: [
                    vec![0xff],
                    b"secret".to_vec(),
                    vec![b'x'; MAX_CAPTURED_CHANNEL_BYTES],
                ]
                .concat(),
                stderr: b"ok".to_vec(),
                exit_code: 0,
            }),
            fault: None,
            cancellation_source: None,
        };
        let frames = execution_events(&observed, "run-1", 7, &SecretRedactor).unwrap();
        let events: Vec<_> = frames
            .iter()
            .map(|frame| sico_observability::parse_execution_event(frame).unwrap())
            .collect();
        assert_eq!(events[0].kind, "accepted");
        assert_eq!(events[1].kind, "started");
        assert!(events.iter().any(|event| event.kind == "truncated"));
        assert_eq!(events.last().unwrap().kind, "terminal");
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.kind.as_str(), "terminal" | "fault"))
                .count(),
            1
        );
        assert!(
            !frames
                .iter()
                .any(|frame| frame.windows(6).any(|window| window == b"secret"))
        );
    }

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

    #[test]
    fn net_grants_and_urls_are_exact_and_canonical() {
        let mut grants = NetGrants::default();
        grants.grant("Example.COM:8080").unwrap();
        grants.grant("127.0.0.1:80").unwrap();
        assert!(grants.allows("example.com", 8080));
        assert!(grants.allows("127.0.0.1", 80));
        assert!(!grants.allows("example.com", 80));
        for invalid in [
            "example.com",
            "example.com:0",
            "*.example.com:80",
            "127.000.0.1:80",
            "[::1]:80",
            "example.com:65536",
        ] {
            assert!(NetGrants::default().grant(invalid).is_err(), "{invalid}");
        }

        let url = parse_http_url("http://Example.COM:8080/path?q=1").unwrap();
        assert_eq!((url.host.as_str(), url.port), ("example.com", 8080));
        assert_eq!(url.authority, "example.com:8080");
        assert_eq!(url.target, "/path?q=1");
        assert_eq!(parse_http_url("http://example.com").unwrap().target, "/");
        for invalid in [
            "https://example.com/",
            "http://user@example.com/",
            "http://example.com/a b",
            "http://example.com/#fragment",
        ] {
            assert!(parse_http_url(invalid).is_err(), "{invalid}");
        }

        assert_eq!(
            prepare_http_request(
                &grants,
                "POST".to_owned(),
                "http://127.0.0.1/".to_owned(),
                vec![0; MAX_HTTP_BODY_BYTES + 1],
            )
            .unwrap_err(),
            "resource-limit"
        );
    }

    #[test]
    fn response_parser_rejects_ambiguous_or_unbounded_headers() {
        assert_eq!(
            parse_response_head(b"HTTP/1.1 200 OK\r\nContent-Length: 3").unwrap(),
            (200, Some(3))
        );
        for invalid in [
            b"HTTP/2 200 OK\r\nContent-Length: 0".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 1".as_slice(),
            b"HTTP/1.1 99 Nope\r\nContent-Length: 0".as_slice(),
        ] {
            assert_eq!(parse_response_head(invalid), Err("protocol".to_owned()));
        }
    }

    #[test]
    fn loopback_http_roundtrip_preserves_status_and_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            while find_header_end(&request).is_none_or(|end| request.len() < end + 4 + 3) {
                let mut chunk = [0_u8; 1024];
                let count = socket.read(&mut chunk).unwrap();
                assert_ne!(count, 0, "client closed before sending the request body");
                request.extend_from_slice(&chunk[..count]);
            }
            let request = std::str::from_utf8(&request).unwrap();
            assert!(request.starts_with("POST /echo?q=1 HTTP/1.1\r\n"));
            assert!(request.contains("Content-Length: 3\r\n"));
            socket
                .write_all(b"HTTP/1.1 307 Temporary Redirect\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc")
                .unwrap();
        });
        let request = HttpRequest {
            method: "POST".to_owned(),
            host: "127.0.0.1".to_owned(),
            port: address.port(),
            authority: format!("127.0.0.1:{}", address.port()),
            target: "/echo?q=1".to_owned(),
            body: b"xyz".to_vec(),
        };
        let response =
            perform_http_request(request, Instant::now() + Duration::from_secs(1)).unwrap();
        assert_eq!(response.status, 307);
        assert_eq!(response.body, b"abc");
        server.join().unwrap();
    }

    #[test]
    fn http_wait_observes_cancellation_without_a_worker_response() {
        let (_sender, receiver) = std::sync::mpsc::sync_channel(1);
        let cancel = CancelToken::new();
        let cancel_thread = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            cancel_thread.cancel();
        });
        let started = Instant::now();
        assert!(matches!(
            wait_http_response(&receiver, &cancel, started + Duration::from_secs(1)),
            Err(error) if error == "cancelled"
        ));
        assert!(started.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn provider_transport_failure_is_typed_not_text_classified() {
        let error =
            provider_result::<()>(Err(HostStreamError::Io), "sico.streams.stdout").unwrap_err();
        let provider = error.downcast_ref::<ProviderFault>().unwrap();
        assert_eq!(provider.0, "sico.streams.stdout");
    }

    #[test]
    fn terminal_arbiter_matches_the_frozen_both_order_race_matrix() {
        let output = RunOutcome::Output(ScriptOutput {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        });
        let domain = RunOutcome::Domain {
            code: "failure".into(),
            message: "failure".into(),
        };
        let host = RunOutcome::HostProviderFailure {
            provider_id: "sico.streams.stdout".into(),
        };

        let completion_first = TerminalArbiter::new();
        assert_eq!(completion_first.commit(output.clone()).outcome, output);
        assert!(!completion_first.request(CancellationSource::Client));
        assert!(matches!(
            completion_first.commit(RunOutcome::Cancelled).outcome,
            RunOutcome::Output(_)
        ));

        let client_first = TerminalArbiter::new();
        assert!(client_first.request(CancellationSource::Client));
        assert_eq!(
            client_first.commit(output).cancellation_source,
            Some(CancellationSource::Client)
        );

        let timeout_first = TerminalArbiter::new();
        assert_eq!(
            timeout_first.commit(RunOutcome::Timeout).outcome,
            RunOutcome::Timeout
        );
        assert!(!timeout_first.request(CancellationSource::Signal));

        let signal_first = TerminalArbiter::new();
        assert!(signal_first.request(CancellationSource::Signal));
        assert_eq!(
            signal_first.commit(RunOutcome::Timeout),
            TerminalDecision {
                outcome: RunOutcome::Cancelled,
                cancellation_source: Some(CancellationSource::Signal),
            }
        );

        let host_first = TerminalArbiter::new();
        assert_eq!(host_first.commit(host.clone()).outcome, host);
        assert!(!host_first.request(CancellationSource::Client));

        let client_before_host = TerminalArbiter::new();
        assert!(client_before_host.request(CancellationSource::Client));
        assert!(matches!(
            client_before_host.commit(host).outcome,
            RunOutcome::Cancelled
        ));

        let failure_first = TerminalArbiter::new();
        assert_eq!(failure_first.commit(domain.clone()).outcome, domain);
        assert_eq!(
            failure_first.commit(RunOutcome::Timeout).outcome.class(),
            "domain-error"
        );

        let timeout_before_failure = TerminalArbiter::new();
        assert_eq!(
            timeout_before_failure.commit(RunOutcome::Timeout).outcome,
            RunOutcome::Timeout
        );
        assert_eq!(
            timeout_before_failure.commit(domain).outcome,
            RunOutcome::Timeout
        );

        let double_cancel = TerminalArbiter::new();
        assert!(double_cancel.request(CancellationSource::Client));
        assert!(!double_cancel.request(CancellationSource::Signal));
        assert_eq!(
            double_cancel
                .commit(RunOutcome::Timeout)
                .cancellation_source,
            Some(CancellationSource::Client)
        );
    }

    #[test]
    fn cancellation_request_is_identity_bound_and_idempotent() {
        let token = CancelToken::new();
        let request = br#"{"schema":"sico.cancel-request.v0","run_id":"run-1","generation_id":2,"cause":"debugger"}"#;
        assert_eq!(apply_cancel_request(&token, request, "run-1", 2), Ok(true));
        assert_eq!(token.requested_source(), Some(CancellationSource::Debugger));
        assert_eq!(apply_cancel_request(&token, request, "run-1", 2), Ok(false));

        let other = CancelToken::new();
        assert_eq!(
            apply_cancel_request(&other, request, "run-1", 3),
            Err(CancelRequestError::IdentityMismatch)
        );
        assert_eq!(other.requested_source(), None);
    }

    #[test]
    fn real_component_exposes_guest_debug_modules_and_patchable_breakpoint() {
        let runner = Runner::new_debug().unwrap();
        let prepared = runner
            .prepare_program_with_net(
                &debug_probe_component(),
                &FsGrants::default(),
                &NetGrants::default(),
            )
            .unwrap();
        let mut store = Store::new(
            &prepared.runner.engine,
            RunState {
                memory_ceiling: RunnerLimits::default().memory_bytes,
                denied: false,
                table: ResourceTable::new(),
                streams_live: 0,
                http_abandoned: false,
                cancel: CancelToken::new(),
                io: spawn_io_workers(),
                http2: crate::http2::Http2State::new(&HttpPolicy::default()),
                scheduler: SchedulerCore::new(RunIdentity {
                    run_id: "debug-probe".to_owned(),
                    generation_id: 1,
                }),
                package_instances: Vec::new(),
                package_instances_capacity: 0,
            },
        );
        let instance = prepared
            .linker
            .instantiate(&mut store, &prepared.component)
            .unwrap();
        let modules = store.debug_all_modules();
        assert_eq!(modules.len(), 1);
        let mut edit = store.edit_breakpoints().expect("guest debug is enabled");
        edit.add_breakpoint(&modules[0], wasmtime::ModulePC::new(0))
            .unwrap();
        drop(edit);
        assert_eq!(store.breakpoints().unwrap().count(), 1);
        let control = RuntimeDebugControl::new(prepared.runner.engine.clone());
        store.set_debug_handler(control.handler());
        store.set_fuel(20_000_000).unwrap();
        store.set_epoch_deadline(u64::MAX);
        let function = instance.get_func(&mut store, "probe").unwrap();
        let execution = std::thread::spawn(move || {
            block_on(function.call_async(&mut store, &[], &mut [])).unwrap();
            (store, instance)
        });
        let stopped = control
            .wait_for_stop(Duration::from_secs(2))
            .expect("real Component breakpoint must stop");
        assert_eq!(stopped.reason, "breakpoint");
        assert_eq!(stopped.frames.len(), 1);
        assert!(control.continue_execution());
        let (mut store, instance) = execution.join().unwrap();

        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(|_| Ok(UpdateDeadline::Continue(1)));
        let busy = instance.get_func(&mut store, "busy").unwrap();
        let paused_control = control.clone();
        let execution = std::thread::spawn(move || {
            block_on(busy.call_async(&mut store, &[], &mut [])).unwrap();
        });
        paused_control.request_pause();
        let stopped = paused_control
            .wait_for_stop(Duration::from_secs(2))
            .expect("epoch pause must stop busy Component code");
        assert_eq!(stopped.reason, "pause");
        assert!(paused_control.continue_execution());
        execution.join().unwrap();
    }

    struct ThreadWake(std::thread::Thread);

    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::park(),
            }
        }
    }

    fn debug_probe_component() -> Vec<u8> {
        use wasm_encoder::{
            CodeSection, ComponentBuilder, ComponentExportKind, ExportKind, ExportSection,
            Function, FunctionSection, Instruction, Module, ModuleArg, TypeSection,
        };

        let mut types = TypeSection::new();
        types.ty().function([], []);
        let mut functions = FunctionSection::new();
        functions.function(0);
        functions.function(0);
        let mut exports = ExportSection::new();
        exports.export("probe", ExportKind::Func, 0);
        exports.export("busy", ExportKind::Func, 1);
        let mut body = Function::new([]);
        body.instruction(&Instruction::Nop);
        body.instruction(&Instruction::End);
        let mut code = CodeSection::new();
        code.function(&body);
        let mut busy = Function::new([(1, wasm_encoder::ValType::I32)]);
        busy.instruction(&Instruction::I32Const(2_000_000));
        busy.instruction(&Instruction::LocalSet(0));
        busy.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        busy.instruction(&Instruction::LocalGet(0));
        busy.instruction(&Instruction::I32Const(1));
        busy.instruction(&Instruction::I32Sub);
        busy.instruction(&Instruction::LocalTee(0));
        busy.instruction(&Instruction::BrIf(0));
        busy.instruction(&Instruction::End);
        busy.instruction(&Instruction::End);
        code.function(&busy);
        let mut module = Module::new();
        module.section(&types);
        module.section(&functions);
        module.section(&exports);
        module.section(&code);

        let mut component = ComponentBuilder::default();
        let module = component.core_module_raw(Some("debug-probe"), &module.finish());
        let instance = component.core_instantiate(
            Some("debug-probe"),
            module,
            std::iter::empty::<(&str, ModuleArg)>(),
        );
        let function =
            component.core_alias_export(Some("probe"), instance, "probe", ExportKind::Func);
        let busy = component.core_alias_export(Some("busy"), instance, "busy", ExportKind::Func);
        let (function_type, mut encoder) = component.type_function(Some("probe"));
        encoder.params(std::iter::empty::<(&str, wasm_encoder::ComponentValType)>());
        encoder.result(None);
        let lifted = component.lift_func(Some("probe"), function, function_type, []);
        component.export("probe", ComponentExportKind::Func, lifted, None);
        let (busy_type, mut encoder) = component.type_function(Some("busy"));
        encoder.params(std::iter::empty::<(&str, wasm_encoder::ComponentValType)>());
        encoder.result(None);
        let lifted = component.lift_func(Some("busy"), busy, busy_type, []);
        component.export("busy", ComponentExportKind::Func, lifted, None);
        component.finish()
    }
}
