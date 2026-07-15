//! Command-line integration for the M1 Sico frontend.

#![forbid(unsafe_code)]

use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
};

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};
use serde_json::{Value, json};
use sico_diagnostics::{render_syntax_json, render_syntax_text, syntax_identity};
use sico_format::format as canonical_format;
use sico_parser::{DeclarationKind, Parse, parse};
use sico_source::{SourceFile, SourceId};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_DIAGNOSTIC: i32 = 1;
pub const EXIT_TOOL_ERROR: i32 = 2;

/// Runs the CLI with injectable streams for binary-level contract tests.
pub fn run<I, T>(
    args: I,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = match command().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            let exit = error.exit_code();
            let rendered = error.render().to_string();
            let written = if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                stdout.write_all(rendered.as_bytes())
            } else {
                stderr.write_all(rendered.as_bytes())
            };
            return if written.is_ok() {
                exit
            } else {
                EXIT_TOOL_ERROR
            };
        }
    };

    match matches.subcommand() {
        Some(("check", command)) => run_check(command, stdin, stdout, stderr),
        Some(("format", command)) => run_format(command, stdin, stdout, stderr),
        Some(("outline", command)) => run_outline(command, stdin, stdout, stderr),
        _ => EXIT_TOOL_ERROR,
    }
}

fn command() -> Command {
    Command::new("sico")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Sico compiler frontend")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Check source syntax (M2 type checker is unavailable)")
                .arg(input_arg())
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Emit RFC-0001 JSON")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("format")
                .about("Produce canonical source layout")
                .arg(input_arg())
                .arg(
                    Arg::new("check")
                        .long("check")
                        .help("Exit 1 when source is not canonical")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("write"),
                )
                .arg(
                    Arg::new("write")
                        .long("write")
                        .help("Rewrite a source file in place")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("check"),
                ),
        )
        .subcommand(
            Command::new("outline")
                .about("List top-level parser declarations")
                .arg(input_arg())
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Emit sico.outline.v0 JSON")
                        .action(ArgAction::SetTrue),
                ),
        )
}

fn input_arg() -> Arg {
    Arg::new("input")
        .value_name("FILE|-")
        .help("Source file, or - for stdin")
        .required(true)
        .allow_hyphen_values(true)
}

fn run_check(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let json_output = matches.get_flag("json");
    let Ok(source) = load_source(input, stdin).map_err(|message| writeln!(stderr, "{message}"))
    else {
        return EXIT_TOOL_ERROR;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return emit_frontend_failure(&source, &parsed, json_output, stdout, stderr);
    }

    if json_output {
        let output = check_json(&source, &parsed, "ok").unwrap();
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else if writeln!(stdout, "syntax ok: {}", source.name()).is_err()
        || writeln!(stdout, "type checker: unavailable (M2)").is_err()
    {
        return EXIT_TOOL_ERROR;
    }
    EXIT_SUCCESS
}

fn run_format(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let write = matches.get_flag("write");
    if write && input == "-" {
        let _ = writeln!(stderr, "sico: format --write requires a file path");
        return EXIT_TOOL_ERROR;
    }
    let Ok(source) = load_source(input, stdin).map_err(|message| writeln!(stderr, "{message}"))
    else {
        return EXIT_TOOL_ERROR;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return emit_frontend_failure(&source, &parsed, false, stdout, stderr);
    }
    let formatted = canonical_format(&source).expect("successful parse must format");

    if matches.get_flag("check") {
        if formatted == source.text() {
            return EXIT_SUCCESS;
        }
        let _ = writeln!(stderr, "{} is not canonically formatted", source.name());
        return EXIT_DIAGNOSTIC;
    }
    if write {
        return match fs::write(input, formatted.as_bytes()) {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => {
                let _ = writeln!(stderr, "sico: cannot write {input}: {error}");
                EXIT_TOOL_ERROR
            }
        };
    }
    if stdout.write_all(formatted.as_bytes()).is_err() {
        return EXIT_TOOL_ERROR;
    }
    EXIT_SUCCESS
}

fn run_outline(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let json_output = matches.get_flag("json");
    let Ok(source) = load_source(input, stdin).map_err(|message| writeln!(stderr, "{message}"))
    else {
        return EXIT_TOOL_ERROR;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return emit_frontend_failure(&source, &parsed, json_output, stdout, stderr);
    }
    let ast = parsed.ast().unwrap();
    if json_output {
        let declarations: Vec<_> = ast
            .declarations()
            .iter()
            .map(|declaration| {
                json!({
                    "kind": declaration_kind(declaration.kind),
                    "name": declaration.name,
                    "range": {
                        "start_byte": u32::from(declaration.range.start()),
                        "end_byte": u32::from(declaration.range.end())
                    }
                })
            })
            .collect();
        let output = serde_json::to_string_pretty(&json!({
            "schema": "sico.outline.v0",
            "file": source.name(),
            "type_checker": "unavailable",
            "declarations": declarations
        }))
        .unwrap();
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else {
        for declaration in ast.declarations() {
            if writeln!(
                stdout,
                "{}\t{}\t{}..{}",
                declaration_kind(declaration.kind),
                declaration.name,
                u32::from(declaration.range.start()),
                u32::from(declaration.range.end())
            )
            .is_err()
            {
                return EXIT_TOOL_ERROR;
            }
        }
    }
    EXIT_SUCCESS
}

fn load_source(input: &str, stdin: &mut dyn Read) -> Result<SourceFile, String> {
    let (name, bytes) = if input == "-" {
        let mut bytes = Vec::new();
        stdin
            .read_to_end(&mut bytes)
            .map_err(|error| format!("sico: cannot read stdin: {error}"))?;
        ("<stdin>".to_owned(), bytes)
    } else {
        let bytes =
            fs::read(input).map_err(|error| format!("sico: cannot read {input}: {error}"))?;
        (input.to_owned(), bytes)
    };
    SourceFile::from_bytes(SourceId::new(0), name.clone(), &bytes).map_err(|error| {
        format!(
            "sico: {name}: source contract error {:?} at bytes {}..{}",
            error.kind,
            u32::from(error.range.start()),
            u32::from(error.range.end())
        )
    })
}

fn emit_frontend_failure(
    source: &SourceFile,
    parsed: &Parse,
    json_output: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if !parsed.lex_errors().is_empty() {
        let first = &parsed.lex_errors()[0];
        let _ = writeln!(
            stderr,
            "sico: {}: unregistered lexical error {:?} at bytes {}..{}",
            source.name(),
            first.kind,
            u32::from(first.range.start()),
            u32::from(first.range.end())
        );
        return EXIT_TOOL_ERROR;
    }
    if !parsed
        .errors()
        .iter()
        .all(|error| syntax_identity(&error.kind).is_some())
    {
        let _ = writeln!(
            stderr,
            "sico: {}: unregistered parser error {:?}",
            source.name(),
            parsed.errors()[0].kind
        );
        return EXIT_TOOL_ERROR;
    }

    if json_output {
        let output = check_json(source, parsed, "error").unwrap();
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else {
        let output = render_syntax_text(source, source.name(), parsed.errors());
        if writeln!(stderr, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    }
    EXIT_DIAGNOSTIC
}

fn check_json(source: &SourceFile, parsed: &Parse, syntax: &str) -> Option<String> {
    let mut value: Value =
        serde_json::from_str(&render_syntax_json(source, source.name(), parsed.errors())?).ok()?;
    value["status"] = json!({
        "syntax": syntax,
        "type_checker": "unavailable",
        "semantic_checks_performed": false
    });
    serde_json::to_string_pretty(&value).ok()
}

const fn declaration_kind(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Newtype => "newtype",
        DeclarationKind::Record => "record",
        DeclarationKind::Enum => "enum",
        DeclarationKind::Capability => "capability",
        DeclarationKind::Resource => "resource",
        DeclarationKind::Interface => "interface",
        DeclarationKind::Function => "function",
    }
}
