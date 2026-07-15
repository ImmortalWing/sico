use std::{env, fs, path::Path, time::Instant};

use serde_json::json;
use sico_semantics::analyze;
use sico_source::{SourceFile, SourceId};

fn main() {
    let arguments: Vec<_> = env::args().collect();
    let root = Path::new(
        arguments
            .get(1)
            .map_or("syntax-candidates/b", String::as_str),
    );
    let iterations: usize = arguments
        .get(2)
        .map_or(Ok(200), |value| value.parse())
        .expect("iterations must be an integer");
    let mut paths = Vec::new();
    collect_sico(root, &mut paths);
    paths.sort();
    let sources: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).expect("corpus is bounded")),
                path.display().to_string(),
                &fs::read(path).expect("corpus source must be readable"),
            )
            .expect("corpus source must satisfy source contract")
        })
        .collect();
    let bytes_per_iteration: usize = sources.iter().map(|source| source.text().len()).sum();
    let start = Instant::now();
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    for _ in 0..iterations {
        for source in &sources {
            let analysis = analyze(source).expect("B corpus must lower");
            if analysis.is_success() {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }
    }
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1_000.0;
    let total_bytes = bytes_per_iteration * iterations;
    let total_bytes_float = f64::from(u32::try_from(total_bytes).expect("measurement is bounded"));
    let mib_per_second = total_bytes_float / 1_048_576.0 / elapsed.as_secs_f64();
    println!(
        "{}",
        serde_json::to_string(&json!({
            "files": sources.len(),
            "bytes_per_iteration": bytes_per_iteration,
            "iterations": iterations,
            "total_analyses": sources.len() * iterations,
            "accepted": accepted,
            "rejected": rejected,
            "elapsed_ms": (elapsed_ms * 1_000.0).round() / 1_000.0,
            "mib_per_second": (mib_per_second * 1_000.0).round() / 1_000.0
        }))
        .expect("measurement JSON serialization cannot fail")
    );
}

fn collect_sico(root: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("corpus directory must be readable") {
        let path = entry.expect("corpus entry must be readable").path();
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
