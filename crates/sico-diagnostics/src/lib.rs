//! RFC-0001 text and JSON rendering for compiler diagnostics.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use sico_parser::{ParseError, ParseErrorKind};
use sico_source::{LineCol, SourceFile, TextRange};

/// Stable catalog identity and default English message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticIdentity {
    pub code: &'static str,
    pub key: &'static str,
    pub message: &'static str,
}

/// Returns the accepted E1xxx identity for a measured syntax root cause.
#[must_use]
pub const fn syntax_identity(kind: &ParseErrorKind) -> Option<DiagnosticIdentity> {
    let identity = match kind {
        ParseErrorKind::MissingFunctionClose => (
            "E1001",
            "SYNTAX_MISSING_FUNCTION_CLOSE",
            "missing end function",
        ),
        ParseErrorKind::MissingMatchArmSeparator => (
            "E1002",
            "SYNTAX_MISSING_MATCH_ARM_SEPARATOR",
            "missing ':' after match arm",
        ),
        ParseErrorKind::MissingRecordClose => {
            ("E1003", "SYNTAX_MISSING_RECORD_CLOSE", "missing end record")
        }
        ParseErrorKind::MissingTypeArgumentClose => (
            "E1004",
            "SYNTAX_MISSING_TYPE_ARGUMENT_CLOSE",
            "missing ']' in type arguments",
        ),
        ParseErrorKind::MissingEnumClose => {
            ("E1005", "SYNTAX_MISSING_ENUM_CLOSE", "missing end enum")
        }
        ParseErrorKind::MissingCallClose => {
            ("E1006", "SYNTAX_MISSING_CALL_CLOSE", "missing ')' in call")
        }
        ParseErrorKind::MissingCapabilityClose => (
            "E1007",
            "SYNTAX_MISSING_CAPABILITY_CLOSE",
            "missing end capability",
        ),
        ParseErrorKind::MissingResourceClose => (
            "E1008",
            "SYNTAX_MISSING_RESOURCE_CLOSE",
            "missing end resource",
        ),
        ParseErrorKind::MissingUsingClose => {
            ("E1009", "SYNTAX_MISSING_USING_CLOSE", "missing end using")
        }
        ParseErrorKind::MissingTaskClose => (
            "E1010",
            "SYNTAX_MISSING_TASK_GROUP_CLOSE",
            "missing end task",
        ),
        ParseErrorKind::MissingInterfaceClose => (
            "E1011",
            "SYNTAX_MISSING_INTERFACE_CLOSE",
            "missing end interface",
        ),
        ParseErrorKind::MissingParameterListClose => (
            "E1012",
            "SYNTAX_MISSING_PARAMETER_LIST_CLOSE",
            "missing ')' in parameter list",
        ),
        _ => return None,
    };
    Some(DiagnosticIdentity {
        code: identity.0,
        key: identity.1,
        message: identity.2,
    })
}

/// Renders RFC-0001 one-line text diagnostics in stable byte/code order.
#[must_use]
pub fn render_syntax_text(source: &SourceFile, file: &str, errors: &[ParseError]) -> String {
    ordered_syntax_errors(errors)
        .into_iter()
        .filter_map(|error| {
            let identity = syntax_identity(&error.kind)?;
            let position = position(source, error.range.start())?;
            Some(format!(
                "{} {}:{}:{} {}",
                identity.code, file, position.line, position.column, identity.message
            ))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Renders an RFC-0001 JSON envelope. Returns `None` only if a parser span is
/// not a valid source scalar boundary, which indicates an internal bug.
#[must_use]
pub fn render_syntax_json(
    source: &SourceFile,
    file: &str,
    errors: &[ParseError],
) -> Option<String> {
    let mut diagnostics = Vec::new();
    for (index, error) in ordered_syntax_errors(errors).into_iter().enumerate() {
        let Some(identity) = syntax_identity(&error.kind) else {
            continue;
        };
        let mut diagnostic = json!({
            "id": format!("d{}", index + 1),
            "code": identity.code,
            "key": identity.key,
            "severity": "error",
            "kind": "root",
            "message": identity.message,
            "arguments": {},
            "file": file,
            "range": range_value(source, error.range)?,
            "recovery_anchor": error.anchor.as_str()
        });
        if let Some(related) = error.related {
            diagnostic["related"] = json!([{
                "file": file,
                "range": range_value(source, related)?,
                "message": "opened here"
            }]);
        }
        diagnostics.push(diagnostic);
    }
    let emitted = diagnostics.len();
    let envelope = json!({
        "schema": "sico.diagnostics.v0",
        "protocol_version": 0,
        "tool": { "name": "sico", "version": env!("CARGO_PKG_VERSION") },
        "coordinate_system": {
            "encoding": "utf-8",
            "byte_base": 0,
            "byte_end": "exclusive",
            "line_base": 1,
            "column_base": 1,
            "column_unit": "unicode-scalar-value"
        },
        "diagnostics": diagnostics,
        "summary": {
            "emitted": emitted,
            "errors": emitted,
            "warnings": 0,
            "info": 0,
            "suppressed": 0,
            "truncated": 0
        }
    });
    serde_json::to_string_pretty(&envelope).ok()
}

fn ordered_syntax_errors(errors: &[ParseError]) -> Vec<&ParseError> {
    let mut result: Vec<_> = errors
        .iter()
        .filter(|error| syntax_identity(&error.kind).is_some())
        .collect();
    result.sort_by_key(|error| {
        (
            error.range.start(),
            syntax_identity(&error.kind).map_or("", |identity| identity.code),
        )
    });
    result
}

fn position(source: &SourceFile, offset: sico_source::TextSize) -> Option<LineCol> {
    source.line_index().line_col(source.text(), offset)
}

fn range_value(source: &SourceFile, range: TextRange) -> Option<Value> {
    let start = position(source, range.start())?;
    let end = position(source, range.end())?;
    Some(json!({
        "start": { "byte": u32::from(range.start()), "line": start.line, "column": start.column },
        "end": { "byte": u32::from(range.end()), "line": end.line, "column": end.column }
    }))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::Value;
    use sico_parser::parse;
    use sico_source::SourceId;

    use super::*;

    #[test]
    fn all_b_mutations_render_registered_text_and_json_spans() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-mutations/b");
        let map_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../diagnostics/syntax-mutation-map.json");
        let map: Value = serde_json::from_str(&fs::read_to_string(map_path).unwrap()).unwrap();
        assert_eq!(map["cases"].as_array().unwrap().len(), 12);
        let mut snapshots = Vec::new();
        for (index, expected) in map["cases"].as_array().unwrap().iter().enumerate() {
            let mutation = expected["mutation"].as_str().unwrap();
            let path = fs::read_dir(&root)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(mutation)
                })
                .unwrap();
            let relative = format!(
                "syntax-mutations/b/{}",
                path.file_name().unwrap().to_string_lossy()
            );
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                &relative,
                &fs::read(&path).unwrap(),
            )
            .unwrap();
            let parsed = parse(&source);
            assert_eq!(parsed.errors().len(), 1, "{mutation}");
            let identity = syntax_identity(&parsed.errors()[0].kind).unwrap();
            assert_eq!(identity.code, expected["code"], "{mutation}");
            assert_eq!(identity.key, expected["key"], "{mutation}");
            assert_eq!(identity.message, expected["message"], "{mutation}");
            assert_eq!(
                parsed.errors()[0].anchor.as_str(),
                expected["anchor"],
                "{mutation}"
            );

            let text = render_syntax_text(&source, &relative, parsed.errors());
            assert!(text.starts_with(identity.code), "{mutation}: {text}");
            assert!(text.ends_with(identity.message), "{mutation}: {text}");
            let json: Value = serde_json::from_str(
                &render_syntax_json(&source, &relative, parsed.errors()).unwrap(),
            )
            .unwrap();
            assert_eq!(json["diagnostics"].as_array().unwrap().len(), 1);
            assert_eq!(json["diagnostics"][0]["code"], identity.code);
            assert_eq!(
                json["diagnostics"][0]["range"]["start"]["byte"],
                u32::from(parsed.errors()[0].range.start())
            );
            assert_eq!(
                json["diagnostics"][0]["recovery_anchor"],
                expected["anchor"]
            );
            assert_eq!(
                json["diagnostics"][0]["related"].as_array().unwrap().len(),
                1
            );
            assert_eq!(json["summary"]["emitted"], 1);
            snapshots.push(format!(
                "{}|{}|{}..{}|{}:{}|{}|{}",
                mutation,
                identity.code,
                json["diagnostics"][0]["range"]["start"]["byte"],
                json["diagnostics"][0]["range"]["end"]["byte"],
                json["diagnostics"][0]["range"]["start"]["line"],
                json["diagnostics"][0]["range"]["start"]["column"],
                expected["anchor"].as_str().unwrap(),
                text
            ));
        }
        let snapshot = format!("{}\n", snapshots.join("\n"));
        if std::env::var_os("SICO_DUMP_DIAGNOSTICS").is_some() {
            print!("{snapshot}");
        } else {
            assert_eq!(
                snapshot,
                include_str!("../../../tests/diagnostics/b-mutations.snap")
            );
        }
    }
}
