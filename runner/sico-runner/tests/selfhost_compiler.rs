//! M22 typed-IR differential: the Sico-written compiler emits canonical
//! `sico.ir.v0` bytes and the permanent Rust verifier accepts and
//! reserializes them byte-exactly.

use sico_ir::{Module, canonical_json, lower_core, verify};
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use sico_source::{SourceFile, SourceId};

const COMPILER_SOURCE: &str = include_str!("../../../selfhost/compiler.sico");
const COMPILER_LEXER_SOURCE: &str = include_str!("../../../selfhost/compiler_lexer.sico");
const COMPILER_PARSER_SOURCE: &str = include_str!("../../../selfhost/compiler_parser.sico");
const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const FORMATTER_SOURCE: &str = include_str!("../../../selfhost/formatter.sico");
const IDENTITY_SOURCE: &str =
    "function identity(value: Int) returns Int:\n  return value\nend function\n";

fn assert_bytes_equal(actual: &[u8], expected: &[u8]) {
    if actual == expected {
        return;
    }
    let first = actual
        .iter()
        .zip(expected)
        .position(|(left, right)| left != right)
        .unwrap_or(actual.len().min(expected.len()));
    let start = first.saturating_sub(80);
    let actual_end = (first + 160).min(actual.len());
    let expected_end = (first + 160).min(expected.len());
    panic!(
        "byte mismatch at {first}; actual_len={}, expected_len={}\nactual: {}\nexpected: {}",
        actual.len(),
        expected.len(),
        String::from_utf8_lossy(&actual[start..actual_end]),
        String::from_utf8_lossy(&expected[start..expected_end]),
    );
}

fn rust_ir(text: &str) -> String {
    let source = SourceFile::from_text(SourceId::new(0), "selfhost-input.sico", text)
        .expect("fixture source is bounded");
    let module = lower_core(&source).expect("Rust oracle lowers fixture");
    canonical_json(&module).expect("Rust oracle emits canonical IR")
}

fn compile_guest() -> &'static [u8] {
    static COMPONENT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    COMPONENT.get_or_init(|| {
        let directory =
            std::env::temp_dir().join(format!("sico-step0197-compiler-{}", std::process::id()));
        if directory.exists() {
            std::fs::remove_dir_all(&directory).unwrap();
        }
        std::fs::create_dir(&directory).unwrap();
        let source_path = directory.join("compiler.sico");
        let component_path = directory.join("compiler.component.wasm");
        std::fs::write(&source_path, COMPILER_SOURCE).unwrap();
        std::fs::write(directory.join("compiler_lexer.sico"), COMPILER_LEXER_SOURCE).unwrap();
        std::fs::write(
            directory.join("compiler_parser.sico"),
            COMPILER_PARSER_SOURCE,
        )
        .unwrap();
        std::fs::write(directory.join("parser.sico"), PARSER_SOURCE).unwrap();
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
    })
}

fn run_guest_with_args(input: &str, arguments: Vec<String>) -> RunOutcome {
    // Reuse native compilation, not guest execution state. PreparedProgram::run
    // allocates a fresh Store, resources and fuel for every fixture. Serialize
    // runs because timeout watchdogs advance the shared Engine's epoch.
    static PREPARED: std::sync::OnceLock<std::sync::Mutex<PreparedProgram>> =
        std::sync::OnceLock::new();
    let prepared = PREPARED
        .get_or_init(|| {
            let runner = Runner::new().expect("runner builds");
            std::sync::Mutex::new(
                runner
                    .prepare_program_with_net(
                        compile_guest(),
                        &FsGrants::default(),
                        &NetGrants::default(),
                    )
                    .expect("compiler component links"),
            )
        })
        .lock()
        .expect("compiler execution lock is healthy");
    prepared
        .run(
            &ScriptInput {
                arguments,
                stdin: input.as_bytes().to_vec(),
            },
            &RunnerLimits {
                fuel: 5_000_000_000,
                timeout: std::time::Duration::from_secs(30),
                memory_bytes: 512 * 1024 * 1024,
                ..RunnerLimits::default()
            },
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

fn run_guest(input: &str) -> RunOutcome {
    run_guest_with_args(input, Vec::new())
}

fn decode_hex(text: &[u8]) -> Vec<u8> {
    fn nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("non-canonical hex byte"),
        }
    }
    assert_eq!(text.len() % 2, 0);
    let (pairs, remainder) = text.as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect()
}

#[derive(Debug, Eq, PartialEq)]
struct GuestToken {
    kind: String,
    start: usize,
    end: usize,
    text: Vec<u8>,
}

fn decode_tokens(mut bytes: &[u8]) -> Vec<GuestToken> {
    let mut tokens = Vec::new();
    while !bytes.is_empty() {
        let newline = bytes.iter().position(|byte| *byte == b'\n').unwrap();
        let header = std::str::from_utf8(&bytes[..newline]).unwrap();
        bytes = &bytes[newline + 1..];
        let fields: Vec<_> = header.split('\t').collect();
        assert_eq!(fields.len(), 4, "{header:?}");
        let start = fields[1].parse::<usize>().unwrap();
        let end = fields[2].parse::<usize>().unwrap();
        let length = fields[3].parse::<usize>().unwrap();
        assert_eq!(end.checked_sub(start), Some(length));
        let text = bytes[..length].to_vec();
        bytes = &bytes[length..];
        assert_eq!(bytes.first(), Some(&b'\n'));
        bytes = &bytes[1..];
        tokens.push(GuestToken {
            kind: fields[0].to_owned(),
            start,
            end,
            text,
        });
    }
    tokens
}

#[test]
fn compiler_entry_uses_lossless_module_lexer_on_its_own_sources() {
    let component = compile_guest();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("compiler component links");
    let limits = RunnerLimits {
        fuel: 1_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        memory_bytes: 512 * 1024 * 1024,
        ..RunnerLimits::default()
    };

    for (source_index, (name, source_text)) in [
        ("selfhost/compiler.sico", COMPILER_SOURCE),
        ("selfhost/compiler_lexer.sico", COMPILER_LEXER_SOURCE),
    ]
    .into_iter()
    .enumerate()
    {
        let source_bytes = source_text.as_bytes();
        let source = SourceFile::from_bytes(
            SourceId::new(u32::try_from(source_index).unwrap()),
            name,
            source_bytes,
        )
        .unwrap();
        let mut actual = Vec::new();
        let mut base = 0usize;
        for chunk in source_bytes.split_inclusive(|byte| *byte == b'\n') {
            let outcome = prepared
                .run(
                    &ScriptInput {
                        arguments: vec!["--emit-tokens".to_owned()],
                        stdin: chunk.to_vec(),
                    },
                    &limits,
                    &CancelToken::new(),
                )
                .expect("input bounds hold");
            let RunOutcome::Output(output) = outcome else {
                panic!("compiler lexer failed for {name}@{base}: {outcome:?}")
            };
            let mut framed = decode_tokens(&output.stdout);
            assert_eq!(framed.last().map(|token| token.kind.as_str()), Some("Eof"));
            framed.pop();
            for token in &mut framed {
                token.start += base;
                token.end += base;
            }
            actual.extend(framed);
            base += chunk.len();
        }
        actual.push(GuestToken {
            kind: "Eof".to_owned(),
            start: source_bytes.len(),
            end: source_bytes.len(),
            text: Vec::new(),
        });
        let expected = sico_lexer::lex(&source);
        assert_eq!(actual.len(), expected.tokens().len(), "{name}");
        for (actual, expected) in actual.iter().zip(expected.tokens()) {
            let start = usize::from(expected.range.start());
            let end = usize::from(expected.range.end());
            assert_eq!(actual.kind, format!("{:?}", expected.kind), "{name}");
            assert_eq!((actual.start, actual.end), (start, end), "{name}");
            assert_eq!(actual.text, source_bytes[start..end], "{name}");
        }
    }
}

#[test]
fn compiler_entry_uses_module_parser_for_its_own_declarations() {
    for (source_index, (name, source_text)) in [
        ("selfhost/compiler.sico", COMPILER_SOURCE),
        ("selfhost/compiler_lexer.sico", COMPILER_LEXER_SOURCE),
        ("selfhost/compiler_parser.sico", COMPILER_PARSER_SOURCE),
    ]
    .into_iter()
    .enumerate()
    {
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(source_index).unwrap()),
            name,
            source_text,
        )
        .unwrap();
        let parsed = sico_parser::parse(&source);
        assert!(parsed.lex_errors().is_empty(), "{name}");
        assert!(parsed.errors().is_empty(), "{name}: {:?}", parsed.errors());
        let expected = parsed
            .ast()
            .unwrap()
            .declarations()
            .iter()
            .map(|declaration| {
                format!(
                    "{:?}\t{}\t{}\t{}\t{:?}\n",
                    declaration.kind,
                    declaration.name,
                    u32::from(declaration.range.start()),
                    u32::from(declaration.range.end()),
                    declaration.detail,
                )
            })
            .collect::<String>();
        let outcome = run_guest_with_args(source_text, vec!["--emit-ast-metadata".to_owned()]);
        let RunOutcome::Output(output) = outcome else {
            panic!("compiler parser failed for {name}: {outcome:?}")
        };
        assert_eq!(output.exit_code, 0, "{name}: {:?}", output.stderr);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn sico_compiler_emits_rust_verified_canonical_ir() {
    for source in [
        IDENTITY_SOURCE,
        "function keep_count(count: I64) returns I64:\n  return count\nend function\n",
        "function keep_size(size: U64) returns U64:\n  return size\nend function\n",
        "function keep_flag(flag: Bool) returns Bool:\n  return flag\nend function\n",
        "function answer() returns Int:\n  return 42\nend function\n",
        "function add(a: Int, b: Int) returns Int:\n  return a + b\nend function\n",
    ] {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("identity fixture must compile: {source}")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        let actual = output.stdout;
        assert_eq!(String::from_utf8_lossy(&actual), expected, "{source}");

        let decoded: Module = serde_json::from_slice(&actual).expect("guest IR parses");
        assert!(verify(&decoded).is_empty(), "{:?}", verify(&decoded));
        assert_eq!(canonical_json(&decoded).unwrap().as_bytes(), actual);
        if source.contains("answer") {
            let rust_module: Module = serde_json::from_str(&expected).unwrap();
            assert_eq!(
                sico_codegen_wasm::compile_component(&decoded).unwrap(),
                sico_codegen_wasm::compile_component(&rust_module).unwrap(),
                "verified guest IR must preserve deterministic codegen bytes"
            );
        }
    }
}

#[test]
fn sico_compiler_lowers_the_formatter_intrinsic_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function ascii_letter")
        .expect("formatter keeps the intrinsic prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter intrinsic prefix must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_fixed_guard_chains_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function scan_space")
        .expect("formatter keeps the fixed guard-chain prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter fixed guard-chain prefix must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_scan_space_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function scan_ident")
        .expect("formatter keeps the scan_space region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter scan_space region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_scan_ident_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function scan_integer")
        .expect("formatter keeps the scan_ident region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter scan_ident region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_scan_integer_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function scan_comment")
        .expect("formatter keeps the scan_integer region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let output = match run_guest(source) {
        RunOutcome::Output(output) => output,
        other => panic!("formatter scan_integer region must compile: {other:?}"),
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_scan_comment_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function scan_string")
        .expect("formatter keeps the scan_comment region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter scan_comment region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_scan_string_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function punctuation_kind")
        .expect("formatter keeps the scan_string region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter scan_string region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_punctuation_kind_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function item")
        .expect("formatter keeps the punctuation_kind region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter punctuation_kind region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_item_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function append_pair")
        .expect("formatter keeps the item region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter item region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_append_pair_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function line_tokens")
        .expect("formatter keeps the append_pair region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter append_pair region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_line_tokens_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function source_has_lex_error")
        .expect("formatter keeps the line_tokens region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter line_tokens region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn dump_lt_region_ir() {
    let end = FORMATTER_SOURCE.find("function source_has_lex_error").unwrap();
    let source = &FORMATTER_SOURCE[..end];
    println!("{}", rust_ir(source));
}

#[test]
fn sico_compiler_refuses_an_invalid_parameter_shape_with_typed_identity() {
    let mutation = IDENTITY_SOURCE.replacen(": Int", "; Int", 1);
    assert_eq!(mutation.len(), IDENTITY_SOURCE.len());
    assert_eq!(
        run_guest(&mutation),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-PARAMETER-TYPE".into(),
        }
    );
    // The same prepared compiler must remain usable after a domain refusal.
    let RunOutcome::Output(output) = run_guest(IDENTITY_SOURCE) else {
        panic!("valid input after refusal must compile in a fresh Store")
    };
    assert_eq!(output.exit_code, 0);
    assert_bytes_equal(&output.stdout, rust_ir(IDENTITY_SOURCE).as_bytes());
}

#[test]
fn sico_compiler_encodes_general_nonnegative_int_constant_core_wasm() {
    for (name, literal) in [
        ("x", "0"),
        ("small", "63"),
        ("sign_boundary", "64"),
        ("one_byte_max", "127"),
        ("two_byte", "128"),
        ("leb_probe", "624485"),
        ("signed_max", "9223372036854775807"),
    ] {
        let source = format!("function {name}() returns Int:\n  return {literal}\nend function\n");
        let source_file =
            SourceFile::from_text(SourceId::new(0), "identity.sico", source.clone()).unwrap();
        let rust_module = lower_core(&source_file).unwrap();
        let expected = sico_codegen_wasm::compile(&rust_module).unwrap();

        let RunOutcome::Output(output) =
            run_guest_with_args(&source, vec!["--emit-core-hex".into()])
        else {
            panic!("constant codegen fixture must succeed: {source}")
        };
        let actual = decode_hex(&output.stdout);
        assert_eq!(actual, expected, "{source}");
        wasmparser::Validator::new().validate_all(&actual).unwrap();
    }

    for source in [
        "function negative() returns Int:\n  return -1\nend function\n",
        "function too_large() returns Int:\n  return 9223372036854775808\nend function\n",
        "function parameter(value: Int) returns Int:\n  return value\nend function\n",
    ] {
        assert_eq!(
            run_guest_with_args(source, vec!["--emit-core-hex".into()]),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "unsupported codegen source shape".into(),
            },
            "unsupported Core Wasm shape must fail closed: {source}"
        );
    }
}
