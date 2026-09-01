//! Bounded expression REPL session for STEP-0091.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::run::eval_constant;
use crate::{EXIT_SUCCESS, EXIT_TOOL_ERROR};

pub const MAX_REPL_CELLS: usize = 256;
pub const MAX_REPL_CELL_BYTES: usize = 4 * 1024;
pub const MAX_REPL_HISTORY_BYTES: usize = 16 * 1024;
const MAX_EXPORT_PATH_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Cell {
    id: String,
    source: String,
    value: i64,
}

#[derive(Default)]
struct ReplSession {
    cells: Vec<Cell>,
    source_bytes: usize,
}

impl ReplSession {
    fn submit(&mut self, source: &str) -> Result<&Cell, String> {
        if source.is_empty() {
            return Err("empty cell".to_owned());
        }
        if source.len() > MAX_REPL_CELL_BYTES {
            return Err("cell exceeds 4 KiB".to_owned());
        }
        if self.cells.len() == MAX_REPL_CELLS {
            return Err("session exceeds 256 cells".to_owned());
        }
        if self.source_bytes.saturating_add(source.len()) > MAX_REPL_HISTORY_BYTES {
            return Err("session source history exceeds 16 KiB".to_owned());
        }

        let value = eval_constant(source)?;
        let ordinal = self.cells.len() + 1;
        let id = cell_id(ordinal, source);
        self.cells.push(Cell {
            id,
            source: source.to_owned(),
            value,
        });
        self.source_bytes += source.len();
        Ok(self.cells.last().expect("submitted cell exists"))
    }

    fn reset(&mut self) {
        self.cells.clear();
        self.source_bytes = 0;
    }

    fn export(&self) -> Result<Vec<u8>, String> {
        self.verify_replay()?;
        serde_json::to_vec_pretty(&json!({
            "schema": "sico.repl.session.v0",
            "cells": self.cells.iter().map(|cell| json!({
                "id": cell.id,
                "source": cell.source,
                "value": cell.value,
            })).collect::<Vec<_>>(),
        }))
        .map_err(|error| format!("cannot serialize session: {error}"))
    }

    fn verify_replay(&self) -> Result<(), String> {
        for cell in &self.cells {
            let replayed = eval_constant(&cell.source)?;
            if replayed != cell.value {
                return Err(format!("replay diverged at {}", cell.id));
            }
        }
        Ok(())
    }
}

fn cell_id(ordinal: usize, source: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"sico.repl.cell.v0\0");
    digest.update((ordinal as u64).to_le_bytes());
    digest.update(source.as_bytes());
    let digest = digest.finalize();
    let mut id = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(id, "{byte:02x}").expect("writing to String cannot fail");
    }
    id
}

pub fn repl_command() -> Command {
    Command::new("repl")
        .about("Evaluate bounded Int expression cells with deterministic replay")
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit sico.repl.event.v0 JSON lines")
                .action(ArgAction::SetTrue),
        )
}

pub fn run_repl(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let json = matches.get_flag("json");
    let mut session = ReplSession::default();
    loop {
        let line = match read_bounded_line(stdin) {
            Ok(Some(line)) => line,
            Ok(None) => return EXIT_SUCCESS,
            Err(LineError::Rejected(message)) => {
                if !emit_error(stderr, json, &message) {
                    return EXIT_TOOL_ERROR;
                }
                continue;
            }
            Err(LineError::Io(error)) => {
                let _ = emit_error(stderr, json, &format!("stdin read failed: {error}"));
                return EXIT_TOOL_ERROR;
            }
        };
        let source = line.trim();
        if source.is_empty() {
            continue;
        }
        let emitted = match source {
            ":quit" => return EXIT_SUCCESS,
            ":reset" => {
                session.reset();
                emit_event(stdout, json, "reset", &json!({"cells": 0}))
            }
            ":history" => match session.verify_replay() {
                Ok(()) => emit_history(stdout, json, &session),
                Err(error) => emit_error(stderr, json, &error),
            },
            _ if source.starts_with(":export ") => {
                let path = source[8..].trim();
                match export_session(&session, path) {
                    Ok(()) => emit_event(stdout, json, "export", &json!({"path": path})),
                    Err(error) => emit_error(stderr, json, &error),
                }
            }
            _ if source.starts_with(':') => emit_error(stderr, json, "unknown REPL command"),
            _ => match session.submit(source) {
                Ok(cell) => emit_event(
                    stdout,
                    json,
                    "cell",
                    &json!({"id": cell.id, "value": cell.value}),
                ),
                Err(error) => emit_error(stderr, json, &error),
            },
        };
        if !emitted {
            return EXIT_TOOL_ERROR;
        }
    }
}

fn emit_history(stdout: &mut dyn Write, json_mode: bool, session: &ReplSession) -> bool {
    if json_mode {
        return emit_event(
            stdout,
            true,
            "history",
            &json!({"cells": session.cells.iter().map(|cell| json!({
                "id": cell.id, "source": cell.source, "value": cell.value,
            })).collect::<Vec<_>>() }),
        );
    }
    for cell in &session.cells {
        if writeln!(stdout, "{} = {} ({})", cell.id, cell.value, cell.source).is_err() {
            return false;
        }
    }
    true
}

fn emit_event(
    stdout: &mut dyn Write,
    json_mode: bool,
    event: &str,
    detail: &serde_json::Value,
) -> bool {
    if json_mode {
        writeln!(
            stdout,
            "{}",
            json!({"schema": "sico.repl.event.v0", "event": event, "detail": detail})
        )
        .is_ok()
    } else if event == "cell" {
        writeln!(
            stdout,
            "{} = {}",
            detail["id"].as_str().unwrap_or("invalid-cell-id"),
            detail["value"].as_i64().unwrap_or_default()
        )
        .is_ok()
    } else {
        writeln!(stdout, "{event}").is_ok()
    }
}

fn emit_error(stderr: &mut dyn Write, json_mode: bool, message: &str) -> bool {
    if json_mode {
        writeln!(
            stderr,
            "{}",
            json!({"schema": "sico.repl.event.v0", "event": "error", "detail": message})
        )
        .is_ok()
    } else {
        writeln!(stderr, "sico repl: {message}").is_ok()
    }
}

fn export_session(session: &ReplSession, path: &str) -> Result<(), String> {
    if path.is_empty() || path.len() > MAX_EXPORT_PATH_BYTES || path.contains('\0') {
        return Err("export path must be 1..=4096 bytes".to_owned());
    }
    let bytes = session.export()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(path))
        .map_err(|error| format!("cannot create export: {error}"))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.flush()) {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(format!("cannot write export: {error}"));
    }
    Ok(())
}

#[derive(Debug)]
enum LineError {
    Rejected(String),
    Io(std::io::Error),
}

fn read_bounded_line(input: &mut dyn Read) -> Result<Option<String>, LineError> {
    let mut bytes = Vec::new();
    let mut overflow = false;
    loop {
        let mut byte = [0_u8; 1];
        match input.read(&mut byte) {
            Ok(0) if bytes.is_empty() && !overflow => return Ok(None),
            Ok(0) => break,
            Ok(1) if byte[0] == b'\n' => break,
            Ok(1) => {
                if bytes.len() < MAX_REPL_CELL_BYTES {
                    bytes.push(byte[0]);
                } else {
                    overflow = true;
                }
            }
            Ok(_) => unreachable!("one-byte read returned more than one byte"),
            Err(error) => return Err(LineError::Io(error)),
        }
    }
    if overflow {
        return Err(LineError::Rejected("line exceeds 4 KiB".to_owned()));
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| LineError::Rejected("line is not UTF-8".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailWriter;

    impl Write for FailWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "closed output",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn session_replays_deterministically_and_reset_restarts_identity() {
        let mut session = ReplSession::default();
        let first = session.submit("40 + 2").unwrap().clone();
        assert_eq!(first.value, 42);
        assert_eq!(session.submit("41 + 1").unwrap().value, 42);
        let export = session.export().unwrap();
        assert!(
            export
                .windows(first.id.len())
                .any(|part| part == first.id.as_bytes())
        );
        session.reset();
        assert_eq!(session.submit("40 + 2").unwrap().id, first.id);
    }

    #[test]
    fn failed_cells_and_exact_limits_do_not_mutate_history() {
        let mut session = ReplSession::default();
        assert!(session.submit("unknown").is_err());
        assert!(session.cells.is_empty());
        assert!(
            session
                .submit(&"1".repeat(MAX_REPL_CELL_BYTES + 1))
                .is_err()
        );
        for _ in 0..MAX_REPL_CELLS {
            session.submit("1").unwrap();
        }
        assert!(session.submit("1").is_err());
        assert_eq!(session.cells.len(), MAX_REPL_CELLS);
    }

    #[test]
    fn history_limit_accepts_exactly_sixteen_kibibytes() {
        let source = format!("1{}+0", " ".repeat(MAX_REPL_CELL_BYTES - 3));
        assert_eq!(source.len(), MAX_REPL_CELL_BYTES);
        let mut session = ReplSession::default();
        for _ in 0..(MAX_REPL_HISTORY_BYTES / MAX_REPL_CELL_BYTES) {
            session.submit(&source).unwrap();
        }
        assert_eq!(session.source_bytes, MAX_REPL_HISTORY_BYTES);
        assert!(session.submit("1").is_err());
        assert_eq!(session.source_bytes, MAX_REPL_HISTORY_BYTES);
    }

    #[test]
    fn bounded_reader_drains_rejected_lines_and_recovers() {
        let mut input = vec![b'x'; MAX_REPL_CELL_BYTES + 1];
        input.extend_from_slice(b"\n1 + 1\r\n");
        let mut input = input.as_slice();
        assert!(matches!(
            read_bounded_line(&mut input),
            Err(LineError::Rejected(_))
        ));
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some("1 + 1".to_owned())
        );
    }

    #[test]
    fn output_failure_stops_the_session() {
        let matches = repl_command().get_matches_from(["repl"]);
        let mut input = b"1\n2\n".as_slice();
        let mut stdout = FailWriter;
        let mut stderr = Vec::new();
        assert_eq!(
            run_repl(&matches, &mut input, &mut stdout, &mut stderr),
            EXIT_TOOL_ERROR
        );
        assert_eq!(input, b"2\n");
    }
}

#[cfg(test)]
mod step0108_tests {
    use super::*;

    #[test]
    fn cells_cannot_create_tasks_resources_or_stores() {
        // STEP-0108: REPL cells are compile-time constant Int expressions;
        // anything that would need a task, channel, stream, fs resource or
        // Store is refused before execution, so cells structurally cannot
        // retain resources into a later generation.
        let mut session = ReplSession::default();
        for source in [
            "spawn shout(\"x\")",
            "task group:",
            "sico.fs.read(\"input.txt\")",
            "sico.channel.open()",
            "main(1)",
        ] {
            assert!(session.submit(source).is_err(), "{source} must be refused");
        }
        assert_eq!(session.submit("1 + 2").unwrap().value, 3);
    }
}
