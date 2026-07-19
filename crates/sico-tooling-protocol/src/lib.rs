//! Shared, non-executing editor/AI execution plans for STEP-0093.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

pub const EXECUTION_PLAN_SCHEMA: &str = "sico.execution-plan.v0";
pub const MAX_PROGRAM_BYTES: usize = 4 * 1024;
pub const MAX_ARGUMENTS: usize = 64;
pub const MAX_ARGUMENT_BYTES: usize = 4 * 1024;
pub const MAX_TOTAL_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_LOG_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionMode {
    Check,
    Run,
    Watch,
    Repl,
}

impl ExecutionMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Run => "run",
            Self::Watch => "watch",
            Self::Repl => "repl",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanError {
    InvalidProgram,
    UnexpectedProgram,
    InvalidArguments,
    InvalidLogLimit,
}

/// Builds a bounded direct-process plan. The returned value is data only: this
/// crate never reads source, spawns a process, opens a shell, or applies grants.
///
/// # Errors
///
/// Returns [`PlanError`] when a path, argument, mode combination, or capture
/// limit exceeds the frozen protocol bounds.
pub fn execution_plan(
    mode: ExecutionMode,
    program: Option<&str>,
    guest_arguments: &[String],
    max_log_bytes: usize,
) -> Result<Value, PlanError> {
    if max_log_bytes == 0 || max_log_bytes > MAX_LOG_BYTES {
        return Err(PlanError::InvalidLogLimit);
    }
    let program = match mode {
        ExecutionMode::Repl if program.is_some() => return Err(PlanError::UnexpectedProgram),
        ExecutionMode::Repl => None,
        _ => Some(
            program
                .filter(|value| valid_program(value))
                .ok_or(PlanError::InvalidProgram)?,
        ),
    };
    if mode == ExecutionMode::Check && !guest_arguments.is_empty() {
        return Err(PlanError::InvalidArguments);
    }
    if mode == ExecutionMode::Repl && !guest_arguments.is_empty() {
        return Err(PlanError::InvalidArguments);
    }
    if guest_arguments.len() > MAX_ARGUMENTS
        || guest_arguments
            .iter()
            .any(|argument| !valid_argument(argument))
        || guest_arguments.iter().map(String::len).sum::<usize>() > MAX_TOTAL_ARGUMENT_BYTES
    {
        return Err(PlanError::InvalidArguments);
    }

    let mut arguments = vec![mode.as_str().to_owned(), "--json".to_owned()];
    if let Some(program) = program {
        arguments.push(program.to_owned());
    }
    if !guest_arguments.is_empty() {
        arguments.push("--".to_owned());
        arguments.extend(guest_arguments.iter().cloned());
    }
    let (stdout, stderr_schemas, runtime_locations) = match mode {
        ExecutionMode::Check => ("sico.diagnostics.v0-json", Vec::<&str>::new(), false),
        ExecutionMode::Run => ("guest-bytes", vec!["sico.runner.outcome.v0"], false),
        ExecutionMode::Watch => (
            "guest-bytes",
            vec!["sico.runner.outcome.v0", "sico.runner.watch.v0"],
            false,
        ),
        ExecutionMode::Repl => (
            "sico.repl.event.v0-jsonl",
            vec!["sico.repl.event.v0"],
            false,
        ),
    };
    Ok(json!({
        "schema": EXECUTION_PLAN_SCHEMA,
        "mode": mode.as_str(),
        "executable": "sico",
        "arguments": arguments,
        "shell": false,
        "cwd": null,
        "output": {
            "stdout": stdout,
            "stderr_schemas": stderr_schemas,
            "capture_limit_bytes": max_log_bytes,
            "overflow": "truncate-and-mark"
        },
        "cancellation": {
            "owner": "client",
            "action": "terminate-direct-child-tree",
            "grace_ms": 1000,
            "typed_runner_exit_guaranteed": false
        },
        "source_map": {
            "compile_coordinates": "utf8-byte-half-open",
            "source": program,
            "runtime_locations": runtime_locations,
            "debug_adapter": false
        }
    }))
}

fn valid_program(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PROGRAM_BYTES && valid_characters(value)
}

fn valid_argument(value: &str) -> bool {
    value.len() <= MAX_ARGUMENT_BYTES && valid_characters(value)
}

fn valid_characters(value: &str) -> bool {
    !value.contains('\0') && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metacharacters_and_spaces_remain_one_direct_argument() {
        let program = "path with spaces/$(touch nope);`name`.sico";
        let guest = vec!["; echo injected".to_owned(), "$(whoami)".to_owned()];
        let plan = execution_plan(ExecutionMode::Run, Some(program), &guest, 64 * 1024).unwrap();
        assert_eq!(
            plan["arguments"],
            json!([
                "run",
                "--json",
                program,
                "--",
                "; echo injected",
                "$(whoami)"
            ])
        );
        assert_eq!(plan["shell"], false);
        assert_eq!(plan["cwd"], Value::Null);

        let empty =
            execution_plan(ExecutionMode::Run, Some("app.sico"), &[String::new()], 1).unwrap();
        assert_eq!(
            empty["arguments"],
            json!(["run", "--json", "app.sico", "--", ""])
        );
    }

    #[test]
    fn mode_protocols_and_honest_debug_boundary_are_explicit() {
        let watch =
            execution_plan(ExecutionMode::Watch, Some("app.sico"), &[], MAX_LOG_BYTES).unwrap();
        assert_eq!(watch["schema"], EXECUTION_PLAN_SCHEMA);
        assert_eq!(watch["output"]["capture_limit_bytes"], MAX_LOG_BYTES);
        assert_eq!(
            watch["output"]["stderr_schemas"],
            json!(["sico.runner.outcome.v0", "sico.runner.watch.v0"])
        );
        assert_eq!(watch["source_map"]["runtime_locations"], false);
        assert_eq!(watch["source_map"]["debug_adapter"], false);

        let repl = execution_plan(ExecutionMode::Repl, None, &[], 1).unwrap();
        assert_eq!(repl["arguments"], json!(["repl", "--json"]));
        assert_eq!(repl["output"]["stdout"], "sico.repl.event.v0-jsonl");
    }

    #[test]
    fn every_input_and_capture_bound_fails_closed() {
        assert_eq!(
            execution_plan(ExecutionMode::Run, None, &[], 1),
            Err(PlanError::InvalidProgram)
        );
        assert_eq!(
            execution_plan(ExecutionMode::Repl, Some("app.sico"), &[], 1),
            Err(PlanError::UnexpectedProgram)
        );
        assert_eq!(
            execution_plan(ExecutionMode::Run, Some("app.sico"), &[], MAX_LOG_BYTES + 1),
            Err(PlanError::InvalidLogLimit)
        );
        assert_eq!(
            execution_plan(
                ExecutionMode::Run,
                Some("app.sico"),
                &vec!["x".to_owned(); MAX_ARGUMENTS + 1],
                1,
            ),
            Err(PlanError::InvalidArguments)
        );
        assert_eq!(
            execution_plan(ExecutionMode::Run, Some("bad\npath"), &[], 1),
            Err(PlanError::InvalidProgram)
        );
    }

    #[test]
    fn repository_schema_tracks_the_frozen_identity_and_bounds() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../tooling/schema/execution-plan-v0.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            EXECUTION_PLAN_SCHEMA
        );
        assert_eq!(
            schema["properties"]["output"]["properties"]["capture_limit_bytes"]["maximum"],
            MAX_LOG_BYTES
        );
        assert_eq!(schema["properties"]["shell"]["const"], false);
        assert_eq!(
            schema["properties"]["source_map"]["properties"]["debug_adapter"]["const"],
            false
        );
    }
}
