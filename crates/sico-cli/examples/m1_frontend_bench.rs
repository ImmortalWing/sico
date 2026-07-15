use std::{env, fs, path::Path, time::Instant};

use serde_json::json;
use sico_format::format;
use sico_parser::parse;
use sico_source::{SourceFile, SourceId};

fn main() {
    let iterations: usize = env::args()
        .nth(1)
        .unwrap_or_else(|| "200".to_owned())
        .parse()
        .expect("iterations must be a positive integer");
    assert!(iterations > 0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    collect_sico(&root.join("syntax-candidates/b"), &mut paths);
    collect_sico(&root.join("syntax-mutations/b"), &mut paths);
    paths.sort();
    assert_eq!(paths.len(), 66);
    let inputs: Vec<_> = paths
        .iter()
        .map(|path| (path.display().to_string(), fs::read(path).unwrap()))
        .collect();
    let bytes_per_iteration: usize = inputs.iter().map(|(_, bytes)| bytes.len()).sum();

    let started = Instant::now();
    let mut success = 0_u64;
    let mut failure = 0_u64;
    for iteration in 0..iterations {
        for (index, (name, bytes)) in inputs.iter().enumerate() {
            let source =
                SourceFile::from_bytes(SourceId::new(u32::try_from(index).unwrap()), name, bytes)
                    .unwrap();
            let parsed = parse(&source);
            if parsed.is_success() {
                let formatted = format(&source).unwrap();
                assert!(!formatted.is_empty());
                success += 1;
            } else {
                assert_eq!(parsed.errors().len(), 1);
                failure += 1;
            }
        }
        assert_eq!(success, u64::try_from((iteration + 1) * 54).unwrap());
        assert_eq!(failure, u64::try_from((iteration + 1) * 12).unwrap());
    }
    let elapsed = started.elapsed();
    let total_bytes = bytes_per_iteration * iterations;
    let measured_bytes = u32::try_from(total_bytes).expect("benchmark byte count fits u32");
    let mib_per_second = f64::from(measured_bytes) / elapsed.as_secs_f64() / (1024.0 * 1024.0);
    println!(
        "{}",
        serde_json::to_string(&json!({
            "schema": "sico.m1.frontend-performance.v0",
            "corpus_files": 66,
            "canonical_files": 54,
            "mutation_files": 12,
            "iterations": iterations,
            "bytes_per_iteration": bytes_per_iteration,
            "total_bytes": total_bytes,
            "elapsed_ms": elapsed.as_millis(),
            "mib_per_second": (mib_per_second * 1000.0).round() / 1000.0,
            "successful_formats": success,
            "diagnostic_parses": failure,
            "scope": "source+lex+parse; format on successful parse",
            "sla": "not-established"
        }))
        .unwrap()
    );
}

fn collect_sico(root: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_sico(&path, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "sico")
        {
            output.push(path);
        }
    }
}
