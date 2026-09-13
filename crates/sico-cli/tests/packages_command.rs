//! RFC-0039 §2.3/§2.4 (STEP-0147): CLI-level package resolution — the
//! fail-closed E80xx gate for `use pkg` declarations. Positive-path
//! (stamp-matching) evidence lives in the `packages_resolve` unit tests
//! and the STEP-0149 clean-room consumer; these fixtures cover every
//! refusal identity that fires before artifact-bound checks.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

const ENTRY_HEAD: &str = "\
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

";

const INTERFACE: &str = "\
interface Csv version 1:
  function parse_line(line: Text) returns Result[List[Text], Text]
end interface

";

const ENTRY_TAIL: &str = "\
function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: sico.text.encode(\"\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))
end function
";

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sico-packages-{tag}-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_sico")
}

fn run<const N: usize>(args: [&str; N]) -> Output {
    let mut command = Command::new(binary());
    command.args(args).stdin(Stdio::null());
    if std::env::var_os("SICO_RUNNER").is_none() {
        let runner = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../runner/sico-runner/target/debug/sico-runner.exe");
        if runner.is_file() {
            command.env("SICO_RUNNER", runner);
        }
    }
    command.output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn check(entry: &Path) -> Output {
    run(["check", &entry.display().to_string()])
}

#[test]
#[allow(clippy::too_many_lines)] // one test per refusal identity, listed in order
fn package_refusal_corpus() {
    // E8013: no lock file at all — acquisition is an explicit owner step.
    let dir = temp_root("no-lock");
    write(
        &dir.join("main.sico"),
        &format!("{ENTRY_HEAD}{INTERFACE}use pkg csv version 1 expose Csv\n\n{ENTRY_TAIL}"),
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8013"), "{}", stderr(&output));

    // E8012: lock present, package missing from it.
    let dir = temp_root("unknown-package");
    write(
        &dir.join("main.sico"),
        &format!("{ENTRY_HEAD}{INTERFACE}use pkg csv version 1 expose Csv\n\n{ENTRY_TAIL}"),
    );
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[]}",
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8012"), "{}", stderr(&output));

    // E8014: the lock pins a different version than the use requires.
    let dir = temp_root("version");
    write(
        &dir.join("main.sico"),
        &format!("{ENTRY_HEAD}{INTERFACE}use pkg csv version 1 expose Csv\n\n{ENTRY_TAIL}"),
    );
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[{\"name\":\"csv\",\"version\":2,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"00\"}]}",
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8014"), "{}", stderr(&output));

    // E8013: artifact digest mismatch (fail-closed before verification).
    let dir = temp_root("digest");
    write(
        &dir.join("main.sico"),
        &format!("{ENTRY_HEAD}{INTERFACE}use pkg csv version 1 expose Csv\n\n{ENTRY_TAIL}"),
    );
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    write(&dir.join("pkgs/csv.sapp"), "not-a-package");
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[{\"name\":\"csv\",\"version\":1,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"deadbeef\"}]}",
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8013"), "{}", stderr(&output));

    // E8018: the package name collides with a module name.
    let dir = temp_root("collision");
    write(
        &dir.join("main.sico"),
        &format!(
            "{ENTRY_HEAD}{INTERFACE}use pkg csv version 1 expose Csv\nuse csv.parse_line\n\n{ENTRY_TAIL}"
        ),
    );
    write(
        &dir.join("csv.sico"),
        "module csv\n\nfunction parse_line(line: Text) returns Text:\n  return line\nend function\n",
    );
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[{\"name\":\"csv\",\"version\":1,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"00\"}]}",
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8018"), "{}", stderr(&output));

    // E8020: expose names an interface the file does not declare.
    let dir = temp_root("unknown-interface");
    write(
        &dir.join("main.sico"),
        &format!("{ENTRY_HEAD}use pkg csv version 1 expose Missing\n\n{ENTRY_TAIL}"),
    );
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[{\"name\":\"csv\",\"version\":1,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"00\"}]}",
    );
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    write(&dir.join("pkgs/csv.sapp"), "placeholder");
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[{\"name\":\"csv\",\"version\":1,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"00\"}]}",
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8020"), "{}", stderr(&output));

    // E8017: the exposed interface carries a type outside the v0 set.
    let dir = temp_root("unsupported-type");
    write(
        &dir.join("main.sico"),
        &format!(
            "{ENTRY_HEAD}interface Bad version 1:\n  function grab(table: Map[Text, Text]) returns Text\nend interface\n\nuse pkg csv version 1 expose Bad\n\n{ENTRY_TAIL}"
        ),
    );
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    write(&dir.join("pkgs/csv.sapp"), "placeholder");
    write(
        &dir.join("sico-lock.json"),
        "{\"schema\":\"sico:source-lock:v0\",\"packages\":[{\"name\":\"csv\",\"version\":1,\"path\":\"pkgs/csv.sapp\",\"sha256\":\"00\"}]}",
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8017"), "{}", stderr(&output));

    // E8019: `export function` is a typed refusal (RFC-0039 §2.5 D1).
    let dir = temp_root("export-refused");
    write(
        &dir.join("main.sico"),
        &format!(
            "{ENTRY_HEAD}{ENTRY_TAIL}\nexport function extra(x: I64) returns I64:\n  return x\nend function\n"
        ),
    );
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8019"), "{}", stderr(&output));

    // E8015: more packages than the v0 bound (limit+1).
    let dir = temp_root("limit");
    let mut uses = String::new();
    let mut entries = String::new();
    for index in 0..17 {
        uses.push_str(
            "use pkg p{index} version 1 expose Iface{index}"
                .replace("{index}", &index.to_string())
                .as_str(),
        );
        uses.push('\n');
        entries.push_str(",{\"name\":\"p");
        entries.push_str(&index.to_string());
        entries.push_str("\",\"version\":1,\"path\":\"pkgs/p");
        entries.push_str(&index.to_string());
        entries.push_str(".sapp\",\"sha256\":\"00\"}}");
        write(&dir.join(format!("if{index}.placeholder")), "");
    }
    write(
        &dir.join("main.sico"),
        &format!(
            "{ENTRY_HEAD}interface Iface0 version 1:\n  function f() returns Unit\nend interface\n{uses}\n{ENTRY_TAIL}"
        ),
    );
    fs::create_dir_all(dir.join("pkgs")).unwrap();
    let mut lock = String::from("{\"schema\":\"sico:source-lock:v0\",\"packages\":[");
    lock.push_str(&entries[1..]);
    lock.push_str("]}");
    write(&dir.join("sico-lock.json"), &lock);
    let output = check(&dir.join("main.sico"));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E8015"), "{}", stderr(&output));
}
