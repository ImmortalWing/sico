//! `sico test`: deterministic discovery and bounded execution of Script
//! test pairs (STEP-0140). A test is a `NAME.sico` source beside a
//! `NAME.test.json` manifest:
//!
//! ```json
//! {"stdin": "", "args": [], "expect_exit": 0, "expect_stdout": "ok\n"}
//! ```
//!
//! Every field is optional; unknown fields are rejected. Discovery sorts
//! paths so runs are reproducible; every test builds through the same
//! cache identity as `sico run` and executes through the real
//! `sico-runner` under its default bounded limits. v0 covers the buffered
//! profile: components importing `sico:script/streams@0.1.0` fail the test
//! with a typed reason because the runner would inherit the harness stdio.

#![forbid(unsafe_code)]

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Deserialize;

use crate::EXIT_TOOL_ERROR;
use crate::run::{CacheIdentity, find_runner};

pub const EXIT_TESTS_FAILED: i32 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TestManifest {
    #[serde(default)]
    stdin: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    expect_exit: i32,
    #[serde(default)]
    expect_stdout: String,
}

pub fn test_command() -> Command {
    Command::new("test")
        .about(
            "Discover NAME.sico + NAME.test.json pairs and run them through \
             sico-runner under default bounded limits",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .default_value(".")
                .help("A .sico file or a directory scanned recursively (sorted)")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit JSON tool diagnostics on stderr")
                .action(ArgAction::SetTrue),
        )
}

#[allow(clippy::too_many_lines)]
pub fn run_test(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let _json = matches.get_flag("json");
    let root = Path::new(matches.get_one::<String>("path").unwrap());
    let mut sources = Vec::new();
    if root.is_file() {
        sources.push(root.to_path_buf());
    } else if root.is_dir() {
        collect_sources(root, &mut sources);
        sources.sort();
    } else {
        let _ = writeln!(stderr, "sico test: path not found: {}", root.display());
        return EXIT_TOOL_ERROR;
    }
    let mut tests = Vec::new();
    for source in sources {
        let manifest_path = source.with_extension("test.json");
        if !manifest_path.is_file() {
            continue;
        }
        tests.push((source, manifest_path));
    }
    if tests.is_empty() {
        let _ = writeln!(
            stdout,
            "sico test: no NAME.sico + NAME.test.json pairs found"
        );
        return EXIT_TESTS_FAILED;
    }
    let Some(runner) = find_runner() else {
        let _ = writeln!(stderr, "sico test: sico-runner executable not found");
        return EXIT_TOOL_ERROR;
    };
    let mut passed = 0_usize;
    let mut failed = 0_usize;
    for (source, manifest_path) in tests {
        let name = source.display().to_string();
        let manifest_text = match fs::read_to_string(&manifest_path) {
            Ok(text) => text,
            Err(error) => {
                failed += 1;
                let _ = writeln!(stdout, "FAIL {name} (manifest unreadable: {error})");
                continue;
            }
        };
        let manifest: TestManifest = match serde_json::from_str(&manifest_text) {
            Ok(manifest) => manifest,
            Err(error) => {
                failed += 1;
                let _ = writeln!(stdout, "FAIL {name} (manifest invalid: {error})");
                continue;
            }
        };
        let source_bytes = match fs::read(&source) {
            Ok(bytes) => bytes,
            Err(error) => {
                failed += 1;
                let _ = writeln!(stdout, "FAIL {name} (source unreadable: {error})");
                continue;
            }
        };
        // RFC-0039 (STEP-0143): assemble before the cache identity so module
        // imports contribute to the key; failures fail the test like any
        // other build failure.
        let assembled =
            match crate::modules::assemble_from_bytes(&source.display().to_string(), &source_bytes)
            {
                Ok(assembled) => assembled,
                Err(crate::modules::AssembleError::Diagnostics(lines)) => {
                    failed += 1;
                    for line in &lines {
                        let _ = writeln!(stdout, "FAIL {name} ({line})");
                    }
                    continue;
                }
                Err(_) => {
                    failed += 1;
                    let _ = writeln!(stdout, "FAIL {name} (build failed)");
                    continue;
                }
            };
        let module_blob = assembled.cache_blob();
        let identity = match CacheIdentity::new(&source_bytes, &module_blob) {
            Ok(identity) => identity,
            Err(message) => {
                failed += 1;
                let _ = writeln!(stdout, "FAIL {name} ({message})");
                continue;
            }
        };
        let component = match identity.cached_component_assembled(
            &assembled,
            false,
            &mut Vec::new(),
            &mut Vec::new(),
        ) {
            Ok(path) => path,
            Err(exit) => {
                failed += 1;
                let _ = writeln!(stdout, "FAIL {name} (build failed, exit {exit})");
                continue;
            }
        };
        if crate::run::component_imports_streams(&component) {
            failed += 1;
            let _ = writeln!(
                stdout,
                "FAIL {name} (streaming components are outside sico test v0)"
            );
            continue;
        }
        match execute_test(&runner, &component, &manifest) {
            Ok(()) => {
                passed += 1;
                let _ = writeln!(stdout, "PASS {name}");
            }
            Err(reason) => {
                failed += 1;
                let _ = writeln!(stdout, "FAIL {name} ({reason})");
            }
        }
    }
    let _ = writeln!(stdout, "{passed} passed, {failed} failed");
    if failed == 0 { 0 } else { EXIT_TESTS_FAILED }
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "sico") {
            out.push(path);
        }
    }
}

fn execute_test(runner: &Path, component: &Path, manifest: &TestManifest) -> Result<(), String> {
    let mut child = ProcessCommand::new(runner)
        .arg(component)
        .arg("--")
        .args(&manifest.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot launch sico-runner: {error}"))?;
    let stdin_handle = child.stdin.take().expect("piped stdin");
    let stdout_handle = child.stdout.take().expect("piped stdout");
    let stdin_bytes = manifest.stdin.clone().into_bytes();
    let writer = std::thread::spawn(move || {
        let mut handle = stdin_handle;
        let _ = handle.write_all(&stdin_bytes);
    });
    let mut actual = Vec::new();
    {
        let mut handle = stdout_handle;
        handle
            .read_to_end(&mut actual)
            .map_err(|error| format!("cannot read runner stdout: {error}"))?;
    }
    let status = child
        .wait()
        .map_err(|error| format!("runner wait failed: {error}"))?;
    let _ = writer.join();
    let exit = status
        .code()
        .unwrap_or(crate::run::EXIT_RUNNER_INCOMPATIBLE);
    if exit != manifest.expect_exit {
        return Err(format!("exit {exit}, expected {}", manifest.expect_exit));
    }
    if actual != manifest.expect_stdout.as_bytes() {
        return Err(format!(
            "stdout mismatch: {} bytes, expected {} bytes",
            actual.len(),
            manifest.expect_stdout.len()
        ));
    }
    Ok(())
}
