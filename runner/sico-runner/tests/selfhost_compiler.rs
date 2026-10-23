//! M22 typed-IR differential: the Sico-written compiler emits canonical
//! `sico.ir.v0` bytes and the permanent Rust verifier accepts and
//! reserializes them byte-exactly.

use sico_codegen_wasm as codegen_oracle;
use sico_ir::{Module, Operation, Terminator, Type, ValueId, canonical_json, lower_core, verify};
use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use sico_source::{SourceFile, SourceId};

#[path = "support/selfhost_common.rs"]
mod selfhost_common;

use selfhost_common::{PreparedPool, cached_component, default_pool_slots, prepare_slot};

const COMPILER_SOURCE: &str = include_str!("../../../selfhost/compiler.sico");
const COMPILER_LEXER_SOURCE: &str = include_str!("../../../selfhost/compiler_lexer.sico");
const COMPILER_PARSER_SOURCE: &str = include_str!("../../../selfhost/compiler_parser.sico");
const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const CODEGEN_SOURCE: &str = include_str!("../../../selfhost/codegen.sico");
const FORMATTER_SOURCE: &str = include_str!("../../../selfhost/formatter.sico");
const IDENTITY_SOURCE: &str =
    "function identity(value: Int) returns Int:\n  return value\nend function\n";

fn compiler_sources() -> [(&'static str, &'static str); 5] {
    [
        ("compiler.sico", COMPILER_SOURCE),
        ("compiler_lexer.sico", COMPILER_LEXER_SOURCE),
        ("compiler_parser.sico", COMPILER_PARSER_SOURCE),
        ("parser.sico", PARSER_SOURCE),
        ("codegen.sico", CODEGEN_SOURCE),
    ]
}

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

/// STEP-0496: the same module the guest reads, kept as structure so the
/// Rust-side plan can be derived from the IR itself and not from its text.
fn rust_module(text: &str) -> Module {
    let source = SourceFile::from_text(SourceId::new(0), "selfhost-input.sico", text)
        .expect("fixture source is bounded");
    lower_core(&source).expect("Rust oracle lowers fixture")
}

/// STEP-0496: the canonical text form of one IR type, spelled exactly as the
/// guest spells it. The guest and this function must agree byte for byte.
fn plan_type_text(ty: &Type) -> Result<String, &'static str> {
    Ok(match ty {
        Type::Unit => "unit".to_owned(),
        Type::Bool => "bool".to_owned(),
        Type::Int => "int".to_owned(),
        Type::I64 => "i64".to_owned(),
        Type::U64 => "u64".to_owned(),
        Type::Float64 => "float64".to_owned(),
        Type::String => "string".to_owned(),
        Type::Bytes => "bytes".to_owned(),
        Type::List(inner) => format!("list({})", plan_type_text(inner)?),
        Type::Named(name) => format!("named({name})"),
        Type::Result { ok, error } => {
            format!("result({},{})", plan_type_text(ok)?, plan_type_text(error)?)
        }
        _ => return Err("ERR:E-SH-CG-IR-TYPE"),
    })
}

/// STEP-0496: the comma-separated value ids of an operand list.
fn plan_id_list(ids: &[ValueId]) -> String {
    ids.iter()
        .map(|id| id.0.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// STEP-0496: the canonical text form of one IR operation, spelled exactly as
/// the guest spells it.
fn plan_op_text(op: &Operation) -> Result<String, &'static str> {
    Ok(match op {
        Operation::Copy(value) => format!("copy {}", value.0),
        Operation::ConstInt(text) => format!(
            "const_int {}",
            serde_json::to_string(text).expect("a string always serializes")
        ),
        Operation::ConstI64(value) => format!("const_i64 {value}"),
        Operation::ConstU64(value) => format!("const_u64 {value}"),
        Operation::ConstBool(value) => format!("const_bool {value}"),
        Operation::ConstString(text) => format!(
            "const_string {}",
            serde_json::to_string(text).expect("a string always serializes")
        ),
        Operation::ConstBytes(bytes) => format!(
            "const_bytes {}",
            serde_json::to_string(bytes).expect("bytes always serialize")
        ),
        Operation::Call {
            function,
            arguments,
        } => {
            format!("call {} args=[{}]", function.0, plan_id_list(arguments))
        }
        Operation::Intrinsic { name, arguments } => {
            format!("intrinsic {name} args=[{}]", plan_id_list(arguments))
        }
        Operation::Project { base, field } => format!("project base={} field={field}", base.0),
        Operation::Construct { name, fields } => {
            let rendered = fields
                .iter()
                .map(|field| format!("{}={}", field.name, field.value.0))
                .collect::<Vec<_>>()
                .join(",");
            format!("construct name={name} fields=[{rendered}]")
        }
        Operation::Variant { name, payload } => {
            format!("variant name={name} payload=[{}]", plan_id_list(payload))
        }
        _ => return Err("ERR:E-SH-CG-IR-OP"),
    })
}

/// STEP-0496: the run emission plan, derived from the module structure. The
/// guest derives the same text from the canonical IR JSON it scanned; the two
/// must agree byte for byte.
fn plan_text(module: &Module) -> Result<String, &'static str> {
    let Some(run) = module.functions.iter().find(|f| f.name == "run") else {
        return Err("ERR:E-SH-CG-IR-PLAN");
    };
    let mut out = String::new();
    for parameter in &run.parameters {
        let ty = plan_type_text(&parameter.ty)?;
        out.push_str(&format!(
            "param {} {} {}\n",
            parameter.id.0, parameter.name, ty
        ));
    }
    for block in &run.blocks {
        for instruction in &block.instructions {
            let ty = plan_type_text(&instruction.ty)?;
            let op = plan_op_text(&instruction.operation)?;
            out.push_str(&format!("inst {} {}\n", instruction.result.0, ty));
            out.push_str(&format!("op {op}\n"));
        }
        match &block.terminator {
            Terminator::Return(Some(value)) => {
                out.push_str(&format!("term return {}\n", value.0));
            }
            _ => return Err("ERR:E-SH-CG-IR-TERMINATOR"),
        }
    }
    for (name, fields) in &module.records {
        for field in fields {
            let ty = plan_type_text(&field.ty)?;
            out.push_str(&format!("record {} {} {}\n", name, field.name, ty));
        }
    }
    Ok(out)
}

fn compile_guest() -> &'static [u8] {
    static COMPONENT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    COMPONENT.get_or_init(|| cached_component("compiler.sico", &compiler_sources()))
}

fn run_guest_with_args(input: &str, arguments: Vec<String>) -> RunOutcome {
    // Reuse native compilation, not guest execution state (STEP-0254).
    // STEP-0317: the pool holds one PreparedProgram per slot, each with its
    // own Runner/Engine so timeout watchdogs cannot advance each other's
    // epochs; PreparedProgram::run still allocates a fresh Store, resources
    // and fuel for every fixture. Slots grow lazily, so a single-fixture
    // invocation compiles exactly one.
    static POOL: std::sync::OnceLock<PreparedPool> = std::sync::OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let component: Vec<u8> = compile_guest().to_vec();
        PreparedPool::new(default_pool_slots(), move || prepare_slot(&component))
    });
    let prepared = pool.acquire();
    let outcome = prepared.run(
        &ScriptInput {
            arguments,
            stdin: input.as_bytes().to_vec(),
        },
        &RunnerLimits {
            // STEP-0383: the transformed parser.sico walks deeper before
            // the typed frontier (CALL-ARGUMENT -> GWPACK-OTHER), so the
            // full-source guard needs a raised fuel budget. STEP-0388: the
            // known-bad-shape sweep pushed the walk past 40B; 120B reaches
            // the same CALL-SHAPE typed refusal (~4s). STEP-0434: the
            // topological reorder + nested-call hoists pushed the walk past
            // 120B; 400B reaches the current typed refusal.
            fuel: 800_000_000_000,
            timeout: std::time::Duration::from_secs(120),
            memory_bytes: 2560 * 1024 * 1024,
            // STEP-0477: the walk now clears every typed door up to the
            // hostcall transport budget (stdin*64+64MiB); the deep
            // general_while body copies more source bytes through hostcalls
            // than that cap admits. Raise the transport budget so the
            // frontier stays a typed refusal, not a transport trap. The
            // STEP-0365 treadmill family stays registered.
            hostcall_fuel: 8 << 30,
        },
        &CancelToken::new(),
    );
    pool.release(prepared);
    outcome.expect("input bounds hold")
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
        memory_bytes: 2560 * 1024 * 1024,
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
fn sico_compiler_lowers_let_utf8_decode_field_rhs_byte_exactly() {
    let source = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nfunction probe(input: ScriptInput) returns U64:\n  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    return U64.literal(1)\n  end if\n  return U64.literal(0)\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("let utf8_decode field RHS must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let wrong_field = source.replace("utf8_decode(input.stdin)", "utf8_decode(input.arguments)");
    assert_eq!(
        run_guest(&wrong_field),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let bytes_parameter = source
        .replace(
            "function probe(input: ScriptInput) returns U64:",
            "function probe(input: ScriptInput, data: Bytes) returns U64:",
        )
        .replace("utf8_decode(input.stdin)", "utf8_decode(data.stdin)");
    assert_eq!(
        run_guest(&bytes_parameter),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let bare_parameter = source.replace("utf8_decode(input.stdin)", "utf8_decode(input)");
    assert_eq!(
        run_guest(&bare_parameter),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let extra_argument = source.replace(
        "utf8_decode(input.stdin)",
        "utf8_decode(input.stdin, input.stdin)",
    );
    assert_eq!(
        run_guest(&extra_argument),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid let utf8_decode must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_error_return_variant_construct_byte_exactly() {
    let source = "record ScriptInput:\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction probe(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  if sico.bytes.is_utf8(input.stdin):\n    return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"A\"))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("error-return variant/construct must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let u64_signature = source
        .replace("returns Result[ScriptOutput, ScriptError]", "returns U64")
        .replace("message: \"source is not UTF-8\"", "message: \"x\"");
    let u64_gw_body = u64_signature.replace(
        "    return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"A\"))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"x\"))",
        "    return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"A\"))\n  end if\n  return U64.literal(0)",
    );
    assert_eq!(
        run_guest(&u64_gw_body),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let wrong_case = source.replace(
        "ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"",
        "ScriptErrorCode.ResourceLimit, message: \"source is not UTF-8\"",
    );
    assert_eq!(
        run_guest(&wrong_case),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let swapped_fields = source.replace(
        "code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"",
        "message: \"source is not UTF-8\", code: ScriptErrorCode.InvalidInput",
    );
    assert_eq!(
        run_guest(&swapped_fields),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid error-return must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_full_formatter_byte_exactly() {
    let expected = rust_ir(FORMATTER_SOURCE);
    let RunOutcome::Output(output) = run_guest(FORMATTER_SOURCE) else {
        panic!("full formatter source must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_ok_return_construct_byte_exactly() {
    let source = "record ScriptInput:\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction source_has_lex_error(source: Text) returns Bool:\n  return true\nend function\n\nfunction normalize_source(source: Text) returns Text:\n  return source\nend function\n\nfunction probe(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    if source_has_lex_error(source):\n      return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))\n    end if\n    let normalized = normalize_source(source)\n    return ok(ScriptOutput(stdout: sico.text.encode(normalized), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("ok-return construct must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let swapped_fields = source.replace(
        "stdout: sico.text.encode(normalized), stderr: sico.text.encode(\"\")",
        "stderr: sico.text.encode(normalized), stdout: sico.text.encode(\"\")",
    );
    assert_eq!(
        run_guest(&swapped_fields),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let wrong_construct = source.replace("ok(ScriptOutput(stdout:", "ok(ScriptError(code:");
    assert_eq!(
        run_guest(&wrong_construct),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TARGET".into(),
        }
    );
    let literal_stdout = source.replace(
        "stdout: sico.text.encode(normalized)",
        "stdout: sico.text.encode(\"\")",
    );
    assert_eq!(
        run_guest(&literal_stdout),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-ARGUMENT".into(),
        }
    );
    let u64_signature = source
        .replace("returns Result[ScriptOutput, ScriptError]", "returns U64")
        .replace(
            "      return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))",
            "      return U64.literal(1)",
        )
        .replace(
            "  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))",
            "  return U64.literal(0)",
        );
    assert_eq!(
        run_guest(&u64_signature),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let missing_paren = source.replace("exit_code: I64.literal(0)))", "exit_code: I64.literal(0))");
    assert_eq!(
        run_guest(&missing_paren),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let u64_exit_code = source.replace("exit_code: I64.literal(0)", "exit_code: U64.literal(0)");
    assert_eq!(
        run_guest(&u64_exit_code),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid ok-return must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_main_condition_byte_exactly() {
    let source = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nfunction probe(input: ScriptInput) returns U64:\n  if sico.bytes.is_utf8(input.stdin):\n    return U64.literal(1)\n  end if\n  return U64.literal(0)\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = &outcome else {
        panic!("main-entry is_utf8 condition must compile: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let wrong_field = source.replace("input.stdin):", "input.arguments):");
    assert_eq!(
        run_guest(&wrong_field),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let bare_parameter = source.replace("input.stdin):", "input):");
    assert_eq!(
        run_guest(&bare_parameter),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-ARGUMENT".into(),
        }
    );
    let extra_argument = source.replace("input.stdin):", "input.stdin, input.stdin):");
    assert_eq!(
        run_guest(&extra_argument),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-ARITY".into(),
        }
    );
    let bytes_parameter = source
        .replace(
            "function probe(input: ScriptInput) returns U64:",
            "function probe(input: ScriptInput, data: Bytes) returns U64:",
        )
        .replace("is_utf8(input.stdin):", "is_utf8(data.stdin):");
    assert_eq!(
        run_guest(&bytes_parameter),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into(),
        }
    );
    let other_intrinsic = source.replace("sico.bytes.is_utf8", "sico.bytes.length");
    assert_eq!(
        run_guest(&other_intrinsic),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-SCICOND".into(),
        }
    );
    let RunOutcome::Output(recovered) = run_guest(source) else {
        panic!("valid main-entry condition must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
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
fn sico_compiler_lowers_parser_is_ident_byte_prefix_byte_exactly() {
    // STEP-0432: parser.sico is now topologically ordered (callees precede
    // callers; the guest frontend refuses forward-referenced user-function
    // calls). `lex_words` therefore sits at the bottom of the module and is
    // no longer a small-prefix anchor. Pin the prefix through the function
    // right after `is_ident_byte` instead; the expected IR is derived from
    // the same source, so the byte-exact property is unchanged.
    let through = PARSER_SOURCE
        .find("function record_named_type")
        .expect("parser keeps the is_ident_byte prefix");
    let offset = PARSER_SOURCE.len() - PARSER_SOURCE.split_once('\n').unwrap().1.len();
    let source = &PARSER_SOURCE[offset..through];
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("parser is_ident_byte prefix must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_control_if_decline_routes_to_general_machine() {
    let source = "function probe(b: U64) returns U64:\n  if U64.less_than(b, U64.literal(95)):\n    return U64.literal(1)\n  end if\n  return U64.literal(0)\nend function\n";
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("if-return/end-if/return must route through the general machine: {source}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let with_else = source.replace(
        "    return U64.literal(1)\n  end if\n",
        "    return U64.literal(1)\n  else:\n    return U64.literal(2)\n  end if\n",
    );
    let RunOutcome::Output(else_output) = run_guest(&with_else) else {
        panic!("if/else/return dual-guard must compile: {with_else}")
    };
    assert_eq!(else_output.exit_code, 0);
    assert_bytes_equal(&else_output.stdout, rust_ir(&with_else).as_bytes());

    let string_guard = "function probe(b: U64) returns Bool:\n  if U64.equal(b, U64.literal(0)):\n    return true\n  end if\n  return false\nend function\n";
    let RunOutcome::Output(bool_output) = run_guest(string_guard) else {
        panic!("Bool guard chain must compile: {string_guard}")
    };
    assert_eq!(bool_output.exit_code, 0);
    assert_bytes_equal(&bool_output.stdout, rust_ir(string_guard).as_bytes());
}

#[test]
fn sico_compiler_lowers_list_get_literal_index_match_byte_exactly() {
    let source = "function probe(words: List[Text]) returns Text:\n  match sico.list.get(words, U64.literal(0)):\n    case ok(w):\n      return w\n    case error(_):\n      return \"?\"\n  end match\nend function\n";
    let expected = rust_ir(source);
    let RunOutcome::Output(output) = run_guest(source) else {
        panic!("literal-index list.get match must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let parameter_index = "function probe(words: List[Text], i: U64) returns Text:\n  match sico.list.get(words, i):\n    case ok(w):\n      return w\n    case error(_):\n      return \"?\"\n  end match\nend function\n";
    let RunOutcome::Output(param_output) = run_guest(parameter_index) else {
        panic!("parameter-index list.get match must compile")
    };
    assert_eq!(param_output.exit_code, 0);
    assert_bytes_equal(&param_output.stdout, rust_ir(parameter_index).as_bytes());

    let literal_in_two_param = parameter_index.replace(
        "sico.list.get(words, i)",
        "sico.list.get(words, U64.literal(0))",
    );
    assert_eq!(
        run_guest(&literal_in_two_param),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-PARAMETERS".into(),
        }
    );
    let negative_literal = source.replace("U64.literal(0)", "U64.literal(-1)");
    assert_eq!(
        run_guest(&negative_literal),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let trailing_operand = source.replace(
        "sico.list.get(words, U64.literal(0))",
        "sico.list.get(words, U64.literal(0), U64.literal(1))",
    );
    assert_eq!(
        run_guest(&trailing_operand),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into(),
        }
    );
    let RunOutcome::Output(recovered_output) = run_guest(source) else {
        panic!("valid literal-index match must compile after typed refusals")
    };
    assert_bytes_equal(&recovered_output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_checked_arithmetic_set_arm_match_byte_exactly() {
    // STEP-0318: `let <cell> = T.literal(k)` followed by an all-scalar
    // `T.checked_add` match whose ok arm is `set <cell> = <bind>`, an
    // error-arm return and a join return. Operands cover parameter, cell
    // and literal forms on both sides, both scalar kinds, one and two
    // parameters, parameter/literal fallbacks and cell/parameter tails.
    let cases = [
        "function probe(seed: U64) returns U64:\n  let string_len = U64.literal(0)\n  match U64.checked_add(string_len, U64.literal(1)):\n    case ok(next):\n      set string_len = next\n    case error(_):\n      return U64.literal(0)\n  end match\n  return string_len\nend function\n",
        "function probe(seed: U64) returns U64:\n  let acc = U64.literal(0)\n  match U64.checked_add(seed, acc):\n    case ok(next):\n      set acc = next\n    case error(_):\n      return seed\n  end match\n  return acc\nend function\n",
        "function probe(seed: I64) returns I64:\n  let total = I64.literal(7)\n  match I64.checked_add(total, I64.literal(2)):\n    case ok(next):\n      set total = next\n    case error(_):\n      return I64.literal(0)\n  end match\n  return total\nend function\n",
        "function probe(seed: U64, other: U64) returns U64:\n  let acc = U64.literal(0)\n  match U64.checked_add(seed, other):\n    case ok(next):\n      set acc = next\n    case error(_):\n      return other\n  end match\n  return seed\nend function\n",
        "function probe(seed: U64) returns U64:\n  let acc = U64.literal(0)\n  match U64.checked_add(seed, U64.literal(3)):\n    case ok(next):\n      set acc = next\n    case error(_):\n      return U64.literal(0)\n  end match\n  return seed\nend function\n",
        "function probe(seed: U64, other: U64) returns U64:\n  let acc = U64.literal(4)\n  match U64.checked_add(acc, U64.literal(5)):\n    case ok(next):\n      set acc = next\n    case error(_):\n      return U64.literal(0)\n  end match\n  return acc\nend function\n",
    ];
    for source in cases {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("set-arm checked_add match must compile: {source}")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

#[test]
fn sico_compiler_refuses_invalid_set_arm_match_shapes_with_typed_identity() {
    let base = "function probe(seed: U64) returns U64:\n  let acc = U64.literal(0)\n  match U64.checked_add(acc, U64.literal(1)):\n    case ok(next):\n      set acc = next\n    case error(_):\n      return U64.literal(0)\n  end match\n  return acc\nend function\n";
    let mutations: [(&str, String); 11] = [
        ("sub-op", base.replacen("checked_add", "checked_sub", 1)),
        (
            "type-mismatch-operand",
            base.replace(
                "function probe(seed: U64) returns U64:",
                "function probe(seed: I64, extra: U64) returns U64:",
            )
            .replacen("U64.checked_add(acc,", "U64.checked_add(seed,", 1),
        ),
        (
            "overflow-u64",
            base.replacen("U64.literal(1)", "U64.literal(18446744073709551616)", 1),
        ),
        (
            "non-canonical",
            base.replacen("U64.literal(1)", "U64.literal(01)", 1),
        ),
        (
            "cross-kind-negative",
            base.replacen("U64.literal(1)", "I64.literal(-7)", 1),
        ),
        ("cell-shadow", base.replacen("let acc = ", "let seed = ", 1)),
        (
            "three-operands",
            base.replacen(
                "(acc, U64.literal(1))",
                "(acc, U64.literal(1), U64.literal(2))",
                1,
            ),
        ),
        (
            "nested-call-operand",
            base.replacen(
                "U64.checked_add(acc,",
                "U64.checked_add(U64.checked_add(acc, U64.literal(1)),",
                1,
            ),
        ),
        (
            "set-other-cell",
            base.replacen("set acc = next", "set other = next", 1),
        ),
        (
            "wrong-return-kind",
            base.replacen("returns U64:", "returns Bool:", 1),
        ),
        (
            "wildcard-binding",
            base.replacen("case ok(next):", "case ok(_):", 1),
        ),
    ];
    let expected_codes = [
        "ERR:E-SH-IR-CALL-TARGET",
        "ERR:E-SH-IR-CALL-TYPE",
        "ERR:E-SH-IR-U64-RANGE",
        "ERR:E-SH-IR-CALL-SHAPE",
        "ERR:E-SH-IR-CALL-TYPE",
        "ERR:E-SH-IR-CELL-SHADOW",
        "ERR:E-SH-IR-EXPRESSION",
        "ERR:E-SH-IR-EXPRESSION",
        "ERR:E-SH-IR-MATCH-SHAPE",
        "ERR:E-SH-IR-TYPE-MISMATCH",
        "ERR:E-SH-IR-MATCH-SHAPE",
    ];
    for ((name, source), code) in mutations.into_iter().zip(expected_codes) {
        assert_eq!(
            run_guest(&source),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: code.into(),
            },
            "mutation {name} must stay typed-refused"
        );
    }
    // The same prepared compiler recovers byte-exactly after the refusals.
    let expected = rust_ir(base);
    let RunOutcome::Output(recovered) = run_guest(base) else {
        panic!("valid set-arm match must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_parser_rearranged_function_subsets_byte_exactly() {
    // STEP-0319: per-function differential coverage beyond the contiguous
    // prefix — the exact frozen function text of parser.sico functions that
    // existing families already lower, byte-compared against the Rust
    // oracle. `next_index` exercises the returning checked_add family,
    // `word_at` the list.get returning-match family and
    // `valid_escape_byte` the control-if guard-chain family.
    // STEP-0320 adds `line_end` (while + if + user-call consumers) with its
    // flipped callees reordered first.
    for name in ["next_index", "word_at", "valid_escape_byte"] {
        let source = format!("{}\n", parser_function_text(name));
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    let mut line_end_source = String::new();
    for callee in ["is_word", "word_at", "next_index"] {
        line_end_source.push_str(&parser_function_text(callee));
        line_end_source.push('\n');
    }
    line_end_source.push_str(&parser_function_text("line_end"));
    line_end_source.push('\n');
    let expected = rust_ir(&line_end_source);
    let RunOutcome::Output(output) = run_guest(&line_end_source) else {
        panic!("line_end with its callees must compile through existing families")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // STEP-0321: census-nominated loop-free dispatch tables flip with the
    // same callee prefix the probe used.
    for name in ["scalar_type_kind", "fixed_op_name"] {
        let mut source = line_end_source.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0324: matching_close flips through the widened gw match subject
    // (checked_add/checked_sub in nested if regions).
    let mut matching_close_source = line_end_source.clone();
    matching_close_source.push_str(&parser_function_text("matching_close"));
    matching_close_source.push('\n');
    let expected = rust_ir(&matching_close_source);
    let RunOutcome::Output(output) = run_guest(&matching_close_source) else {
        panic!("matching_close with its callees must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // STEP-0327: five more parser functions flip with the same callee
    // prefix — decimal_at_most additionally exercises the widened
    // bytes.at cell-cursor emission (cursor argument id fix).
    for name in [
        "binding_parameter_index",
        "parameter_type_word",
        "script_result_return_json",
        "decimal_at_most",
        "return_expression_end",
    ] {
        let mut source = line_end_source.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0338: the guard-chain machine gains host-call lets (gw_rhs_packed
    // reuse), cell/host-call guard operands (chain_condition_packed with the
    // bytes.at literal-cursor form verified against the oracle) and locals
    // emission — is_digit_token and is_string_token flip byte-exactly.
    // is_ident_byte flips through the control_if-declined general-while
    // retry. All three use only intrinsics, so no callee prefix is needed.
    for name in ["is_digit_token", "is_string_token", "is_ident_byte"] {
        let source = format!("{}\n", parser_function_text(name));
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through the widened guard chain")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0339: full-function census over the widened machines — seven more
    // parser functions flip with the line_end callee prefix, and two more
    // (function_declaration_index/ordinal) with the extended eleven-function
    // prefix that includes the pass-2 flips.
    for name in [
        "is_declaration_start",
        "literal_value_index",
        "list_aware_parameter_index",
        "is_intrinsic_path",
        "record_scalar_kind",
        "record_close_bracket",
        "record_map_comma",
    ] {
        let mut source = line_end_source.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    let mut census_source = line_end_source.clone();
    for callee in [
        "is_declaration_start",
        "literal_value_index",
        "list_aware_parameter_index",
        "is_intrinsic_path",
        "record_scalar_kind",
        "record_close_bracket",
        "record_map_comma",
    ] {
        census_source.push_str(&parser_function_text(callee));
        census_source.push('\n');
    }
    for name in ["function_declaration_index", "function_declaration_ordinal"] {
        let mut source = census_source.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0340: sico.text.starts_with conditions in the general-while
    // machine (gw_starts_with_condition_packed: receiver param/cell/call via
    // gw_rhs_packed, string-literal argument, intrinsic bool condition) —
    // the most common guard shape in parser.sico (185 sites). Five more
    // functions flip: is_punct_byte and type_json with the fourteen-function
    // census prefix, declared_return_kind with the fifteen-function prefix,
    // and binding_parameter_kind/parameter_kind_by_id with the
    // twenty-eight-function all-flipped prefix.
    let mut census14 = census_source.clone();
    for callee in [
        "function_declaration_index",
        "function_declaration_ordinal",
        "is_ident_byte",
    ] {
        census14.push_str(&parser_function_text(callee));
        census14.push('\n');
    }
    for name in ["is_punct_byte", "type_json"] {
        let mut source = census14.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    let mut census15 = census14.clone();
    census15.push_str(&parser_function_text("scalar_type_kind"));
    census15.push('\n');
    let mut source = census15.clone();
    source.push_str(&parser_function_text("declared_return_kind"));
    source.push('\n');
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("declared_return_kind must compile through existing families")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let mut census28 = census15.clone();
    for callee in [
        "is_punct_byte",
        "is_digit_token",
        "is_string_token",
        "valid_escape_byte",
        "matching_close",
        "decimal_at_most",
        "binding_parameter_index",
        "parameter_type_word",
        "script_result_return_json",
        "fixed_op_name",
        "return_expression_end",
        "type_json",
        "declared_return_kind",
    ] {
        census28.push_str(&parser_function_text(callee));
        census28.push('\n');
    }
    for name in ["binding_parameter_kind", "parameter_kind_by_id"] {
        let mut source = census28.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0341: the general-while machine's match state becomes a stack
    // (gmt_stack/gmt_depth push on nested open, pop on end match) and the
    // match>1-without-while ladder gate routes to the general machine
    // instead of refusing CONTROL. count_params (list.get match with a
    // nested checked_sub match inside an if-then, inside a while) flips
    // byte-exactly with the line_end callee prefix.
    {
        let mut source = line_end_source.clone();
        source.push_str(&parser_function_text("count_params"));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("count_params must compile through the match-stack machine")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0342: the deferred-join emission no longer reuses gmt_len as its
    // scan variable (gdefer_scan instead), so the match-stack length survives
    // deferred-block assembly. find_body_end and count_until_end_function —
    // while + match with an if/else whose BOTH arms return inside the match
    // arm, the post-dominated return join — flip byte-exactly with the same
    // line_end callee prefix.
    for name in ["find_body_end", "count_until_end_function"] {
        let mut source = line_end_source.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through the match-stack machine")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0343: two stack bookkeeping repairs in the general machine —
    // the while-frame stack and the match stack are physically truncated on
    // pop (previously only their length counters shrank, so a sibling
    // while/match re-push read a stale frame slot). expression_shape_depth
    // (two sequential top-level whiles + recursive self-call in a concat
    // argument), expression_shape and return_shapes flip byte-exactly with
    // the seven-function callee prefix; the chain composes cumulatively
    // (expression_shape wraps depth, return_shapes calls expression_shape).
    let mut shape_source = line_end_source.clone();
    for callee in ["is_string_token", "is_digit_token", "matching_close"] {
        shape_source.push_str(&parser_function_text(callee));
        shape_source.push('\n');
    }
    for name in [
        "expression_shape_depth",
        "expression_shape",
        "return_shapes",
    ] {
        shape_source.push_str(&parser_function_text(name));
        shape_source.push('\n');
        let expected = rust_ir(&shape_source);
        let RunOutcome::Output(output) = run_guest(&shape_source) else {
            panic!("parser chain through {name} must compile through the general machine")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0344: straight machines accept sico.text.concat right-hand sides
    // via the shared straight_text_concat_packed helper (nested concat, a
    // sico.u64.to_text argument, string literals, params, and locals in ssa
    // or cell mode), wired into straight_environment let/return positions
    // and straight_cell set/return positions. fn_entry (eight counter lets
    // + seven concat sets with nested to_text + concat return) flips on the
    // cumulative STEP-0343 chain; the two instruction-json helpers flip
    // standalone.
    let mut fn_entry_source = shape_source.clone();
    fn_entry_source.push_str(&parser_function_text("count_until_end_function"));
    fn_entry_source.push('\n');
    fn_entry_source.push_str(&parser_function_text("fn_entry"));
    fn_entry_source.push('\n');
    let expected = rust_ir(&fn_entry_source);
    let RunOutcome::Output(output) = run_guest(&fn_entry_source) else {
        panic!("fn_entry must compile through the straight machines")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    for name in [
        "scalar_const_instruction_json",
        "write_local_instruction_json",
    ] {
        let source = format!("{}\n", parser_function_text(name));
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("helper {name} must compile standalone")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }

    // STEP-0345: gw_condition_packed routes I64/U64 receivers through
    // chain_condition_packed first (host-call left operands — bytes.at /
    // text.length / bytes.length — already byte-proven there) and falls
    // back to gw_compare_packed on SKIP. is_canonical_magnitude flips with
    // the four-function line_end prefix; the census over the forty-function
    // flipped prefix adds constant_argument_kind, fixed_literal_argument
    // and comma_count on pass one, and fixed_operation_operand_width on
    // pass two over the pass-one flips (census compositions frozen
    // exactly).
    let mut magnitude_source = line_end_source.clone();
    magnitude_source.push_str(&parser_function_text("is_canonical_magnitude"));
    magnitude_source.push('\n');
    let expected = rust_ir(&magnitude_source);
    let RunOutcome::Output(output) = run_guest(&magnitude_source) else {
        panic!("is_canonical_magnitude must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let mut census40 = String::new();
    for callee in [
        "is_word",
        "word_at",
        "next_index",
        "line_end",
        "is_declaration_start",
        "literal_value_index",
        "list_aware_parameter_index",
        "is_intrinsic_path",
        "record_scalar_kind",
        "record_close_bracket",
        "record_map_comma",
        "function_declaration_index",
        "function_declaration_ordinal",
        "is_ident_byte",
        "scalar_type_kind",
        "is_punct_byte",
        "is_digit_token",
        "is_string_token",
        "valid_escape_byte",
        "matching_close",
        "decimal_at_most",
        "binding_parameter_index",
        "parameter_type_word",
        "script_result_return_json",
        "fixed_op_name",
        "return_expression_end",
        "type_json",
        "declared_return_kind",
        "binding_parameter_kind",
        "parameter_kind_by_id",
        "count_params",
        "find_body_end",
        "count_until_end_function",
        "expression_shape_depth",
        "expression_shape",
        "return_shapes",
        "fn_entry",
        "is_canonical_magnitude",
        "scalar_const_instruction_json",
        "write_local_instruction_json",
    ] {
        census40.push_str(&parser_function_text(callee));
        census40.push('\n');
    }
    for name in [
        "constant_argument_kind",
        "fixed_literal_argument",
        "comma_count",
    ] {
        let mut source = census40.clone();
        source.push_str(&parser_function_text(name));
        source.push('\n');
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    let mut census44 = census40.clone();
    for callee in [
        "constant_argument_kind",
        "fixed_literal_argument",
        "comma_count",
    ] {
        census44.push_str(&parser_function_text(callee));
        census44.push('\n');
    }
    let mut source = census44;
    source.push_str(&parser_function_text("fixed_operation_operand_width"));
    source.push('\n');
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("fixed_operation_operand_width must compile through existing families")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // STEP-0346: chain_condition_packed (the general machine's chained
    // comparison condition path) accepts nested sico.bytes.at call operands —
    // the left operand dispatches to the parameter-source packer when the
    // first argument is a bytes parameter (previously cell-only, refusing
    // CALL-ARGUMENT), and the right operand accepts a nested bytes.at call in
    // addition to parameter/cell/literal. Five functions flip on a cumulative
    // callee chain: has_return_at, parameter_name_start and find_text_from
    // (the bisected shapes; find_text_from exercises both operands as nested
    // calls) plus return_expression_start and local_binding_index for free
    // from the census fixpoint.
    let mut call_operand_source = String::new();
    for callee in ["is_word", "word_at", "next_index", "is_ident_byte"] {
        call_operand_source.push_str(&parser_function_text(callee));
        call_operand_source.push('\n');
    }
    for name in [
        "has_return_at",
        "parameter_name_start",
        "find_text_from",
        "return_expression_start",
        "local_binding_index",
    ] {
        call_operand_source.push_str(&parser_function_text(name));
        call_operand_source.push('\n');
        let expected = rust_ir(&call_operand_source);
        let RunOutcome::Output(output) = run_guest(&call_operand_source) else {
            panic!("parser chain through {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    // Minimal regression guard: the bytes-parameter left operand of an
    // I64.equal condition inside a while body, the exact bisected shape.
    let probe = format!(
        "{}function probe(src: Bytes, from_i: U64) returns Bool:\n  let n = sico.bytes.length(src)\n  let i = from_i\n  while U64.less_than(i, n):\n    if I64.equal(sico.bytes.at(src, i, I64.literal(0)), I64.literal(114)):\n      return true\n    end if\n    set i = next_index(i, n)\n  end while\n  return false\nend function\n",
        call_operand_source
    );
    let expected = rust_ir(&probe);
    let RunOutcome::Output(output) = run_guest(&probe) else {
        panic!("bytes-parameter compare operand must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_slice_plain_subjects_and_utf8_decode_args_byte_exactly() {
    // STEP-0347: the general machine's bytes.slice match subject accepts a
    // plain third argument — a u64 cell and an X.literal(N) const span
    // (previously only sub(bytes.length(...)) and two-argument user calls,
    // refusing CALL-TARGET) — and the text.concat argument grammar accepts a
    // nested sico.bytes.utf8_decode call (gw_intrinsic_call_packed reuse,
    // mirroring the trim branch). json_string_token's remaining frontier is
    // the after-region match/if (STATEMENT), a separate CFG slice.
    let base = format!("{}\n", parser_function_text("next_index"));
    // Slice match subject with a plain cell third argument, ok-arm binding
    // feeding utf8_decode in a concat — the json_string_token in-loop shape.
    let g2 = format!(
        "{base}function probe2(src: Bytes) returns Text:\n  let n = sico.bytes.length(src)\n  let start = U64.literal(0)\n  let len = U64.literal(1)\n  let out = \"\"\n  let i = U64.literal(0)\n  while U64.less_than(i, n):\n    match sico.bytes.slice(src, start, len):\n      case ok(seg):\n        set out = sico.text.concat(out, sico.bytes.utf8_decode(seg))\n      case error(_):\n        set out = \"e\"\n    end match\n    set i = next_index(i, n)\n  end while\n  return out\nend function\n"
    );
    let expected = rust_ir(&g2);
    let RunOutcome::Output(output) = run_guest(&g2) else {
        panic!("slice plain-cell subject must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // Slice match subject with an X.literal(N) third argument — const_u64
    // span emission via fixed_literal_const_instruction.
    let g3 = format!(
        "{base}function probe3(src: Bytes) returns Text:\n  let out = \"\"\n  let i = U64.literal(0)\n  let n = sico.bytes.length(src)\n  while U64.less_than(i, n):\n    match sico.bytes.slice(src, i, U64.literal(2)):\n      case ok(seg):\n        set out = sico.bytes.utf8_decode(seg)\n      case error(_):\n        set out = \"e\"\n    end match\n    set i = next_index(i, n)\n  end while\n  return out\nend function\n"
    );
    let expected = rust_ir(&g3);
    let RunOutcome::Output(output) = run_guest(&g3) else {
        panic!("slice literal-arg3 subject must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // text.concat argument grammar accepting a nested utf8_decode call.
    let g1 = format!(
        "{base}function probe1(token: Text) returns Text:\n  let bytes = sico.text.encode(token)\n  let n = sico.bytes.length(bytes)\n  let out = \"\"\n  let i = U64.literal(0)\n  while U64.less_than(i, n):\n    set out = sico.text.concat(out, sico.bytes.utf8_decode(bytes))\n    set i = next_index(i, n)\n  end while\n  return out\nend function\n"
    );
    let expected = rust_ir(&g1);
    let RunOutcome::Output(output) = run_guest(&g1) else {
        panic!("concat utf8_decode argument must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_after_region_match_and_if_byte_exactly() {
    // STEP-0348: the general machine accepts match and if statements in the
    // after region (the block following a closed while) — the region gates in
    // gw_match_open_pack and the if handler were conservative refusals; the
    // block assembly is region-agnostic (case arms reuse the current region
    // string, end-match seals the join, end-if restores to join). The
    // json_string_token tail (match, then if-wrapped match) was the last gate;
    // the exact frozen text now flips byte-exactly.
    let base = format!("{}\n", parser_function_text("next_index"));
    // match directly in the after region.
    let a1 = format!(
        "{base}function probe1(src: Bytes) returns Text:\n  let n = sico.bytes.length(src)\n  let out = \"\"\n  let i = U64.literal(0)\n  while U64.less_than(i, n):\n    set out = sico.text.concat(out, \"x\")\n    set i = next_index(i, n)\n  end while\n  match sico.bytes.slice(src, i, U64.literal(1)):\n    case ok(seg):\n      set out = sico.bytes.utf8_decode(seg)\n    case error(_):\n      set out = \"e\"\n  end match\n  return out\nend function\n"
    );
    let expected = rust_ir(&a1);
    let RunOutcome::Output(output) = run_guest(&a1) else {
        panic!("match in the after region must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // if/else directly in the after region.
    let a2 = format!(
        "{base}function probe2(src: Bytes) returns Text:\n  let n = sico.bytes.length(src)\n  let out = \"\"\n  let i = U64.literal(0)\n  while U64.less_than(i, n):\n    set i = next_index(i, n)\n  end while\n  if U64.equal(i, n):\n    set out = \"done\"\n  else:\n    set out = \"mid\"\n  end if\n  return out\nend function\n"
    );
    let expected = rust_ir(&a2);
    let RunOutcome::Output(output) = run_guest(&a2) else {
        panic!("if in the after region must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // if + nested match in the after region — the json_string_token tail shape.
    let a3 = format!(
        "{base}function probe3(src: Bytes) returns Text:\n  let n = sico.bytes.length(src)\n  let out = \"\"\n  let i = U64.literal(0)\n  while U64.less_than(i, n):\n    set i = next_index(i, n)\n  end while\n  match sico.bytes.slice(src, i, U64.literal(1)):\n    case ok(seg):\n      set out = sico.bytes.utf8_decode(seg)\n    case error(_):\n      set out = \"e\"\n  end match\n  let tail_len = U64.literal(0)\n  match U64.checked_sub(n, i):\n    case ok(length):\n      set tail_len = length\n    case error(_):\n      set tail_len = U64.literal(0)\n  end match\n  if U64.less_than(U64.literal(0), tail_len):\n    match sico.bytes.slice(src, i, tail_len):\n      case ok(tail):\n        set out = sico.text.concat(out, sico.bytes.utf8_decode(tail))\n      case error(_):\n        set out = \"t\"\n    end match\n  end if\n  return out\nend function\n"
    );
    let expected = rust_ir(&a3);
    let RunOutcome::Output(output) = run_guest(&a3) else {
        panic!("if+match in the after region must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // The target: exact frozen json_string_token text (tail match + tail if).
    let jsrc = format!("{base}{}", parser_function_text("json_string_token"));
    let expected = rust_ir(&jsrc);
    let RunOutcome::Output(output) = run_guest(&jsrc) else {
        panic!("json_string_token must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_text_list_binary_nested_args_byte_exactly() {
    // STEP-0349: gw_text_list_binary_packed (sico.text.join / sico.list.append
    // packer) accepts multi-token second arguments — the arity gate required a
    // single atom and both argument spans passed their start offset as the
    // range end, emitting zero-width ranges for nested calls. The widened
    // gate requires at least one token; argument range ends now come from the
    // last token of each argument (the STEP-0347 slice-argument pattern).
    // lex_words flips byte-exactly (List[Text] appends with nested
    // utf8_decode-of-slice calls inside match ok arms and nested if/else).
    let base = format!(
        "{}\n{}\n{}\n{}\n",
        parser_function_text("next_index"),
        parser_function_text("valid_escape_byte"),
        parser_function_text("is_ident_byte"),
        parser_function_text("is_punct_byte"),
    );
    // List[Text] append with a string literal, in a while body.
    let l1 = format!(
        "{base}function probe1(src: Bytes) returns List[Text]:\n  let n = sico.bytes.length(src)\n  let words = sico.text.split_lines(\"\")\n  let i = U64.literal(0)\n  while U64.less_than(i, n):\n    set words = sico.list.append(words, \"<nl>\")\n    set i = next_index(i, n)\n  end while\n  return words\nend function\n"
    );
    let expected = rust_ir(&l1);
    let RunOutcome::Output(output) = run_guest(&l1) else {
        panic!("List[Text] append with a literal must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // Call in return position (split_lines with a literal).
    let l2 = format!(
        "{base}function probe2(src: Bytes) returns List[Text]:\n  let words = sico.text.split_lines(\"\")\n  if U64.equal(sico.bytes.length(src), U64.literal(0)):\n    return sico.text.split_lines(\"ERR-x\")\n  end if\n  return words\nend function\n"
    );
    let expected = rust_ir(&l2);
    let RunOutcome::Output(output) = run_guest(&l2) else {
        panic!("split_lines in return position must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // Append with a nested utf8_decode-of-slice call, match ok arm in a while
    // body — the lex_words inner shape.
    let l3 = format!(
        "{base}function probe3(src: Bytes) returns List[Text]:\n  let words = sico.text.split_lines(\"\")\n  let i = U64.literal(0)\n  let n = sico.bytes.length(src)\n  while U64.less_than(i, n):\n    match sico.bytes.slice(src, i, U64.literal(1)):\n      case ok(seg):\n        set words = sico.list.append(words, sico.bytes.utf8_decode(seg))\n      case error(_):\n        set words = sico.text.split_lines(\"ERR-slice\")\n    end match\n    set i = next_index(i, n)\n  end while\n  return words\nend function\n"
    );
    let expected = rust_ir(&l3);
    let RunOutcome::Output(output) = run_guest(&l3) else {
        panic!("append with a nested call must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // The target: exact frozen lex_words text (the module lexer).
    let lsrc = format!("{base}{}", parser_function_text("lex_words"));
    let expected = rust_ir(&lsrc);
    let RunOutcome::Output(output) = run_guest(&lsrc) else {
        panic!("lex_words must compile through the general machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_straight_concat_user_call_args_byte_exactly() {
    // STEP-0350: straight_text_concat_packed's argument grammar accepts a
    // single-argument user call (type_json(kind)) — param, cell-mode
    // read_local, ssa-mode value-forward and string-literal arguments, with
    // the call emitted through call_instruction_json at the callee's ordinal.
    // Three JSON-constructor functions flip. Frozen compositions avoid the
    // pre-existing sequential-lowering trap (see the STEP for the frontier).
    let tj = format!("{}\n", parser_function_text("type_json"));
    for name in [
        "read_local_instruction_json",
        "call_instruction_json",
        "intrinsic_instruction_json",
    ] {
        let source = format!("{}{}\n", tj, parser_function_text(name));
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    // Minimal regression guard: the bisected shape — concat with a user call
    // taking a Text param, in a set statement (scc machine).
    let rl = parser_function_text("read_local_instruction_json");
    let guard = format!(
        "{tj}{rl}function probe_guard(kind: Text) returns Text:\n  let out = \"\"\n  set out = sico.text.concat(\"a:\", type_json(kind))\n  return out\nend function\n"
    );
    let expected = rust_ir(&guard);
    let RunOutcome::Output(output) = run_guest(&guard) else {
        panic!("concat with a user-call argument must compile through the cell machine")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_guest_arena_accommodates_full_parser_selfhost_input() {
    // STEP-0351 → STEP-0454: the script profile's guest linear-memory ceiling
    // rose 64 MiB (1,024 pages) → 128 MiB (2,048) → 256 MiB (4,096) → 512 MiB
    // (8,192) → 2 GiB (33,792 pages) — RFC-0029 budget-table amendments
    // (STEP-0351/0402/0421/0454). The self-host compiler's own lowering
    // workspace (monotonic bump-arena allocations across a whole-module
    // lowering, never reset between functions) exhausted the ceiling mid-run,
    // surfacing as a bare wasm `unreachable` in the generated realloc instead
    // of a typed refusal. STEP-0484 added the per-function arena treadmill, so
    // the ceiling is now a backstop rather than the working budget. Guards:
    // (1) the previously-trapping 53-function composition lowers byte-exactly;
    // (2) the FULL parser.sico text lowers through every function to a clean
    // exit 0 byte-exact result against the Rust oracle — never a trap, never a
    // typed refusal.
    let mut composition = String::new();
    for name in [
        "type_json",
        "read_local_instruction_json",
        "is_word",
        "word_at",
        "next_index",
        "line_end",
        "is_declaration_start",
        "literal_value_index",
        "list_aware_parameter_index",
        "is_intrinsic_path",
        "record_scalar_kind",
        "record_close_bracket",
        "record_map_comma",
        "function_declaration_index",
        "function_declaration_ordinal",
        "is_ident_byte",
        "scalar_type_kind",
        "is_punct_byte",
        "is_digit_token",
        "is_string_token",
        "valid_escape_byte",
        "matching_close",
        "decimal_at_most",
        "binding_parameter_index",
        "parameter_type_word",
        "script_result_return_json",
        "fixed_op_name",
        "return_expression_end",
        "declared_return_kind",
        "binding_parameter_kind",
        "parameter_kind_by_id",
        "count_params",
        "find_body_end",
        "count_until_end_function",
        "expression_shape_depth",
        "expression_shape",
        "return_shapes",
        "fn_entry",
        "is_canonical_magnitude",
        "scalar_const_instruction_json",
        "write_local_instruction_json",
        "constant_argument_kind",
        "fixed_literal_argument",
        "comma_count",
        "fixed_operation_operand_width",
        "has_return_at",
        "parameter_name_start",
        "find_text_from",
        "return_expression_start",
        "local_binding_index",
        "json_string_token",
        "lex_words",
    ] {
        composition.push_str(&parser_function_text(name));
        composition.push('\n');
    }
    let fixture = format!(
        "{}{}\n",
        composition,
        parser_function_text("call_instruction_json")
    );
    let expected = rust_ir(&fixture);
    let RunOutcome::Output(output) = run_guest(&fixture) else {
        panic!("the 53-function composition must lower without exhausting the arena")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // The full self-host parser source: every function walks the ladder; the
    // first not-yet-supported shape must fail as a typed refusal. The whole
    // module walks all 127 functions, so this guard runs with a raised fuel
    // and time budget instead of the shared run_guest defaults.
    let source = format!("{PARSER_SOURCE}\n");
    // STEP-0495: the canonical IR of the whole module (~8.6 MB) no longer
    // fits one Script v0 channel (RFC-0029 freezes a channel at 8 MiB), so
    // the default single-channel path now ends in the deterministic transport
    // refusal instead of a lowering failure. The byte-exact comparison
    // therefore runs over the `--emit-ir-split` driver mode, which halves the
    // IR across the two channels; concatenating stdout and stderr restores
    // the exact module IR.
    let outcome = run_guest_with_args(&source, vec!["--emit-ir-split".to_owned()]);
    // STEP-0484 TREADMILL INVERSION: per-function arena reuse (RFC-0029
    // amendment) cleared the STEP-0365 memory wall. The self-host walk now
    // arms the scratch zone once, reclaims every function's garbage via
    // `sico.arena.push_chunk`, and joins the kept chunks once at the end —
    // peak memory is perm(live set) + max(per-function scratch) instead of
    // the monotonic sum. The pin inverts: the full source MUST lower to a
    // clean exit 0 byte-exact against the Rust oracle (any trap or typed
    // refusal fails), closing the M22 S6 full-source lowering gate.
    let RunOutcome::Output(output) = outcome else {
        panic!(
            "full parser.sico walk must lower cleanly after the STEP-0484 treadmill, not trap or refuse: {outcome:?}"
        )
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let expected = rust_ir(&source);
    let mut assembled = output.stdout.clone();
    assembled.extend_from_slice(&output.stderr);
    assert_bytes_equal(&assembled, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_compiler_parser_function_subsets_byte_exactly() {
    // STEP-0321: first compiler_parser.sico flips — `same` (byte compare
    // delegation) and `add`/`sub` (match-subject checked arithmetic), the
    // exact frozen function text against the Rust oracle.
    for name in ["same", "add", "sub"] {
        let start = COMPILER_PARSER_SOURCE
            .find(&format!("function {name}("))
            .expect("compiler_parser keeps the function");
        let end = COMPILER_PARSER_SOURCE[start..]
            .find("\nend function\n")
            .expect("function ends")
            + start
            + "\nend function\n".len();
        let source = &COMPILER_PARSER_SOURCE[start..end];
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("compiler_parser function {name} must compile through existing families")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

fn parser_function_text(name: &str) -> String {
    let start = PARSER_SOURCE
        .find(&format!("function {name}("))
        .expect("parser keeps the function");
    let end = PARSER_SOURCE[start..]
        .find("\nend function\n")
        .expect("function ends")
        + start
        + "\nend function\n".len();
    PARSER_SOURCE[start..end].to_owned()
}

#[test]
fn sico_compiler_lowers_gw_checked_arithmetic_match_subjects_byte_exactly() {
    // STEP-0324: the general-while machine accepts `I64/U64.checked_add`
    // and `checked_sub` match subjects inside statement regions (if/else
    // branches and loop bodies' nested regions), with parameter, cell and
    // literal operands — previously only `sico.map.get`/`sico.bytes.slice`
    // subjects were lowered and these refused CALL-TARGET. STEP-0353
    // widens the family with `checked_mul`/`checked_div` (same operand
    // shapes and Result kind; gw_decimal_value's shape).
    let base = format!(
        "{}\n{}\n{}\n{}\n",
        parser_function_text("is_word"),
        parser_function_text("word_at"),
        parser_function_text("next_index"),
        parser_function_text("line_end"),
    );
    let cases = [
        // set-arm checked_add match in an if branch, cell + literal operands.
        format!(
            "{base}function probe(words: List[Text], i: U64, end_idx: U64) returns U64:\n  let depth = U64.literal(0)\n  let w = word_at(words, i)\n  if is_word(w, \"(\"):\n    match U64.checked_add(depth, U64.literal(1)):\n      case ok(next):\n        set depth = next\n      case error(_):\n        return end_idx\n    end match\n  end if\n  return depth\nend function\n"
        ),
        // checked_sub subject with parameter and cell operands, nested if in
        // the ok arm — the matching_close shape.
        format!(
            "{base}function probe(words: List[Text], i: U64, end_idx: U64) returns U64:\n  let depth = U64.literal(2)\n  let w = word_at(words, i)\n  if is_word(w, \")\"):\n    match U64.checked_sub(depth, i):\n      case ok(next):\n        set depth = next\n        if U64.equal(depth, U64.literal(0)):\n          return i\n        end if\n      case error(_):\n        return end_idx\n    end match\n  end if\n  return depth\nend function\n"
        ),
        // STEP-0353: checked_mul joins the family (gw_decimal_value shape) —
        // cell * literal with the early return in the error arm.
        format!(
            "{base}function probe(words: List[Text], i: U64, end_idx: U64) returns U64:\n  let acc = U64.literal(1)\n  let w = word_at(words, i)\n  if is_word(w, \"(\"):\n    match U64.checked_mul(acc, U64.literal(10)):\n      case ok(next):\n        set acc = next\n      case error(_):\n        return end_idx\n    end match\n  end if\n  return acc\nend function\n"
        ),
        // matching_close itself, exact frozen text with its callees.
        format!("{}{}", base, parser_function_text("matching_close")),
    ];
    for source in &cases {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("gw checked-arithmetic match subject must compile")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    // Typed refusals: cross-kind operand, unknown operand, and an
    // out-of-family operation stay typed-refused, then recovery.
    let bad_type = format!(
        "{base}function probe(words: List[Text], seed: I64, end_idx: U64) returns U64:\n  let depth = U64.literal(0)\n  let w = word_at(words, end_idx)\n  if is_word(w, \"(\"):\n    match U64.checked_add(seed, U64.literal(1)):\n      case ok(next):\n        set depth = next\n      case error(_):\n        return end_idx\n    end match\n  end if\n  return depth\nend function\n"
    );
    assert_eq!(
        run_guest(&bad_type),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    // STEP-0353: checked_mul/div are in-family now; keep the out-of-family
    // guard on an operation that stays refused (bit_and).
    let bad_op = format!(
        "{base}function probe(words: List[Text], i: U64, end_idx: U64) returns U64:\n  let depth = U64.literal(0)\n  let w = word_at(words, i)\n  if is_word(w, \"(\"):\n    match U64.bit_and(depth, U64.literal(1)):\n      case ok(next):\n        set depth = next\n      case error(_):\n        return end_idx\n    end match\n  end if\n  return depth\nend function\n"
    );
    assert_eq!(
        run_guest(&bad_op),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TARGET".into()
        }
    );
    let good = &cases[0];
    let expected = rust_ir(good);
    let RunOutcome::Output(recovered) = run_guest(good) else {
        panic!("valid gw subject must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_list_get_match_subjects_byte_exactly() {
    // STEP-0325: the general-while machine accepts `sico.list.get(<param
    // list>, <index>)` match subjects inside statement regions — the fourth
    // in-body subject family — and the result temp local carries the
    // string-ok result type (new result-string kind; result-i64 added for
    // the checked-arithmetic subjects).
    let base = format!(
        "{}\n{}\n{}\n{}\n",
        parser_function_text("is_word"),
        parser_function_text("word_at"),
        parser_function_text("next_index"),
        parser_function_text("line_end"),
    );
    let good = format!(
        "{base}function probe(words: List[Text], i: U64, n: U64) returns U64:\n  if U64.less_than(i, n):\n    match sico.list.get(words, i):\n      case ok(w):\n        if is_word(w, \"end\"):\n          return i\n        end if\n      case error(_):\n        return n\n    end match\n  end if\n  return n\nend function\n"
    );
    let expected = rust_ir(&good);
    let RunOutcome::Output(output) = run_guest(&good) else {
        panic!("gw list.get match subject must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // Declared bounds: a list-cell operand (not a parameter) and an
    // out-of-family member stay typed-refused, then recovery.
    let list_cell = format!(
        "{base}function probe(words: List[Text], i: U64, n: U64) returns U64:\n  let copy = words\n  if U64.less_than(i, n):\n    match sico.list.get(copy, i):\n      case ok(w):\n        return i\n      case error(_):\n        return n\n    end match\n  end if\n  return n\nend function\n"
    );
    assert_eq!(
        run_guest(&list_cell),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    let wrong_member = format!(
        "{base}function probe(words: List[Text], i: U64, n: U64) returns U64:\n  if U64.less_than(i, n):\n    match sico.list.set(words, i):\n      case ok(w):\n        return i\n      case error(_):\n        return n\n    end match\n  end if\n  return n\nend function\n"
    );
    assert_eq!(
        run_guest(&wrong_member),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TARGET".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&good) else {
        panic!("valid list.get subject must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_host_call_lets_byte_exactly() {
    // STEP-0326: `sico.text.encode`/`sico.bytes.length` let/set RHS and
    // `sico.bytes.at` with a parameter cursor inside general-while
    // functions, byte-exact against the Rust oracle. The bytes.at packer's
    // return format gains an explicit count line (consumers updated).
    let cases = [
        "function probe(w: Text, i: U64) returns U64:\n  let b = sico.text.encode(w)\n  let n = U64.literal(0)\n  while U64.less_than(i, n):\n    set n = sico.bytes.length(b)\n  end while\n  return n\nend function\n",
        "function probe(b: Bytes, i: U64, m: U64) returns I64:\n  let x = I64.literal(0)\n  while U64.less_than(i, m):\n    set x = sico.bytes.at(b, i, I64.literal(0))\n  end while\n  return x\nend function\n",
    ];
    for source in cases {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("gw host-call let/set must compile: {source}")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    // Typed refusal: a non-u64 cursor parameter stays refused, then recovery.
    let bad_cursor = "function probe(b: Bytes, i: I64, m: U64) returns I64:\n  let x = I64.literal(0)\n  while U64.less_than(m, U64.literal(1)):\n    set x = sico.bytes.at(b, i, I64.literal(0))\n  end while\n  return x\nend function\n";
    assert_eq!(
        run_guest(bad_cursor),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    let expected = rust_ir(cases[0]);
    let RunOutcome::Output(recovered) = run_guest(cases[0]) else {
        panic!("valid host-call lets must compile after typed refusal")
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
        let expected = codegen_oracle::compile(&rust_module).unwrap();

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

#[test]
fn sico_compiler_encodes_declared_scalar_core_wasm_families_byte_exactly() {
    for (name, source) in [
        (
            "const-i64",
            "function probe() returns I64:\n  return I64.literal(624485)\nend function\n",
        ),
        (
            "const-u64",
            "function probe() returns U64:\n  return U64.literal(624485)\nend function\n",
        ),
        (
            "const-bool-true",
            "function probe() returns Bool:\n  return true\nend function\n",
        ),
        (
            "const-bool-false",
            "function probe() returns Bool:\n  return false\nend function\n",
        ),
        (
            "identity-i64",
            "function probe(value: I64) returns I64:\n  return value\nend function\n",
        ),
        (
            "identity-u64",
            "function probe(value: U64) returns U64:\n  return value\nend function\n",
        ),
        (
            "identity-bool",
            "function probe(value: Bool) returns Bool:\n  return value\nend function\n",
        ),
        (
            "identity-pair-second",
            "function probe(a: I64, b: I64) returns I64:\n  return b\nend function\n",
        ),
        (
            "identity-pair-first",
            "function probe(a: I64, b: I64) returns I64:\n  return a\nend function\n",
        ),
        (
            "identity-pair-bool",
            "function probe(a: Bool, b: Bool) returns Bool:\n  return b\nend function\n",
        ),
        (
            "less-than-i64",
            "function probe(a: I64, b: I64) returns Bool:\n  return I64.less_than(a, b)\nend function\n",
        ),
        (
            "less-than-u64",
            "function probe(a: U64, b: U64) returns Bool:\n  return U64.less_than(a, b)\nend function\n",
        ),
        (
            "checked-add-i64",
            "function probe(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, b)\nend function\n",
        ),
        (
            "checked-add-u64",
            "function probe(a: U64, b: U64) returns Result[U64, NumericError]:\n  return U64.checked_add(a, b)\nend function\n",
        ),
        (
            "checked-sub-i64",
            "function probe(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_sub(a, b)\nend function\n",
        ),
        (
            "checked-sub-u64",
            "function probe(a: U64, b: U64) returns Result[U64, NumericError]:\n  return U64.checked_sub(a, b)\nend function\n",
        ),
    ] {
        let source_file =
            SourceFile::from_text(SourceId::new(0), "identity.sico", source.to_string()).unwrap();
        let rust_module = lower_core(&source_file).unwrap();
        let expected = codegen_oracle::compile(&rust_module).unwrap();

        let RunOutcome::Output(output) =
            run_guest_with_args(source, vec!["--emit-core-hex".into()])
        else {
            panic!("{name} fixture must compile: {source}")
        };
        let actual = decode_hex(&output.stdout);
        assert_eq!(actual, expected, "{name}");
        wasmparser::Validator::new().validate_all(&actual).unwrap();
    }

    for source in [
        "function probe(x: I64) returns I64:\n  return x\nend function\n",
        "function probe(value: I64) returns U64:\n  return value\nend function\n",
        "function probe(a: I64, b: I64, c: I64) returns I64:\n  return c\nend function\n",
        "function probe(a: I64, b: I64) returns Bool:\n  return I64.greater_than(a, b)\nend function\n",
        "function probe(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_mul(a, b)\nend function\n",
        "function probe() returns U64:\n  return U64.literal(9223372036854775808)\nend function\n",
        "function probe(value: I64) returns I64:\n  return I64.add(value, value)\nend function\n",
    ] {
        assert_eq!(
            run_guest_with_args(source, vec!["--emit-core-hex".into()]),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "unsupported codegen source shape".into(),
            },
            "undeclared codegen shape must fail closed: {source}"
        );
    }
    let RunOutcome::Output(recovered) = run_guest_with_args(
        "function probe(value: I64) returns I64:\n  return value\nend function\n",
        vec!["--emit-core-hex".into()],
    ) else {
        panic!("valid identity fixture must compile after typed refusals")
    };
    let source_file = SourceFile::from_text(
        SourceId::new(0),
        "identity.sico",
        "function probe(value: I64) returns I64:\n  return value\nend function\n",
    )
    .unwrap();
    let expected = codegen_oracle::compile(&lower_core(&source_file).unwrap()).unwrap();
    assert_eq!(decode_hex(&recovered.stdout), expected);
}

#[test]
fn sico_codegen_guest_emits_component_bytes_byte_exactly() {
    // STEP-0487 (STEP-0486 S5 codegen wave): the guest emits the full script
    // Component bytes for the frozen minimal echo fixture through the new
    // `--emit-component-hex` driver mode, byte-equal to the Rust backend
    // (RFC-0011 deterministic emission, sico_codegen_wasm::compile_script_
    // program as the direct oracle). The skeleton covers the fixed component
    // scaffolding plus the frozen echo run body; every other source shape
    // must fail closed with the typed "unsupported codegen source shape"
    // refusal (ADR-0016 decision 6: no silent fallback).
    let expected = {
        let source_file = SourceFile::from_text(
            SourceId::new(0),
            "echo.sico",
            ECHO_COMPONENT_FIXTURE_SOURCE.to_owned(),
        )
        .unwrap();
        let module = lower_core(&source_file).unwrap();
        codegen_oracle::compile_script_program(&module).unwrap()
    };
    let RunOutcome::Output(output) = run_guest_with_args(
        ECHO_COMPONENT_FIXTURE_SOURCE,
        vec!["--emit-component-hex".into()],
    ) else {
        panic!("component codegen fixture must succeed through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "guest-emitted component bytes must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("guest-emitted component must pass the real Wasm validator");

    // Any source outside the frozen minimal shape must fail closed, and the
    // compiler must stay usable for ordinary lowering afterwards.
    assert_eq!(
        run_guest_with_args(IDENTITY_SOURCE, vec!["--emit-component-hex".into()],),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "unsupported codegen source shape".into(),
        },
        "unsupported component codegen shape must fail closed"
    );
    let RunOutcome::Output(recovered) = run_guest(IDENTITY_SOURCE) else {
        panic!("ordinary lowering must work after the typed refusal")
    };
    assert_eq!(recovered.exit_code, 0);
    assert_bytes_equal(&recovered.stdout, rust_ir(IDENTITY_SOURCE).as_bytes());
}

/// The Script-profile reference form with a different exit code: the same
/// shape, one different constant. The Rust backend emits a different
/// Component for it, so a guest that replayed a frozen run body would emit
/// the wrong bytes.
const ECHO_COMPONENT_FIXTURE_SOURCE_EXIT7: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

/// The Script-profile reference form whose `run` returns the `error` arm:
/// same entry signature as the reference form, but the payload is a
/// `ScriptError` instead of a `ScriptOutput`. The Rust backend really emits
/// a Component for it, so the `ScriptError` result-area offsets have a
/// measured truth instead of a frozen derivation.
const SCRIPT_ERROR_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"boom\"))\nend function\n";

#[test]
fn oracle_error_arm_result_area_truth_is_pinned() {
    // STEP-0505: the `error` arm's measured truth, read out of the Rust
    // backend's production output (the Component `compile_script_program`
    // really emits for a `run` that returns `error(...)`, opened with
    // wasmparser). Until this slice the `ScriptError` result-area offsets
    // were only a frozen derivation; here they are measured.
    //
    // The result-area return splits at the same unambiguous marker as the
    // `ok` arm: `i32.const 32` (the 32-byte result area) followed by
    // `call 1` (the allocator). The marker occurs exactly once.
    const ERR_LOCALS: &str = "017f017f017f017f017f017f017f017f017f017f017f017e017f017f017f017f027f";
    const ERR_FULL_BODY: &str = "4100211302400340201341004604404100210441002105410421062004210720052108200621094101210a4100210b4100210c4100210d4100210e4200210f2007211020082111200921124120100121142014200a3a0000201420103a0008201210012115201520112012fc0a00002014201536020c2014201236021020140f0b000b0b000b";
    const ERR_RESULT_SEGMENT: &str = "4120100121142014200a3a0000201420103a0008201210012115201520112012fc0a00002014201536020c2014201236021020140f0b000b0b000b";

    // The fixture really emits: the backend does not refuse the `error` arm,
    // so the offsets below are a measurement and not a derivation.
    let module = rust_module(SCRIPT_ERROR_FIXTURE_SOURCE);
    let component = sico_codegen_wasm::compile_script_program(&module)
        .expect("the backend emits the Script-profile Component for the error arm");
    wasmparser::Validator::new()
        .validate_all(&component)
        .expect("the error-arm Component passes the real Wasm validator");

    // The local declaration differs from the `ok` arm: the error payload
    // needs fewer slots (16 groups / 17 slots, one i64) than the output
    // payload (20 groups / 21 slots, three i64).
    assert_eq!(
        oracle_run_locals_hex(SCRIPT_ERROR_FIXTURE_SOURCE),
        ERR_LOCALS,
        "the error arm's local declaration section"
    );

    let hex = oracle_run_body_code_hex(SCRIPT_ERROR_FIXTURE_SOURCE);
    assert_eq!(hex, ERR_FULL_BODY, "the error arm's run body");
    assert_eq!(
        hex.matches("41201001").count(),
        1,
        "the result-area marker is unique in the error arm"
    );
    let cut = hex
        .find("41201001")
        .expect("the result-area marker is present");
    assert_eq!(
        &hex[cut..],
        ERR_RESULT_SEGMENT,
        "the ScriptError result-area return and the dispatcher epilogue"
    );

    // The measured `ScriptError` result-area offsets, in the order the
    // backend emits them: the discriminant byte at 0, `code` at 8 as a
    // single byte, `message` as a list at 12 (pointer) and 16 (length).
    // The `i32.store8` at offset 8 and the two `i32.store` at 12 and 16 are
    // the bytes the guest has to reproduce.
    for (offset, store) in [
        ("3a0008", "i32.store8 at offset 8 (ScriptError.code)"),
        (
            "36020c",
            "i32.store at offset 12 (ScriptError.message pointer)",
        ),
        (
            "360210",
            "i32.store at offset 16 (ScriptError.message length)",
        ),
    ] {
        assert!(
            ERR_RESULT_SEGMENT.contains(offset),
            "the measured error arm must contain {store}"
        );
    }
    // The error arm stores no i64 field, so it carries no `i64.store`; the
    // `ok` arm's `exit_code` is what produced the `37 03 18` there.
    assert!(
        !ERR_RESULT_SEGMENT.contains("3703"),
        "the error arm stores no 64-bit result field"
    );
    // The tail is the same six-byte dispatcher epilogue as the `ok` arm.
    assert!(
        ERR_RESULT_SEGMENT.ends_with("0b000b0b000b"),
        "the six-byte dispatcher epilogue"
    );
    assert_eq!(
        ERR_RESULT_SEGMENT.len() / 2,
        59,
        "the measured error-arm result segment is 59 bytes"
    );
}

#[test]
fn sico_compiler_lowers_the_enum_case_operand_as_a_variant_byte_exactly() {
    // STEP-0506, frontend knife: an enum case is an operand in its own
    // right. The Rust frontend lowers `Enum.Case` into an empty-payload
    // `variant` op whose type is the enum -- it looks at the variant before
    // it looks at field projections -- and the guest frontend must agree byte
    // for byte: the operation, the type, the result id and the source range.
    //
    // The `error`-arm fixture is the shape that needs it: `ScriptError.code`
    // is an enum case, so without this lowering the whole construct was
    // refused with `ERR:E-SH-IR-CONSTRUCT-VALUE` and the guest could not
    // reach the codegen at all.
    let expected = rust_ir(SCRIPT_ERROR_FIXTURE_SOURCE);
    let outcome = run_guest(SCRIPT_ERROR_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = outcome else {
        panic!("the error-arm fixture must lower through the guest frontend: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_codegen_guest_emits_the_error_arm_component_byte_exactly() {
    // STEP-0506, codegen knife: the `error` arm is a second Script-profile
    // shape, and the guest's component emission has to follow the IR into
    // the second arm instead of stopping at the `ok` one.
    //
    // Three rules are new here, and all three are measured from the Rust
    // backend's production output (STEP-0505):
    //   * the one-of member is an empty-payload `variant` op whose
    //     discriminant is the frozen `script-error-code` tag;
    //   * the `error` payload is right-aligned: the inactive `ok` region is
    //     zero-filled and the error payload sits after it;
    //   * the `ScriptError.code` field is a one-byte discriminant in the
    //     result area (`i32.store8` at 8), while `message` is a list at 12
    //     and 16.
    //
    // The exit criterion of this slice is the whole component, byte for
    // byte against the backend -- the local declaration section (17 groups
    // / 18 slots), the run body and the result-area return.
    let module = rust_module(SCRIPT_ERROR_FIXTURE_SOURCE);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the error arm");
    let RunOutcome::Output(output) = run_guest_with_args(
        SCRIPT_ERROR_FIXTURE_SOURCE,
        vec!["--emit-component-hex".into()],
    ) else {
        panic!("the error-arm component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted error-arm component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted error-arm component must pass the real Wasm validator");
}

const MULTI_FUNCTION_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction helper(value: I64) returns I64:\n  return value\nend function\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

/// The same shape with a **multi-slot** helper parameter: a `Text` flattens
/// to two Wasm slots, so the derived core type carries two valtypes and the
/// flat group hex carries two groups.
const MULTI_SLOT_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction helper(text: Text) returns Text:\n  return text\nend function\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

#[test]
fn sico_codegen_guest_emits_the_multi_function_component_byte_exactly() {
    // STEP-0507, codegen knife: a module with more than one function. The
    // backend emits every module function into the core module, in the
    // module's function order, and then the three frozen arena helpers, so the
    // type section, the function table, the export indices and the code bodies
    // all follow the module's function count.
    //
    // The rules are measured from the Rust backend's production output
    // (STEP-0507 §1):
    //   * the type section lists the module functions first and the three
    //     frozen helper types last; `run` keeps the frozen Script ABI type
    //     while every other function's type is derived from the flat parameter
    //     and result types;
    //   * the function table carries one entry per module function plus the
    //     three helpers, and a core function's type index is its own index;
    //   * the four exports keep their names while their function indices
    //     follow the module function count;
    //   * a module function other than `run` returns its flat value directly
    //     (one `local.get` per slot, then `return`) instead of through the
    //     Script result area;
    //   * the local declaration rule is the backend's own: the parameters are
    //     not declared, every instruction result slot is its own group, and
    //     the dispatcher and the Script scratch pair follow.
    //
    // The exit criterion of this slice is the whole component, byte for byte
    // against the backend.
    //
    // Two fixtures: one helper with a single-slot parameter and one with a
    // multi-slot parameter, so the derived core type covers one and two flat
    // groups.
    for (label, source) in [
        ("single-slot helper", MULTI_FUNCTION_FIXTURE_SOURCE),
        ("multi-slot helper", MULTI_SLOT_FIXTURE_SOURCE),
    ] {
        let module = rust_module(source);
        let expected = codegen_oracle::compile_script_program(&module).expect(
            "the Rust backend emits the Script-profile Component for the multi-function module",
        );
        let RunOutcome::Output(output) =
            run_guest_with_args(source, vec!["--emit-component-hex".into()])
        else {
            panic!("the multi-function component must be emitted through the guest")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        let actual = decode_hex(&output.stdout);
        assert_eq!(
            actual, expected,
            "the guest-emitted multi-function component ({label}) must equal the Rust backend exactly"
        );
        wasmparser::Validator::new()
            .validate_all(&actual)
            .expect("the guest-emitted multi-function component must pass the real Wasm validator");
    }
}

const CALL_IN_FIELD_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction helper(value: I64) returns I64:\n  return value\nend function\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: helper(I64.literal(7))))\nend function\n";

#[test]
fn sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly() {
    // STEP-0508: a call in a construct field value position. The call's
    // arguments are lowered into their own instructions first -- each one
    // takes a result id -- and the call itself takes the next id; the field
    // references the call's id.
    //
    // The rules are measured from the Rust backend's production output
    // (STEP-0508 §1):
    //   * the call's `function` is the callee's declaration id (1-based);
    //   * the call's `arguments` are the argument value ids;
    //   * the call's `ty` is the callee's declared return type;
    //   * the call's `range` runs from the callee's name to the closing
    //     parenthesis.
    let expected = rust_ir(CALL_IN_FIELD_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest(CALL_IN_FIELD_FIXTURE_SOURCE) else {
        panic!("the call-in-a-field fixture must lower through the guest frontend")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

fn codegen_function_text(name: &str) -> String {
    let start = CODEGEN_SOURCE
        .find(&format!("function {name}("))
        .expect("codegen keeps the function");
    let end = CODEGEN_SOURCE[start..]
        .find("\nend function\n")
        .expect("function ends")
        + start
        + "\nend function\n".len();
    CODEGEN_SOURCE[start..end].to_owned()
}

/// Every function name `codegen.sico` declares.
fn codegen_function_names() -> Vec<&'static str> {
    CODEGEN_SOURCE
        .lines()
        .filter_map(|line| line.strip_prefix("function "))
        .filter_map(|rest| rest.split('(').next())
        .map(|name| name.trim())
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .collect()
}

/// The parameter list and the return type of a `codegen.sico` function.
fn codegen_signature(name: &str) -> (String, String) {
    let head = codegen_function_text(name)
        .lines()
        .next()
        .expect("the function has a signature line")
        .to_owned();
    let open = head.find('(').expect("the signature has parameters");
    let close = head
        .rfind(')')
        .expect("the signature closes its parameters");
    let params = head[open + 1..close].trim().to_owned();
    let returns = head[close + 1..]
        .trim()
        .trim_start_matches("returns ")
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_owned();
    (params, returns)
}

/// A stub for one `codegen.sico` function: the real signature, a trivial body.
/// The signature is what the call lowering reads, so a call to a stub lowers
/// exactly like a call to the real function.
fn codegen_stub(name: &str) -> String {
    let (params, returns) = codegen_signature(name);
    let body = match returns.as_str() {
        "U64" => "U64.literal(0)",
        "I64" => "I64.literal(0)",
        "Bool" => "false",
        "Text" => "\"\"",
        "Bytes" => "sico.text.encode(\"\")",
        other => panic!("codegen function {name} has an unexpected return type {other}"),
    };
    format!("function {name}({params}) returns {returns}:\n  return {body}\nend function\n")
}

/// The `codegen.sico` functions the text calls, in first-call order.
fn codegen_callees(text: &str, own: &str) -> Vec<String> {
    let names = codegen_function_names();
    let chars: Vec<char> = text.chars().collect();
    let mut callees: Vec<String> = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].is_ascii_alphabetic() && chars[index] != '_' {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && (chars[index].is_ascii_alphanumeric() || chars[index] == '_') {
            index += 1;
        }
        let word: String = chars[start..index].iter().collect();
        let member = start > 0 && chars[start - 1] == '.';
        let mut after = index;
        while after < chars.len() && chars[after] == ' ' {
            after += 1;
        }
        let called = after < chars.len() && chars[after] == '(';
        if called
            && !member
            && word != own
            && names.contains(&word.as_str())
            && !callees.contains(&word)
        {
            callees.push(word);
        }
    }
    callees
}

/// The source that lowers one `codegen.sico` function in isolation: a stub for
/// every callee it declares, then the function itself.
fn codegen_function_source(name: &str) -> String {
    let text = codegen_function_text(name);
    let mut source = String::new();
    for callee in codegen_callees(&text, name) {
        source.push_str(&codegen_stub(&callee));
    }
    source.push_str(&text);
    source
}

#[test]
fn sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly() {
    // STEP-0507: the guest's own new codegen code has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the guest's
    // own emitter, otherwise the bootstrap could never consume its own code.
    // Every function this slice added or reshaped is lowered through the guest
    // and compared against the Rust frontend's canonical IR, byte for byte.
    //
    // The shape rules this slice had to respect (measured, not guessed): a
    // user call may not sit directly in a return expression, a call argument
    // may not be another call, and a host call may not take a host call as its
    // argument -- every one of them goes through a cell first.
    for name in [
        "cgs_alloc_index",
        "cgs_alloc_call_hex",
        "cgs_function_at_index",
        "cgs_function_count",
        "cgs_run_index",
        "cgs_flat_valtypes",
        "cgs_core_type_hex",
        "cgs_core_types_hex",
        "cgs_core_funcs_hex",
        "cgs_core_exports_hex",
        "cgs_function_return_hex",
        "cgs_function_body_hex",
        "cgs_core_code_hex",
        "cgs_result_list_hex",
        "cgs_result_field_hex",
        "cgs_result_error_field_hex",
        "cgs_result_return_hex",
        "cg_core_globals_hex",
        "cg_run_body_hex_from_ir",
        "cgs_local_hex",
        "cgs_run_instr_hex",
    ] {
        let source = codegen_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("codegen function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

#[test]
fn sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly() {
    // STEP-0509, codegen knife: the guest's own new emission code has to stay
    // inside the self-hosting shape frontier, exactly like the STEP-0507
    // slice established for its own functions: the guest frontend must lower
    // the guest's own emitter, otherwise the bootstrap could never consume its
    // own code.
    //
    // The shape rules are the measured ones from STEP-0507: a user call may not
    // sit directly in a return expression, a call argument may not be another
    // call, and a host call may not take a host call as its argument -- every
    // one of them goes through a cell first.
    for name in ["cgs_function_position", "cgs_call_hex", "cgs_op_hex"] {
        let source = codegen_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("codegen function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

#[test]
fn sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly() {
    // STEP-0509, codegen knife: a call in a construct field value position
    // reaches the component emitter. The whole component must equal the Rust
    // backend's own production output byte for byte.
    //
    // The rules are measured from the backend's production output
    // (STEP-0509 §1):
    //   * the call's target is the Wasm function index the module's own
    //     function list gives for the call's declaration id (the callee is
    //     the first declared function, so the index is 0);
    //   * the arguments' flat slots are pushed first, the call follows, and
    //     the result's slots are stored afterwards;
    //   * the local declaration section follows the backend's own rule, so
    //     the two new instruction results add one i64 group each.
    let module = rust_module(CALL_IN_FIELD_FIXTURE_SOURCE);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the call fixture");
    let RunOutcome::Output(output) = run_guest_with_args(
        CALL_IN_FIELD_FIXTURE_SOURCE,
        vec!["--emit-component-hex".into()],
    ) else {
        panic!("the call-in-a-field component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted call-in-a-field component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted call-in-a-field component must pass the real Wasm validator");
}

/// STEP-0510: the same call shape, but in a **return expression** position --
/// `helper2` returns the result of `helper(...)`, and `run` calls `helper2` in
/// a construct field. The guest lowers this shape byte-exactly; it was never
/// covered by a pin, and the STEP-0508 / STEP-0509 records described it as
/// refused (they were wrong -- see the STEP-0510 record).
const CALL_IN_RETURN_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction helper(value: I64) returns I64:\n  return value\nend function\n\nfunction helper2(value: I64) returns I64:\n  return helper(I64.literal(3))\nend function\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: helper2(I64.literal(7))))\nend function\n";

#[test]
fn sico_compiler_lowers_the_call_in_a_return_position_byte_exactly() {
    // STEP-0510: a user call in a return expression position. The measured
    // rules (STEP-0510 §1) are the ones the STEP-0508 record established for
    // the construct field position, and they hold here unchanged:
    //   * the call's `function` is the callee's declaration id (1-based);
    //   * the call's `arguments` are the argument value ids;
    //   * the call's `ty` is the callee's declared return type;
    //   * the call's `range` runs from the callee's name to the closing
    //     parenthesis;
    //   * the enclosing function's terminator returns the **call's** value id.
    let expected = rust_ir(CALL_IN_RETURN_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest(CALL_IN_RETURN_FIXTURE_SOURCE) else {
        panic!("the call-in-a-return fixture must lower through the guest frontend")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_codegen_guest_emits_the_call_in_a_return_component_byte_exactly() {
    // STEP-0510: the same shape reaches the component emitter. The call in
    // the return position lands in the **callee's** own core function body
    // (`helper2`), while the `run` body only carries the call to `helper2`.
    // The whole component must equal the Rust backend's production output
    // byte for byte.
    let module = rust_module(CALL_IN_RETURN_FIXTURE_SOURCE);
    let expected = codegen_oracle::compile_script_program(&module).expect(
        "the Rust backend emits the Script-profile Component for the call-in-a-return fixture",
    );
    let RunOutcome::Output(output) = run_guest_with_args(
        CALL_IN_RETURN_FIXTURE_SOURCE,
        vec!["--emit-component-hex".into()],
    ) else {
        panic!("the call-in-a-return component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted call-in-a-return component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted call-in-a-return component must pass the real Wasm validator");
}

/// STEP-0511: the measured truth for a **host call (`sico.*`) in a straight-line
/// function body** -- the shape STEP-0510 §4.6 located as the open gap after
/// the frontend frontier was mapped shape by shape.
///
/// The `size` helper is a straight-line body (no `let`, no `set`, no `if`, no
/// `while`) whose return expression *is* a host call; `run` is the reference
/// Script-profile form.
const HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction size(data: Bytes) returns U64:\n  return sico.bytes.length(data)\nend function\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

#[test]
fn oracle_host_call_in_a_straight_line_body_truth_is_pinned() {
    // STEP-0511, truth slice. Everything here is read from the **production
    // output**, not from a design document:
    //
    //   * the Rust backend really emits a Component for this fixture, and the
    //     component is 1025 B;
    //   * the canonical IR shape of the host call is one `intrinsic`
    //     operation whose `arguments` are the argument **value ids** and
    //     whose `ty` is the intrinsic's result kind;
    //   * the guest frontend refused the shape at the STEP-0511 measurement
    //     time, with a **typed** identity (fail-closed, no silent fallback);
    //     STEP-0512 landed the frontend knife, so the guest half below is now
    //     the byte-exact comparison this pin promised -- the original refusal
    //     identity stays in the STEP-0511 record for traceability.
    //
    // This pin is the oracle for the implementation slice: when the guest
    // learns the shape, this pin's guest half flips to a byte-exact
    // comparison and the component size stays 1025 B.
    let module = rust_module(HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE);
    let component = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the host-call fixture");
    assert_eq!(
        component.len(),
        1025,
        "the measured host-call component size is pinned"
    );
    wasmparser::Validator::new()
        .validate_all(&component)
        .expect("the measured host-call component must pass the real Wasm validator");

    let ir = canonical_json(&module).expect("canonical IR");
    assert!(
        ir.contains("\"op\":\"intrinsic\""),
        "a host call lowers to one intrinsic operation: {ir}"
    );
    assert!(
        ir.contains("\"name\":\"sico.bytes.length\""),
        "the intrinsic keeps its dotted name: {ir}"
    );
    assert!(
        ir.contains("\"arguments\":[0]"),
        "the intrinsic arguments are the argument value ids: {ir}"
    );

    // STEP-0512: the promised flip -- the guest half is now the byte-exact
    // comparison against the Rust frontend's canonical IR.
    let expected = rust_ir(HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest(HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE) else {
        panic!("the guest must lower a host call in a straight-line body after STEP-0512")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

/// STEP-0512: the two shapes next to the supported one, kept as **typed
/// refusals**. The knife claims exactly one shape -- a one-argument
/// `sico.bytes.length` over a `Bytes` parameter, in a body whose only
/// statement is the return -- so neither of these may be silently widened
/// into something the guest lowers.
const HOST_CALL_SHAPE_PRELUDE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";
const HOST_CALL_SHAPE_RUN: &str = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

#[test]
fn sico_compiler_lowers_the_host_call_in_a_straight_line_body_byte_exactly() {
    // STEP-0512, frontend knife: a host call (`sico.bytes.length`) as the whole
    // return expression of a straight-line function body now lowers to the
    // canonical intrinsic operation -- **byte for byte equal** to the Rust
    // frontend's own canonical IR, not merely shape-equal.
    //
    // The landing point is the dedicated `direct_host_call_return_ir` route
    // in the guest frontend, wired into the route ladder ahead of the
    // general return analysis. The route claims one shape and returns SKIP
    // for everything else, so no existing refusal identity changes.
    let expected = rust_ir(HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE);
    assert_eq!(
        expected.len(),
        2215,
        "the measured canonical IR size of the host-call fixture is pinned"
    );
    let RunOutcome::Output(output) = run_guest(HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE) else {
        panic!("the guest must lower a host call in a straight-line body after STEP-0512")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let ir = String::from_utf8(output.stdout).expect("the guest IR is utf-8");
    assert!(
        ir.contains("\"op\":\"intrinsic\""),
        "the host call lowers to one intrinsic operation: {ir}"
    );
    assert!(
        ir.contains("\"name\":\"sico.bytes.length\""),
        "the intrinsic keeps its dotted name: {ir}"
    );
    assert!(
        ir.contains("\"arguments\":[0]"),
        "the intrinsic arguments are the argument value ids: {ir}"
    );
}

#[test]
fn sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused() {
    // STEP-0512 / STEP-0515: the knife is one shape, not a widening of the
    // host-call surface. The shapes the STEP-0511 truth slice measured next to
    // the supported one stay typed refusals:
    //
    //   * the three-argument `sico.bytes.at` -- a different intrinsic, a
    //     different arity, a literal argument;
    //   * a `let` binding whose `return` names something else -- the binding
    //     the Rust frontend propagates is the one the `return` names;
    //   * a `let` binding followed by a second statement -- the propagated
    //     straight-line binding is one statement, not two;
    //   * a `let` binding of a different intrinsic (`sico.text.length`);
    //   * a `let` binding of the three-argument `sico.bytes.at`.
    //
    // STEP-0515 claimed the one-argument `sico.bytes.length` in a `let`
    // binding -- the shape this pin refused at STEP-0512 -- so that case
    // moved to the STEP-0515 pins below. Every shape the knife does *not*
    // claim keeps the identity the STEP-0511 measurement recorded
    // (`ERR:E-SH-IR-EXPRESSION`): fail-closed, no silent reinterpretation.
    let three_argument = format!(
        "{HOST_CALL_SHAPE_PRELUDE}function byte_at(data: Bytes, index: U64) returns I64:\n  return sico.bytes.at(data, index, I64.literal(0))\nend function\n\n{HOST_CALL_SHAPE_RUN}"
    );
    let let_other_name = format!(
        "{HOST_CALL_SHAPE_PRELUDE}function size(data: Bytes) returns U64:\n  let n = sico.bytes.length(data)\n  return other\nend function\n\n{HOST_CALL_SHAPE_RUN}"
    );
    let let_two_statements = format!(
        "{HOST_CALL_SHAPE_PRELUDE}function size(data: Bytes) returns U64:\n  let n = sico.bytes.length(data)\n  let m = n\n  return m\nend function\n\n{HOST_CALL_SHAPE_RUN}"
    );
    let let_other_intrinsic = format!(
        "{HOST_CALL_SHAPE_PRELUDE}function size(data: Bytes) returns U64:\n  let n = sico.text.length(data)\n  return n\nend function\n\n{HOST_CALL_SHAPE_RUN}"
    );
    let let_three_argument = format!(
        "{HOST_CALL_SHAPE_PRELUDE}function byte_at(data: Bytes, index: U64) returns I64:\n  let n = sico.bytes.at(data, index, I64.literal(0))\n  return n\nend function\n\n{HOST_CALL_SHAPE_RUN}"
    );
    for (label, source) in [
        ("three-argument", three_argument),
        ("let-other-name", let_other_name),
        ("let-two-statements", let_two_statements),
        ("let-other-intrinsic", let_other_intrinsic),
        ("let-three-argument", let_three_argument),
    ] {
        let RunOutcome::Domain { code, message } = run_guest(&source) else {
            panic!("the {label} host-call shape must stay a typed refusal")
        };
        assert_eq!(code, "invalid-input", "{label}");
        assert_eq!(message, "ERR:E-SH-IR-EXPRESSION", "{label}");
    }
}

#[test]
fn sico_compiler_lowers_the_step0512_parser_function_byte_exactly() {
    // STEP-0512: the guest's own new frontend route has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the
    // guest's own new code, otherwise the bootstrap could never consume its
    // own source. Exactly like the STEP-0507 / STEP-0509 slices established
    // for the functions they added.
    //
    // The closure is the transitive callee set of `direct_host_call_return_ir`
    // in parser.sico, emitted in file order (the guest frontend refuses
    // forward-referenced user-function calls).
    let base = [
        "next_index",
        "word_at",
        "is_word",
        "is_ident_byte",
        "record_named_type",
        "type_json",
        "line_end",
        "intrinsic_instruction_json",
        "find_text_from",
        "binding_parameter_index",
        "matching_close",
        "is_intrinsic_path",
        "scalar_type_kind",
        "binding_parameter_kind",
        "gw_find_code_text",
        "return_expression_end",
        "parameter_name_start",
        "parameters_ir",
        "has_return_at",
        "return_expression_start",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("direct_host_call_return_ir")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("direct_host_call_return_ir closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_codegen_guest_emits_the_host_call_component_byte_exactly() {
    // STEP-0513, codegen knife: the `intrinsic` operand reaches the component
    // emitter. The **whole** component must equal the Rust backend's own
    // production output byte for byte.
    //
    // The rules are measured from the backend's production output (the oracle
    // component's code section, parsed):
    //   * `sico.bytes.length` over a `Bytes` argument is
    //     `local.get <length>` / `i64.extend_i32_u` / `local.set <result>`;
    //   * the length slot is the **second** slot of the argument's
    //     (pointer, length) pair -- the first flat slot of the `Bytes`
    //     parameter is the pointer;
    //   * the result slot is the instruction's own flat slot, which the
    //     guest's existing slot derivation already places (the parameter
    //     pair first, then the instruction results in IR order).
    let module = rust_module(HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the host-call fixture");
    assert_eq!(
        expected.len(),
        1025,
        "the measured host-call component size is pinned"
    );
    let RunOutcome::Output(output) = run_guest_with_args(
        HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE,
        vec!["--emit-component-hex".into()],
    ) else {
        panic!("the host-call component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted host-call component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted host-call component must pass the real Wasm validator");
}

#[test]
fn sico_compiler_lowers_the_step0513_codegen_function_byte_exactly() {
    // STEP-0513: the guest's own new emission code has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the
    // guest's own emitter, otherwise the bootstrap could never consume its
    // own code. Exactly like the STEP-0507 / STEP-0509 slices established
    // for the functions they added.
    let source = codegen_function_source("cgs_intrinsic_hex");
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("codegen function cgs_intrinsic_hex must compile through the guest frontend")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly() {
    // STEP-0504: the guest's component emission is no longer a replay of a
    // frozen run body. The run body is derived from the ingested IR -- the
    // local declaration section (STEP-0499) followed by the code bytes
    // (STEP-0502/STEP-0503) -- and the frozen constant is gone from the guest
    // source.
    //
    // The proof that the emission follows the IR instead of replaying a
    // constant: a second reference form that differs only in one constant
    // must produce a *different* component, and both must equal the Rust
    // backend's own production output byte for byte.
    let mut guest_components = Vec::new();
    for source in [
        ECHO_COMPONENT_FIXTURE_SOURCE,
        ECHO_COMPONENT_FIXTURE_SOURCE_EXIT7,
    ] {
        let source_file =
            SourceFile::from_text(SourceId::new(0), "echo.sico", source.to_owned()).unwrap();
        let module = lower_core(&source_file).unwrap();
        let expected = codegen_oracle::compile_script_program(&module).unwrap();
        let RunOutcome::Output(output) =
            run_guest_with_args(source, vec!["--emit-component-hex".into()])
        else {
            panic!("component codegen must succeed through the guest")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        let actual = decode_hex(&output.stdout);
        assert_eq!(
            actual, expected,
            "guest-emitted component bytes must equal the Rust backend exactly"
        );
        wasmparser::Validator::new()
            .validate_all(&actual)
            .expect("guest-emitted component must pass the real Wasm validator");
        guest_components.push(actual);
    }
    // The two reference forms differ in one constant, so their components
    // must differ: a replayed frozen body would have produced identical bytes.
    assert_ne!(
        guest_components[0], guest_components[1],
        "the component emission must follow the IR, not a frozen constant"
    );
}

#[test]
fn sico_codegen_guest_refuses_the_component_emission_outside_the_subset() {
    // STEP-0504: the component emission is fail-closed (ADR-0016 decision 6).
    // STEP-0507 widened the supported subset from "exactly one function" to
    // "every module function the derivation covers", so the remaining refusals
    // are the shapes the derivation does not cover: a source outside the
    // Script profile, and a module function that returns a checked result
    // (the packed checked-result return is not implemented yet).
    let checked_result_helper = format!(
        "{ECHO_COMPONENT_FIXTURE_SOURCE}\nfunction helper(value: I64) returns Result[I64, ScriptError]:\n  return ok(value)\nend function\n"
    );
    for source in [IDENTITY_SOURCE, checked_result_helper.as_str()] {
        assert_eq!(
            run_guest_with_args(source, vec!["--emit-component-hex".into()]),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "unsupported codegen source shape".into(),
            },
            "a shape outside the component subset must stay a typed refusal: {source}"
        );
    }
}

const ECHO_COMPONENT_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";

#[test]
fn sico_compiler_lowers_the_script_profile_echo_fixture_byte_exactly() {
    // STEP-0488 → STEP-0495: the guest frontend could not lower the
    // Script-profile shape yet (record projection, `ok(...)` variant, record
    // construct); the pin was "typed refusal, stable". STEP-0491 (projection),
    // STEP-0493 (named return type), STEP-0494 (record construct) and STEP-0495
    // (`ok(...)` / `error(...)` variant) closed all three gaps, so the frontier
    // pin is re-pinned from "typed refusal" to "byte-exact lowering against the
    // Rust oracle" — the IR-driven component emitter consumes exactly this IR.
    // The reproducible frontier probe is tools/probe-m22-frontend-frontier.ps1.
    let expected = rust_ir(ECHO_COMPONENT_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest(ECHO_COMPONENT_FIXTURE_SOURCE) else {
        panic!("the script-profile shape must lower through the guest frontend")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_script_input_field_projection_byte_exactly() {
    // STEP-0491: the guest frontend lowers a declared-record field
    // projection of a named parameter (`input.stdin` where `input` is typed
    // `ScriptInput`) into the canonical `project` operation. The Rust
    // lowerer emits the field projection as `Operation::Project` with the
    // range of the "." + field-name token pair; the guest must agree
    // byte-for-byte, and the record declaration must still reach the
    // `records` table.
    let source = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nfunction probe(input: ScriptInput) returns Bytes:\n  return input.stdin\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = outcome else {
        panic!(
            "the ScriptInput field projection must lower through the guest frontend: {outcome:?}"
        )
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_any_declared_record_field_projection_byte_exactly() {
    // STEP-0492: STEP-0491 landed the projection with the base parameter
    // typed `ScriptInput` only. The Script profile needs one instance of
    // that, but the lowering must not carry a hardcoded record name: any
    // declared record parameter projects its own declared field, with the
    // parameter type reported as the declared record name.
    let source = "record ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\nend record\n\nfunction probe(out: ScriptOutput) returns Bytes:\n  return out.stdout\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = outcome else {
        panic!(
            "a declared-record field projection must lower through the guest frontend: {outcome:?}"
        )
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_refuses_unresolved_record_field_projections_with_a_typed_identity() {
    // STEP-0491: the projection lowering is fail-closed. A projection whose
    // record is not declared, or whose field does not exist in the declared
    // record, must stay a typed refusal (no silent partial lowering and no
    // invented field), because the Rust oracle refuses both as well.
    let record =
        "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\n";
    for (source, message) in [
        (
            "function probe(input: ScriptInput) returns Bytes:\n  return input.stdin\nend function\n",
            "ERR:E-SH-IR-EXPRESSION",
        ),
        (
            "function probe(input: ScriptInput) returns Bytes:\n  return input.missing\nend function\n",
            "ERR:E-SH-IR-EXPRESSION",
        ),
    ] {
        assert_eq!(
            run_guest(source),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: message.into(),
            },
            "an unresolved record field projection must stay a typed refusal"
        );
    }
    // The same unknown field stays refused when the record IS declared.
    assert_eq!(
        run_guest(&format!(
            "{record}function probe(input: ScriptInput) returns Bytes:\n  return input.missing\nend function\n"
        )),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-EXPRESSION".into(),
        },
        "an unknown field of a declared record must stay a typed refusal"
    );
}

#[test]
fn sico_compiler_lowers_named_record_return_type_pass_through_byte_exactly() {
    // STEP-0493: a declared record may be the declared return type. The Rust
    // oracle reports it as a named type and lowers a pass-through of the named
    // parameter into an empty instruction list whose terminator returns the
    // parameter id. The guest frontend must agree byte-for-byte. The named
    // return type travels through the type-JSON channel (`{"kind":"named",...}`)
    // and never through the scalar kind channel, which stays reserved for
    // scalar kinds — the scalar channel has 19 call sites that treat a kind as
    // a bare scalar and would otherwise splice a JSON blob into an IR field.
    let source = "record ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\nend record\n\nfunction probe(out: ScriptOutput) returns ScriptOutput:\n  return out\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = outcome else {
        panic!("a named record return type must lower through the guest frontend: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_refuses_named_record_return_types_with_a_typed_identity() {
    // STEP-0493: the named return type stays fail-closed. A record that is
    // never declared must not be treated as a named type, a named return type
    // whose body carries statements the guest cannot account for must stay a
    // typed refusal (before STEP-0493 that shape emitted a malformed IR with
    // an empty return kind), and a named return type must not weaken the
    // return/parameter type checks.
    let declared =
        "record ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\nend record\n\n";
    for (source, message) in [
        (
            "function probe(out: ScriptOutput) returns ScriptOutput:\n  return out\nend function\n"
                .to_string(),
            "ERR:E-SH-IR-PARAMETER-TYPE",
        ),
        (
            "function probe(out: ScriptOutput) returns ScriptOutput:\n  return I64.literal(1)\nend function\n"
                .to_string(),
            "ERR:E-SH-IR-RETURN-TYPE",
        ),
        (
            format!(
                "{declared}function probe(out: ScriptOutput) returns ScriptOutput:\n  let x = U64.literal(0)\n  return out\nend function\n"
            ),
            "ERR:E-SH-IR-RETURN-TYPE",
        ),
        (
            format!(
                "{declared}function probe(out: ScriptOutput) returns ScriptOutput:\n  let x = U64.literal(0)\n  set x = x\n  return out\nend function\n"
            ),
            "ERR:E-SH-IR-RETURN-TYPE",
        ),
        (
            format!(
                "{declared}function probe(out: ScriptOutput) returns ScriptOutput:\n  let x = U64.literal(0)\n  while U64.less_than(x, U64.literal(1)):\n    set x = x\n  end while\n  return out\nend function\n"
            ),
            "ERR:E-SH-IR-RETURN-TYPE",
        ),
    ] {
        assert_eq!(
            run_guest(&source),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: message.into(),
            },
            "an unsupported named record return type must stay a typed refusal: {}",
            source
        );
    }
    // The declared record keeps the pass-through lowerable, and the return
    // type check still applies to a named return type.
    assert_eq!(
        run_guest(&format!(
            "{declared}function probe(out: ScriptOutput) returns ScriptOutput:\n  return I64.literal(1)\nend function\n"
        )),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-TYPE-MISMATCH".into(),
        },
        "a named return type must not weaken the return type check"
    );
}

#[test]
fn sico_compiler_lowers_record_construct_byte_exactly() {
    // STEP-0494: a declared record may be constructed. The Rust oracle lowers
    // `ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code:
    // I64.literal(0))` by lowering every field value into its own
    // instruction first (results 1..3) and then emitting the `construct`
    // operation that references those ids (result 4). The guest frontend must
    // agree byte-for-byte, including the result-id ledger and the construct
    // range (the constructor name through the closing parenthesis).
    let source = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nfunction probe(input: ScriptInput) returns ScriptOutput:\n  return ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0))\nend function\n";
    let expected = rust_ir(source);
    let outcome = run_guest(source);
    let RunOutcome::Output(output) = outcome else {
        panic!("a record construct must lower through the guest frontend: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_refuses_unresolved_record_constructs_with_a_typed_identity() {
    // STEP-0494: the construct lowering is fail-closed. A field the declared
    // record does not have must not be invented, a field value the guest
    // cannot lower must stay a typed refusal (the Rust oracle lowers a wider
    // value grammar, so this is a narrowing), and a construct whose record
    // does not match the declared return type must stay refused.
    let records = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\n";
    for (source, message) in [
        (
            format!(
                "{records}function probe(input: ScriptInput) returns ScriptOutput:\n  return ScriptOutput(missing: input.stdin)\nend function\n"
            ),
            "ERR:E-SH-IR-CONSTRUCT-FIELD",
        ),
        (
            format!(
                "{records}function probe(input: ScriptInput) returns ScriptOutput:\n  return ScriptOutput(stdout: sico.text.encode(\"x\"))\nend function\n"
            ),
            "ERR:E-SH-IR-CONSTRUCT-VALUE",
        ),
        (
            format!(
                "{records}function probe(input: ScriptInput) returns ScriptOutput:\n  return ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0), extra: input.stdin)\nend function\n"
            ),
            "ERR:E-SH-IR-CONSTRUCT-FIELD",
        ),
    ] {
        assert_eq!(
            run_guest(&source),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: message.into(),
            },
            "an unresolved record construct must stay a typed refusal: {}",
            source
        );
    }
}

#[test]
fn sico_compiler_lowers_result_variant_byte_exactly() {
    // STEP-0495: `ok(...)` and `error(...)` are the built-in Result variants.
    // The Rust oracle lowers the payload into its own instruction(s) first and
    // then emits the `variant` operation that references those ids; the guest
    // frontend must agree byte-for-byte, including the result-id ledger, the
    // variant range (the variant name through the closing parenthesis) and the
    // declared `Result[ok, error]` return type.
    let cases = [
        // A scalar payload: one `const_i64` then the `ok` variant.
        "function probe() returns Result[I64, NumericError]:\n  return ok(I64.literal(1))\nend function\n",
        // The same shape behind an enum declaration: the error arm is a named
        // type, so the enum cases must not leak into the arm type JSON.
        "enum NumericError:\n  case Invalid\nend enum\n\nfunction probe() returns Result[I64, NumericError]:\n  return ok(I64.literal(1))\nend function\n",
        // A record payload on the `error` arm: the fields lower first, the
        // construct comes next, the variant references the construct result.
        "record ScriptError:\n  field code: I64\n  field message: Text\nend record\n\nfunction probe() returns Result[I64, ScriptError]:\n  return error(ScriptError(code: I64.literal(2), message: \"bad\"))\nend function\n",
        // The Script profile shape: a projected field, a literal, a construct
        // and the `ok` variant.
        "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n",
    ];
    for source in cases {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("a result variant must lower through the guest frontend: {source}")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

#[test]
fn sico_compiler_refuses_unresolved_result_variants_with_a_typed_identity() {
    // STEP-0495: the variant lowering is fail-closed. A payload whose type does
    // not match the declared arm must not be lowered, a payload shape the guest
    // cannot lower must stay a typed refusal, and `ok(...)` without a declared
    // `Result` return type must stay an unresolved call target.
    for (source, message) in [
        (
            "function probe() returns Result[I64, NumericError]:\n  return ok(\"text\")\nend function\n",
            "ERR:E-SH-IR-VARIANT-TYPE",
        ),
        (
            "function probe() returns Result[I64, NumericError]:\n  return error(I64.literal(1))\nend function\n",
            "ERR:E-SH-IR-VARIANT-TYPE",
        ),
        (
            "function probe() returns Result[I64, NumericError]:\n  return ok(sico.text.encode(\"x\"))\nend function\n",
            "ERR:E-SH-IR-VARIANT-PAYLOAD",
        ),
        (
            "function probe() returns I64:\n  return ok(I64.literal(1))\nend function\n",
            "ERR:E-SH-IR-CALL-TARGET",
        ),
    ] {
        assert_eq!(
            run_guest(source),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: message.into(),
            },
            "an unresolved result variant must stay a typed refusal: {}",
            source
        );
    }
}

#[test]
fn sico_compiler_extracts_the_run_emission_plan_byte_exactly() {
    // STEP-0496: the guest ingests the canonical IR JSON and derives the `run`
    // emission plan from it; the Rust oracle derives the same plan from the
    // same module. The two texts must agree byte for byte — otherwise the
    // guest is reading a different IR than the oracle emits, and every later
    // emission slice would be built on a wrong premise.
    //
    // The fixtures cover the supported subset end to end: a projected field, a
    // literal, a record construct, a result variant (scalar and record
    // payload), a string literal payload, and a call.
    let cases = [
        // The Script profile shape: project, const_i64, construct, variant.
        ECHO_COMPONENT_FIXTURE_SOURCE,
        // A scalar variant payload.
        "enum NumericError:\n  case Invalid\nend enum\n\nfunction run() returns Result[I64, NumericError]:\n  return ok(I64.literal(1))\nend function\n",
        // A record variant payload: const_i64, const_string, construct, variant.
        "record ScriptError:\n  field code: I64\n  field message: Text\nend record\n\nfunction run() returns Result[I64, ScriptError]:\n  return error(ScriptError(code: I64.literal(2), message: \"bad\"))\nend function\n",
        // A call to a sibling function.
        "function helper() returns I64:\n  return I64.literal(1)\nend function\n\nfunction run() returns I64:\n  return helper()\nend function\n",
    ];
    for source in cases {
        let expected =
            plan_text(&rust_module(source)).expect("the Rust oracle plans the run function");
        let RunOutcome::Output(output) =
            run_guest_with_args(source, vec!["--emit-ir-plan".to_owned()])
        else {
            panic!("the run plan must be extracted from the canonical IR: {source}")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

/// The local declaration bytes the Rust backend actually emits for the
/// Script-profile `run` function, in the same hexadecimal shape the guest
/// emits: one `count`/`valtype` group per slot.
fn oracle_run_locals_hex(source: &str) -> String {
    let module = rust_module(source);
    let bytes = sico_codegen_wasm::compile_script_program(&module)
        .expect("the oracle emits the Script-profile Component");
    let mut run_index = None;
    let mut bodies: Vec<Vec<(u32, u8)>> = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes).flatten() {
        match payload {
            wasmparser::Payload::ExportSection(reader) => {
                for export in reader.into_iter().flatten() {
                    if export.name == "run" && matches!(export.kind, wasmparser::ExternalKind::Func)
                    {
                        run_index = Some(export.index);
                    }
                }
            }
            wasmparser::Payload::CodeSectionEntry(body) => {
                let mut groups = Vec::new();
                let locals = body
                    .get_locals_reader()
                    .expect("the code body carries its local declarations");
                for local in locals.into_iter() {
                    let (count, ty) = local.expect("the local declaration group parses");
                    let byte = match ty {
                        wasmparser::ValType::I32 => 0x7f_u8,
                        wasmparser::ValType::I64 => 0x7e_u8,
                        other => {
                            panic!("the Script profile declares only i32 and i64 locals: {other:?}")
                        }
                    };
                    groups.push((count, byte));
                }
                bodies.push(groups);
            }
            _ => {}
        }
    }
    let index = run_index.expect("the core module exports run");
    bodies[index as usize]
        .iter()
        .map(|(count, byte)| format!("{count:02x}{byte:02x}"))
        .collect()
}

/// The `run` function's code bytes -- after the local declarations -- exactly
/// as the Rust backend emits them, in hexadecimal.
///
/// STEP-0501: the run-body emission truth. The oracle side reads the emitted
/// Component (production output, not a re-implementation), so the measured
/// table is the backend's own emission.
fn oracle_run_body_code_hex(source: &str) -> String {
    let module = rust_module(source);
    let bytes = sico_codegen_wasm::compile_script_program(&module)
        .expect("the oracle emits the Script-profile Component");
    let mut run_index = None;
    let mut bodies: Vec<String> = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes).flatten() {
        match payload {
            wasmparser::Payload::ExportSection(reader) => {
                for export in reader.into_iter().flatten() {
                    if export.name == "run" && matches!(export.kind, wasmparser::ExternalKind::Func)
                    {
                        run_index = Some(export.index);
                    }
                }
            }
            wasmparser::Payload::CodeSectionEntry(body) => {
                let mut reader = body
                    .get_binary_reader_for_operators()
                    .expect("the code body carries its operators");
                let remaining = reader.range().end - reader.original_position();
                let code = reader
                    .read_bytes(remaining)
                    .expect("the code body is bounded");
                bodies.push(code.iter().map(|byte| format!("{byte:02x}")).collect());
            }
            _ => {}
        }
    }
    let index = run_index.expect("the core module exports run");
    bodies[index as usize].clone()
}

/// The instruction segment of the `run` body: everything before the Script
/// result-area return.
///
/// STEP-0501: the terminator starts with `i32.const <result area size>`
/// followed by `call <allocator>`. The marker occurs exactly once in the
/// reference shape, so the cut is unambiguous and reproducible.
fn oracle_run_instruction_segment_hex(source: &str) -> String {
    const RESULT_AREA_MARKER: &str = "41201001";
    let hex = oracle_run_body_code_hex(source);
    let cut = hex
        .find(RESULT_AREA_MARKER)
        .expect("the Script result-area marker is present");
    hex[..cut].to_owned()
}

/// The Script result-area return and the dispatcher epilogue: everything from
/// the terminator to the end of the `run` body.
///
/// STEP-0503: the same unambiguous marker as the instruction segment; the
/// result segment is what follows it.
fn oracle_run_result_segment_hex(source: &str) -> String {
    const RESULT_AREA_MARKER: &str = "41201001";
    let hex = oracle_run_body_code_hex(source);
    let cut = hex
        .find(RESULT_AREA_MARKER)
        .expect("the Script result-area marker is present");
    hex[cut..].to_owned()
}

#[test]
fn oracle_run_body_emission_table_is_pinned() {
    // STEP-0501: the run-body emission truth, measured from the Rust
    // backend's production output (the Component `compile_script_program`
    // really emits, opened with wasmparser). This is the target the
    // guest-side emission has to reproduce byte for byte.
    //
    // The body splits at the Script result-area return: the terminator
    // begins with `i32.const 32` (the 32-byte result area) followed by
    // `call 1` (the allocator). The marker occurs exactly once in the
    // reference shape, so the cut is unambiguous.
    const FULL_BODY: &str = "4100211702400340201741004604402002210420032105200221062003210742002108200421092005210a2006210b2007210c2008210d4100210e2009210f200a2110200b2111200c2112200d21134100211441002115410021164120100121182018200e3a00002010100121192019200f2010fc0a0000201820193602082018201036020c201210012119201920112012fc0a000020182019360210201820123602142018201337031820180f0b000b0b000b";
    const INSTRUCTION_SEGMENT: &str = "4100211702400340201741004604402002210420032105200221062003210742002108200421092005210a2006210b2007210c2008210d4100210e2009210f200a2110200b2111200c2112200d2113410021144100211541002116";
    const TERMINATOR_AND_EPILOGUE: &str = "4120100121182018200e3a00002010100121192019200f2010fc0a0000201820193602082018201036020c201210012119201920112012fc0a000020182019360210201820123602142018201337031820180f0b000b0b000b";

    let hex = oracle_run_body_code_hex(ECHO_COMPONENT_FIXTURE_SOURCE);
    assert_eq!(hex, FULL_BODY, "the run body the backend actually emits");
    assert_eq!(
        hex.matches("41201001").count(),
        1,
        "the result-area marker is unique in the reference shape"
    );
    let cut = hex
        .find("41201001")
        .expect("the result-area marker is present");
    assert_eq!(&hex[..cut], INSTRUCTION_SEGMENT, "the instruction segment");
    assert_eq!(
        &hex[cut..],
        TERMINATOR_AND_EPILOGUE,
        "the Script result-area return and the dispatcher epilogue"
    );
    // The guest-side target for the next slice: the same segment, read from
    // the same production output.
    assert_eq!(
        oracle_run_instruction_segment_hex(ECHO_COMPONENT_FIXTURE_SOURCE),
        INSTRUCTION_SEGMENT
    );
}

#[test]
fn sico_compiler_emits_the_run_local_declaration_plan_byte_exactly() {
    // STEP-0499: the guest recomputes the local declaration plan of the
    // Script-profile `run` function from the ingested IR — one group per
    // instruction slot in IR order, then the dispatcher slot and the Script
    // scratch pair — and the plan must be byte-identical to the local
    // declaration section the Rust backend actually emits.
    //
    // The oracle side reads the emitted Component (production output, not a
    // re-implementation), so the comparison proves the guest's slot
    // assignment matches the backend's own emission rather than a parallel
    // model of it.
    let expected = oracle_run_locals_hex(ECHO_COMPONENT_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest_with_args(
        ECHO_COMPONENT_FIXTURE_SOURCE,
        vec!["--emit-locals-hex".to_owned()],
    ) else {
        panic!("the local declaration plan must be derived from the canonical IR")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // The Script-profile shape is pinned: four parameter slots are not local
    // declarations, the nineteen instruction slots are, then the dispatcher
    // and the scratch pair.
    assert_eq!(expected.len(), 84, "21 local declaration groups");
    assert!(
        expected.ends_with("017f027f"),
        "dispatcher then scratch pair"
    );
}

#[test]
fn sico_compiler_emits_the_run_instruction_segment_byte_exactly() {
    // STEP-0502: the guest derives the instruction segment of the `run` body
    // -- the dispatcher shell, the per-block dispatch heads and the block
    // instructions, up to (but not including) the Script result-area return --
    // from the ingested IR, and the bytes must be identical to the segment the
    // Rust backend actually emits.
    //
    // The oracle side reads the emitted Component (production output, not a
    // re-implementation), so the comparison proves the guest's slot assignment
    // and instruction emission match the backend's own emission rather than a
    // parallel model of it.
    let expected = oracle_run_instruction_segment_hex(ECHO_COMPONENT_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest_with_args(
        ECHO_COMPONENT_FIXTURE_SOURCE,
        vec!["--emit-run-instr-hex".to_owned()],
    ) else {
        panic!("the instruction segment must be derived from the canonical IR")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // The reference shape is pinned: the segment is 91 bytes, it opens with
    // the dispatcher shell and the block dispatch head, and it ends with the
    // zeroing of the inactive arm.
    assert_eq!(expected.len(), 182, "91 bytes of instruction segment");
    assert!(
        expected.starts_with("410021170240034020174100460440"),
        "dispatcher shell then the block dispatch head"
    );
    assert!(
        expected.ends_with("4100211541002116"),
        "the inactive arm is zeroed last"
    );
    assert_eq!(
        expected,
        "4100211702400340201741004604402002210420032105200221062003210742002108200421092005210a2006210b2007210c2008210d4100210e2009210f200a2110200b2111200c2112200d2113410021144100211541002116"
    );
}

#[test]
fn sico_compiler_refuses_the_run_instruction_segment_outside_the_subset() {
    // STEP-0502: the emission subset is the one the reference Script shape
    // uses. Anything else fails closed with a typed identity instead of an
    // approximation.
    let RunOutcome::Domain { code, message } =
        run_guest_with_args(IDENTITY_SOURCE, vec!["--emit-run-instr-hex".to_owned()])
    else {
        panic!("a source without a Script-profile run function must fail closed")
    };
    assert_eq!(code, "invalid-input");
    assert_eq!(message, "ERR:E-SH-CG-IR-PLAN");
}

#[test]
fn sico_compiler_emits_the_run_body_byte_exactly() {
    // STEP-0503: the guest derives the whole `run` body -- the instruction
    // segment, the Script result-area return and the dispatcher epilogue --
    // from the ingested IR, and the bytes must be identical to the body the
    // Rust backend actually emits.
    //
    // The oracle side reads the emitted Component (production output, not a
    // re-implementation), so the comparison proves the guest's slot
    // assignment and emission match the backend's own emission rather than a
    // parallel model of it.
    let expected = oracle_run_body_code_hex(ECHO_COMPONENT_FIXTURE_SOURCE);
    let RunOutcome::Output(output) = run_guest_with_args(
        ECHO_COMPONENT_FIXTURE_SOURCE,
        vec!["--emit-run-body-hex".to_owned()],
    ) else {
        panic!("the run body must be derived from the canonical IR")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    // The reference shape is pinned: the body is 180 bytes, it opens with the
    // dispatcher shell and it ends with the dispatcher's epilogue.
    assert_eq!(expected.len(), 360, "180 bytes of run body");
    assert!(
        expected.starts_with("410021170240034020174100460440"),
        "dispatcher shell then the block dispatch head"
    );
    assert!(
        expected.ends_with("0b000b0b000b"),
        "the dispatcher epilogue closes the run body"
    );
    // The result-area return and the epilogue are the STEP-0503 slice on their
    // own: the 89 bytes behind the unambiguous `i32.const 32` + `call 1`
    // marker.
    let result_segment = oracle_run_result_segment_hex(ECHO_COMPONENT_FIXTURE_SOURCE);
    assert_eq!(
        result_segment.len(),
        178,
        "89 bytes of result-area return and epilogue"
    );
    assert_eq!(
        result_segment,
        "4120100121182018200e3a00002010100121192019200f2010fc0a0000201820193602082018201036020c201210012119201920112012fc0a000020182019360210201820123602142018201337031820180f0b000b0b000b"
    );
}

#[test]
fn sico_compiler_refuses_the_run_body_outside_the_subset() {
    // STEP-0503: the result-area return is fail-closed (ADR-0016 decision 6).
    // A module without a Script-profile `run` and a `run` whose result is not
    // an `ok` variant of a `ScriptOutput` construct must each surface as their
    // own typed identity -- never as an approximation of the result area.
    for (source, message) in [
        // No `run` function in the module.
        (IDENTITY_SOURCE, "ERR:E-SH-CG-IR-PLAN"),
        // A `run` that returns a scalar is not the Script result shape, so
        // there is no result area to lower.
        (
            "function run() returns I64:\n  return I64.literal(0)\nend function\n",
            "ERR:E-SH-CG-IR-EMIT",
        ),
    ] {
        assert_eq!(
            run_guest_with_args(source, vec!["--emit-run-body-hex".to_owned()]),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: message.into(),
            },
            "a shape outside the result-area subset must stay a typed refusal: {source}"
        );
    }
}

#[test]
fn sico_compiler_refuses_ir_shapes_outside_the_run_plan_subset() {
    // STEP-0496: the plan extraction is fail-closed (ADR-0016 decision 6).
    // A module without a `run` function, an operation outside the supported
    // subset, a type outside the supported subset and a terminator other than
    // `return` must each surface as their own typed identity — never as an
    // approximation of the plan.
    for (source, message) in [
        // No `run` function in the module.
        (
            "function probe() returns I64:\n  return I64.literal(0)\nend function\n",
            "ERR:E-SH-CG-IR-PLAN",
        ),
        // A local cell lowers to `write_local`/`read_local`, which the plan
        // subset does not cover.
        (
            "function run() returns I64:\n  let value = I64.literal(1)\n  set value = I64.literal(2)\n  return value\nend function\n",
            "ERR:E-SH-CG-IR-OP",
        ),
        // A map parameter type is outside the plan type subset.
        (
            "function run(input: Map[Text, Text]) returns I64:\n  return I64.literal(0)\nend function\n",
            "ERR:E-SH-CG-IR-TYPE",
        ),
        // A branching body ends in a branch terminator, not `return`.
        (
            "function run(flag: Bool) returns I64:\n  if flag:\n    return I64.literal(1)\n  end if\n  return I64.literal(2)\nend function\n",
            "ERR:E-SH-CG-IR-TERMINATOR",
        ),
    ] {
        assert_eq!(
            run_guest_with_args(source, vec!["--emit-ir-plan".to_owned()]),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: message.into(),
            },
            "a shape outside the plan subset must stay a typed refusal: {source}"
        );
    }
}

#[test]
fn sico_compiler_emits_the_module_ir_split_across_both_channels_byte_exactly() {
    // STEP-0495: the canonical IR of the whole `selfhost/parser.sico` module
    // grew past the frozen Script v0 channel budget (RFC-0029 caps one
    // channel at 8 MiB), so the single-channel default path can no longer
    // carry it. The `--emit-ir-split` driver mode halves the IR across the
    // two channels; concatenating stdout and stderr must restore the exact
    // module IR, and the split point must sit at the byte midpoint.
    let source = format!("{PARSER_SOURCE}\n");
    let expected = rust_ir(&source);
    let outcome = run_guest_with_args(&source, vec!["--emit-ir-split".to_owned()]);
    let RunOutcome::Output(output) = outcome else {
        panic!("the split driver mode must return the module IR across both channels: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let mut assembled = output.stdout.clone();
    assembled.extend_from_slice(&output.stderr);
    assert_bytes_equal(&assembled, expected.as_bytes());
    assert_eq!(
        output.stdout.len(),
        expected.len() / 2,
        "the split must sit at the byte midpoint"
    );
    // The split mode stays fail-closed: a lowering refusal must surface as
    // the same typed identity the default path reports.
    assert_eq!(
        run_guest_with_args(
            "function probe() returns I64:\n  return missing_call()\nend function\n",
            vec!["--emit-ir-split".to_owned()],
        ),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TARGET".into(),
        },
        "the split driver mode must not hide a typed lowering refusal"
    );
}

#[test]
fn sico_compiler_lowers_gw_tail_match_both_arms_return_byte_exactly() {
    // STEP-0354: the general-while machine lowers a match whose both arms
    // return when it sits at the function tail: the oracle keeps the join
    // block but gives it an `unreachable` terminator (probe diagnostics), so
    // the machine registers the join in a new unreachable table and accepts
    // the terminated tail at the epilogue. Dead code after the same match
    // instead lands in the join block with a normal terminator (oracle
    // verified), which the machine already lowered — both shapes frozen.
    let base = format!("{}\n", parser_function_text("next_index"));
    let tail = format!(
        "{base}function probe(n: U64) returns U64:\n  let i = n\n  while U64.less_than(U64.literal(0), i):\n    set i = U64.literal(0)\n  end while\n  match U64.checked_add(i, U64.literal(1)):\n    case ok(t):\n      return t\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n"
    );
    let dead_code = format!(
        "{base}function probe(n: U64) returns U64:\n  let i = n\n  while U64.less_than(U64.literal(0), i):\n    set i = U64.literal(0)\n  end while\n  match U64.checked_add(i, U64.literal(1)):\n    case ok(t):\n      return t\n    case error(_):\n      return U64.literal(0)\n  end match\n  return U64.literal(1)\nend function\n"
    );
    let instr_count = format!("{}{}", base, parser_function_text("gw_instr_count"));
    for source in [&tail, &dead_code, &instr_count] {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("gw tail match with both arms returning must compile")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    // Declared bounds: an out-of-family match subject stays typed-refused,
    // then recovery.
    let bad_op = format!(
        "{base}function probe(n: U64) returns U64:\n  let i = n\n  while U64.less_than(U64.literal(0), i):\n    set i = U64.literal(0)\n  end while\n  match U64.bit_and(i, U64.literal(1)):\n    case ok(t):\n      return t\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n"
    );
    assert_eq!(
        run_guest(&bad_op),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TARGET".into()
        }
    );
    let good = &tail;
    let expected = rust_ir(good);
    let RunOutcome::Output(recovered) = run_guest(good) else {
        panic!("valid gw tail match must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_list_get_generic_u64_match_byte_exactly() {
    // STEP-0355: the list.get match-only machine accepts the generic subject
    // `sico.list.get[U64](values, idx)` on a three-parameter
    // `(values: List[U64], idx: U64, fallback: U64) returns U64` shape
    // (u64_list_at itself) — the intrinsic name carries the `[U64]` generic,
    // the result ok type and return type are u64, and the error arm returns
    // the third parameter. Out-of-family generic types, a U64 element without
    // the generic, and a string-literal fallback on the U64 shape stay
    // typed-refused.
    let good = parser_function_text("u64_list_at");
    let expected = rust_ir(&good);
    let RunOutcome::Output(output) = run_guest(&good) else {
        panic!("u64_list_at must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let bad_generic = good.replace("get[U64]", "get[I64]");
    assert_eq!(
        run_guest(&bad_generic),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    // A U64 element without the explicit generic reaches the return-kind
    // gate first (typed-refusal identity recorded as-is).
    let no_generic = good.replace("get[U64]", "get");
    assert_eq!(
        run_guest(&no_generic),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-TYPE-MISMATCH".into()
        }
    );
    let literal_fallback = good.replace("return fallback", "return \"0\"");
    assert_eq!(
        run_guest(&literal_fallback),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&good) else {
        panic!("u64_list_at must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_condition_generic_list_length_byte_exactly() {
    // STEP-0356: the general-while compare machine accepts the generic
    // `sico.list.length[U64](<param>)` as a condition operand (right side of
    // a while-header / if-condition compare) — gw_unreachable_terminator's
    // header shape — emitting the intrinsic name with the `[U64]` generic,
    // and accepts user calls on the right compare operand (gw_deferred_tail_json's
    // if shape in accessible declaration order). Out-of-family generic types
    // and a bare non-generic `sico.list.length` operand stay typed-refused.
    let base = format!(
        "{}\n{}\n",
        parser_function_text("u64_list_at"),
        parser_function_text("next_index"),
    );
    let good = format!(
        "{}{}",
        base,
        parser_function_text("gw_unreachable_terminator")
    );
    let expected = rust_ir(&good);
    let RunOutcome::Output(output) = run_guest(&good) else {
        panic!("gw_unreachable_terminator must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    let bad_generic = good.replace("length[U64]", "length[Text]");
    assert_eq!(
        run_guest(&bad_generic),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    // STEP-0382: the bare non-generic `sico.list.length(<list>)` operand
    // now lowers (this branch widened it), replacing the former
    // GWCOND-EXPRESSION refusal. This U64-list context is oracle-invalid
    // for the bare form, so the positive here is compile-only; the
    // byte-exact coverage of the bare form lives in the List[Text]
    // operand_nonparam_kind fixture below.
    let no_generic = good.replace("length[U64]", "length");
    let RunOutcome::Output(plain_output) = run_guest(&no_generic) else {
        panic!("bare sico.list.length operand must compile after STEP-0382")
    };
    assert_eq!(plain_output.exit_code, 0, "{:?}", plain_output.stderr);
    let extra_arg = no_generic.replace("length(gunreach)", "length(gunreach, gunreach)");
    assert_eq!(
        run_guest(&extra_arg),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&good) else {
        panic!("gw_unreachable_terminator must compile after typed refusals")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_compare_user_call_right_operand_byte_exactly() {
    // STEP-0357: the general-while compare machine accepts a user call on the
    // RIGHT compare operand (`U64.less_than(n, u64_list_at(rs, b, ...))`) —
    // gw_deferred_tail_json's if-condition shape. STEP-0356 landed the branch
    // without closing the gcp_r_done gate, so the shared width tail re-ran on
    // the call shape and refused EXPRESSION; this fixture pins the fixed
    // behavior byte-exactly.
    let source = format!(
        "{}\n{}\n{}",
        parser_function_text("u64_list_at"),
        parser_function_text("next_index"),
        "function probe(rs: List[U64], a: U64, b: U64) returns U64:\n  let n = U64.literal(0)\n  while U64.less_than(n, a):\n    if U64.less_than(n, u64_list_at(rs, b, U64.literal(0))):\n      set n = next_index(n, n)\n    end if\n  end while\n  return n\nend function\n",
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("user call on the right compare operand must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // A trailing extra argument keeps the shape refused (arity identity
    // propagated from the call packer).
    let malformed = source.replace(
        "u64_list_at(rs, b, U64.literal(0))",
        "u64_list_at(rs, b, U64.literal(0), b)",
    );
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-ARITY".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&source) else {
        panic!("valid right-operand call must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_concat_to_text_and_span_fixes_byte_exactly() {
    // STEP-0361: the general-while machine accepts `sico.u64.to_text(<cell>)`
    // as a concat argument (gw_text_concat_packed widened; the parser family
    // emits every JSON chain through it), and two latent span defects are
    // fixed: (A) the statement keyword search no longer hits substrings in
    // function names containing "let" (gcursor now starts after the
    // declaration colon), and (B) every function-end search uses
    // gw_find_code_text, which skips `//` comments and string literals —
    // machine bodies literally contain `find_text_from(src, "end function",
    // fn_start)` source text. This probe exercises all three at once: its
    // name contains "let", its body contains an "end function" string
    // literal, and it lowers a to_text concat argument byte-exactly.
    let source = format!(
        "{}
{}",
        parser_function_text("next_index"),
        "function probe_let_span(words: List[Text], lim: U64) returns Text:
  let marker = \"end function\"
  let acc = \"\"
  let i = U64.literal(0)
  while U64.less_than(i, lim):
    set acc = sico.text.concat(acc, sico.u64.to_text(i))
    set i = next_index(i, lim)
  end while
  return acc
end function
",
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("probe_let_span must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // A to_text concat argument with an extra operand stays refused, then
    // recovery.
    let malformed = source.replace("sico.u64.to_text(i)", "sico.u64.to_text(i, i)");
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&source) else {
        panic!("valid probe must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_find_code_text_byte_exactly() {
    // STEP-0363: gw_find_code_text — the string-literal/comment-aware search
    // that every function-end computation depends on — lowers itself through
    // the general-while machine (escape parity tracked by a previous-index
    // cell so no backward walk is needed). Its flip unblocks the
    // straight_let / list_get_match_ir / chained_let closures, which are
    // frozen via straight_let below.
    let base = format!(
        "{}\n{}\n{}",
        parser_function_text("find_text_from"),
        parser_function_text("next_index"),
        parser_function_text("gw_find_code_text"),
    );
    let straight_callees = [
        "is_word",
        "word_at",
        "line_end",
        "matching_close",
        "count_params",
        "is_canonical_magnitude",
        "decimal_at_most",
        "parameter_name_start",
        "scalar_type_kind",
        "is_ident_byte",
        "record_named_type",
        "parameters_ir",
        "binding_parameter_index",
        "binding_parameter_kind",
        "word_span_offset",
        "fixed_literal_argument",
        "fixed_op_name",
        "fixed_operation_operand_width",
        "fixed_operation_operand_atom_idx",
        "fixed_operation_operand_check",
        "fixed_operation_operand_value",
        "fixed_operation_literal_count",
        "fixed_operation_literal_instructions",
        "return_expression_end",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let good = format!(
        "{}\n{}\n{}",
        base,
        straight_callees,
        parser_function_text("straight_let_function_ir")
    );
    let expected = rust_ir(&good);
    let RunOutcome::Output(output) = run_guest(&good) else {
        panic!("gw_find_code_text closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let RunOutcome::Output(base_out) = run_guest(&base) else {
        panic!("gw_find_code_text base closure must compile")
    };
    assert_bytes_equal(&base_out.stdout, rust_ir(&base).as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_entry_region_match_byte_exactly() {
    // STEP-0370: the general-while machine accepts a checked-arithmetic
    // match in the function entry region (before any while) — the gate in
    // gw_match_open_pack refused entry-region matches with STATEMENT, which
    // blocked every source function that validates an argument before its
    // first loop (the parser family does this pervasively). Both placements
    // are frozen byte-exact: match-then-while (probe_t3) and
    // while-then-match (probe_t4).
    let entry_first = "function probe_t3(v: U64, lim: U64) returns U64:\n  let x = v\n  match U64.checked_add(v, U64.literal(1)):\n    case ok(n):\n      set x = n\n    case error(_):\n      set x = v\n  end match\n  let i = U64.literal(0)\n  while U64.less_than(i, lim):\n    set i = i\n  end while\n  return x\nend function\n";
    let loop_first = "function probe_t4(v: U64, lim: U64) returns U64:\n  let x = v\n  let i = U64.literal(0)\n  while U64.less_than(i, lim):\n    set i = i\n  end while\n  match U64.checked_add(v, U64.literal(1)):\n    case ok(n):\n      set x = n\n    case error(_):\n      set x = v\n  end match\n  return x\nend function\n";
    for source in [entry_first, loop_first] {
        let expected = rust_ir(source);
        let RunOutcome::Output(output) = run_guest(source) else {
            panic!("entry-region match probe must compile")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
    // An out-of-family entry match subject stays typed-refused, then
    // recovery.
    let malformed = entry_first.replace(
        "U64.checked_add(v, U64.literal(1))",
        "U64.bit_and(v, U64.literal(1))",
    );
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TARGET".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(entry_first) else {
        panic!("valid entry match must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, rust_ir(entry_first).as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_compare_non_generic_list_length_byte_exactly() {
    // STEP-0382: the general-while compare machine accepts the non-generic
    // `sico.list.length(<list>)` as a condition operand (the generic
    // `sico.list.length[U64]` form landed in STEP-0356; the bare form is
    // what operand-style guards actually use). This unblocked the whole
    // EXPRESSION refusal cluster — operand_nonparam_kind is frozen here as
    // the cluster's canary.
    let base = [
        "decimal_at_most",
        "fixed_literal_argument",
        "is_canonical_magnitude",
        "is_word",
        "local_binding_index",
        "next_index",
        "word_at",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("operand_nonparam_kind"),
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("operand_nonparam_kind closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // A trailing operand that is neither a list, parameter, cell, nor
    // literal stays typed-refused, then recovery.
    let malformed = source.replace(
        "sico.list.length(cell_names)",
        "sico.list.length(cell_names, cell_names)",
    );
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&source) else {
        panic!("valid operand closure must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_list_sort_and_text_join_byte_exactly() {
    // STEP-0384: the general-while RHS packer learns `sico.list.sort(<cell>)`
    // (new gw_list_sort_rhs_packed: read_local + intrinsic, oracle-range
    // exact) and `sico.text.join(<list>, <text>)` (routed to the existing
    // two-arg binary packer). A nested user-call set RHS was split into two
    // straight sets in records_table_json instead of another machine branch.
    // This unblocked the records_table_json census flip (GWPACK-OTHER -> FLIP).
    let helper = parser_function_text("next_index");
    let sort_src = format!(
        "{helper}
{}",
        "function t_sort_cells(entries_in: List[Text], n: U64) returns Text:
  let i = U64.literal(0)
  let entries = sico.text.split_lines(\"\")
  while U64.less_than(i, n):
    set entries = sico.list.sort(entries)
    set i = next_index(i, n)
  end while
  return \"ok\"
end function
"
    );
    let expected_sort = rust_ir(&sort_src);
    let RunOutcome::Output(sort_out) = run_guest(&sort_src) else {
        panic!("gw sort RHS shape must compile")
    };
    assert_eq!(sort_out.exit_code, 0, "{:?}", sort_out.stderr);
    assert_bytes_equal(&sort_out.stdout, expected_sort.as_bytes());

    let join_src = format!(
        "{helper}
{}",
        "function t_join_cells(entries_in: List[Text], n: U64) returns Text:
  let i = U64.literal(0)
  let entries = sico.text.split_lines(\"\")
  let body = \"\"
  while U64.less_than(i, n):
    set body = sico.text.join(entries, \",\")
    set i = next_index(i, n)
  end while
  return \"ok\"
end function
"
    );
    let expected_join = rust_ir(&join_src);
    let RunOutcome::Output(join_out) = run_guest(&join_src) else {
        panic!("gw join RHS shape must compile")
    };
    assert_eq!(join_out.exit_code, 0, "{:?}", join_out.stderr);
    assert_bytes_equal(&join_out.stdout, expected_join.as_bytes());

    // Guard: sort with a trailing argument stays typed-refused, then recovery.
    let malformed = sort_src.replace(
        "sico.list.sort(entries)",
        "sico.list.sort(entries, entries)",
    );
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&sort_src) else {
        panic!("valid sort shape must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected_sort.as_bytes());

    // The census flip itself: records_table_json closure byte-exact.
    let base = [
        "is_ident_byte",
        "is_word",
        "line_end",
        "next_index",
        "record_close_bracket",
        "record_map_comma",
        "record_named_type",
        "record_scalar_kind",
        "record_type_json",
        "type_json",
        "word_at",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join(
        "
",
    );
    let source = format!(
        "{}
{}",
        base,
        parser_function_text("records_table_json")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("records_table_json closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_text_length_rhs_byte_exactly() {
    // STEP-0385: the general-while RHS packer routes `sico.text.length`
    // through gw_intrinsic_call_packed (parameter and cell operands; the
    // call/literal argument forms are hoisted to cells at the two
    // straight_environment_literal_count / fixed_literal_argument sites).
    // This unblocked the straight_environment_literal_count census flip
    // (GWPACK-OTHER -> FLIP) and cascaded most of the GWPACK-OTHER cluster
    // to their next real gate.
    let helper = parser_function_text("next_index");
    let len_src = format!(
        "{helper}
{}",
        "function t_len_cells(words: List[Text], n: U64) returns Text:
  let i = U64.literal(0)
  let acc = \"\"
  let m = U64.literal(0)
  while U64.less_than(i, n):
    let w = word_at(words, i)
    set acc = sico.text.concat(acc, w)
    set m = sico.text.length(acc)
    set i = next_index(i, n)
  end while
  return acc
end function
"
    );
    let word_at = parser_function_text("word_at");
    let len_src = format!(
        "{word_at}
{len_src}"
    );
    let expected_len = rust_ir(&len_src);
    let RunOutcome::Output(len_out) = run_guest(&len_src) else {
        panic!("gw text.length cell shape must compile")
    };
    assert_eq!(len_out.exit_code, 0, "{:?}", len_out.stderr);
    assert_bytes_equal(&len_out.stdout, expected_len.as_bytes());

    // Guard: a call argument (not hoisted) stays typed-refused, then recovery.
    let malformed = len_src.replace(
        "set m = sico.text.length(acc)",
        "set m = sico.text.length(word_at(words, i))",
    );
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-SHAPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&len_src) else {
        panic!("valid text.length shape must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected_len.as_bytes());

    // The census flip itself: straight_environment_literal_count closure
    // byte-exact.
    let base = [
        "binding_parameter_index",
        "comma_count",
        "count_params",
        "decimal_at_most",
        "fixed_literal_argument",
        "fixed_operation_operand_atom_idx",
        "fixed_operation_operand_width",
        "is_canonical_magnitude",
        "is_word",
        "local_binding_index",
        "matching_close",
        "next_index",
        "word_at",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join(
        "
",
    );
    let source = format!(
        "{}
{}",
        base,
        parser_function_text("straight_environment_literal_count")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("straight_environment_literal_count closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_argument_instruction_count_closure_byte_exactly() {
    // STEP-0386: CALL-SHAPE cluster source reshaping — nested user calls
    // hoisted out of `to_text(..)` concat arguments (gw_deferred_tail_json),
    // a `to_text(U64.literal(..))` const argument hoisted to a cell
    // (nested_intrinsic_return_ir), and `text.length(user_call(..))` /
    // nested `next_index(matching_close(..))` sets split into let+set pairs
    // (argument_instruction_count family, including the
    // bytes.length-in-gw-set dispatch for gw_intrinsic_call_packed's own
    // body). Seven census closures flipped with zero regressions.
    let base = [
        "constant_argument_kind",
        "comma_count",
        "count_params",
        "decimal_at_most",
        "binding_parameter_index",
        "fixed_literal_argument",
        "function_declaration_index",
        "is_canonical_magnitude",
        "is_declaration_start",
        "is_string_token",
        "is_word",
        "matching_close",
        "next_index",
        "word_at",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join(
        "
",
    );
    let source = format!(
        "{}
{}",
        base,
        parser_function_text("argument_instruction_count")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("argument_instruction_count closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_intrinsic_call_packed_closure_byte_exactly() {
    // STEP-0387: RETURN-COUNT cluster root = gw_intrinsic_call_packed's own
    // body - the if/else "both arms return" shape is restructured to a
    // single trailing return (R3 single-return discipline), unblocking
    // the whole RETURN-COUNT census cluster.
    let base = [
        "binding_parameter_index",
        "find_text_from",
        "intrinsic_instruction_json",
        "is_word",
        "local_binding_index",
        "matching_close",
        "next_index",
        "read_local_instruction_json",
        "type_json",
        "u64_list_at",
        "word_at",
        "word_span_offset",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("gw_intrinsic_call_packed")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("gw_intrinsic_call_packed closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_argument_instructions_closure_byte_exactly() {
    // STEP-0388 canary flip: argument_instructions closure byte-exact
    // after the known-bad-shape library sweep.
    let base = [
        "argument_instruction_count",
        "argument_values",
        "argument_values_at",
        "binding_parameter_index",
        "binding_parameter_kind",
        "constant_argument_kind",
        "comma_count",
        "count_params",
        "decimal_at_most",
        "declared_return_kind",
        "find_text_from",
        "fixed_literal_argument",
        "function_declaration_index",
        "function_declaration_ordinal",
        "is_canonical_magnitude",
        "is_declaration_start",
        "is_string_token",
        "is_word",
        "json_string_token",
        "line_end",
        "matching_close",
        "next_index",
        "parameter_kind_by_id",
        "scalar_type_kind",
        "word_at",
        "word_span_offset",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("argument_instructions")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("argument_instructions closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_list_empty_rhs_byte_exactly() {
    // STEP-0389: the general-while RHS packer learns `sico.list.empty[U64]()`
    // (new gw_list_empty_rhs_packed, mirroring the gw_map_empty_rhs_packed
    // zero-argument pattern; offsets are word-position based, not
    // character-based - the first cut used +1 and every guard tripped
    // CALL-SHAPE). The shape initializes cell_locals in 41 parser.sico
    // sites, so this unblocked the user_call_expression_json census flip.
    let helper = parser_function_text("next_index");
    let src_fixture = format!(
        "{helper}
{}",
        "function t_empty_cells(words: List[Text], n: U64) returns Text:
  let i = U64.literal(0)
  let values = sico.list.empty[U64]()
  while U64.less_than(i, n):
    set i = next_index(i, n)
  end while
  return \"ok\"
end function
"
    );
    let expected = rust_ir(&src_fixture);
    let RunOutcome::Output(output) = run_guest(&src_fixture) else {
        panic!("gw list.empty shape must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());

    // Guard: a wrong element type stays typed-refused, then recovery.
    let malformed = src_fixture.replace("sico.list.empty[U64]()", "sico.list.empty[Text]()");
    assert_eq!(
        run_guest(&malformed),
        RunOutcome::Domain {
            code: "invalid-input".into(),
            message: "ERR:E-SH-IR-CALL-TYPE".into()
        }
    );
    let RunOutcome::Output(recovered) = run_guest(&src_fixture) else {
        panic!("valid list.empty shape must compile after typed refusal")
    };
    assert_bytes_equal(&recovered.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_straight_text_concat_packed_closure_byte_exactly() {
    // STEP-0394: straight_text_concat_packed census flip - a nested
    // `next_index(matching_close(..))` set and a `to_text(gw_decimal_value(..))`
    // concat argument hoisted to cells. wcr RETURN-COUNT hunting paused
    // (truncation bisection confounded by routing changes).
    let base = [
        "binding_parameter_index",
        "binding_parameter_kind",
        "call_instruction_json",
        "declared_return_kind",
        "find_text_from",
        "function_declaration_index",
        "function_declaration_ordinal",
        "gw_decimal_value",
        "intrinsic_instruction_json",
        "is_canonical_magnitude",
        "is_declaration_start",
        "is_intrinsic_path",
        "is_string_token",
        "is_word",
        "json_string_token",
        "line_end",
        "local_binding_index",
        "matching_close",
        "next_index",
        "read_local_instruction_json",
        "scalar_const_instruction_json",
        "scalar_type_kind",
        "type_json",
        "u64_list_at",
        "word_at",
        "word_span_offset",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("straight_text_concat_packed")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("straight_text_concat_packed closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_gw_list_sort_rhs_packed_closure_byte_exactly() {
    // STEP-0395: gw_list_sort_rhs_packed's own closure compiles - the
    // straight-context token-end computation must stay in the
    // `match checked_add(.., bytes.length(encode(word_at(..))))` subject
    // form (the straight machine rejects both hoisted-cell and let-bound
    // bytes.length operands; only the match-subject shape is modeled).
    let base = [
        "find_text_from",
        "intrinsic_instruction_json",
        "is_word",
        "local_binding_index",
        "matching_close",
        "next_index",
        "read_local_instruction_json",
        "type_json",
        "u64_list_at",
        "word_at",
        "word_span_offset",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("gw_list_sort_rhs_packed")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("gw_list_sort_rhs_packed closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_checked_arithmetic_match_ir_closure_byte_exactly() {
    // STEP-0396: the match-subject token-end folding (69 sites library-wide:
    // `let __enc = encode(ARG)` + `set __h = bytes.length(__enc)` +
    // `match checked_add(off, __h)` folded into a single match subject)
    // flipped checked_arithmetic_match_ir; checked_arithmetic_set_match_ir
    // progresses into the registered casm wasm-trap defect class (0365).
    let base = [
        "argument_instruction_count",
        "argument_instructions",
        "argument_region_instruction_count",
        "argument_segment_kind",
        "argument_values",
        "argument_values_at",
        "binding_parameter_index",
        "binding_parameter_kind",
        "constant_argument_kind",
        "comma_count",
        "count_params",
        "decimal_at_most",
        "declared_return_kind",
        "find_text_from",
        "fixed_literal_argument",
        "function_declaration_index",
        "function_declaration_ordinal",
        "gw_find_code_text",
        "is_canonical_magnitude",
        "is_declaration_start",
        "is_string_token",
        "is_word",
        "json_string_token",
        "line_end",
        "matching_close",
        "next_index",
        "parameter_kind_by_id",
        "parameter_name_start",
        "is_ident_byte",
        "record_named_type",
        "parameters_ir",
        "scalar_type_kind",
        "word_at",
        "word_span_offset",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("checked_arithmetic_match_ir")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("checked_arithmetic_match_ir closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_while_call_rhs_packed_closure_byte_exactly() {
    // STEP-0400: the function-level giant-if flattening (guard + top-level
    // body) plus the while_call_rhs_empty_args helper extraction unblocked
    // the whole RETURN-COUNT machine cluster - 19 closures flipped at once,
    // most cascading into the registered STEP-0365 wasm-trap defect class.
    // wcr is frozen here as the cluster's canary.
    let base = [
        "binding_parameter_index",
        "binding_parameter_kind",
        "call_instruction_json",
        "constant_argument_kind",
        "comma_count",
        "count_params",
        "decimal_at_most",
        "declared_return_kind",
        "find_text_from",
        "fixed_literal_argument",
        "fixed_literal_operand_json",
        "fixed_literal_const_segment",
        "fixed_operation_operand_width",
        "function_declaration_index",
        "function_declaration_ordinal",
        "gw_call_chain_packed",
        "gw_instr_count",
        "gw_ut_chain_packed",
        "intrinsic_instruction_json",
        "is_canonical_magnitude",
        "is_declaration_start",
        "is_intrinsic_path",
        "is_string_token",
        "is_word",
        "json_string_token",
        "line_end",
        "local_binding_index",
        "matching_close",
        "next_index",
        "parameter_kind_by_id",
        "read_local_instruction_json",
        "scalar_const_instruction_json",
        "scalar_type_kind",
        "type_json",
        "u64_list_at",
        "while_call_rhs_empty_args",
        "word_at",
        "word_span_offset",
        "fixed_literal_const_instruction",
        "fixed_op_name",
        "fixed_operation_instruction_json",
        "fixed_operation_operand_atom_idx",
        "gw_decimal_value",
        "gw_intrinsic_call_packed",
        "gw_list_append_u64_rhs_packed",
        "gw_list_empty_rhs_packed",
        "gw_list_sort_rhs_packed",
        "gw_map_empty_rhs_packed",
        "gw_map_put_rhs_packed",
        "gw_ok_wrap_rhs_packed",
        "gw_rhs_at_dispatch",
        "gw_rhs_at_param_arm",
        "gw_rhs_error_arm",
        "gw_rhs_multi_pack",
        "gw_rhs_ok_arm",
        "gw_rhs_packed",
        "gw_rhs_single_pack",
        "gw_rhs_split_lines_emit",
        "gw_rhs_utf8_decode_arm",
        "gw_text_concat_packed",
        "gw_text_list_binary_packed",
        "gw_text_trim_user_call_packed",
        "gw_u64_less_than_value_packed",
        "gwf_l_operand",
        "gwf_r_operand",
        "operand_cell_or_literal_json",
        "operand_nonparam_kind",
        "parameter_type_word",
        "script_result_return_json",
        "while_bytes_at_local_src_packed",
        "while_bytes_at_rhs_packed",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("while_call_rhs_packed")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("while_call_rhs_packed closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

/// STEP-0515: the same Script-profile fixture, but the host call sits in a
/// `let` binding instead of the return position. The canonical IR is the
/// **same shape** as the return-position fixture -- one `intrinsic`, result 1,
/// the terminator returning 1 -- with only the source range shifted, because
/// the Rust frontend propagates a straight-line `let` binding into the SSA
/// value instead of creating a local.
const HOST_CALL_IN_LET_BINDING_FIXTURE_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction size(data: Bytes) returns U64:\n  let n = sico.bytes.length(data)\n  return n\nend function\n\nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

#[test]
fn sico_compiler_lowers_the_host_call_in_a_let_binding_byte_exactly() {
    // STEP-0515, frontend knife: a host call (`sico.bytes.length`) in a `let`
    // binding of a straight-line body now lowers to the canonical intrinsic
    // operation -- **byte for byte equal** to the Rust frontend's own canonical
    // IR, not merely shape-equal.
    //
    // The truth is the Rust frontend's production output (STEP-0511, the
    // `host-c` artifact): the `let` binding is propagated into the SSA value,
    // so the canonical IR is the *same shape* as the return-position fixture --
    // one `intrinsic`, result 1, the terminator returning 1, **no local** --
    // with only the source range shifted (369..392 instead of 361..384).
    //
    // The landing point is the dedicated `direct_host_call_let_ir` route in the
    // guest frontend, wired into the route ladder ahead of the general
    // statement analysis. The route claims one shape and returns SKIP for
    // everything else, so no existing refusal identity changes.
    let expected = rust_ir(HOST_CALL_IN_LET_BINDING_FIXTURE_SOURCE);
    assert_eq!(
        expected.len(),
        2215,
        "the measured canonical IR size of the let-binding fixture is pinned"
    );
    let RunOutcome::Output(output) = run_guest(HOST_CALL_IN_LET_BINDING_FIXTURE_SOURCE) else {
        panic!("the guest must lower a host call in a let binding after STEP-0515")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let ir = String::from_utf8(output.stdout).expect("the guest IR is utf-8");
    assert!(
        ir.contains("\"op\":\"intrinsic\""),
        "the host call lowers to one intrinsic operation: {ir}"
    );
    assert!(
        ir.contains("\"name\":\"sico.bytes.length\""),
        "the intrinsic keeps its dotted name: {ir}"
    );
    assert!(
        ir.contains("\"arguments\":[0]"),
        "the intrinsic arguments are the argument value ids: {ir}"
    );
    assert!(
        ir.contains("\"range\":{\"start\":369,\"end\":392}"),
        "the intrinsic range is the call expression span: {ir}"
    );
    assert!(
        !ir.contains("\"op\":\"write_local\"") && !ir.contains("\"locals\""),
        "the let binding is propagated into the SSA value, not lowered into a local: {ir}"
    );
}

#[test]
fn sico_codegen_guest_emits_the_host_call_let_binding_component_byte_exactly() {
    // STEP-0515, codegen side: the let-binding fixture lowers to the same
    // intrinsic shape, so the whole component must equal the Rust backend's
    // own production output byte for byte -- 1025 B, measured in STEP-0511.
    let module = rust_module(HOST_CALL_IN_LET_BINDING_FIXTURE_SOURCE);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the let-binding fixture");
    assert_eq!(
        expected.len(),
        1025,
        "the measured let-binding component size is pinned"
    );
    let RunOutcome::Output(output) = run_guest_with_args(
        HOST_CALL_IN_LET_BINDING_FIXTURE_SOURCE,
        vec!["--emit-component-hex".into()],
    ) else {
        panic!("the let-binding component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted let-binding component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted let-binding component must pass the real Wasm validator");
}

#[test]
fn sico_compiler_lowers_the_step0515_parser_function_byte_exactly() {
    // STEP-0515: the guest's own new frontend route has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the
    // guest's own new code, otherwise the bootstrap could never consume its
    // own source. Exactly like the STEP-0507 / STEP-0509 / STEP-0512 slices
    // established for the functions they added.
    //
    // The closure is the transitive callee set of `direct_host_call_let_ir`
    // in parser.sico, emitted in file order (the guest frontend refuses
    // forward-referenced user-function calls).
    let base = [
        "next_index",
        "word_at",
        "is_word",
        "is_ident_byte",
        "record_named_type",
        "type_json",
        "line_end",
        "intrinsic_instruction_json",
        "find_text_from",
        "word_span_offset",
        "binding_parameter_index",
        "matching_close",
        "is_intrinsic_path",
        "scalar_type_kind",
        "binding_parameter_kind",
        "gw_find_code_text",
        "return_expression_end",
        "parameter_name_start",
        "parameters_ir",
    ]
    .iter()
    .map(|name| parser_function_text(name))
    .collect::<Vec<_>>()
    .join("\n");
    let source = format!(
        "{}\n{}",
        base,
        parser_function_text("direct_host_call_let_ir")
    );
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("direct_host_call_let_ir closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

// ---------------------------------------------------------------------------
// STEP-0517: the T1 family truth -- `sico.text.concat` over two `Text`
// arguments.
//
// STEP-0516 closed the *scale* question for the W2 wave. STEP-0517 measures
// the first *family* the W2 wave has to close: `sico.text.concat`, the call
// the M22 plan ranks first by call-site count in S.
//
// Every number below is measured from the Rust frontend's canonical IR and
// the Rust backend's own production component -- never designed.
// ---------------------------------------------------------------------------

/// The Script-profile prelude and the `run` entry, shared with the host-call
/// fixtures above so the two families stay comparable.
fn text_concat_fixture(body: &str) -> String {
    format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
}

#[test]
fn sico_compiler_lowers_the_text_concat_in_a_let_binding_byte_exactly() {
    // STEP-0517, T1 truth (frontend side): the `sico.text.concat` call in a
    // `let` binding is **already** lowered by the guest frontend -- byte for
    // byte equal to the Rust frontend's canonical IR, not merely shape-equal.
    //
    // The canonical shape is one `intrinsic` operation named
    // `sico.text.concat` whose two arguments are the two argument value ids:
    //
    //   {"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}
    //
    // The `let` binding is propagated into the SSA value (no `write_local`,
    // no `locals`), exactly like the STEP-0515 host-call fixture. Both
    // already-lowered shapes are pinned here: the arguments as parameters and
    // the arguments as local cells.
    for (label, body) in [
        (
            "parameters",
            "function join(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, b)\n  return c\nend function",
        ),
        (
            "cells",
            "function join(a: Text, b: Text) returns Text:\n  let x = a\n  let y = b\n  let c = sico.text.concat(x, y)\n  return c\nend function",
        ),
    ] {
        let source = text_concat_fixture(body);
        let expected = rust_ir(&source);
        assert_eq!(
            expected.len(),
            2295,
            "the measured T1 canonical IR size is pinned ({label})"
        );
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("the guest must lower a text concat in a let binding ({label})")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
        let ir = String::from_utf8(output.stdout).expect("the guest IR is utf-8");
        assert!(
            ir.contains("\"op\":\"intrinsic\""),
            "the text concat lowers to one intrinsic operation ({label}): {ir}"
        );
        assert!(
            ir.contains("\"name\":\"sico.text.concat\""),
            "the intrinsic keeps its dotted name ({label}): {ir}"
        );
        assert!(
            ir.contains("\"arguments\":[0,1]"),
            "the intrinsic arguments are the two argument value ids ({label}): {ir}"
        );
        assert!(
            !ir.contains("\"op\":\"write_local\"") && !ir.contains("\"locals\""),
            "the let binding is propagated into the SSA value, not lowered into a local ({label}): {ir}"
        );
    }
}

#[test]
fn sico_compiler_lowers_the_text_concat_in_a_return_position_byte_exactly() {
    // STEP-0517, T1 truth: the return-expression position is **not** a
    // missing capability -- it is a *gated* one.
    //
    // The same `return sico.text.concat(a, b)` lowers **byte for byte** as soon
    // as the function body carries one more statement (a `let` or a `set`).
    // The Rust frontend drops the dead binding and keeps the intrinsic in the
    // return position:
    //
    //   {"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}
    //   ... "terminator":{"kind":"return","data":2}
    //
    // The gate lives in the guest's route ladder: the `straight_cell_*` family
    // is only entered when the body has a statement besides the return
    // (`body_statement_count = max(let_count, set_count) > 0`). A body whose
    // only statement is the return never reaches the route that already
    // implements this shape, and falls through to the scalar-return analysis,
    // which refuses it with `ERR:E-SH-IR-EXPRESSION`.
    //
    // This is the measured cut point for the W2 frontend knife: the shape
    // works, the *reachability* is what is missing.
    let source = text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, b)\nend function",
    );
    let expected = rust_ir(&source);
    assert_eq!(
        expected.len(),
        2295,
        "the measured T1 return-position canonical IR size is pinned"
    );
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!(
            "the guest must lower a text concat in a return position when the body has a statement"
        )
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let ir = String::from_utf8(output.stdout).expect("the guest IR is utf-8");
    assert!(
        ir.contains("\"op\":\"intrinsic\""),
        "the text concat lowers to one intrinsic operation: {ir}"
    );
    assert!(
        ir.contains("\"name\":\"sico.text.concat\""),
        "the intrinsic keeps its dotted name: {ir}"
    );
    assert!(
        ir.contains("\"arguments\":[0,1]"),
        "the intrinsic arguments are the two argument value ids: {ir}"
    );
    assert!(
        ir.contains("\"terminator\":{\"kind\":\"return\",\"data\":2}"),
        "the intrinsic result is the returned value: {ir}"
    );
}

#[test]
fn oracle_text_concat_component_truth_is_pinned() {
    // STEP-0517, T1 truth (codegen side): the frontend lowers the shape, but
    // at the STEP-0517 measurement time the **component** emitter did not. The
    // Rust backend's own production output for the same source is 1094 bytes;
    // the guest emitter refused with the typed "unsupported codegen source
    // shape".
    //
    // That refusal was the fail-closed behaviour ADR-0016 decision 6 demands:
    // the emitter supported `sico.bytes.length` only (STEP-0513), so
    // `sico.text.concat` was outside the subset and was never approximated.
    //
    // STEP-0519 landed the W2 codegen knife, so the guest half below is now
    // the byte-exact comparison this pin promised -- the original refusal
    // identity stays in the STEP-0517 record for traceability.
    let source = text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, b)\n  return c\nend function",
    );
    let module = rust_module(&source);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the T1 fixture");
    assert_eq!(
        expected.len(),
        1094,
        "the measured T1 component size is pinned"
    );
    wasmparser::Validator::new()
        .validate_all(&expected)
        .expect("the measured T1 component must pass the real Wasm validator");
    let RunOutcome::Output(output) =
        run_guest_with_args(&source, vec!["--emit-component-hex".into()])
    else {
        panic!("the text-concat component must be emitted through the guest after STEP-0519")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted text-concat component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted text-concat component must pass the real Wasm validator");
}

#[test]
fn sico_compiler_lowers_the_step0518_parser_function_byte_exactly() {
    // STEP-0518: the guest's own new frontend route has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the
    // guest's own new code, otherwise the bootstrap could never consume its
    // own source. Exactly like the STEP-0507 / STEP-0509 / STEP-0512 / STEP-0515
    // slices established for the functions they added.
    //
    // The closure is the transitive callee set of `direct_text_concat_return_ir`
    // in parser.sico, emitted in file order. It is the STEP-0512 closure of
    // the sibling route (`direct_host_call_return_ir`) -- the new route calls
    // the same helpers -- plus the route itself, which sits after them.
    //
    // STEP-0523 reshaped the route: the operand lowering moved into
    // `straight_text_concat_packed` (the helper the `let`-binding routes
    // already use), so the closure grew by that helper and its own callees.
    // The closure is now computed from the file instead of being kept by
    // hand, so it cannot drift out of date when the route changes again.
    let source = parser_closure_source("direct_text_concat_return_ir");
    let expected = rust_ir(&source);
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("direct_text_concat_return_ir closure must compile")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
}

#[test]
fn sico_compiler_lowers_the_text_concat_as_the_whole_expression_byte_exactly() {
    // STEP-0518, T1 frontend knife: the `sico.text.concat` call as the **whole
    // expression** of a straight-line body -- the shape the STEP-0517 truth
    // slice measured as still refused -- now lowers to the canonical
    // intrinsic operation, **byte for byte equal** to the Rust frontend's own
    // canonical IR.
    //
    // The STEP-0517 slice measured the real cause: the guest already had a
    // working, byte-exact return-position route (`straight_cell_function_ir`
    // and `straight_environment_function_ir`), but the route ladder only
    // entered that family when the body carried a statement besides the
    // return (`body_statement_count = max(let_count, set_count) > 0`). A body
    // whose only statement is the return never reached it.
    //
    // The landing point is the dedicated `direct_text_concat_return_ir` route
    // in the guest frontend, wired into the route ladder ahead of the general
    // statement analysis. The route claims one shape and returns SKIP for
    // everything else, so no existing refusal identity changes.
    let source = text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  return sico.text.concat(a, b)\nend function",
    );
    let expected = rust_ir(&source);
    assert_eq!(
        expected.len(),
        2295,
        "the measured T1 whole-expression canonical IR size is pinned"
    );
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("the guest must lower a text concat as the whole expression after STEP-0518")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let ir = String::from_utf8(output.stdout).expect("the guest IR is utf-8");
    assert!(
        ir.contains("\"op\":\"intrinsic\""),
        "the text concat lowers to one intrinsic operation: {ir}"
    );
    assert!(
        ir.contains("\"name\":\"sico.text.concat\""),
        "the intrinsic keeps its dotted name: {ir}"
    );
    assert!(
        ir.contains("\"arguments\":[0,1]"),
        "the intrinsic arguments are the two parameter value ids: {ir}"
    );
    assert!(
        ir.contains("\"terminator\":{\"kind\":\"return\",\"data\":2}"),
        "the intrinsic result is the returned value: {ir}"
    );
    assert!(
        ir.contains("\"range\":{\"start\":374,\"end\":396}"),
        "the intrinsic range is the call expression span: {ir}"
    );
}

#[test]
fn sico_compiler_keeps_the_unclosed_text_concat_shapes_typed_refused() {
    // STEP-0517 / STEP-0518: the knife is one shape, not a widening of the
    // `sico.text.concat` surface. The shapes the STEP-0517 truth slice
    // measured next to the supported one stay typed refusals.
    //
    // STEP-0518 claimed the two-parameter shape whose whole expression is the
    // call -- the shape this pin refused at STEP-0517 -- so that case moved
    // to the STEP-0518 pin above. Every shape the knife does *not* claim
    // keeps the identity the STEP-0517 measurement recorded
    // (`ERR:E-SH-IR-EXPRESSION`): fail-closed, no silent reinterpretation.
    //
    // STEP-0523 widened the single-`return` route: the three shapes this pin
    // used to refuse -- two literal operands, a nested host call in operand
    // position, and a mixed operand pair -- are now **claimed** (they are
    // pinned byte for byte by the T1 family truth table and by
    // `sico_codegen_guest_lowers_the_gd_single_return_shapes_byte_exactly`).
    // The shapes that are still unclaimed are the ones below, and they keep
    // the same fail-closed identity.
    for (label, body) in [
        (
            "user-call-with-two-arguments",
            "function join(a: Text, b: U64) returns Text:\n  return sico.text.concat(helper(a, b), \"20\")\nend function\nfunction helper(x: Text, y: U64) returns Text:\n  return \"z\"\nend function",
        ),
        (
            "wrong-operand-kind",
            "function join(a: U64) returns Text:\n  return sico.text.concat(a, a)\nend function",
        ),
    ] {
        let source = text_concat_fixture(body);
        let RunOutcome::Domain { code, message } = run_guest(&source) else {
            panic!("the {label} text-concat shape must stay a typed refusal")
        };
        assert_eq!(code, "invalid-input", "{label}");
        assert_eq!(message, "ERR:E-SH-IR-EXPRESSION", "{label}");
    }
}

#[test]
fn the_text_concat_arity_is_refused_by_the_rust_frontend_itself() {
    // STEP-0517, T1 truth: the arity boundary of `sico.text.concat` is
    // enforced by the **semantic layer**, not by the IR layer -- the Rust
    // frontend itself refuses one argument and three arguments with
    // `stdlib intrinsic arity`.
    //
    // That matters for the W2 wave: the guest frontend must not invent its
    // own arity check, and the guest's refusal on these shapes is not a
    // guest-specific gap.
    for (label, body) in [
        (
            "one-argument",
            "function join(a: Text) returns Text:\n  return sico.text.concat(a)\nend function",
        ),
        (
            "three-arguments",
            "function join(a: Text, b: Text, c: Text) returns Text:\n  return sico.text.concat(a, b, c)\nend function",
        ),
    ] {
        let source = text_concat_fixture(body);
        let source_file = SourceFile::from_text(SourceId::new(0), "selfhost-input.sico", source)
            .expect("the fixture source is bounded");
        let error = match lower_core(&source_file) {
            Ok(_) => panic!("the Rust frontend must refuse the {label} arity"),
            Err(error) => format!("{error:?}"),
        };
        assert!(
            error.contains("stdlib intrinsic arity"),
            "the {label} arity must be refused by the semantic layer: {error}"
        );
    }
}

// ---------------------------------------------------------------------------
// STEP-0519: the W2 family's T1 codegen knife -- the `sico.text.concat`
// intrinsic reaches the component emitter.
//
// The rules are measured from the backend's production output (the oracle
// component's type, function, export and code sections, parsed in STEP-0519):
//   * the call site pushes the two operands' four flat slots (pointer and
//     length each), calls the helper, then stores the result length and the
//     result pointer;
//   * the helper is a core function of its own: its core function index is
//     the module function count plus the three arena helpers, its type is
//     the four-parameter / two-result function type, and its body is the
//     backend's own `emit_concat` shape (the total length, the arena
//     allocation, the two linear copies);
//   * the type, function and code sections carry one extra entry for the
//     helper -- and only when the module really uses the intrinsic, so a
//     module without it keeps the three-entry shape.
// ---------------------------------------------------------------------------

/// The T1 fixture with a third module function, so the helper's core function
/// index is not the two-function value.
fn text_concat_three_function_fixture() -> String {
    format!(
        "{HOST_CALL_SHAPE_PRELUDE}{}\n{}\n{HOST_CALL_SHAPE_RUN}",
        "function join(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, b)\n  return c\nend function",
        "function size(data: Bytes) returns U64:\n  let n = sico.bytes.length(data)\n  return n\nend function"
    )
}

#[test]
fn sico_codegen_guest_emits_the_text_concat_helper_index_from_the_module_function_count() {
    // STEP-0519: the concat helper's core function index is **derived**, not
    // hard-coded: it is the module function count plus the three arena
    // helpers. A module with a third function must move the helper -- and
    // the type, function and code section entries with it -- and the whole
    // component must still equal the Rust backend's own production output
    // byte for byte.
    let source = text_concat_three_function_fixture();
    let module = rust_module(&source);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the T1 fixture");
    let RunOutcome::Output(output) =
        run_guest_with_args(&source, vec!["--emit-component-hex".into()])
    else {
        panic!("the three-function text-concat component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted three-function component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted three-function component must pass the real Wasm validator");
}

#[test]
fn sico_codegen_guest_keeps_the_modules_without_the_concat_helper_unchanged() {
    // STEP-0519: the helper's section entries are **conditional**. A module
    // that does not use `sico.text.concat` must keep the three-entry shape
    // (three arena helper types, three arena helper functions, three arena
    // helper bodies) -- the extra type, function and code entry belong to the
    // concat helper alone.
    //
    // The host-call fixture is the reference: it uses `sico.bytes.length`,
    // which the backend emits inline and which therefore registers no helper.
    let source = HOST_CALL_IN_STRAIGHT_LINE_FIXTURE_SOURCE;
    let module = rust_module(source);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the host-call fixture");
    assert_eq!(
        expected.len(),
        1025,
        "the measured host-call component size is pinned"
    );
    let RunOutcome::Output(output) =
        run_guest_with_args(source, vec!["--emit-component-hex".into()])
    else {
        panic!("the host-call component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "a module without the concat helper must keep the three-entry shape"
    );
}

#[test]
fn sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly() {
    // STEP-0519: the guest's own new emission code has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the
    // guest's own emitter, otherwise the bootstrap could never consume its
    // own code. Exactly like the STEP-0507 / STEP-0509 / STEP-0513 slices
    // established for the functions they added.
    //
    // The four scan functions and the `sico.text.concat` branch are new; the
    // three section functions and the intrinsic emitter were reshaped.
    for name in [
        "cgs_function_count",
        "cgs_instruction_is_text_concat",
        "cgs_block_has_text_concat",
        "cgs_function_has_text_concat",
        "cgs_module_has_text_concat",
        "cg_concat_helper_body_hex",
        "cgs_intrinsic_hex",
        "cgs_core_types_hex",
        "cgs_core_funcs_hex",
        "cgs_core_code_hex",
    ] {
        let source = codegen_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("codegen function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// STEP-0520: the W2 family's **local cell** support in the guest codegen.
//
// The gap STEP-0519 left open is measured, not guessed: a `let` binding that
// is later **assigned** is lowered by the Rust frontend into a *cell* -- the
// function carries `locals`, and the binding becomes two `write_local`
// operations whose result type is `unit`. The backend's own layout then puts
// the cells after the instruction results and before the dispatcher slot, and
// emits a cell write as a slot-by-slot copy.
//
// The reference shape (the first one that needs a cell at all):
//
//   function join(a: Text, b: Text) returns Text:
//     let x = a
//     set x = b
//     return sico.text.concat(a, b)
//   end function
//
// Its canonical IR is 2630 bytes and its production Component is 1114 bytes;
// both numbers are measured from the Rust frontend and the Rust backend.
// ---------------------------------------------------------------------------

/// The first shape that needs the guest codegen's local cell support: a
/// `let` binding that is later assigned, so the frontend materialises it as a
/// cell with two unit-typed `write_local` operations.
fn text_concat_cell_fixture() -> String {
    text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  let x = a\n  set x = b\n  return sico.text.concat(a, b)\nend function",
    )
}

#[test]
fn sico_compiler_lowers_the_text_concat_with_an_assigned_cell_byte_exactly() {
    // STEP-0520, frontend half: the assigned-binding shape lowers to the
    // canonical IR **byte for byte** -- the cell is materialised exactly the
    // way the Rust frontend materialises it.
    //
    // The canonical shape is a function with a `locals` array, two
    // `write_local` operations whose result type is `unit`, and the intrinsic
    // that reads the two parameters:
    //
    //   {"op":"write_local","data":{"local":0,"value":0}}   ty: unit
    //   {"op":"write_local","data":{"local":0,"value":1}}   ty: unit
    //   {"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}
    let source = text_concat_cell_fixture();
    let expected = rust_ir(&source);
    assert_eq!(
        expected.len(),
        2630,
        "the measured local-cell canonical IR size is pinned"
    );
    let RunOutcome::Output(output) = run_guest(&source) else {
        panic!("the guest must lower the assigned-binding text concat shape")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    assert_bytes_equal(&output.stdout, expected.as_bytes());
    let ir = String::from_utf8(output.stdout).expect("the guest IR is utf-8");
    assert!(
        ir.contains("\"op\":\"write_local\""),
        "the assigned binding lowers into a cell write: {ir}"
    );
    assert!(
        ir.contains("\"locals\":["),
        "the function carries the cell declaration: {ir}"
    );
    assert!(
        ir.contains("\"ty\":{\"kind\":\"unit\"}"),
        "a cell write carries no value, so its result type is unit: {ir}"
    );
    assert_eq!(
        ir.matches("\"op\":\"write_local\"").count(),
        2,
        "the assigned binding is written twice, once per assignment: {ir}"
    );
    assert!(
        ir.contains("\"locals\":[{\"name\":\"x\",\"ty\":{\"kind\":\"string\"}"),
        "the cell keeps the declared binding and its type: {ir}"
    );
}

#[test]
fn oracle_local_cell_component_truth_is_pinned() {
    // STEP-0520, codegen half: the guest's Component emitter now covers the
    // local-cell shape, and the Component is **byte for byte** the Rust
    // backend's own production output.
    //
    // The measured truth (the oracle's production Component for the same
    // source) is 1114 bytes; the guest must emit exactly those bytes, and
    // both sides must pass the real Wasm validator.
    let source = text_concat_cell_fixture();
    let module = rust_module(&source);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the cell fixture");
    assert_eq!(
        expected.len(),
        1114,
        "the measured local-cell component size is pinned"
    );
    wasmparser::Validator::new()
        .validate_all(&expected)
        .expect("the measured local-cell component must pass the real Wasm validator");
    let RunOutcome::Output(output) =
        run_guest_with_args(&source, vec!["--emit-component-hex".into()])
    else {
        panic!("the local-cell component must be emitted through the guest after STEP-0520")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted local-cell component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted local-cell component must pass the real Wasm validator");
}

#[test]
fn sico_codegen_guest_emits_the_local_cell_reads_byte_exactly() {
    // STEP-0520, cell **read** half: the guest codegen emits the cell read as
    // well as the cell write. The measured shape writes the cell twice and
    // then reads it back twice:
    //
    //   let x = a
    //   set x = b
    //   return sico.text.concat(x, x)
    //
    // Its canonical IR carries two `write_local` and two `read_local`
    // operations (2868 bytes) and its production Component is 1138 bytes.
    let source = text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  let x = a\n  set x = b\n  return sico.text.concat(x, x)\nend function",
    );
    let ir = rust_ir(&source);
    assert_eq!(
        ir.matches("\"op\":\"read_local\"").count(),
        2,
        "the cell is read twice: {ir}"
    );
    let module = rust_module(&source);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the cell-read fixture");
    assert_eq!(
        expected.len(),
        1138,
        "the measured local-cell-read component size is pinned"
    );
    wasmparser::Validator::new()
        .validate_all(&expected)
        .expect("the measured local-cell-read component must pass the real Wasm validator");
    let RunOutcome::Output(output) =
        run_guest_with_args(&source, vec!["--emit-component-hex".into()])
    else {
        panic!("the local-cell-read component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted local-cell-read component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted local-cell-read component must pass the real Wasm validator");
}

#[test]
fn sico_codegen_guest_emits_the_i64_local_cell_byte_exactly() {
    // STEP-0520, cell **type** half: the cell support is not Text-only. A cell
    // whose declared type is a 64-bit integer is one flat slot, and the
    // backend's own layout gives it the same width the guest derives.
    //
    // The measured truth: canonical IR 2837 bytes, production Component 1116
    // bytes.
    let source = text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  let n = I64.literal(7)\n  set n = I64.literal(8)\n  return sico.text.concat(a, b)\nend function",
    );
    let module = rust_module(&source);
    let expected = codegen_oracle::compile_script_program(&module)
        .expect("the Rust backend emits the Script-profile Component for the i64-cell fixture");
    assert_eq!(
        expected.len(),
        1116,
        "the measured i64-cell component size is pinned"
    );
    wasmparser::Validator::new()
        .validate_all(&expected)
        .expect("the measured i64-cell component must pass the real Wasm validator");
    let RunOutcome::Output(output) =
        run_guest_with_args(&source, vec!["--emit-component-hex".into()])
    else {
        panic!("the i64-cell component must be emitted through the guest")
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let actual = decode_hex(&output.stdout);
    assert_eq!(
        actual, expected,
        "the guest-emitted i64-cell component must equal the Rust backend exactly"
    );
    wasmparser::Validator::new()
        .validate_all(&actual)
        .expect("the guest-emitted i64-cell component must pass the real Wasm validator");
}

#[test]
fn sico_codegen_guest_refuses_the_local_cell_types_outside_the_reached_subset() {
    // STEP-0520, fail-closed half (ADR-0016 decision 6): a cell whose type is
    // outside the reached subset stays a **typed refusal** -- the guest never
    // emits approximate bytes for it.
    //
    // The measured boundary: the guest frontend lowers a `Bytes` cell and a
    // `List[Text]` cell neither (the frontend refuses them with
    // `ERR:E-SH-IR-EXPRESSION`), and the component emission refuses them with
    // the typed "unsupported codegen source shape".
    //
    // The codegen's own cell subset is wider than that (bool, i64, u64, string
    // and bytes cells all have a declared flat layout); the narrower boundary
    // here is the **frontend's**, and this pin holds that boundary where it is
    // measured -- at the refusal, not at an approximation.
    for (label, body) in [
        (
            "bytes-cell",
            "function join(input: ScriptInput, b: Text) returns Text:\n  let x = input.stdin\n  set x = input.stdin\n  return b\nend function",
        ),
        (
            "list-cell",
            "function join(input: ScriptInput, b: Text) returns Text:\n  let x = input.arguments\n  set x = input.arguments\n  return b\nend function",
        ),
    ] {
        let source = text_concat_fixture(body);
        let RunOutcome::Domain { code, message } = run_guest(&source) else {
            panic!("the {label} shape must stay a typed refusal")
        };
        assert_eq!(code, "invalid-input", "{label}");
        assert_eq!(message, "ERR:E-SH-IR-EXPRESSION", "{label}");
        assert_eq!(
            run_guest_with_args(&source, vec!["--emit-component-hex".into()]),
            RunOutcome::Domain {
                code: "invalid-input".into(),
                message: "unsupported codegen source shape".into(),
            },
            "the {label} shape must not emit approximate component bytes"
        );
    }
}

#[test]
fn sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly() {
    // STEP-0520: the guest's own new cell support has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the guest's
    // own emitter, otherwise the bootstrap could never consume its own code.
    // Exactly like the STEP-0507 / STEP-0509 / STEP-0513 / STEP-0519 slices
    // established for the functions they added.
    //
    // Eight functions are new (the unit-type test, the cell flat layout, the
    // cell type/width/slot helpers and the two cell operation emitters) and
    // four are reshaped (the local declaration, the value slot, the dispatcher
    // slot and the operation router).
    for name in [
        "cgs_is_unit_ty",
        "cgs_cell_flat_hex",
        "cgs_cell_ty",
        "cgs_cell_width",
        "cgs_cell_slots",
        "cgs_cell_slot",
        "cgs_write_local_hex",
        "cgs_read_local_hex",
        "cgs_function_local_hex",
        "cgs_value_slot",
        "cgs_dispatcher_slot",
        "cgs_op_hex",
    ] {
        let source = codegen_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("codegen function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// STEP-0521: the T1 family truth table and the support-surface registration.
//
// Every slice so far pinned one shape of the `sico.text.concat` family. The
// family itself was never measured as a table, so "what does the guest
// support" was an accumulation of per-slice claims. The table below measures
// twenty shapes on four axes -- the Rust frontend's canonical IR, the Rust
// backend's production component, the guest frontend's verdict and the guest
// emitter's verdict -- and pins the result.
//
// The pins keep the refusals honest: a shape the guest does not support must
// stay a typed refusal on both sides, never an approximate component.
// ---------------------------------------------------------------------------

/// The guest's measured verdict on one family shape.
#[derive(Debug, Eq, PartialEq)]
enum GuestVerdict {
    /// The guest lowers the shape byte for byte and emits the component byte
    /// for byte.
    Exact,
    /// The guest frontend refuses the shape with the typed identity.
    FrontendRefused,
    /// The guest frontend lowers the shape byte for byte, but the component
    /// emission refuses it with the typed identity.
    CodegenRefused,
}

#[test]
fn sico_codegen_guest_pins_the_t1_family_truth_table() {
    // STEP-0521: the family truth table of `sico.text.concat`.
    //
    // Each row is measured from the Rust frontend and the Rust backend (never
    // designed); the guest side is asserted against the same truth -- byte
    // for byte where the guest supports the shape, and by its typed refusal
    // identity where it does not.
    //
    // The rows that are refused are the point of the table: they are the
    // shapes the support-surface registration has to carry (D15), and they
    // are the shapes that keep the family from closing.
    let rows: [(&str, &str, usize, usize, GuestVerdict); 24] = [
        (
            "let-binding-parameters",
            "function join(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, b)\n  return c\nend function",
            2295,
            1094,
            GuestVerdict::Exact,
        ),
        (
            "let-binding-cells",
            "function join(a: Text, b: Text) returns Text:\n  let x = a\n  let y = b\n  let c = sico.text.concat(x, y)\n  return c\nend function",
            2295,
            1094,
            GuestVerdict::Exact,
        ),
        (
            "return-position-with-a-statement",
            "function join(a: Text, b: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, b)\nend function",
            2295,
            1094,
            GuestVerdict::Exact,
        ),
        (
            "return-position-only",
            "function join(a: Text, b: Text) returns Text:\n  return sico.text.concat(a, b)\nend function",
            2295,
            1094,
            GuestVerdict::Exact,
        ),
        (
            "assigned-cell-write",
            "function join(a: Text, b: Text) returns Text:\n  let x = a\n  set x = b\n  return sico.text.concat(a, b)\nend function",
            2630,
            1114,
            GuestVerdict::Exact,
        ),
        (
            "assigned-cell-read",
            "function join(a: Text, b: Text) returns Text:\n  let x = a\n  set x = b\n  return sico.text.concat(x, x)\nend function",
            2868,
            1138,
            GuestVerdict::Exact,
        ),
        (
            "i64-cell",
            "function join(a: Text, b: Text) returns Text:\n  let n = I64.literal(7)\n  set n = I64.literal(8)\n  return sico.text.concat(a, b)\nend function",
            2837,
            1116,
            GuestVerdict::Exact,
        ),
        (
            "literal-operands",
            "function join() returns Text:\n  return sico.text.concat(\"a\", \"b\")\nend function",
            2372,
            1124,
            GuestVerdict::Exact,
        ),
        (
            "mixed-operand-pair",
            "function join(a: Text) returns Text:\n  return sico.text.concat(a, \"\\t\")\nend function",
            2334,
            1113,
            GuestVerdict::Exact,
        ),
        (
            "nested-host-call",
            "function join(a: Text, b: Text) returns Text:\n  return sico.text.concat(a, sico.text.concat(b, a))\nend function",
            2447,
            1112,
            GuestVerdict::Exact,
        ),
        (
            "literal-operands-with-a-statement",
            "function join() returns Text:\n  let x = \"x\"\n  return sico.text.concat(\"a\", \"b\")\nend function",
            2485,
            1137,
            GuestVerdict::Exact,
        ),
        (
            "mixed-operand-pair-with-a-statement",
            "function join(a: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, \"\\t\")\nend function",
            2334,
            1113,
            GuestVerdict::Exact,
        ),
        (
            "nested-host-call-with-a-statement",
            "function join(a: Text, b: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, sico.text.concat(b, a))\nend function",
            2447,
            1112,
            GuestVerdict::Exact,
        ),
        (
            "mixed-operand-plain-literal-with-a-statement",
            "function join(a: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, \"x\")\nend function",
            2333,
            1113,
            GuestVerdict::Exact,
        ),
        (
            "literal-pair-with-a-tab",
            "function join() returns Text:\n  let x = \"x\"\n  return sico.text.concat(\"a\", \"\\t\")\nend function",
            2486,
            1137,
            GuestVerdict::Exact,
        ),
        (
            "mixed-operand-newline-literal-with-a-statement",
            "function join(a: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, \"\\n\")\nend function",
            2334,
            1113,
            GuestVerdict::Exact,
        ),
        (
            "mixed-operand-tab-literal-without-a-statement",
            "function join(a: Text) returns Text:\n  return sico.text.concat(a, \"\\t\")\nend function",
            2334,
            1113,
            GuestVerdict::Exact,
        ),
        (
            "literal-in-a-later-function-only",
            "function first(a: Text) returns Text:\n  return a\nend function\nfunction second(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, \"B\")\n  return c\nend function",
            2721,
            1155,
            GuestVerdict::Exact,
        ),
        (
            "user-call-operand-second",
            "function join(slot: U64) returns Text:\n  return sico.text.concat(\"20\", helper(slot))\nend function\nfunction helper(x: U64) returns Text:\n  return \"z\"\nend function",
            2887,
            1177,
            GuestVerdict::Exact,
        ),
        (
            "user-call-operand-first",
            "function join(slot: U64) returns Text:\n  return sico.text.concat(helper(slot), \"20\")\nend function\nfunction helper(x: U64) returns Text:\n  return \"z\"\nend function",
            2887,
            1177,
            GuestVerdict::Exact,
        ),
        (
            "user-call-operand-nested",
            "function join(slot: U64) returns Text:\n  return sico.text.concat(\"20\", sico.text.concat(helper(slot), \"x\"))\nend function\nfunction helper(x: U64) returns Text:\n  return \"z\"\nend function",
            3152,
            1208,
            GuestVerdict::Exact,
        ),
        (
            "user-call-operand-chain",
            "function join(slot: U64) returns Text:\n  return sico.text.concat(\"20\", helper(slot))\nend function\nfunction helper(x: U64) returns Text:\n  return sico.text.concat(\"21\", helper2(x))\nend function\nfunction helper2(y: U64) returns Text:\n  return \"z\"\nend function",
            3594,
            1260,
            GuestVerdict::Exact,
        ),
        (
            "user-call-operand-two-arguments",
            "function join(a: Text, b: U64) returns Text:\n  return sico.text.concat(helper(a, b), \"20\")\nend function\nfunction helper(x: Text, y: U64) returns Text:\n  return \"z\"\nend function",
            3036,
            1185,
            GuestVerdict::FrontendRefused,
        ),
        (
            "concat-inside-an-if-block",
            "function join(a: Text) returns Text:\n  if U64.equal(U64.literal(1), U64.literal(1)):\n    return sico.text.concat(\"a\", a)\n  end if\n  return \"\"\nend function",
            3119,
            1198,
            GuestVerdict::CodegenRefused,
        ),
    ];
    for (label, body, ir_bytes, component_bytes, verdict) in rows {
        let source = text_concat_fixture(body);
        let module = rust_module(&source);
        let expected_ir = rust_ir(&source);
        assert_eq!(
            expected_ir.len(),
            ir_bytes,
            "the measured canonical IR size is pinned ({label})"
        );
        let expected_component = codegen_oracle::compile_script_program(&module)
            .expect("the Rust backend emits the Script-profile Component for the family fixture");
        assert_eq!(
            expected_component.len(),
            component_bytes,
            "the measured component size is pinned ({label})"
        );
        wasmparser::Validator::new()
            .validate_all(&expected_component)
            .expect("the measured component must pass the real Wasm validator");
        match verdict {
            GuestVerdict::FrontendRefused => {
                let RunOutcome::Domain { code, message } = run_guest(&source) else {
                    panic!("the guest must refuse the {label} shape with a typed identity")
                };
                assert_eq!(code, "invalid-input", "{label}");
                assert_eq!(message, "ERR:E-SH-IR-EXPRESSION", "{label}");
                let RunOutcome::Domain { code, message } =
                    run_guest_with_args(&source, vec!["--emit-component-hex".into()])
                else {
                    panic!("the guest must not emit a component for the {label} shape")
                };
                assert_eq!(code, "invalid-input", "{label}");
                assert_eq!(message, "unsupported codegen source shape", "{label}");
            }
            GuestVerdict::Exact => {
                let RunOutcome::Output(output) = run_guest(&source) else {
                    panic!("the guest must lower the {label} shape")
                };
                assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
                assert_bytes_equal(&output.stdout, expected_ir.as_bytes());
                let RunOutcome::Output(output) =
                    run_guest_with_args(&source, vec!["--emit-component-hex".into()])
                else {
                    panic!("the guest must emit the {label} component")
                };
                assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
                let actual = decode_hex(&output.stdout);
                assert_eq!(
                    actual, expected_component,
                    "the guest-emitted {label} component must equal the Rust backend exactly"
                );
                wasmparser::Validator::new()
                    .validate_all(&actual)
                    .expect("the guest-emitted component must pass the real Wasm validator");
            }
            GuestVerdict::CodegenRefused => {
                // STEP-0523: the frontend lowers the shape byte for byte, but the
                // component emission refuses it.  That is the state of every
                // shape that needs a **branch** in the IR: the guest's emitter
                // only knows a single straight-line block, so the refusal is
                // about the IR's control flow, not about the T1 family.
                let RunOutcome::Output(output) = run_guest(&source) else {
                    panic!("the guest frontend must lower the {label} shape")
                };
                assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
                assert_bytes_equal(&output.stdout, expected_ir.as_bytes());
                let RunOutcome::Domain { code, message } =
                    run_guest_with_args(&source, vec!["--emit-component-hex".into()])
                else {
                    panic!("the guest must refuse the {label} component with a typed identity")
                };
                assert_eq!(code, "invalid-input", "{label}");
                assert_eq!(message, "unsupported codegen source shape", "{label}");
            }
        }
    }
}

#[test]
fn sico_codegen_guest_derives_the_module_wide_data_offsets() {
    // STEP-0521: the static data segment is a **module-wide** property -- the
    // backend collects every string literal of the program, in function order,
    // into one segment, and a literal that repeats an earlier one shares that
    // earlier offset.
    //
    // The guest used to derive the offset from the *current* function's
    // instructions only. A module whose earlier function also holds a literal
    // therefore got a too-small offset, and the guest emitted a component of
    // the **same length as the oracle but with a different byte** -- the data
    // pointer. That is a silent divergence, the one outcome the fail-closed
    // discipline forbids, so it is pinned here on the corrected behaviour.
    //
    // The repeated-literal shape is the second half of the same correction:
    // the backend keeps the first occurrence's bytes, so the repeated literal
    // resolves to that first offset instead of refusing the program.
    for (label, body, component_bytes) in [
        (
            "two-functions-one-literal-each",
            "function first(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, \"A\")\n  return c\nend function\nfunction second(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, \"B\")\n  return c\nend function",
            1188,
        ),
        (
            "repeated-literal-in-one-function",
            "function join(a: Text, b: Text) returns Text:\n  let c = sico.text.concat(a, \"A\")\n  let d = sico.text.concat(b, \"A\")\n  return d\nend function",
            1145,
        ),
    ] {
        let source = text_concat_fixture(body);
        let module = rust_module(&source);
        let expected = codegen_oracle::compile_script_program(&module)
            .expect("the Rust backend emits the Script-profile Component for the data fixture");
        assert_eq!(
            expected.len(),
            component_bytes,
            "the measured component size is pinned ({label})"
        );
        wasmparser::Validator::new()
            .validate_all(&expected)
            .expect("the measured component must pass the real Wasm validator");
        let RunOutcome::Output(output) =
            run_guest_with_args(&source, vec!["--emit-component-hex".into()])
        else {
            panic!("the guest must emit the {label} component")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        let actual = decode_hex(&output.stdout);
        assert_eq!(
            actual, expected,
            "the guest-emitted {label} component must equal the Rust backend exactly"
        );
        wasmparser::Validator::new()
            .validate_all(&actual)
            .expect("the guest-emitted component must pass the real Wasm validator");
    }
}

#[test]
fn sico_compiler_lowers_the_step0521_codegen_functions_byte_exactly() {
    // STEP-0521: the guest's own reshaped data-segment code has to stay inside
    // the self-hosting shape frontier -- the guest frontend must lower the
    // guest's own emitter, otherwise the bootstrap could never consume its own
    // code. Exactly like the STEP-0507 / STEP-0509 / STEP-0513 / STEP-0519 /
    // STEP-0520 slices established for the functions they added or reshaped.
    //
    // Both functions were reshaped in this slice: `cgs_data_offset` walks the
    // whole module instead of one function, and `cgs_const_string_hex` lost
    // its nested host call. The nested host call is what kept them outside
    // the frontier before (`ERR:E-SH-IR-CALL-SHAPE`): splitting it into two
    // statements -- the shape STEP-0515 already used for the same reason --
    // brought both of them back in.
    for name in ["cgs_data_offset", "cgs_const_string_hex"] {
        let source = codegen_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("codegen function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

/// STEP-0522: the core module a Script-profile Component wraps.
fn component_core_module(component: &[u8]) -> Vec<u8> {
    for payload in wasmparser::Parser::new(0).parse_all(component) {
        if let wasmparser::Payload::ModuleSection {
            unchecked_range, ..
        } = payload.expect("the component parses")
        {
            return component[unchecked_range.start..unchecked_range.end].to_vec();
        }
    }
    panic!("the Component must wrap one core module");
}

#[test]
fn sico_codegen_guest_emits_the_escaped_literal_component_byte_exactly() {
    // STEP-0522, G-b: a string literal that carries a JSON escape.
    //
    // The backend's static data segment holds the literal's **decoded** bytes:
    // the source escape `\t` is one TAB byte, not the two characters `\t`. The
    // guest used to refuse every literal with a backslash
    // (`ERR:E-SH-CG-IR-EMIT`), because it decoded no escapes at all -- so a
    // program with an escaped literal could not be compiled by the guest at
    // all, and the 952 escaped-literal call sites of the frozen source set S
    // were a hard block (G-b).
    //
    // The guest resolves the escapes now: its IR keeps the literal in the JSON
    // form, the backend's segment keeps the decoded bytes, so the whole
    // component matches the Rust backend byte for byte.
    //
    // The empty literal (G-e) is the second half of the same correction: it
    // carries no bytes, so it points at the offset the next literal starts at
    // and its length is zero -- the backend's `script_data` gives it the same
    // treatment, so the segment and the pointer agree.
    for (label, body, component_bytes) in [
        (
            "tab-and-newline",
            r#"function first(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "tab\there")
  return c
end function
function second(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "nl\nhere")
  return c
end function"#,
            1201,
        ),
        (
            "quote-backslash-carriage-return",
            r#"function quoted(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "say \"hi\"")
  return c
end function
function slashed(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "c:\\dir")
  return c
end function
function carriage(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "one\rtwo")
  return c
end function"#,
            1279,
        ),
        (
            "nul-escape",
            r#"function nul(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "nul\0byte")
  return c
end function"#,
            1122,
        ),
        (
            "empty-literal",
            r#"function empty(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "")
  return c
end function
function tail(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "tail")
  return c
end function"#,
            1190,
        ),
    ] {
        let source = text_concat_fixture(body);
        let module = rust_module(&source);
        let expected = codegen_oracle::compile_script_program(&module)
            .expect("the Rust backend emits the Script-profile Component for the literal fixture");
        assert_eq!(
            expected.len(),
            component_bytes,
            "the measured component size is pinned ({label})"
        );
        wasmparser::Validator::new()
            .validate_all(&expected)
            .expect("the measured component must pass the real Wasm validator");
        let RunOutcome::Output(output) =
            run_guest_with_args(&source, vec!["--emit-component-hex".into()])
        else {
            panic!("the guest must emit the {label} component")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        let actual = decode_hex(&output.stdout);
        assert_eq!(
            actual, expected,
            "the guest-emitted {label} component must equal the Rust backend exactly"
        );
        wasmparser::Validator::new()
            .validate_all(&actual)
            .expect("the guest-emitted component must pass the real Wasm validator");
    }
}

#[test]
fn sico_codegen_guest_derives_the_decoded_literal_bytes() {
    // STEP-0522: the static data segment carries the literal's **decoded**
    // bytes. This is the truth the guest has to reproduce, and it is measured
    // from the Rust backend's production output: the backend's `script_data`
    // collects `value.as_bytes()` -- the decoded string -- and keys the data
    // offsets by those bytes.
    //
    // The bytes are not designed: `\t` is one TAB byte (09), `\n` one LF byte
    // (0a), `\r` one CR byte (0d), `\0` one NUL byte (00), `\"` one quote byte
    // (22) and `\\` one backslash byte (5c). The guest used to spell the
    // literal out in the IR's JSON form and refused to decode it, so the
    // segment it derived would have carried the two characters `\t` instead of
    // the one byte the backend puts there.
    for (label, body, decoded) in [
        (
            "tab",
            r#"function join(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "a\tb")
  return c
end function"#,
            &b"a\x09b"[..],
        ),
        (
            "newline",
            r#"function join(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "a\nb")
  return c
end function"#,
            &b"a\x0ab"[..],
        ),
        (
            "carriage-return",
            r#"function join(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "a\rb")
  return c
end function"#,
            &b"a\x0db"[..],
        ),
        (
            "nul",
            r#"function join(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "a\0b")
  return c
end function"#,
            &b"a\x00b"[..],
        ),
        (
            "quote",
            r#"function join(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "a\"b")
  return c
end function"#,
            &b"a\"b"[..],
        ),
        (
            "backslash",
            r#"function join(a: Text, b: Text) returns Text:
  let c = sico.text.concat(a, "a\\b")
  return c
end function"#,
            &b"a\\b"[..],
        ),
    ] {
        let source = text_concat_fixture(body);
        let module = rust_module(&source);
        let expected = codegen_oracle::compile_script_program(&module)
            .expect("the Rust backend emits the Script-profile Component for the literal fixture");
        let core = component_core_module(&expected);
        assert!(
            core.windows(decoded.len()).any(|window| window == decoded),
            "the backend's static data segment carries the decoded literal bytes ({label})"
        );
        let RunOutcome::Output(output) =
            run_guest_with_args(&source, vec!["--emit-component-hex".into()])
        else {
            panic!("the guest must emit the {label} component")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        let actual = decode_hex(&output.stdout);
        let guest_core = component_core_module(&actual);
        assert!(
            guest_core
                .windows(decoded.len())
                .any(|window| window == decoded),
            "the guest's static data segment carries the decoded literal bytes ({label})"
        );
        assert_eq!(
            actual, expected,
            "the guest-emitted {label} component must equal the Rust backend exactly"
        );
    }
}

#[test]
fn sico_codegen_guest_refuses_the_unknown_escape_with_a_typed_identity() {
    // STEP-0522: the decoder accepts exactly the escapes the IR producer can
    // write -- `\"`, `\\`, `\n`, `\r`, `\t` and `\u0000` (the JSON escape the
    // producer writes for the source `\0`). An escape it does not accept is
    // never an approximate byte: the fail-closed discipline.
    //
    // Measured: **both** frontends refuse the unknown escape -- the Rust lexer
    // reports `UnknownStringEscape`, and the guest's own lexer reports
    // `ERR:E-SH-IR-STRING-ESCAPE`. The guest decoder's own refusal
    // (`ERR:E-SH-CG-IR-EMIT`) is therefore a second line of defence that the
    // frontend never lets through; the shape never reaches the Rust backend.
    let source = text_concat_fixture(
        "function join(a: Text, b: Text) returns Text:\n  let x = a\n  return sico.text.concat(a, \"a\\qb\")\nend function",
    );
    let fixture = SourceFile::from_text(SourceId::new(0), "selfhost-input.sico", &source)
        .expect("fixture source is bounded");
    assert!(
        lower_core(&fixture).is_err(),
        "the Rust frontend refuses the unknown escape at the lexer level"
    );
    let RunOutcome::Domain { code, message } = run_guest(&source) else {
        panic!("the guest frontend must refuse the unknown escape with a typed identity")
    };
    assert_eq!(code, "invalid-input");
    assert_eq!(message, "ERR:E-SH-IR-STRING-ESCAPE");
    let RunOutcome::Domain { code, message } =
        run_guest_with_args(&source, vec!["--emit-component-hex".into()])
    else {
        panic!("the guest must refuse the unknown escape with a typed identity")
    };
    assert_eq!(code, "invalid-input");
    // The component path wraps the frontend's refusal in the compiler's own
    // identity, exactly like every other refused shape.
    assert_eq!(message, "unsupported codegen source shape");
}

#[test]
fn sico_compiler_lowers_the_step0522_codegen_functions_byte_exactly() {
    // STEP-0522, G-b: the guest's own literal code has to stay inside the
    // self-hosting shape frontier -- the guest frontend must lower the guest's
    // own emitter, otherwise the bootstrap could never consume its own code.
    // Exactly like the STEP-0507 / STEP-0509 / STEP-0513 / STEP-0519 /
    // STEP-0520 / STEP-0521 slices established for the functions they added or
    // reshaped.
    //
    // The literal code is where the frontier was tightest: `cgs_literal_text`
    // -- the function that refused every escaped literal -- reported
    // `ERR:E-SH-IR-CALL-ARITY` and stayed **outside** the frontier. The
    // replacement works in the byte domain (the literal's identity is its
    // decoded bytes, spelled as hex byte pairs), so it needs no text-shaped
    // host call at all, and all eight functions of this slice lower.
    for name in [
        "cgs_is_const_string",
        "cgs_literal_hex",
        "cgj_string_hex",
        "cgj_escape_hex",
        "cgs_hex_bytes",
        "cgs_data_offset",
        "cgs_const_string_hex",
        "cgs_data_hex",
    ] {
        let source = codegen_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("codegen function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}

/// Every function name `parser.sico` declares.
fn parser_function_names() -> Vec<&'static str> {
    PARSER_SOURCE
        .lines()
        .filter_map(|line| line.strip_prefix("function "))
        .filter_map(|rest| rest.split('(').next())
        .map(|name| name.trim())
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .collect()
}

/// The parameter list and the return type of a `parser.sico` function.
fn parser_signature(name: &str) -> (String, String) {
    let head = parser_function_text(name)
        .lines()
        .next()
        .expect("the function has a signature line")
        .to_owned();
    let open = head.find('(').expect("the signature has parameters");
    let close = head
        .rfind(')')
        .expect("the signature closes its parameters");
    let params = head[open + 1..close].trim().to_owned();
    let returns = head[close + 1..]
        .trim()
        .trim_start_matches("returns ")
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_owned();
    (params, returns)
}

/// A stub for one `parser.sico` function: the real signature, a trivial body.
fn parser_stub(name: &str) -> String {
    let (params, returns) = parser_signature(name);
    let body = match returns.as_str() {
        "U64" => "U64.literal(0)",
        "I64" => "I64.literal(0)",
        "Bool" => "false",
        "Text" => "\"\"",
        "Bytes" => "sico.text.encode(\"\")",
        other => panic!("parser function {name} has an unexpected return type {other}"),
    };
    format!("function {name}({params}) returns {returns}:\n  return {body}\nend function\n")
}

/// The `parser.sico` functions the text calls, in first-call order.
fn parser_callees(text: &str, own: &str) -> Vec<String> {
    let names = parser_function_names();
    let chars: Vec<char> = text.chars().collect();
    let mut callees: Vec<String> = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].is_ascii_alphabetic() && chars[index] != '_' {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && (chars[index].is_ascii_alphanumeric() || chars[index] == '_') {
            index += 1;
        }
        let word: String = chars[start..index].iter().collect();
        let member = start > 0 && chars[start - 1] == '.';
        let mut after = index;
        while after < chars.len() && chars[after] == ' ' {
            after += 1;
        }
        let called = after < chars.len() && chars[after] == '(';
        if called
            && !member
            && word != own
            && names.contains(&word.as_str())
            && !callees.contains(&word)
        {
            callees.push(word);
        }
    }
    callees
}

/// A `parser.sico` function with a stub for every function it calls.
fn parser_function_source(name: &str) -> String {
    let text = parser_function_text(name);
    let mut source = String::new();
    for callee in parser_callees(&text, name) {
        source.push_str(&parser_stub(&callee));
    }
    source.push_str(&text);
    source
}

/// The transitive callee closure of one `parser.sico` function, emitted in
/// **file order** and with the real function texts.
///
/// STEP-0518 kept this list by hand. STEP-0523 reshaped the route it
/// describes -- the route now calls `straight_text_concat_packed` and
/// `gw_decimal_value` -- so a hand-maintained list drifts out of the closure
/// the moment the route changes. The closure is computed from the file
/// instead, which is what the list always meant.
fn parser_closure_source(name: &str) -> String {
    let mut wanted: Vec<String> = Vec::new();
    let mut queue: Vec<String> = vec![name.to_owned()];
    while let Some(current) = queue.pop() {
        if wanted.contains(&current) {
            continue;
        }
        let text = parser_function_text(&current);
        for callee in parser_callees(&text, &current) {
            if !wanted.contains(&callee) {
                queue.push(callee);
            }
        }
        wanted.push(current);
    }
    let mut ordered: Vec<(usize, String)> = wanted
        .iter()
        .map(|item| {
            (
                PARSER_SOURCE
                    .find(&format!("function {item}("))
                    .expect("the closure member is in the file"),
                item.clone(),
            )
        })
        .collect();
    ordered.sort();
    ordered
        .iter()
        .map(|(_, item)| parser_function_text(item))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn sico_codegen_guest_lowers_the_gd_single_return_shapes_byte_exactly() {
    // STEP-0523, G-d: a function body whose **only** statement is a single
    // `return` whose expression is a `sico.text.concat` call.
    //
    // The route that owns that shape (`direct_text_concat_return_ir`) used to
    // accept **two `Text` parameters only**.  Every other operand shape -- a
    // literal, a nested concat, a call to another function -- was refused with
    // `ERR:E-SH-IR-EXPRESSION`, even though the Rust backend lowers all of
    // them.  The frozen set S contains six such call sites, so the gap was a
    // hard blocker for the family (D10: no downgrade).
    //
    // The shapes below are the ones S actually contains, spelled out:
    //   * `cg_post_body_hex`  -- a literal beside a **nested** concat whose
    //     first operand is a call to another function,
    //   * `cgs_local_get_hex` -- a literal beside a call to another function,
    //   * `cgs_zero_hex`      -- the same call inside an `if` block,
    //   * `declaration_name`  -- a call with **two** arguments inside an `if`
    //     block, and a nested concat beside it.
    //
    // The first two are the ones this slice closed.  The last two are still
    // refused -- but not by the frontend: the frontend lowers them byte for
    // byte, and the **component emission** refuses them, because the guest's
    // emitter only knows a single straight-line block.  That is a branch gap
    // in the emitter, not a T1 family gap, and it is pinned here so the
    // register can name it exactly.
    let closed: [(&str, &str, usize, usize); 2] = [
        (
            "s-post-body-hex",
            "function join(arena_base: U64) returns Text:\n  return sico.text.concat(\"00\", sico.text.concat(helper(arena_base), \"240041ffffffff072401410024020b\"))\nend function\nfunction helper(base: U64) returns Text:\n  return \"z\"\nend function",
            3190,
            1237,
        ),
        (
            "s-local-get-hex",
            "function join(slot: U64) returns Text:\n  return sico.text.concat(\"20\", helper(slot))\nend function\nfunction helper(x: U64) returns Text:\n  return \"z\"\nend function",
            2887,
            1177,
        ),
    ];
    for (label, body, ir_bytes, component_bytes) in closed {
        let source = text_concat_fixture(body);
        let module = rust_module(&source);
        let expected_ir = rust_ir(&source);
        assert_eq!(
            expected_ir.len(),
            ir_bytes,
            "the measured canonical IR size is pinned ({label})"
        );
        let expected_component = codegen_oracle::compile_script_program(&module)
            .expect("the Rust backend emits the Script-profile Component for the G-d fixture");
        assert_eq!(
            expected_component.len(),
            component_bytes,
            "the measured component size is pinned ({label})"
        );
        wasmparser::Validator::new()
            .validate_all(&expected_component)
            .expect("the measured component must pass the real Wasm validator");
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("the guest must lower the {label} shape")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected_ir.as_bytes());
        let RunOutcome::Output(output) =
            run_guest_with_args(&source, vec!["--emit-component-hex".into()])
        else {
            panic!("the guest must emit the {label} component")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        assert_eq!(
            decode_hex(&output.stdout),
            expected_component,
            "the guest-emitted {label} component must equal the Rust backend exactly"
        );
    }
    let branched: [(&str, &str, usize, usize); 2] = [
        (
            "s-zero-hex",
            "function join(kind: Text, slot: U64) returns Text:\n  if U64.equal(U64.literal(1), U64.literal(1)):\n    return sico.text.concat(\"4100\", helper(slot))\n  end if\n  return \"\"\nend function\nfunction helper(x: U64) returns Text:\n  return \"z\"\nend function",
            3753,
            1266,
        ),
        (
            "s-declaration-name",
            "function join(kind: Text, items: List[Text]) returns Text:\n  if U64.equal(U64.literal(1), U64.literal(1)):\n    return sico.text.concat(helper(items, U64.literal(1)), sico.text.concat(\".\", helper(items, U64.literal(2))))\n  end if\n  return \"\"\nend function\nfunction helper(items: List[Text], index: U64) returns Text:\n  return \"z\"\nend function",
            4387,
            1316,
        ),
    ];
    for (label, body, ir_bytes, component_bytes) in branched {
        let source = text_concat_fixture(body);
        let module = rust_module(&source);
        let expected_ir = rust_ir(&source);
        assert_eq!(
            expected_ir.len(),
            ir_bytes,
            "the measured canonical IR size is pinned ({label})"
        );
        let expected_component = codegen_oracle::compile_script_program(&module)
            .expect("the Rust backend emits the Script-profile Component for the G-d fixture");
        assert_eq!(
            expected_component.len(),
            component_bytes,
            "the measured component size is pinned ({label})"
        );
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("the guest frontend must lower the {label} shape")
        };
        assert_eq!(output.exit_code, 0, "{label}: {:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected_ir.as_bytes());
        let RunOutcome::Domain { code, message } =
            run_guest_with_args(&source, vec!["--emit-component-hex".into()])
        else {
            panic!("the guest must refuse the {label} component with a typed identity")
        };
        assert_eq!(code, "invalid-input", "{label}");
        assert_eq!(message, "unsupported codegen source shape", "{label}");
    }
}

#[test]
fn sico_compiler_lowers_the_step0523_parser_functions_byte_exactly() {
    // STEP-0523, G-d: the route that owns the single-`return` concat shape
    // (`direct_text_concat_return_ir`) was reshaped, and the guest frontend
    // has to keep lowering it -- otherwise the bootstrap could never consume
    // its own frontend code.  This is the same self-hosting shape frontier the
    // STEP-0507 / STEP-0509 / STEP-0513 / STEP-0519 / STEP-0520 / STEP-0521 /
    // STEP-0522 slices established for the functions they added or reshaped.
    //
    // The reshape moved the operand lowering into `straight_text_concat_packed`
    // -- the helper the `let`-binding routes already use -- so the route no
    // longer carries its own copy of the operand rules.  The frontier did not
    // move: the route lowers, and so does the helper it now calls.
    for name in [
        "direct_text_concat_return_ir",
        "straight_text_concat_packed",
    ] {
        let source = parser_function_source(name);
        let expected = rust_ir(&source);
        let RunOutcome::Output(output) = run_guest(&source) else {
            panic!("parser function {name} must compile through the guest frontend")
        };
        assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
        assert_bytes_equal(&output.stdout, expected.as_bytes());
    }
}
