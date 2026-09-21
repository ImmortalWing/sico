//! M22 S6 (STEP-0212): the Sico-written frontend lowers fixed-width
//! literal operands of fixed-width operations — const instructions emit
//! before the operation with digits/sign ranges and SSA-ordered values.

use std::sync::atomic::{AtomicU64, Ordering};

use sico_ir::{Module, canonical_json, lower_core, verify};
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits,
    ScriptInput,
};
use sico_source::{SourceFile, SourceId};

const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const PARSER_DRIVER_SOURCE: &str = include_str!("../../../selfhost/parser_driver.sico");
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
const INPUT: &[u8] = b"function main() returns Int:\n  let xs = sico.list.empty()\n  let total = I64.literal(0)\n  for x in xs:\n    if I64.equal(total, I64.literal(0)):\n      set total = I64.literal(1)\n    else:\n      while I64.less_than(total, I64.literal(2)):\n        match I64.checked_add(total, I64.literal(1)):\n          case ok(next):\n            set total = next\n          case error(_):\n            return -1\n        end match\n      end while\n    end if\n  end for\n  return 0\nend function\n\nfunction add(a: I64, b: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(I64.literal(1), b)\nend function\n\nfunction message() returns Text:\n  return \"ready, (nested-looking)\"\nend function\n";

fn compile_parser() -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!(
        "sico-step0201-parser-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("parser_driver.sico");
    let component_path = directory.join("parser.component.wasm");
    std::fs::write(directory.join("parser.sico"), PARSER_SOURCE).unwrap();
    std::fs::write(&source_path, PARSER_DRIVER_SOURCE).unwrap();
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
    let source_text = std::str::from_utf8(source).unwrap();
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
        panic!("expected canonical IR output for {source_text:?}, got {outcome:?}");
    };
    assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
    let module: Module = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "IR JSON deserializes for {source_text:?}: {error}; output={}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let verify_errors = verify(&module);
    assert!(
        verify_errors.is_empty(),
        "self-host IR must verify for {source_text:?}: {verify_errors:?}; raw={}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        canonical_json(&module).unwrap().as_bytes(),
        output.stdout,
        "self-host IR is not canonical JSON"
    );
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
        b"function add(a: I64, b: I64) returns I64:\n  match I64.checked_add(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(a: U64, b: U64) returns U64:\n  match U64.checked_add(b, a):\n    case ok(sum):\n      return sum\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n",
    );
    for source in [
        b"function op(a: I64, b: I64) returns I64:\n  match I64.checked_sub(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n".as_slice(),
        b"function op(a: U64, b: U64) returns U64:\n  match U64.checked_sub(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n".as_slice(),
        b"function op(a: I64, b: I64) returns I64:\n  match I64.checked_mul(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n".as_slice(),
        b"function op(a: U64, b: U64) returns U64:\n  match U64.checked_mul(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n".as_slice(),
        b"function op(a: I64, b: I64) returns I64:\n  match I64.checked_div(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n".as_slice(),
        b"function op(a: U64, b: U64) returns U64:\n  match U64.checked_div(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n".as_slice(),
    ] {
        assert_ir_matches_rust(&prepared, source);
    }
    assert_ir_matches_rust(
        &prepared,
        b"function op(a: I64, b: I64) returns I64:\n  match I64.checked_add(a, I64.literal(-1)):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function op(a: U64, b: U64) returns U64:\n  match U64.checked_mul(U64.literal(2), b):\n    case ok(value):\n      return value\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function id(value: I64) returns I64:\n  return value\nend function\n\nfunction op(a: I64, b: I64) returns I64:\n  match I64.checked_add(id(a), b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function id(value: U64) returns U64:\n  return value\nend function\n\nfunction op(a: U64, b: U64) returns U64:\n  match U64.checked_div(a, id(b)):\n    case ok(value):\n      return value\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function id(value: I64) returns I64:\n  return value\nend function\n\nfunction op(a: I64, b: I64) returns I64:\n  match I64.checked_sub(id(id(a)), b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function id(value: U64) returns U64:\n  return value\nend function\n\nfunction op(a: U64, b: U64) returns U64:\n  match U64.checked_mul(id(U64.literal(3)), b):\n    case ok(value):\n      return value\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function op(a: I64, b: I64) returns I64:\n  let value = I64.bit_and(a, b)\n  return value\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function shift(a: U64) returns U64:\n  let value = U64.shl(a, U64.literal(3))\n  return value\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function op(a: I64, b: I64) returns I64:\n  let masked = I64.bit_and(a, b)\n  let value = I64.bit_or(masked, a)\n  return value\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function shift(a: U64, amount: U64) returns U64:\n  let shifted = U64.shl(a, amount)\n  let value = U64.bit_xor(a, shifted)\n  return value\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64, b: I64) returns I64:\n  let first = I64.bit_and(a, b)\n  let second = I64.bit_or(first, a)\n  let third = I64.bit_xor(b, second)\n  let fourth = I64.bit_and(third, first)\n  return fourth\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: U64, b: U64) returns U64:\n  let first = U64.shl(a, b)\n  let second = U64.bit_or(first, a)\n  let third = U64.bit_xor(second, first)\n  let fourth = U64.shr(third, b)\n  let fifth = U64.bit_and(fourth, second)\n  return a\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64) returns I64:\n  let first = I64.bit_and(a, I64.literal(-1))\n  let second = I64.bit_or(I64.literal(2), first)\n  let third = I64.bit_xor(second, I64.literal(7))\n  return third\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: U64) returns U64:\n  let first = U64.bit_or(U64.literal(1), U64.literal(2))\n  let second = U64.shl(first, U64.literal(3))\n  let third = U64.bit_and(second, a)\n  return third\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function merge(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction pipeline(a: I64, b: I64) returns I64:\n  let first = I64.bit_and(a, b)\n  let second = merge(first, I64.literal(3))\n  let third = I64.bit_or(second, first)\n  return third\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function choose(value: Int, ready: Bool, label: Text) returns Text:\n  return label\nend function\n\nfunction pipeline(value: Int, ready: Bool, label: Text) returns Text:\n  let first = choose(value, ready, label)\n  let second = choose(value, true, first)\n  let third = choose(7, ready, second)\n  return third\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function identity(value: I64) returns I64:\n  return value\nend function\n\nfunction pipeline(value: I64) returns I64:\n  let result = identity(value)\n  return result\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function identity(value: U64) returns U64:\n  return value\nend function\n\nfunction pipeline(value: U64) returns U64:\n  let first = identity(U64.literal(4))\n  let second = identity(first)\n  return second\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function answer() returns Int:\n  let value = 42\n  return value\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(left: Int, right: Int) returns Int:\n  return left\nend function\n\nfunction pipeline(ready: Bool) returns Int:\n  let base = 7\n  let total = add(base, 35)\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function choose(ready: Bool, alternate: Bool) returns Bool:\n  return ready\nend function\n\nfunction pipeline() returns Bool:\n  let first = true\n  let second = choose(first, false)\n  return second\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function label(text: Text) returns Text:\n  return text\nend function\n\nfunction pipeline() returns Text:\n  let first = \"line\\n\\\"ok\\\"\"\n  let second = label(first)\n  return second\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        "function pick(count: Int, ready: Bool, name: Text) returns Text:\n  return name\nend function\n\nfunction pipeline() returns Text:\n  let count = 3\n  let ready = true\n  let name = \"你好\"\n  let result = pick(count, ready, name)\n  return result\nend function\n".as_bytes(),
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64, b: I64) returns I64:\n  let x = a\n  let y = x\n  let z = I64.bit_and(y, b)\n  return z\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64) returns I64:\n  let x = a\n  let y = I64.bit_or(x, a)\n  return x\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(flag: Bool) returns Int:\n  let base = 7\n  let copy = base\n  return 0\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline() returns Bool:\n  let first = true\n  return false\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        "function pipeline(label: Text) returns Text:\n  let first = \"a\"\n  return \"你好\"\nend function\n".as_bytes(),
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64) returns I64:\n  let masked = I64.bit_and(a, I64.literal(-1))\n  return I64.literal(3)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: U64) returns U64:\n  let masked = U64.bit_and(a, U64.literal(7))\n  return U64.literal(18446744073709551615)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function merge(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction pipeline(a: I64, b: I64) returns I64:\n  let first = I64.bit_and(a, b)\n  return merge(first, I64.literal(3))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function sub(left: Int, right: Int) returns Int:\n  return left\nend function\n\nfunction pipeline(x: Int, y: Int) returns Int:\n  let z = 7\n  return sub(y, x)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction pipeline(v: Int) returns Int:\n  let base = 1\n  return f(g(v), base)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function choose(ready: Bool, label: Text) returns Text:\n  return label\nend function\n\nfunction pipeline(ready: Bool) returns Text:\n  let name = \"hi\"\n  return choose(ready, name)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64, b: I64) returns I64:\n  let x = a\n  return I64.bit_and(x, b)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: U64) returns U64:\n  let x = U64.shl(a, U64.literal(1))\n  return U64.shr(x, U64.literal(2))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64) returns I64:\n  let x = I64.bit_and(a, a)\n  return I64.bit_xor(I64.literal(1), x)\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: U64, b: U64) returns U64:\n  let first = U64.bit_or(a, b)\n  let second = U64.bit_xor(first, b)\n  return U64.bit_and(second, U64.literal(15))\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(start: I64) returns I64:\n  let total = start\n  set total = total\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(flag: Bool) returns Int:\n  let count = 0\n  set count = 7\n  return count\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(ready: Bool) returns Bool:\n  let state = false\n  set state = ready\n  return state\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64) returns I64:\n  let mask = I64.literal(-1)\n  set mask = I64.literal(3)\n  return mask\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(start: I64) returns I64:\n  let total = start\n  let copy = total\n  set total = copy\n  return copy\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(label: Text) returns Text:\n  let name = label\n  set name = \"hi\"\n  return name\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64, mask: I64) returns I64:\n  let total = a\n  set total = I64.bit_and(total, mask)\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: U64) returns U64:\n  let acc = a\n  set acc = U64.shl(acc, U64.literal(1))\n  set acc = U64.bit_or(acc, U64.literal(3))\n  return acc\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pipeline(a: I64, b: I64) returns I64:\n  let masked = I64.bit_or(a, b)\n  set masked = I64.bit_and(masked, a)\n  return masked\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function merge(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction pipeline(a: I64) returns I64:\n  let total = a\n  set total = merge(total, I64.literal(7))\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function add(l: Int, r: Int) returns Int:\n  return l\nend function\n\nfunction pipeline(x: Int) returns Int:\n  let total = x\n  set total = add(total, 5)\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function choose(count: Int, ready: Bool, name: Text) returns Int:\n  return count\nend function\n\nfunction pipeline(x: Int) returns Int:\n  let total = x\n  set total = choose(total, true, \"go\")\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function one() returns Int:\n  return 1\nend function\n\nfunction pipeline(x: Int) returns Int:\n  let total = x\n  set total = one()\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction pipeline(x: Int, y: Int) returns Int:\n  let total = x\n  set total = f(g(y), total)\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction pipeline(x: Int, y: Int) returns Int:\n  let total = x\n  set total = f(g(y), g(y))\n  return total\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction pipeline(x: Int, y: Int) returns Int:\n  let combined = f(g(y), x)\n  set combined = g(combined)\n  return combined\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(a: I64, b: I64) returns I64:\n  if I64.equal(a, b):\n    return a\n  else:\n    return b\n  end if\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(a: I64, b: I64) returns I64:\n  let best = a\n  if I64.less_than(b, a):\n    set best = b\n  end if\n  return best\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(a: I64, b: I64) returns I64:\n  let best = a\n  if I64.less_than(b, a):\n    set best = b\n  else:\n    set best = a\n  end if\n  return best\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(a: I64, limit: I64) returns I64:\n  let best = a\n  if I64.less_than(best, limit):\n    set best = limit\n  end if\n  return best\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(a: I64, b: I64) returns I64:\n  if I64.equal(a, b):\n    return I64.literal(0)\n  else:\n    return I64.literal(1)\n  end if\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function pick(a: I64, b: I64) returns I64:\n  let best = a\n  let worst = b\n  if I64.less_than(b, a):\n    set best = b\n    set worst = a\n  else:\n    set best = a\n    set worst = b\n  end if\n  return best\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function work(a: I64) returns I64:\n  let n = a\n  while I64.less_than(I64.literal(0), n):\n    set n = I64.bit_and(n, I64.literal(-2))\n  end while\n  return n\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function work(a: I64) returns I64:\n  let n = a\n  while I64.less_than(I64.literal(0), n):\n    if I64.equal(n, I64.literal(1)):\n      break\n    end if\n    set n = I64.bit_and(n, I64.literal(-2))\n  end while\n  return n\nend function\n",
    );
    assert_ir_matches_rust(
        &prepared,
        b"function work(a: I64) returns I64:\n  let n = a\n  let steps = I64.literal(0)\n  while I64.less_than(I64.literal(0), n):\n    set steps = I64.bit_or(steps, I64.literal(1))\n    if I64.equal(n, I64.literal(1)):\n      continue\n    end if\n    set n = I64.bit_and(n, I64.literal(-2))\n  end while\n  return steps\nend function\n",
    );

    let unknown_let_atom_source =
        b"function pipeline(value: I64) returns I64:\n  let first = 7\n  let second = ghost\n  return first\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(unknown_let_atom_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject an unknown bare let atom"
    );
    let unknown_let_atom = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: unknown_let_atom_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unknown_let_atom,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-EXPRESSION".to_owned(),
        },
        "unknown bare let atoms must fail closed"
    );

    let bad_const_escape_source =
        b"function pipeline(value: Int) returns Int:\n  let first = 7\n  let label = \"bad\\x\"\n  return first\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(bad_const_escape_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject an invalid escape in a constant binding"
    );
    let bad_const_escape = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: bad_const_escape_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        bad_const_escape,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-STRING-ESCAPE".to_owned(),
        },
        "invalid escapes in constant bindings must fail closed"
    );

    let noncanonical_const_source =
        b"function pipeline(value: Int) returns Int:\n  let first = 7\n  let second = 07\n  return first\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(noncanonical_const_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject a non-canonical integer constant binding"
    );
    let noncanonical_const = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: noncanonical_const_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        noncanonical_const,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-EXPRESSION".to_owned(),
        },
        "non-canonical integer constant bindings must fail closed"
    );

    for (source, expected) in [
        (
            b"function pipeline(a: I64) returns I64:\n  let masked = I64.bit_and(a, a)\n  return ghost(a)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TARGET",
        ),
        (
            b"function pipeline(a: I64) returns I64:\n  let masked = I64.bit_and(a, a)\n  return I64.literal(9223372036854775808)\nend function\n".as_slice(),
            "ERR:E-SH-IR-I64-RANGE",
        ),
        (
            b"function merge(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction pipeline(a: I64, label: Text) returns I64:\n  let masked = I64.bit_and(a, a)\n  return merge(label, a)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function merge(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction pipeline(a: I64) returns I64:\n  let masked = I64.bit_and(a, a)\n  return merge(a)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARITY",
        ),
        (
            b"function pipeline(a: I64, label: Text) returns I64:\n  let masked = I64.bit_and(a, a)\n  return I64.bit_and(a, label)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function pipeline(a: I64, b: I64) returns I64:\n  let masked = I64.bit_and(a, b)\n  return I64.bit_and(a, b, a)\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-ARITY",
        ),
        (
            b"function pipeline(start: I64) returns I64:\n  set total = start\n  return total\nend function\n".as_slice(),
            "ERR:E-SH-IR-SET-CELL",
        ),
        (
            b"function pipeline(flag: Bool) returns Int:\n  let count = 0\n  set count = true\n  return count\nend function\n".as_slice(),
            "ERR:E-SH-IR-TYPE-MISMATCH",
        ),
        (
            b"function pipeline(flag: Bool) returns Int:\n  let count = 0\n  set count = 1\n  let count = 2\n  return count\nend function\n".as_slice(),
            "ERR:E-SH-IR-CELL-REDEFINED",
        ),
        (
            b"function pipeline(a: I64, label: Text) returns I64:\n  let total = a\n  set total = I64.bit_and(total, label)\n  return total\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function merge(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction pipeline(a: I64) returns I64:\n  let total = a\n  set total = ghost(total)\n  return total\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TARGET",
        ),
        (
            b"function g(v: Int) returns Int:\n  return v\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction pipeline(x: Int, y: Int) returns Int:\n  let total = x\n  set total = f(ghost(y), total)\n  return total\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TARGET",
        ),
        (
            b"function g(v: Int) returns Text:\n  return \"t\"\nend function\n\nfunction f(a: Int, b: Int) returns Int:\n  return a\nend function\n\nfunction pipeline(x: Int, y: Int) returns Int:\n  let total = x\n  set total = f(g(y), total)\n  return total\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
        ),
        (
            b"function pick(a: I64, b: I64) returns I64:\n  if I64.equal(a, b):\n    return 0\n  else:\n    return 1\n  end if\nend function\n".as_slice(),
            "ERR:E-SH-IR-TYPE-MISMATCH",
        ),
        (
            b"function pick(a: I64, label: Text) returns I64:\n  if I64.less_than(a, label):\n    return a\n  else:\n    return label\n  end if\nend function\n".as_slice(),
            "ERR:E-SH-IR-CALL-TYPE",
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
            "Rust lowering must refuse this return-position violation: {source_text:?}",
            source_text = std::str::from_utf8(source).unwrap()
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
            "return-position violations must have typed refusals"
        );
    }

    let unknown_let_call_source =
        b"function pipeline(value: I64) returns I64:\n  let result = missing(value)\n  return result\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(unknown_let_call_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject an unknown let-call target"
    );
    let unknown_let_call = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: unknown_let_call_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unknown_let_call,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-TARGET".to_owned(),
        },
        "unknown let-call targets must fail closed"
    );

    let long_block_call_type_source =
        b"function accept(value: I64) returns I64:\n  return value\nend function\n\nfunction pipeline(a: I64, label: Text) returns I64:\n  let first = accept(a)\n  let second = accept(label)\n  let third = I64.bit_or(first, second)\n  return third\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(long_block_call_type_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject a wrong-type long-block call argument"
    );
    let long_block_call_type = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: long_block_call_type_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        long_block_call_type,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-TYPE".to_owned(),
        },
        "wrong-type long-block call arguments must fail closed"
    );

    let long_block_literal_overflow_source =
        b"function op(a: U64) returns U64:\n  let first = U64.bit_or(a, U64.literal(1))\n  let second = U64.bit_xor(first, U64.literal(18446744073709551616))\n  let third = U64.bit_and(second, a)\n  return third\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(long_block_literal_overflow_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject an overflowing long-block literal"
    );
    let long_block_literal_overflow = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: long_block_literal_overflow_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        long_block_literal_overflow,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-U64-RANGE".to_owned(),
        },
        "overflowing long-block literals must fail closed"
    );

    let future_binding_source =
        b"function op(a: I64, b: I64) returns I64:\n  let first = I64.bit_and(later, b)\n  let second = I64.bit_or(first, a)\n  let later = I64.bit_xor(second, b)\n  return later\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(future_binding_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject a forward local reference"
    );
    let future_binding = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: future_binding_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        future_binding,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-ARGUMENT".to_owned(),
        },
        "forward local references must fail closed"
    );

    let wrong_chained_kind_source =
        b"function op(a: I64, b: I64, bits: U64) returns U64:\n  let masked = I64.bit_and(a, b)\n  let value = U64.bit_or(masked, bits)\n  return value\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(wrong_chained_kind_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject a wrong-width chained binding"
    );
    let wrong_chained_kind = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: wrong_chained_kind_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        wrong_chained_kind,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-TYPE".to_owned(),
        },
        "wrong-width chained bindings must fail closed"
    );

    let extra_chained_operand_source =
        b"function op(a: I64, b: I64) returns I64:\n  let masked = I64.bit_and(a, b)\n  let value = I64.bit_or(masked, a, b)\n  return value\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(extra_chained_operand_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject an extra chained operand"
    );
    let extra_chained_operand = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: extra_chained_operand_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        extra_chained_operand,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-ARITY".to_owned(),
        },
        "extra chained operands must fail closed"
    );

    let unresolved_let_return_source =
        b"function op(a: I64, b: I64) returns I64:\n  let value = I64.bit_or(a, b)\n  return ghost\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(unresolved_let_return_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject an unresolved straight-line return"
    );
    let unresolved_let_return = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: unresolved_let_return_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unresolved_let_return,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-UNRESOLVED".to_owned(),
        },
        "unresolved straight-line returns must fail closed"
    );

    let unsupported_match_shape = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function add(a: I64, b: I64) returns I64:\n  match I64.checked_add(a, b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(1)\n  end match\nend function\n"
                    .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        unsupported_match_shape,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-MATCH-SHAPE".to_owned(),
        },
        "unsupported match fallback must fail closed without IR bytes"
    );

    let wrong_nested_type_source =
        b"function narrow() returns Int:\n  return 1\nend function\n\nfunction op(a: I64, b: I64) returns I64:\n  match I64.checked_add(narrow(), b):\n    case ok(value):\n      return value\n    case error(_):\n      return I64.literal(0)\n  end match\nend function\n";
    let rust_source = SourceFile::from_text(
        SourceId::new(0),
        "selfhost-input.sico",
        std::str::from_utf8(wrong_nested_type_source)
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert!(
        lower_core(&rust_source).is_err(),
        "Rust lowering must reject a wrong-width nested checked operand"
    );
    let wrong_nested_type = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: wrong_nested_type_source.to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        wrong_nested_type,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CALL-TYPE".to_owned(),
        },
        "wrong-width nested checked operands must fail closed"
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
                    b"function widget(value: Widget) returns Widget:\n  return value\nend function\n"
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

    // STEP-0250 opened Bytes parameters: the previously-refused shape now
    // lowers through the verifier with its Bytes parameter intact.
    let bytes_parameter = prepared
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
    let RunOutcome::Output(bytes_output) = bytes_parameter else {
        panic!("Bytes parameters must lower after STEP-0250: {bytes_parameter:?}")
    };
    assert_eq!(bytes_output.exit_code, 0, "{:?}", bytes_output.stderr);
    assert!(
        String::from_utf8_lossy(&bytes_output.stdout)
            .contains("\"name\":\"bytes\",\"parameters\":[{\"id\":0,\"name\":\"value\",\"ty\":{\"kind\":\"bytes\"}"),
        "Bytes parameter must surface as a bytes-typed parameter"
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

    let shadowed_parameter = prepared
        .run(
            &ScriptInput {
                arguments: vec!["--emit-ir".to_owned()],
                stdin: b"function shadow(x: I64) returns I64:\n  let x = I64.literal(1)\n  set x = I64.literal(2)\n  return x\nend function\n"
                    .to_vec(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(
        shadowed_parameter,
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "ERR:E-SH-IR-CELL-SHADOW".to_owned(),
        },
        "the advanced cell frontend must fail closed instead of resolving a shadowed parameter differently from Rust"
    );
}
