//! M22 bounded semantic AST differential. The executable parser emits the
//! exact `ModuleAst::shape()` declaration tree consumed by later lowering.

use sico_parser::parse;
use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use sico_source::{SourceFile, SourceId};

const PARSER_SOURCE: &str = include_str!("../../../selfhost/declaration_parser.sico");

fn compile_parser() -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!(
        "sico-step0216-declaration-parser-{}",
        std::process::id()
    ));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("parser.sico");
    let component_path = directory.join("parser.component.wasm");
    std::fs::write(&source_path, PARSER_SOURCE).unwrap();
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
}

#[test]
fn sico_parser_matches_rust_module_ast_on_every_accepted_source() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(repository.join("selfhost/corpus-v0.json")).unwrap())
            .unwrap();
    let accepted: Vec<_> = manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["rust_format"] == "accepted")
        .map(|entry| entry["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(accepted.len(), 99);

    let component = compile_parser();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("parser component links");
    let limits = RunnerLimits {
        fuel: 1_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        ..RunnerLimits::default()
    };

    for (index, path) in accepted.iter().enumerate() {
        let bytes = std::fs::read(repository.join(path)).unwrap();
        let source =
            SourceFile::from_bytes(SourceId::new(u32::try_from(index).unwrap()), path, &bytes)
                .unwrap();
        let parsed = parse(&source);
        let expected = parsed.ast().unwrap_or_else(|| {
            panic!("manifest says formatter accepted but parser refused {path}")
        });
        let outcome = prepared
            .run(
                &ScriptInput {
                    stdin: bytes,
                    ..ScriptInput::default()
                },
                &limits,
                &CancelToken::new(),
            )
            .expect("input bounds hold");
        let RunOutcome::Output(output) = outcome else {
            panic!("Sico parser failed for {path}: {outcome:?}")
        };
        assert_eq!(output.exit_code, 0, "{path}: {:?}", output.stderr);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected.shape(),
            "{path}"
        );

        let expected_metadata = expected
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
        let metadata_outcome = prepared
            .run(
                &ScriptInput {
                    arguments: vec!["--metadata".to_owned()],
                    stdin: std::fs::read(repository.join(path)).unwrap(),
                },
                &limits,
                &CancelToken::new(),
            )
            .expect("input bounds hold");
        let RunOutcome::Output(metadata_output) = metadata_outcome else {
            panic!("Sico parser metadata failed for {path}: {metadata_outcome:?}")
        };
        assert_eq!(
            metadata_output.exit_code, 0,
            "{path}: {:?}",
            metadata_output.stderr
        );
        assert_eq!(
            String::from_utf8(metadata_output.stdout).unwrap(),
            expected_metadata,
            "{path} metadata"
        );
    }
}
