//! RFC-0002 compiler-produced Semantic Index and bounded query engine.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fmt, fmt::Write as _};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sico_semantics::{SemanticFactKind, analyze};
use sico_source::{SourceFile, TextRange, TextSize};

pub const INDEX_SCHEMA: &str = "sico.semantic-index.v0";
pub const QUERY_SCHEMA: &str = "sico.semantic-query.v0";
pub const RESPONSE_SCHEMA: &str = "sico.semantic-response.v0";

pub struct IndexInput {
    pub module_name: String,
    pub file: String,
    pub source: SourceFile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SemanticIndex {
    pub schema: &'static str,
    pub protocol_version: u8,
    pub producer: Producer,
    pub snapshot: Snapshot,
    pub package: Package,
    pub modules: Vec<ModuleEntry>,
    pub symbols: Vec<Symbol>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Producer {
    pub name: &'static str,
    pub version: &'static str,
    pub mode: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    pub id: String,
    pub quality: Quality,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModuleEntry {
    pub id: String,
    pub name: String,
    pub source_file: String,
    pub quality: Quality,
    pub facets: Map<String, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Symbol {
    pub id: String,
    pub module_id: String,
    pub kind: String,
    pub name: String,
    pub visibility: &'static str,
    pub source: SourceRange,
    pub quality: Quality,
    pub facets: Map<String, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Relation {
    pub kind: &'static str,
    pub from: String,
    pub to: String,
    pub basis: &'static str,
    pub evidence: Vec<Evidence>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Quality {
    pub state: &'static str,
    pub reasons: Vec<String>,
    pub blocked_by: Vec<String>,
}

impl Quality {
    fn complete() -> Self {
        Self {
            state: "complete",
            reasons: Vec::new(),
            blocked_by: Vec::new(),
        }
    }

    fn partial(reasons: Vec<String>, blocked_by: Vec<String>) -> Self {
        Self {
            state: "partial",
            reasons,
            blocked_by,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceRange {
    pub file: String,
    pub range: CoordinateRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CoordinateRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Position {
    pub byte: u32,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Evidence {
    pub kind: &'static str,
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QueryRequest {
    pub schema: String,
    pub protocol_version: u8,
    pub request_id: String,
    pub snapshot_id: String,
    pub operation: Operation,
    pub target: QueryTarget,
    #[serde(default)]
    pub parameters: Value,
    pub options: QueryOptions,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Outline,
    Describe,
    Slice,
    Impact,
    Flow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QueryTarget {
    pub id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QueryOptions {
    pub max_depth: u8,
    pub max_items: usize,
    pub max_bytes: usize,
    pub include_private: bool,
    pub include_source: bool,
    pub transitive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryError {
    InvalidMetadata,
    SnapshotMismatch,
    UnknownTarget,
    InvalidBudget,
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidMetadata => "unsupported semantic query metadata",
            Self::SnapshotMismatch => "semantic query snapshot mismatch",
            Self::UnknownTarget => "semantic query target is not in the snapshot",
            Self::InvalidBudget => "semantic query budget is outside v0 limits",
        })
    }
}

/// Builds a deterministic compiler snapshot from real semantic analysis.
#[must_use]
pub fn build_index(package_name: &str, version: &str, inputs: &[IndexInput]) -> SemanticIndex {
    let package_identity = identity_segment(package_name);
    let package_id = format!("sico://{package_identity}/package");
    let mut modules = Vec::new();
    let mut symbols = Vec::new();
    let mut relations = Vec::new();
    let mut ordered: Vec<_> = inputs.iter().collect();
    ordered.sort_by_key(|input| (&input.module_name, &input.file));

    for input in ordered {
        let indexed = index_module(&package_identity, input);
        modules.push(indexed.module);
        symbols.extend(indexed.symbols);
        relations.extend(indexed.relations);
    }
    modules.sort_by(|left, right| left.id.cmp(&right.id));
    symbols.sort_by(|left, right| left.id.cmp(&right.id));
    symbols.dedup_by(|left, right| left.id == right.id);
    let known: BTreeSet<_> = symbols.iter().map(|symbol| symbol.id.as_str()).collect();
    relations.retain(|relation| known.contains(relation.to.as_str()));
    relations.sort_by(|left, right| {
        (&left.from, &left.to, left.kind).cmp(&(&right.from, &right.to, right.kind))
    });
    relations.dedup_by(|left, right| {
        left.from == right.from && left.to == right.to && left.kind == right.kind
    });
    let incomplete: Vec<_> = modules
        .iter()
        .filter(|module| module.quality.state != "complete")
        .collect();
    let snapshot_quality = if incomplete.is_empty() {
        Quality::complete()
    } else {
        Quality::partial(
            vec!["incomplete_modules".to_owned()],
            incomplete
                .iter()
                .flat_map(|module| module.quality.blocked_by.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        )
    };
    let snapshot_id = snapshot_id(package_name, version, inputs);
    SemanticIndex {
        schema: INDEX_SCHEMA,
        protocol_version: 0,
        producer: Producer {
            name: "sico-index",
            version: env!("CARGO_PKG_VERSION"),
            mode: "compiler",
        },
        snapshot: Snapshot {
            id: snapshot_id,
            quality: snapshot_quality,
        },
        package: Package {
            id: package_id,
            name: package_name.to_owned(),
            version: version.to_owned(),
        },
        modules,
        symbols,
        relations,
    }
}

struct IndexedModule {
    module: ModuleEntry,
    symbols: Vec<Symbol>,
    relations: Vec<Relation>,
}

fn index_module(package_identity: &str, input: &IndexInput) -> IndexedModule {
    let module_segment = identity_segment(&input.module_name);
    let module_id = format!("sico://{package_identity}/{module_segment}");
    let analysis = analyze(&input.source);
    let (quality, compiler_facts) = match analysis {
        Ok(analysis) if analysis.diagnostics.is_empty() => (Quality::complete(), analysis.facts),
        Ok(analysis) => {
            let blocked_by: Vec<_> = analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.to_owned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            (
                Quality::partial(vec!["semantic_diagnostics".to_owned()], blocked_by),
                analysis.facts,
            )
        }
        Err(_) => (
            Quality::partial(vec!["syntax_or_lexical_errors".to_owned()], Vec::new()),
            Vec::new(),
        ),
    };
    let module = ModuleEntry {
        id: module_id.clone(),
        name: input.module_name.clone(),
        source_file: normalize_file(&input.file),
        quality: quality.clone(),
        facets: Map::from_iter([(
            "compiler_facts".to_owned(),
            fact_value(
                "verified",
                &json!(compiler_facts.len()),
                "compiler:sico-semantics",
            ),
        )]),
    };
    let mut symbols = Vec::new();
    let mut relations = Vec::new();
    for fact in compiler_facts {
        let Some((kind, name)) = indexable_fact(fact.kind, &fact.name) else {
            continue;
        };
        let range = exact_name_range(&input.source, fact.range, &name).unwrap_or(fact.range);
        let id = format!(
            "{module_id}/{kind}/{}",
            percent_encode(fact.name.as_bytes())
        );
        let mut symbol_facets = Map::new();
        if let Some(ty) = fact.ty {
            symbol_facets.insert(
                "type".to_owned(),
                fact_value(
                    "verified",
                    &json!(ty.to_string()),
                    "compiler:sico-semantics",
                ),
            );
        }
        symbols.push(Symbol {
            id: id.clone(),
            module_id: module_id.clone(),
            kind: kind.to_owned(),
            name,
            visibility: visibility(&input.source, fact.range),
            source: source_range(&input.source, &input.file, range),
            quality: quality.clone(),
            facets: symbol_facets,
        });
        relations.push(Relation {
            kind: "contains",
            from: module_id.clone(),
            to: id,
            basis: "verified",
            evidence: vec![Evidence {
                kind: "analysis",
                reference: "compiler:sico-semantics".to_owned(),
            }],
        });
    }
    IndexedModule {
        module,
        symbols,
        relations,
    }
}

/// Executes one bounded RFC-0002 query over a matching snapshot.
///
/// # Errors
///
/// Returns a typed error for bad metadata, stale snapshots, unknown targets, or invalid budgets.
pub fn execute(index: &SemanticIndex, request: &QueryRequest) -> Result<Value, QueryError> {
    validate_request(index, request)?;
    let mut ids = select_ids(index, request);
    ids.sort();
    ids.dedup();
    let total = ids.len();
    ids.truncate(request.options.max_items);
    let mut items: Vec<_> = ids
        .iter()
        .filter_map(|id| query_item(index, id, request.operation))
        .collect();
    let selected: BTreeSet<_> = ids.iter().map(String::as_str).collect();
    let edges: Vec<_> = index
        .relations
        .iter()
        .filter(|relation| {
            selected.contains(relation.from.as_str()) && selected.contains(relation.to.as_str())
        })
        .map(|relation| json!(relation))
        .collect();
    let target_quality = target_quality(index, &request.target.id);
    let mut omitted = total.saturating_sub(items.len());
    let mut truncated = omitted > 0;
    let mut result = query_result(request, &items, &edges, target_quality, truncated, omitted);
    while canonical_json(&result).len() > request.options.max_bytes && !items.is_empty() {
        items.pop();
        omitted += 1;
        truncated = true;
        result = query_result(request, &items, &edges, target_quality, truncated, omitted);
    }
    let used_bytes = canonical_json(&result).len();
    Ok(json!({
        "schema": RESPONSE_SCHEMA,
        "protocol_version": 0,
        "request_id": request.request_id,
        "snapshot_id": request.snapshot_id,
        "operation": request.operation,
        "budget": {
            "max_depth": request.options.max_depth,
            "max_items": request.options.max_items,
            "max_bytes": request.options.max_bytes,
            "used_depth": usize::from(!items.is_empty()),
            "used_items": items.len(),
            "used_bytes": used_bytes,
            "truncated": truncated,
            "omitted_items": omitted
        },
        "result": result
    }))
}

fn validate_request(index: &SemanticIndex, request: &QueryRequest) -> Result<(), QueryError> {
    if request.schema != QUERY_SCHEMA || request.protocol_version != 0 {
        return Err(QueryError::InvalidMetadata);
    }
    if request.snapshot_id != index.snapshot.id {
        return Err(QueryError::SnapshotMismatch);
    }
    if !known_id(index, &request.target.id) {
        return Err(QueryError::UnknownTarget);
    }
    if request.options.max_depth > 16
        || request.options.max_items == 0
        || request.options.max_items > 1000
        || request.options.max_bytes < 1024
        || request.options.max_bytes > 1_048_576
    {
        return Err(QueryError::InvalidBudget);
    }
    Ok(())
}

fn select_ids(index: &SemanticIndex, request: &QueryRequest) -> Vec<String> {
    match request.operation {
        Operation::Outline => index
            .modules
            .iter()
            .map(|module| module.id.clone())
            .collect(),
        Operation::Describe => vec![request.target.id.clone()],
        Operation::Slice | Operation::Flow => {
            let mut ids = vec![request.target.id.clone()];
            for relation in &index.relations {
                if relation.from == request.target.id {
                    ids.push(relation.to.clone());
                }
                if relation.to == request.target.id {
                    ids.push(relation.from.clone());
                }
            }
            ids
        }
        Operation::Impact => {
            let mut ids = vec![request.target.id.clone()];
            ids.extend(
                index
                    .relations
                    .iter()
                    .filter(|relation| relation.to == request.target.id)
                    .map(|relation| relation.from.clone()),
            );
            ids
        }
    }
}

fn query_item(index: &SemanticIndex, id: &str, operation: Operation) -> Option<Value> {
    let role = match operation {
        Operation::Outline => "module",
        Operation::Describe => "target",
        Operation::Slice => "slice",
        Operation::Impact => "impact",
        Operation::Flow => "flow",
    };
    if let Some(module) = index.modules.iter().find(|module| module.id == id) {
        return Some(json!({
            "id": module.id,
            "kind": "module",
            "name": module.name,
            "role": role,
            "completeness": module.quality.state,
            "facets": module.facets
        }));
    }
    index
        .symbols
        .iter()
        .find(|symbol| symbol.id == id)
        .map(|symbol| {
            json!({
                "id": symbol.id,
                "kind": symbol.kind,
                "name": symbol.name,
                "role": role,
                "completeness": symbol.quality.state,
                "facets": symbol.facets,
                "source": symbol.source
            })
        })
}

fn query_result(
    request: &QueryRequest,
    items: &[Value],
    edges: &[Value],
    quality: &Quality,
    truncated: bool,
    omitted: usize,
) -> Value {
    let complete = quality.state == "complete" && !truncated;
    let mut reasons = quality.reasons.clone();
    if truncated {
        reasons.push(format!("budget omitted {omitted} items"));
    }
    json!({
        "target": request.target.id,
        "completeness": if complete { "complete" } else { "partial" },
        "reasons": reasons,
        "blocked_by": quality.blocked_by,
        "items": items,
        "edges": edges,
        "next_queries": []
    })
}

fn known_id(index: &SemanticIndex, id: &str) -> bool {
    id == index.package.id
        || index.modules.iter().any(|module| module.id == id)
        || index.symbols.iter().any(|symbol| symbol.id == id)
}

fn target_quality<'a>(index: &'a SemanticIndex, id: &str) -> &'a Quality {
    if id == index.package.id {
        return &index.snapshot.quality;
    }
    if let Some(module) = index.modules.iter().find(|module| module.id == id) {
        return &module.quality;
    }
    &index
        .symbols
        .iter()
        .find(|symbol| symbol.id == id)
        .expect("validated target exists")
        .quality
}

fn indexable_fact(kind: SemanticFactKind, qualified_name: &str) -> Option<(&'static str, String)> {
    let kind = match kind {
        SemanticFactKind::Type => "type",
        SemanticFactKind::Function => "function",
        SemanticFactKind::Parameter => "parameter",
        SemanticFactKind::Field => "field",
        SemanticFactKind::Variant => "variant",
        _ => return None,
    };
    let name = qualified_name
        .rsplit_once('.')
        .map_or(qualified_name, |(_, name)| name)
        .to_owned();
    Some((kind, name))
}

fn exact_name_range(source: &SourceFile, range: TextRange, name: &str) -> Option<TextRange> {
    let start = usize::from(range.start());
    let end = usize::from(range.end());
    let offset = source.text().get(start..end)?.find(name)? + start;
    Some(TextRange::new(
        TextSize::try_from(offset).ok()?,
        TextSize::try_from(offset + name.len()).ok()?,
    ))
}

fn source_range(source: &SourceFile, file: &str, range: TextRange) -> SourceRange {
    SourceRange {
        file: normalize_file(file),
        range: CoordinateRange {
            start: position(source, range.start()),
            end: position(source, range.end()),
        },
    }
}

fn position(source: &SourceFile, offset: TextSize) -> Position {
    let line_col = source
        .line_index()
        .line_col(source.text(), offset)
        .expect("semantic fact ranges are scalar boundaries");
    Position {
        byte: u32::from(offset),
        line: line_col.line,
        column: line_col.column,
    }
}

fn visibility(source: &SourceFile, range: TextRange) -> &'static str {
    if source
        .text()
        .get(usize::from(range.start())..usize::from(range.end()))
        .is_some_and(|text| text.trim_start().starts_with("export "))
    {
        "public"
    } else {
        "private"
    }
}

fn fact_value(basis: &str, value: &Value, evidence: &str) -> Value {
    json!({
        "basis": basis,
        "value": value,
        "evidence": [{ "kind": "analysis", "ref": evidence }]
    })
}

fn identity_segment(value: &str) -> String {
    let normalized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    normalized.trim_matches('-').to_owned()
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'~' | b'-') {
            output.push(char::from(*byte));
        } else {
            write!(output, "%{byte:02X}").expect("writing to a string cannot fail");
        }
    }
    output
}

fn normalize_file(file: &str) -> String {
    file.replace('\\', "/")
}

fn snapshot_id(package: &str, version: &str, inputs: &[IndexInput]) -> String {
    let mut descriptor = format!("sico-index-v0\0{package}\0{version}\0").into_bytes();
    let mut ordered: Vec<_> = inputs.iter().collect();
    ordered.sort_by_key(|input| (&input.module_name, &input.file));
    for input in ordered {
        descriptor.extend_from_slice(input.module_name.as_bytes());
        descriptor.push(0);
        descriptor.extend_from_slice(normalize_file(&input.file).as_bytes());
        descriptor.push(0);
        descriptor.extend_from_slice(input.source.text().as_bytes());
        descriptor.push(0);
    }
    format!("sha256:{}", sha256_hex(&descriptor))
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).expect("JSON scalar serialization cannot fail")
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            format!(
                "{{{}}}",
                keys.into_iter()
                    .map(|key| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("JSON key serialization cannot fail"),
                        canonical_json(&map[key])
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

#[allow(clippy::many_single_char_names)]
#[allow(clippy::too_many_lines)]
fn sha256_hex(input: &[u8]) -> String {
    const INITIAL: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let bit_length = u64::try_from(input.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    let mut state = INITIAL;
    for chunk in message.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, word) in words[..16].iter_mut().enumerate() {
            *word = u32::from_be_bytes(chunk[index * 4..index * 4 + 4].try_into().unwrap());
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (value, addition) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *value = value.wrapping_add(addition);
        }
    }
    state.iter().fold(String::new(), |mut output, value| {
        write!(output, "{value:08x}").expect("writing to a string cannot fail");
        output
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use sico_source::SourceId;

    use super::*;

    #[test]
    fn sha256_implementation_matches_standard_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn ten_b_modules_produce_stable_complete_schema_conforming_index() {
        let root = repository();
        let paths = valid_b_paths();
        let inputs = load_inputs(&root, &paths);
        let index = build_index("m2-semantic", "0.0.0", &inputs);
        assert_eq!(index, build_index("m2-semantic", "0.0.0", &inputs));
        assert_eq!(index.modules.len(), 10);
        assert_eq!(index.snapshot.quality.state, "complete");
        assert!(index.snapshot.id.starts_with("sha256:"));
        assert_eq!(index.snapshot.id.len(), 71);
        assert!(!index.symbols.is_empty());
        assert!(index.symbols.windows(2).all(|pair| pair[0].id < pair[1].id));
        for symbol in &index.symbols {
            assert!(!symbol.id.contains("byte="));
            let input = inputs
                .iter()
                .find(|input| normalize_file(&input.file) == symbol.source.file)
                .unwrap();
            let start = usize::try_from(symbol.source.range.start.byte).unwrap();
            let end = usize::try_from(symbol.source.range.end.byte).unwrap();
            assert_eq!(&input.source.text()[start..end], symbol.name);
        }
        assert_index_schema_contract(&index);
    }

    #[test]
    fn partial_and_blocking_quality_are_honest() {
        let root = repository();
        let matrix: Value = serde_json::from_str(
            &fs::read_to_string(root.join("semantic-index/sample-matrix.json")).unwrap(),
        )
        .unwrap();
        let paths: Vec<_> = matrix["samples"]
            .as_array()
            .unwrap()
            .iter()
            .map(|sample| sample["source"].as_str().unwrap().to_owned())
            .collect();
        let examples = load_inputs(&root, &paths);
        let partial = build_index("examples", "0.0.0", &examples);
        assert_eq!(partial.modules.len(), 10);
        assert_eq!(partial.snapshot.quality.state, "partial");
        assert!(
            partial
                .modules
                .iter()
                .all(|module| module.quality.state == "partial")
        );

        let invalid = load_inputs(
            &root,
            &["syntax-candidates/b/numbers-units/invalid/text-as-int.sico".to_owned()],
        );
        let blocked = build_index("blocked", "0.0.0", &invalid);
        assert_eq!(blocked.modules[0].quality.blocked_by, ["E2001"]);
        assert_eq!(blocked.snapshot.quality.blocked_by, ["E2001"]);
    }

    /// RFC-0036 §7: scope-qualified `AsyncState` facts ride the existing
    /// compiler fact stream; the index exposes them through the module
    /// `compiler_facts` facet without any new query authority.
    #[test]
    fn task_scope_facts_ride_the_existing_fact_stream() {
        let root = repository();
        let inputs = load_inputs(
            &root,
            &["syntax-candidates/b/future-task/valid/structured-pair.sico".to_owned()],
        );
        let facts = sico_semantics::analyze(&inputs[0].source).unwrap().facts;
        assert!(
            facts
                .iter()
                .any(|fact| fact.name == "scope-0:first->pending")
                && facts
                    .iter()
                    .any(|fact| fact.name == "scope-0:second->collected"),
            "{:?}",
            facts.iter().map(|fact| &fact.name).collect::<Vec<_>>()
        );
        let index = build_index("tasks", "0.0.0", &inputs);
        assert_eq!(index.modules[0].quality.state, "complete");
        assert_eq!(
            index.modules[0].facets["compiler_facts"]["value"],
            json!(facts.len())
        );
    }

    #[test]
    fn all_five_queries_share_snapshot_and_enforce_budgets() {
        let root = repository();
        let inputs = load_inputs(&root, &valid_b_paths());
        let index = build_index("m2-semantic", "0.0.0", &inputs);
        let symbol = index.symbols[0].id.clone();
        for operation in [
            Operation::Outline,
            Operation::Describe,
            Operation::Slice,
            Operation::Impact,
            Operation::Flow,
        ] {
            let target = if operation == Operation::Outline {
                index.package.id.clone()
            } else {
                symbol.clone()
            };
            let request = request(&index, operation, target, 64, 65_536);
            let response = execute(&index, &request).unwrap();
            assert_schema_required_contract(
                &serde_json::to_value(&request).unwrap(),
                "query-v0.schema.json",
            );
            assert_schema_required_contract(&response, "response-v0.schema.json");
            assert_eq!(response["schema"], RESPONSE_SCHEMA);
            assert_eq!(response["snapshot_id"], index.snapshot.id);
            assert_eq!(
                response["operation"],
                serde_json::to_value(operation).unwrap()
            );
            assert!(response["budget"]["used_items"].as_u64().unwrap() <= 64);
            assert!(response["budget"]["used_bytes"].as_u64().unwrap() <= 65_536);
            assert_eq!(
                response["budget"]["used_bytes"].as_u64().unwrap(),
                u64::try_from(canonical_json(&response["result"]).len()).unwrap()
            );
        }
        let stale = QueryRequest {
            snapshot_id: format!("sha256:{}", "0".repeat(64)),
            ..request(&index, Operation::Describe, symbol.clone(), 64, 65_536)
        };
        assert_eq!(execute(&index, &stale), Err(QueryError::SnapshotMismatch));
        let truncated = execute(
            &index,
            &request(
                &index,
                Operation::Outline,
                index.package.id.clone(),
                1,
                1024,
            ),
        )
        .unwrap();
        assert_eq!(truncated["budget"]["truncated"], true);
        assert_eq!(truncated["result"]["completeness"], "partial");
    }

    fn repository() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn valid_b_paths() -> Vec<String> {
        [
            "numbers-units/valid/explicit-int-to-float.sico",
            "nominal-invariants/valid/satisfied-invariant.sico",
            "exhaustive-match/valid/all-variants.sico",
            "result-mapping/valid/same-error-try.sico",
            "effects-capabilities/valid/explicit-boundary.sico",
            "component-call/valid/wit-safe-interface.sico",
            "affine-resources/valid/close-once.sico",
            "future-task/valid/await-once.sico",
            "stream/valid/pull-one.sico",
            "revision/valid/versioned-commit.sico",
        ]
        .into_iter()
        .map(|path| format!("syntax-candidates/b/{path}"))
        .collect()
    }

    fn load_inputs(root: &Path, paths: &[String]) -> Vec<IndexInput> {
        paths
            .iter()
            .enumerate()
            .map(|(index, file)| {
                let path = root.join(file);
                IndexInput {
                    module_name: path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .replace('_', "-"),
                    file: file.clone(),
                    source: SourceFile::from_bytes(
                        SourceId::new(u32::try_from(index).unwrap()),
                        file.clone(),
                        &fs::read(path).unwrap(),
                    )
                    .unwrap(),
                }
            })
            .collect()
    }

    fn request(
        index: &SemanticIndex,
        operation: Operation,
        target: String,
        max_items: usize,
        max_bytes: usize,
    ) -> QueryRequest {
        QueryRequest {
            schema: QUERY_SCHEMA.to_owned(),
            protocol_version: 0,
            request_id: format!("q-{operation:?}").to_ascii_lowercase(),
            snapshot_id: index.snapshot.id.clone(),
            operation,
            target: QueryTarget { id: target },
            parameters: Value::Object(Map::new()),
            options: QueryOptions {
                max_depth: 2,
                max_items,
                max_bytes,
                include_private: true,
                include_source: false,
                transitive: false,
            },
        }
    }

    fn assert_index_schema_contract(index: &SemanticIndex) {
        let value = serde_json::to_value(index).unwrap();
        assert_eq!(value["schema"], INDEX_SCHEMA);
        assert_eq!(value["protocol_version"], 0);
        for required in [
            "producer",
            "snapshot",
            "package",
            "modules",
            "symbols",
            "relations",
        ] {
            assert!(value.get(required).is_some(), "missing {required}");
        }
        assert_schema_required_contract(&value, "index-v0.schema.json");
        assert_eq!(index.producer.mode, "compiler");
    }

    fn assert_schema_required_contract(value: &Value, schema_file: &str) {
        let schema: Value = serde_json::from_str(
            &fs::read_to_string(repository().join("semantic-index/schema").join(schema_file))
                .unwrap(),
        )
        .unwrap();
        let required: BTreeSet<_> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect();
        assert!(required.iter().all(|name| value.get(*name).is_some()));
    }
}
