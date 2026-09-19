//! M22 S6 (STEP-0219): the Sico-written frontend lowers checked
//! arithmetic, the frozen checked-Result payload match, and straight-line
//! fixed-width `let` SSA bindings, aliases, operands and call arguments byte-identically.

use std::sync::atomic::{AtomicU64, Ordering};

use sico_ir::{Module, canonical_json, lower_core, verify};
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits,
    ScriptInput,
};
use sico_source::{SourceFile, SourceId};

const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
const INPUT: &[u8] = b"function main() returns Int:\n  let xs = sico.list.empty()\n  let total = I64.literal(0)\n  for x in xs:\n    if I64.equal(total, I64.literal(0)):\n      set total = I64.literal(1)\n    else:\n      while I64.less_than(total, I64.literal(2)):\n        match I64.checked_add(total, I64.literal(1)):\n          case ok(next):\n            set total = next\n          case error(_):\n            return -1\n        end match\n      end while\n    end if\n  end for\n  return 0\nend function\n\nfunction add(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(I64.literal(1), b)\nend function\n\nfunction message() returns Text:\n  return \"ready, (nested-looking)\"\nend function\n";

fn compile_parser() -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!(
        "sico-step0201-parser-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("parser.sico");
    let component_path = directory.join("parser.component.wasm");
    std::fs::write(&source_path, PARSER_SOURCE).unwrap();
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
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
}

#[test]
fn sico_parser_emits_recursive_return_expression_trees() {
    let component = compile_parser();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("parser component links");
    match prepared
        .run(
            &ScriptInput {
                stdin: INPUT.to_vec(),
                ..ScriptInput::default()
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
    {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            // STEP-0200: exact signatures and statement inventory remain
            // stable. Return expressions preserve the recursive argument
            // tree, negative integers, and one escaped string token.
            assert_eq!(
                output.stdout,
                b"fn:main/0{for=1,while=1,if=1,match=1,let=2,set=2,return=2,expr=[int:-1,int:0]};fn:add/2{for=0,while=0,if=0,match=0,let=0,set=0,return=1,expr=[call:I64.checked_add(call:I64.literal(int:1),name:b)]};fn:message/0{for=0,while=0,if=0,match=0,let=0,set=0,return=1,expr=[string:\"ready, (nested-looking)\"]};".to_vec(),
                "parser summary drifted"
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

fn assert_ir_matches_rust(prepared: &PreparedProgram, source: &[u8]) {
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    let RunOutcome::Output(output) = outcome else {
        panic!("expected canonical IR output for {}, got {outcome:?}", String::from_utf8_lossy(source));
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let module: Module = serde_json::from_slice(&output.stdout).expect("IR JSON deserializes");
    assert!(verify(&module).is_empty(), "self-host IR must verify for {}: {:?}", String::from_utf8_lossy(&output.stdout), output.stdout);
    assert_eq!(
        canonical_json(&module).unwrap().as_bytes(),
        output.stdout,
        "self-host IR is not canonical JSON"
    );
    let source_text = std::str::from_utf8(source).unwrap();
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        source_text.to_owned(),
    )
    .unwrap();
    let rust_ir = canonical_json(&lower_core(&rust_source).unwrap()).unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        rust_ir,
        "Sico and Rust lowering must be byte-identical"
    );
}

#[test]
fn sico_lowering_emits_verifier_accepted_scalar_ir_and_refuses_noncanonical_input() {
    let component = compile_parser();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("parser component links");
    assert_ir_matches_rust(
        &prepared,
        b"function answer() returns Int:\n  return 7\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function identity(value: Int) returns Int:\n  return value\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function truth() returns Bool:\n  return true\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function greeting() returns Text:\n  return \"line\\n\\\"ok\\\"\"\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function nul() returns Text:\n  return \"a\\0b\"\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        "function unicode() returns Text:\n  return \"你好\"\nend function\n".as_bytes(),
    );
    assert_ir_matches_rust(
        &prepared,
        b"function signed_max() returns I64:\n  return I64.literal(9223372036854775807)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function signed_min() returns I64:\n  return I64.literal(-9223372036854775808)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function unsigned_max() returns U64:\n  return U64.literal(18446744073709551615)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function choose(count: Int, ready: Bool, label: Text, delta: I64, bits: U64) returns Text:\n  return label\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function swap(left: Int, right: Int) returns Int:\n  return swap(right, left)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function identity(value: Int) returns Int:\n  return value\nend function\n\nfunction main(value: Int) returns Int:\n  return identity(value)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function sub(left: Int, right: Int) returns Int:\n  return left\nend function\n\nfunction main(x: Int, y: Int) returns Int:\n  return sub(y, x)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(left: Int, right: Int) returns Int:\n  return left\nend function\n\nfunction main() returns Int:\n  return add(1, 22)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(left: Int, right: Int) returns Int:\n  return left\nend function\n\nfunction main(v: Int) returns Int:\n  return add(v, 7)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function flag(ready: Bool) returns Bool:\n  return ready\nend function\n\nfunction main() returns Bool:\n  return flag(true)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function label(text: Text) returns Text:\n  return text\nend function\n\nfunction main() returns Text:\n  return label(\"hi\\n\")\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function bits(a: I64, b: I64) returns I64:\n  return I64.bit_and(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function combine(a: I64, b: I64) returns I64:\n  return I64.bit_or(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function mix(a: I64, b: I64) returns I64:\n  return I64.bit_xor(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function raise(a: U64, b: U64) returns U64:\n  return U64.shl(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function lower(a: U64, b: U64) returns U64:\n  return U64.shr(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main(v: I64) returns I64:\n  return add(v, I64.literal(2))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main() returns I64:\n  return add(I64.literal(1), I64.literal(-5))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function bits(a: U64, b: U64) returns U64:\n  return a\nend function\n\nfunction main(v: U64) returns U64:\n  return bits(v, U64.literal(7))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(v: Int) returns Int:\n  return v\nend function\n\nfunction main(v: Int) returns Int:\n  return f(g(v))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(v: Int) returns Int:\n  return v\nend function\n\nfunction main() returns Int:\n  return f(g(1))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction main(v: Int) returns Int:\n  return f(v, g(v))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction main(v: Int) returns Int:\n  return f(g(v), g(v))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function shifts(a: U64) returns U64:\n  return U64.shl(a, U64.literal(3))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function mask(a: I64) returns I64:\n  return I64.bit_and(a, I64.literal(-1))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function seed(a: I64) returns I64:\n  return I64.bit_or(I64.literal(1), a)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function inc(a: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, I64.literal(1))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function mul(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_mul(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function div(a: U64, b: U64) returns Result[U64, NumericError]:\n  return U64.checked_div(a, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function make(a: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, I64.literal(1))\nend function\n\nfunction pass(a: I64) returns Result[I64, NumericError]:\n  return make(a)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add_or_zero(left: I64, right: I64) returns I64:\n  match I64.checked_add(left, right):\n    case ok(value):\n      return value\n    case error(reason):\n      return I64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function div_or_one(left: U64, right: U64) returns U64:\n  match U64.checked_div(left, right):\n    case ok(value):\n      return value\n    case error(reason):\n      return U64.literal(1)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function constants() returns U64:\n  let signed = I64.literal(7)\n  let result = U64.literal(9)\n  return result\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function mask() returns I64:\n  let left = I64.literal(7)\n  let right = I64.literal(3)\n  return I64.bit_and(left, right)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function checked() returns Result[U64, NumericError]:\n  let left = U64.literal(8)\n  return U64.checked_div(left, U64.literal(2))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction main() returns I64:\n  let left = I64.literal(5)\n  let right = I64.literal(6)\n  return pick(right, left)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction main() returns I64:\n  let left = I64.literal(5)\n  return pick(left, I64.literal(6))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function alias(value: I64) returns I64:\n  let copy = value\n  return copy\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function chain() returns I64:\n  let base = I64.literal(6)\n  let copy = base\n  return I64.bit_or(copy, I64.literal(1))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function mask(a: I64, b: I64) returns I64:\n  let masked = I64.bit_and(a, b)\n  return masked\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function kept(a: I64, b: I64) returns Result[I64, NumericError]:\n  let added = I64.checked_add(a, b)\n  return added\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main() returns I64:\n  let total = add(I64.literal(3), I64.literal(4))\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function chain(a: I64) returns I64:\n  let masked = I64.bit_and(a, I64.literal(7))\n  let copy = masked\n  return I64.bit_or(copy, I64.literal(1))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction main(v: I64) returns I64:\n  let other = I64.literal(9)\n  let got = pick(v, other)\n  return got\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function make(a: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, I64.literal(1))\nend function\n\nfunction pass(a: I64) returns Result[I64, NumericError]:\n  let got = make(a)\n  return got\nend function\n",
    );

    let refused = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function answer() returns Int:\n  return -0\nend function\n".to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        refused,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-NEGATIVE-BARE".to_owned(),
        },
        "invalid input must produce a typed refusal and no IR bytes"
    );

    let bad_escape = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function bad() returns Text:\n  return \"bad\\x\"\nend function\n"
                    .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        bad_escape,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-STRING-ESCAPE".to_owned(),
        },
        "invalid string escape must produce a typed refusal and no IR bytes"
    );

    for (source, expected) in [
        (
            b"function overflow() returns I64:\n  return I64.literal(9223372036854775808)\nend function\n"
                .as_slice(),
            "ERR:E-SH-IR-I64-RANGE",
        ),
        (
            b"function overflow() returns U64:\n  return U64.literal(18446744073709551616)\nend function\n"
                .as_slice(),
            "ERR:E-SH-IR-U64-RANGE",
        ),
    ] {
        let rust_source = SourceFile::from_text(
            SourceId::new(0),
            "selfhost-input.sico",
            std::str::from_utf8(source).unwrap().to_owned(),
        )
        .unwrap();
        assert!(
            lower_core(&rust_source).is_err(),
            "Rust lowering must refuse the same out-of-range literal"
        );
        let outcome = prepared
            .run(
                &ScriptInput {
                    arguments: vec!["--emit-ir".to_owned()],
                    stdin: source.to_vec(),
                },
                &RunnerLimits::default(),
                &CancelToken::new(),
            )
            .expect("input bounds hold");
        assert_eq!(
            outcome,
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: expected.to_owned(),
            },
            "out-of-range fixed-width literal must have a typed refusal"
        );
    }

    let unsupported_parameter = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin:
                    b"function bytes(value: Bytes) returns Bytes:\n  return value\nend function\n"
                        .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unsupported_parameter,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-PARAMETER-TYPE".to_owned(),
        },
        "the declared scalar subset must refuse wider parameter types"
    );

    let wrong_call_arity_source =
        b"function bounce(value: Int) returns Int:\n  return bounce(value, value)\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(wrong_call_arity_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must refuse recursive call arity mismatch"
    );
    let wrong_call_arity = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: wrong_call_arity_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        wrong_call_arity,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-ARITY".to_owned(),
        },
        "recursive call arity mismatch must have a typed refusal"
    );

    for (source, expected) in [
        (
            b"function main(value: Int) returns Int:\n  return ghost(value)\nend function\n"
                .as_slice(),
            "ERR:E-SH-IR-CALL-TARGET",
        ),
        (
            b"function one(value: Int) returns Int:\n  return value\nend function\n\nfunction main(value: Int) returns Int:\n  return one(value, value)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARITY",
        ),
        (
            b"function wide(amount: I64) returns I64:\n  return amount\nend function\n\nfunction main(value: Int) returns Int:\n  return wide(value)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function f(value: Int) returns Int:\n  return value\nend function\n\nfunction f(other: Int) returns Int:\n  return other\nend function\n".as_slice(),
            "ERR:E-SH-IR-FUNCTION-DUPLICATE",
        ),
        (
            b"function wide(amount: I64) returns I64:\n  return amount\nend function\n\nfunction main() returns I64:\n  return wide(1)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function one(value: Int) returns Int:\n  return value\nend function\n\nfunction main() returns Int:\n  return one(ghost)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARGUMENT",
        ),
        (
            b"function one(value: Int) returns Int:\n  return value\nend function\n\nfunction main() returns Int:\n  return one(01)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARGUMENT",
        ),
        (
            b"function bad(a: I64, b: Int) returns I64:\n  return I64.bit_and(a, b)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function extra(a: I64, b: I64) returns I64:\n  return I64.bit_and(a, b, a)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARITY",
        ),
        (
            b"function lonely(a: I64) returns I64:\n  return I64.bit_and(a, ghost)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARGUMENT",
        ),
        (
            b"function add(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main(v: I64) returns I64:\n  return add(v, I64.literal(9223372036854775808))\nend function\n".as_slice(),
            "ERR:E-SH-IR-I64-RANGE",
        ),
        (
            b"function add(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main(v: I64) returns I64:\n  return add(v, 3)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main(v: Int) returns Int:\n  return f(g(v), g(v))\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction main(v: Int) returns Int:\n  return f(ghost(v))\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TARGET",
        ),
        (
            b"function over(a: U64) returns U64:\n  return U64.shr(a, U64.literal(18446744073709551616))\nend function\n".as_slice(),
            "ERR:E-SH-IR-U64-RANGE",
        ),
        (
            b"function bad(a: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, U64.literal(1))\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function bad(a: I64) returns I64:\n  return I64.checked_add(a, I64.literal(1))\nend function\n".as_slice(),
            "ERR:E-SH-IR-TYPE-MISMATCH",
        ),
        (
            b"function lonely(a: I64) returns I64:\n  let masked = I64.bit_and(a, ghost)\n  return masked\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARGUMENT",
        ),
        (
            b"function main(v: I64) returns I64:\n  let got = ghost(v)\n  return got\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TARGET",
        ),
        (
            b"function bad(a: I64, b: I64) returns I64:\n  let added = I64.checked_add(a, b)\n  return added\nend function\n".as_slice(),
            "ERR:E-SH-IR-TYPE-MISMATCH",
        ),
    ] {
        let rust_source = SourceFile::from_text(
            SourceId::new(0),
            "selfhost-input.sico",
            std::str::from_utf8(source).unwrap().to_owned(),
        )
        .unwrap();
        assert!(
            lower_core(&rust_source).is_err(),
            "Rust lowering must refuse this cross-function violation"
        );
        let outcome = prepared
            .run(
                &ScriptInput {
                    arguments: vec!["--emit-ir".to_owned()],
                    stdin: source.to_vec(),
                },
                &RunnerLimits::default(),
                &CancelToken::new(),
            )
            .expect("input bounds hold");
        assert_eq!(
            outcome,
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: expected.to_owned(),
            },
            "cross-function violations must have typed refusals"
        );
    }
    let wider_match = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function add_or_zero(left: I64, right: I64) returns I64:\n  match I64.checked_add(left, right):\n    case ok(value):\n      let copy = value\n      return copy\n    case error(reason):\n      return I64.literal(0)\n  end match\nend function\n"
                    .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        wider_match,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CONTROL".to_owned(),
        },
        "match shapes outside the frozen slice must remain typed refusals"
    );

    let unresolved_let = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function keep(value: I64) returns I64:\n  let copy = ghost\n  return copy\nend function\n"
                    .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unresolved_let,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-STATEMENT".to_owned(),
        },
        "unresolved let aliases must remain typed refusals"
    );

    let unsupported_let_op = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function equal(a: I64, b: I64) returns Bool:\n  let flag = I64.equal(a, b)\n  return flag\nend function\n"
                    .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unsupported_let_op,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-STATEMENT".to_owned(),
        },
        "let RHS operations outside the declared subset must remain typed refusals"
    );
}
