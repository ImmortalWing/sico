//! `sico-runner`: in-process runner for `sico:script/program@0.1.0` Program
//! Components with the RFC-0029 exit mapping.
//!
//! Usage: `sico-runner [--json] [--fs-read-root PATH]... [--fs-write-root PATH]... [--allow-net HOST:PORT]... PROGRAM.component.wasm [-- ARGS...]`
//!
//! Guest stdin is the process stdin (bounded, 8 MiB). Guest stdout goes to
//! process stdout; everything else is a machine-readable JSON diagnostic on
//! stderr. The process exit code follows RFC-0029: 0–119 guest values,
//! 122 domain error, 123 cancelled, 124 timeout, 125 resource limit/trap,
//! 126 launch failure, 127 incompatible component, and 121 for CLI/IO
//! failures of the runner itself. Scoped filesystem roots are canonicalized
//! once at startup; without them the fs channel fails closed.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const EXIT_TOOL_ERROR: i32 = 121;
const MAX_WATCH_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;
const WATCH_DEBOUNCE: Duration = Duration::from_millis(100);

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut json = false;
    let mut component = None;
    let mut arguments = Vec::new();
    let mut passthrough = false;
    let mut grants = FsGrants::default();
    let mut net = NetGrants::default();
    let mut cancel_after_ms = None;
    let mut watch = false;
    let mut watch_runs = None;
    let mut watch_poll_ms = 25_u64;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        if passthrough {
            arguments.push(argument);
        } else if argument == "--" {
            passthrough = true;
        } else if argument == "--json" {
            json = true;
        } else if argument == "--watch" {
            watch = true;
        } else if argument == "--watch-runs" {
            let Some(value) = args.next() else {
                return diagnostic(json, "cli", "missing value after --watch-runs");
            };
            match value.parse::<u32>() {
                Ok(runs) if runs > 0 => watch_runs = Some(runs),
                _ => return diagnostic(json, "cli", "--watch-runs expects a positive integer"),
            }
        } else if argument == "--watch-poll-ms" {
            let Some(value) = args.next() else {
                return diagnostic(json, "cli", "missing value after --watch-poll-ms");
            };
            match value.parse::<u64>() {
                Ok(ms) if (5..=1000).contains(&ms) => watch_poll_ms = ms,
                _ => return diagnostic(json, "cli", "--watch-poll-ms expects 5..=1000"),
            }
        } else if argument == "--cancel-after-ms" {
            let Some(value) = args.next() else {
                return diagnostic(json, "cli", "missing value after --cancel-after-ms");
            };
            match value.parse::<u64>() {
                Ok(ms) => cancel_after_ms = Some(ms),
                Err(_) => return diagnostic(json, "cli", "--cancel-after-ms expects milliseconds"),
            }
        } else if argument == "--allow-net" {
            let Some(endpoint) = args.next() else {
                return diagnostic(json, "cli", "missing endpoint after --allow-net");
            };
            if let Err(error) = net.grant(&endpoint) {
                return diagnostic(json, "cli", &format!("invalid --allow-net: {error}"));
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
            "usage: sico-runner [--watch] [--json] [--fs-read-root PATH]... [--fs-write-root PATH]... [--allow-net HOST:PORT]... PROGRAM.component.wasm [-- ARGS...]",
        );
    };
    if !watch && watch_runs.is_some() {
        return diagnostic(json, "cli", "--watch-runs requires --watch");
    }
    let component_path = PathBuf::from(component);
    let component = match read_component(&component_path) {
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
    if watch {
        if streams_component {
            return diagnostic(
                json,
                "incompatible",
                "watch v0 refuses streaming stdin; use buffered Script input",
            );
        }
        return watch_program(
            &runner,
            &component_path,
            component,
            &input,
            &grants,
            &net,
            json,
            watch_runs,
            Duration::from_millis(watch_poll_ms),
            cancel_after_ms,
        );
    }
    let cancel = CancelToken::new();
    if let Some(ms) = cancel_after_ms {
        let token = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            token.cancel();
        });
    }
    let outcome = match runner.run_program_with_net(
        &component,
        &input,
        &RunnerLimits::default(),
        &cancel,
        &grants,
        &net,
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

#[allow(clippy::too_many_arguments)]
fn watch_program(
    runner: &Runner,
    path: &Path,
    initial: Vec<u8>,
    input: &ScriptInput,
    grants: &FsGrants,
    net: &NetGrants,
    json: bool,
    max_runs: Option<u32>,
    poll: Duration,
    cancel_after_ms: Option<u64>,
) -> i32 {
    let limits = RunnerLimits::default();
    let mut prepared = match runner.prepare_program_with_net(&initial, grants, net) {
        Ok(prepared) => prepared,
        Err(outcome) => return report(json, &outcome),
    };
    let mut accepted = initial;
    let mut pending: Option<(Vec<u8>, Instant)> = None;
    let mut generation = 1_u32;
    let mut last_exit =
        run_watch_generation(&prepared, input, &limits, json, generation, cancel_after_ms);
    if max_runs.is_some_and(|runs| generation >= runs) {
        return last_exit;
    }

    loop {
        std::thread::sleep(poll);
        let Ok(candidate) = read_component(path) else {
            continue;
        };
        if candidate == accepted {
            pending = None;
            continue;
        }
        match &mut pending {
            Some((bytes, since)) if *bytes == candidate => {
                if since.elapsed() < WATCH_DEBOUNCE {
                    continue;
                }
            }
            _ => {
                pending = Some((candidate, Instant::now()));
                continue;
            }
        }
        let (candidate, _) = pending.take().expect("stable pending generation");
        accepted.clone_from(&candidate);
        match runner.prepare_program_with_net(&candidate, grants, net) {
            Ok(next) => prepared = next,
            Err(outcome) => {
                watch_event("generation-rejected", generation + 1, outcome.exit_code());
                continue;
            }
        }
        generation += 1;
        last_exit =
            run_watch_generation(&prepared, input, &limits, json, generation, cancel_after_ms);
        if max_runs.is_some_and(|runs| generation >= runs) {
            return last_exit;
        }
    }
}

fn run_watch_generation(
    prepared: &sico_runner::PreparedProgram,
    input: &ScriptInput,
    limits: &RunnerLimits,
    json: bool,
    generation: u32,
    cancel_after_ms: Option<u64>,
) -> i32 {
    let cancel = CancelToken::new();
    if let Some(ms) = cancel_after_ms {
        let token = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(ms));
            token.cancel();
        });
    }
    let outcome = prepared
        .run(input, limits, &cancel)
        .unwrap_or(RunOutcome::MemoryLimit);
    let exit = report(json, &outcome);
    watch_event("run-complete", generation, outcome.exit_code());
    exit
}

fn watch_event(event: &str, generation: u32, exit: u8) {
    let _ = writeln!(
        std::io::stderr().lock(),
        "{}",
        serde_json::json!({
            "schema": "sico.runner.watch.v0",
            "event": event,
            "pid": std::process::id(),
            "generation": generation,
            "exit": exit,
        })
    );
}

fn read_component(path: &Path) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_WATCH_COMPONENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_WATCH_COMPONENT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "component exceeds the 64 MiB watch bound",
        ));
    }
    Ok(bytes)
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
