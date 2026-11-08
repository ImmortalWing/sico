//! Re-derive the frozen `match` site ledger `selfhost/match-component-coverage-v1.json`
//! without Python.
//!
//! This is the field-level equivalent of `tools/census-m22-match-sites.py`
//! (`--update` / `--check`) for hosts with no Python interpreter, where the
//! ledger refreeze was blocked.  It re-derives the candidate sites from the
//! frozen S exactly like the Python census (every `match` statement whose head
//! is `U64.checked_add`, at any parameter count), keeps the recorded verdict and
//! digests of the sites that did not move, and merges the actual outcomes the
//! firmware test `sico_compiler_reproduces_the_frozen_match_site_ledger`
//! measured into `sico-m22-match-coverage/actual.json`.
//!
//! The writer is byte-identical to the Python census
//! (`json.dumps(ledger, indent=2, ensure_ascii=False, sort_keys=False) + "\n"`),
//! so `--check` accepts a ledger the Python census produced and vice versa.
//! No digest is ever invented here: every `*_sha256` comes either from the
//! previously frozen ledger or from the measured `actual.json`.
//!
//! Modes:
//! ```text
//! merge-m22-match-coverage --verify          # compare the re-derivation against the ledger
//! merge-m22-match-coverage --check           # verify only, never write
//! merge-m22-match-coverage --update          # merge measured verdicts, write
//! merge-m22-match-coverage --update --actual <path>
//! ```

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const SCHEMA: &str = "sico.m22.match-coverage.v1";
const GENERATED_BY: &str = "tools/census-m22-match-sites.py";
const SOURCE_SET: &str = "selfhost/*.sico (frozen S)";
const EXPECTED_SITES: usize = 396;

/// The files whose whole-file canonical IR differential is pinned by the
/// firmware; a site whose closure is too large leans on that pin.
const WHOLE_FILE_PINS: &[(&str, &str)] = &[(
    "parser.sico",
    "sico_guest_arena_accommodates_full_parser_selfhost_input",
)];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct LedgerFile {
    schema: String,
    #[serde(default)]
    generated_by: String,
    #[serde(default)]
    source_set: String,
    #[serde(default)]
    rule: String,
    sites: Vec<LedgerSite>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct LedgerSite {
    id: String,
    path: String,
    line: u64,
    function: String,
    function_line: u64,
    parameter_count: u64,
    head: String,
    labels: String,
    case_shape: String,
    verdict: String,
    source_sha256: String,
    ir_sha256: String,
    component_sha256: String,
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActualEntry {
    path: String,
    function: String,
    verdict: String,
    source_sha256: String,
    ir_sha256: String,
    component_sha256: String,
}

/// A re-derived candidate site.  The structural fields must match the frozen
/// ledger exactly; only `line` moves when the frozen S grows or shrinks.
#[derive(Clone)]
struct Site {
    path: String,
    line: usize,
    function: String,
    function_line: usize,
    parameter_count: usize,
    head: String,
    labels: String,
    case_shape: String,
    ordinal: usize,
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `function <name>(<params>):` -- returns `(name, param_count)`.
fn parse_function_head(trimmed: &str) -> Option<(String, usize)> {
    let rest = trimmed.strip_prefix("function ")?;
    let rest = rest.trim_start();
    let bytes = rest.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() && is_ident_byte(bytes[cursor]) {
        cursor += 1;
    }
    let name = &rest[..cursor];
    if name.is_empty() || !is_ident_start(name.as_bytes()[0]) {
        return None;
    }
    let after = rest[cursor..].trim_start();
    let open = after.find('(')?;
    let close = after.rfind(')')?;
    if close < open {
        return None;
    }
    let raw = &after[open + 1..close];
    let parameters = raw
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .count();
    Some((name.to_string(), parameters))
}

fn parse_case_label(body: &str) -> Option<String> {
    let rest = body.strip_prefix("case ")?;
    let label: String = rest
        .chars()
        .take_while(|character| is_ident_byte(*character as u8))
        .collect();
    if label.is_empty() { None } else { Some(label) }
}

/// Every `checked_add` match head in every `selfhost/*.sico`, in file order.
fn enumerate_sites(source_directory: &Path) -> Vec<Site> {
    let mut paths: Vec<PathBuf> = fs::read_dir(source_directory)
        .unwrap_or_else(|error| panic!("cannot read {source_directory:?}: {error}"))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("sico"))
        .collect();
    paths.sort();

    let mut sites = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {path:?}: {error}"));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();
        let lines: Vec<&str> = text.split('\n').collect();
        let mut current_function = String::new();
        let mut current_function_line = 0usize;
        let mut current_params = 0usize;
        let mut current_ordinal = 0usize;
        let mut index = 0usize;
        while index < lines.len() {
            let line = lines[index];
            if let Some((name_text, parameters)) = parse_function_head(line.trim_start()) {
                current_function = name_text;
                current_function_line = index + 1;
                current_params = parameters;
                current_ordinal = 0;
            }
            let indent = indent_of(line);
            let body = line.trim_start();
            let match_rest = body
                .strip_prefix("match ")
                .and_then(|rest| rest.strip_suffix(':'));
            let Some(head) = match_rest else {
                index += 1;
                continue;
            };
            let head = head.trim();
            let match_line = index + 1;
            let operator = match head.find('(') {
                Some(open) => head[..open]
                    .trim()
                    .rsplit_once('.')
                    .map(|(_, last)| last.to_string())
                    .unwrap_or_else(|| String::from("?")),
                None => String::from("?"),
            };
            let mut labels: Vec<String> = Vec::new();
            let mut shapes: Vec<String> = Vec::new();
            let mut case_indent: Option<usize> = None;
            index += 1;
            while index < lines.len() {
                let body_line = lines[index];
                let body_indent = indent_of(body_line);
                if body_line.trim() == "end match" && body_indent == indent {
                    break;
                }
                let trimmed_body = body_line.trim_start();
                let is_case = trimmed_body.strip_prefix("case ").is_some()
                    && trimmed_body.trim_end().ends_with(':')
                    && case_indent.is_none_or(|seen| body_indent == seen);
                if is_case {
                    case_indent = Some(body_indent);
                    if let Some(label) = parse_case_label(trimmed_body) {
                        labels.push(label);
                    }
                    let mut statements = 0usize;
                    let mut kinds: Vec<String> = Vec::new();
                    index += 1;
                    while index < lines.len() {
                        let inner_line = lines[index];
                        let inner_indent = indent_of(inner_line);
                        if inner_line.trim().is_empty() || inner_indent <= indent {
                            break;
                        }
                        let inner_trimmed = inner_line.trim_start();
                        if inner_trimmed.strip_prefix("case ").is_some()
                            && inner_trimmed.trim_end().ends_with(':')
                        {
                            break;
                        }
                        kinds.push(inner_trimmed.split(' ').next().unwrap_or("").to_string());
                        statements += 1;
                        index += 1;
                    }
                    shapes.push(format!("{statements}:{}", kinds.join("/")));
                    continue;
                }
                index += 1;
            }
            if operator != "checked_add" {
                continue;
            }
            sites.push(Site {
                path: name.clone(),
                line: match_line,
                function: current_function.clone(),
                function_line: current_function_line,
                parameter_count: current_params,
                head: head.to_string(),
                labels: labels.join("+"),
                case_shape: shapes.join(" | "),
                ordinal: current_ordinal,
            });
            current_ordinal += 1;
        }
    }
    sites
}

fn whole_file_pin(path: &str) -> Option<&'static str> {
    WHOLE_FILE_PINS
        .iter()
        .find(|(file, _)| *file == path)
        .map(|(_, pin)| *pin)
}

struct Record {
    id: String,
    path: String,
    line: usize,
    function: String,
    function_line: usize,
    parameter_count: usize,
    head: String,
    labels: String,
    case_shape: String,
    verdict: String,
    source_sha256: String,
    ir_sha256: String,
    component_sha256: String,
    note: String,
}

/// Emits exactly `json.dumps(..., indent=2, ensure_ascii=False)`: two-space
/// indent per level, `": "` after keys, no trailing spaces, non-ASCII literal.
struct Writer {
    out: String,
}

impl Writer {
    fn new() -> Self {
        Self { out: String::new() }
    }

    fn pad(&mut self, level: usize) {
        for _ in 0..level {
            self.out.push_str("  ");
        }
    }

    fn string(&mut self, value: &str) {
        self.out.push('"');
        for character in value.chars() {
            match character {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                control if (control as u32) < 0x20 => {
                    self.out.push_str(&format!("\\u{:04x}", control as u32));
                }
                other => self.out.push(other),
            }
        }
        self.out.push('"');
    }

    fn text_field(&mut self, level: usize, name: &str, value: &str, comma: bool) {
        self.pad(level);
        self.string(name);
        self.out.push_str(": ");
        self.string(value);
        self.out.push_str(if comma { ",\n" } else { "\n" });
    }

    fn number_field(&mut self, level: usize, name: &str, value: usize) {
        self.pad(level);
        self.string(name);
        self.out.push_str(": ");
        self.out.push_str(&value.to_string());
        self.out.push_str(",\n");
    }
}

fn render(rule: &str, records: &[Record]) -> String {
    let mut writer = Writer::new();
    writer.out.push_str("{\n");
    writer.text_field(1, "schema", SCHEMA, true);
    writer.text_field(1, "generated_by", GENERATED_BY, true);
    writer.text_field(1, "source_set", SOURCE_SET, true);
    writer.text_field(1, "rule", rule, true);
    writer.pad(1);
    writer.out.push_str("\"sites\": [\n");
    let total = records.len();
    for (position, record) in records.iter().enumerate() {
        writer.pad(2);
        writer.out.push_str("{\n");
        writer.text_field(3, "id", &record.id, true);
        writer.text_field(3, "path", &record.path, true);
        writer.number_field(3, "line", record.line);
        writer.text_field(3, "function", &record.function, true);
        writer.number_field(3, "function_line", record.function_line);
        writer.number_field(3, "parameter_count", record.parameter_count);
        writer.text_field(3, "head", &record.head, true);
        writer.text_field(3, "labels", &record.labels, true);
        writer.text_field(3, "case_shape", &record.case_shape, true);
        writer.text_field(3, "verdict", &record.verdict, true);
        writer.text_field(3, "source_sha256", &record.source_sha256, true);
        writer.text_field(3, "ir_sha256", &record.ir_sha256, true);
        writer.text_field(3, "component_sha256", &record.component_sha256, true);
        writer.text_field(3, "note", &record.note, false);
        writer.pad(2);
        writer.out.push('}');
        writer
            .out
            .push_str(if position + 1 < total { ",\n" } else { "\n" });
    }
    writer.pad(1);
    writer.out.push_str("]\n");
    writer.out.push_str("}\n");
    writer.out
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// The measured outcomes keyed by `(path, function)` -- the same identity the
/// frozen ledger uses.
type ActualMap = BTreeMap<(String, String), ActualEntry>;

/// Loads `actual.json` for merging.  A missing file is reported as `Ok(None)`
/// so each mode can decide whether absence is fatal (`--update` has nothing to
/// merge) or benign (`--check` keeps the frozen verdicts); malformed JSON and a
/// malformed digest are always typed rejections, never a silent skip.
fn load_actual(path: &Path) -> Result<Option<ActualMap>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };
    let entries: Vec<ActualEntry> = serde_json::from_str(&text).map_err(|error| {
        format!("ACTUAL_INVALID {path:?} is not a valid actual report: {error}")
    })?;
    let mut map = BTreeMap::new();
    for entry in entries {
        for (field, digest) in [
            ("source_sha256", &entry.source_sha256),
            ("ir_sha256", &entry.ir_sha256),
            ("component_sha256", &entry.component_sha256),
        ] {
            if !is_lower_hex_digest(digest) {
                return Err(format!(
                    "ACTUAL_DIGEST {} entry {}:{} field {field} is not a 64-char \
                     lowercase hex sha256: {digest:?}",
                    path.display(),
                    entry.path,
                    entry.function
                ));
            }
        }
        map.insert((entry.path.clone(), entry.function.clone()), entry);
    }
    Ok(Some(map))
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let update = arguments.iter().any(|value| value == "--update");
    let check = arguments.iter().any(|value| value == "--check");
    let verify = arguments.iter().any(|value| value == "--verify");
    if !update && !check && !verify {
        eprintln!(
            "USAGE merge-m22-match-coverage --verify|--check|--update [--actual <path>]\n\
             refusing to run without an explicit mode: the default used to write the ledger"
        );
        std::process::exit(2);
    }
    let actual_override = arguments
        .iter()
        .position(|value| value == "--actual")
        .and_then(|position| arguments.get(position + 1))
        .map(PathBuf::from);

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest
        .parent()
        .and_then(|parent| parent.parent())
        .expect("the runner crate sits two levels under the repo");
    let source_directory = repo.join("selfhost");
    let ledger = source_directory.join("match-component-coverage-v1.json");
    let actual = actual_override.unwrap_or_else(|| {
        std::env::temp_dir()
            .join("sico-m22-match-coverage")
            .join("actual.json")
    });

    let sites = enumerate_sites(&source_directory);
    if sites.len() != EXPECTED_SITES {
        eprintln!(
            "CENSUS_MISMATCH expected={EXPECTED_SITES} actual={}",
            sites.len()
        );
        std::process::exit(1);
    }

    let previous_ledger: Option<LedgerFile> = if ledger.exists() {
        let text = fs::read_to_string(&ledger)
            .unwrap_or_else(|error| panic!("cannot read the ledger: {error}"));
        let parsed: LedgerFile = serde_json::from_str(&text).unwrap_or_else(|error| {
            eprintln!("LEDGER_INVALID the ledger is not valid JSON: {error}");
            std::process::exit(1);
        });
        if parsed.schema != SCHEMA {
            eprintln!("LEDGER_SCHEMA {}", parsed.schema);
            std::process::exit(1);
        }
        Some(parsed)
    } else {
        None
    };

    // The frozen ledger is keyed by (path, function, ordinal) -- the site's
    // identity, independent of the line it happens to sit on.
    let previous: BTreeMap<String, &LedgerSite> = match &previous_ledger {
        Some(parsed) => {
            let mut counters: BTreeMap<(&str, &str), usize> = BTreeMap::new();
            let mut map = BTreeMap::new();
            for site in &parsed.sites {
                let ordinal = counters
                    .get(&(site.path.as_str(), site.function.as_str()))
                    .copied()
                    .unwrap_or(0);
                counters.insert((site.path.as_str(), site.function.as_str()), ordinal + 1);
                map.insert(format!("{}:{}:{}", site.path, site.function, ordinal), site);
            }
            map
        }
        None => BTreeMap::new(),
    };

    if verify {
        let mut mismatches = 0usize;
        for (number, site) in sites.iter().enumerate() {
            let key = format!("{}:{}:{}", site.path, site.function, site.ordinal);
            let Some(old) = previous.get(&key) else {
                eprintln!(
                    "VERIFY {} {}:{}:{} has no ledger entry",
                    number + 1,
                    site.path,
                    site.function,
                    site.ordinal
                );
                mismatches += 1;
                continue;
            };
            for (field, derived, frozen) in [
                (
                    "function_line",
                    site.function_line.to_string(),
                    old.function_line.to_string(),
                ),
                (
                    "parameter_count",
                    site.parameter_count.to_string(),
                    old.parameter_count.to_string(),
                ),
                ("head", site.head.clone(), old.head.clone()),
                ("labels", site.labels.clone(), old.labels.clone()),
                (
                    "case_shape",
                    site.case_shape.clone(),
                    old.case_shape.clone(),
                ),
            ] {
                if derived != frozen {
                    eprintln!(
                        "VERIFY {} ({}) {field}: derived={derived:?} frozen={frozen:?}",
                        old.id, key
                    );
                    mismatches += 1;
                }
            }
        }
        if mismatches == 0 {
            println!(
                "VERIFY_OK the re-derivation matches all {} frozen structural fields",
                sites.len()
            );
        } else {
            eprintln!("VERIFY_FAILED mismatches={mismatches}");
            std::process::exit(1);
        }
        return;
    }

    let actuals = match load_actual(&actual) {
        Ok(actuals) => actuals,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };
    if update && actuals.is_none() {
        eprintln!(
            "ACTUAL_MISSING {} has no measured actual report; refusing to rewrite \
             the ledger with nothing measured (run the firmware ledger test first, \
             or pass --actual <path>)",
            actual.display()
        );
        std::process::exit(1);
    }
    let actuals = actuals.unwrap_or_default();
    let mut records = Vec::new();
    let mut pending = 0usize;
    for (number, site) in sites.iter().enumerate() {
        let key = format!("{}:{}:{}", site.path, site.function, site.ordinal);
        let old = previous.get(&key).copied();
        let entry = actuals.get(&(site.path.clone(), site.function.clone()));
        let mut verdict = old
            .map(|site| site.verdict.clone())
            .unwrap_or_else(|| String::from("pending"));
        let mut source_digest = old
            .map(|site| site.source_sha256.clone())
            .unwrap_or_default();
        let mut ir_digest = old.map(|site| site.ir_sha256.clone()).unwrap_or_default();
        let mut component_digest = old
            .map(|site| site.component_sha256.clone())
            .unwrap_or_default();
        let mut note = old.map(|site| site.note.clone()).unwrap_or_default();
        if let Some(entry) = entry
            && (update || verdict == "pending")
        {
            verdict = entry.verdict.clone();
            source_digest = entry.source_sha256.clone();
            ir_digest = entry.ir_sha256.clone();
            component_digest = entry.component_sha256.clone();
            if verdict.starts_with("closure-too-large") {
                let Some(pin) = whole_file_pin(&site.path) else {
                    eprintln!(
                        "CENSUS_MISMATCH {} has no whole-file IR pin for a closure-too-large site",
                        site.path
                    );
                    std::process::exit(1);
                };
                note = format!("IR witness: the whole-file differential pin {pin}");
            } else if note.is_empty() {
                note = String::from("measured by the frozen ledger firmware test");
            }
        }
        if verdict == "pending" {
            pending += 1;
        }
        records.push(Record {
            id: format!("MS-{:03}", number + 1),
            path: site.path.clone(),
            line: site.line,
            function: site.function.clone(),
            function_line: site.function_line,
            parameter_count: site.parameter_count,
            head: site.head.clone(),
            labels: site.labels.clone(),
            case_shape: site.case_shape.clone(),
            verdict,
            source_sha256: source_digest,
            ir_sha256: ir_digest,
            component_sha256: component_digest,
            note,
        });
    }

    let rule = format!(
        "a `match` statement whose head is U64.checked_add, at any parameter \
         count -- the {EXPECTED_SITES} candidate sites of the frozen S (42 below the \
         removed parameter-count gate and 354 behind it)"
    );
    let text = render(&rule, &records);

    if check {
        if !ledger.exists() {
            eprintln!("LEDGER_MISSING");
            std::process::exit(1);
        }
        let current = fs::read_to_string(&ledger)
            .unwrap_or_else(|error| panic!("cannot read the ledger: {error}"));
        if current != text {
            eprintln!("LEDGER_STALE");
            std::process::exit(1);
        }
        println!("LEDGER_CURRENT sites={}", records.len());
        return;
    }

    fs::write(&ledger, text.as_bytes())
        .unwrap_or_else(|error| panic!("cannot write the ledger: {error}"));
    println!("LEDGER_WRITTEN sites={} pending={pending}", records.len());
}
