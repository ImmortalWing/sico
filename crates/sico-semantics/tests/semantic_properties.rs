use std::fmt::Write as _;

use sico_semantics::{MAX_SEMANTIC_DIAGNOSTICS, analyze};
use sico_source::{SourceFile, SourceId};

#[test]
fn task_escape_is_rejected_after_local_binding() {
    let text = "async function compute() returns Int:\n  return 1\nend function\n\nasync function leak() returns Task[Int]:\n  task group:\n    let escaped = spawn compute()\n    return escaped\n  end task\nend function\n";
    let source =
        SourceFile::from_text(SourceId::new(0), "indirect-task-escape.sico", text).unwrap();
    let analysis = analyze(&source).unwrap();
    // RFC-0036: the handle is both returned (escapes its scope, E5102) and never
    // consumed by await/collect_tasks before the scope closes (E5103).
    assert_eq!(analysis.diagnostics.len(), 2, "{:?}", analysis.diagnostics);
    assert_eq!(analysis.diagnostics[0].code, "E5103");
    assert_eq!(analysis.diagnostics[0].key, "TASK_NOT_CONSUMED");
    assert_eq!(analysis.diagnostics[1].code, "E5102");
    assert_eq!(analysis.diagnostics[1].key, "TASK_ESCAPES_SCOPE");
}

#[test]
fn task_scope_rules_cover_detached_nested_and_limit_dimensions() {
    // Detached spawn outside any task group is E5104; the handle still consumes once.
    let detached = "async function compute() returns Int:\n  return 1\nend function\n\nasync function main() returns Int:\n  let handle = spawn compute()\n  return await handle\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "detached.sico", detached).unwrap();
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.diagnostics.len(), 1, "{:?}", analysis.diagnostics);
    assert_eq!(analysis.diagnostics[0].code, "E5104");
    assert_eq!(analysis.diagnostics[0].key, "TASK_DETACHED");

    // A handle spawned in an outer scope may be consumed inside a nested scope.
    let nested = "async function compute() returns Int:\n  return 1\nend function\n\nasync function main() returns Int:\n  task group:\n    let handle = spawn compute()\n    task group:\n      let done = await handle\n    end task\n    return 0\n  end task\nend function\n";
    let source = SourceFile::from_text(SourceId::new(1), "nested.sico", nested).unwrap();
    let analysis = analyze(&source).unwrap();
    assert!(analysis.is_success(), "{:?}", analysis.diagnostics);
    assert!(
        analysis
            .facts
            .iter()
            .any(|fact| fact.name == "scope-0:handle->pending"),
        "{:?}",
        analysis.facts
    );
    assert!(
        analysis
            .facts
            .iter()
            .any(|fact| fact.name == "scope-0:handle->awaited"),
        "{:?}",
        analysis.facts
    );
}

#[test]
fn task_scope_limit_covers_nesting_spawn_and_collect_dimensions() {
    // 64 nested task groups stay inside the limit; the 65th is E5105.
    for (index, (depth, expected)) in [(64_usize, 0_usize), (65, 1)].into_iter().enumerate() {
        let mut text = String::from(
            "async function compute() returns Int:\n  return 1\nend function\n\nasync function deep() returns Int:\n",
        );
        for level in 0..depth {
            writeln!(text, "{}task group:", "  ".repeat(level + 1))
                .expect("writing to a string cannot fail");
        }
        let inner = "  ".repeat(depth + 1);
        writeln!(text, "{inner}let handle = spawn compute()").expect("writing cannot fail");
        writeln!(text, "{inner}let done = await handle").expect("writing cannot fail");
        writeln!(text, "{inner}return done").expect("writing cannot fail");
        for level in (0..depth).rev() {
            writeln!(text, "{}end task", "  ".repeat(level + 1))
                .expect("writing to a string cannot fail");
        }
        writeln!(text, "end function").expect("writing to a string cannot fail");
        let source = SourceFile::from_bytes(
            SourceId::new(u32::try_from(index).unwrap()),
            "scope-nesting.sico",
            text.as_bytes(),
        )
        .unwrap();
        let analysis = analyze(&source).unwrap();
        assert_eq!(
            analysis.diagnostics.len(),
            expected,
            "depth {depth}: {:?}",
            analysis.diagnostics
        );
        if expected == 1 {
            assert_eq!(analysis.diagnostics[0].code, "E5105");
            assert_eq!(
                analysis.diagnostics[0].arguments["dimension"],
                sico_semantics::DiagnosticArgument::Text("nesting".to_owned())
            );
        }
    }

    // 1,024 spawns in one scope stay inside the limit; the 1,025th is E5105.
    let mut text = String::from(
        "async function compute() returns Int:\n  return 1\nend function\n\nasync function wide() returns Int:\n  task group:\n",
    );
    for index in 0..1_025 {
        writeln!(text, "    let handle_{index} = spawn compute()")
            .expect("writing to a string cannot fail");
    }
    for index in 0..1_025 {
        writeln!(text, "    let done_{index} = await handle_{index}")
            .expect("writing to a string cannot fail");
    }
    writeln!(text, "    return 0\n  end task\nend function")
        .expect("writing to a string cannot fail");
    let source =
        SourceFile::from_bytes(SourceId::new(2), "scope-spawns.sico", text.as_bytes()).unwrap();
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.diagnostics.len(), 1, "{:?}", analysis.diagnostics);
    assert_eq!(analysis.diagnostics[0].code, "E5105");
    assert_eq!(
        analysis.diagnostics[0].arguments["dimension"],
        sico_semantics::DiagnosticArgument::Text("spawns".to_owned())
    );

    // A statically-known collect list longer than 1,024 is E5105.
    let elements = (0..1_025)
        .map(|index| format!("task_{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let text = format!(
        "async function collect() returns Int:\n  task group:\n    return await collect_tasks([{elements}], order: input)\n  end task\nend function\n"
    );
    let source =
        SourceFile::from_bytes(SourceId::new(3), "collect-limit.sico", text.as_bytes()).unwrap();
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.diagnostics.len(), 1, "{:?}", analysis.diagnostics);
    assert_eq!(analysis.diagnostics[0].code, "E5105");
    assert_eq!(
        analysis.diagnostics[0].arguments["dimension"],
        sico_semantics::DiagnosticArgument::Text("collect".to_owned())
    );
}

#[test]
fn collect_tasks_consumes_handles_once_and_reports_scoped_facts() {
    let text = "async function compute(value: Int) returns Int:\n  return value + 1\nend function\n\nasync function pair() returns List[Int]:\n  task group:\n    let first = spawn compute(1)\n    let second = spawn compute(2)\n    return await collect_tasks([first, second], order: input)\n  end task\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "collect.sico", text).unwrap();
    let analysis = analyze(&source).unwrap();
    assert!(analysis.is_success(), "{:?}", analysis.diagnostics);
    let names: Vec<_> = analysis
        .facts
        .iter()
        .filter(|fact| fact.kind == sico_semantics::SemanticFactKind::AsyncState)
        .map(|fact| fact.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "scope-0:first->pending",
            "scope-0:second->pending",
            "scope-0:first->collected",
            "scope-0:second->collected",
        ]
    );

    // A handle consumed by collect_tasks cannot be awaited again.
    let twice = "async function compute() returns Int:\n  return 1\nend function\n\nasync function pair() returns Int:\n  task group:\n    let handle = spawn compute()\n    let both = await collect_tasks([handle, handle], order: input)\n    return 0\n  end task\nend function\n";
    let source = SourceFile::from_text(SourceId::new(1), "collect-twice.sico", twice).unwrap();
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.diagnostics.len(), 1, "{:?}", analysis.diagnostics);
    assert_eq!(analysis.diagnostics[0].code, "E5101");
}

#[test]
fn borrow_across_suspension_requires_an_await_to_be_rejected() {
    // The same program without the await line is accepted.
    let without_await = "resource Reader:\n  function read(borrow self) returns Text\n  function close(self) returns Unit\nend resource\n\nfunction read_now(reader: Reader) returns Text:\n  let borrowed = borrow reader\n  return borrowed.read()\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "borrow.sico", without_await).unwrap();
    let analysis = analyze(&source).unwrap();
    assert!(analysis.is_success(), "{:?}", analysis.diagnostics);
}

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
