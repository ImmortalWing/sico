//! M22 S2: the Sico-written checker runs end-to-end. STEP-0245 closes the
//! declared identity-level diagnostic subset on all 215 frozen sources plus
//! W1 structural/zero-indent regressions, while keeping full rendered Rust
//! diagnostics outside the claim.

use sha2::{Digest, Sha256};
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits,
    ScriptInput,
};

const CHECKER_SOURCE: &str = include_str!("../../../selfhost/checker.sico");
const COMPILER_LEXER_SOURCE: &str = include_str!("../../../selfhost/compiler_lexer.sico");
const COMPILER_PARSER_SOURCE: &str = include_str!("../../../selfhost/compiler_parser.sico");
const COMPILER_SEMANTICS_SOURCE: &str = include_str!("../../../selfhost/compiler_semantics.sico");
const OK_SOURCE: &str = "function main() returns Int:\n  return 42\nend function\n";

fn canonical_source(bytes: Vec<u8>, path: &str) -> Vec<u8> {
    let text =
        String::from_utf8(bytes).unwrap_or_else(|error| panic!("non-UTF-8 source {path}: {error}"));
    let canonical = text.replace("\r\n", "\n");
    assert!(
        !canonical.contains('\r'),
        "non-CRLF carriage return: {path}"
    );
    canonical.into_bytes()
}

fn compile_checker() -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0180-checker-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("checker.sico");
    let component_path = directory.join("checker.component.wasm");
    std::fs::write(&source_path, CHECKER_SOURCE).unwrap();
    std::fs::write(directory.join("compiler_lexer.sico"), COMPILER_LEXER_SOURCE).unwrap();
    std::fs::write(
        directory.join("compiler_parser.sico"),
        COMPILER_PARSER_SOURCE,
    )
    .unwrap();
    std::fs::write(
        directory.join("compiler_semantics.sico"),
        COMPILER_SEMANTICS_SOURCE,
    )
    .unwrap();
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

fn run_checker(prepared: &PreparedProgram, stdin: &[u8]) -> RunOutcome {
    let limits = RunnerLimits {
        fuel: 5_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        ..RunnerLimits::default()
    };
    prepared
        .run(
            &ScriptInput {
                stdin: stdin.to_vec(),
                ..ScriptInput::default()
            },
            &limits,
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn sico_checker_matches_rust_on_well_formed_program() {
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
    match run_checker(&prepared, OK_SOURCE.as_bytes()) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert!(
                output.stdout.starts_with(b"check ok"),
                "expected check ok, got {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn sico_checker_catches_missing_colon_where_rust_refuses() {
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
    let bad = b"function main() returns Int\n  return 42\nend function\n";
    assert_eq!(
        run_checker(&prepared, bad),
        RunOutcome::Domain {
            code: "invalid-input".to_owned(),
            message: "E-SH-SYNTAX-FUNCTION-COLON".to_owned(),
        }
    );
}

#[test]
fn sico_checker_matches_the_complete_frozen_diagnostic_partition() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(repository.join("selfhost/corpus-v0.json")).unwrap())
            .unwrap();
    let entries = manifest["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 215);

    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
    let limits = RunnerLimits {
        fuel: 5_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        ..RunnerLimits::default()
    };
    let mut lexical = 0;
    let mut semantic = 0;
    let mut accepted = 0;
    let mut unsupported = 0;

    for entry in entries {
        let path = entry["path"].as_str().unwrap();
        let diagnostic = entry["diagnostic_ids"]
            .as_array()
            .and_then(|ids| ids.first())
            .and_then(serde_json::Value::as_str);
        let source = canonical_source(std::fs::read(repository.join(path)).unwrap(), path);
        assert_eq!(
            entry["bytes"].as_u64().unwrap(),
            u64::try_from(source.len()).unwrap(),
            "frozen source length drifted: {path}"
        );
        assert_eq!(
            entry["source_sha256"].as_str().unwrap(),
            format!("{:x}", Sha256::digest(&source)),
            "frozen source digest drifted: {path}"
        );
        let outcome = prepared
            .run(
                &ScriptInput {
                    stdin: source,
                    ..ScriptInput::default()
                },
                &limits,
                &CancelToken::new(),
            )
            .expect("frozen source fits runner input bounds");
        match diagnostic {
            Some("LEXICAL") => {
                lexical += 1;
                assert_eq!(
                    outcome,
                    RunOutcome::Domain {
                        code: "invalid-input".to_owned(),
                        message: "LEXICAL".to_owned(),
                    },
                    "{path}"
                );
            }
            Some(
                code @ ("E2001" | "E2002" | "E2010" | "E2011" | "E2020" | "E3001" | "E3002"
                | "E3003" | "E3101" | "E3102" | "E3103" | "E3104" | "E4001" | "E4002"
                | "E5001" | "E5002" | "E5003" | "E5101" | "E5102" | "E5103" | "E5104"
                | "E5105" | "E5201" | "E5202" | "E6001" | "E6002" | "E7001" | "E7002"
                | "E8010"),
            ) => {
                semantic += 1;
                assert_eq!(
                    outcome,
                    RunOutcome::Domain {
                        code: "invalid-input".to_owned(),
                        message: code.to_owned(),
                    },
                    "{path}"
                );
            }
            _ if entry["rust_check"] == "accepted" => {
                accepted += 1;
                let RunOutcome::Output(output) = outcome else {
                    panic!("accepted source was refused: {path}: {outcome:?}")
                };
                assert!(output.stdout.starts_with(b"check ok"), "{path}");
            }
            _ => {
                unsupported += 1;
                assert!(
                    matches!(outcome, RunOutcome::Output(_)),
                    "unsupported semantic source must remain open: {path}: {outcome:?}"
                );
            }
        }
    }

    assert_eq!((lexical, semantic, accepted, unsupported), (116, 34, 65, 0));
}

#[test]
fn sico_checker_matches_all_frozen_e1xxx_mutation_identities() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases = [
        ("MUT-001-missing-function-close.sico", "E1001"),
        ("MUT-002-missing-match-arm-separator.sico", "E1002"),
        ("MUT-003-missing-record-close.sico", "E1003"),
        ("MUT-004-missing-type-argument-close.sico", "E1004"),
        ("MUT-005-missing-enum-close.sico", "E1005"),
        ("MUT-006-missing-call-close.sico", "E1006"),
        ("MUT-007-missing-capability-close.sico", "E1007"),
        ("MUT-008-missing-resource-close.sico", "E1008"),
        ("MUT-009-missing-using-close.sico", "E1009"),
        ("MUT-010-missing-task-group-close.sico", "E1010"),
        ("MUT-011-missing-interface-close.sico", "E1011"),
        ("MUT-012-missing-parameter-list-close.sico", "E1012"),
    ];
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");

    for (file, code) in cases {
        let source = std::fs::read(repository.join("syntax-mutations/b").join(file)).unwrap();
        assert_eq!(
            run_checker(&prepared, &source),
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: code.to_owned(),
            },
            "{file}"
        );
    }
}

#[test]
fn sico_checker_matches_remaining_e1xxx_shape_identities() {
    let cases = [
        (
            b"stdout.write(stdin.read_all())\nfunction main() returns Int:\n  return 1\nend function\n".as_slice(),
            "E1013",
        ),
        (b"module math extra\n".as_slice(), "E1014"),
        (b"use math_util\n".as_slice(), "E1015"),
        (
            b"interface Bad version 0:\nend interface\n".as_slice(),
            "E1016",
        ),
    ];
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");

    for (source, code) in cases {
        assert_eq!(
            run_checker(&prepared, source),
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: code.to_owned(),
            },
            "{code}"
        );
    }
}

#[test]
fn sico_checker_matches_frozen_e2xxx_semantic_identities() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases = [
        (
            "syntax-candidates/b/nominal-invariants/invalid/nominal-confusion.sico",
            "E2001",
        ),
        (
            "syntax-candidates/b/nominal-invariants/invalid/structural-record-confusion.sico",
            "E2001",
        ),
        (
            "syntax-candidates/b/numbers-units/invalid/cross-currency.sico",
            "E2001",
        ),
        (
            "syntax-candidates/b/numbers-units/invalid/mixed-unit.sico",
            "E2001",
        ),
        (
            "syntax-candidates/b/numbers-units/invalid/text-as-int.sico",
            "E2001",
        ),
        (
            "syntax-candidates/b/numbers-units/invalid/implicit-int-to-float.sico",
            "E2002",
        ),
        (
            "syntax-candidates/b/nominal-invariants/invalid/missing-field.sico",
            "E2010",
        ),
        (
            "syntax-candidates/b/nominal-invariants/invalid/unknown-field.sico",
            "E2011",
        ),
        (
            "syntax-candidates/b/nominal-invariants/invalid/invariant-violation.sico",
            "E2020",
        ),
    ];
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");

    for (path, code) in cases {
        let source = std::fs::read(repository.join(path)).unwrap();
        assert_eq!(
            run_checker(&prepared, &source),
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: code.to_owned(),
            },
            "{path}"
        );
    }
}

#[test]
fn sico_checker_matches_frozen_e3xxx_match_and_result_identities() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases = [
        (
            "syntax-candidates/b/exhaustive-match/invalid/missing-variant.sico",
            "E3001",
        ),
        (
            "syntax-candidates/b/exhaustive-match/invalid/wildcard-sealed-enum.sico",
            "E3002",
        ),
        (
            "syntax-candidates/b/exhaustive-match/invalid/wildcard-sealed-product.sico",
            "E3002",
        ),
        (
            "syntax-candidates/b/exhaustive-match/invalid/unreachable-branch.sico",
            "E3003",
        ),
        (
            "syntax-candidates/b/result-mapping/invalid/error-type-mismatch.sico",
            "E3101",
        ),
        (
            "syntax-candidates/b/result-mapping/invalid/incomplete-error-map.sico",
            "E3102",
        ),
        (
            "syntax-candidates/b/result-mapping/invalid/catch-all-error-map.sico",
            "E3103",
        ),
        (
            "syntax-candidates/b/result-mapping/invalid/unhandled-result.sico",
            "E3104",
        ),
    ];
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");

    for (path, code) in cases {
        let source = std::fs::read(repository.join(path)).unwrap();
        assert_eq!(
            run_checker(&prepared, &source),
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: code.to_owned(),
            },
            "{path}"
        );
    }
}

#[test]
fn sico_checker_matches_remaining_frozen_semantic_and_module_identities() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases = [
        (
            "syntax-candidates/b/effects-capabilities/invalid/undeclared-capability.sico",
            "E4001",
        ),
        (
            "syntax-candidates/b/effects-capabilities/invalid/undeclared-effect.sico",
            "E4002",
        ),
        (
            "syntax-candidates/b/affine-resources/invalid/use-after-move.sico",
            "E5001",
        ),
        (
            "syntax-candidates/b/affine-resources/invalid/use-after-close.sico",
            "E5002",
        ),
        (
            "syntax-candidates/b/affine-resources/invalid/borrow-across-await.sico",
            "E5003",
        ),
        (
            "syntax-candidates/b/future-task/invalid/await-twice.sico",
            "E5101",
        ),
        (
            "syntax-candidates/b/future-task/invalid/task-escapes-scope.sico",
            "E5102",
        ),
        (
            "syntax-candidates/b/future-task/invalid/uncollected-task.sico",
            "E5103",
        ),
        (
            "syntax-candidates/b/future-task/invalid/detached-spawn.sico",
            "E5104",
        ),
        (
            "syntax-candidates/b/future-task/invalid/scope-nesting-limit.sico",
            "E5105",
        ),
        (
            "syntax-candidates/b/stream/invalid/unbounded-collect.sico",
            "E5201",
        ),
        (
            "syntax-candidates/b/stream/invalid/missing-await.sico",
            "E5202",
        ),
        (
            "syntax-candidates/b/component-call/invalid/version-as-type.sico",
            "E6001",
        ),
        (
            "syntax-candidates/b/component-call/invalid/trap-as-domain-error.sico",
            "E6002",
        ),
        (
            "syntax-candidates/b/revision/invalid/missing-commit-revision.sico",
            "E7001",
        ),
        (
            "syntax-candidates/b/revision/invalid/unchecked-stale-result.sico",
            "E7002",
        ),
        ("tests/end-to-end/modules-report/math_util.sico", "E8010"),
    ];
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");

    for (path, code) in cases {
        let source = std::fs::read(repository.join(path)).unwrap();
        assert_eq!(
            run_checker(&prepared, &source),
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: code.to_owned(),
            },
            "{path}"
        );
    }
}

#[test]
fn sico_checker_matches_w1_revision_and_zero_indent_corpus() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases = [
        (
            "checked-stale-result-renamed.sico",
            "65944dbaf8e9627fef372fc82a6f2a48c4dcbb628c84cd7ad873d2eb1d95b43b",
            None,
        ),
        (
            "unchecked-after-guard.sico",
            "fa1a8ea097120396866be16e55c557fd76b660b51580e97e46c7584752776f15",
            Some("E7002"),
        ),
        (
            "unchecked-stale-result-renamed.sico",
            "ab29e491e2bb6491036a13ab0f9ca4667edf6b19b98f2b2ab89bd87149203ec3",
            Some("E7002"),
        ),
        (
            "unchecked-stale-result-zero-indent.sico",
            "3b1e7cfdd52bfee0f7c3159a4f3d8d369d306af8bc5b9ef8cc83c8186e3374cb",
            Some("E7002"),
        ),
        (
            "zero-indent-function-body.sico",
            "557780cd8e840143f0e1e0ac3cc6c355fd9b06c8fad5a97d1ad38c9299da4bb2",
            None,
        ),
    ];
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");

    for (file, source_sha256, expected) in cases {
        let path = repository.join("selfhost/corpus-w1").join(file);
        let source = canonical_source(std::fs::read(&path).unwrap(), file);
        assert_eq!(
            format!("{:x}", Sha256::digest(&source)),
            source_sha256,
            "W1 source digest drifted: {file}"
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let rust_exit = sico_cli::run(
            [
                std::ffi::OsString::from("sico"),
                std::ffi::OsString::from("check"),
                std::ffi::OsString::from("--json"),
                path.as_os_str().to_owned(),
            ],
            &mut std::io::empty(),
            &mut stdout,
            &mut stderr,
        );
        let report: serde_json::Value = serde_json::from_slice(&stdout)
            .unwrap_or_else(|error| panic!("Rust oracle JSON for {file}: {error}: {stderr:?}"));
        let actual = report["diagnostics"]
            .as_array()
            .and_then(|diagnostics| diagnostics.first())
            .and_then(|diagnostic| diagnostic["code"].as_str());
        assert_eq!(actual, expected, "Rust oracle: {file}");
        assert_eq!(
            rust_exit,
            i32::from(expected.is_some()),
            "Rust exit: {file}"
        );

        let outcome = run_checker(&prepared, &source);
        if let Some(code) = expected {
            assert_eq!(
                outcome,
                RunOutcome::Domain {
                    code: "invalid-input".to_owned(),
                    message: code.to_owned(),
                },
                "guest: {file}"
            );
        } else {
            let RunOutcome::Output(output) = outcome else {
                panic!("guest accepted source was refused: {file}: {outcome:?}");
            };
            assert!(output.stdout.starts_with(b"check ok"), "guest: {file}");
        }
    }
}
