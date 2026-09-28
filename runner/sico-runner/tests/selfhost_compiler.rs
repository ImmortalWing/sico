//! M22 typed-IR differential: the Sico-written compiler emits canonical
//! `sico.ir.v0` bytes and the permanent Rust verifier accepts and
//! reserializes them byte-exactly.

use sico_ir::{Module, canonical_json, lower_core, verify};
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits,
    ScriptInput,
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
    let end = FORMATTER_SOURCE
        .find("function source_has_lex_error")
        .unwrap();
    let source = &FORMATTER_SOURCE[..end];
    println!("{}", rust_ir(source));
}

#[test]
fn dump_shle_region_ir() {
    let end = FORMATTER_SOURCE.find("function no_space_before").unwrap();
    let source = &FORMATTER_SOURCE[..end];
    println!("{}", rust_ir(source));
}

#[test]
fn dump_nm_region_ir() {
    let end = FORMATTER_SOURCE.find("function close_code").unwrap();
    let source = &FORMATTER_SOURCE[..end];
    println!("{}", rust_ir(source));
}

#[test]
fn sico_compiler_lowers_the_source_has_lex_error_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function no_space_before")
        .expect("formatter keeps the source_has_lex_error region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter source_has_lex_error region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_format_code_region_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function nearest_match")
        .expect("formatter keeps the format_code region prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("formatter format_code region must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_u64_to_text_in_while_body_byte_exactly() {
    let source = "function convert(depth: U64) returns Text:\n  let cursor = depth\n  while U64.less_than(U64.literal(0), cursor):\n    let key = sico.u64.to_text(cursor)\n    set cursor = U64.literal(0)\n  end while\n  return \"\"\nend function\n";
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("while-body intrinsic RHS must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let malformed = source.replace(
        "sico.u64.to_text(cursor)",
        "sico.u64.to_text(cursor, cursor)",
    );
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );

    let wrong_type = source
        .replace("convert(depth: U64)", "convert(depth: U64, word: Text)")
        .replace("sico.u64.to_text(cursor)", "sico.u64.to_text(word)");
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );

    let wrong_local = source
        .replace(
            "let cursor = depth",
            "let cursor = depth\n  let word = \"no\"",
        )
        .replace("sico.u64.to_text(cursor)", "sico.u64.to_text(word)");
    assert_eq!(
        run_guest(&wrong_local),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
}

#[test]
fn sico_compiler_lowers_map_get_match_subject_in_while_body_byte_exactly() {
    let source = "function lookup(blocks: Map[Text,U64]) returns U64:\n  let cursor = U64.literal(1)\n  while U64.less_than(U64.literal(0), cursor):\n    match sico.map.get[Text,U64](blocks, \"0\"):\n      case ok(close):\n        set cursor = close\n      case error(_):\n        set cursor = U64.literal(0)\n    end match\n  end while\n  return cursor\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("map.get match subject must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let local_key = source
        .replace(
            "  while U64.less_than",
            "  let key = \"0\"\n  while U64.less_than",
        )
        .replace("(blocks, \"0\")", "(blocks, key)");
    let RunOutcome::Output(local_output) = run_guest(&local_key) else {
        panic!("map.get with a Text local key must compile")
    };
    assert_eq!(local_output.exit_code, 0, "{:?}", local_output.stderr);
    assert_bytes_equal(&local_output.stdout, rust_ir(&local_key).as_bytes());

    let parameter_key = source
        .replace("blocks: Map[Text,U64]", "blocks: Map[Text,U64], key: Text")
        .replace("(blocks, \"0\")", "(blocks, key)");
    let RunOutcome::Output(parameter_output) = run_guest(&parameter_key) else {
        panic!("map.get with a Text parameter key must compile")
    };
    assert_eq!(
        parameter_output.exit_code, 0,
        "{:?}",
        parameter_output.stderr
    );
    assert_bytes_equal(&parameter_output.stdout, rust_ir(&parameter_key).as_bytes());

    for malformed in [
        source.replace("(blocks, \"0\")", "(blocks, \"0\", \"1\")"),
        source.replace("get[Text,U64]", "get[Text,U64,U64]"),
    ] {
        assert_eq!(
            run_guest(&malformed),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "ERR:E-SH-IR-CALL-SHAPE".into(),
            }
        );
    }

    let wrong_map = source.replace("blocks: Map[Text,U64]", "blocks: Map[Text,I64]");
    assert_eq!(
        run_guest(&wrong_map),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let wrong_key = source
        .replace("blocks: Map[Text,U64]", "blocks: Map[Text,U64], key: U64")
        .replace("(blocks, \"0\")", "(blocks, key)");
    assert_eq!(
        run_guest(&wrong_key),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid map.get input must compile after refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_nearest_match_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function set_nearest_match")
        .expect("formatter keeps the nearest_match prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("nearest_match prefix must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let unreachable_arm_statement = source.replace(
        "          return sico.text.concat(\":\", key)\n        end if",
        "          return sico.text.concat(\":\", key)\n          set cursor = cursor\n        end if",
    );
    assert_ne!(unreachable_arm_statement, source);
    assert_eq!(
        run_guest(&unreachable_arm_statement),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-STATEMENT".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid nested match arm must compile after a refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_match_in_whileless_if_byte_exactly() {
    let source = "function lookup(blocks: Map[Text,U64], count: U64) returns U64:\n  let cursor = count\n  if U64.less_than(U64.literal(0), cursor):\n    match sico.map.get[Text,U64](blocks, \"0\"):\n      case ok(value):\n        set cursor = value\n      case error(_):\n        set cursor = cursor\n    end match\n  end if\n  return cursor\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("while-less if/match shape must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let nested_length = source
        .replace("count: U64)", "count: U64, word: Text)")
        .replace(
            "U64.less_than(U64.literal(0), cursor):",
            "U64.less_than(U64.literal(0), sico.text.length(word)):",
        );
    let RunOutcome::Output(nested_output) = run_guest(&nested_length) else {
        panic!("nested text.length condition must compile")
    };
    assert_eq!(nested_output.exit_code, 0, "{:?}", nested_output.stderr);
    assert_bytes_equal(&nested_output.stdout, rust_ir(&nested_length).as_bytes());
    let nested_local = nested_length
        .replace(
            "  if U64.less_than",
            "  let label = word\n  if U64.less_than",
        )
        .replace("sico.text.length(word)", "sico.text.length(label)");
    let RunOutcome::Output(local_output) = run_guest(&nested_local) else {
        panic!("nested text.length of a Text local must compile")
    };
    assert_eq!(local_output.exit_code, 0, "{:?}", local_output.stderr);
    assert_bytes_equal(&local_output.stdout, rust_ir(&nested_local).as_bytes());
    for malformed in [
        nested_length.replace("sico.text.length(word)", "sico.text.length(word, word)"),
        nested_length.replace("sico.text.length(word)", "sico.text.length()"),
    ] {
        assert_eq!(
            run_guest(&malformed),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "ERR:E-SH-IR-CALL-SHAPE".into(),
            }
        );
    }
    let wrong_type = nested_length.replace("word: Text", "word: U64");
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let wrong_local = nested_local.replace("let label = word", "let label = count");
    assert_eq!(
        run_guest(&wrong_local),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&nested_length) else {
        panic!("valid nested condition must compile after refusals")
    };
    assert_bytes_equal(&recovered.stdout, rust_ir(&nested_length).as_bytes());
}

#[test]
fn sico_compiler_lowers_set_nearest_match_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function match_arm_levels")
        .expect("formatter keeps the set_nearest_match prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("set_nearest_match prefix must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_match_arm_levels_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function close_code")
        .expect("formatter keeps the match_arm_levels prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("match_arm_levels prefix must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let extra_key_argument = source.replace(
        "sico.u64.to_text(cursor)):",
        "sico.u64.to_text(cursor, cursor)):",
    );
    assert_ne!(extra_key_argument, source);
    assert_eq!(
        run_guest(&extra_key_argument),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let extra_map_argument = source.replace(
        "sico.u64.to_text(cursor)):",
        "sico.u64.to_text(cursor), cursor):",
    );
    assert_ne!(extra_map_argument, source);
    assert_eq!(
        run_guest(&extra_map_argument),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid match_arm_levels prefix must compile after refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_direct_close_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function opener_close")
        .expect("formatter keeps the direct_close prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("direct_close prefix must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_opener_close_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function normalize_source")
        .expect("formatter keeps the opener_close prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("opener_close prefix must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let recovered_end = FORMATTER_SOURCE.find("function opener_close").unwrap();
    let recovered_source = &FORMATTER_SOURCE[..recovered_end];
    let RunOutcome::Output(recovered) = run_guest(recovered_source) else {
        panic!("direct_close prefix must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, rust_ir(recovered_source).as_bytes());
}

#[test]
fn sico_compiler_lowers_normalize_source_prefix_byte_exactly() {
    let end = FORMATTER_SOURCE
        .find("function main")
        .expect("formatter keeps the normalize_source prefix");
    let source = &FORMATTER_SOURCE[..end];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("normalize_source prefix must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let recovered_end = FORMATTER_SOURCE.find("function normalize_source").unwrap();
    let recovered_source = &FORMATTER_SOURCE[..recovered_end];
    let RunOutcome::Output(recovered) = run_guest(recovered_source) else {
        panic!("opener_close prefix must compile after normalize_source")
    };
    assert_bytes_equal(&recovered.stdout, rust_ir(recovered_source).as_bytes());
}

#[test]
fn sico_compiler_refuses_full_formatter_at_main_condition() {
    assert_eq!(
        run_guest(FORMATTER_SOURCE),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-SCICOND".into(),
        }
    );
    let end = FORMATTER_SOURCE.find("function main").unwrap();
    let prefix = &FORMATTER_SOURCE[..end];
    let RunOutcome::Output(recovered) = run_guest(prefix) else {
        panic!("normalize_source prefix must compile after full-source refusal")
    };
    assert_bytes_equal(&recovered.stdout, rust_ir(prefix).as_bytes());
}

#[test]
fn sico_compiler_lowers_while_in_else_and_join_regions_byte_exactly() {
    for source in [
        "function else_loop(n: U64) returns U64:\n  let x = U64.literal(0)\n  if U64.less_than(n, U64.literal(1)):\n    set x = U64.literal(1)\n  else:\n    while U64.less_than(x, n):\n      set x = n\n    end while\n  end if\n  return x\nend function\n",
        "function join_loop(n: U64) returns U64:\n  let x = U64.literal(0)\n  if U64.less_than(n, U64.literal(1)):\n    set x = U64.literal(1)\n  end if\n  while U64.less_than(x, n):\n    set x = n\n  end while\n  return x\nend function\n",
    ] {
        let expected = rust_ir(source);
        let outcome = run_guest(source);
        let RunOutcome::Output(output) = &outcome else {
            panic!("nested while must compile: {outcome:?}")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

#[test]
fn sico_compiler_lowers_map_put_return_byte_exactly() {
    let source = "function put_probe(active: Map[Text,U64], key_bytes: Bytes, value: U64) returns Map[Text,U64]:\n  while U64.less_than(U64.literal(0), value):\n    return sico.map.put[Text,U64](active, sico.bytes.utf8_decode(key_bytes), value)\n  end while\n  return active\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("map.put return must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let extra_arg = source.replace("key_bytes), value)", "key_bytes), value, value)");
    assert_eq!(
        run_guest(&extra_arg),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let wrong_map = source.replace("active: Map[Text,U64]", "active: Text");
    assert_eq!(
        run_guest(&wrong_map),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let wrong_bytes = source.replace("key_bytes: Bytes", "key_bytes: Text");
    assert_eq!(
        run_guest(&wrong_bytes),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let wrong_value = source.replace("value: U64", "value: Text");
    assert_eq!(
        run_guest(&wrong_value),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid map.put return must compile after refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_map_empty_local_byte_exactly() {
    let source = "function empty_probe(n: U64) returns Map[Text,U64]:\n  let blocks = sico.map.empty[Text,U64]()\n  while U64.less_than(U64.literal(0), n):\n    return blocks\n  end while\n  return blocks\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("map.empty local must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    for malformed in [
        source.replace("sico.map.empty[Text,U64]()", "sico.map.empty[Text,U64](n)"),
        source.replace("sico.map.empty[Text,U64]()", "sico.map.empty[Text,U64]() n"),
    ] {
        assert_ne!(malformed, source);
        assert_eq!(
            run_guest(&malformed),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "ERR:E-SH-IR-CALL-SHAPE".into(),
            }
        );
    }
    let wrong_type = source.replace("sico.map.empty[Text,U64]()", "sico.map.empty[Text,I64]()");
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid map.empty local must compile after refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_map_get_from_local_byte_exactly() {
    let source = "function local_map(key: Text) returns U64:\n  let blocks = sico.map.empty[Text,U64]()\n  let cursor = U64.literal(1)\n  while U64.less_than(U64.literal(0), cursor):\n    match sico.map.get[Text,U64](blocks, key):\n      case ok(value):\n        set cursor = value\n      case error(_):\n        set cursor = U64.literal(0)\n    end match\n  end while\n  return cursor\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("map.get local subject must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let nested_key = source.replace("(blocks, key)", "(blocks, sico.u64.to_text(cursor))");
    let nested_expected = rust_ir(&nested_key);
    let RunOutcome::Output(nested_output) = run_guest(&nested_key) else {
        panic!("local map with nested key must compile")
    };
    assert_eq!(nested_output.exit_code, 0, "{:?}", nested_output.stderr);
    assert_bytes_equal(&nested_output.stdout, nested_expected.as_bytes());

    let wrong_type = source.replace(
        "let blocks = sico.map.empty[Text,U64]()",
        "let blocks = U64.literal(0)",
    );
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid local map must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_user_call_as_compare_left_operand_byte_exactly() {
    let source = "function helper(value: U64) returns U64:\n  return value\nend function\nfunction compare_call(value: U64) returns U64:\n  let cursor = U64.literal(0)\n  while U64.less_than(cursor, U64.literal(1)):\n    if U64.equal(helper(value), value):\n      return value\n    end if\n    set cursor = value\n  end while\n  return cursor\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("user call as comparison left operand must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let prefix = &FORMATTER_SOURCE[..FORMATTER_SOURCE.find("function normalize_source").unwrap()];
    let formatter_probe = format!(
        "{prefix}function compare_close(close: Text, expected: U64) returns U64:\n  let cursor = U64.literal(0)\n  while U64.less_than(cursor, U64.literal(1)):\n    if U64.equal(close_code(close), expected):\n      return expected\n    end if\n    set cursor = expected\n  end while\n  return cursor\nend function\n"
    );
    let formatter_expected = rust_ir(&formatter_probe);
    let RunOutcome::Output(formatter_output) = run_guest(&formatter_probe) else {
        panic!("formatter close_code comparison must compile")
    };
    assert_eq!(
        formatter_output.exit_code, 0,
        "{:?}",
        formatter_output.stderr
    );
    assert_bytes_equal(&formatter_output.stdout, formatter_expected.as_bytes());

    let wrong_type = source.replace("helper(value)", "helper(\"wrong\")");
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid compare call must compile after refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_user_calls_inside_text_concat_byte_exactly() {
    let source = "function repeat_indent(level: U64) returns Text:\n  return \" \"\nend function\nfunction format_code(level: U64) returns Text:\n  return \"x\"\nend function\nfunction concat_calls(level: U64) returns Text:\n  let out = \"\"\n  while U64.less_than(U64.literal(0), level):\n    set out = sico.text.concat(repeat_indent(level), format_code(level))\n    return out\n  end while\n  return out\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("text.concat with user-call arguments must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let wrong_type = source.replace(
        "function repeat_indent(level: U64) returns Text:\n  return \" \"",
        "function repeat_indent(level: U64) returns U64:\n  return level",
    );
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid nested concat calls must compile after refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_trimmed_user_call_inside_text_concat_byte_exactly() {
    let source = "function add(left: U64, right: U64) returns U64:\n  return left\nend function\nfunction item(tokens: List[Text], index: U64) returns Text:\n  return \"x\"\nend function\nfunction trim_probe(tokens: List[Text], comment_index: U64) returns Text:\n  let rendered = \"\"\n  while U64.less_than(U64.literal(0), comment_index):\n    set rendered = sico.text.concat(rendered, sico.text.trim(item(tokens, add(comment_index, U64.literal(1)))))\n    return rendered\n  end while\n  return rendered\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("trimmed user call in concat must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let wrong_type = source.replace(
        "function item(tokens: List[Text], index: U64) returns Text:\n  return \"x\"",
        "function item(tokens: List[Text], index: U64) returns U64:\n  return index",
    );
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid trimmed call must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_text_list_append_rhs_byte_exactly() {
    let source = "function append_probe(items: List[Text], value: Text, n: U64) returns List[Text]:\n  let output = items\n  while U64.less_than(U64.literal(0), n):\n    set output = sico.list.append(output, \"\")\n    set output = sico.list.append(output, value)\n    return output\n  end while\n  return output\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("Text list append RHS must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let extra_arg = source.replace("(output, value)", "(output, value, value)");
    assert_eq!(
        run_guest(&extra_arg),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let wrong_type = source.replace("value: Text", "value: U64");
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid list append must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_map_put_fixed_literal_rhs_byte_exactly() {
    let source = "function put_zero(items: Map[Text,U64], key: Text, n: U64) returns Map[Text,U64]:\n  let active = items\n  while U64.less_than(U64.literal(0), n):\n    set active = sico.map.put[Text,U64](active, key, U64.literal(0))\n    return active\n  end while\n  return active\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("map.put fixed literal RHS must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let extra_arg = source.replace("U64.literal(0))", "U64.literal(0), n)");
    assert_eq!(
        run_guest(&extra_arg),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let wrong_type = source.replace("U64.literal(0))", "I64.literal(0))");
    assert_eq!(
        run_guest(&wrong_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid map.put literal must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_normalize_source_before_final_join_byte_exactly() {
    let end = FORMATTER_SOURCE.find("function main").unwrap();
    let prefix = &FORMATTER_SOURCE[..end];
    let source = prefix.replace(
        "return sico.text.concat(sico.text.join(output, \"\\n\"), \"\\n\")",
        "return \"\"",
    );
    assert_ne!(
        source, prefix,
        "formatter final join marker must be present"
    );
    let expected = rust_ir(&source);
    let outcome = run_guest(&source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("normalize_source before final join must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_text_join_inside_concat_byte_exactly() {
    let source = "function join_probe(items: List[Text], n: U64) returns Text:\n  while U64.less_than(U64.literal(0), n):\n    return sico.text.concat(sico.text.join(items, \"\\n\"), \"\\n\")\n  end while\n  return \"\"\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("text.join inside concat must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let wrong_list = source.replace("items: List[Text]", "items: Text");
    assert_eq!(
        run_guest(&wrong_list),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let extra_arg = source.replace("items, \"\\n\")", "items, \"\\n\", \"x\")");
    assert_eq!(
        run_guest(&extra_arg),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid text.join must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_nested_slice_match_subject_byte_exactly() {
    let prefix = &FORMATTER_SOURCE[..FORMATTER_SOURCE.find("function same").unwrap()];
    let source = format!(
        "{prefix}function slice_probe(found_bytes: Bytes) returns U64:\n  let cursor = U64.literal(1)\n  if U64.less_than(U64.literal(0), cursor):\n    match sico.bytes.slice(found_bytes, U64.literal(1), sub(sico.bytes.length(found_bytes), U64.literal(1))):\n      case ok(part):\n        set cursor = U64.literal(1)\n      case error(_):\n        set cursor = cursor\n    end match\n  end if\n  return cursor\nend function\n"
    );
    let expected = rust_ir(&source);
    let outcome = run_guest(&source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("nested bytes.slice match subject must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    for malformed in [
        source.replace(
            "sub(sico.bytes.length(found_bytes), U64.literal(1))):",
            "sub(sico.bytes.length(found_bytes), U64.literal(1)), U64.literal(1)):",
        ),
        source.replace(
            "sico.bytes.length(found_bytes)",
            "sico.bytes.length(found_bytes, found_bytes)",
        ),
        source.replace(
            "sub(sico.bytes.length(found_bytes), U64.literal(1))",
            "sub(sico.bytes.length(found_bytes), U64.literal(1), U64.literal(1))",
        ),
    ] {
        assert_ne!(malformed, source);
        assert_eq!(
            run_guest(&malformed),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "ERR:E-SH-IR-CALL-SHAPE".into(),
            }
        );
    }
    let wrong_bytes = source.replace("found_bytes: Bytes", "found_bytes: Text");
    assert_eq!(
        run_guest(&wrong_bytes),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&source) else {
        panic!("valid nested slice match must compile after refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
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
