use serde::Deserialize;
use sha2::{Digest, Sha256};
use sico_runner::bootstrap::{
    MAX_SOURCE_BUNDLE_BYTES, MAX_SOURCE_BUNDLE_FILES, MAX_SOURCE_FILE_BYTES, SourceBundleEntry,
    SourceBundleError, decode_source_bundle, encode_source_bundle,
};

fn entry(path: &str, source: &[u8]) -> SourceBundleEntry {
    SourceBundleEntry {
        path: path.to_owned(),
        source: source.to_vec(),
    }
}

#[test]
fn canonical_bundle_round_trips_byte_exact() {
    let entries = vec![
        entry("selfhost/checker.sico", b"checker\n"),
        entry("selfhost/compiler.sico", b"compiler\n"),
    ];
    let encoded = encode_source_bundle(&entries).unwrap();
    assert_eq!(decode_source_bundle(&encoded).unwrap(), entries);
    assert_eq!(
        encode_source_bundle(&decode_source_bundle(&encoded).unwrap()).unwrap(),
        encoded
    );
}

#[test]
fn malformed_bundles_fail_closed() {
    assert_eq!(encode_source_bundle(&[]), Err(SourceBundleError::Empty));
    for path in [
        "/absolute.sico",
        "C:/drive.sico",
        "a\\b.sico",
        "a//b.sico",
        "a/./b.sico",
        "a/../b.sico",
        "a/",
        "a\0b.sico",
    ] {
        assert_eq!(
            encode_source_bundle(&[entry(path, b"x")]),
            Err(SourceBundleError::InvalidPath),
            "{path:?}"
        );
    }
    assert_eq!(
        encode_source_bundle(&[entry("b.sico", b"b"), entry("a.sico", b"a")]),
        Err(SourceBundleError::UnsortedPath)
    );
    assert_eq!(
        encode_source_bundle(&[entry("a.sico", b"a"), entry("a.sico", b"b")]),
        Err(SourceBundleError::DuplicatePath)
    );

    let encoded = encode_source_bundle(&[entry("a.sico", b"source")]).unwrap();
    assert_eq!(
        decode_source_bundle(&encoded[..encoded.len() - 1]),
        Err(SourceBundleError::Truncated)
    );
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert_eq!(
        decode_source_bundle(&trailing),
        Err(SourceBundleError::TrailingData)
    );
    let mut bad_magic = encoded.clone();
    bad_magic[0] ^= 1;
    assert_eq!(
        decode_source_bundle(&bad_magic),
        Err(SourceBundleError::BadMagic)
    );
    let mut bad_digest = encoded;
    *bad_digest.last_mut().unwrap() ^= 1;
    assert_eq!(
        decode_source_bundle(&bad_digest),
        Err(SourceBundleError::DigestMismatch)
    );
}

#[test]
fn every_adr_limit_and_limit_plus_one_is_exact() {
    let max_files: Vec<_> = (0..MAX_SOURCE_BUNDLE_FILES)
        .map(|index| entry(&format!("f/{index:03}.sico"), b""))
        .collect();
    assert!(encode_source_bundle(&max_files).is_ok());
    let too_many: Vec<_> = (0..=MAX_SOURCE_BUNDLE_FILES)
        .map(|index| entry(&format!("f/{index:03}.sico"), b""))
        .collect();
    assert_eq!(
        encode_source_bundle(&too_many),
        Err(SourceBundleError::TooManyFiles)
    );

    assert!(encode_source_bundle(&[entry("max.sico", &vec![0; MAX_SOURCE_FILE_BYTES])]).is_ok());
    assert_eq!(
        encode_source_bundle(&[entry("large.sico", &vec![0; MAX_SOURCE_FILE_BYTES + 1])]),
        Err(SourceBundleError::FileTooLarge)
    );

    let exact_total: Vec<_> = (0..4)
        .map(|index| {
            entry(
                &format!("total/{index}.sico"),
                &vec![0; MAX_SOURCE_BUNDLE_BYTES / 4],
            )
        })
        .collect();
    assert!(encode_source_bundle(&exact_total).is_ok());
    let mut over_total = exact_total;
    over_total.push(entry("total/4.sico", b"x"));
    assert_eq!(
        encode_source_bundle(&over_total),
        Err(SourceBundleError::BundleTooLarge)
    );
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusManifest {
    schema: String,
    roots: Vec<String>,
    entry_count: usize,
    entries: Vec<CorpusEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusEntry {
    path: String,
    bytes: usize,
    source_sha256: String,
    rust_format: String,
    formatted_sha256: Option<String>,
    format_idempotent: bool,
    rust_check: String,
    rust_check_exit: i32,
    diagnostic_ids: Vec<String>,
    diagnostic_sha256: String,
}

#[test]
fn frozen_corpus_manifest_is_complete_and_forms_a_canonical_bundle() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest_bytes = std::fs::read(repository.join("selfhost/corpus-v0.json")).unwrap();
    let manifest: CorpusManifest = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest.schema, "sico.m22.corpus.v0");
    assert_eq!(
        manifest.roots,
        ["semantic-cases", "syntax-candidates", "tests/end-to-end"]
    );
    assert_eq!(manifest.entry_count, 215);
    assert_eq!(manifest.entries.len(), manifest.entry_count);

    let mut discovered = Vec::new();
    for root in &manifest.roots {
        collect_sico(&repository.join(root), &repository, &mut discovered);
    }
    discovered.sort();
    let declared: Vec<_> = manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    assert_eq!(
        declared, discovered,
        "manifest must name every frozen source"
    );

    let mut bundle = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        let source = std::fs::read(repository.join(&entry.path)).unwrap();
        assert_eq!(entry.bytes, source.len(), "{}", entry.path);
        assert_eq!(entry.source_sha256, hex(&Sha256::digest(&source)));
        assert!(matches!(entry.rust_format.as_str(), "accepted" | "refused"));
        assert!(matches!(entry.rust_check.as_str(), "accepted" | "refused"));
        assert_eq!(
            entry.rust_format == "accepted",
            entry.formatted_sha256.is_some()
        );
        assert_eq!(entry.rust_format == "accepted", entry.format_idempotent);
        assert_eq!(entry.rust_check == "accepted", entry.rust_check_exit == 0);
        assert!(!entry.diagnostic_sha256.is_empty());
        if entry.rust_check == "refused" {
            assert!(!entry.diagnostic_ids.is_empty(), "{}", entry.path);
        }
        bundle.push(SourceBundleEntry {
            path: entry.path.clone(),
            source,
        });
    }
    let encoded = encode_source_bundle(&bundle).unwrap();
    assert_eq!(decode_source_bundle(&encoded).unwrap(), bundle);
    assert_eq!(
        encode_source_bundle(&decode_source_bundle(&encoded).unwrap()).unwrap(),
        encoded
    );
}

fn collect_sico(root: &std::path::Path, repository: &std::path::Path, output: &mut Vec<String>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_sico(&path, repository, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "sico")
        {
            output.push(
                path.strip_prefix(repository)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut value, byte| {
        use std::fmt::Write as _;
        write!(value, "{byte:02x}").unwrap();
        value
    })
}
