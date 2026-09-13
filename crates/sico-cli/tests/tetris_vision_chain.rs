//! M18 pilot 4 / M17 gate 1 (RFC-0043): the tetris recognition chain runs
//! end to end in Sico — a source-level consumer feeds deterministic board
//! captures (BGRA8) through the signed `image-vision@1` package
//! (`occupancy-mask`) and reports the per-column ASCII mask. The consumer
//! needs zero compiler/Runtime patches.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sico-tetris-chain-{tag}-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_sico")
}

fn run<const N: usize>(args: [&str; N], stdin: &[u8]) -> (i32, Vec<u8>, Vec<u8>) {
    let stdin_path = std::env::temp_dir().join(format!(
        "sico-tetris-stdin-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&stdin_path, stdin).unwrap();
    let stdin_file = fs::File::open(&stdin_path).unwrap();
    let mut command = Command::new(binary());
    command
        .args(args)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if std::env::var_os("SICO_RUNNER").is_none() {
        let runner = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../runner/sico-runner/target/debug/sico-runner.exe");
        if runner.is_file() {
            command.env("SICO_RUNNER", runner);
        }
    }
    let output = command.output().unwrap();
    let _ = fs::remove_file(&stdin_path);
    (
        output.status.code().unwrap_or(127),
        output.stdout,
        output.stderr,
    )
}

fn stage_pilot(tag: &str) -> PathBuf {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pilot = repository.join("pilots/tetris-vision");
    let dir = temp_root(tag);
    for file in ["board-reader.sico", "sico-lock.json"] {
        fs::copy(pilot.join(file), dir.join(file)).unwrap();
    }
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    fs::copy(
        pilot.join("pkgs/image_vision.sapp"),
        dir.join("pkgs/image_vision.sapp"),
    )
    .unwrap();
    dir
}

#[test]
fn tetris_chain_reads_board_masks() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = stage_pilot("masks");
    let cases = [
        ("board-a.bgra", "width=8; mask=01001010"),
        ("board-b.bgra", "width=8; mask=10000001"),
        ("board-empty.bgra", "width=8; mask=00000000"),
    ];
    for (fixture, expected) in cases {
        let capture = fs::read(repository.join("pilots/tetris-vision/cases").join(fixture))
            .expect("board fixture");
        let (exit, stdout, stderr) = run(
            ["run", &dir.join("board-reader.sico").display().to_string()],
            &capture,
        );
        assert_eq!(
            exit,
            0,
            "{fixture}: stderr={} stdout={}",
            String::from_utf8_lossy(&stderr),
            String::from_utf8_lossy(&stdout)
        );
        assert_eq!(
            String::from_utf8_lossy(&stdout),
            expected,
            "{fixture} mask byte-exact"
        );
    }
}

#[test]
fn tetris_chain_fails_closed_on_empty_capture() {
    let dir = stage_pilot("empty");
    let (exit, stdout, stderr) = run(
        ["run", &dir.join("board-reader.sico").display().to_string()],
        b"",
    );
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    let text = String::from_utf8_lossy(&stderr);
    assert!(text.contains("empty board capture"), "{text}");
}
