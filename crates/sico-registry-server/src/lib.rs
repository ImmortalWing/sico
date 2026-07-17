//! Bounded read-only HTTP origin for immutable Sico registry trees.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

pub const DEFAULT_MAX_REQUEST_BYTES: usize = 16 * 1024;
pub const DEFAULT_MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
pub const DEFAULT_MAX_CONNECTIONS: usize = 64;

/// Immutable server configuration after the registry root is canonicalized.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    root: PathBuf,
    pub max_request_bytes: usize,
    pub max_file_bytes: u64,
    pub max_connections: usize,
    pub io_timeout: Duration,
}

impl ServerConfig {
    /// Opens a readable registry root with conservative resource limits.
    ///
    /// # Errors
    ///
    /// Returns an error when `root` is missing, not a directory, or cannot be
    /// canonicalized.
    pub fn open(root: &Path) -> io::Result<Self> {
        let root = fs::canonicalize(root)?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "registry root is not a directory",
            ));
        }
        Ok(Self {
            root,
            max_request_bytes: DEFAULT_MAX_REQUEST_BYTES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            io_timeout: Duration::from_secs(5),
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Verifies that the configured root still exists and can be enumerated.
    ///
    /// # Errors
    ///
    /// Returns an error when the origin is not ready to serve registry data.
    pub fn check_ready(&self) -> io::Result<()> {
        let mut entries = fs::read_dir(&self.root)?;
        if let Some(entry) = entries.next() {
            entry?;
        }
        Ok(())
    }
}

/// Serves connections until the listener is closed or its accept loop fails.
///
/// # Errors
///
/// Returns the first listener accept error. Per-connection failures are logged
/// and isolated from the accept loop.
pub fn serve(listener: &TcpListener, config: ServerConfig) -> io::Result<()> {
    let config = Arc::new(config);
    let active = Arc::new(AtomicUsize::new(0));
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        let current = active.fetch_add(1, Ordering::AcqRel);
        if current >= config.max_connections {
            active.fetch_sub(1, Ordering::AcqRel);
            write_error(&mut stream, 503, "Service Unavailable", "origin is busy\n")?;
            continue;
        }
        let config = Arc::clone(&config);
        let active = Arc::clone(&active);
        thread::spawn(move || {
            if let Err(error) = handle_connection(&mut stream, &config) {
                eprintln!("sico-registry: connection failed: {error}");
            }
            active.fetch_sub(1, Ordering::AcqRel);
        });
    }
    Ok(())
}

/// Handles one HTTP connection and then closes it.
///
/// This is public so integration tests and embedding service managers can use
/// the exact production request boundary.
///
/// # Errors
///
/// Returns transport errors while reading the request or writing the response.
pub fn handle_connection(stream: &mut TcpStream, config: &ServerConfig) -> io::Result<()> {
    stream.set_read_timeout(Some(config.io_timeout))?;
    stream.set_write_timeout(Some(config.io_timeout))?;
    let request = match read_request(stream, config.max_request_bytes) {
        Ok(request) => request,
        Err(RequestError::TooLarge) => {
            return write_error(
                stream,
                431,
                "Request Header Fields Too Large",
                "request headers are too large\n",
            );
        }
        Err(RequestError::Invalid) => {
            return write_error(stream, 400, "Bad Request", "invalid request\n");
        }
        Err(RequestError::Io(error)) => return Err(error),
    };

    if request.method != "GET" && request.method != "HEAD" {
        return write_response(
            stream,
            405,
            "Method Not Allowed",
            &[
                ("Content-Type", "text/plain; charset=utf-8"),
                ("Cache-Control", "no-store"),
                ("Allow", "GET, HEAD"),
            ],
            b"method not allowed\n",
            false,
        );
    }
    let head = request.method == "HEAD";
    match request.target.as_str() {
        "/healthz" => write_response(
            stream,
            200,
            "OK",
            &[
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-store"),
            ],
            b"{\"status\":\"ok\"}\n",
            head,
        ),
        "/readyz" => match config.check_ready() {
            Ok(()) => write_response(
                stream,
                200,
                "OK",
                &[
                    ("Content-Type", "application/json"),
                    ("Cache-Control", "no-store"),
                ],
                b"{\"status\":\"ready\"}\n",
                head,
            ),
            Err(_) => write_error(stream, 503, "Service Unavailable", "origin is not ready\n"),
        },
        target if target.starts_with("/v1/") => serve_registry_file(stream, config, target, head),
        _ => write_error(stream, 404, "Not Found", "not found\n"),
    }
}

#[derive(Debug)]
struct Request {
    method: String,
    target: String,
}

#[derive(Debug)]
enum RequestError {
    TooLarge,
    Invalid,
    Io(io::Error),
}

fn read_request(stream: &mut TcpStream, max_bytes: usize) -> Result<Request, RequestError> {
    let mut bytes = Vec::with_capacity(max_bytes.min(4096));
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).map_err(RequestError::Io)?;
        if read == 0 {
            return Err(RequestError::Invalid);
        }
        if bytes.len().saturating_add(read) > max_bytes {
            return Err(RequestError::TooLarge);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    if !bytes.is_ascii() {
        return Err(RequestError::Invalid);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| RequestError::Invalid)?;
    let first = text.split("\r\n").next().ok_or(RequestError::Invalid)?;
    let mut fields = first.split(' ');
    let method = fields.next().ok_or(RequestError::Invalid)?;
    let target = fields.next().ok_or(RequestError::Invalid)?;
    let version = fields.next().ok_or(RequestError::Invalid)?;
    if fields.next().is_some()
        || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
        || method.is_empty()
        || !method.bytes().all(|byte| byte.is_ascii_uppercase())
        || !target.starts_with('/')
        || target.contains(['?', '#', '%', '\\'])
    {
        return Err(RequestError::Invalid);
    }
    Ok(Request {
        method: method.to_owned(),
        target: target.to_owned(),
    })
}

fn serve_registry_file(
    stream: &mut TcpStream,
    config: &ServerConfig,
    target: &str,
    head: bool,
) -> io::Result<()> {
    let relative = &target[4..];
    let relative = Path::new(relative);
    if !valid_relative_path(relative) {
        return write_error(stream, 400, "Bad Request", "invalid registry path\n");
    }
    let requested = config.root.join(relative);
    let Ok(canonical) = fs::canonicalize(&requested) else {
        return write_error(stream, 404, "Not Found", "not found\n");
    };
    if !canonical.starts_with(&config.root) {
        return write_error(stream, 400, "Bad Request", "invalid registry path\n");
    }
    let metadata = match fs::metadata(&canonical) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => return write_error(stream, 404, "Not Found", "not found\n"),
    };
    if metadata.len() > config.max_file_bytes {
        return write_error(
            stream,
            413,
            "Content Too Large",
            "registry object is too large\n",
        );
    }
    let content_type = if canonical.extension().is_some_and(|value| value == "json") {
        "application/json"
    } else {
        "application/octet-stream"
    };
    write_file_response(stream, &canonical, metadata.len(), content_type, head)
}

fn valid_relative_path(path: &Path) -> bool {
    let mut count = 0_usize;
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return false;
        };
        let Some(segment) = segment.to_str() else {
            return false;
        };
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return false;
        }
        count = count.saturating_add(1);
        if count > 16 {
            return false;
        }
    }
    count > 0
}

fn write_file_response(
    stream: &mut TcpStream,
    path: &Path,
    length: u64,
    content_type: &str,
    head: bool,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nContent-Type: {content_type}\r\nCache-Control: public, max-age=31536000, immutable\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n"
    )?;
    if !head {
        let mut file = File::open(path)?;
        io::copy(&mut file, stream)?;
    }
    stream.flush()
}

fn write_error(stream: &mut TcpStream, status: u16, reason: &str, body: &str) -> io::Result<()> {
    write_response(
        stream,
        status,
        reason,
        &[
            ("Content-Type", "text/plain; charset=utf-8"),
            ("Cache-Control", "no-store"),
        ],
        body.as_bytes(),
        false,
    )
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    head: bool,
) -> io::Result<()> {
    write!(stream, "HTTP/1.1 {status} {reason}\r\n")?;
    write!(stream, "Content-Length: {}\r\n", body.len())?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(
        stream,
        "X-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n"
    )?;
    if !head {
        stream.write_all(body)?;
    }
    stream.flush()
}
