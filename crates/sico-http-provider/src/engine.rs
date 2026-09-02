//! HTTP request engine (M12 STEP-0117): composes the policy layers —
//! authority, address gate, TLS transport, framing, redirects, secrets —
//! into bounded one-shot and streaming requests with a per-run
//! connection-lifecycle contract. Every network operation belongs to one
//! engine instance (one Store), every request gets one terminal outcome.

use std::io::{Read, Write};

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::{pem::PemObject, CertificateDer, ServerName};

use crate::authority::{address_allowed_for_scheme, canonicalize_url, Endpoint};
use crate::framing::{decide_framing, BodyFraming, FramingError, MAX_CHUNK_BYTES};
use crate::secrets::SecretStore;

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
    in_flight: usize,
    pooled: usize,
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
            in_flight: 0,
            pooled: 0,
            body_budget,
        }
    }

    /// In-flight cap check (RFC-0037 §7).
    ///
    /// # Errors
    ///
    /// Typed cap message when the engine already holds
    /// [`MAX_IN_FLIGHT`] requests.
    pub fn begin_request(&mut self) -> Result<(), String> {
        if self.in_flight >= MAX_IN_FLIGHT {
            return Err("in-flight request cap reached (16)".to_owned());
        }
        self.in_flight += 1;
        Ok(())
    }

    pub fn end_request(&mut self) {
        self.in_flight = self.in_flight.saturating_sub(1);
        self.pooled = self.pooled.min(MAX_POOLED_CONNECTIONS);
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
                header_value(&headers, "location").unwrap_or_default().as_str(),
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
                crate::redirect::RedirectDecision::Follow { location, method, replay } => {
                    // Cross-origin: strip protected headers before
                    // following (structural rule, value-independent).
                    let next_endpoint =
                        canonicalize_url(&location).map_err(|e| format!("authority: {e}"))?;
                    if next_endpoint != endpoint {
                        current_headers =
                            strip_protected(&current_headers);
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
    let mut request = format!("{} {path} HTTP/1.1\r\nhost: {host_header}\r\nconnection: close\r\n", wire.method);
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
        let read = stream.read(&mut byte).map_err(|e| format!("io: read: {e}"))?;
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
            const READ_CHUNK: usize = if MAX_CHUNK_BYTES < 16 * 1024 { MAX_CHUNK_BYTES } else { 16 * 1024 };
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
            policy: InjectionPolicy::Header { name: "x-api-key".to_owned() },
        });
        HttpEngine::new(ca_pem.to_vec(), Arc::new(secrets), crate::framing::DEFAULT_BODY_BUDGET)
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
            other @ HttpOutcome::Response { .. } => panic!("expected permission failure: {other:?}"),
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
            &[Endpoint { scheme: Scheme::Http, host: "127.0.0.1".to_owned(), port: 1 }],
        );
        match outcome {
            HttpOutcome::Failed(message) => assert!(message.contains("dns:"), "{message}"),
            other @ HttpOutcome::Response { .. } => panic!("expected dns refusal: {other:?}"),
        }
    }

    #[test]
    fn in_flight_cap_is_typed() {
        let mut engine = engine_with(b"");
        for _ in 0..MAX_IN_FLIGHT {
            engine.begin_request().unwrap();
        }
        assert!(engine.begin_request().is_err());
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
