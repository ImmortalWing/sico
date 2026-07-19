//! `sico-runner`: in-process runner for `sico:script/program@0.1.0` Program
//! Components with the RFC-0029 exit mapping.
//!
//! Usage: `sico-runner [--json] [--fs-read-root PATH]... [--fs-write-root PATH]... PROGRAM.component.wasm [-- ARGS...]`
//!
//! Guest stdin is the process stdin (bounded, 8 MiB). Guest stdout goes to
//! process stdout; everything else is a machine-readable JSON diagnostic on
//! stderr. The process exit code follows RFC-0029: 0–119 guest values,
//! 122 domain error, 123 cancelled, 124 timeout, 125 resource limit/trap,
//! 126 launch failure, 127 incompatible component, and 121 for CLI/IO
//! failures of the runner itself. Scoped filesystem roots are canonicalized
//! once at startup; without them the fs channel fails closed.

use std::io::{Read, Write};
use std::path::PathBuf;

use sico_runner::{CancelToken, FsGrants, RunOutcome, Runner, RunnerLimits, ScriptInput};

const EXIT_TOOL_ERROR: i32 = 121;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut json = false;
    let mut component = None;
    let mut arguments = Vec::new();
    let mut passthrough = false;
    let mut grants = FsGrants::default();
    let mut cancel_after_ms = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        if passthrough {
            arguments.push(argument);
        } else if argument == "--" {
            passthrough = true;
        } else if argument == "--json" {
            json = true;
        } else if argument == "--cancel-after-ms" {
            let Some(value) = args.next() else {
                return diagnostic(json, "cli", "missing value after --cancel-after-ms");
            };
            match value.parse::<u64>() {
                Ok(ms) => cancel_after_ms = Some(ms),
                Err(_) => return diagnostic(json, "cli", "--cancel-after-ms expects milliseconds"),
            }
        } else if argument == "--fs-read-root" || argument == "--fs-write-root" {
            let read = argument == "--fs-read-root";
            let Some(path) = args.next() else {
                return diagnostic(json, "cli", "missing path after fs root flag");
            };
            let canonical = match std::fs::canonicalize(PathBuf::from(&path)) {
                Ok(path) => path,
                Err(error) => {
                    return diagnostic(
                        json,
                        "cli",
                        &format!("cannot canonicalize fs root {path}: {error}"),
                    );
                }
            };
            if !canonical.is_dir() {
                return diagnostic(json, "cli", &format!("fs root is not a directory: {path}"));
            }
            if read {
                grants.read_roots.push(canonical);
            } else {
                grants.write_roots.push(canonical);
            }
        } else if component.is_none() {
            component = Some(argument);
        } else {
            arguments.push(argument);
        }
    }
    let Some(component) = component else {
        return diagnostic(
            json,
            "cli",
            "usage: sico-runner [--json] [--fs-read-root PATH]... [--fs-write-root PATH]... PROGRAM.component.wasm [-- ARGS...]",
        );
    };
    let component = match std::fs::read(&component) {
        Ok(bytes) => bytes,
        Err(error) => {
            return diagnostic(json, "cli", &format!("cannot read component: {error}"));
        }
    };
    let runner = match Runner::new() {
        Ok(runner) => runner,
        Err(error) => return diagnostic(json, "cli", &format!("engine setup failed: {error}")),
    };
    // RFC-0030 mixing rule: each channel is consumed exactly once. When the
    // component imports the streams interface the buffered stdin stays empty
    // and the OS stream belongs to the streaming host calls.
    let streams_component = runner.component_imports_streams(&component);
    let mut stdin = Vec::new();
    if !streams_component && let Err(error) = std::io::stdin().read_to_end(&mut stdin) {
        return diagnostic(json, "cli", &format!("cannot read stdin: {error}"));
    }
    let input = ScriptInput { arguments, stdin };
    let cancel = CancelToken::new();
    if let Some(ms) = cancel_after_ms {
        let token = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            token.cancel();
        });
    }
    let outcome = match runner.run_program(
        &component,
        &input,
        &RunnerLimits::default(),
        &cancel,
        &grants,
    ) {
        Ok(outcome) => outcome,
        Err(violation) => {
            return diagnostic(
                json,
                "resource-limit.input",
                &format!("input bound violated: {violation:?}"),
            );
        }
    };
    report(json, &outcome)
}

fn report(_json: bool, outcome: &RunOutcome) -> i32 {
    if let RunOutcome::Output(output) = outcome {
        let mut stdout = std::io::stdout().lock();
        let mut stderr = std::io::stderr().lock();
        let _ = stdout.write_all(&output.stdout);
        let _ = stdout.flush();
        let _ = stderr.write_all(&output.stderr);
        let _ = stderr.flush();
    } else {
        let class = outcome.class();
        let detail = serde_json::to_string(&serde_json::json!({
            "schema": "sico.runner.outcome.v0",
            "class": class,
            "exit": outcome.exit_code(),
            "detail": format!("{outcome:?}"),
        }))
        .expect("outcome JSON serialization cannot fail");
        let _ = writeln!(std::io::stderr().lock(), "{detail}");
    }
    i32::from(outcome.exit_code())
}

fn diagnostic(json: bool, class: &str, message: &str) -> i32 {
    let _ = json;
    let _ = writeln!(
        std::io::stderr().lock(),
        "{}",
        serde_json::json!({
            "schema": "sico.runner.outcome.v0",
            "class": class,
            "exit": EXIT_TOOL_ERROR,
            "detail": message,
        })
    );
    EXIT_TOOL_ERROR
}
