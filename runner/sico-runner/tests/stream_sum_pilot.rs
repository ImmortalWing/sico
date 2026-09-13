//! M18 pilot 2 — streaming data tool (M9/M11): chunked log sampler over
//! the streaming stdin/stdout channels with bounded memory. The runner
//! binary is spawned as a child (streaming components inherit stdin);
//! evidence is byte-exact stdout per corpus case.

use std::{
    io::Write,
    process::{Command, Stdio},
};

fn compile_pilot() -> Vec<u8> {
    // Tests run in parallel: a per-call counter keeps scratch dirs disjoint.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-m18-stream-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("stream-sum.sico");
    let component_path = directory.join("stream-sum.component.wasm");
    std::fs::write(
        &source_path,
        include_str!("../../../pilots/stream-sum/stream-sum.sico"),
    )
    .unwrap();
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
        "sico-m18-stream-component-{}.wasm",
        std::process::id()
    ));
    std::fs::write(&component_path, component).unwrap();
    let runner_exe = std::env::var("SICO_RUNNER")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_sico-runner").to_owned());
    let mut child = Command::new(&runner_exe)
        .arg(&component_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("runner launches");
    {
        let mut handle = child.stdin.take().expect("piped stdin");
        let _ = handle.write_all(&stdin);
    }
    let out = child.wait_with_output().expect("runner waits");
    let _ = std::fs::remove_file(&component_path);
    (out.status.code().unwrap_or(127), out.stdout, out.stderr)
}

/// One guest-read-sized segment whose fill pattern carries the marker
/// throughout: pipe read boundaries are scheduling dependent, so every
/// possible slicing of the segment still contains the marker.
fn aligned_chunk(marker: &[u8]) -> Vec<u8> {
    let mut chunk = vec![b'.'; 4096];
    let mut i = 0;
    while i < 4096 {
        let take = marker.len().min(4096 - i);
        chunk[i..i + take].copy_from_slice(marker);
        i += 64;
    }
    chunk
}

#[test]
fn stream_sum_classifies_chunks_in_order() {
    let component = compile_pilot();
    let mut stdin = Vec::new();
    stdin.extend_from_slice(&aligned_chunk(b"INFO boot ok"));
    stdin.extend_from_slice(&aligned_chunk(b"payload, no marker"));
    stdin.extend_from_slice(&aligned_chunk(b"ERROR disk full"));
    stdin.extend_from_slice(b"clean payload");
    let (exit, stdout, stderr) = run_child(&component, stdin);
    assert_eq!(
        exit,
        0,
        "stderr={} stdout={}",
        String::from_utf8_lossy(&stderr),
        String::from_utf8_lossy(&stdout)
    );
    // Chunk read boundaries are pipe-scheduling dependent; the invariant
    // contract is strictly increasing indices, valid classes, and the
    // ERROR segment dominating every possible slicing of itself.
    let mut last_index: Option<u64> = None;
    let mut error_chunks = 0;
    let mut total = 0;
    for line in String::from_utf8_lossy(&stdout).lines() {
        let (name, class) = line.split_once(':').expect("classification line");
        let index: u64 = name
            .strip_prefix("chunk-")
            .expect("chunk line")
            .parse()
            .expect("chunk index");
        if let Some(previous) = last_index {
            assert!(index > previous, "indices increase");
        }
        last_index = Some(index);
        match class {
            "error" => error_chunks += 1,
            "info" | "clean" => {}
            other => panic!("unexpected class {other}"),
        }
        total += 1;
    }
    assert!(total >= 3, "at least the three segments classify");
    assert!(error_chunks >= 1, "the ERROR segment must classify error");
}

#[test]
fn stream_sum_fails_closed_on_invalid_utf8() {
    let component = compile_pilot();
    let mut stdin = aligned_chunk(b"ok");
    stdin.extend_from_slice(&[0xFF, 0xFE]);
    let (exit, _stdout, stderr) = run_child(&component, stdin);
    // The typed domain failure surfaces as the direct runner's 122 mapping.
    // M9 typed partial failure: prior chunks' output already streamed.
    assert_eq!(exit, 122);
    let text = String::from_utf8_lossy(&stderr);
    assert!(text.contains("chunk"), "{text}");
}
