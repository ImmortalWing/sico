use std::fmt::Write as _;

use sico_semantics::{MAX_SEMANTIC_DIAGNOSTICS, analyze};
use sico_source::{SourceFile, SourceId};

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
