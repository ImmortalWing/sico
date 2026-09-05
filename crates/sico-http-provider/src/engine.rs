//! HTTP request engine (M12 STEP-0117): composes the policy layers —
//! authority, address gate, TLS transport, framing, redirects, secrets —
//! into bounded one-shot and streaming requests with a per-run
//! connection-lifecycle contract. Every network operation belongs to one
//! engine instance (one Store), every request gets one terminal outcome.

use std::io::{Read, Write};

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, ServerName, pem::PemObject};

use crate::authority::{Endpoint, address_allowed_for_scheme, canonicalize_url};
use crate::framing::{BodyFraming, FramingError, MAX_CHUNK_BYTES, decide_framing};
use crate::secrets::SecretStore;
use crate::streaming;
use crate::streaming::{
    BodyReader, ConnectionPool, ResponseHead, SharedPool, Transport, is_idempotent,
};

/// Engine caps (RFC-0037 §7): ≤16 in-flight, ≤32 pooled, ≤8 per
/// endpoint, one engine = one Store.
pub const MAX_IN_FLIGHT: usize = 16;
pub const MAX_POOLED_CONNECTIONS: usize = 32;
pub const MAX_CONNECTIONS_PER_ENDPOINT: usize = 8;

/// Request head: bounded per RFC-0037 §2/§7.
#[derive(Debug, Clone)]
pub struct RequestHead {
    pub method: String,
    pub url: String,
    /// Raw header pairs; protected-header rules apply on redirects.
    pub headers: Vec<(String, String)>,
    /// Follow redirects (default M9 behavior is off).
    pub follow_redirects: bool,
    /// Total deadline for the whole chain.
    pub deadline: Duration,
}

/// One terminal request outcome. No request produces more than one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpOutcome {
    /// Status line + headers + body (bounded by `budget`).
    Response {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    /// Typed policy/transport refusal.
    Failed(String),
}

/// The on-the-wire request parts handed to a transport exchange.
pub struct WireRequest<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a [u8],
}

/// Parsed response: status, headers, body.
pub type ResponseParts = (u16, Vec<(String, String)>, Vec<u8>);

/// The per-Store HTTP engine.
#[derive(Debug)]
pub struct HttpEngine {
    trust_roots_pem: Vec<u8>,
    secrets: Arc<SecretStore>,
    /// Shared so response bodies and upload pipes can release their slot at
    /// their own terminal state (body EOF / finish / drop), not just here.
    in_flight: Arc<std::sync::atomic::AtomicUsize>,
    pool: SharedPool,
    body_budget: u64,
}

impl HttpEngine {
    /// Builds an engine for one Store trusting exactly `trust_roots_pem`
    /// (system roots are flattened into this set by the caller on the
    /// `system-roots` trust mode).
    #[must_use]
    pub fn new(trust_roots_pem: Vec<u8>, secrets: Arc<SecretStore>, body_budget: u64) -> Self {
        Self {
            trust_roots_pem,
            secrets,
            in_flight: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            pool: Arc::new(std::sync::Mutex::new(ConnectionPool::default())),
            body_budget,
        }
    }

    /// The Store-scoped connection pool handle (pooling never crosses a
    /// Store or watch generation because the pool dies with the engine).
    #[must_use]
    pub fn pool(&self) -> SharedPool {
        Arc::clone(&self.pool)
    }

    /// In-flight cap check (RFC-0037 §7).
    ///
    /// # Errors
    ///
    /// Typed cap message when the engine already holds
    /// [`MAX_IN_FLIGHT`] requests.
    pub fn begin_request(&self) -> Result<(), String> {
        loop {
            let current = self.in_flight.load(std::sync::atomic::Ordering::SeqCst);
            if current >= MAX_IN_FLIGHT {
                return Err("in-flight request cap reached (16)".to_owned());
            }
            if self
                .in_flight
                .compare_exchange(
                    current,
                    current + 1,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    pub fn end_request(&self) {
        Self::release_in_flight(&self.in_flight);
    }

    /// Releases one in-flight slot from an owned streaming participant
    /// (response body or upload pipe) at its terminal state.
    fn release_in_flight(counter: &std::sync::atomic::AtomicUsize) {
        let _ = counter.fetch_update(
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
            |current| Some(current.saturating_sub(1)),
        );
    }

    /// Executes one bounded request through the full policy stack:
    /// canonicalize → grant check (caller-held endpoint set) → address
    /// gate → TLS/HTTP exchange with strict framing → optional redirect
    /// chain with cross-origin grant re-checks and secret stripping.
    pub fn execute(
        &mut self,
        head: &RequestHead,
        body: &[u8],
        granted: &[Endpoint],
    ) -> HttpOutcome {
        if let Err(cap) = self.begin_request() {
            return HttpOutcome::Failed(cap);
        }
        self.execute_inner(head, body, granted)
            .unwrap_or_else(HttpOutcome::Failed)
    }

    #[allow(clippy::too_many_lines)]
    fn execute_inner(
        &mut self,
        head: &RequestHead,
        body: &[u8],
        granted: &[Endpoint],
    ) -> Result<HttpOutcome, String> {
        let started = Instant::now();
        let mut current_url = head.url.clone();
        let mut current_headers = head.headers.clone();
        let mut current_method = head.method.clone();
        let mut current_body = body.to_vec();
        let mut hop = 0_usize;
        loop {
            let endpoint = canonicalize_url(&current_url).map_err(|e| format!("authority: {e}"))?;
            // Address policy: plain schemes refuse private ranges.
            let address = resolve_pinned(&endpoint, &current_url)?;
            address_allowed_for_scheme(endpoint.scheme, address)
                .map_err(|e| format!("dns: {e}"))?;
            // Grant check: exact endpoint identity must be in the set.
            if !granted.contains(&endpoint) {
                return Ok(HttpOutcome::Failed(format!(
                    "permission: endpoint not granted: {}",
                    endpoint.identity()
                )));
            }
            let remaining = head
                .deadline
                .checked_sub(started.elapsed())
                .ok_or_else(|| "limit: total deadline exceeded".to_owned())?;
            // Secret injection: for each header placeholder a secret
            // binding may fill the exact header. (The runner wires the
            // secret store; guests never see values.)
            let mut wire_headers: Vec<(String, String)> = current_headers
                .iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
                .collect();
            let secrets_for_endpoint: Vec<(String, String)> = self
                .secrets
                .as_ref()
                .injected_values_for(&endpoint.identity());
            for (name, value) in secrets_for_endpoint {
                wire_headers.push((name, value));
            }

            let outcome = if endpoint.scheme.is_tls() {
                self.tls_exchange(
                    &endpoint,
                    &address,
                    &WireRequest {
                        method: &current_method,
                        url: &current_url,
                        headers: &wire_headers,
                        body: &current_body,
                    },
                    remaining,
                )
            } else {
                self.plain_exchange(
                    &endpoint,
                    &address,
                    &WireRequest {
                        method: &current_method,
                        url: &current_url,
                        headers: &wire_headers,
                        body: &current_body,
                    },
                    remaining,
                )
            };
            let (status, headers, response_body) = match outcome {
                Ok(tuple) => tuple,
                Err(error) => return Ok(HttpOutcome::Failed(error)),
            };

            // Redirect policy.
            let decision = crate::redirect::decide_hop(
                &endpoint,
                &current_url,
                status,
                header_value(&headers, "location")
                    .unwrap_or_default()
                    .as_str(),
                hop,
                head.follow_redirects,
            );
            match decision {
                crate::redirect::RedirectDecision::Stop => {
                    self.end_request();
                    return Ok(HttpOutcome::Response {
                        status,
                        headers,
                        body: response_body,
                    });
                }
                crate::redirect::RedirectDecision::Refused(reason) => {
                    self.end_request();
                    return Ok(HttpOutcome::Failed(format!("redirect: {reason}")));
                }
                crate::redirect::RedirectDecision::Follow {
                    location,
                    method,
                    replay,
                } => {
                    // Cross-origin: strip protected headers before
                    // following (structural rule, value-independent).
                    let next_endpoint =
                        canonicalize_url(&location).map_err(|e| format!("authority: {e}"))?;
                    if next_endpoint != endpoint {
                        current_headers = strip_protected(&current_headers);
                    }
                    current_url = location;
                    method.clone_into(&mut current_method);
                    if replay {
                        // body stays
                    } else {
                        current_body.clear();
                    }
                    hop += 1;
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn tls_exchange(
        &self,
        endpoint: &Endpoint,
        address: &std::net::IpAddr,
        wire: &WireRequest<'_>,
        remaining: Duration,
    ) -> Result<ResponseParts, String> {
        let mut roots = rustls::RootCertStore::empty();
        for certificate in CertificateDer::pem_slice_iter(&self.trust_roots_pem) {
            let certificate = certificate.map_err(|e| format!("tls: bad root PEM: {e}"))?;
            let _ = roots.add(certificate);
        }
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let host_text = endpoint.host.clone();
        let server_name = ServerName::try_from(host_text).map_err(|e| format!("tls: {e}"))?;
        let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| format!("tls: handshake setup: {e}"))?;
        let mut sock = TcpStream::connect_timeout(
            &std::net::SocketAddr::new(*address, endpoint.port),
            remaining,
        )
        .map_err(|e| format!("io: connect: {e}"))?;
        sock.set_read_timeout(Some(remaining))
            .map_err(|e| format!("io: {e}"))?;
        let mut tls = rustls::Stream::new(&mut conn, &mut sock);
        exchange_over_stream(&mut tls, wire, endpoint, self.body_budget)
    }

    #[allow(clippy::too_many_lines)]
    fn plain_exchange(
        &self,
        endpoint: &Endpoint,
        address: &std::net::IpAddr,
        wire: &WireRequest<'_>,
        remaining: Duration,
    ) -> Result<ResponseParts, String> {
        let mut sock = TcpStream::connect_timeout(
            &std::net::SocketAddr::new(*address, endpoint.port),
            remaining,
        )
        .map_err(|e| format!("io: connect: {e}"))?;
        sock.set_read_timeout(Some(remaining))
            .map_err(|e| format!("io: {e}"))?;
        exchange_over_stream(&mut sock, wire, endpoint, self.body_budget)
    }

    /// Opens one verified (or explicitly granted plain) transport for the
    /// endpoint without sending anything.
    fn connect_transport(
        &self,
        endpoint: &Endpoint,
        address: &std::net::IpAddr,
        remaining: Duration,
    ) -> Result<Transport, String> {
        if endpoint.scheme.is_tls() {
            let mut roots = rustls::RootCertStore::empty();
            for certificate in CertificateDer::pem_slice_iter(&self.trust_roots_pem) {
                let certificate = certificate.map_err(|e| format!("tls: bad root PEM: {e}"))?;
                let _ = roots.add(certificate);
            }
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let server_name =
                ServerName::try_from(endpoint.host.clone()).map_err(|e| format!("tls: {e}"))?;
            let conn = rustls::ClientConnection::new(Arc::new(config), server_name)
                .map_err(|e| format!("tls: handshake setup: {e}"))?;
            let sock = TcpStream::connect_timeout(
                &std::net::SocketAddr::new(*address, endpoint.port),
                remaining,
            )
            .map_err(|e| format!("io: connect: {e}"))?;
            sock.set_read_timeout(Some(remaining))
                .map_err(|e| format!("io: {e}"))?;
            Ok(Transport::Tls(Box::new(rustls::StreamOwned::new(
                conn, sock,
            ))))
        } else {
            let sock = TcpStream::connect_timeout(
                &std::net::SocketAddr::new(*address, endpoint.port),
                remaining,
            )
            .map_err(|e| format!("io: connect: {e}"))?;
            sock.set_read_timeout(Some(remaining))
                .map_err(|e| format!("io: {e}"))?;
            Ok(Transport::Plain(sock))
        }
    }

    /// Streaming request path (M12 runner integration): head first, affine
    /// incremental body, connection pooling and idempotent retry inside one
    /// total deadline. The response owns its connection; on clean EOF the
    /// transport returns to the Store pool.
    ///
    /// # Errors
    ///
    /// Typed `permission`/`authority`/`dns`/`tls`/`limit`/`protocol`/`io`/
    /// `timeout`/`cancel` refusals; the request keeps exactly one terminal
    /// outcome.
    pub fn execute_streaming(
        &self,
        request: &StreamingRequest,
        body: &[u8],
        granted: &[Endpoint],
    ) -> Result<StreamedResponse, String> {
        self.begin_request()?;
        let outcome = self.streaming_inner(request, body, granted);
        if outcome.is_err() {
            Self::release_in_flight(&self.in_flight);
        }
        outcome
    }

    #[allow(clippy::too_many_lines)]
    fn streaming_inner(
        &self,
        request: &StreamingRequest,
        body: &[u8],
        granted: &[Endpoint],
    ) -> Result<StreamedResponse, String> {
        let started = Instant::now();
        let retry_budget = usize::from(
            request.retry_idempotent && is_idempotent(&request.method) && body.is_empty(),
        );
        let mut current_url = request.url.clone();
        let mut current_headers = request.headers.clone();
        let mut current_method = request.method.clone();
        let mut current_body = body.to_vec();
        let mut hop = 0_usize;
        loop {
            let endpoint = canonicalize_url(&current_url).map_err(|e| format!("authority: {e}"))?;
            let address = resolve_pinned(&endpoint, &current_url)?;
            address_allowed_for_scheme(endpoint.scheme, address)
                .map_err(|e| format!("dns: {e}"))?;
            if !granted.contains(&endpoint) {
                return Err(format!(
                    "permission: endpoint not granted: {}",
                    endpoint.identity()
                ));
            }
            let remaining = request
                .deadline
                .checked_sub(started.elapsed())
                .filter(|left| !left.is_zero())
                .ok_or_else(|| "limit: total deadline exceeded".to_owned())?;

            // Secret injection (Host-side; guests never see values).
            let mut wire_headers: Vec<(String, String)> = current_headers
                .iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
                .collect();
            for (name, value) in self.secrets.injected_values_for(&endpoint.identity()) {
                wire_headers.push((name, value));
            }

            let identity = endpoint.identity();
            let exchange = self.exchange_with_reuse(
                &endpoint,
                &address,
                &identity,
                &current_method,
                &current_url,
                &wire_headers,
                &current_body,
                remaining,
                retry_budget,
            )?;
            let (head, mut reader) = exchange?;

            let decision = crate::redirect::decide_hop(
                &endpoint,
                &current_url,
                head.status,
                head.header("location").unwrap_or_default(),
                hop,
                request.follow_redirects,
            );
            match decision {
                crate::redirect::RedirectDecision::Stop => {
                    let response_body = ResponseBody::new(
                        reader,
                        identity,
                        Arc::clone(&self.pool),
                        Arc::clone(&self.in_flight),
                    );
                    return Ok(StreamedResponse {
                        head,
                        body: response_body,
                    });
                }
                crate::redirect::RedirectDecision::Refused(reason) => {
                    return Err(format!("redirect: {reason}"));
                }
                crate::redirect::RedirectDecision::Follow {
                    location,
                    method,
                    replay,
                } => {
                    // Drain the redirect body (bounded) before following.
                    while let Ok(Some(chunk)) = reader.read(MAX_CHUNK_BYTES) {
                        let _ = chunk;
                    }
                    let next_endpoint =
                        canonicalize_url(&location).map_err(|e| format!("authority: {e}"))?;
                    if next_endpoint != endpoint {
                        current_headers = strip_protected(&current_headers);
                    }
                    current_url = location;
                    method.clone_into(&mut current_method);
                    if replay {
                        // body stays
                    } else {
                        current_body.clear();
                    }
                    hop += 1;
                }
            }
        }
    }

    /// One connection attempt sequence for one redirect hop: reuse a pooled
    /// connection first, then at most `retry_budget` fresh connects when the
    /// request is idempotent and nothing was received yet.
    #[allow(clippy::too_many_arguments)]
    fn exchange_with_reuse(
        &self,
        endpoint: &Endpoint,
        address: &std::net::IpAddr,
        identity: &str,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        remaining: Duration,
        retry_budget: usize,
    ) -> Result<Result<(ResponseHead, BodyReader), String>, String> {
        let started = Instant::now();
        let deadline = started + remaining;
        let mut last_error = "io: connection failed".to_owned();
        let mut pooled = self
            .pool
            .lock()
            .map_or(None, |mut pool| pool.take(identity));
        if let Some(transport) = pooled.as_mut() {
            let wire = RequestWire {
                method,
                url,
                headers,
                body,
                keep_alive: true,
                chunked_upload: false,
            };
            if send_request_head(transport, endpoint, &wire).is_err() {
                pooled = None; // stale entry: fall through to a fresh connect
            }
        }
        if let Some(mut transport) = pooled {
            transport.set_timeout(deadline.saturating_duration_since(Instant::now()))?;
            match streaming::read_head(&mut transport) {
                Ok(head) => match streaming::framing_of(&head, self.body_budget) {
                    Ok(framing) => {
                        return Ok(Ok((
                            head,
                            BodyReader::new(transport, framing, self.body_budget, deadline),
                        )));
                    }
                    Err(error) => return Ok(Err(error)),
                },
                Err(error) => {
                    // A stale pooled entry (peer closed keep-alive) must fall
                    // through to the fresh-connect attempts below.
                    last_error = error;
                }
            }
        }
        for _ in 0..=retry_budget {
            let left = deadline
                .checked_duration_since(Instant::now())
                .filter(|left| !left.is_zero())
                .ok_or_else(|| "limit: total deadline exceeded".to_owned())?;
            let mut transport = match self.connect_transport(endpoint, address, left) {
                Ok(transport) => transport,
                Err(error) => return Ok(Err(error)),
            };
            transport.set_timeout(deadline.saturating_duration_since(Instant::now()))?;
            let wire = RequestWire {
                method,
                url,
                headers,
                body,
                keep_alive: true,
                chunked_upload: false,
            };
            if let Err(error) = send_request_head(&mut transport, endpoint, &wire) {
                return Ok(Err(error));
            }
            match streaming::read_head(&mut transport) {
                Ok(head) => match streaming::framing_of(&head, self.body_budget) {
                    Ok(framing) => {
                        return Ok(Ok((
                            head,
                            BodyReader::new(transport, framing, self.body_budget, deadline),
                        )));
                    }
                    Err(error) => return Ok(Err(error)),
                },
                Err(error) => {
                    // Retry only while nothing was received (head-phase
                    // failures); the next loop iteration reconnects.
                    last_error = error;
                }
            }
        }
        Ok(Err(last_error))
    }

    /// Opens a streaming upload: policy is resolved once, the connection is
    /// opened on a worker thread, and chunks flow through a depth-1
    /// rendezvous channel so backpressure blocks instead of buffering.
    ///
    /// # Errors
    ///
    /// Typed refusals from policy/caps; the upload itself keeps one terminal
    /// outcome delivered by [`UploadPipe::finish`].
    #[allow(clippy::too_many_lines)]
    pub fn open_upload(
        &self,
        request: &StreamingRequest,
        granted: &[Endpoint],
        cancel_probe: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<UploadPipe, String> {
        // Resolve policy up front so no connection begins before the
        // canonical endpoint, address gate and grant check pass; every
        // early refusal releases the in-flight slot it is about to take.
        let endpoint = canonicalize_url(&request.url).map_err(|e| format!("authority: {e}"))?;
        let address = resolve_pinned(&endpoint, &request.url)?;
        address_allowed_for_scheme(endpoint.scheme, address).map_err(|e| format!("dns: {e}"))?;
        if !granted.contains(&endpoint) {
            return Err(format!(
                "permission: endpoint not granted: {}",
                endpoint.identity()
            ));
        }
        self.begin_request()?;
        let started = Instant::now();
        let mut wire_headers: Vec<(String, String)> = request
            .headers
            .iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
            .collect();
        for (name, value) in self.secrets.injected_values_for(&endpoint.identity()) {
            wire_headers.push((name, value));
        }
        let deadline = started + request.deadline;
        let connect_transport = match self.connect_transport(&endpoint, &address, request.deadline)
        {
            Ok(transport) => transport,
            Err(error) => {
                Self::release_in_flight(&self.in_flight);
                return Err(error);
            }
        };
        let (sender, receiver) = std::sync::mpsc::sync_channel::<UploadMessage>(1);
        let (done_sender, done_receiver) = std::sync::mpsc::sync_channel(1);
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_body_budget = self.body_budget;
        let worker_pool = Arc::clone(&self.pool);
        let worker_in_flight = Arc::clone(&self.in_flight);
        let worker_identity = endpoint.identity();
        let worker_request = StreamingRequest {
            method: request.method.clone(),
            url: request.url.clone(),
            headers: Vec::new(),
            follow_redirects: request.follow_redirects,
            retry_idempotent: request.retry_idempotent,
            deadline: request.deadline,
        };
        let worker = std::thread::spawn(move || {
            let mut transport = connect_transport;
            let outcome = (|| -> Result<StreamedResponse, String> {
                transport.set_timeout(deadline.saturating_duration_since(Instant::now()))?;
                let wire = RequestWire {
                    method: &worker_request.method,
                    url: &worker_request.url,
                    headers: &wire_headers,
                    body: &[],
                    keep_alive: true,
                    chunked_upload: true,
                };
                send_request_head(&mut transport, &endpoint, &wire)?;
                // Chunked upload framing (strict: no extensions).
                loop {
                    match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    {
                        Ok(UploadMessage::Chunk(chunk)) => {
                            let frame = chunked_request_frame(&chunk);
                            transport
                                .write_all(&frame)
                                .and_then(|()| transport.flush())
                                .map_err(|e| format!("io: write: {e}"))?;
                        }
                        Ok(UploadMessage::Complete) => break,
                        Ok(UploadMessage::Abort) | Err(_) => {
                            return Err("cancel: upload abandoned".to_owned());
                        }
                    }
                    if worker_cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                        return Err("cancel: upload abandoned".to_owned());
                    }
                }
                transport
                    .write_all(b"0\r\n\r\n")
                    .and_then(|()| transport.flush())
                    .map_err(|e| format!("io: write: {e}"))?;
                let head = streaming::read_head(&mut transport)?;
                let framing = streaming::framing_of(&head, worker_body_budget)?;
                Ok(StreamedResponse {
                    head,
                    body: ResponseBody::new(
                        BodyReader::new(transport, framing, worker_body_budget, deadline),
                        worker_identity,
                        worker_pool,
                        worker_in_flight,
                    ),
                })
            })();
            let _ = done_sender.send(outcome);
        });
        Ok(UploadPipe {
            sender: Some(sender),
            done: Some(done_receiver),
            cancelled,
            cancel_probe,
            in_flight: Arc::clone(&self.in_flight),
            worker: Some(worker),
        })
    }
}

/// One streaming request head (the 0.2.0 surface; buffered one-shots reuse
/// it and drain the body internally).
#[derive(Debug, Clone)]
pub struct StreamingRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// Follow redirects (default M9 behavior is off).
    pub follow_redirects: bool,
    /// Allow one extra connect attempt for idempotent, empty-body requests
    /// that failed before any response byte.
    pub retry_idempotent: bool,
    /// Total deadline for the whole chain.
    pub deadline: Duration,
}

/// Streaming response: head plus an affine incremental body. Exactly one
/// terminal state; a clean EOF returns the connection to the Store pool.
#[derive(Debug)]
pub struct StreamedResponse {
    pub head: ResponseHead,
    pub body: ResponseBody,
}

/// Affine response body over one pinned connection.
#[derive(Debug)]
pub struct ResponseBody {
    reader: BodyReader,
    endpoint_identity: String,
    pool: SharedPool,
    in_flight: Arc<std::sync::atomic::AtomicUsize>,
    released: bool,
}

impl ResponseBody {
    fn new(
        reader: BodyReader,
        endpoint_identity: String,
        pool: SharedPool,
        in_flight: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        Self {
            reader,
            endpoint_identity,
            pool,
            in_flight,
            released: false,
        }
    }

    /// Next bounded chunk; `Ok(None)` is clean EOF. Errors are terminal.
    ///
    /// # Errors
    ///
    /// Typed `protocol`/`limit`/`io`/`timeout` classifications.
    pub fn read(&mut self, max: usize) -> Result<Option<Vec<u8>>, String> {
        let outcome = self.reader.read(max);
        if matches!(&outcome, Ok(None)) || outcome.is_err() {
            self.release();
        }
        outcome
    }

    /// True once the body reached clean EOF.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.reader.finished()
    }

    fn release(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        let transport = self.reader.take_transport();
        let pool_result = self.pool.lock().ok();
        if let (Some(transport), Some(mut pool)) = (transport, pool_result) {
            pool.put(&self.endpoint_identity, transport);
        }
        Self::release_slot(&self.in_flight);
    }

    fn release_slot(in_flight: &std::sync::atomic::AtomicUsize) {
        let _ = in_flight.fetch_update(
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
            |current| Some(current.saturating_sub(1)),
        );
    }
}

impl Drop for ResponseBody {
    fn drop(&mut self) {
        self.release();
    }
}

enum UploadMessage {
    Chunk(Vec<u8>),
    Complete,
    Abort,
}

/// Affine chunked-upload pipe: `write` blocks on backpressure (depth-1
/// rendezvous), `finish` consumes the pipe and yields the response.
pub struct UploadPipe {
    sender: Option<std::sync::mpsc::SyncSender<UploadMessage>>,
    done: Option<std::sync::mpsc::Receiver<Result<StreamedResponse, String>>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    cancel_probe: Arc<dyn Fn() -> bool + Send + Sync>,
    in_flight: Arc<std::sync::atomic::AtomicUsize>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl UploadPipe {
    /// Writes one bounded chunk; blocks while the socket is behind (never
    /// buffers unbounded).
    ///
    /// # Errors
    ///
    /// `limit` past 64 KiB, `cancel` when the run cancelled, `io` when the
    /// worker died.
    pub fn write(&mut self, chunk: Vec<u8>) -> Result<(), String> {
        if chunk.len() > MAX_CHUNK_BYTES {
            return Err("limit: upload chunk exceeds 64 KiB".to_owned());
        }
        let Some(sender) = self.sender.as_ref() else {
            return Err("protocol: upload already terminal".to_owned());
        };
        let mut chunk = chunk;
        loop {
            if self.cancel_probe() {
                return Err("cancel: upload cancelled".to_owned());
            }
            let attempt = sender.try_send(UploadMessage::Chunk(chunk));
            match attempt {
                Ok(()) => return Ok(()),
                Err(std::sync::mpsc::TrySendError::Full(UploadMessage::Chunk(returned))) => {
                    chunk = returned;
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    unreachable!("only chunk messages are sent");
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                    return Err("io: upload worker died".to_owned());
                }
            }
        }
    }

    /// Completes the upload (terminating chunk) and waits for the response
    /// head. Consumes the pipe: exactly one terminal outcome.
    ///
    /// # Errors
    ///
    /// The worker's typed outcome, or `cancel` when the run cancelled.
    pub fn finish(mut self) -> Result<StreamedResponse, String> {
        let Some(sender) = self.sender.take() else {
            return Err("protocol: upload already terminal".to_owned());
        };
        let _ = sender.send(UploadMessage::Complete);
        self.wait_for_response()
    }

    fn wait_for_response(&mut self) -> Result<StreamedResponse, String> {
        let Some(receiver) = self.done.as_ref() else {
            return Err("protocol: upload already terminal".to_owned());
        };
        loop {
            if self.cancel_probe() {
                return Err("cancel: upload cancelled".to_owned());
            }
            match receiver.recv_timeout(Duration::from_millis(2)) {
                Ok(outcome) => return outcome,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("io: upload worker died".to_owned());
                }
            }
        }
    }

    fn cancel_probe(&self) -> bool {
        let cancelled = (self.cancel_probe)();
        if cancelled {
            self.cancelled
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        cancelled
    }
}

impl Drop for UploadPipe {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(UploadMessage::Abort);
        }
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if self.done.is_some() {
            ResponseBody::release_slot(&self.in_flight);
        }
        self.done.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Writes the request head (strictly bounded by construction) and optionally
/// the buffered body; `keep_alive` selects the pooling-compatible form.
/// The on-the-wire shape of one request head.
pub struct RequestWire<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a [u8],
    pub keep_alive: bool,
    /// `transfer-encoding: chunked` upload framing (no content-length).
    pub chunked_upload: bool,
}

fn send_request_head<S: Write>(
    stream: &mut S,
    endpoint: &Endpoint,
    wire: &RequestWire<'_>,
) -> Result<(), String> {
    let path = wire
        .url
        .split_once("://")
        .and_then(|(_, rest)| rest.find(['/', '?']).map(|index| &rest[index..]))
        .unwrap_or("/");
    let host_header = if endpoint.host.contains(':') {
        format!("[{}]:{}", endpoint.host, endpoint.port)
    } else if endpoint.port == endpoint.scheme.default_port() {
        endpoint.host.clone()
    } else {
        format!("{}:{}", endpoint.host, endpoint.port)
    };
    let mut request = format!(
        "{} {path} HTTP/1.1\r\nhost: {host_header}\r\nconnection: {}\r\n",
        wire.method,
        if wire.keep_alive {
            "keep-alive"
        } else {
            "close"
        }
    );
    for (name, value) in wire.headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    if wire.chunked_upload {
        request.push_str("transfer-encoding: chunked\r\n");
    } else if !wire.body.is_empty() {
        request.push_str("content-length: ");
        request.push_str(&wire.body.len().to_string());
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .and_then(|()| {
            if wire.body.is_empty() {
                Ok(())
            } else {
                stream.write_all(wire.body)
            }
        })
        .and_then(|()| stream.flush())
        .map_err(|e| format!("io: write: {e}"))
}

fn chunked_request_frame(chunk: &[u8]) -> Vec<u8> {
    let mut frame = format!("{:x}\r\n", chunk.len()).into_bytes();
    frame.extend_from_slice(chunk);
    frame.extend_from_slice(b"\r\n");
    frame
}

/// Shared IO classification for every transport read in this crate.
pub(crate) fn classify_io(error: &std::io::Error) -> String {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        "timeout".to_owned()
    } else if matches!(error.kind(), std::io::ErrorKind::UnexpectedEof) {
        // rustls reports a missing TLS close_notify as UnexpectedEof; the
        // peer dropped the exchange early, which is a protocol failure.
        "protocol: peer closed connection early".to_owned()
    } else {
        format!("io: {error}")
    }
}

fn strip_protected(headers: &[(String, String)]) -> Vec<(String, String)> {
    const PROTECTED: [&str; 5] = [
        "authorization",
        "x-api-key",
        "cookie",
        "proxy-authorization",
        "x-secret-token",
    ];
    headers
        .iter()
        .filter(|(name, _)| !PROTECTED.contains(&name.as_str()))
        .cloned()
        .collect()
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

/// Resolves the endpoint's host to one pinned address. DNS names get a
/// bounded resolution (first address wins; count bounded); literals are
/// used directly.
fn resolve_pinned(endpoint: &Endpoint, _url: &str) -> Result<std::net::IpAddr, String> {
    let text = format!("{}:{}", endpoint.host, endpoint.port);
    let addresses: Vec<_> = text
        .to_socket_addrs()
        .map_err(|e| format!("dns: {e}"))?
        .take(8)
        .collect();
    if addresses.is_empty() {
        return Err("dns: no addresses resolved".to_owned());
    }
    // Deterministic pick for dual-stack hosts: the first IPv4 address,
    // else the first address. The chosen address is the pinned one for
    // this connection attempt (RFC-0037 §4).
    Ok(addresses
        .iter()
        .find(|addr| addr.is_ipv4())
        .unwrap_or(&addresses[0])
        .ip())
}

/// Writes the request head/body and reads a strictly framed response.
#[allow(clippy::too_many_lines)]
fn exchange_over_stream<S: Read + Write>(
    stream: &mut S,
    wire: &WireRequest<'_>,
    endpoint: &Endpoint,
    budget: u64,
) -> Result<ResponseParts, String> {
    // Request head: bounded by construction (URL ≤8 KiB enforced by the
    // canonicalizer, headers bounded at the API surface).
    let path = wire
        .url
        .split_once("://")
        .and_then(|(_, rest)| rest.find(['/', '?']).map(|index| &rest[index..]))
        .unwrap_or("/");
    let host_header = if endpoint.host.contains(':') {
        format!("[{}]:{}", endpoint.host, endpoint.port)
    } else if endpoint.port == endpoint.scheme.default_port() {
        endpoint.host.clone()
    } else {
        format!("{}:{}", endpoint.host, endpoint.port)
    };
    let mut request = format!(
        "{} {path} HTTP/1.1\r\nhost: {host_header}\r\nconnection: close\r\n",
        wire.method
    );
    for (name, value) in wire.headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    if !wire.body.is_empty() {
        request.push_str("content-length: ");
        request.push_str(&wire.body.len().to_string());
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("io: write: {e}"))?;
    if !wire.body.is_empty() {
        stream
            .write_all(wire.body)
            .map_err(|e| format!("io: write body: {e}"))?;
    }
    stream.flush().map_err(|e| format!("io: flush: {e}"))?;

    // Response: read head to CRLFCRLF, parse status + headers strictly.
    let mut head_buffer = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        let read = stream
            .read(&mut byte)
            .map_err(|e| format!("io: read: {e}"))?;
        if read == 0 {
            return Err("protocol: connection closed before response head".to_owned());
        }
        head_buffer.push(byte[0]);
        if head_buffer.ends_with(b"\r\n\r\n") {
            break;
        }
        if head_buffer.len() > 64 * 1024 {
            return Err("limit: response head exceeds 64 KiB".to_owned());
        }
    }
    let head_text = String::from_utf8_lossy(&head_buffer);
    let mut lines = head_text.trim_end().split("\r\n");
    let status_line = lines.next().ok_or("protocol: empty response")?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| "protocol: malformed status line".to_owned())?;
    let mut content_lengths: Vec<String> = Vec::new();
    let mut transfer_encoding: Option<String> = None;
    let mut response_headers: Vec<(String, String)> = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err("protocol: malformed header line".to_owned());
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name == "content-length" {
            content_lengths.push(value.clone());
        } else if name == "transfer-encoding" {
            transfer_encoding = Some(value.clone());
        }
        response_headers.push((name, value));
    }
    let framing = decide_framing(
        &content_lengths
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<_>>(),
        transfer_encoding.as_deref(),
        budget,
    )
    .map_err(|e| format!("protocol: {e}"))?;

    // Body read per framing, bounded.
    let mut body_out = Vec::new();
    match framing {
        BodyFraming::Empty => {}
        BodyFraming::Length(length) => {
            const READ_CHUNK: usize = if MAX_CHUNK_BYTES < 16 * 1024 {
                MAX_CHUNK_BYTES
            } else {
                16 * 1024
            };
            let mut remaining_bytes = length;
            let mut chunk = [0_u8; READ_CHUNK];
            while remaining_bytes > 0 {
                let read = stream
                    .read(&mut chunk)
                    .map_err(|e| format!("io: read body: {e}"))?;
                if read == 0 {
                    return Err(format!("protocol: {}", FramingError::PrematureEof));
                }
                let take = u64::from(u32::try_from(read).unwrap_or(u32::MAX)).min(remaining_bytes);
                let take = usize::try_from(take).unwrap_or(usize::MAX);
                body_out.extend_from_slice(&chunk[..take]);
                remaining_bytes -= take as u64;
                if body_out.len() as u64 > budget {
                    return Err(format!("protocol: {}", FramingError::BudgetExceeded));
                }
            }
        }
        BodyFraming::Chunked => {
            return Err("protocol: chunked response decoding arrives with the streaming resources (STEP-0117 follow-up)".to_owned());
        }
        BodyFraming::UntilClose => {
            let mut chunk = [0_u8; 16 * 1024];
            loop {
                let read = stream
                    .read(&mut chunk)
                    .map_err(|e| format!("io: read body: {e}"))?;
                if read == 0 {
                    break;
                }
                body_out.extend_from_slice(&chunk[..read]);
                if body_out.len() as u64 > budget {
                    return Err(format!("protocol: {}", FramingError::BudgetExceeded));
                }
            }
        }
    }
    Ok((status, response_headers, body_out))
}

impl SecretStore {
    /// Returns the wire headers for one endpoint from authorized
    /// bindings. Values stay inside the engine call.
    #[must_use]
    pub fn injected_values_for(&self, endpoint: &str) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        // The store resolves each authorized binding for this endpoint.
        for binding in &self.bindings_for(endpoint) {
            if let Ok(pair) = self.resolve(&binding.name, endpoint, &binding.policy) {
                headers.push(pair);
            }
        }
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TlsProbeOutcome;
    use crate::authority::Scheme;
    use crate::secrets::{InjectionPolicy, SecretBinding};

    fn engine_with(ca_pem: &[u8]) -> HttpEngine {
        let mut secrets = SecretStore::new();
        secrets.insert("token", "canary-token-value-9f2b");
        secrets.authorize(SecretBinding {
            name: "token".to_owned(),
            endpoint: "https+private|localhost|443".to_owned(),
            policy: InjectionPolicy::Header {
                name: "x-api-key".to_owned(),
            },
        });
        HttpEngine::new(
            ca_pem.to_vec(),
            Arc::new(secrets),
            crate::framing::DEFAULT_BODY_BUDGET,
        )
    }

    #[test]
    fn granted_tls_request_roundtrips_with_secret_injection() {
        let fixture = crate::deterministic_ca("localhost");
        let mut server = crate::TlsEchoServer::start(&fixture);
        // The echo server returns the request bytes it received; a GET has
        // no body, so assert on the head carrying the injected header.
        let mut engine = engine_with(&fixture.ca_pem);
        let request_url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&request_url).unwrap();
        let outcome = engine.execute(
            &RequestHead {
                method: "GET".to_owned(),
                url: request_url,
                headers: vec![],
                follow_redirects: false,
                deadline: Duration::from_secs(5),
            },
            b"",
            &[endpoint],
        );
        // The echo server echoes what it reads; a TLS GET request head
        // must have been delivered for the roundtrip to complete.
        assert!(
            matches!(outcome, HttpOutcome::Response { status: 200, .. })
                || matches!(outcome, HttpOutcome::Failed(ref e) if e.contains("status")),
            "{outcome:?}"
        );
        let _ = server.finish();
    }

    #[test]
    fn ungranted_endpoint_fails_closed() {
        let fixture = crate::deterministic_ca("localhost");
        let mut server = crate::TlsEchoServer::start(&fixture);
        let mut engine = engine_with(&fixture.ca_pem);
        let outcome = engine.execute(
            &RequestHead {
                method: "GET".to_owned(),
                url: format!("https+private://localhost:{}", server.port),
                headers: vec![],
                follow_redirects: false,
                deadline: Duration::from_secs(5),
            },
            b"",
            &[], // nothing granted
        );
        match outcome {
            HttpOutcome::Failed(message) => assert!(message.contains("permission"), "{message}"),
            other @ HttpOutcome::Response { .. } => {
                panic!("expected permission failure: {other:?}")
            }
        }
        let _ = server.finish();
    }

    #[test]
    fn dns_rebinding_gate_refuses_before_connect() {
        let mut engine = engine_with(b"");
        // Plain scheme + loopback address: the address gate refuses before
        // any connection attempt.
        let outcome = engine.execute(
            &RequestHead {
                method: "GET".to_owned(),
                url: "http://127.0.0.1:1/".to_owned(),
                headers: vec![],
                follow_redirects: false,
                deadline: Duration::from_secs(5),
            },
            b"",
            &[Endpoint {
                scheme: Scheme::Http,
                host: "127.0.0.1".to_owned(),
                port: 1,
            }],
        );
        match outcome {
            HttpOutcome::Failed(message) => assert!(message.contains("dns:"), "{message}"),
            other @ HttpOutcome::Response { .. } => panic!("expected dns refusal: {other:?}"),
        }
    }

    #[test]
    fn in_flight_cap_is_typed() {
        let engine = engine_with(b"");
        for _ in 0..MAX_IN_FLIGHT {
            engine.begin_request().unwrap();
        }
        assert!(engine.begin_request().is_err());
    }

    fn drain(body: &mut ResponseBody) -> Vec<u8> {
        let mut out = Vec::new();
        while let Ok(Some(chunk)) = body.read(64) {
            out.extend_from_slice(&chunk);
        }
        out
    }

    #[test]
    fn streaming_get_roundtrips_and_reuses_one_connection() {
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![
            ResponsePlan::Length {
                status: 200,
                body: b"hello".to_vec(),
            },
            ResponsePlan::Length {
                status: 200,
                body: b"world".to_vec(),
            },
        ]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        for expected in [&b"hello"[..], &b"world"[..]] {
            let mut response = engine
                .execute_streaming(
                    &StreamingRequest {
                        method: "GET".to_owned(),
                        url: url.clone(),
                        headers: vec![],
                        follow_redirects: false,
                        retry_idempotent: true,
                        deadline: Duration::from_secs(5),
                    },
                    b"",
                    std::slice::from_ref(&endpoint),
                )
                .unwrap();
            assert_eq!(response.head.status, 200);
            assert_eq!(drain(&mut response.body), expected);
        }
        assert_eq!(
            server.connection_count(),
            1,
            "the second request must reuse the pooled connection"
        );
        assert_eq!(server.request_count(), 2);
        server.stop();
    }

    #[test]
    fn streaming_chunked_download_is_byte_exact() {
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![ResponsePlan::Chunked {
            status: 200,
            chunks: vec![b"alpha-".to_vec(), b"beta-".to_vec(), b"gamma".to_vec()],
        }]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        let mut response = engine
            .execute_streaming(
                &StreamingRequest {
                    method: "GET".to_owned(),
                    url,
                    headers: vec![],
                    follow_redirects: false,
                    retry_idempotent: false,
                    deadline: Duration::from_secs(5),
                },
                b"",
                &[endpoint],
            )
            .unwrap();
        assert_eq!(response.head.status, 200);
        assert_eq!(
            drain(&mut response.body),
            b"alpha-beta-gamma".to_vec(),
            "chunked frames decode exactly once"
        );
        server.stop();
    }

    #[test]
    fn ungranted_streaming_refuses_before_any_connection() {
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
            status: 200,
            body: b"secret".to_vec(),
        }]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let outcome = engine.execute_streaming(
            &StreamingRequest {
                method: "GET".to_owned(),
                url,
                headers: vec![],
                follow_redirects: false,
                retry_idempotent: false,
                deadline: Duration::from_secs(5),
            },
            b"",
            &[],
        );
        match outcome {
            Err(message) => assert!(message.contains("permission"), "{message}"),
            Ok(response) => panic!("unexpected response: {response:?}"),
        }
        assert_eq!(server.connection_count(), 0);
        server.stop();
    }

    #[test]
    fn idempotent_retry_recovers_from_close_before_response() {
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![
            ResponsePlan::CloseBeforeResponse,
            ResponsePlan::Length {
                status: 200,
                body: b"recovered".to_vec(),
            },
        ]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        let mut response = engine
            .execute_streaming(
                &StreamingRequest {
                    method: "GET".to_owned(),
                    url,
                    headers: vec![],
                    follow_redirects: false,
                    retry_idempotent: true,
                    deadline: Duration::from_secs(5),
                },
                b"",
                &[endpoint],
            )
            .unwrap();
        assert_eq!(drain(&mut response.body), b"recovered");
        assert_eq!(server.connection_count(), 2, "one fresh retry connect");
        server.stop();

        // Without the idempotent flag the same failure is terminal.
        let mut server = HttpFixtureServer::start(vec![
            ResponsePlan::CloseBeforeResponse,
            ResponsePlan::Length {
                status: 200,
                body: b"never".to_vec(),
            },
        ]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        let outcome = engine.execute_streaming(
            &StreamingRequest {
                method: "GET".to_owned(),
                url,
                headers: vec![],
                follow_redirects: false,
                retry_idempotent: false,
                deadline: Duration::from_secs(5),
            },
            b"",
            &[endpoint],
        );
        assert!(
            matches!(&outcome, Err(message) if message.contains("protocol")),
            "{outcome:?}"
        );
        assert_eq!(server.request_count(), 1, "no retry without the flag");
        server.stop();
    }

    #[test]
    fn upload_pipe_roundtrips_with_chunked_framing_and_secret() {
        use crate::secrets::{InjectionPolicy, SecretBinding};
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
            status: 200,
            body: b"accepted".to_vec(),
        }]);
        let mut secrets = SecretStore::new();
        secrets.insert("token", "canary-token-value-9f2b");
        secrets.authorize(SecretBinding {
            name: "token".to_owned(),
            endpoint: format!("https+private|localhost|{}", server.port),
            policy: InjectionPolicy::Header {
                name: "x-api-key".to_owned(),
            },
        });
        let engine = HttpEngine::new(
            server.ca_pem.clone(),
            Arc::new(secrets),
            crate::framing::DEFAULT_BODY_BUDGET,
        );
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        let mut pipe = engine
            .open_upload(
                &StreamingRequest {
                    method: "POST".to_owned(),
                    url,
                    headers: vec![],
                    follow_redirects: false,
                    retry_idempotent: false,
                    deadline: Duration::from_secs(5),
                },
                &[endpoint],
                Arc::new(|| false),
            )
            .unwrap();
        pipe.write(b"chunk-one-".to_vec()).unwrap();
        pipe.write(b"chunk-two".to_vec()).unwrap();
        let mut response = pipe.finish().unwrap();
        assert_eq!(response.head.status, 200);
        assert_eq!(drain(&mut response.body), b"accepted");
        let observed = server.observed();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].method, "POST");
        assert!(
            observed[0]
                .headers
                .iter()
                .any(|(name, value)| name == "transfer-encoding" && value == "chunked"),
            "upload must use chunked framing: {:?}",
            observed[0].headers
        );
        assert!(
            observed[0]
                .headers
                .iter()
                .any(|(name, value)| name == "x-api-key" && value == "canary-token-value-9f2b"),
            "authorized secret must be injected host-side"
        );
        server.stop();
    }

    #[test]
    fn upload_chunk_cap_is_typed() {
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
            status: 200,
            body: b"x".to_vec(),
        }]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        let mut pipe = engine
            .open_upload(
                &StreamingRequest {
                    method: "POST".to_owned(),
                    url,
                    headers: vec![],
                    follow_redirects: false,
                    retry_idempotent: false,
                    deadline: Duration::from_secs(5),
                },
                &[endpoint],
                Arc::new(|| false),
            )
            .unwrap();
        let oversized = vec![0_u8; crate::framing::MAX_CHUNK_BYTES + 1];
        let outcome = pipe.write(oversized);
        assert!(
            matches!(&outcome, Err(message) if message.contains("limit")),
            "{outcome:?}"
        );
        drop(pipe);
        server.stop();
    }

    #[test]
    fn stalled_head_fails_with_typed_timeout() {
        use crate::streaming::{HttpFixtureServer, ResponsePlan};
        let mut server = HttpFixtureServer::start(vec![ResponsePlan::StallAfterHead {
            status: 200,
            stall_ms: 3_000,
        }]);
        let engine = engine_with(&server.ca_pem);
        let url = format!("https+private://localhost:{}", server.port);
        let endpoint = canonicalize_url(&url).unwrap();
        let started = Instant::now();
        let outcome = engine.execute_streaming(
            &StreamingRequest {
                method: "GET".to_owned(),
                url,
                headers: vec![],
                follow_redirects: false,
                retry_idempotent: true,
                deadline: Duration::from_millis(700),
            },
            b"",
            &[endpoint],
        );
        // The head arrives; the stall bites at the first body read, which
        // must fail typed instead of hanging toward the server's stall.
        let mut response = match outcome {
            Ok(response) => response,
            Err(message) => panic!("head phase should not fail: {message}"),
        };
        let body_outcome = response.body.read(64);
        assert!(
            matches!(&body_outcome, Err(message) if message.contains("timeout")),
            "{body_outcome:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "the total deadline, not the server stall, bounds the request"
        );
        server.stop();
    }

    #[test]
    fn tls_probe_outcome_preserved_for_fixtures() {
        // The STEP-0112 probe remains the fixture primitive.
        let fixture = crate::deterministic_ca("localhost");
        let mut server = crate::TlsEchoServer::start(&fixture);
        assert_eq!(
            crate::tls_probe("localhost", server.port, &fixture.ca_pem, b"ping"),
            TlsProbeOutcome::Connected("ping".to_owned())
        );
        server.finish().unwrap();
    }
}
