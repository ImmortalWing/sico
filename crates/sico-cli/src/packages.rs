//! RFC-0039 §2.3/§2.4 package resolution for the CLI link pass (STEP-0147).
//!
//! A source dependency is declared as `use pkg <name> version <n>
//! [expose <interface>]` and resolved through `sico-lock.json` beside the
//! entry file: a deterministic projection of the M7 lock boundary carrying
//! the signed package artifact path and its SHA-256. Acquisition is an
//! explicit owner step — there is no download, no registry access and no
//! fallback here. Every failure is a typed E80xx diagnostic.
//!
//! Shape validation (the E8016 gate) is digest-based for v0: the package
//! producer stamps the canonical interface identity and specification
//! digest into a `sico:user-interface` custom section of the component it
//! builds (see `sico-codegen-wasm::package_builder`); the CLI renders the
//! same canonical form from the source-declared interface and compares.
//! The lock pins the exact artifact bytes, the producer is the only
//! stamped-artifact source, and the runner re-checks arities at link time,
//! so the trust chain stays explicit and declared rather than silently
//! approximate.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::Deserialize;
use sico_semantics::{ImportedFunction, Type};
use sico_source::TextRange;

/// Package-count bound per program (limit+1 tested).
pub(crate) const MAX_PACKAGES: usize = 16;
/// Source-level lock projection beside the entry file (RFC-0039 §2.3).
pub(crate) const LOCK_FILE: &str = "sico-lock.json";
const LOCK_SCHEMA: &str = "sico:source-lock:v0";
/// Custom section the package builder stamps with the interface identity
/// and specification digest (E8016 gate, STEP-0147).
pub(crate) const INTERFACE_STAMP_SECTION: &str = "sico:user-interface";

/// One resolved package dependency, ready for analysis, lowering and the
/// runner hand-off.
pub(crate) struct ResolvedPackage {
    pub name: String,
    /// The package version the lock pinned (not the interface version).
    pub version: u64,
    /// The exposed interface's boundary identity
    /// `sico:user/<interface>@<interface-version>`.
    pub interface_identity: String,
    /// The interface name alone, in boundary kebab-case (lowering input).
    pub interface_kebab: String,
    /// The exposed interface's functions in the semantic surface.
    pub functions: BTreeMap<String, ImportedFunction>,
    /// The verified inner component bytes handed to the runner.
    pub component: Vec<u8>,
    /// SHA-256 of the signed package artifact; contributes to the cache
    /// identity exactly like module bytes do.
    pub artifact_sha256: String,
}

/// One `use pkg` declaration found in an assembled file.
pub(crate) struct PackageUse {
    pub package: String,
    pub version: u64,
    pub expose: Option<String>,
    /// The file (entry name or module name) that declared the use.
    pub file: String,
    pub range: TextRange,
}

/// One declared (versioned) interface: `interface <name> version <n>:`.
pub(crate) struct DeclaredInterface {
    pub name: String,
    pub version: u64,
    pub functions: BTreeMap<String, ImportedFunction>,
}

/// A diagnostic-shaped failure: code, owning file, span and message, in
/// the modules link-pass style (rendered by the caller).
pub(crate) struct PackageDiagnostic {
    pub code: &'static str,
    pub file: String,
    pub range: TextRange,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceLock {
    schema: String,
    packages: Vec<LockedPackage>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockedPackage {
    name: String,
    version: u64,
    path: String,
    sha256: String,
}

/// Renders one v0 interface type in the canonical WIT spelling. The same
/// rendering drives the producer stamp and the CLI comparison, so any
/// drift between the two sides changes the digest and fails closed.
pub(crate) fn render_interface_type(ty: &Type) -> Option<String> {
    match ty {
        Type::Named(name) => match name.as_str() {
            "Bool" => Some("bool".into()),
            "I64" => Some("s64".into()),
            "U64" => Some("u64".into()),
            "Text" => Some("string".into()),
            "Bytes" => Some("list<u8>".into()),
            "Unit" => Some("unit".into()),
            _ => None,
        },
        Type::Generic { name, arguments } => match (name.as_str(), arguments.as_slice()) {
            ("List", [inner]) => Some(format!("list<{}>", render_interface_type(inner)?)),
            ("Result", [ok, error]) => {
                let error = render_interface_type(error)?;
                if matches!(ok, Type::Named(unit) if unit == "Unit") {
                    Some(format!("result<_, {error}>"))
                } else {
                    Some(format!("result<{}, {error}>", render_interface_type(ok)?))
                }
            }
            (..) => None,
        },
        Type::Unknown => None,
    }
}

/// The canonical interface specification text a package stamp carries:
/// the identity line followed by one line per function in sorted order.
pub(crate) fn interface_spec_text(
    identity: &str,
    functions: &BTreeMap<String, ImportedFunction>,
) -> Option<String> {
    let mut text = format!("{identity}\n");
    for (name, imported) in functions {
        let mut params = Vec::new();
        for parameter in &imported.parameters {
            params.push(render_interface_type(parameter)?);
        }
        let result = render_interface_type(&imported.returns)?;
        let boundary = name.replace('_', "-");
        text.push_str(&boundary);
        text.push_str(" (");
        text.push_str(&params.join(", "));
        text.push_str(") -> ");
        text.push_str(&result);
        text.push('\n');
    }
    Some(text)
}

/// Extracts the interface stamp `(identity, digest)` a package builder
/// embedded in the component's `sico:user-interface` custom section.
pub(crate) fn read_interface_stamp(component: &[u8]) -> Option<(String, String)> {
    use wasmparser::{Parser, Payload};
    let parser = Parser::new(0);
    for payload in parser.parse_all(component) {
        let Payload::CustomSection(section) = payload.ok()? else {
            continue;
        };
        if section.name() != INTERFACE_STAMP_SECTION {
            continue;
        }
        let text = std::str::from_utf8(section.data()).ok()?;
        let mut lines = text.lines();
        let identity = lines.next()?.to_owned();
        let digest = lines.next()?.to_owned();
        return Some((identity, digest));
    }
    None
}

/// Resolves every declared package use into a [`ResolvedPackage`], applying
/// the full RFC-0039 §2.3/§2.4 fail-closed gate. `interfaces` are the
/// versioned interfaces declared per file (keyed by file name).
#[allow(clippy::too_many_lines)] // the fail-closed gate is one coherent decision table
pub(crate) fn resolve(
    root: &Path,
    uses: &[PackageUse],
    interfaces: &BTreeMap<String, Vec<DeclaredInterface>>,
    module_names: &[String],
) -> Result<Vec<ResolvedPackage>, Vec<PackageDiagnostic>> {
    let mut diagnostics: Vec<PackageDiagnostic> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut by_name: BTreeMap<String, &PackageUse> = BTreeMap::new();
    // One use line per (file, package): the same package may serve several
    // module files, each binding its own exposed interface.
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for use_item in uses {
        if !seen.insert((use_item.file.clone(), use_item.package.clone())) {
            record(
                &mut diagnostics,
                "E8006",
                &use_item.file,
                use_item.range,
                format!("duplicate use of package {}", use_item.package),
            );
            continue;
        }
        if module_names.iter().any(|name| name == &use_item.package) {
            record(
                &mut diagnostics,
                "E8018",
                &use_item.file,
                use_item.range,
                format!(
                    "package name `{}` collides with a module name; one name is one thing",
                    use_item.package
                ),
            );
            continue;
        }
        order.push(use_item.package.clone());
        by_name.insert(use_item.package.clone(), use_item);
    }
    if by_name.len() > MAX_PACKAGES {
        let first = &uses[0];
        record(
            &mut diagnostics,
            "E8015",
            &first.file,
            first.range,
            format!("more than the {MAX_PACKAGES}-package v0 bound"),
        );
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    if order.is_empty() {
        return Ok(Vec::new());
    }

    let lock_path = root.join(LOCK_FILE);
    let Ok(lock_bytes) = fs::read(&lock_path) else {
        let first = &uses[0];
        record(
            &mut diagnostics,
            "E8013",
            &first.file,
            first.range,
            format!(
                "package lock {} not found; acquisition is an explicit owner step",
                lock_path.display()
            ),
        );
        return Err(diagnostics);
    };
    let lock: SourceLock = match serde_json::from_slice::<SourceLock>(lock_bytes.as_slice()) {
        Ok(lock) if lock.schema == LOCK_SCHEMA => lock,
        Ok(lock) => {
            let first = &uses[0];
            record(
                &mut diagnostics,
                "E8013",
                &first.file,
                first.range,
                format!(
                    "package lock declares schema `{}` but `{LOCK_SCHEMA}` is required",
                    lock.schema
                ),
            );
            return Err(diagnostics);
        }
        Err(error) => {
            let first = &uses[0];
            record(
                &mut diagnostics,
                "E8013",
                &first.file,
                first.range,
                format!(
                    "package lock {} is not a valid {LOCK_SCHEMA} lock: {error}",
                    lock_path.display()
                ),
            );
            return Err(diagnostics);
        }
    };

    let mut resolved = Vec::new();
    for name in &order {
        let use_item = by_name.get(name).expect("order entries come from by_name");
        let Some(entry) = lock.packages.iter().find(|entry| &entry.name == name) else {
            record(
                &mut diagnostics,
                "E8012",
                &use_item.file,
                use_item.range,
                format!(
                    "package `{name}` is not declared in {}",
                    lock_path.display()
                ),
            );
            continue;
        };
        if entry.version != use_item.version {
            record(
                &mut diagnostics,
                "E8014",
                &use_item.file,
                use_item.range,
                format!(
                    "package `{name}` version {}: the lock pins {}",
                    use_item.version, entry.version
                ),
            );
            continue;
        }
        // The exposed interface must be declared (versioned) in the same
        // file as the use line (RFC-0039 §2.4).
        let Some(expose) = &use_item.expose else {
            record(
                &mut diagnostics,
                "E8020",
                &use_item.file,
                use_item.range,
                format!(
                    "package `{name}` declares no exposed interface; v0 consumers need `expose <interface>`"
                ),
            );
            continue;
        };
        let Some(declared) = interfaces
            .get(&use_item.file)
            .and_then(|list| list.iter().find(|interface| &interface.name == expose))
        else {
            record(
                &mut diagnostics,
                "E8020",
                &use_item.file,
                use_item.range,
                format!(
                    "expose names interface `{expose}` but {} declares no such versioned interface",
                    use_item.file
                ),
            );
            continue;
        };
        // Component names demand full semver; source versions are major
        // numbers rendered as `<major>.0.0` (builder-aligned).
        let interface_kebab = sico_ir::boundary_kebab(&declared.name);
        let interface_identity = format!("sico:user/{interface_kebab}@{}.0.0", declared.version);
        // E8016 gate: the stamped interface identity and specification
        // digest must match the source-declared interface exactly.
        let Some(spec) = interface_spec_text(&interface_identity, &declared.functions) else {
            record(
                &mut diagnostics,
                "E8017",
                &use_item.file,
                use_item.range,
                format!(
                    "interface `{}` carries a type outside the v0 user-WIT value set",
                    declared.name
                ),
            );
            continue;
        };
        let artifact_path = root.join(&entry.path);
        let Ok(artifact) = fs::read(&artifact_path) else {
            record(
                &mut diagnostics,
                "E8013",
                &use_item.file,
                use_item.range,
                format!(
                    "package artifact {} not found; acquisition is an explicit owner step",
                    artifact_path.display()
                ),
            );
            continue;
        };
        let digest = sico_package::sha256_hex(&artifact);
        if digest != entry.sha256 {
            record(
                &mut diagnostics,
                "E8013",
                &use_item.file,
                use_item.range,
                format!(
                    "package artifact {} digest mismatch: the lock declares {} but the content is {digest}",
                    artifact_path.display(),
                    entry.sha256
                ),
            );
            continue;
        }
        let package = match sico_package::verify(&artifact) {
            Ok(package) => package,
            Err(error) => {
                record(
                    &mut diagnostics,
                    "E8013",
                    &use_item.file,
                    use_item.range,
                    format!(
                        "package artifact {} failed verification: {error:?}",
                        artifact_path.display()
                    ),
                );
                continue;
            }
        };
        if !package.component_imports.is_empty() {
            record(
                &mut diagnostics,
                "E8021",
                &use_item.file,
                use_item.range,
                format!(
                    "package `{name}` imports from its environment; v0 packages are pure and receive no authority"
                ),
            );
            continue;
        }
        let expected_digest = sico_package::sha256_hex(spec.as_bytes());
        let Some((stamped_identity, stamped_digest)) = read_interface_stamp(&package.component)
        else {
            record(
                &mut diagnostics,
                "E8016",
                &use_item.file,
                use_item.range,
                format!(
                    "package `{name}` carries no {INTERFACE_STAMP_SECTION} stamp; v0 packages must be producer-stamped"
                ),
            );
            continue;
        };
        if stamped_identity != interface_identity || stamped_digest != expected_digest {
            record(
                &mut diagnostics,
                "E8016",
                &use_item.file,
                use_item.range,
                format!("package `{name}` exports do not match {interface_identity} as declared"),
            );
            continue;
        }
        resolved.push(ResolvedPackage {
            name: name.clone(),
            version: entry.version,
            interface_identity,
            interface_kebab,
            functions: declared.functions.clone(),
            component: package.component,
            artifact_sha256: digest,
        });
    }
    if diagnostics.is_empty() {
        Ok(resolved)
    } else {
        Err(diagnostics)
    }
}

fn record(
    diagnostics: &mut Vec<PackageDiagnostic>,
    code: &'static str,
    file: &str,
    range: TextRange,
    message: String,
) {
    if diagnostics.len() < MAX_PACKAGES * 4 {
        diagnostics.push(PackageDiagnostic {
            code,
            file: file.to_owned(),
            range,
            message,
        });
    }
}
