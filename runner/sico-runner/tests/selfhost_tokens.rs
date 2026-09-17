//! M22 lossless lexer differential. The Sico implementation emits a bounded
//! kind/start/end/length/raw-text frame for every token, including trivia and
//! EOF. The Rust lexer is the permanent oracle.

use sico_lexer::lex;
use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use sico_source::{SourceFile, SourceId};

const TOKENS_SOURCE: &str = include_str!("../../../selfhost/tokens.sico");

fn compile_tokens() -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0202-tokens-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("tokens.sico");
    let component_path = directory.join("tokens.component.wasm");
    std::fs::write(&source_path, TOKENS_SOURCE).unwrap();
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

#[derive(Debug, Eq, PartialEq)]
struct GuestToken {
    kind: String,
    start: usize,
    end: usize,
    text: Vec<u8>,
}

fn decode(mut bytes: &[u8]) -> Vec<GuestToken> {
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
fn sico_lexer_is_lossless_and_matches_rust_on_frozen_corpus() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(repository.join("selfhost/corpus-v0.json")).unwrap())
            .unwrap();
    let paths: Vec<_> = manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(paths.len(), 215);

    let component = compile_tokens();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("token component links");
    let limits = RunnerLimits {
        fuel: 1_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        memory_bytes: 512 * 1024 * 1024,
        ..RunnerLimits::default()
    };

    for (index, path) in paths.iter().enumerate() {
        let source_bytes = std::fs::read(repository.join(path)).unwrap();
        let source = SourceFile::from_bytes(
            SourceId::new(u32::try_from(index).unwrap()),
            path,
            &source_bytes,
        )
        .unwrap();
        // The v0 Sico collection primitives are copy-on-write. Run one
        // newline-terminated lexical segment at a time so the differential is
        // bounded independently of file length; no token class can cross a
        // physical newline under RFC-0006.
        let mut actual = Vec::new();
        let mut base = 0usize;
        for chunk in source_bytes.split_inclusive(|byte| *byte == b'\n') {
            let outcome = prepared
                .run(
                    &ScriptInput {
                        stdin: chunk.to_vec(),
                        ..ScriptInput::default()
                    },
                    &limits,
                    &CancelToken::new(),
                )
                .expect("input bounds hold");
            let RunOutcome::Output(output) = outcome else {
                panic!("Sico lexer failed for {path}@{base}: {outcome:?}")
            };
            assert_eq!(output.exit_code, 0, "{path}: {:?}", output.stderr);
            let mut framed = decode(&output.stdout);
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
        let expected = lex(&source);
        assert_eq!(actual.len(), expected.tokens().len(), "{path}");
        for (actual, expected) in actual.iter().zip(expected.tokens()) {
            let start = usize::from(expected.range.start());
            let end = usize::from(expected.range.end());
            assert_eq!(actual.kind, format!("{:?}", expected.kind), "{path}");
            assert_eq!((actual.start, actual.end), (start, end), "{path}");
            assert_eq!(actual.text, source_bytes[start..end], "{path}");
        }
        let reconstructed: Vec<_> = actual
            .iter()
            .filter(|token| token.kind != "Eof")
            .flat_map(|token| token.text.iter().copied())
            .collect();
        assert_eq!(reconstructed, source_bytes, "{path}");
    }
}
