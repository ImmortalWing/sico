//! Live streaming responses (M12 runner integration): one pinned connection
//! per response, strict incremental framing decode, and the deterministic
//! TLS HTTP server fixture used by the runner-level corpus. A body reader
//! never buffers the whole payload; every read returns at most one bounded
//! chunk and a connection is reusable only within one engine (one Store).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

use crate::framing::{
    BodyFraming, ChunkedReader, FramingError, MAX_CHUNK_BYTES, MAX_TRAILER_LINES, decide_framing,
};

/// Response head: status plus the raw header pairs as received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseHead {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

impl ResponseHead {
    /// First header value for `name` (case-insensitive), if present.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// True when the server promised no reuse for this connection.
    #[must_use]
    pub fn connection_close(&self) -> bool {
        self.header("connection")
            .is_some_and(|value| value.eq_ignore_ascii_case("close"))
    }
}

/// A pooled transport: plain TCP or a completed rustls client session.
#[derive(Debug)]
pub enum Transport {
    /// Unencrypted loopback/development transport behind an `http://` grant.
    Plain(TcpStream),
    /// Verified TLS session behind an `https://` grant.
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Read for Transport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buf),
            Self::Tls(session) => session.read(buf),
        }
    }
}

impl Write for Transport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buf),
            Self::Tls(session) => session.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Tls(session) => session.flush(),
        }
    }
}

impl Transport {
    /// Applies one read/write timeout to the underlying socket.
    ///
    /// # Errors
    ///
    /// Socket configuration failure.
    pub fn set_timeout(&self, timeout: Duration) -> Result<(), String> {
        let stream = match self {
            Self::Plain(stream) => stream,
            Self::Tls(session) => session.get_ref(),
        };
        stream
            .set_read_timeout(Some(timeout))
            .and_then(|()| stream.set_write_timeout(Some(timeout)))
            .map_err(|error| format!("io: {error}"))
    }

    /// True once the peer half-closed, so a pooled entry must be discarded
    /// instead of reused. TLS sessions report not-drained (the record layer
    /// would consume any close-notify on the next read anyway).
    #[must_use]
    pub fn drained(&self) -> bool {
        match self {
            Self::Plain(stream) => stream.peek(&mut [0_u8; 1]).is_ok_and(|read| read == 0),
            Self::Tls(_) => false,
        }
    }
}

/// Reads the strictly framed response head: bounded head bytes, no obs-fold,
/// duplicate `content-length`/`transfer-encoding` preserved for the framing
/// decision. This is the one head parser shared by the buffered engine path
/// and the streaming path.
///
/// # Errors
///
/// `protocol` on malformed heads, `io` on transport failures, `limit` past
/// the 64 KiB head bound. Socket timeouts are the caller's responsibility.
pub fn read_head<S: Read>(stream: &mut S) -> Result<ResponseHead, String> {
    let mut buffer = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        let read = stream
            .read(&mut byte)
            .map_err(|error| crate::engine::classify_io(&error))?;
        if read == 0 {
            return Err("protocol: connection closed before response head".to_owned());
        }
        buffer.push(byte[0]);
        if buffer.ends_with(b"\r\n\r\n") {
            break;
        }
        if buffer.len() > 64 * 1024 {
            return Err("limit: response head exceeds 64 KiB".to_owned());
        }
    }
    parse_response_head(&buffer)
}

/// Parses one complete head (status line + CRLF-terminated header section).
///
/// # Errors
///
/// `protocol` on malformed status/header lines, non-UTF-8 heads, or
/// out-of-bounds header sets.
pub fn parse_response_head(buffer: &[u8]) -> Result<ResponseHead, String> {
    let text = std::str::from_utf8(buffer).map_err(|_| "protocol: head is not UTF-8".to_owned())?;
    let mut lines = text.trim_end().split("\r\n");
    let status_line = lines.next().ok_or("protocol: empty response")?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or("protocol: malformed status line")?;
    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err("protocol: malformed header line".to_owned());
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name.is_empty() || name.len() > 256 || value.len() > 8 * 1024 {
            return Err("protocol: header bounds exceeded".to_owned());
        }
        headers.push((name, value));
    }
    if headers.len() > 256 {
        return Err("limit: more than 256 response headers".to_owned());
    }
    Ok(ResponseHead { status, headers })
}

/// Decides the body framing from one parsed head (strict, RFC-0037 §7).
///
/// # Errors
///
/// Framing refusals (conflicting lengths, mixed encodings) as `protocol`,
/// over-budget declared lengths as `limit`.
pub fn framing_of(head: &ResponseHead, budget: u64) -> Result<BodyFraming, String> {
    let content_lengths: Vec<&str> = head
        .headers
        .iter()
        .filter(|(name, _)| name == "content-length")
        .map(|(_, value)| value.as_str())
        .collect();
    let transfer_encoding = head
        .headers
        .iter()
        .find(|(name, _)| name == "transfer-encoding")
        .map(|(_, value)| value.as_str());
    decide_framing(&content_lengths, transfer_encoding, budget).map_err(|error| match error {
        FramingError::BudgetExceeded => "limit: body exceeds budget".to_owned(),
        other => format!("protocol: {other}"),
    })
}

/// Incremental body reader over one transport: serves bounded chunks as the
/// wire provides them, never buffering the whole payload. One terminal state:
/// the first `Err` poisons the reader, and clean EOF is reported once.
#[derive(Debug)]
pub struct BodyReader {
    transport: Option<Transport>,
    framing: BodyFraming,
    remaining: u64,
    chunked: ChunkedReader,
    consumed: u64,
    budget: u64,
    deadline: Instant,
    finished: bool,
    poisoned: bool,
}

impl BodyReader {
    /// Builds a reader for a head whose framing was already decided.
    #[must_use]
    pub fn new(transport: Transport, framing: BodyFraming, budget: u64, deadline: Instant) -> Self {
        let remaining = match framing {
            BodyFraming::Length(length) => length,
            _ => 0,
        };
        Self {
            transport: Some(transport),
            framing,
            remaining,
            chunked: ChunkedReader::new(),
            consumed: 0,
            budget,
            deadline,
            finished: false,
            poisoned: false,
        }
    }

    /// True once the body ended cleanly (framing completed exactly).
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finished
    }

    fn transport(&mut self) -> Result<&mut Transport, String> {
        self.transport
            .as_mut()
            .ok_or_else(|| "protocol: body already terminal".to_owned())
    }

    fn deadline_left(&self) -> Result<Duration, String> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| "timeout: body deadline elapsed".to_owned())
    }

    /// Next bounded payload chunk: `Ok(None)` is clean EOF, `Ok(Some(bytes))`
    /// is at most [`MAX_CHUNK_BYTES`], and the first `Err` is terminal.
    ///
    /// # Errors
    ///
    /// Typed `protocol`/`limit`/`io`/`timeout` classifications.
    pub fn read(&mut self, max: usize) -> Result<Option<Vec<u8>>, String> {
        if self.poisoned || self.finished {
            return Err("protocol: body already terminal".to_owned());
        }
        let take = max.min(MAX_CHUNK_BYTES);
        let outcome = match self.framing {
            BodyFraming::Empty => Ok(Vec::new()),
            BodyFraming::Length(_) => self.read_length(take),
            BodyFraming::Chunked => self.read_chunked(take),
            BodyFraming::UntilClose => self.read_until_close(take),
        };
        match outcome {
            Ok(chunk) if chunk.is_empty() => {
                self.finished = true;
                Ok(None)
            }
            Ok(chunk) => Ok(Some(chunk)),
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }

    fn read_length(&mut self, take: usize) -> Result<Vec<u8>, String> {
        if self.remaining == 0 {
            return Ok(Vec::new());
        }
        let want = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(take);
        let mut buffer = vec![0_u8; want];
        let left = self.deadline_left()?;
        let transport = self.transport()?;
        transport.set_timeout(left)?;
        let read = transport
            .read(&mut buffer)
            .map_err(|error| crate::engine::classify_io(&error))?;
        if read == 0 {
            return Err("protocol: connection closed inside body".to_owned());
        }
        buffer.truncate(read);
        self.consumed += read as u64;
        self.remaining -= read as u64;
        if self.consumed > self.budget {
            return Err("limit: body exceeds budget".to_owned());
        }
        Ok(buffer)
    }

    fn read_until_close(&mut self, take: usize) -> Result<Vec<u8>, String> {
        let mut buffer = vec![0_u8; take];
        let left = self.deadline_left()?;
        let transport = self.transport()?;
        transport.set_timeout(left)?;
        let read = transport
            .read(&mut buffer)
            .map_err(|error| crate::engine::classify_io(&error))?;
        if read == 0 {
            return Ok(Vec::new());
        }
        buffer.truncate(read);
        self.consumed += read as u64;
        if self.consumed > self.budget {
            return Err("limit: body exceeds budget".to_owned());
        }
        Ok(buffer)
    }

    fn read_chunked(&mut self, take: usize) -> Result<Vec<u8>, String> {
        loop {
            if self.chunked.trailer_expected() {
                self.read_trailers()?;
                return Ok(Vec::new());
            }
            if self.chunked.expecting_data() {
                // Read at most the pending chunk bytes: the strict chunked
                // reader is incremental, and bytes beyond the current chunk
                // belong to the next size line.
                let want = usize::try_from(self.chunked.pending_chunk_bytes())
                    .unwrap_or(usize::MAX)
                    .min(take);
                let mut buffer = vec![0_u8; want];
                let left = self.deadline_left()?;
                let transport = self.transport()?;
                transport.set_timeout(left)?;
                let read = transport
                    .read(&mut buffer)
                    .map_err(|error| crate::engine::classify_io(&error))?;
                if read == 0 {
                    return Err("protocol: connection closed inside chunk".to_owned());
                }
                let payload = self
                    .chunked
                    .chunk_data(&buffer[..read], self.consumed, self.budget)
                    .map_err(|error| match error {
                        FramingError::BudgetExceeded => "limit: body exceeds budget".to_owned(),
                        other => format!("protocol: {other}"),
                    })?;
                self.consumed += payload as u64;
                buffer.truncate(payload);
                if self.chunked.pending_chunk_bytes() == 0 {
                    // The chunk-terminating CRLF belongs to this chunk's
                    // framing; consume and verify it before the next size
                    // line.
                    const CHUNK_CRLF: &[u8] = b"\r\n";
                    self.read_exact_framing(CHUNK_CRLF)?;
                }
                if payload > 0 {
                    return Ok(buffer);
                }
                continue;
            }
            let line = self.read_line(16)?;
            self.chunked
                .chunk_size_line(&line)
                .map_err(|error| format!("protocol: {error}"))?;
        }
    }

    fn read_trailers(&mut self) -> Result<(), String> {
        let mut trailers = Vec::new();
        loop {
            let line = self.read_line(2048)?;
            if line.is_empty() {
                let lines: Vec<&str> = trailers.iter().map(String::as_str).collect();
                crate::framing::validate_trailer(&lines)
                    .map_err(|error| format!("protocol: {error}"))?;
                return Ok(());
            }
            trailers.push(line);
            if trailers.len() > MAX_TRAILER_LINES {
                return Err("protocol: too many trailer lines".to_owned());
            }
        }
    }

    /// Reads and verifies one exact framing byte sequence (chunk CRLF).
    fn read_exact_framing(&mut self, expected: &[u8]) -> Result<(), String> {
        let left = self.deadline_left()?;
        let transport = self.transport()?;
        transport.set_timeout(left)?;
        let mut buffer = [0_u8; 2];
        let mut filled = 0;
        while filled < expected.len() {
            let read = transport
                .read(&mut buffer[filled..])
                .map_err(|error| crate::engine::classify_io(&error))?;
            if read == 0 {
                return Err("protocol: connection closed inside framing".to_owned());
            }
            filled += read;
        }
        if &buffer[..filled] != expected {
            return Err("protocol: chunk terminator missing".to_owned());
        }
        Ok(())
    }

    /// Reads one CRLF-terminated line byte-wise, bounded by `limit`.
    fn read_line(&mut self, limit: usize) -> Result<String, String> {
        let left = self.deadline_left()?;
        let transport = self.transport()?;
        transport.set_timeout(left)?;
        let mut line = Vec::new();
        loop {
            let mut byte = [0_u8; 1];
            let read = transport
                .read(&mut byte)
                .map_err(|error| crate::engine::classify_io(&error))?;
            if read == 0 {
                return Err("protocol: connection closed inside framing".to_owned());
            }
            if byte[0] == b'\n' {
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return std::str::from_utf8(&line)
                    .map_err(|_| "protocol: framing line is not UTF-8".to_owned())
                    .map(str::to_owned);
            }
            line.push(byte[0]);
            if line.len() > limit {
                return Err("protocol: framing line exceeds bound".to_owned());
            }
        }
    }

    /// Takes the transport back after clean EOF (pooling handoff). `None`
    /// while the body is unfinished or poisoned.
    pub fn take_transport(&mut self) -> Option<Transport> {
        if self.finished && !self.poisoned {
            self.transport.take()
        } else {
            None
        }
    }
}

impl Drop for BodyReader {
    fn drop(&mut self) {
        // The transport drops with the reader; the engine pools it only
        // through take_transport, so an abandoned body closes the socket.
        self.transport.take();
    }
}

/// True when a method may be replayed on a transport failure that happened
/// before any response byte (RFC-0037 §5: retry is idempotent-only).
#[must_use]
pub fn is_idempotent(method: &str) -> bool {
    matches!(method, "GET" | "HEAD")
}

/// One pooled connection entry: endpoint identity plus the idle deadline.
#[derive(Debug)]
pub struct PooledConnection {
    pub endpoint_identity: String,
    pub transport: Transport,
    pub idle_since: Instant,
}

/// Bounded per-Store pool: ≤ [`crate::engine::MAX_POOLED_CONNECTIONS`]
/// entries, ≤ [`crate::engine::MAX_CONNECTIONS_PER_ENDPOINT`] per endpoint,
/// every entry dropped after [`MAX_IDLE`].
#[derive(Debug, Default)]
pub struct ConnectionPool {
    entries: Vec<PooledConnection>,
}

/// Explicit idle bound for pooled connections (RFC-0037 §7).
pub const MAX_IDLE: Duration = Duration::from_secs(15);

impl ConnectionPool {
    /// Takes one live connection for `endpoint_identity`, dropping stale or
    /// drained entries first.
    pub fn take(&mut self, endpoint_identity: &str) -> Option<Transport> {
        let now = Instant::now();
        self.entries.retain(|entry| {
            now.duration_since(entry.idle_since) < MAX_IDLE && !entry.transport.drained()
        });
        let index = self.entries.iter().position(|entry| {
            entry.endpoint_identity == endpoint_identity
                && now.duration_since(entry.idle_since) < MAX_IDLE
        })?;
        let entry = self.entries.remove(index);
        Some(entry.transport)
    }

    /// Returns one connection to the pool. Caps are enforced by dropping
    /// the oldest entry (never by failing the response).
    pub fn put(&mut self, endpoint_identity: &str, transport: Transport) {
        let per_endpoint = self
            .entries
            .iter()
            .filter(|entry| entry.endpoint_identity == endpoint_identity)
            .count();
        if per_endpoint >= crate::engine::MAX_CONNECTIONS_PER_ENDPOINT {
            return;
        }
        while self.entries.len() >= crate::engine::MAX_POOLED_CONNECTIONS {
            self.entries.remove(0);
        }
        self.entries.push(PooledConnection {
            endpoint_identity: endpoint_identity.to_owned(),
            transport,
            idle_since: Instant::now(),
        });
    }

    /// Drops every entry (Store teardown; no socket survives the run).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of pooled entries right now.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no entry is pooled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Shared pool handle handed to response bodies so a clean EOF can return
/// the connection without owning the engine.
pub type SharedPool = Arc<Mutex<ConnectionPool>>;

// ---------------------------------------------------------------------------
// Deterministic TLS HTTP server fixture (runner-level corpus).
// ---------------------------------------------------------------------------

/// One planned response of the fixture server.
#[derive(Clone, Debug)]
pub enum ResponsePlan {
    /// Content-Length framed body.
    Length { status: u16, body: Vec<u8> },
    /// Chunked framed body served as the given payload chunks.
    Chunked { status: u16, chunks: Vec<Vec<u8>> },
    /// No framing headers; the connection closes after the body.
    UntilClose { status: u16, body: Vec<u8> },
    /// Accept + TLS handshake, then close without any response bytes
    /// (transport-failure retry fixture).
    CloseBeforeResponse,
    /// Send the head, hang past the client deadline, then close (cancellation
    /// and deadline fixtures).
    StallAfterHead { status: u16, stall_ms: u64 },
    /// Wait `delay_ms` before each body chunk so the client can cancel
    /// mid-body; served with chunked framing.
    SlowBody {
        status: u16,
        body: Vec<u8>,
        chunk: usize,
        delay_ms: u64,
    },
}

impl ResponsePlan {
    fn closes(&self) -> bool {
        matches!(
            self,
            Self::UntilClose { .. } | Self::CloseBeforeResponse | Self::StallAfterHead { .. }
        )
    }
}

/// One observed fixture request (shared back to the test).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedRequest {
    pub method: String,
    pub target: String,
    pub headers: Vec<(String, String)>,
}

/// Deterministic HTTPS fixture speaking real HTTP/1.1. Connections are
/// served one at a time; responses follow the plan list in order and the
/// last plan repeats. Keep-alive is honored unless the plan closes.
pub struct HttpFixtureServer {
    pub port: u16,
    pub ca_pem: Vec<u8>,
    connections: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
    observed: Arc<Mutex<Vec<ObservedRequest>>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl HttpFixtureServer {
    /// Starts the server against a fresh deterministic CA for `localhost`.
    ///
    /// # Panics
    ///
    /// Panics on bind/certificate failures, which the caller controls.
    #[must_use]
    pub fn start(plans: Vec<ResponsePlan>) -> Self {
        let fixture = crate::deterministic_ca("localhost");
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("ephemeral bind");
        let port = listener.local_addr().expect("local addr").port();
        let leaf_cert = CertificateDer::from_pem_slice(&fixture.leaf_pem).expect("leaf PEM parses");
        let leaf_key =
            PrivateKeyDer::from_pem_slice(&fixture.leaf_key_pem).expect("leaf key PEM parses");
        let config = Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![leaf_cert], leaf_key)
                .expect("server cert/key pair"),
        );
        let connections = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(AtomicUsize::new(0));
        let observed = Arc::new(Mutex::new(Vec::new()));
        let worker_connections = Arc::clone(&connections);
        let worker_requests = Arc::clone(&requests);
        let worker_observed = Arc::clone(&observed);
        let plans = Mutex::new(plans);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            listener
                .set_nonblocking(true)
                .expect("fixture listener nonblocking");
            loop {
                if worker_stop.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        worker_connections.fetch_add(1, Ordering::SeqCst);
                        if serve_connection(
                            stream,
                            &config,
                            &plans,
                            &worker_requests,
                            &worker_observed,
                        )
                        .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            port,
            ca_pem: fixture.ca_pem,
            connections,
            requests,
            observed,
            stop,
            worker: Some(worker),
        }
    }

    /// Number of TCP connections accepted so far (pooling evidence).
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Number of complete requests observed so far.
    #[must_use]
    pub fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    /// Snapshots every observed request so far.
    #[must_use]
    pub fn observed(&self) -> Vec<ObservedRequest> {
        self.observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Stops accepting and joins the server thread.
    ///
    /// # Panics
    ///
    /// Panics if the server thread panicked, which only a fixture bug does.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("fixture server thread");
        }
    }
}

impl Drop for HttpFixtureServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn serve_connection(
    mut stream: TcpStream,
    config: &Arc<rustls::ServerConfig>,
    plans: &Mutex<Vec<ResponsePlan>>,
    requests: &Arc<AtomicUsize>,
    observed: &Arc<Mutex<Vec<ObservedRequest>>>,
) -> Result<(), String> {
    // Windows sockets inherit the listener's non-blocking state; the
    // request/response loop below needs blocking reads.
    stream
        .set_nonblocking(false)
        .and_then(|()| stream.set_read_timeout(Some(Duration::from_secs(30))))
        .map_err(|error| error.to_string())?;
    let mut conn =
        rustls::ServerConnection::new(Arc::clone(config)).map_err(|error| error.to_string())?;
    let mut tls = rustls::Stream::new(&mut conn, &mut stream);
    loop {
        let Some(request) = read_request(&mut tls)? else {
            return Ok(());
        };
        requests.fetch_add(1, Ordering::SeqCst);
        observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(request);
        let plan = {
            let mut guard = plans
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if guard.is_empty() {
                return Ok(());
            }
            if guard.len() == 1 {
                guard[0].clone()
            } else {
                guard.remove(0)
            }
        };
        let close = plan.closes();
        write_response(&mut tls, &plan)?;
        if close {
            return Ok(());
        }
    }
}

/// Reads one bounded request head (+ body) from the fixture stream. `None`
/// on clean client close before any byte.
fn read_request<S: Read>(stream: &mut S) -> Result<Option<ObservedRequest>, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let head_end = loop {
        let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if read == 0 {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err("client closed mid-head".to_owned());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
        if buffer.len() > 64 * 1024 {
            return Err("request head too large".to_owned());
        }
    };
    let head_text = std::str::from_utf8(&buffer[..head_end])
        .map_err(|_| "request head is not UTF-8".to_owned())?;
    let mut lines = head_text.split("\r\n");
    let request_line = lines.next().ok_or("empty request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_owned();
    let target = parts.next().ok_or("missing target")?.to_owned();
    let mut headers = Vec::new();
    let mut content_length = 0_usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err("malformed request header".to_owned());
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name == "content-length" {
            content_length = value.parse().map_err(|_| "bad content-length")?;
        }
        headers.push((name, value));
    }
    if content_length > 1024 * 1024 {
        return Err("fixture body cap exceeded".to_owned());
    }
    let mut body = buffer[head_end + 4..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("client closed mid-body".to_owned());
        }
        body.extend_from_slice(&chunk[..read]);
    }
    drop(body);
    Ok(Some(ObservedRequest {
        method,
        target,
        headers,
    }))
}

fn write_response<S: Write>(stream: &mut S, plan: &ResponsePlan) -> Result<(), String> {
    match plan {
        ResponsePlan::CloseBeforeResponse => {
            // Nothing: the caller closes the connection.
            Ok(())
        }
        ResponsePlan::Length { status, body } => {
            let head = format!(
                "HTTP/1.1 {status} OK\r\ncontent-length: {}\r\n\r\n",
                body.len()
            );
            stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(body))
                .and_then(|()| stream.flush())
                .map_err(|error| error.to_string())
        }
        ResponsePlan::Chunked { status, chunks } => {
            let head = format!("HTTP/1.1 {status} OK\r\ntransfer-encoding: chunked\r\n\r\n");
            stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.flush())
                .map_err(|error| error.to_string())?;
            for chunk in chunks {
                write_all(stream, &chunk_frame(chunk))?;
            }
            write_all(stream, b"0\r\n\r\n")
        }
        ResponsePlan::UntilClose { status, body } => {
            let head = format!("HTTP/1.1 {status} OK\r\n\r\n");
            stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(body))
                .and_then(|()| stream.flush())
                .map_err(|error| error.to_string())
        }
        ResponsePlan::StallAfterHead { status, stall_ms } => {
            let head = format!("HTTP/1.1 {status} OK\r\n\r\n");
            stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.flush())
                .map_err(|error| error.to_string())?;
            std::thread::sleep(Duration::from_millis(*stall_ms));
            Ok(())
        }
        ResponsePlan::SlowBody {
            status,
            body,
            chunk,
            delay_ms,
        } => {
            let head = format!("HTTP/1.1 {status} OK\r\ntransfer-encoding: chunked\r\n\r\n");
            stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.flush())
                .map_err(|error| error.to_string())?;
            for piece in body.chunks((*chunk).max(1)) {
                std::thread::sleep(Duration::from_millis(*delay_ms));
                write_all(stream, &chunk_frame(piece))?;
            }
            write_all(stream, b"0\r\n\r\n")
        }
    }
}

fn chunk_frame(chunk: &[u8]) -> Vec<u8> {
    let mut frame = format!("{:x}\r\n", chunk.len()).into_bytes();
    frame.extend_from_slice(chunk);
    frame.extend_from_slice(b"\r\n");
    frame
}

fn write_all<S: Write>(stream: &mut S, mut bytes: &[u8]) -> Result<(), String> {
    while !bytes.is_empty() {
        let written = stream.write(bytes).map_err(|error| error.to_string())?;
        if written == 0 {
            return Err("fixture write made no progress".to_owned());
        }
        bytes = &bytes[written..];
    }
    stream.flush().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_parser_preserves_status_and_headers() {
        let head =
            parse_response_head(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nx-a: b\r\n\r\n").unwrap();
        assert_eq!(head.status, 200);
        assert_eq!(head.header("content-length"), Some("5"));
        assert!(!head.connection_close());
    }

    #[test]
    fn head_parser_refuses_garbage() {
        assert!(parse_response_head(b"not http\r\n\r\n").is_err());
        assert!(parse_response_head(b"HTTP/1.1 nnn OK\r\n\r\n").is_err());
        assert!(parse_response_head(b"HTTP/1.1 200 OK\r\nbroken\r\n\r\n").is_err());
    }

    #[test]
    fn length_body_reader_serves_exact_bytes_then_eof() {
        let (client, mut server) = pair();
        server.set_nonblocking(true).unwrap();
        let mut reader = BodyReader::new(
            Transport::Plain(client),
            BodyFraming::Length(5),
            1024,
            Instant::now() + Duration::from_secs(5),
        );
        server
            .set_nonblocking(false)
            .and_then(|()| server.write_all(b"hello"))
            .unwrap();
        assert_eq!(reader.read(64).unwrap().unwrap(), b"hello");
        assert!(reader.read(64).unwrap().is_none());
        assert!(reader.finished());
        assert!(reader.take_transport().is_some());
    }

    #[test]
    fn chunked_body_reader_decodes_incrementally() {
        let (client, mut server) = pair();
        server.set_nonblocking(true).unwrap();
        let mut reader = BodyReader::new(
            Transport::Plain(client),
            BodyFraming::Chunked,
            1024,
            Instant::now() + Duration::from_secs(5),
        );
        server.set_nonblocking(false).unwrap();
        server
            .write_all(b"3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n")
            .unwrap();
        assert_eq!(reader.read(64).unwrap().unwrap(), b"abc");
        assert_eq!(reader.read(64).unwrap().unwrap(), b"de");
        assert!(reader.read(64).unwrap().is_none());
    }

    #[test]
    fn chunked_reader_refuses_bad_size() {
        let (client, mut server) = pair();
        server.set_nonblocking(true).unwrap();
        let mut reader = BodyReader::new(
            Transport::Plain(client),
            BodyFraming::Chunked,
            1024,
            Instant::now() + Duration::from_secs(5),
        );
        server.set_nonblocking(false).unwrap();
        server.write_all(b"zz\r\n").unwrap();
        assert!(reader.read(64).is_err());
        assert!(reader.take_transport().is_none());
    }

    #[test]
    fn idempotent_methods_are_exactly_get_and_head() {
        assert!(is_idempotent("GET"));
        assert!(is_idempotent("HEAD"));
        assert!(!is_idempotent("POST"));
        assert!(!is_idempotent("get"));
    }

    #[test]
    fn pool_respects_per_endpoint_cap() {
        let (clients, server) = many_pair(crate::engine::MAX_CONNECTIONS_PER_ENDPOINT + 2);
        drop(server);
        let mut pool = ConnectionPool::default();
        for client in clients {
            pool.put("https|localhost|443", Transport::Plain(client));
        }
        assert_eq!(
            pool.len(),
            crate::engine::MAX_CONNECTIONS_PER_ENDPOINT,
            "per-endpoint cap enforced"
        );
        pool.clear();
        assert!(pool.is_empty());
    }

    #[test]
    fn chunk_frame_is_canonical() {
        assert_eq!(chunk_frame(b"abc"), b"3\r\nabc\r\n".to_vec());
    }

    fn pair() -> (std::net::TcpStream, std::net::TcpStream) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    fn many_pair(count: usize) -> (Vec<std::net::TcpStream>, std::net::TcpListener) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let clients = (0..count)
            .map(|_| std::net::TcpStream::connect(("127.0.0.1", port)).unwrap())
            .collect();
        let _ = listener.set_nonblocking(true);
        let servers: Vec<_> = (0..count)
            .filter_map(|_| listener.accept().ok())
            .map(|(stream, _)| stream)
            .collect();
        drop(servers);
        (clients, listener)
    }
}
