//! RFC-0039 module assembly for the CLI (STEP-0143).
//!
//! Discovery walks the entry file's `use` declarations depth-first in
//! source order; every imported file must sit in the entry's directory as
//! `<module>.sico` and declare a matching `module <name>`. Verification is
//! fail-closed with stable E8xxx identities; nothing is silently dropped.
//! The entry file is the program root: it declares no module, and a
//! program without `use` declarations never touches the filesystem, so
//! single-file programs assemble exactly as before.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use sico_parser::{DeclarationKind, Parse, parse};
use sico_semantics::{analyze_with_interfaces, exported_functions};
use sico_source::{SourceFile, SourceId, TextRange};

/// `use`-graph depth bound (limit+1 tested).
pub(crate) const MAX_USE_DEPTH: usize = 32;
/// Imported-module count bound (limit+1 tested).
pub(crate) const MAX_MODULES: usize = 64;

pub(crate) struct ImportedModule {
    pub name: String,
    pub source: SourceFile,
    /// Module names this import directly uses (its own `use` edges).
    pub uses: Vec<String>,
}

pub(crate) struct Assembled {
    pub entry: SourceFile,
    /// Module names the entry directly uses (its `use` edges).
    pub entry_uses: Vec<String>,
    /// Deterministic discovery order (entry `use` order, then DFS).
    pub imports: Vec<ImportedModule>,
    /// RFC-0039 §2.3/§2.4 (STEP-0147): resolved package dependencies in
    /// deterministic (declared) order; empty without package uses.
    pub packages: Vec<crate::packages::ResolvedPackage>,
}

impl Assembled {
    /// Cache-key contribution: length-prefixed (name, bytes) per import in
    /// discovery order plus each resolved package's artifact digest; the
    /// entry bytes are already part of the frozen key. Empty for plain
    /// single-file programs.
    pub(crate) fn cache_blob(&self) -> Vec<u8> {
        let mut blob = Vec::new();
        for import in &self.imports {
            let name = import.source.name().as_bytes();
            blob.extend((name.len() as u64).to_le_bytes());
            blob.extend_from_slice(name);
            blob.extend(u64::from(u32::from(import.source.len())).to_le_bytes());
            blob.extend_from_slice(import.source.text().as_bytes());
        }
        for package in &self.packages {
            let name = package.name.as_bytes();
            blob.extend((name.len() as u64).to_le_bytes());
            blob.extend_from_slice(name);
            let digest = package.artifact_sha256.as_bytes();
            blob.extend((digest.len() as u64).to_le_bytes());
            blob.extend_from_slice(digest);
        }
        blob
    }
}

pub(crate) enum AssembleError {
    /// The entry or an imported file failed to parse; the caller renders
    /// it. Boxed: the pair is large and the `Err` arm must stay small.
    Frontend(Box<FrontendFailure>),
    /// Fail-closed link diagnostics, already rendered as RFC-0001 lines.
    Diagnostics(Vec<String>),
    Tool(String),
}

#[derive(Clone)]
struct UseItem {
    module: String,
    item: String,
    range: TextRange,
}

/// A parse failure carried out of assembly for the caller to render.
pub(crate) struct FrontendFailure {
    pub source: SourceFile,
    pub parsed: Parse,
}

struct LinkDiagnostic {
    code: &'static str,
    file: String,
    range: TextRange,
    message: String,
}

struct ModuleFile {
    source: SourceFile,
    uses: Vec<UseItem>,
    functions: BTreeSet<String>,
    other_declarations: BTreeMap<String, &'static str>,
}

struct Linker {
    root: PathBuf,
    files: BTreeMap<String, ModuleFile>,
    gray: BTreeSet<String>,
    done: BTreeSet<String>,
    seen: BTreeSet<(String, String)>,
    next_source_id: u32,
    diagnostics: Vec<LinkDiagnostic>,
}

/// Assembles the compilation set for one entry source. A program without
/// `use` declarations returns an empty import list without touching the
/// filesystem.
#[allow(clippy::too_many_lines)] // discovery, package gate and module linking in one pass
pub(crate) fn assemble_from_bytes(name: &str, bytes: &[u8]) -> Result<Assembled, AssembleError> {
    let entry =
        SourceFile::from_bytes(SourceId::new(0), name.to_owned(), bytes).map_err(|error| {
            AssembleError::Tool(format!(
                "sico: {name}: source contract error {:?} at bytes {}..{}",
                error.kind,
                u32::from(error.range.start()),
                u32::from(error.range.end())
            ))
        })?;
    let parsed = parse(&entry);
    if !parsed.is_success() {
        return Err(AssembleError::Frontend(Box::new(FrontendFailure {
            source: entry,
            parsed,
        })));
    }
    let uses = scan_entry_uses(&entry, &parsed)?;
    let entry_uses: Vec<String> = uses.iter().map(|item| item.module.clone()).collect();
    let entry_pkg_uses = scan_package_uses(&entry, name);
    if uses.is_empty() && entry_pkg_uses.is_empty() {
        return Ok(Assembled {
            entry,
            entry_uses,
            imports: Vec::new(),
            packages: Vec::new(),
        });
    }
    if !entry_pkg_uses.is_empty() && uses.is_empty() {
        // RFC-0039 §2.3: entry-only package programs resolve immediately.
        // Mixed programs (packages plus module imports) fall through to
        // the full assembly so the collision gate sees every name; stdin
        // sources have no resolution root either way.
        if name == "<stdin>" {
            let line = render(
                &entry,
                "E8009",
                sico_source::TextRange::up_to(entry.len()),
                "package uses need a file entry; stdin sources have no resolution root",
            );
            return Err(AssembleError::Diagnostics(vec![line]));
        }
        let interfaces = interface_surface(&entry);
        let sources: Vec<&SourceFile> = vec![&entry];
        match crate::packages::resolve(
            &Path::new(name)
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf),
            &entry_pkg_uses,
            &BTreeMap::from([(name.to_owned(), interfaces)]),
            &[],
        ) {
            Ok(packages) => {
                return Ok(Assembled {
                    entry,
                    entry_uses,
                    imports: Vec::new(),
                    packages,
                });
            }
            Err(diagnostics) => {
                return Err(AssembleError::Diagnostics(render_package_diagnostics(
                    &sources,
                    &diagnostics,
                )));
            }
        }
    }
    if name == "<stdin>" {
        return Err(AssembleError::Diagnostics(vec![render(
            &entry,
            "E8009",
            uses[0].range,
            "module imports need a file entry; stdin sources have no resolution root",
        )]));
    }
    let root = Path::new(name)
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut linker = Linker {
        root,
        files: BTreeMap::new(),
        gray: BTreeSet::new(),
        done: BTreeSet::new(),
        seen: BTreeSet::new(),
        next_source_id: 1,
        diagnostics: Vec::new(),
    };
    for use_item in &uses {
        linker.link(name, use_item, 1)?;
    }
    if !linker.diagnostics.is_empty() {
        return Err(AssembleError::Diagnostics(render_link_diagnostics(
            &entry, &linker,
        )));
    }
    // RFC-0039 §2.3/§2.4 (STEP-0147): collect every file's package uses and
    // declared interfaces, then run the fail-closed resolution gate.
    let mut pkg_uses = entry_pkg_uses;
    let mut interfaces: BTreeMap<String, Vec<crate::packages::DeclaredInterface>> = BTreeMap::new();
    interfaces.insert(name.to_owned(), interface_surface(&entry));
    let mut module_names: Vec<String> = Vec::new();
    for (module_name, file) in &linker.files {
        module_names.push(module_name.clone());
        let file_pkg_uses = scan_package_uses(&file.source, file.source.name());
        pkg_uses.extend(file_pkg_uses);
        interfaces.insert(
            file.source.name().to_owned(),
            interface_surface(&file.source),
        );
    }
    let packages =
        match crate::packages::resolve(&linker.root, &pkg_uses, &interfaces, &module_names) {
            Ok(packages) => packages,
            Err(diagnostics) => {
                let sources: Vec<&SourceFile> = std::iter::once(&entry)
                    .chain(linker.files.values().map(|file| &file.source))
                    .collect();
                return Err(AssembleError::Diagnostics(render_package_diagnostics(
                    &sources,
                    &diagnostics,
                )));
            }
        };
    let imports = linker
        .files
        .into_iter()
        .map(|(name, file)| ImportedModule {
            name,
            uses: file
                .uses
                .iter()
                .map(|use_item| use_item.module.clone())
                .collect(),
            source: file.source,
        })
        .collect();
    Ok(Assembled {
        entry,
        entry_uses,
        imports,
        packages,
    })
}

/// RFC-0039 §2.3 (STEP-0147): collects one file's `use pkg` declarations.
fn scan_package_uses(source: &SourceFile, file_label: &str) -> Vec<crate::packages::PackageUse> {
    let parsed = parse(source);
    if !parsed.is_success() {
        return Vec::new();
    }
    parsed
        .ast()
        .expect("successful parse carries an AST")
        .declarations()
        .iter()
        .filter_map(|declaration| {
            let sico_parser::DeclarationDetail::Package {
                package,
                version,
                expose,
            } = &declaration.detail
            else {
                return None;
            };
            Some(crate::packages::PackageUse {
                package: package.clone(),
                version: *version,
                expose: expose.clone(),
                file: file_label.to_owned(),
                range: declaration.range,
            })
        })
        .collect()
}

/// RFC-0039 §2.4 (STEP-0147): one file's versioned interfaces, for the
/// package-expose gate.
fn interface_surface(source: &SourceFile) -> Vec<crate::packages::DeclaredInterface> {
    sico_semantics::declared_interfaces(source)
        .into_iter()
        .filter_map(|(name, interface)| {
            let version = interface.version?;
            Some(crate::packages::DeclaredInterface {
                name,
                version,
                functions: interface.functions,
            })
        })
        .collect()
}

/// Renders package-gate diagnostics against their owning files.
fn render_package_diagnostics(
    sources: &[&SourceFile],
    diagnostics: &[crate::packages::PackageDiagnostic],
) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let source = sources
                .iter()
                .find(|source| source.name() == diagnostic.file)
                .copied()
                .unwrap_or(sources[0]);
            render(
                source,
                diagnostic.code,
                diagnostic.range,
                &diagnostic.message,
            )
        })
        .collect()
}

/// Scans the entry's top-level declarations for the module/use contract:
/// a module declaration is refused (E8010) and every `use` line becomes a
/// link edge.
fn scan_entry_uses(entry: &SourceFile, parsed: &Parse) -> Result<Vec<UseItem>, AssembleError> {
    let ast = parsed.ast().expect("successful parse carries an AST");
    let mut uses = Vec::new();
    for declaration in ast.declarations() {
        if declaration.kind == DeclarationKind::Module {
            return Err(AssembleError::Diagnostics(vec![render(
                entry,
                "E8010",
                declaration.range,
                "the CLI entry file is the program root and does not declare a module",
            )]));
        }
        if declaration.kind == DeclarationKind::Use {
            // RFC-0039 §2.3 (STEP-0147): `use pkg` lines carry their own
            // detail and flow through the package gate, not this scan.
            if matches!(
                declaration.detail,
                sico_parser::DeclarationDetail::Package { .. }
            ) {
                continue;
            }
            let (module, item) = declaration
                .name
                .split_once('.')
                .expect("parser shapes module use names as module.item");
            uses.push(UseItem {
                module: module.to_owned(),
                item: item.to_owned(),
                range: declaration.range,
            });
        }
    }
    Ok(uses)
}

/// Renders accumulated link diagnostics against their owning files.
fn render_link_diagnostics(entry: &SourceFile, linker: &Linker) -> Vec<String> {
    let sources: Vec<&SourceFile> = std::iter::once(entry)
        .chain(linker.files.values().map(|file| &file.source))
        .collect();
    linker
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let source = sources
                .iter()
                .find(|source| source.name() == diagnostic.file)
                .expect("diagnostics reference assembled files");
            render(
                source,
                diagnostic.code,
                diagnostic.range,
                &diagnostic.message,
            )
        })
        .collect()
}

/// RFC-0039 §2.2 (STEP-0144): analyzes the assembled set with each file's
/// direct-import surface injected, so qualified cross-module calls resolve
/// and type-check at check time. Callers handle the failure cases in their
/// own output order (entry first, then imports in discovery order).
pub(crate) fn analyze_set(assembled: &Assembled) -> AnalyzedSet {
    let surface: BTreeMap<String, BTreeMap<String, sico_semantics::ImportedFunction>> = assembled
        .imports
        .iter()
        .map(|import| (import.name.clone(), exported_functions(&import.source)))
        .collect();
    // RFC-0039 §2.3/§2.4 (STEP-0147): the resolved package surface reaches
    // check time for every file, so qualified `<package>.<function>` calls
    // resolve and type-check exactly like module calls.
    let package_surface: BTreeMap<String, sico_semantics::PackageInterface> = assembled
        .packages
        .iter()
        .map(|package| {
            (
                package.name.clone(),
                sico_semantics::PackageInterface {
                    package: package.name.clone(),
                    version: package.version,
                    interface: package.interface_identity.clone(),
                    functions: package.functions.clone(),
                },
            )
        })
        .collect();
    let map_for = |uses: &[String]| {
        uses.iter()
            .filter_map(|name| {
                surface
                    .get(name)
                    .map(|functions| (name.clone(), functions.clone()))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let analyze = |source: &sico_source::SourceFile, uses: &[String]| {
        analyze_with_interfaces(source, &map_for(uses), &package_surface)
            .expect("successful parse must lower for semantic analysis")
    };
    let entry = analyze(&assembled.entry, &assembled.entry_uses);
    let imports = assembled
        .imports
        .iter()
        .map(|import| {
            let analysis = analyze(&import.source, &import.uses);
            (import.name.clone(), analysis)
        })
        .collect();
    AnalyzedSet { entry, imports }
}

/// One module file's classified top-level surface (RFC-0039 §2.2/§2.0).
type ModuleSurface = (
    Vec<(String, TextRange)>,
    BTreeSet<String>,
    BTreeMap<String, &'static str>,
    Vec<UseItem>,
);

pub(crate) struct AnalyzedSet {
    pub entry: sico_semantics::Analysis,
    pub imports: Vec<(String, sico_semantics::Analysis)>,
}

impl Linker {
    fn link(
        &mut self,
        importer: &str,
        use_item: &UseItem,
        depth: usize,
    ) -> Result<(), AssembleError> {
        let module = use_item.module.clone();
        if depth > MAX_USE_DEPTH {
            self.fail(
                "E8007",
                importer,
                use_item.range,
                format!("use chain deeper than the {MAX_USE_DEPTH}-module v0 bound"),
            );
            return Ok(());
        }
        if self.gray.contains(&module) {
            self.fail(
                "E8003",
                importer,
                use_item.range,
                format!("module `{module}` is already on the current import chain"),
            );
            return Ok(());
        }
        if !self.seen.insert((module.clone(), use_item.item.clone())) {
            self.fail(
                "E8006",
                importer,
                use_item.range,
                format!("duplicate use of {}.{}", use_item.module, use_item.item),
            );
            return Ok(());
        }
        if !self.done.contains(&module) {
            let Some(child_uses) = self.load_module(importer, &module, use_item.range)? else {
                return Ok(());
            };
            let file_name = self
                .root
                .join(format!("{module}.sico"))
                .display()
                .to_string();
            for child in &child_uses {
                self.link(&file_name, child, depth + 1)?;
            }
            self.gray.remove(&module);
            self.done.insert(module.clone());
        }
        // Item check: compute the refusal inside the borrow, emit after.
        let missing = self
            .files
            .get(&module)
            .filter(|file| !file.functions.contains(&use_item.item))
            .map(|file| {
                let code = if file.other_declarations.contains_key(&use_item.item) {
                    "E8011"
                } else {
                    "E8005"
                };
                let message = match file.other_declarations.get(&use_item.item) {
                    Some(kind) => format!(
                        "module `{module}` exports functions only in v0; `{}` is a {kind} declaration",
                        use_item.item
                    ),
                    None => format!("module `{module}` has no item `{}`", use_item.item),
                };
                (code, message)
            });
        if let Some((code, message)) = missing {
            self.fail(code, importer, use_item.range, message);
        }
        Ok(())
    }

    /// Classifies one module file's top-level declarations: module
    /// declarations, exported functions, other named declarations, and its
    /// `use` edges. A file without any module declaration is recorded as
    /// E8001 and yields `None`.
    fn classify_module_file(parsed: &Parse) -> ModuleSurface {
        let ast = parsed.ast().expect("successful parse carries an AST");
        let mut module_declarations = Vec::new();
        let mut functions = BTreeSet::new();
        let mut other_declarations = BTreeMap::new();
        let mut child_uses = Vec::new();
        for declaration in ast.declarations() {
            match declaration.kind {
                DeclarationKind::Module => {
                    module_declarations.push((declaration.name.clone(), declaration.range));
                }
                DeclarationKind::Use => {
                    // RFC-0039 §2.3 (STEP-0147): `use pkg` declarations ride
                    // the package gate (scan_package_uses), not the module
                    // use graph.
                    if matches!(
                        declaration.detail,
                        sico_parser::DeclarationDetail::Package { .. }
                    ) {
                        continue;
                    }
                    let (child_module, child_item) = declaration
                        .name
                        .split_once('.')
                        .expect("parser shapes module use names as module.item");
                    child_uses.push(UseItem {
                        module: child_module.to_owned(),
                        item: child_item.to_owned(),
                        range: declaration.range,
                    });
                }
                DeclarationKind::Function => {
                    functions.insert(declaration.name.clone());
                }
                other => {
                    other_declarations.insert(declaration.name.clone(), declaration_kind(other));
                }
            }
        }
        (
            module_declarations,
            functions,
            other_declarations,
            child_uses,
        )
    }
    /// Reads, parses and registers one module file. Returns `Ok(None)` when
    /// a typed diagnostic was recorded (the caller stops that edge);
    /// `Err` carries a parse failure of the module file itself. On success
    /// it returns the file's child `use` edges for the caller's recursion.
    fn load_module(
        &mut self,
        importer: &str,
        module: &str,
        range: TextRange,
    ) -> Result<Option<Vec<UseItem>>, AssembleError> {
        self.gray.insert(module.to_owned());
        let module_path = self.root.join(format!("{module}.sico"));
        let file_name = module_path.display().to_string();
        let Ok(bytes) = fs::read(&module_path) else {
            self.fail(
                "E8004",
                importer,
                range,
                format!("module file {} not found", module_path.display()),
            );
            return Ok(None);
        };
        let source = SourceFile::from_bytes(
            SourceId::new(self.next_source_id),
            file_name.clone(),
            &bytes,
        )
        .map_err(|error| {
            AssembleError::Tool(format!(
                "sico: {file_name}: source contract error {:?} at bytes {}..{}",
                error.kind,
                u32::from(error.range.start()),
                u32::from(error.range.end())
            ))
        })?;
        self.next_source_id += 1;
        let parsed = parse(&source);
        if !parsed.is_success() {
            return Err(AssembleError::Frontend(Box::new(FrontendFailure {
                source,
                parsed,
            })));
        }
        let (module_declarations, functions, other_declarations, child_uses) =
            Self::classify_module_file(&parsed);
        // Register before the contract checks: a failing file stays
        // renderable for its own diagnostics.
        self.files.insert(
            module.to_owned(),
            ModuleFile {
                source,
                uses: child_uses.clone(),
                functions,
                other_declarations,
            },
        );
        match module_declarations.as_slice() {
            [] => self.fail(
                "E8001",
                &file_name,
                TextRange::up_to(u32::try_from(bytes.len()).unwrap_or(u32::MAX).into()),
                format!("module file {file_name} does not declare module {module}"),
            ),
            [(name, declared_range)] if name != module => self.fail(
                "E8002",
                &file_name,
                *declared_range,
                format!("module declaration names {name} but the file is {file_name}"),
            ),
            [(_name, _range)] => {}
            [(_, first), (_, second), ..] => self.fail(
                "E8001",
                &file_name,
                *second,
                format!(
                    "multiple module declarations in {file_name} (first at byte {})",
                    u32::from(first.start())
                ),
            ),
        }
        if self.files.len() >= MAX_MODULES {
            self.fail(
                "E8008",
                importer,
                range,
                format!("more than the {MAX_MODULES}-module v0 bound"),
            );
        }
        Ok(Some(child_uses))
    }

    fn fail(&mut self, code: &'static str, file: &str, range: TextRange, message: String) {
        self.diagnostics.push(LinkDiagnostic {
            code,
            file: file.to_owned(),
            range,
            message,
        });
    }
}

fn render(source: &SourceFile, code: &str, range: TextRange, message: &str) -> String {
    let position = source
        .line_index()
        .line_col(source.text(), range.start())
        .expect("link diagnostic ranges sit on scalar boundaries");
    format!(
        "{code} {}:{}:{} {message}",
        source.name(),
        position.line,
        position.column
    )
}

const fn declaration_kind(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Newtype => "newtype",
        DeclarationKind::Record => "record",
        DeclarationKind::Enum => "enum",
        DeclarationKind::Capability => "capability",
        DeclarationKind::Resource => "resource",
        DeclarationKind::Interface => "interface",
        DeclarationKind::Function => "function",
        DeclarationKind::Module => "module",
        DeclarationKind::Use => "use",
    }
}

/// Cheap pre-check for the `--debug-info` path: whether the entry declares
/// `use` imports (parse failures report `false`; the caller re-parses and
/// reports them properly).
pub(crate) fn has_module_uses(name: &str, bytes: &[u8]) -> bool {
    let Ok(source) = SourceFile::from_bytes(SourceId::new(0), name.to_owned(), bytes) else {
        return false;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return false;
    }
    parsed
        .ast()
        .expect("successful parse carries an AST")
        .declarations()
        .iter()
        .any(|declaration| declaration.kind == DeclarationKind::Use)
}
