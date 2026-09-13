//! M14 STEP-0139: the streaming transform acceptance application
//! (`tests/end-to-end/stream-transform.sico`) through the real runner
//! binary. The guest reads stdin in fixed 4 KiB chunks — guest memory
//! stays bounded by the chunk regardless of input size — numbers each
//! chunk's lines, writes them to the output stream, and finishes with a
//! deterministic `total=` marker. Non-UTF-8 input fails with a typed
//! partial-failure message (exit 122, `domain-error`).
//!
//! `sico.stream.*` reads the runner process's own stdio (RFC-0030), so
//! both scenarios drive `sico-runner` as a child process with piped
//! stdio. Typed stream-read cancellation is host-side (`send_cancellable`
//! / `recv_cancellable`) and is covered by the existing typed
//! cancellation fixtures.

use std::io::{Read as _, Write as _};
use std::process::{Command, Stdio};

const STREAM_SOURCE: &str = include_str!("../../../tests/end-to-end/stream-transform.sico");

const CHUNK: usize = 4096;
const LINE_LEN: usize = 63;

fn compile_source(tag: u32) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0139-stream-{}-{tag}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("stream-transform.sico");
    let component_path = directory.join("stream-transform.component.wasm");
    std::fs::write(&source_path, STREAM_SOURCE).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component_path.as_os_str().to_owned(),
            source_path.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

fn run_child(component: &[u8], stdin: Vec<u8>) -> (i32, Vec<u8>, Vec<u8>) {
    let component_path = std::env::temp_dir().join(format!(
        "sico-step0139-run-{}.component.wasm",
        std::process::id()
    ));
    std::fs::write(&component_path, component).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_sico-runner"))
        .arg(&component_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sico-runner spawns");
    let mut child = child;
    let stdin_handle = child.stdin.take().expect("piped stdin");
    let stdout_handle = child.stdout.take().expect("piped stdout");
    let stderr_handle = child.stderr.take().expect("piped stderr");
    let writer = std::thread::spawn(move || {
        let mut handle = stdin_handle;
        // A child that exits early closes the pipe; the outcome assertions
        // below report the real failure, so swallow BrokenPipe here.
        let _ = handle.write_all(&stdin);
    });
    let reader = std::thread::spawn(move || {
        let mut handle = stdout_handle;
        let mut buffer = Vec::new();
        handle.read_to_end(&mut buffer).expect("stdout read");
        buffer
    });
    let err_reader = std::thread::spawn(move || {
        let mut handle = stderr_handle;
        let mut buffer = Vec::new();
        handle.read_to_end(&mut buffer).expect("stderr read");
        buffer
    });
    let exit = child.wait().expect("child exits").code().unwrap_or(-1);
    let stdout = reader.join().expect("stdout reader");
    let stderr = err_reader.join().expect("stderr reader");
    writer.join().expect("stdin writer");
    let _ = std::fs::remove_file(&component_path);
    (exit, stdout, stderr)
}

#[test]
fn stream_transform_handles_one_mebibyte_with_bounded_chunks() {
    let component = compile_source(1);
    // 256 lines of 4096 bytes (4095 + LF): every 4 KiB chunk carries exactly
    // one line, so the 1 MiB run fits the runner's default fuel budget while
    // guest memory stays bounded by a single chunk.
    let lines = 256;
    let mut stdin = Vec::with_capacity(lines * CHUNK);
    let mut expected = Vec::new();
    for i in 0..lines {
        let mut l = format!("line-{i:06}-");
        while l.len() < CHUNK - 1 {
            l.push('y');
        }
        stdin.extend_from_slice(l.as_bytes());
        stdin.push(b'\n');
        expected.extend_from_slice(l.as_bytes());
        expected.push(b'\n');
        expected.extend_from_slice(format!("chunk {i}\n").as_bytes());
    }
    expected.extend_from_slice(b"total=1048576\n");
    let (exit, stdout, stderr) = run_child(&component, stdin);
    assert_eq!(
        exit,
        0,
        "guest exit; stderr: {} stdout-prefix: {}",
        String::from_utf8_lossy(&stderr),
        String::from_utf8_lossy(&stdout[..stdout.len().min(120)])
    );
    assert_eq!(stdout, expected, "1 MiB stream must round-trip");
}

#[test]
fn stream_transform_reports_typed_partial_failure() {
    let component = compile_source(2);
    let mut stdin = format!("line-000000-{}", "x".repeat(LINE_LEN - 12)).into_bytes();
    stdin.push(b'\n');
    stdin.push(0xFF);
    stdin.extend_from_slice(b"tail\n");
    let (exit, _stdout, stderr) = run_child(&component, stdin);
    assert_eq!(exit, 122, "domain errors exit 122");
    let text = String::from_utf8_lossy(&stderr);
    assert!(
        text.contains("\"class\":\"domain-error\"") && text.contains("bad utf8 at chunk 0"),
        "typed partial failure expected, got: {text}"
    );
}
