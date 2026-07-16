//! Bounded compiler-backed inspect and fix validation for AI tooling.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sico_diagnostics::syntax_identity;
use sico_format::format;
use sico_index::{IndexInput, build_index};
use sico_parser::parse;
use sico_semantics::analyze;
use sico_source::{SourceFile, SourceId};

pub const REQUEST_SCHEMA: &str = "sico.ai-tool.request.v0";
pub const RESPONSE_SCHEMA: &str = "sico.ai-tool.response.v0";
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_FILES: usize = 16;
pub const MAX_WORKSPACE_BYTES: usize = 1024 * 1024;
pub const MAX_SYMBOLS: usize = 256;
pub const MAX_DIAGNOSTICS: usize = 100;
pub const MAX_CHANGED_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Inspect,
    ValidateFix,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    protocol_version: u8,
    #[serde(rename = "request_id")]
    id: String,
    operation: Operation,
    budget: Budget,
    input: Value,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    #[serde(rename = "max_files")]
    files: usize,
    #[serde(rename = "max_input_bytes")]
    input_bytes: usize,
    #[serde(rename = "max_symbols")]
    symbols: usize,
    #[serde(rename = "max_diagnostics")]
    diagnostics: usize,
    #[serde(rename = "max_response_bytes")]
    response_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectInput {
    files: Vec<InputFile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFile {
    uri: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixInput {
    uri: String,
    original: String,
    candidate: String,
    original_sha256: String,
    expected_diagnostics: Vec<String>,
    max_changed_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Diagnostic {
    code: String,
    key: String,
    severity: &'static str,
    message: String,
    start_byte: u32,
    end_byte: u32,
}

#[derive(Clone, Copy, Debug)]
enum ToolError {
    RequestTooLarge,
    InvalidJson,
    InvalidRequest,
    InvalidMetadata,
    InvalidBudget,
    InvalidInput,
    InvalidSource,
    StaleSource,
    DiagnosticMismatch,
    CandidateDiagnostics,
    FixTooLarge,
    ResponseTooLarge,
}

impl ToolError {
    const fn code(self) -> &'static str {
        match self {
            Self::RequestTooLarge => "request_too_large",
            Self::InvalidJson => "invalid_json",
            Self::InvalidRequest => "invalid_request",
            Self::InvalidMetadata => "invalid_metadata",
            Self::InvalidBudget => "invalid_budget",
            Self::InvalidInput => "invalid_input",
            Self::InvalidSource => "invalid_source",
            Self::StaleSource => "stale_source",
            Self::DiagnosticMismatch => "diagnostic_mismatch",
            Self::CandidateDiagnostics => "candidate_has_diagnostics",
            Self::FixTooLarge => "fix_too_large",
            Self::ResponseTooLarge => "response_too_large",
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::RequestTooLarge => "request exceeds the one MiB limit",
            Self::InvalidJson => "request is not valid JSON",
            Self::InvalidRequest => "request shape is invalid",
            Self::InvalidMetadata => "request metadata is unsupported",
            Self::InvalidBudget => "request budget is outside protocol limits",
            Self::InvalidInput => "operation input is invalid",
            Self::InvalidSource => "source violates the compiler source contract",
            Self::StaleSource => "original source digest does not match",
            Self::DiagnosticMismatch => "expected diagnostics do not match the original source",
            Self::CandidateDiagnostics => "candidate still has compiler diagnostics",
            Self::FixTooLarge => "candidate changes more bytes than authorized",
            Self::ResponseTooLarge => "response cannot fit the requested budget",
        }
    }
}

/// Executes one complete protocol request from UTF-8 JSON bytes.
#[must_use]
pub fn execute_bytes(bytes: &[u8]) -> Value {
    if bytes.len() > MAX_REQUEST_BYTES {
        return failure("unknown", ToolError::RequestTooLarge);
    }
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return failure("unknown", ToolError::InvalidJson),
    };
    execute_value(value)
}

/// Executes one decoded protocol request without filesystem, process, or network access.
#[must_use]
pub fn execute_value(value: Value) -> Value {
    let request_id = value
        .get("request_id")
        .and_then(Value::as_str)
        .filter(|request_id| valid_request_id(request_id))
        .unwrap_or("unknown")
        .to_owned();
    if !matches!(encoded_len(&value), Ok(length) if length <= MAX_REQUEST_BYTES) {
        return failure(&request_id, ToolError::RequestTooLarge);
    }
    let request: Request = match serde_json::from_value(value) {
        Ok(request) => request,
        Err(_) => return failure(&request_id, ToolError::InvalidRequest),
    };
    if request.schema != REQUEST_SCHEMA || request.protocol_version != 0 {
        return failure(&request.id, ToolError::InvalidMetadata);
    }
    if !valid_request_id(&request.id) {
        return failure("unknown", ToolError::InvalidRequest);
    }
    if validate_budget(request.budget).is_err() {
        return failure(&request.id, ToolError::InvalidBudget);
    }
    let result = match request.operation {
        Operation::Inspect => inspect(&request),
        Operation::ValidateFix => validate_fix(&request),
    };
    match result {
        Ok(result) => success(&request.id, request.operation, &result),
        Err(error) => failure(&request.id, error),
    }
}

fn inspect(request: &Request) -> Result<Value, ToolError> {
    let input: InspectInput =
        serde_json::from_value(request.input.clone()).map_err(|_| ToolError::InvalidInput)?;
    if input.files.is_empty()
        || input.files.len() > request.budget.files
        || input.files.len() > MAX_FILES
    {
        return Err(ToolError::InvalidInput);
    }
    let mut uris = BTreeSet::new();
    let mut total_bytes = 0_usize;
    let mut sources = Vec::new();
    let mut file_records = Vec::new();
    let mut all_diagnostics = Vec::new();
    for (index, file) in input.files.iter().enumerate() {
        if !valid_uri(&file.uri) || !uris.insert(file.uri.as_str()) {
            return Err(ToolError::InvalidInput);
        }
        total_bytes = total_bytes.saturating_add(file.text.len());
        if total_bytes > request.budget.input_bytes || total_bytes > MAX_WORKSPACE_BYTES {
            return Err(ToolError::InvalidInput);
        }
        let source = source(
            u32::try_from(index).map_err(|_| ToolError::InvalidInput)?,
            file,
        )?;
        let diagnostics = diagnostics(&source)?;
        file_records.push(json!({
            "uri": file.uri,
            "sha256": sha256_hex(file.text.as_bytes()),
            "bytes": file.text.len(),
            "diagnostics": diagnostics.len()
        }));
        all_diagnostics.extend(
            diagnostics
                .into_iter()
                .map(|diagnostic| json!({ "uri": file.uri, "diagnostic": diagnostic })),
        );
        sources.push(IndexInput {
            module_name: module_name(&file.uri),
            file: file.uri.clone(),
            source,
        });
    }
    let index = build_index("ai-workspace", "0.0.0", &sources);
    let total_symbols = index.symbols.len();
    let total_diagnostics = all_diagnostics.len();
    let mut symbols = index
        .symbols
        .into_iter()
        .take(request.budget.symbols)
        .collect::<Vec<_>>();
    all_diagnostics.truncate(request.budget.diagnostics);
    let mut truncated = symbols.len() < total_symbols || all_diagnostics.len() < total_diagnostics;
    loop {
        let result = json!({
            "snapshot_id": index.snapshot.id,
            "quality": index.snapshot.quality,
            "files": file_records,
            "diagnostics": all_diagnostics,
            "symbols": symbols,
            "budget": {
                "input_bytes": total_bytes,
                "returned_symbols": symbols.len(),
                "total_symbols": total_symbols,
                "returned_diagnostics": all_diagnostics.len(),
                "total_diagnostics": total_diagnostics,
                "truncated": truncated
            }
        });
        if encoded_len(&result)? <= request.budget.response_bytes {
            return Ok(result);
        }
        truncated = true;
        if symbols.pop().is_none() && all_diagnostics.pop().is_none() {
            return Err(ToolError::ResponseTooLarge);
        }
    }
}

fn validate_fix(request: &Request) -> Result<Value, ToolError> {
    let input: FixInput =
        serde_json::from_value(request.input.clone()).map_err(|_| ToolError::InvalidInput)?;
    if !valid_uri(&input.uri)
        || input.original.len() > request.budget.input_bytes
        || input.candidate.len() > request.budget.input_bytes
        || input.original.len().saturating_add(input.candidate.len()) > MAX_WORKSPACE_BYTES
        || input.max_changed_bytes == 0
        || input.max_changed_bytes > MAX_CHANGED_BYTES
        || input.expected_diagnostics.is_empty()
        || input.expected_diagnostics.len() > MAX_DIAGNOSTICS
    {
        return Err(ToolError::InvalidInput);
    }
    if sha256_hex(input.original.as_bytes()) != input.original_sha256 {
        return Err(ToolError::StaleSource);
    }
    let original_file = source(
        0,
        &InputFile {
            uri: input.uri.clone(),
            text: input.original.clone(),
        },
    )?;
    let before = diagnostics(&original_file)?;
    let mut actual = before
        .iter()
        .map(|diagnostic| diagnostic.key.as_str())
        .collect::<Vec<_>>();
    let mut expected = input
        .expected_diagnostics
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    actual.sort_unstable();
    expected.sort_unstable();
    if actual != expected {
        return Err(ToolError::DiagnosticMismatch);
    }
    let candidate_file = source(
        1,
        &InputFile {
            uri: input.uri.clone(),
            text: input.candidate.clone(),
        },
    )?;
    let candidate_diagnostics = diagnostics(&candidate_file)?;
    if !candidate_diagnostics.is_empty() {
        return Err(ToolError::CandidateDiagnostics);
    }
    let edit = contiguous_edit(&input.original, &input.candidate)?;
    let changed_bytes = edit.removed_bytes.saturating_add(edit.inserted.len());
    if changed_bytes > input.max_changed_bytes || changed_bytes > MAX_CHANGED_BYTES {
        return Err(ToolError::FixTooLarge);
    }
    let canonical = format(&candidate_file).map_err(|_| ToolError::CandidateDiagnostics)?;
    let canonical_file = SourceFile::from_text(SourceId::new(2), &input.uri, canonical.clone())
        .map_err(|_| ToolError::InvalidSource)?;
    let after = diagnostics(&canonical_file)?;
    if !after.is_empty() {
        return Err(ToolError::CandidateDiagnostics);
    }
    let result = json!({
        "uri": input.uri,
        "original_sha256": input.original_sha256,
        "candidate_sha256": sha256_hex(input.candidate.as_bytes()),
        "canonical_sha256": sha256_hex(canonical.as_bytes()),
        "canonical_text": canonical,
        "diagnostics_before": before,
        "diagnostics_after": after,
        "edit": {
            "start_byte": edit.start,
            "end_byte": edit.end,
            "inserted_text": edit.inserted,
            "removed_bytes": edit.removed_bytes,
            "changed_bytes": changed_bytes
        },
        "accepted": true
    });
    if encoded_len(&result)? > request.budget.response_bytes {
        return Err(ToolError::ResponseTooLarge);
    }
    Ok(result)
}

fn validate_budget(budget: Budget) -> Result<(), ToolError> {
    if budget.files == 0
        || budget.files > MAX_FILES
        || budget.input_bytes == 0
        || budget.input_bytes > MAX_WORKSPACE_BYTES
        || budget.symbols == 0
        || budget.symbols > MAX_SYMBOLS
        || budget.diagnostics == 0
        || budget.diagnostics > MAX_DIAGNOSTICS
        || budget.response_bytes < 1024
        || budget.response_bytes > MAX_REQUEST_BYTES
    {
        return Err(ToolError::InvalidBudget);
    }
    Ok(())
}

fn source(id: u32, input: &InputFile) -> Result<SourceFile, ToolError> {
    SourceFile::from_text(SourceId::new(id), &input.uri, input.text.clone())
        .map_err(|_| ToolError::InvalidSource)
}

fn diagnostics(source: &SourceFile) -> Result<Vec<Diagnostic>, ToolError> {
    let parsed = parse(source);
    let mut records = Vec::new();
    for error in parsed.lex_errors() {
        records.push(Diagnostic {
            code: "E0002".to_owned(),
            key: "LEXICAL_ERROR".to_owned(),
            severity: "error",
            message: "lexical input is invalid".to_owned(),
            start_byte: error.range.start().into(),
            end_byte: error.range.end().into(),
        });
    }
    for error in parsed.errors() {
        let identity = syntax_identity(&error.kind);
        records.push(Diagnostic {
            code: identity.map_or("E1999", |value| value.code).to_owned(),
            key: identity
                .map_or("SYNTAX_ERROR", |value| value.key)
                .to_owned(),
            severity: "error",
            message: identity
                .map_or("syntax error", |value| value.message)
                .to_owned(),
            start_byte: error.range.start().into(),
            end_byte: error.range.end().into(),
        });
    }
    if records.is_empty() {
        let analysis = analyze(source).map_err(|_| ToolError::InvalidSource)?;
        records.extend(
            analysis
                .diagnostics
                .into_iter()
                .map(|diagnostic| Diagnostic {
                    code: diagnostic.code.to_owned(),
                    key: diagnostic.key.to_owned(),
                    severity: "error",
                    message: diagnostic.message,
                    start_byte: diagnostic.range.start().into(),
                    end_byte: diagnostic.range.end().into(),
                }),
        );
    }
    records.sort_by(|left, right| {
        (&left.start_byte, &left.code, &left.key).cmp(&(&right.start_byte, &right.code, &right.key))
    });
    records.truncate(MAX_DIAGNOSTICS);
    Ok(records)
}

struct Edit {
    start: usize,
    end: usize,
    removed_bytes: usize,
    inserted: String,
}

fn contiguous_edit(original: &str, candidate: &str) -> Result<Edit, ToolError> {
    let mut prefix = 0;
    for (left, right) in original.chars().zip(candidate.chars()) {
        if left != right {
            break;
        }
        prefix += left.len_utf8();
    }
    let remaining_original = &original[prefix..];
    let remaining_candidate = &candidate[prefix..];
    let mut suffix = 0;
    for (left, right) in remaining_original
        .chars()
        .rev()
        .zip(remaining_candidate.chars().rev())
    {
        if left != right {
            break;
        }
        suffix += left.len_utf8();
    }
    let original_end = original.len().saturating_sub(suffix);
    let candidate_end = candidate.len().saturating_sub(suffix);
    if prefix > original_end || prefix > candidate_end {
        return Err(ToolError::InvalidInput);
    }
    Ok(Edit {
        start: prefix,
        end: original_end,
        removed_bytes: original_end - prefix,
        inserted: candidate[prefix..candidate_end].to_owned(),
    })
}

fn encoded_len(value: &Value) -> Result<usize, ToolError> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|_| ToolError::ResponseTooLarge)
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

fn valid_uri(value: &str) -> bool {
    value.starts_with("file://") && value.len() <= 4096 && !value.chars().any(char::is_control)
}

fn module_name(uri: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(uri.len() * 2);
    for byte in uri.bytes() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn success(request_id: &str, operation: Operation, result: &Value) -> Value {
    json!({
        "schema": RESPONSE_SCHEMA,
        "protocol_version": 0,
        "request_id": request_id,
        "operation": match operation { Operation::Inspect => "inspect", Operation::ValidateFix => "validate_fix" },
        "ok": true,
        "result": result
    })
}

fn failure(request_id: &str, error: ToolError) -> Value {
    json!({
        "schema": RESPONSE_SCHEMA,
        "protocol_version": 0,
        "request_id": request_id,
        "ok": false,
        "error": { "code": error.code(), "message": error.message() }
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    fn budget() -> Value {
        json!({
            "max_files": 16,
            "max_input_bytes": MAX_WORKSPACE_BYTES,
            "max_symbols": 256,
            "max_diagnostics": 100,
            "max_response_bytes": MAX_REQUEST_BYTES
        })
    }

    fn request(operation: &str, input: Value) -> Value {
        let mut request = json!({
            "schema": REQUEST_SCHEMA,
            "protocol_version": 0,
            "request_id": "test-request",
            "operation": operation,
            "budget": budget(),
        });
        request["input"] = input;
        request
    }

    fn valid_source() -> &'static str {
        "function helper(value: Int) returns Int:\n  return value\nend function\n"
    }

    fn evaluation_source(text: &str) -> String {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let kept = normalized
            .lines()
            .filter(|line| {
                ![
                    "// case:",
                    "// syntax:",
                    "// expect:",
                    "// semantics:",
                    "// mutation:",
                    "// source-case:",
                    "// recovery:",
                ]
                .iter()
                .any(|prefix| line.starts_with(prefix))
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("{}\n", kept.trim_matches('\n'))
    }

    fn collect_sico(root: &Path, paths: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_sico(&path, paths);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "sico")
            {
                paths.push(path);
            }
        }
    }

    #[test]
    fn inspect_is_deterministic_and_compiler_backed() {
        let request = request(
            "inspect",
            json!({ "files": [{ "uri": "file:///main.sico", "text": valid_source() }] }),
        );
        let first = execute_value(request.clone());
        let second = execute_value(request);
        assert_eq!(first, second);
        assert_eq!(first["ok"], true);
        assert_eq!(first["result"]["diagnostics"], json!([]));
        assert!(
            first["result"]["symbols"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["name"] == "helper")
        );
        assert_eq!(first["result"]["budget"]["truncated"], false);
    }

    #[test]
    fn inspect_reports_stable_syntax_diagnostics_without_source_echo() {
        let response = execute_value(request(
            "inspect",
            json!({ "files": [{
                "uri": "file:///bad.sico",
                "text": "function main() returns Int:\n  return 1\n"
            }] }),
        ));
        assert_eq!(
            response["result"]["diagnostics"][0]["diagnostic"]["code"],
            "E1001"
        );
        assert_eq!(
            response["result"]["diagnostics"][0]["diagnostic"]["key"],
            "SYNTAX_MISSING_FUNCTION_CLOSE"
        );
        assert!(response.to_string().find("return 1").is_none());
    }

    #[test]
    fn budgets_duplicate_uris_and_invalid_sources_fail_closed() {
        let mut invalid_budget = request(
            "inspect",
            json!({ "files": [{ "uri": "file:///main.sico", "text": "" }] }),
        );
        invalid_budget["budget"]["max_symbols"] = json!(MAX_SYMBOLS + 1);
        assert_eq!(
            execute_value(invalid_budget)["error"]["code"],
            "invalid_budget"
        );
        let duplicate = request(
            "inspect",
            json!({ "files": [
                { "uri": "file:///main.sico", "text": "" },
                { "uri": "file:///main.sico", "text": "" }
            ] }),
        );
        assert_eq!(execute_value(duplicate)["error"]["code"], "invalid_input");
        let control = request(
            "inspect",
            json!({ "files": [{ "uri": "file:///main.sico", "text": "a\0b" }] }),
        );
        assert_eq!(execute_value(control)["error"]["code"], "invalid_source");
    }

    #[test]
    fn all_twelve_b_repair_oracles_pass_compiler_fix_validation() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest: Value = serde_json::from_str(
            &fs::read_to_string(root.join("ai-eval/tasks/repair.json")).unwrap(),
        )
        .unwrap();
        let tasks = manifest["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|task| task["syntax"] == "B")
            .collect::<Vec<_>>();
        assert_eq!(tasks.len(), 12);
        for task in tasks {
            let original = evaluation_source(
                &fs::read_to_string(root.join(task["input"].as_str().unwrap())).unwrap(),
            );
            let candidate = evaluation_source(
                &fs::read_to_string(root.join(task["expected"].as_str().unwrap())).unwrap(),
            );
            let response = execute_value(request(
                "validate_fix",
                json!({
                    "uri": format!("file:///{}", task["task_id"].as_str().unwrap()),
                    "original": original,
                    "candidate": candidate,
                    "original_sha256": sha256_hex(original.as_bytes()),
                    "expected_diagnostics": [task["diagnostic"].as_str().unwrap()],
                    "max_changed_bytes": 256
                }),
            ));
            assert_eq!(response["ok"], true, "{}: {response}", task["task_id"]);
            assert_eq!(response["result"]["diagnostics_after"], json!([]));
        }

        let mut paths = Vec::new();
        collect_sico(&root.join("syntax-candidates/b"), &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 54);
        let mut clean = 0;
        let mut diagnosed = 0;
        for (index, path) in paths.iter().enumerate() {
            let text = fs::read_to_string(path).unwrap();
            let response = execute_value(request(
                "inspect",
                json!({ "files": [{
                    "uri": format!("file:///corpus/{index}.sico"),
                    "text": text
                }] }),
            ));
            assert_eq!(response["ok"], true, "{}: {response}", path.display());
            let count = response["result"]["diagnostics"].as_array().unwrap().len();
            if text.contains("// expect: accept") {
                assert_eq!(count, 0, "{}", path.display());
                clean += 1;
            } else {
                assert!(count > 0, "{}", path.display());
                diagnosed += 1;
            }
        }
        assert_eq!((clean, diagnosed), (25, 29));
    }

    #[test]
    fn stale_source_and_diagnostic_confusion_are_rejected() {
        let original = "function main() returns Int:\n  return 1\n";
        let candidate = format!("{original}end function\n");
        let stale = execute_value(request(
            "validate_fix",
            json!({
                "uri": "file:///main.sico", "original": original, "candidate": candidate,
                "original_sha256": "0".repeat(64),
                "expected_diagnostics": ["SYNTAX_MISSING_FUNCTION_CLOSE"], "max_changed_bytes": 256
            }),
        ));
        assert_eq!(stale["error"]["code"], "stale_source");
        let confused = execute_value(request(
            "validate_fix",
            json!({
                "uri": "file:///main.sico", "original": original, "candidate": candidate,
                "original_sha256": sha256_hex(original.as_bytes()),
                "expected_diagnostics": ["SYNTAX_MISSING_RECORD_CLOSE"], "max_changed_bytes": 256
            }),
        ));
        assert_eq!(confused["error"]["code"], "diagnostic_mismatch");
    }

    #[test]
    fn unrelated_wide_rewrite_is_rejected_even_when_candidate_compiles() {
        let original = "function main() returns Int:\n  return 1\n";
        let candidate = format!(
            "{}\nfunction main() returns Int:\n  return 1\nend function\n",
            "// unrelated".repeat(40)
        );
        let response = execute_value(request(
            "validate_fix",
            json!({
                "uri": "file:///main.sico", "original": original, "candidate": candidate,
                "original_sha256": sha256_hex(original.as_bytes()),
                "expected_diagnostics": ["SYNTAX_MISSING_FUNCTION_CLOSE"], "max_changed_bytes": 32
            }),
        ));
        assert_eq!(response["error"]["code"], "fix_too_large");
    }

    #[test]
    fn syntactic_repair_with_semantic_diagnostics_is_rejected() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let candidate = fs::read_to_string(
            root.join("syntax-candidates/b/numbers-units/invalid/text-as-int.sico"),
        )
        .unwrap();
        let candidate = evaluation_source(&candidate);
        let original = candidate.replace("end function\n", "");
        let response = execute_value(request(
            "validate_fix",
            json!({
                "uri": "file:///main.sico", "original": original, "candidate": candidate,
                "original_sha256": sha256_hex(original.as_bytes()),
                "expected_diagnostics": ["SYNTAX_MISSING_FUNCTION_CLOSE"], "max_changed_bytes": 256
            }),
        ));
        assert_eq!(response["error"]["code"], "candidate_has_diagnostics");
    }

    #[test]
    fn five_hundred_twelve_protocol_mutations_are_rejected() {
        let mut rejected = 0;
        for index in 0..256 {
            let mut invalid = request(
                "inspect",
                json!({ "files": [{ "uri": "file:///main.sico", "text": "" }] }),
            );
            invalid["schema"] = json!(format!("invalid-{index}"));
            rejected += usize::from(execute_value(invalid)["ok"] == false);
        }
        for index in 0..256 {
            let mut invalid = request(
                "inspect",
                json!({ "files": [{ "uri": "file:///main.sico", "text": "" }] }),
            );
            invalid["budget"]["max_response_bytes"] = json!(MAX_REQUEST_BYTES + index + 1);
            rejected += usize::from(execute_value(invalid)["ok"] == false);
        }
        assert_eq!(rejected, 512);
    }
}
