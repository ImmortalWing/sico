//! Minimal selected-Runtime boundary for compiler-produced Components.

#![forbid(unsafe_code)]

use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::{Command, ExitStatus},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum RuntimeError {
    TemporaryArtifact(std::io::Error),
    Launch(std::io::Error),
}

#[derive(Debug)]
pub struct RuntimeOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Executes a compiler-produced Component with the selected Wasmtime CLI.
///
/// The Component is written to a process-unique temporary file and removed
/// before this function returns. Runtime stdout, stderr, and status are kept
/// separate so the CLI can preserve its public channel contract.
///
/// # Errors
///
/// Returns an error when the temporary artifact cannot be created or the
/// Runtime process cannot be launched. A Runtime non-zero exit is returned as
/// a normal [`RuntimeOutput`] and is classified by the caller.
pub fn run_component(
    runtime: &OsStr,
    component: &[u8],
    invocation: &str,
) -> Result<RuntimeOutput, RuntimeError> {
    let path = temporary_path();
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(RuntimeError::TemporaryArtifact)?;
    if let Err(error) = file.write_all(component).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&path);
        return Err(RuntimeError::TemporaryArtifact(error));
    }
    drop(file);

    let result = Command::new(runtime)
        .arg("run")
        .arg("--invoke")
        .arg(invocation)
        .arg(&path)
        .output()
        .map_err(RuntimeError::Launch);
    let _ = fs::remove_file(path);
    result.map(|output| RuntimeOutput {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn temporary_path() -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "sico-runtime-{}-{id}.component.wasm",
        std::process::id()
    ))
}
