//! Secure HTTP Host provider (M12): TLS transport, endpoint authority,
//! DNS policy, streaming bodies, and Host-owned secrets.
//!
//! STEP-0111/0112 foundation: deterministic certificate fixtures plus the
//! rustls transport with strict verification. Nothing here disables
//! certificate checks or widens grants; policy functions return typed
//! errors for every refusal.

pub mod authority;
pub mod engine;
pub mod framing;
pub mod redirect;
pub mod secrets;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, pem::PemObject};

/// Deterministic in-memory certificate authority plus a leaf for `host`.
pub struct CaFixture {
    pub ca_pem: Vec<u8>,
    pub leaf_pem: Vec<u8>,
    pub leaf_key_pem: Vec<u8>,
    pub host: String,
}

/// Builds a fresh self-signed CA and a leaf certificate for `host`.
///
/// # Panics
///
/// Panics only on rcgen key/parameter construction failures, which are
/// library-internal and not input-dependent.
#[must_use]
pub fn deterministic_ca(host: &str) -> CaFixture {
    let ca_params =
        rcgen::CertificateParams::new(vec!["Sico Test CA".to_owned()]).expect("CA parameters");
    let ca_key = rcgen::KeyPair::generate().expect("CA key");
    let ca_cert = ca_params.self_signed(&ca_key).expect("CA self-sign");

    let leaf_params = rcgen::CertificateParams::new(vec![host.to_owned()]).expect("leaf params");
    let leaf_key = rcgen::KeyPair::generate().expect("leaf key");
    let leaf_cert = leaf_params
        .signed_by(&leaf_key, &ca_cert, &ca_key)
        .expect("leaf signing");

    CaFixture {
        ca_pem: ca_cert.pem().into_bytes(),
        leaf_pem: leaf_cert.pem().into_bytes(),
        leaf_key_pem: leaf_key.serialize_pem().into_bytes(),
        host: host.to_owned(),
    }
}

/// A one-shot TLS echo server bound to an ephemeral loopback port.
pub struct TlsEchoServer {
    pub port: u16,
    server: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl TlsEchoServer {
    /// Starts a TLS server answering exactly one echo roundtrip with the
    /// fixture leaf. Deterministic: no external trust, no persistence.
    ///
    /// # Panics
    ///
    /// Panics on bind/accept failure, which the caller controls.
    #[must_use]
    pub fn start(fixture: &CaFixture) -> Self {
        let leaf_cert = CertificateDer::from_pem_slice(&fixture.leaf_pem).expect("leaf PEM parses");
        let leaf_key =
            PrivateKeyDer::from_pem_slice(&fixture.leaf_key_pem).expect("leaf key PEM parses");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("ephemeral bind");
        let port = listener.local_addr().expect("local addr").port();
        let config = Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![leaf_cert], leaf_key)
                .expect("server cert/key pair"),
        );
        let server = std::thread::spawn(move || {
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                        break stream;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() > deadline {
                            return Err("no client connected within 15s".to_owned());
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(error) => return Err(error.to_string()),
                }
            };
            let mut conn = rustls::ServerConnection::new(config).map_err(|e| e.to_string())?;
            let mut sock = stream;
            {
                let mut tls = rustls::Stream::new(&mut conn, &mut sock);
                let mut chunk = [0_u8; 4096];
                let read = tls.read(&mut chunk).map_err(|e| e.to_string())?;
                tls.write_all(&chunk[..read]).map_err(|e| e.to_string())?;
                tls.flush().map_err(|e| e.to_string())?;
            }
            conn.send_close_notify();
            Ok(())
        });
        Self {
            port,
            server: Some(server),
        }
    }

    /// Joins the server thread; a failed echo is an error, not a panic.
    ///
    /// # Errors
    ///
    /// Returns the server thread's echo error, or `no client connected
    /// within 15s` when no client ever arrived (the refusal-fixture case).
    ///
    /// # Panics
    ///
    /// Panics only if the server thread itself panicked.
    pub fn finish(&mut self) -> Result<(), String> {
        self.server
            .take()
            .expect("server not yet joined")
            .join()
            .expect("server thread")
    }
}

/// Outcome of one verifying client request against a TLS endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsProbeOutcome {
    /// Handshake and exchange succeeded; payload is the echoed bytes.
    Connected(String),
    /// The handshake was refused; the detail names the verification
    /// failure class (hostname, unknown issuer, expired, malformed…).
    Refused(String),
}

/// Runs one verifying client exchange against `host:port` trusting only
#[must_use]
/// `ca_pem`. Every certificate failure surfaces as `Refused` — there is
/// no insecure fallback in this crate.
pub fn tls_probe(host: &str, port: u16, ca_pem: &[u8], payload: &[u8]) -> TlsProbeOutcome {
    let run = || -> Result<String, String> {
        let mut roots = rustls::RootCertStore::empty();
        let certificate = CertificateDer::from_pem_slice(ca_pem).map_err(|e| e.to_string())?;
        roots.add(certificate).map_err(|e| e.to_string())?;
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let server_name = ServerName::try_from(host.to_owned()).map_err(|e| e.to_string())?;
        let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| e.to_string())?;
        let mut sock = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        sock.set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        {
            let mut tls = rustls::Stream::new(&mut conn, &mut sock);
            tls.write_all(payload).map_err(|e| e.to_string())?;
            tls.flush().map_err(|e| e.to_string())?;
            loop {
                let mut chunk = [0_u8; 4096];
                match tls.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => buffer.extend_from_slice(&chunk[..read]),
                }
            }
        }
        String::from_utf8(buffer).map_err(|e| e.to_string())
    };
    match run() {
        Ok(reply) => TlsProbeOutcome::Connected(reply),
        Err(error) => TlsProbeOutcome::Refused(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_chain_completes_a_verifying_roundtrip() {
        let fixture = deterministic_ca("localhost");
        let mut server = TlsEchoServer::start(&fixture);
        let outcome = tls_probe("localhost", server.port, &fixture.ca_pem, b"ping");
        assert_eq!(outcome, TlsProbeOutcome::Connected("ping".to_owned()));
        server.finish().unwrap();
    }

    #[test]
    fn wrong_hostname_is_refused() {
        let fixture = deterministic_ca("localhost");
        let mut server = TlsEchoServer::start(&fixture);
        // The IP literal is not in the leaf's SAN; verification must fail.
        let outcome = tls_probe("127.0.0.1", server.port, &fixture.ca_pem, b"ping");
        assert!(
            matches!(outcome, TlsProbeOutcome::Refused(_)),
            "{outcome:?}"
        );
        let _ = server.finish();
    }

    #[test]
    fn untrusted_issuer_is_refused() {
        let fixture = deterministic_ca("localhost");
        let other = deterministic_ca("localhost");
        let mut server = TlsEchoServer::start(&fixture);
        // Trusting the wrong CA: chain must not validate.
        let outcome = tls_probe("localhost", server.port, &other.ca_pem, b"ping");
        assert!(
            matches!(outcome, TlsProbeOutcome::Refused(_)),
            "{outcome:?}"
        );
        let _ = server.finish();
    }

    #[test]
    fn malformed_ca_pem_is_refused_before_any_connection() {
        // PEM parsing fails during trust-root construction, before any TCP
        // connect: no server is needed, and the refusal must be immediate.
        let started = std::time::Instant::now();
        let outcome = tls_probe("localhost", 1, b"not a pem", b"ping");
        assert!(
            matches!(outcome, TlsProbeOutcome::Refused(_)),
            "{outcome:?}"
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
}
