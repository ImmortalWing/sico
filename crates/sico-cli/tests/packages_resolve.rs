//! RFC-0039 §4.2 (STEP-0147): the package consumer exit fixture — a
//! source-level consumer imports user WIT interfaces from versioned,
//! signed packages through the lock and runs through the real Component
//! Runtime. Positive path for the E8016 stamp gate (digest agreement
//! between the producer stamp and the source-declared interface).

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use sha2::{Digest, Sha256};
use sico_codegen_wasm::package_builder::{csv_package, table_stats_package};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sign_development};

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

const CSV_CONSUMER: &str = "\
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

interface Csv version 1:
  function parse_line(line: Text) returns Result[List[Text], Text]
end interface

use pkg csv version 1 expose Csv

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let line = sico.bytes.utf8_decode(input.stdin)
  match csv.parse_line(line):
    case ok(fields):
      let joined = sico.text.join(fields, \"|\")
      return ok(ScriptOutput(stdout: sico.text.encode(joined), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))
    case error(reason):
      return ok(ScriptOutput(stdout: sico.text.encode(\"\"), stderr: sico.text.encode(reason), exit_code: I64.literal(10)))
  end match
end function
";

const STATS_CONSUMER: &str = "
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

interface Csv version 1:
  function parse_line(line: Text) returns Result[List[Text], Text]
end interface

interface TableStats version 1:
  function aggregate(values: List[Text], op: Text) returns Result[List[Text], Text]
end interface

use pkg csv version 1 expose Csv
use pkg table_stats version 1 expose TableStats

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let line = sico.bytes.utf8_decode(input.stdin)
  match csv.parse_line(line):
    case ok(fields):
      match table_stats.aggregate(fields, \"count\"):
        case ok(counts):
          match sico.list.get(counts, U64.literal(0)):
            case ok(count):
              match table_stats.aggregate(fields, \"mean\"):
                case ok(means):
                  match sico.list.get(means, U64.literal(0)):
                    case ok(mean):
                      return ok(ScriptOutput(stdout: sico.text.encode(sico.text.concat(sico.text.concat(\"count=\", count), sico.text.concat(\" mean=\", mean))), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))
                    case error(_):
                      return fail_reason()
                  end match
                case error(_):
                  return fail_reason()
              end match
            case error(_):
              return fail_reason()
          end match
        case error(_):
          return fail_reason()
      end match
    case error(_):
      return fail_reason()
  end match
end function

function fail_reason() returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: sico.text.encode(\"\"), stderr: sico.text.encode(\"aggregate failed\"), exit_code: I64.literal(10)))
end function
";

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sico-pkgresolve-{tag}-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sapp(component: Vec<u8>, app_id: &str) -> Vec<u8> {
    let unsigned = build_unsigned(BuildInput {
        app_id: app_id.to_owned(),
        app_version: "1.0.0".to_owned(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("package builds");
    sign_development(&unsigned, &[7_u8; 32]).expect("development signature applies")
}

fn write_lock(dir: &Path, entries: &[(&str, Vec<u8>)]) {
    let mut parts = Vec::new();
    for (name, bytes) in entries {
        let digest = format!("{:x}", Sha256::digest(bytes));
        parts.push(format!(
            "{{\"name\":\"{name}\",\"version\":1,\"path\":\"pkgs/{name}.sapp\",\"sha256\":\"{digest}\"}}"
        ));
        fs::write(dir.join("pkgs").join(format!("{name}.sapp")), bytes).unwrap();
    }
    let lock = format!(
        "{{\"schema\":\"sico:source-lock:v0\",\"packages\":[{}]}}",
        parts.join(",")
    );
    fs::write(dir.join("sico-lock.json"), lock).unwrap();
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_sico")
}

fn run<const N: usize>(args: [&str; N], stdin: &[u8]) -> Output {
    // Guest stdin rides a temp file: a piped stdin interacted badly with
    // the CLI's inherited-stdio runner hand-off under capture.
    let stdin_path = std::env::temp_dir().join(format!(
        "sico-pkgresolve-stdin-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&stdin_path, stdin).unwrap();
    let stdin_file = fs::File::open(&stdin_path).unwrap();
    let mut command = Command::new(binary());
    command
        .args(args)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if std::env::var_os("SICO_RUNNER").is_none() {
        let runner = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../runner/sico-runner/target/debug/sico-runner.exe");
        if runner.is_file() {
            command.env("SICO_RUNNER", runner);
        }
    }
    let output = command.output().unwrap();
    let _ = fs::remove_file(&stdin_path);
    output
}

#[test]
fn csv_package_consumer_runs_end_to_end() {
    let dir = temp_root("csv");
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    let csv = csv_package();
    write_lock(&dir, &[("csv", sapp(csv, "pkg.csv"))]);
    fs::write(dir.join("main.sico"), CSV_CONSUMER).unwrap();

    let output = run(
        ["run", &dir.join("main.sico").display().to_string()],
        b"alpha,\"be,ta\",\"gam\"\"ma\"",
    );
    eprintln!(
        "CSV_TEST rc={:?} out={:?} err={:?}",
        output.status.code(),
        output.stdout,
        output.stderr
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {} stdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(&output.stdout, b"alpha|be,ta|gam\"ma");
}

#[test]
fn stats_package_consumer_runs_end_to_end() {
    let dir = temp_root("stats");
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    let csv = csv_package();
    let stats = table_stats_package();
    write_lock(
        &dir,
        &[
            ("csv", sapp(csv, "pkg.csv")),
            ("table_stats", sapp(stats, "pkg.table-stats")),
        ],
    );
    fs::write(dir.join("main.sico"), STATS_CONSUMER).unwrap();

    let output = run(
        ["run", &dir.join("main.sico").display().to_string()],
        b"4,10,-2
",
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(&output.stdout, b"count=3 mean=4000");
}

#[test]
fn tampered_package_fails_digest_or_shape_gate() {
    let dir = temp_root("stamp");
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    let csv = csv_package();
    let sapp_bytes = sapp(csv, "pkg.csv");
    // The lock pins the pristine bytes; the artifact on disk is tampered.
    let mut tampered = sapp_bytes.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xFF;
    fs::write(dir.join("pkgs/csv.sapp"), tampered).unwrap();
    let digest = format!("{:x}", Sha256::digest(sapp_bytes.as_slice()));
    let lock = format!(
        "{{\"schema\":\"sico:source-lock:v0\",\"packages\":[{{\"name\":\"csv\",\"version\":1,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"{digest}\"}}]}}"
    );
    fs::write(dir.join("sico-lock.json"), lock).unwrap();
    fs::write(dir.join("main.sico"), CSV_CONSUMER).unwrap();

    let output = run(["check", &dir.join("main.sico").display().to_string()], b"");
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E8013") || stderr.contains("E8016"),
        "{stderr}"
    );
}
