//! Bounded local Language Server Protocol adapter backed by the Sico compiler.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
};

use serde_json::{Value, json};
use sico_diagnostics::syntax_identity;
use sico_format::format;
use sico_index::{IndexInput, SemanticIndex, Symbol, build_index};
use sico_parser::parse;
use sico_semantics::analyze;
use sico_source::{SourceFile, SourceId, TextRange};

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const MAX_HEADER_BYTES: usize = 8 * 1024;
pub const MAX_OPEN_DOCUMENTS: usize = 128;
pub const MAX_URI_BYTES: usize = 4096;
pub const MAX_WORKSPACE_SOURCE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_COMPLETIONS: usize = 256;
pub const MAX_REFERENCES: usize = 256;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Lifecycle {
    #[default]
    New,
    Ready,
    Shutdown,
    Exited,
}

#[derive(Clone, Debug)]
struct Document {
    version: i64,
    text: String,
}

#[derive(Debug, Default)]
pub struct LanguageServer {
    lifecycle: Lifecycle,
    documents: BTreeMap<String, Document>,
}

impl LanguageServer {
    #[must_use]
    pub const fn has_exited(&self) -> bool {
        matches!(self.lifecycle, Lifecycle::Exited)
    }

    /// Processes one decoded JSON-RPC message and returns responses or notifications.
    #[must_use]
    pub fn process(&mut self, message: Value) -> Vec<Value> {
        let Value::Object(message) = message else {
            return Vec::new();
        };
        let id = message.get("id").cloned();
        if message.get("jsonrpc") != Some(&Value::String("2.0".to_owned())) {
            return id.map_or_else(Vec::new, |id| {
                vec![error(id, -32600, "invalid JSON-RPC version")]
            });
        }
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return id.map_or_else(Vec::new, |id| vec![error(id, -32600, "missing method")]);
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        if method == "exit" {
            self.lifecycle = Lifecycle::Exited;
            return Vec::new();
        }
        if method == "initialize" {
            return self.initialize(id);
        }
        if !matches!(self.lifecycle, Lifecycle::Ready) {
            return id.map_or_else(Vec::new, |id| {
                vec![error(id, -32002, "server is not initialized")]
            });
        }

        match method {
            "initialized" => Vec::new(),
            "shutdown" => {
                self.lifecycle = Lifecycle::Shutdown;
                id.map_or_else(Vec::new, |id| vec![success(id, Value::Null)])
            }
            "textDocument/didOpen" => self.did_open(&params),
            "textDocument/didChange" => self.did_change(&params),
            "textDocument/didClose" => self.did_close(&params),
            "textDocument/diagnostic" => {
                self.request(id, |server| server.pull_diagnostics(&params))
            }
            "textDocument/documentSymbol" => {
                self.request(id, |server| server.document_symbols(&params))
            }
            "textDocument/completion" => self.request(id, |server| server.completion(&params)),
            "textDocument/hover" => self.request(id, |server| server.hover(&params)),
            "textDocument/definition" => self.request(id, |server| server.definition(&params)),
            "textDocument/references" => self.request(id, |server| server.references(&params)),
            "textDocument/formatting" => self.request(id, |server| server.formatting(&params)),
            "workspace/executeCommand" => {
                self.request(id, |_server| Self::execute_command(&params))
            }
            _ => id.map_or_else(Vec::new, |id| {
                vec![error(id, -32601, "method is not supported")]
            }),
        }
    }

    fn request<F>(&self, id: Option<Value>, operation: F) -> Vec<Value>
    where
        F: FnOnce(&Self) -> Result<Value, ProtocolError>,
    {
        id.map_or_else(Vec::new, |id| match operation(self) {
            Ok(result) => vec![success(id, result)],
            Err(failure) => vec![error(id, failure.code, failure.message)],
        })
    }

    fn initialize(&mut self, id: Option<Value>) -> Vec<Value> {
        let Some(id) = id else {
            return Vec::new();
        };
        if !matches!(self.lifecycle, Lifecycle::New) {
            return vec![error(id, -32600, "initialize may only be sent once")];
        }
        self.lifecycle = Lifecycle::Ready;
        vec![success(
            id,
            json!({
                "capabilities": {
                    "positionEncoding": "utf-16",
                    "textDocumentSync": { "openClose": true, "change": 1 },
                    "diagnosticProvider": {
                        "identifier": "sico-compiler",
                        "interFileDependencies": false,
                        "workspaceDiagnostics": false
                    },
                    "documentSymbolProvider": true,
                    "completionProvider": { "triggerCharacters": ["."] },
                    "hoverProvider": true,
                    "definitionProvider": true,
                    "referencesProvider": true,
                    "documentFormattingProvider": true,
                    "executeCommandProvider": {
                        "commands": ["sico.check", "sico.run", "sico.debug"]
                    }
                },
                "serverInfo": { "name": "sico-language-server", "version": env!("CARGO_PKG_VERSION") }
            }),
        )]
    }

    fn did_open(&mut self, params: &Value) -> Vec<Value> {
        let Some(item) = params.get("textDocument") else {
            return Vec::new();
        };
        let Some(uri) = item.get("uri").and_then(Value::as_str) else {
            return Vec::new();
        };
        let Some(text) = item.get("text").and_then(Value::as_str) else {
            return Vec::new();
        };
        let version = item.get("version").and_then(Value::as_i64).unwrap_or(0);
        let existing_bytes = self
            .documents
            .get(uri)
            .map_or(0, |document| document.text.len());
        let projected_bytes = self
            .workspace_source_bytes()
            .saturating_sub(existing_bytes)
            .saturating_add(text.len());
        if !valid_uri(uri)
            || text.len() > MAX_MESSAGE_BYTES
            || projected_bytes > MAX_WORKSPACE_SOURCE_BYTES
            || (!self.documents.contains_key(uri) && self.documents.len() >= MAX_OPEN_DOCUMENTS)
        {
            return vec![show_error("document exceeds language-server limits")];
        }
        self.documents.insert(
            uri.to_owned(),
            Document {
                version,
                text: text.to_owned(),
            },
        );
        vec![self.publish_diagnostics(uri)]
    }

    fn did_change(&mut self, params: &Value) -> Vec<Value> {
        let Some(identifier) = params.get("textDocument") else {
            return Vec::new();
        };
        let Some(uri) = identifier.get("uri").and_then(Value::as_str) else {
            return Vec::new();
        };
        let Some(version) = identifier.get("version").and_then(Value::as_i64) else {
            return vec![show_error("document change requires a version")];
        };
        let workspace_bytes = self.workspace_source_bytes();
        let Some(document) = self.documents.get_mut(uri) else {
            return vec![show_error("document is not open")];
        };
        if version <= document.version {
            return vec![show_error("stale document version was rejected")];
        }
        let Some(changes) = params.get("contentChanges").and_then(Value::as_array) else {
            return vec![show_error("document change requires contentChanges")];
        };
        if changes.len() != 1 || changes[0].get("range").is_some() {
            return vec![show_error("only one full-document change is supported")];
        }
        let Some(text) = changes[0].get("text").and_then(Value::as_str) else {
            return vec![show_error("document change requires text")];
        };
        let projected_bytes = workspace_bytes
            .saturating_sub(document.text.len())
            .saturating_add(text.len());
        if text.len() > MAX_MESSAGE_BYTES || projected_bytes > MAX_WORKSPACE_SOURCE_BYTES {
            return vec![show_error("document exceeds language-server limits")];
        }
        document.version = version;
        text.clone_into(&mut document.text);
        vec![self.publish_diagnostics(uri)]
    }

    fn workspace_source_bytes(&self) -> usize {
        self.documents
            .values()
            .map(|document| document.text.len())
            .sum()
    }

    fn did_close(&mut self, params: &Value) -> Vec<Value> {
        let Some(uri) = document_uri(params) else {
            return Vec::new();
        };
        self.documents.remove(uri);
        vec![json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": [] }
        })]
    }

    fn source(&self, uri: &str) -> Result<SourceFile, ProtocolError> {
        let document = self
            .documents
            .get(uri)
            .ok_or_else(|| ProtocolError::invalid("document is not open"))?;
        SourceFile::from_text(SourceId::new(0), uri, document.text.clone())
            .map_err(|_| ProtocolError::invalid("document violates the source contract"))
    }

    fn publish_diagnostics(&self, uri: &str) -> Value {
        let diagnostics = self.diagnostics(uri).unwrap_or_else(|failure| {
            vec![json!({
                "range": zero_range(),
                "severity": 1,
                "code": "E0001",
                "source": "sico",
                "message": failure.message
            })]
        });
        let version = self
            .documents
            .get(uri)
            .map_or(0, |document| document.version);
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "version": version, "diagnostics": diagnostics }
        })
    }

    fn diagnostics(&self, uri: &str) -> Result<Vec<Value>, ProtocolError> {
        let source = self.source(uri)?;
        let parsed = parse(&source);
        if !parsed.lex_errors().is_empty() {
            return Ok(parsed
                .lex_errors()
                .iter()
                .filter_map(|diagnostic| {
                    Some(json!({
                        "range": lsp_range(source.text(), diagnostic.range)?,
                        "severity": 1,
                        "code": "E0002",
                        "source": "sico",
                        "message": "lexical input is invalid"
                    }))
                })
                .collect());
        }
        if !parsed.errors().is_empty() {
            return Ok(parsed
                .errors()
                .iter()
                .filter_map(|diagnostic| {
                    let identity = syntax_identity(&diagnostic.kind);
                    Some(json!({
                        "range": lsp_range(source.text(), diagnostic.range)?,
                        "severity": 1,
                        "code": identity.map_or("E1999", |value| value.code),
                        "source": "sico",
                        "message": identity.map_or("syntax error", |value| value.message),
                        "data": {
                            "key": identity.map_or("SYNTAX_ERROR", |value| value.key),
                            "recoveryAnchor": diagnostic.anchor.as_str()
                        }
                    }))
                })
                .collect());
        }
        let analysis = analyze(&source).map_err(|_| ProtocolError::internal("analysis failed"))?;
        Ok(analysis
            .diagnostics
            .iter()
            .filter_map(|diagnostic| {
                Some(json!({
                    "range": lsp_range(source.text(), diagnostic.range)?,
                    "severity": 1,
                    "code": diagnostic.code,
                    "source": "sico",
                    "message": diagnostic.message,
                    "data": { "key": diagnostic.key }
                }))
            })
            .collect())
    }

    fn pull_diagnostics(&self, params: &Value) -> Result<Value, ProtocolError> {
        let uri = required_uri(params)?;
        Ok(json!({ "kind": "full", "items": self.diagnostics(uri)? }))
    }

    fn index(&self) -> SemanticIndex {
        let inputs = self
            .documents
            .iter()
            .filter_map(|(uri, document)| {
                SourceFile::from_text(SourceId::new(0), uri, document.text.clone())
                    .ok()
                    .map(|source| IndexInput {
                        module_name: module_name(uri),
                        file: uri.clone(),
                        source,
                    })
            })
            .collect::<Vec<_>>();
        build_index("workspace", "0.0.0", &inputs)
    }

    fn document_symbols(&self, params: &Value) -> Result<Value, ProtocolError> {
        let uri = required_uri(params)?;
        let source = self.source(uri)?;
        let symbols = self
            .index()
            .symbols
            .into_iter()
            .filter(|symbol| symbol.source.file == uri)
            .filter_map(|symbol| {
                let range = index_range(source.text(), &symbol)?;
                Some(json!({
                    "name": symbol.name,
                    "kind": symbol_kind(&symbol.kind),
                    "range": range,
                    "selectionRange": range,
                    "detail": symbol.id
                }))
            })
            .collect::<Vec<_>>();
        Ok(json!(symbols))
    }

    fn completion(&self, params: &Value) -> Result<Value, ProtocolError> {
        let (uri, byte) = document_position(self, params)?;
        let prefix = identifier_prefix(self.documents[uri].text.as_str(), byte);
        let mut items = BTreeMap::<String, Value>::new();
        for symbol in self.index().symbols {
            if symbol.name.starts_with(prefix) {
                items.entry(symbol.name.clone()).or_insert_with(|| {
                    json!({
                        "label": symbol.name,
                        "kind": completion_kind(&symbol.kind),
                        "detail": symbol.id,
                        "data": { "symbolId": symbol.id }
                    })
                });
            }
        }
        for keyword in KEYWORDS {
            if keyword.starts_with(prefix) {
                items
                    .entry((*keyword).to_owned())
                    .or_insert_with(|| json!({ "label": keyword, "kind": 14 }));
            }
        }
        Ok(json!({
            "isIncomplete": items.len() > MAX_COMPLETIONS,
            "items": items.into_values().take(MAX_COMPLETIONS).collect::<Vec<_>>()
        }))
    }

    fn hover(&self, params: &Value) -> Result<Value, ProtocolError> {
        let (uri, byte) = document_position(self, params)?;
        let Some(name) = identifier_at(&self.documents[uri].text, byte) else {
            return Ok(Value::Null);
        };
        let matches = self
            .index()
            .symbols
            .into_iter()
            .filter(|symbol| symbol.name == name)
            .collect::<Vec<_>>();
        let [symbol] = matches.as_slice() else {
            return Ok(Value::Null);
        };
        let ty = symbol
            .facets
            .get("type")
            .and_then(|facet| facet.get("value"))
            .and_then(Value::as_str)
            .map_or_else(String::new, |ty| format!(" : {ty}"));
        Ok(json!({
            "contents": { "kind": "markdown", "value": format!("`{} {name}{ty}`\n\nCompiler Semantic Index: `{}`", symbol.kind, symbol.id) }
        }))
    }

    fn definition(&self, params: &Value) -> Result<Value, ProtocolError> {
        let (uri, byte) = document_position(self, params)?;
        let Some(name) = identifier_at(&self.documents[uri].text, byte) else {
            return Ok(Value::Null);
        };
        let matches = self
            .index()
            .symbols
            .into_iter()
            .filter(|symbol| symbol.name == name)
            .collect::<Vec<_>>();
        let [symbol] = matches.as_slice() else {
            return Ok(Value::Null);
        };
        Ok(self.symbol_location(symbol).unwrap_or(Value::Null))
    }

    fn references(&self, params: &Value) -> Result<Value, ProtocolError> {
        let (uri, byte) = document_position(self, params)?;
        let Some(name) = identifier_at(&self.documents[uri].text, byte) else {
            return Ok(json!([]));
        };
        if self
            .index()
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name)
            .count()
            != 1
        {
            return Ok(json!([]));
        }
        let mut references = Vec::new();
        for (document_uri, document) in &self.documents {
            for range in identifier_occurrences(&document.text, name) {
                if let Some(range) = lsp_range(&document.text, range) {
                    references.push(json!({ "uri": document_uri, "range": range }));
                    if references.len() == MAX_REFERENCES {
                        return Ok(json!(references));
                    }
                }
            }
        }
        Ok(json!(references))
    }

    fn symbol_location(&self, symbol: &Symbol) -> Option<Value> {
        let text = &self.documents.get(&symbol.source.file)?.text;
        Some(json!({
            "uri": symbol.source.file,
            "range": index_range(text, symbol)?
        }))
    }

    fn formatting(&self, params: &Value) -> Result<Value, ProtocolError> {
        let uri = required_uri(params)?;
        let source = self.source(uri)?;
        let formatted = format(&source).map_err(|_| {
            ProtocolError::invalid("formatting requires lexically and syntactically valid source")
        })?;
        if formatted == source.text() {
            return Ok(json!([]));
        }
        let end = byte_to_position(source.text(), source.text().len())
            .ok_or_else(|| ProtocolError::internal("formatter produced an invalid range"))?;
        Ok(json!([{
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": end
            },
            "newText": formatted
        }]))
    }

    fn execute_command(params: &Value) -> Result<Value, ProtocolError> {
        let command = params
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| ProtocolError::invalid("executeCommand requires command"))?;
        let program = params
            .get("arguments")
            .and_then(Value::as_array)
            .and_then(|arguments| arguments.first())
            .and_then(Value::as_str)
            .ok_or_else(|| ProtocolError::invalid("command requires one program path"))?;
        if program.is_empty() || program.len() > 4096 || program.chars().any(char::is_control) {
            return Err(ProtocolError::invalid("program path is invalid"));
        }
        let subcommand = match command {
            "sico.check" => "check",
            "sico.run" => "run",
            "sico.debug" => {
                return Err(ProtocolError::unavailable(
                    "source debugging is unavailable until Runtime pause/step/inspect hooks exist",
                ));
            }
            _ => return Err(ProtocolError::invalid("unknown Sico editor command")),
        };
        Ok(json!({
            "schema": "sico.editor-command.v0",
            "executable": "sico",
            "arguments": [subcommand, program],
            "shell": false,
            "cwd": null
        }))
    }
}

#[derive(Clone, Copy, Debug)]
struct ProtocolError {
    code: i64,
    message: &'static str,
}

impl ProtocolError {
    const fn invalid(message: &'static str) -> Self {
        Self {
            code: -32602,
            message,
        }
    }

    const fn internal(message: &'static str) -> Self {
        Self {
            code: -32603,
            message,
        }
    }

    const fn unavailable(message: &'static str) -> Self {
        Self {
            code: -32004,
            message,
        }
    }
}

fn document_uri(params: &Value) -> Option<&str> {
    params.get("textDocument")?.get("uri")?.as_str()
}

fn required_uri(params: &Value) -> Result<&str, ProtocolError> {
    document_uri(params).ok_or_else(|| ProtocolError::invalid("request requires document URI"))
}

fn document_position<'a>(
    server: &'a LanguageServer,
    params: &Value,
) -> Result<(&'a str, usize), ProtocolError> {
    let uri = required_uri(params)?;
    let (uri, document) = server
        .documents
        .get_key_value(uri)
        .ok_or_else(|| ProtocolError::invalid("document is not open"))?;
    let position = params
        .get("position")
        .ok_or_else(|| ProtocolError::invalid("request requires position"))?;
    let line = position
        .get("line")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| ProtocolError::invalid("position line is invalid"))?;
    let character = position
        .get("character")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| ProtocolError::invalid("position character is invalid"))?;
    let byte = position_to_byte(&document.text, line, character)
        .ok_or_else(|| ProtocolError::invalid("position is outside the document"))?;
    Ok((uri.as_str(), byte))
}

fn lsp_range(text: &str, range: TextRange) -> Option<Value> {
    Some(json!({
        "start": byte_to_position(text, usize::from(range.start()))?,
        "end": byte_to_position(text, usize::from(range.end()))?
    }))
}

fn index_range(text: &str, symbol: &Symbol) -> Option<Value> {
    let start = usize::try_from(symbol.source.range.start.byte).ok()?;
    let end = usize::try_from(symbol.source.range.end.byte).ok()?;
    Some(json!({
        "start": byte_to_position(text, start)?,
        "end": byte_to_position(text, end)?
    }))
}

fn byte_to_position(text: &str, byte: usize) -> Option<Value> {
    if byte > text.len() || !text.is_char_boundary(byte) {
        return None;
    }
    let mut line = 0;
    for value in text.as_bytes().iter().take(byte) {
        if *value == b'\n' {
            line += 1;
        }
    }
    let line_start = text.as_bytes()[..byte]
        .iter()
        .rposition(|value| *value == b'\n')
        .map_or(0, |position| position + 1);
    let character = text[line_start..byte]
        .chars()
        .map(char::len_utf16)
        .sum::<usize>();
    Some(json!({ "line": line, "character": character }))
}

fn position_to_byte(text: &str, target_line: usize, target_character: usize) -> Option<usize> {
    let mut line = 0;
    let mut line_start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if line == target_line {
            break;
        }
        if byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    if line != target_line {
        return None;
    }
    let mut line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |relative| line_start + relative);
    if line_end > line_start && text.as_bytes()[line_end - 1] == b'\r' {
        line_end -= 1;
    }
    if target_character == 0 {
        return Some(line_start);
    }
    let mut utf16 = 0;
    for (relative, character) in text[line_start..line_end].char_indices() {
        utf16 += character.len_utf16();
        if utf16 == target_character {
            return Some(line_start + relative + character.len_utf8());
        }
        if utf16 > target_character {
            return None;
        }
    }
    (utf16 == target_character).then_some(line_end)
}

fn identifier_prefix(text: &str, byte: usize) -> &str {
    let start = text[..byte]
        .char_indices()
        .rev()
        .find(|(_, character)| !is_identifier(*character))
        .map_or(0, |(index, character)| index + character.len_utf8());
    &text[start..byte]
}

fn identifier_at(text: &str, byte: usize) -> Option<&str> {
    if byte > text.len() || !text.is_char_boundary(byte) {
        return None;
    }
    let start = text[..byte]
        .char_indices()
        .rev()
        .find(|(_, character)| !is_identifier(*character))
        .map_or(0, |(index, character)| index + character.len_utf8());
    let end = text[byte..]
        .char_indices()
        .find(|(_, character)| !is_identifier(*character))
        .map_or(text.len(), |(relative, _)| byte + relative);
    (start < end).then(|| &text[start..end])
}

fn identifier_occurrences(text: &str, name: &str) -> Vec<TextRange> {
    let mut ranges = Vec::new();
    let mut offset = 0;
    while let Some(relative) = text[offset..].find(name) {
        let start = offset + relative;
        let end = start + name.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if before.is_none_or(|character| !is_identifier(character))
            && after.is_none_or(|character| !is_identifier(character))
            && let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end))
        {
            ranges.push(TextRange::new(start.into(), end.into()));
        }
        offset = end;
    }
    ranges
}

fn is_identifier(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
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

fn valid_uri(uri: &str) -> bool {
    !uri.is_empty()
        && uri.len() <= MAX_URI_BYTES
        && uri.starts_with("file://")
        && !uri.chars().any(char::is_control)
}

const fn symbol_kind(kind: &str) -> u8 {
    match kind.as_bytes() {
        b"function" => 12,
        b"field" => 8,
        b"parameter" => 13,
        b"variant" => 22,
        _ => 5,
    }
}

const fn completion_kind(kind: &str) -> u8 {
    match kind.as_bytes() {
        b"function" => 3,
        b"field" => 5,
        b"parameter" => 6,
        b"variant" => 20,
        _ => 7,
    }
}

fn success(id: Value, result: Value) -> Value {
    let mut response = serde_json::Map::new();
    response.insert("jsonrpc".to_owned(), Value::String("2.0".to_owned()));
    response.insert("id".to_owned(), id);
    response.insert("result".to_owned(), result);
    Value::Object(response)
}

fn error(id: Value, code: i64, message: &str) -> Value {
    let mut failure = serde_json::Map::new();
    failure.insert("code".to_owned(), Value::from(code));
    failure.insert("message".to_owned(), Value::String(message.to_owned()));
    let mut response = serde_json::Map::new();
    response.insert("jsonrpc".to_owned(), Value::String("2.0".to_owned()));
    response.insert("id".to_owned(), id);
    response.insert("error".to_owned(), Value::Object(failure));
    Value::Object(response)
}

fn show_error(message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "window/showMessage",
        "params": { "type": 1, "message": message }
    })
}

fn zero_range() -> Value {
    json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": 0, "character": 0 }
    })
}

const KEYWORDS: &[&str] = &[
    "async",
    "await",
    "capabilities",
    "capability",
    "case",
    "component",
    "effects",
    "end",
    "enum",
    "export",
    "field",
    "function",
    "if",
    "interface",
    "let",
    "match",
    "newtype",
    "record",
    "resource",
    "return",
    "task",
    "using",
];

/// Reads one LSP/DAP-style `Content-Length` framed JSON message.
///
/// # Errors
///
/// Rejects malformed headers, oversized bodies, truncated input, and invalid JSON.
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Value>> {
    let mut content_length = None;
    let mut has_content_type = false;
    let mut header_bytes = 0;
    loop {
        let Some(header) = read_header_line(reader, &mut header_bytes)? else {
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "truncated headers",
                ))
            };
        };
        if header == b"\r\n" {
            break;
        }
        if !header.is_ascii() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "headers require ASCII",
            ));
        }
        let header = std::str::from_utf8(&header[..header.len() - 2]).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "headers require valid ASCII")
        })?;
        let (name, value) = header
            .split_once(": ")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "malformed header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "duplicate Content-Length",
                ));
            }
            let length = value.parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?;
            if length > MAX_MESSAGE_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "message is too large",
                ));
            }
            content_length = Some(length);
        } else if name.eq_ignore_ascii_case("Content-Type") {
            if has_content_type {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "duplicate Content-Type",
                ));
            }
            if !matches!(
                value,
                "application/vscode-jsonrpc; charset=utf-8"
                    | "application/vscode-jsonrpc; charset=utf8"
            ) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported Content-Type",
                ));
            }
            has_content_type = true;
        }
    }
    let length = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid JSON body"))
}

fn read_header_line<R: BufRead>(reader: &mut R, total: &mut usize) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "truncated header line",
                ))
            };
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        if total.saturating_add(consumed) > MAX_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "headers are too large",
            ));
        }
        line.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        *total += consumed;
        if line.last() == Some(&b'\n') {
            if !line.ends_with(b"\r\n") {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "headers require CRLF",
                ));
            }
            return Ok(Some(line));
        }
    }
}

/// Writes one canonical compact framed JSON message.
///
/// # Errors
///
/// Returns underlying serialization or output errors.
pub fn write_message<W: Write>(writer: &mut W, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "cannot encode JSON"))?;
    if body.len() > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message is too large",
        ));
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        io::{BufReader, Cursor},
    };

    use super::*;

    fn request(id: i64, method: &str, params: Value) -> Value {
        let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });
        message["params"] = params;
        message
    }

    fn notification(method: &str, params: Value) -> Value {
        let mut message = json!({ "jsonrpc": "2.0", "method": method });
        message["params"] = params;
        message
    }

    fn ready() -> LanguageServer {
        let mut server = LanguageServer::default();
        let response = server.process(request(1, "initialize", json!({})));
        assert_eq!(
            response[0]["result"]["capabilities"]["positionEncoding"],
            "utf-16"
        );
        server
    }

    fn open(server: &mut LanguageServer, uri: &str, version: i64, text: &str) -> Vec<Value> {
        server.process(notification(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "sico", "version": version, "text": text } }),
        ))
    }

    #[test]
    fn lifecycle_capabilities_and_unknown_methods_are_bounded() {
        let mut server = LanguageServer::default();
        assert_eq!(
            server.process(request(1, "textDocument/hover", json!({})))[0]["error"]["code"],
            -32002
        );
        let initialized = server.process(request(2, "initialize", json!({})));
        assert_eq!(
            initialized[0]["result"]["capabilities"]["textDocumentSync"]["change"],
            1
        );
        assert_eq!(
            server.process(request(3, "unknown", json!({})))[0]["error"]["code"],
            -32601
        );
        assert_eq!(
            server.process(request(4, "shutdown", json!({})))[0]["result"],
            Value::Null
        );
        assert!(server.process(notification("exit", json!({}))).is_empty());
        assert!(server.has_exited());
    }

    #[test]
    fn diagnostics_versions_and_utf16_positions_are_correct() {
        let mut server = ready();
        let uri = "file:///workspace/emoji.sico";
        let published = open(
            &mut server,
            uri,
            1,
            "// 😀\nfunction main() returns Int:\n  return 1\n",
        );
        assert_eq!(published[0]["params"]["diagnostics"][0]["code"], "E1001");
        assert_eq!(published[0]["params"]["version"], 1);
        assert_eq!(
            byte_to_position("😀x", 4).unwrap(),
            json!({ "line": 0, "character": 2 })
        );
        assert_eq!(position_to_byte("😀x", 0, 2), Some(4));
        assert_eq!(position_to_byte("😀x", 0, 1), None);

        let stale = server.process(notification(
            "textDocument/didChange",
            json!({ "textDocument": { "uri": uri, "version": 1 }, "contentChanges": [{ "text": "" }] }),
        ));
        assert_eq!(stale[0]["method"], "window/showMessage");
        assert_eq!(server.documents[uri].version, 1);
    }

    #[test]
    fn semantic_index_drives_symbols_completion_hover_definition_and_references() {
        let mut server = ready();
        let uri = "file:///workspace/main.sico";
        let source = "function helper(value: Int) returns Int:\n  return value\nend function\n\nfunction main() returns Int:\n  return helper(1)\nend function\n";
        open(&mut server, uri, 1, source);

        let symbols = server.process(request(
            2,
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": uri } }),
        ));
        assert!(
            symbols[0]["result"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["name"] == "helper")
        );

        let completion = server.process(request(
            3,
            "textDocument/completion",
            json!({ "textDocument": { "uri": uri }, "position": { "line": 5, "character": 11 } }),
        ));
        assert!(
            completion[0]["result"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["label"] == "helper")
        );

        for (id, method) in [(4, "textDocument/hover"), (5, "textDocument/definition")] {
            let response = server.process(request(
                id,
                method,
                json!({ "textDocument": { "uri": uri }, "position": { "line": 5, "character": 12 } }),
            ));
            assert!(!response[0]["result"].is_null(), "{method}");
        }
        let references = server.process(request(
            6,
            "textDocument/references",
            json!({ "textDocument": { "uri": uri }, "position": { "line": 5, "character": 12 }, "context": { "includeDeclaration": true } }),
        ));
        assert_eq!(references[0]["result"].as_array().unwrap().len(), 2);

        let second_uri = "file:///workspace/other/main.sico";
        open(
            &mut server,
            second_uri,
            1,
            "function helper() returns Int:\n  return 2\nend function\n",
        );
        let index = server.index();
        assert_eq!(index.modules.len(), 2);
        assert_ne!(index.modules[0].id, index.modules[1].id);
        let ambiguous = server.process(request(
            7,
            "textDocument/definition",
            json!({ "textDocument": { "uri": uri }, "position": { "line": 5, "character": 12 } }),
        ));
        assert!(ambiguous[0]["result"].is_null());
    }

    #[test]
    fn formatter_and_editor_commands_are_deterministic_and_never_use_a_shell() {
        let mut server = ready();
        let uri = "file:///workspace/main.sico";
        open(
            &mut server,
            uri,
            1,
            "function  main ( ) returns Int :\nreturn 1\nend function\n",
        );
        let formatted = server.process(request(
            2,
            "textDocument/formatting",
            json!({ "textDocument": { "uri": uri }, "options": { "tabSize": 2, "insertSpaces": true } }),
        ));
        assert_eq!(
            formatted[0]["result"][0]["newText"],
            "function main() returns Int:\n  return 1\nend function\n"
        );

        let command = server.process(request(
            3,
            "workspace/executeCommand",
            json!({ "command": "sico.run", "arguments": ["path with spaces/app.sico"] }),
        ));
        assert_eq!(
            command[0]["result"]["arguments"],
            json!(["run", "path with spaces/app.sico"])
        );
        assert_eq!(command[0]["result"]["shell"], false);
        let debug = server.process(request(
            4,
            "workspace/executeCommand",
            json!({ "command": "sico.debug", "arguments": ["app.sico"] }),
        ));
        assert_eq!(debug[0]["error"]["code"], -32004);
    }

    #[test]
    fn framing_roundtrips_and_rejects_malformed_or_oversized_input() {
        let message = request(1, "initialize", json!({}));
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).unwrap();
        let mut reader = BufReader::new(Cursor::new(bytes));
        assert_eq!(read_message(&mut reader).unwrap(), Some(message));
        assert_eq!(read_message(&mut reader).unwrap(), None);

        for invalid in [
            b"Content-Length: nope\r\n\r\n{}".as_slice(),
            b"Content-Length: 2\n\n{}".as_slice(),
            b"Content-Type: application/json\r\n\r\n{}".as_slice(),
            b"Content-Length: 1\r\n\r\n{".as_slice(),
        ] {
            let mut reader = BufReader::new(Cursor::new(invalid));
            assert!(read_message(&mut reader).is_err());
        }
        let oversized = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE_BYTES + 1);
        let mut reader = BufReader::new(Cursor::new(oversized));
        assert!(read_message(&mut reader).is_err());
        let oversized_header = format!("X-Long: {}\r\n\r\n", "x".repeat(MAX_HEADER_BYTES));
        let mut reader = BufReader::new(Cursor::new(oversized_header));
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn document_count_change_shape_and_close_are_bounded() {
        let mut server = ready();
        for index in 0..MAX_OPEN_DOCUMENTS {
            let uri = format!("file:///workspace/{index}.sico");
            assert_eq!(
                open(&mut server, &uri, 1, "")[0]["method"],
                "textDocument/publishDiagnostics"
            );
        }
        let rejected = open(&mut server, "file:///workspace/overflow.sico", 1, "");
        assert_eq!(rejected[0]["method"], "window/showMessage");
        assert_eq!(server.documents.len(), MAX_OPEN_DOCUMENTS);

        let uri = "file:///workspace/0.sico";
        let incremental = server.process(notification(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "range": zero_range(), "text": "x" }]
            }),
        ));
        assert_eq!(incremental[0]["method"], "window/showMessage");
        assert_eq!(server.documents[uri].text, "");
        let closed = server.process(notification(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        ));
        assert_eq!(closed[0]["params"]["diagnostics"], json!([]));
        assert!(!server.documents.contains_key(uri));
    }

    #[test]
    fn five_hundred_twelve_protocol_mutations_fail_closed() {
        let mut rejected = 0;
        for length in 0..256 {
            let frame = format!("Content-Length: {length}\r\n\r\n{}", "{".repeat(length));
            let mut reader = BufReader::new(Cursor::new(frame));
            rejected += usize::from(read_message(&mut reader).is_err());
        }
        for delta in 1..=256 {
            let frame = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE_BYTES + delta);
            let mut reader = BufReader::new(Cursor::new(frame));
            rejected += usize::from(read_message(&mut reader).is_err());
        }
        assert_eq!(rejected, 512);
    }

    #[test]
    fn requests_are_deterministic_under_key_order_and_unknown_fields() {
        let mut first = ready();
        let mut second = ready();
        let left = first.process(request(
            9,
            "workspace/executeCommand",
            json!({
                "command": "sico.check", "arguments": ["main.sico"], "unknown": [1, 2, 3]
            }),
        ));
        let right = second.process(json!({
            "params": { "unknown": [1, 2, 3], "arguments": ["main.sico"], "command": "sico.check" },
            "method": "workspace/executeCommand", "id": 9, "jsonrpc": "2.0"
        }));
        assert_eq!(left, right);
        let labels = KEYWORDS.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(labels.len(), KEYWORDS.len());
    }
}
