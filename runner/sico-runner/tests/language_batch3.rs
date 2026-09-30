//! M23 language v1 batch 3 (STEP-0328/STEP-0329, RFC-0048/RFC-0049):
//! infix comparison conditions, `&&`, and the two let-bound statement
//! forms end-to-end through the real Component Runtime, plus the typed
//! refusal corpora — check-channel E2025/E2026/E2027, and the named
//! build-channel refusals for operand types, chained comparisons, `&&`
//! position violations, and nested if-expression conditions.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const INFIX_COMPARISONS_SOURCE: &str =
    include_str!("../../../tests/end-to-end/infix-comparisons.sico");
const LET_BIND_SOURCE: &str = include_str!("../../../tests/end-to-end/let-bind.sico");

fn compile_source(tag: &str, source: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0328-{tag}-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join(format!("{tag}.sico"));
    let component_path = directory.join(format!("{tag}.component.wasm"));
    std::fs::write(&source_path, source).unwrap();
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

fn run_component(component: &[u8]) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("batch-3 component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn infix_comparisons_and_conjunction_run_deterministically() {
    let component = compile_source("infix-comparisons", INFIX_COMPARISONS_SOURCE);
    let expected: &[u8] =
        b"v|lt|le|gt|ge-boundary|ne|le-boundary|ugt|ule|ult|uge-boundary|une|ueq|and3|loop|jdone|done";
    for _ in 0..2 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
                assert_eq!(output.stdout, expected, "stdout drifted");
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

fn cli_refused(tag: &str, source: &str, command: &str, needle: &str) {
    let directory = std::env::temp_dir().join(format!("sico-step0328-refusal-{tag}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join(format!("{tag}.sico"));
    std::fs::write(&source_path, source).unwrap();
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let mut arguments = vec![
        std::ffi::OsString::from("sico"),
        std::ffi::OsString::from(command),
    ];
    if command == "build" {
        arguments.push(std::ffi::OsString::from("--profile"));
        arguments.push(std::ffi::OsString::from("script-v0"));
    }
    arguments.push(source_path.as_os_str().to_owned());
    let exit = sico_cli::run(arguments, &mut std::io::empty(), &mut stdout, &mut stderr);
    let _ = std::fs::remove_dir_all(directory);
    // Semantic diagnostics exit 1; lexical, syntax and build failures exit 2.
    assert_ne!(exit, 0, "{tag} must be refused");
    let text = String::from_utf8_lossy(&stderr);
    assert!(
        text.contains(needle),
        "{tag}: typed refusal expected, got: {text}"
    );
}

const HEADER: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

fn main_wrapping(body: &str) -> String {
    format!(
        "{HEADER}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n{body}  return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    )
}

#[test]
fn non_atom_swap_form_operands_are_refused_at_check_with_e2025() {
    // `>` and `<=` are the swap-form operators: their operands must be
    // atoms (RFC-0048 §3), and a call-shaped operand is E2025.
    cli_refused(
        "e2025-greater-call",
        &main_wrapping(
            "  let a = I64.literal(3)\n  if a > I64.literal(4):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "check",
        "E2025",
    );
    cli_refused(
        "e2025-less-equal-call",
        &main_wrapping(
            "  let a = I64.literal(3)\n  if a <= I64.literal(4):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "check",
        "E2025",
    );
    // Atom operands (identifier bindings) pass check and build.
    let component = compile_source(
        "swap-atoms",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let four = I64.literal(4)\n  let out = \"n\"\n  if four > a:\n    set out = \"y\"\n  end if\n  return ok(ScriptOutput(stdout: sico.text.encode(out), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n",
        ),
    );
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(output.stdout, b"y", "swap-form atom run drifted");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn mismatched_and_non_fixed_comparison_operands_are_refused_at_build() {
    // Mixed width: I64 vs U64.
    cli_refused(
        "mixed-width",
        &main_wrapping(
            "  let a = I64.literal(3)\n  if a < U64.literal(4):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "build",
        "infix comparison condition operand types",
    );
    // Bare integer literals stay unbounded Int (NUM-001).
    cli_refused(
        "bare-int-operand",
        &main_wrapping(
            "  let a = I64.literal(3)\n  if a < 4:\n    set a = I64.literal(0)\n  end if\n",
        ),
        "build",
        "infix comparison condition operand types",
    );
    // Text operands have no ordered comparison in the IR.
    cli_refused(
        "text-operand",
        &main_wrapping(
            "  if \"a\" < \"b\":\n    return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n",
        ),
        "build",
        "infix comparison condition operand types",
    );
    // Bool operands likewise.
    cli_refused(
        "bool-operand",
        &main_wrapping(
            "  if true < false:\n    return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n",
        ),
        "build",
        "infix comparison condition operand types",
    );
}

#[test]
fn chained_comparisons_are_refused_at_build() {
    cli_refused(
        "chained",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let b = I64.literal(7)\n  if a < b < I64.literal(9):\n    return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n",
        ),
        "build",
        "chained comparison",
    );
}

#[test]
fn conjunction_outside_conditions_and_non_condition_operands_are_refused_at_build() {
    // `let`-bound `&&` is a value position — refused (RFC-0048 §3).
    cli_refused(
        "and-value-position",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let b = I64.literal(7)\n  let c = a < b && a != I64.literal(0)\n",
        ),
        "build",
        "&& outside a condition position",
    );
    // A Bool binding is not a comparison condition.
    cli_refused(
        "and-non-condition",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let flag = true\n  if flag && a < I64.literal(4):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "build",
        "&& operand is not a comparison condition",
    );
}

#[test]
fn disjunction_and_bare_operator_characters_stay_lexical_refusals() {
    cli_refused(
        "disjunction",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let b = I64.literal(7)\n  if a < b || a != I64.literal(0):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "check",
        "UnexpectedCharacter('|')",
    );
    cli_refused(
        "bare-bang",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let b = I64.literal(7)\n  if !(a < b):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "check",
        "UnexpectedCharacter('!')",
    );
    cli_refused(
        "bare-amp",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let b = I64.literal(7)\n  if a < b & a != I64.literal(0):\n    set a = I64.literal(0)\n  end if\n",
        ),
        "check",
        "UnexpectedCharacter('&')",
    );
}

#[test]
fn batch3_condition_nesting_refuses_at_the_block_depth_limit_plus_one() {
    // At-limit: function block + 255 nested batch-3 condition blocks sits
    // exactly at MAX_PARSE_DEPTH (256) and builds.
    let at_limit = nested_batch3_ifs(255);
    let _ = compile_source("batch3-nesting-at-limit", &at_limit);
    // Limit+1 refuses with the parse nesting diagnostic.
    let over_limit = nested_batch3_ifs(256);
    cli_refused(
        "batch3-nesting-over-limit",
        &over_limit,
        "check",
        "NestingLimitExceeded { limit: 256 }",
    );
}

fn nested_batch3_ifs(depth: usize) -> String {
    let mut text = format!(
        "{HEADER}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let a = I64.literal(3)\n  let b = I64.literal(7)\n"
    );
    for _ in 0..depth {
        text.push_str("  if a < b && a != I64.literal(0):\n");
    }
    text.push_str("  return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n");
    for _ in 0..depth {
        text.push_str("  end if\n");
    }
    text.push_str(
        "  return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"no\"))\n",
    );
    text.push_str("end function\n");
    text
}

#[test]
fn let_bound_if_and_match_forms_run_deterministically() {
    let component = compile_source("let-bind", LET_BIND_SOURCE);
    let expected: &[u8] = b"vif-taken4eqand7limit|bound";
    for _ in 0..2 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
                assert_eq!(output.stdout, expected, "stdout drifted");
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

#[test]
fn let_bound_arm_content_rules_are_refused_at_check_with_e2026() {
    // A `set` plus an expression is two statements, not one arm expression.
    cli_refused(
        "lb-two-statements",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let x = if a < I64.literal(4):\n    set a = I64.literal(0)\n    \"y\"\n  else:\n    \"n\"\n  end if\n",
        ),
        "check",
        "E2026",
    );
    // `return` inside an arm is refused — arms bind values, not control.
    cli_refused(
        "lb-return-arm",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let x = if a < I64.literal(4):\n    return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"no\"))\n  else:\n    \"n\"\n  end if\n",
        ),
        "check",
        "E2026",
    );
    // The same for match arms.
    cli_refused(
        "lb-match-two-statements",
        &main_wrapping(
            "  let x = match I64.checked_add(I64.literal(3), I64.literal(4)):\n    case ok(sum):\n      set x = I64.literal(0)\n      sico.i64.to_text(sum)\n    case error(_):\n      \"overflow\"\n  end match\n",
        ),
        "check",
        "E2026",
    );
}

#[test]
fn let_bound_if_without_else_is_refused_at_check_with_e2027() {
    cli_refused(
        "lb-missing-else",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let x = if a < I64.literal(4):\n    \"y\"\n  end if\n",
        ),
        "check",
        "E2027",
    );
}

#[test]
fn let_bound_arm_type_mismatch_is_refused_at_check_with_e2001() {
    cli_refused(
        "lb-arm-mismatch",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let x = if a < I64.literal(4):\n    \"y\"\n  else:\n    I64.literal(0)\n  end if\n",
        ),
        "check",
        "expected Text, found I64",
    );
}

#[test]
fn annotated_let_bound_form_stays_a_parse_refusal() {
    // `let x: T = if …` is refused in v0 (RFC-0049 §3); the annotation
    // shifts the header tokens, so the line is not a recognized let-bound
    // opener and the legacy close mismatch diagnostic returns.
    cli_refused(
        "lb-annotated",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let x: I64 = if a < I64.literal(4):\n    \"y\"\n  else:\n    \"n\"\n  end if\n",
        ),
        "check",
        "MismatchedClose { expected: Function, actual: If }",
    );
}

#[test]
fn nested_if_expression_condition_is_refused_at_build() {
    cli_refused(
        "lb-nested-condition",
        &main_wrapping(
            "  let a = I64.literal(3)\n  let x = if (if a < I64.literal(4): true else: false end if):\n    \"y\"\n  else:\n    \"n\"\n  end if\n",
        ),
        "build",
        "core expression",
    );
}

#[test]
fn let_bind_nesting_refuses_at_the_block_depth_limit_plus_one() {
    // Each level opens a statement-if (stays open) and a let-bound if
    // (closes immediately): k levels peak at k + 2 of the 256 block slots.
    // 254 levels peak at exactly MAX_PARSE_DEPTH and build.
    let at_limit = nested_let_binds(254);
    let _ = compile_source("let-bind-nesting-at-limit", &at_limit);
    // One more level exceeds MAX_PARSE_DEPTH and refuses at parse.
    let over_limit = nested_let_binds(255);
    cli_refused(
        "let-bind-nesting-over-limit",
        &over_limit,
        "check",
        "NestingLimitExceeded { limit: 256 }",
    );
}

fn nested_let_binds(levels: usize) -> String {
    let mut body = String::new();
    for level in 0..levels {
        body.push_str(&format!(
            "  if a < I64.literal(7):\n  let t{level} = if a < I64.literal(7):\n    \"y\"\n  else:\n    \"n\"\n  end if\n"
        ));
    }
    let mut text = format!(
        "{HEADER}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let a = I64.literal(3)\n"
    );
    text.push_str(&body);
    text.push_str("  return ok(ScriptOutput(stdout: sico.text.encode(\"deep\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n");
    for _ in 0..levels {
        text.push_str("  end if\n");
    }
    text.push_str("  return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"no\"))\nend function\n");
    text
}
