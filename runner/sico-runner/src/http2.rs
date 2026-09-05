//! The guest-visible `sico:script/http@0.2.0` provider (RFC-0037, M12 runner
//! integration). Every call composes the same policy stack as the buffered
//! engine — canonical endpoint, address gate, exact grants, TLS/plaintext
//! transport, strict framing, redirects, Host-injected secrets — under one
//! per-Store lifecycle: 16 in-flight requests, one terminal outcome per
//! request, bounded streaming reads through a dedicated worker, and a
//! connection pool that dies with the Store.
//!
//! Error texts never cross into the guest: every refusal is one typed
//! `http-error` discriminant (`permission`, `authority`, `tls`, `dns`,
//! `limit`, `cancel`, `protocol`, `io`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use sico_http_provider::engine::{
    HttpEngine, StreamedResponse, StreamingRequest as ProviderRequest,
};
use sico_http_provider::framing::DEFAULT_BODY_BUDGET;
use wasmtime::component::{ComponentType, Lift, Lower, Resource, ResourceType};

use crate::scheduler::{OperationId, ReadinessClass};
use crate::{CancelToken, HttpPolicy, NetGrants, RunState, send_cancellable};

/// The versioned import this module links (RFC-0037 §1).
pub const HTTP2_INTERFACE: &str = "sico:script/http@0.2.0";

/// Per-Store bound on live response-body / request-body handles.
const MAX_LIVE_BODIES: u32 = 16;
/// Guest-selectable request deadlines are clamped to this Host ceiling.
pub const MAX_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
/// Deadline applied when the guest passes `timeout-ms: 0`.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Buffered requests drain at most this many bytes (the RFC-0037 §7 default
/// body budget).
pub const MAX_BUFFERED_BODY_BYTES: usize = DEFAULT_BODY_BUDGET as usize;
/// Request head bounds mirrored at the WIT surface (RFC-0037 §2/§7).
const MAX_HEADERS: usize = 256;
const MAX_HEADER_NAME: usize = 256;
const MAX_HEADER_VALUE: usize = 8 * 1024;
const MAX_SERIALIZED_HEADERS: usize = 64 * 1024;
/// Streaming read clamp per call (one bounded chunk).
const MAX_BODY_READ: u64 = 64 * 1024;
/// Host-side wait ceiling for a finishing upload (the pipe's own total
/// deadline bounds the request; this bounds the wait, not the transfer).
const FINISH_WAIT: Duration = Duration::from_secs(310);

/// RFC-0037 §1 structured error taxonomy, in WIT declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentType, Lower)]
#[component(enum)]
#[repr(u8)]
pub enum HttpError {
    #[component(name = "permission")]
    Permission,
    #[component(name = "authority")]
    Authority,
    #[component(name = "tls")]
    Tls,
    #[component(name = "dns")]
    Dns,
    #[component(name = "limit")]
    Limit,
    #[component(name = "cancel")]
    Cancel,
    #[component(name = "protocol")]
    Protocol,
    #[component(name = "io")]
    Io,
}

/// Maps one provider diagnostic (`"class: detail"`) to its typed
/// discriminant. The detail text stays Host-side.
#[must_use]
pub fn classify_provider_error(message: &str) -> HttpError {
    let class = message.split(':').next().unwrap_or_default().trim();
    match class {
        "permission" => HttpError::Permission,
        "authority" => HttpError::Authority,
        "tls" => HttpError::Tls,
        "dns" => HttpError::Dns,
        "limit" | "timeout" => HttpError::Limit,
        "cancel" => HttpError::Cancel,
        "protocol" | "redirect" => HttpError::Protocol,
        _ => HttpError::Io,
    }
}

/// One request/response header at the WIT surface.
#[derive(Debug, ComponentType, Lift, Lower)]
#[component(record)]
pub struct HostHeader {
    #[component(name = "name")]
    pub name: String,
    #[component(name = "value")]
    pub value: String,
}

/// Response head: status plus headers.
#[derive(Debug, ComponentType, Lower)]
#[component(record)]
pub struct HostResponseHead {
    #[component(name = "status")]
    pub status: i64,
    #[component(name = "headers")]
    pub headers: Vec<HostHeader>,
}

/// Buffered response (the 0.1.0 compatibility shape plus headers).
#[derive(Debug, ComponentType, Lower)]
#[component(record)]
pub struct HostResponse {
    #[component(name = "status")]
    pub status: i64,
    #[component(name = "headers")]
    pub headers: Vec<HostHeader>,
    #[component(name = "body")]
    pub body: Vec<u8>,
}

/// Streaming response: head plus the affine body handle.
#[derive(ComponentType, Lower)]
#[component(record)]
pub struct HostStreamingResponse {
    #[component(name = "head")]
    pub head: HostResponseHead,
    #[component(name = "body")]
    pub body: Resource<HostResponseBody>,
}

/// Per-request options (redirect/retry policy is explicit; never ambient).
#[derive(Debug, ComponentType, Lift)]
#[component(record)]
pub struct HostRequestOptions {
    #[component(name = "headers")]
    pub headers: Vec<HostHeader>,
    #[component(name = "follow-redirects")]
    pub follow_redirects: bool,
    #[component(name = "retry-idempotent")]
    pub retry_idempotent: bool,
    #[component(name = "timeout-ms")]
    pub timeout_ms: u64,
}

/// Host payload behind one affine `response-body` handle: a dedicated
/// worker owns the provider body so a blocked read stays cancellable.
pub struct HostResponseBody {
    request_tx: Option<std::sync::mpsc::SyncSender<u64>>,
    chunk_rx: Option<std::sync::mpsc::Receiver<Result<Vec<u8>, String>>>,
    terminal: bool,
}

/// Host payload behind one affine `request-body` handle.
pub struct HostRequestBody {
    pipe: Option<sico_http_provider::engine::UploadPipe>,
}

/// Per-Store HTTP 0.2.0 state: one engine (one pool, one secret view), the
/// abandoned flag, and the live-handle counter. Everything dies with the
/// Store; no socket or pooled connection crosses a run or generation.
pub struct Http2State {
    engine: Arc<HttpEngine>,
    abandoned: bool,
    bodies_live: u32,
}

impl Http2State {
    /// Builds the per-Store state from the frozen policy and the default
    /// body budget.
    #[must_use]
    pub fn new(policy: &HttpPolicy) -> Self {
        let secrets = policy.secrets.clone().unwrap_or_else(|| {
            std::sync::Arc::new(sico_http_provider::secrets::SecretStore::new())
        });
        Self {
            engine: Arc::new(HttpEngine::new(
                policy.trust_roots_pem.clone(),
                secrets,
                DEFAULT_BODY_BUDGET,
            )),
            abandoned: false,
            bodies_live: 0,
        }
    }
}

fn provider_request(
    method: String,
    url: String,
    options: &HostRequestOptions,
) -> Result<ProviderRequest, HttpError> {
    validate_method(&method)?;
    validate_headers(&options.headers)?;
    Ok(ProviderRequest {
        method,
        url,
        headers: options
            .headers
            .iter()
            .map(|header| (header.name.clone(), header.value.clone()))
            .collect(),
        follow_redirects: options.follow_redirects,
        retry_idempotent: options.retry_idempotent,
        deadline: request_deadline(options.timeout_ms),
    })
}

fn request_deadline(timeout_ms: u64) -> Duration {
    if timeout_ms == 0 {
        DEFAULT_REQUEST_TIMEOUT
    } else {
        Duration::from_millis(timeout_ms).min(MAX_REQUEST_TIMEOUT)
    }
}

fn validate_method(method: &str) -> Result<(), HttpError> {
    const ALLOWED: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"];
    if ALLOWED.contains(&method) {
        Ok(())
    } else {
        Err(HttpError::Protocol)
    }
}

fn validate_headers(headers: &[HostHeader]) -> Result<(), HttpError> {
    if headers.len() > MAX_HEADERS {
        return Err(HttpError::Limit);
    }
    let mut serialized = 0_usize;
    for header in headers {
        if header.name.is_empty()
            || header.name.len() > MAX_HEADER_NAME
            || header.value.len() > MAX_HEADER_VALUE
            || !header
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || header.value.bytes().any(|byte| byte < 0x20 || byte == 0x7f)
        {
            return Err(HttpError::Protocol);
        }
        serialized = serialized.saturating_add(header.name.len() + header.value.len() + 4);
    }
    if serialized > MAX_SERIALIZED_HEADERS {
        return Err(HttpError::Limit);
    }
    Ok(())
}

fn head_to_host(head: &sico_http_provider::streaming::ResponseHead) -> HostResponseHead {
    HostResponseHead {
        status: i64::from(head.status),
        headers: head
            .headers
            .iter()
            .map(|(name, value)| HostHeader {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
    }
}

/// Spawns the per-body worker owning `response`'s body. Reads flow through
/// a depth-1 rendezvous so the guest observes backpressure; EOF is one
/// empty ok payload and the first error is terminal. A clean EOF returns
/// the connection to the Store pool inside the provider body.
fn spawn_body_worker(response: StreamedResponse) -> HostResponseBody {
    let (request_tx, request_rx) = std::sync::mpsc::sync_channel::<u64>(1);
    let (chunk_tx, chunk_rx) = std::sync::mpsc::sync_channel(1);
    let mut body = response.body;
    let worker = std::thread::spawn(move || {
        while let Ok(max) = request_rx.recv() {
            let want = usize::try_from(max)
                .unwrap_or(usize::MAX)
                .min(MAX_BODY_READ as usize);
            match body.read(want) {
                Ok(Some(chunk)) => {
                    if chunk_tx.send(Ok(chunk)).is_err() {
                        break;
                    }
                }
                Ok(None) => {
                    let _ = chunk_tx.send(Ok(Vec::new()));
                    break;
                }
                Err(error) => {
                    let _ = chunk_tx.send(Err(error));
                    break;
                }
            }
        }
        // Dropping the body closes an unfinished connection; a clean EOF
        // already returned the transport to the pool.
        drop(body);
    });
    let _ = worker;
    HostResponseBody {
        request_tx: Some(request_tx),
        chunk_rx: Some(chunk_rx),
        terminal: false,
    }
}

fn drain_buffered(response: StreamedResponse) -> Result<Vec<u8>, String> {
    let mut body = response.body;
    let mut out = Vec::new();
    loop {
        match body.read(MAX_BODY_READ as usize) {
            Ok(Some(chunk)) => {
                if out.len().saturating_add(chunk.len()) > MAX_BUFFERED_BODY_BYTES {
                    return Err("limit: buffered body exceeds budget".to_owned());
                }
                out.extend_from_slice(&chunk);
            }
            Ok(None) => return Ok(out),
            Err(error) => return Err(error),
        }
    }
}

type HttpOutcome<T> = Result<(Result<T, HttpError>,), wasmtime::Error>;

/// Wires the default-deny `sico:script/http@0.2.0` instance. The instance
/// always exists so a component importing it links; every call re-checks
/// the exact endpoint grants and fails closed with a typed error.
///
/// # Errors
///
/// Linker setup failures only; policy refusals are guest-visible results.
#[allow(clippy::too_many_lines)]
pub(crate) fn link_http2(
    linker: &mut wasmtime::component::Linker<RunState>,
    net: &NetGrants,
) -> Result<(), wasmtime::Error> {
    let endpoint_set = Arc::new(net.http2_endpoint_set());
    let mut http = linker.instance(HTTP2_INTERFACE)?;

    http.resource(
        "response-body",
        ResourceType::host::<HostResponseBody>(),
        |mut cx, rep| {
            let state = cx.data_mut();
            let body = state
                .table
                .delete(Resource::<HostResponseBody>::new_own(rep))?;
            // Dropping the senders ends the worker; its body drop closes an
            // unfinished connection (a clean EOF already pooled it).
            drop(body);
            let http2 = &mut state.http2;
            http2.bodies_live = http2.bodies_live.saturating_sub(1);
            Ok(())
        },
    )?;
    http.resource(
        "request-body",
        ResourceType::host::<HostRequestBody>(),
        |mut cx, rep| {
            let state = cx.data_mut();
            let handle = state
                .table
                .delete(Resource::<HostRequestBody>::new_own(rep))?;
            // UploadPipe::drop aborts the worker, joins it, and releases
            // the request's in-flight slot; drop = abort with no response.
            drop(handle);
            let http2 = &mut state.http2;
            http2.bodies_live = http2.bodies_live.saturating_sub(1);
            Ok(())
        },
    )?;

    // Buffered one-shot: the 0.1.0 compatibility profile over the secure
    // engine. Blocking phases run on a worker; cancellation and timeout
    // abandon the Store's HTTP surface (fail closed) exactly like 0.1.0.
    let endpoints = Arc::clone(&endpoint_set);
    http.func_wrap(
        "request",
        move |mut store: wasmtime::StoreContextMut<'_, RunState>,
              (method, url, options, body): (String, String, HostRequestOptions, Vec<u8>)|
              -> HttpOutcome<HostResponse> {
            if store.data().http2.abandoned {
                return Ok((Err(HttpError::Limit),));
            }
            let request = match provider_request(method, url, &options) {
                Ok(request) => request,
                Err(error) => return Ok((Err(error),)),
            };
            if body.len() > MAX_BUFFERED_BODY_BYTES {
                return Ok((Err(HttpError::Limit),));
            }
            let cancel = store.data().cancel.clone();
            let operation = register_operation(&mut store)?;
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            let engine = Arc::clone(&store.data().http2.engine);
            let endpoints = Arc::clone(&endpoints);
            let total_deadline = request.deadline;
            std::thread::spawn(move || {
                let _ = sender.send(
                    engine
                        .execute_streaming(&request, &body, &endpoints)
                        .and_then(|response| {
                            let head = head_to_host(&response.head);
                            drain_buffered(response).map(|drained| (head, drained))
                        }),
                );
            });
            let outcome = wait_worker(&receiver, &cancel, total_deadline);
            let payload = match &outcome {
                Ok(Ok((_, drained))) => drained.len(),
                _ => 0,
            };
            complete_operation(&mut store, operation, payload)?;
            if abandon_on(&outcome) {
                store.data_mut().http2.abandoned = true;
            }
            Ok((match outcome {
                Ok(Ok((head, drained))) => Ok(HostResponse {
                    status: head.status,
                    headers: head.headers,
                    body: drained,
                }),
                Ok(Err(message)) => Err(classify_provider_error(&message)),
                Err(http_error) => Err(http_error),
            },))
        },
    )?;

    // Streaming response: head plus the affine body handle.
    let endpoints = Arc::clone(&endpoint_set);
    http.func_wrap(
        "request-streaming",
        move |mut store: wasmtime::StoreContextMut<'_, RunState>,
              (method, url, options, body): (String, String, HostRequestOptions, Vec<u8>)|
              -> HttpOutcome<HostStreamingResponse> {
            if store.data().http2.abandoned {
                return Ok((Err(HttpError::Limit),));
            }
            let request = match provider_request(method, url, &options) {
                Ok(request) => request,
                Err(error) => return Ok((Err(error),)),
            };
            if body.len() > MAX_BUFFERED_BODY_BYTES {
                return Ok((Err(HttpError::Limit),));
            }
            let cancel = store.data().cancel.clone();
            let operation = register_operation(&mut store)?;
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            let engine = Arc::clone(&store.data().http2.engine);
            let endpoints = Arc::clone(&endpoints);
            let total_deadline = request.deadline;
            std::thread::spawn(move || {
                let _ = sender.send(engine.execute_streaming(&request, &body, &endpoints));
            });
            let outcome = wait_worker(&receiver, &cancel, total_deadline);
            complete_operation(&mut store, operation, 0)?;
            if abandon_on(&outcome) {
                store.data_mut().http2.abandoned = true;
            }
            let response = match outcome {
                Ok(Ok(response)) => response,
                Ok(Err(message)) => return Ok((Err(classify_provider_error(&message)),)),
                Err(http_error) => return Ok((Err(http_error),)),
            };
            let head = head_to_host(&response.head);
            let body = push_body_resource(&mut store, spawn_body_worker(response))?;
            Ok((Ok(HostStreamingResponse { head, body }),))
        },
    )?;

    // Affine bounded read: at most `max` bytes; empty ok list is EOF; the
    // first error is terminal for the handle.
    http.func_wrap(
        "[method]response-body.read",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         (handle, max): (Resource<HostResponseBody>, u64)|
         -> HttpOutcome<Vec<u8>> {
            let cancel = store.data().cancel.clone();
            if max == 0 {
                if let Ok(body) = store.data_mut().table.get_mut(&handle) {
                    body.terminal = true;
                }
                return Ok((Err(HttpError::Limit),));
            }
            // The receiver cannot be cloned; take it out for the duration
            // of this read and put it back unless the body went terminal.
            let channel = {
                let body = match store.data_mut().table.get_mut(&handle) {
                    Ok(body) if !body.terminal => body,
                    _ => return Ok((Err(HttpError::Protocol),)),
                };
                let request_tx = body.request_tx.clone();
                (request_tx, body.chunk_rx.take())
            };
            let (request_tx, chunk_rx) = channel;
            let (Some(request_tx), Some(chunk_rx)) = (request_tx, chunk_rx) else {
                return Ok((Err(HttpError::Protocol),));
            };
            let operation = register_operation(&mut store)?;
            let received = match send_cancellable(&request_tx, max, &cancel) {
                Ok(()) => recv_body_chunk(&chunk_rx, &cancel),
                Err(error) => Err(error),
            };
            let payload = match &received {
                Ok(Ok(bytes)) => bytes.len(),
                _ => 0,
            };
            complete_operation(&mut store, operation, payload)?;
            let outcome = match received {
                Ok(Ok(chunk)) => {
                    if chunk.is_empty()
                        && let Ok(body) = store.data_mut().table.get_mut(&handle)
                    {
                        body.terminal = true;
                    }
                    Ok(chunk)
                }
                Ok(Err(message)) => {
                    if let Ok(body) = store.data_mut().table.get_mut(&handle) {
                        body.terminal = true;
                    }
                    Err(classify_provider_error(&message))
                }
                Err(crate::HostStreamError::Cancelled) => Err(HttpError::Cancel),
                Err(crate::HostStreamError::Io)
                | Err(crate::HostStreamError::Closed)
                | Err(crate::HostStreamError::ResourceLimit) => Err(HttpError::Io),
            };
            // Return the receiver unless the body is terminal; dropping it
            // ends the worker and closes the connection.
            if let Ok(body) = store.data_mut().table.get_mut(&handle)
                && !body.terminal
            {
                body.chunk_rx = Some(chunk_rx);
            }
            Ok((outcome,))
        },
    )?;

    // Streaming upload: affine writer first, response head on finish.
    let endpoints = Arc::clone(&endpoint_set);
    http.func_wrap(
        "open-upload",
        move |mut store: wasmtime::StoreContextMut<'_, RunState>,
              (method, url, options): (String, String, HostRequestOptions)|
              -> HttpOutcome<Resource<HostRequestBody>> {
            if store.data().http2.abandoned {
                return Ok((Err(HttpError::Limit),));
            }
            if store.data().http2.bodies_live >= MAX_LIVE_BODIES {
                return Ok((Err(HttpError::Limit),));
            }
            let request = match provider_request(method, url, &options) {
                Ok(request) => request,
                Err(error) => return Ok((Err(error),)),
            };
            let cancel: CancelToken = store.data().cancel.clone();
            let probe: Arc<dyn Fn() -> bool + Send + Sync> =
                Arc::new(move || cancel.is_cancelled());
            let engine = Arc::clone(&store.data().http2.engine);
            let endpoints = Arc::clone(&endpoints);
            match engine.open_upload(&request, &endpoints, probe) {
                Ok(pipe) => {
                    store.data_mut().http2.bodies_live += 1;
                    let resource = store
                        .data_mut()
                        .table
                        .push(HostRequestBody { pipe: Some(pipe) })?;
                    Ok((Ok(resource),))
                }
                Err(message) => Ok((Err(classify_provider_error(&message)),)),
            }
        },
    )?;

    // Bounded chunked upload write; backpressure blocks inside the pipe.
    http.func_wrap(
        "[method]request-body.write",
        |mut store: wasmtime::StoreContextMut<'_, RunState>,
         (handle, bytes): (Resource<HostRequestBody>, Vec<u8>)|
         -> HttpOutcome<()> {
            // Cancellation of a blocked write flows through the pipe's own
            // probe, which this Store wired to the run token at open-upload.
            let operation = register_operation(&mut store)?;
            let write_outcome = {
                let body = match store.data_mut().table.get_mut(&handle) {
                    Ok(body) => body,
                    Err(_) => {
                        complete_operation(&mut store, operation, 0)?;
                        return Ok((Err(HttpError::Protocol),));
                    }
                };
                match body.pipe.as_mut() {
                    Some(pipe) => pipe.write(bytes),
                    None => {
                        complete_operation(&mut store, operation, 0)?;
                        return Ok((Err(HttpError::Protocol),));
                    }
                }
            };
            complete_operation(&mut store, operation, 0)?;
            match write_outcome {
                Ok(()) => Ok((Ok(()),)),
                Err(message) => {
                    // A failed write is terminal: consume the pipe so the
                    // worker aborts and the in-flight slot is released.
                    if let Ok(body) = store.data_mut().table.get_mut(&handle) {
                        body.pipe.take();
                    }
                    Ok((Err(classify_provider_error(&message)),))
                }
            }
        },
    )?;

    // Finishing consumes the writer and yields head + affine response. The
    // pipe moves to a worker so the wait observes cancellation.
    http.func_wrap(
        "finish",
        move |mut store: wasmtime::StoreContextMut<'_, RunState>,
              (handle,): (Resource<HostRequestBody>,)|
              -> HttpOutcome<HostStreamingResponse> {
            let cancel = store.data().cancel.clone();
            let operation = register_operation(&mut store)?;
            let pipe = match store.data_mut().table.delete(handle) {
                Ok(mut body) => body.pipe.take(),
                Err(_) => {
                    complete_operation(&mut store, operation, 0)?;
                    return Ok((Err(HttpError::Protocol),));
                }
            };
            let Some(pipe) = pipe else {
                complete_operation(&mut store, operation, 0)?;
                return Ok((Err(HttpError::Protocol),));
            };
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = sender.send(pipe.finish());
            });
            let outcome = wait_worker(&receiver, &cancel, FINISH_WAIT);
            complete_operation(&mut store, operation, 0)?;
            if abandon_on(&outcome) {
                store.data_mut().http2.abandoned = true;
            }
            let response = match outcome {
                Ok(Ok(response)) => response,
                Ok(Err(message)) => return Ok((Err(classify_provider_error(&message)),)),
                Err(http_error) => return Ok((Err(http_error),)),
            };
            let head = head_to_host(&response.head);
            let body = push_body_resource(&mut store, spawn_body_worker(response))?;
            Ok((Ok(HostStreamingResponse { head, body }),))
        },
    )?;
    Ok(())
}

/// Registers one live response body; fails closed at the handle cap.
fn push_body_resource(
    store: &mut wasmtime::StoreContextMut<'_, RunState>,
    body: HostResponseBody,
) -> Result<Resource<HostResponseBody>, wasmtime::Error> {
    {
        let http2 = &mut store.data_mut().http2;
        if http2.bodies_live >= MAX_LIVE_BODIES {
            return Err(wasmtime::Error::msg(
                "resource-limit: too many live http bodies",
            ));
        }
        http2.bodies_live += 1;
    }
    store
        .data_mut()
        .table
        .push(body)
        .map_err(wasmtime::Error::from)
}

/// Waits for one body-worker payload while preserving the provider's
/// typed error text (`recv_cancellable` collapses worker errors to `Io`,
/// which would lose the RFC-0037 classification).
fn recv_body_chunk(
    receiver: &std::sync::mpsc::Receiver<Result<Vec<u8>, String>>,
    cancel: &CancelToken,
) -> Result<Result<Vec<u8>, String>, crate::HostStreamError> {
    loop {
        if cancel.is_cancelled() {
            return Err(crate::HostStreamError::Cancelled);
        }
        match receiver.try_recv() {
            Ok(result) => return Ok(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(crate::HostStreamError::Io);
            }
        }
    }
}

fn register_operation(
    store: &mut wasmtime::StoreContextMut<'_, RunState>,
) -> Result<OperationId, wasmtime::Error> {
    store
        .data_mut()
        .scheduler
        .register_root_operation(ReadinessClass::HostCompletion)
        .map_err(|fault| wasmtime::Error::msg(format!("scheduler: {fault}")))
}

fn complete_operation(
    store: &mut wasmtime::StoreContextMut<'_, RunState>,
    operation: OperationId,
    payload: usize,
) -> Result<(), wasmtime::Error> {
    store
        .data_mut()
        .scheduler
        .complete_root_operation(operation, payload)
        .or_else(|fault| {
            store
                .data_mut()
                .scheduler
                .abandon_operation(operation)
                .map_err(|_| fault)
        })
        .map_err(|fault| wasmtime::Error::msg(format!("scheduler: {fault}")))
}

/// Waits for one worker result, observing cancellation and the request's
/// total deadline instead of blocking behind the OS call.
fn wait_worker<T>(
    receiver: &std::sync::mpsc::Receiver<Result<T, String>>,
    cancel: &CancelToken,
    deadline: Duration,
) -> Result<Result<T, String>, HttpError> {
    let expiry = Instant::now() + deadline;
    loop {
        if cancel.is_cancelled() {
            return Err(HttpError::Cancel);
        }
        if Instant::now() >= expiry {
            return Err(HttpError::Limit);
        }
        match receiver.try_recv() {
            Ok(result) => return Ok(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(HttpError::Io);
            }
        }
    }
}

/// True when a worker outcome means the Store's HTTP surface must fail
/// closed for the remainder of the run (an abandoned worker cannot be
/// synchronously joined).
fn abandon_on<T>(outcome: &Result<Result<T, String>, HttpError>) -> bool {
    match outcome {
        Err(HttpError::Cancel | HttpError::Limit) => true,
        Err(_) => false,
        Ok(Err(message)) => message.starts_with("cancel:") || message.starts_with("timeout:"),
        Ok(Ok(_)) => false,
    }
}
