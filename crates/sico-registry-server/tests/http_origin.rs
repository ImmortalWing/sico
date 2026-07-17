use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

use sico_registry_server::{ServerConfig, handle_connection};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn serves_health_readiness_and_exact_immutable_bytes() {
    let root = temporary_root();
    let object = root.join("records/releases/abc.json");
    fs::create_dir_all(object.parent().unwrap()).unwrap();
    fs::write(&object, b"{\"signed\":true}\n").unwrap();
    let config = ServerConfig::open(&root).unwrap();

    let health = request(&config, "GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(health.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(health.ends_with("{\"status\":\"ok\"}\n"));

    let ready = request(&config, "GET /readyz HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(ready.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(ready.ends_with("{\"status\":\"ready\"}\n"));

    let get = request(
        &config,
        "GET /v1/records/releases/abc.json HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(get.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(get.contains("Cache-Control: public, max-age=31536000, immutable\r\n"));
    assert!(get.contains("Content-Type: application/json\r\n"));
    assert!(get.ends_with("{\"signed\":true}\n"));

    let head = request(
        &config,
        "HEAD /v1/records/releases/abc.json HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(head.ends_with("\r\n\r\n"));
    assert!(!head.contains("{\"signed\""));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fails_closed_on_mutation_traversal_encoding_and_directories() {
    let root = temporary_root();
    fs::create_dir_all(root.join("records")).unwrap();
    let config = ServerConfig::open(&root).unwrap();

    for (request_line, status) in [
        ("POST /healthz HTTP/1.1", "405 Method Not Allowed"),
        ("GET /v1/../secret HTTP/1.1", "400 Bad Request"),
        ("GET /v1/%2e%2e/secret HTTP/1.1", "400 Bad Request"),
        ("GET /v1/records HTTP/1.1", "404 Not Found"),
        ("GET /v1/missing HTTP/1.1", "404 Not Found"),
        ("GET /v1/file?query=1 HTTP/1.1", "400 Bad Request"),
    ] {
        let response = request(
            &config,
            &format!("{request_line}\r\nHost: localhost\r\n\r\n"),
        );
        assert!(response.starts_with(&format!("HTTP/1.1 {status}\r\n")));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn enforces_request_and_file_bounds() {
    let root = temporary_root();
    fs::write(root.join("large.sapp"), b"12345").unwrap();
    let mut config = ServerConfig::open(&root).unwrap();
    config.max_file_bytes = 4;
    config.max_request_bytes = 64;

    let response = request(
        &config,
        "GET /v1/large.sapp HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 413 Content Too Large\r\n"));

    let oversized = format!(
        "GET /healthz HTTP/1.1\r\nX-Fill: {}\r\n\r\n",
        "a".repeat(80)
    );
    let response = request(&config, &oversized);
    assert!(response.starts_with("HTTP/1.1 431 Request Header Fields Too Large\r\n"));
    fs::remove_dir_all(root).unwrap();
}

fn request(config: &ServerConfig, request: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let config = config.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        handle_connection(&mut stream, &config).unwrap();
    });
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    server.join().unwrap();
    response
}

fn temporary_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sico-registry-origin-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    root
}
