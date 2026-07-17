use std::fmt::Write as _;

use sico_semantics::{MAX_SEMANTIC_DIAGNOSTICS, analyze};
use sico_source::{SourceFile, SourceId};

#[test]
fn fixed_width_intrinsics_have_explicit_checked_semantics() {
    let text = "function signed_add(left: I64, right: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(left, right)\nend function\n\nfunction unsigned_sub(left: U64, right: U64) returns Result[U64, NumericError]:\n  return U64.checked_sub(left, right)\nend function\n\nfunction signed_less(left: I64, right: I64) returns Bool:\n  return I64.less_than(left, right)\nend function\n\nfunction unsigned_equal(left: U64, right: U64) returns Bool:\n  return U64.equal(left, right)\nend function\n\nfunction signed_literal() returns I64:\n  return I64.literal(-9223372036854775808)\nend function\n\nfunction unsigned_literal() returns U64:\n  return U64.literal(18446744073709551615)\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "fixed-valid.sico", text).unwrap();
    let analysis = analyze(&source).unwrap();
    assert!(analysis.is_success(), "{:?}", analysis.diagnostics);
    assert_eq!(analysis, analyze(&source).unwrap());
}

#[test]
fn fixed_width_implicit_and_unchecked_operations_are_rejected() {
    for (index, (expression, returns, code, message)) in [
        (
            "left + right",
            "I64",
            "E2001",
            "expected I64.checked_add, found unchecked +",
        ),
        (
            "1",
            "I64",
            "E2002",
            "expected I64, found Int; convert explicitly",
        ),
        (
            "I64.literal(9223372036854775808)",
            "I64",
            "E2001",
            "expected I64 literal in range, found 9223372036854775808",
        ),
        (
            "U64.literal(18446744073709551616)",
            "U64",
            "E2001",
            "expected U64 literal in range, found 18446744073709551616",
        ),
        (
            "I64.literal(-9223372036854775809)",
            "I64",
            "E2001",
            "expected I64 literal in range, found -9223372036854775809",
        ),
        (
            "U64.literal(-1)",
            "U64",
            "E2001",
            "expected U64 literal in range, found -1",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let parameters = if expression == "left + right" {
            "left: I64, right: I64"
        } else {
            ""
        };
        let text = format!(
            "function invalid({parameters}) returns {returns}:\n  return {expression}\nend function\n"
        );
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            "fixed-invalid.sico",
            text,
        )
        .unwrap();
        let analysis = analyze(&source).unwrap();
        assert_eq!(analysis.diagnostics.len(), 1, "{:?}", analysis.diagnostics);
        assert_eq!(analysis.diagnostics[0].code, code);
        assert_eq!(analysis.diagnostics[0].message, message);
    }
}

#[test]
fn fixed_width_intrinsic_arity_is_checked_semantically() {
    for (index, (expression, returns, expected)) in [
        (
            "I64.literal()",
            "I64",
            "expected 1 argument, found 0 arguments",
        ),
        (
            "U64.checked_add(left)",
            "Result[U64, NumericError]",
            "expected 2 arguments, found 1 arguments",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let parameters = if expression.contains("left") {
            "left: U64"
        } else {
            ""
        };
        let text = format!(
            "function invalid({parameters}) returns {returns}:\n  return {expression}\nend function\n"
        );
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            "fixed-arity.sico",
            text,
        )
        .unwrap();
        let analysis = analyze(&source).unwrap();
        assert_eq!(analysis.diagnostics.len(), 1, "{:?}", analysis.diagnostics);
        assert_eq!(analysis.diagnostics[0].code, "E2001");
        assert_eq!(analysis.diagnostics[0].message, expected);
    }
}

#[test]
fn deterministic_semantic_inputs_are_bounded_and_repeatable() {
    let mut accepted = 0;
    let mut rejected = 0;
    for index in 0..2_048_u32 {
        let depth = usize::try_from(index % 32).unwrap();
        let expression = if index % 2 == 0 {
            let mut expression = format!("{} + 1", index / 2);
            for _ in 0..depth {
                expression = format!("({expression})");
            }
            expression
        } else {
            format!("\"type-{index}\"")
        };
        let text = format!(
            "function generated_{index}() returns Int:\n  return {expression}\nend function\n"
        );
        let source = SourceFile::from_bytes(
            SourceId::new(index),
            format!("generated-{index}.sico"),
            text.as_bytes(),
        )
        .unwrap();
        let analysis = analyze(&source).unwrap();
        assert_eq!(analysis, analyze(&source).unwrap());
        assert!(analysis.diagnostics.len() <= MAX_SEMANTIC_DIAGNOSTICS);
        assert!(
            analysis
                .diagnostics
                .iter()
                .all(|diagnostic| source.span(diagnostic.range).is_some())
        );
        if index % 2 == 0 {
            accepted += 1;
            assert!(analysis.is_success());
        } else {
            rejected += 1;
            assert_eq!(analysis.diagnostics.len(), 1);
            assert_eq!(analysis.diagnostics[0].code, "E2001");
        }
    }
    assert_eq!((accepted, rejected), (1_024, 1_024));
}

#[test]
fn diagnostic_cap_is_exact_under_many_independent_root_causes() {
    let arguments = (0..150)
        .map(|index| format!("unknown_{index}: {index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let text = format!(
        "record OnlyKnown:\n  field known: Int\nend record\n\nfunction main() returns OnlyKnown:\n  return OnlyKnown({arguments})\nend function\n"
    );
    let source =
        SourceFile::from_bytes(SourceId::new(0), "diagnostic-cap.sico", text.as_bytes()).unwrap();
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.diagnostics.len(), MAX_SEMANTIC_DIAGNOSTICS);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == "E2011")
    );
}

#[test]
fn large_local_scope_and_deep_expression_remain_deterministic() {
    let mut text = String::from("function large() returns Int:\n");
    for index in 0..2_000 {
        writeln!(text, "  let value_{index} = {index}").expect("writing to a string cannot fail");
    }
    let mut expression = "value_1999".to_owned();
    for _ in 0..200 {
        expression = format!("({expression})");
    }
    write!(text, "  return {expression}\nend function\n").expect("writing to a string cannot fail");
    let source = SourceFile::from_bytes(SourceId::new(0), "large.sico", text.as_bytes()).unwrap();
    let analysis = analyze(&source).unwrap();
    assert!(analysis.is_success());
    assert_eq!(analysis, analyze(&source).unwrap());
    assert_eq!(
        analysis
            .facts
            .iter()
            .filter(|fact| fact.name.starts_with("large.value_"))
            .count(),
        2_000
    );
}
