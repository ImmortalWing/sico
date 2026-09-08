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
use sico_source::{SourceFile, SourceId, TextRange};

/// `use`-graph depth bound (limit+1 tested).
pub(crate) const MAX_USE_DEPTH: usize = 32;
/// Imported-module count bound (limit+1 tested).
pub(crate) const MAX_MODULES: usize = 64;

pub(crate) struct ImportedModule {
    pub name: String,
    pub source: SourceFile,
}

pub(crate) struct Assembled {
    pub entry: SourceFile,
    /// Deterministic discovery order (entry `use` order, then DFS).
    pub imports: Vec<ImportedModule>,
}

impl Assembled {
    /// Cache-key contribution: length-prefixed (name, bytes) per import in
    /// discovery order; the entry bytes are already part of the frozen key.
    /// Empty for single-file programs.
    pub(crate) fn cache_blob(&self) -> Vec<u8> {
        let mut blob = Vec::new();
        for import in &self.imports {
            let name = import.source.name().as_bytes();
            blob.extend((name.len() as u64).to_le_bytes());
            blob.extend_from_slice(name);
            blob.extend(u64::from(u32::from(import.source.len())).to_le_bytes());
            blob.extend_from_slice(import.source.text().as_bytes());
        }
        blob
    }
}

pub(crate) enum AssembleError {
    /// The entry or an imported file failed to parse; the caller renders it.
    Frontend(SourceFile, Parse),
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
        return Err(AssembleError::Frontend(entry, parsed));
    }
    let ast = parsed.ast().expect("successful parse carries an AST");
    let mut uses = Vec::new();
    for declaration in ast.declarations() {
        match declaration.kind {
            DeclarationKind::Module => {
                return Err(AssembleError::Diagnostics(vec![render(
                    &entry,
                    "E8010",
                    declaration.range,
                    "the CLI entry file is the program root and does not declare a module",
                )]));
            }
            DeclarationKind::Use => {
                let (module, item) = declaration
                    .name
                    .split_once('.')
                    .expect("parser shapes use names as module.item");
                uses.push(UseItem {
                    module: module.to_owned(),
                    item: item.to_owned(),
                    range: declaration.range,
                });
            }
            _ => {}
        }
    }
    if uses.is_empty() {
        return Ok(Assembled {
            entry,
            imports: Vec::new(),
        });
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
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
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
        let sources: Vec<&SourceFile> = std::iter::once(&entry)
            .chain(linker.files.values().map(|file| &file.source))
            .collect();
        let rendered = linker
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
            .collect();
        return Err(AssembleError::Diagnostics(rendered));
    }
    let imports = linker
        .files
        .into_iter()
        .map(|(name, file)| ImportedModule {
            name,
            source: file.source,
        })
        .collect();
    Ok(Assembled { entry, imports })
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
            self.gray.insert(module.clone());
            let path = self.root.join(format!("{module}.sico"));
            let file_name = path.display().to_string();
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    self.fail(
                        "E8004",
                        importer,
                        use_item.range,
                        format!("module file {} not found", path.display()),
                    );
                    return Ok(());
                }
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
                return Err(AssembleError::Frontend(source, parsed));
            }
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
                        let (child_module, child_item) = declaration
                            .name
                            .split_once('.')
                            .expect("parser shapes use names as module.item");
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
                        other_declarations
                            .insert(declaration.name.clone(), declaration_kind(other));
                    }
                }
            }
            match module_declarations.as_slice() {
                [] => self.fail(
                    "E8001",
                    &file_name,
                    TextRange::up_to(source.len()),
                    format!("module file {file_name} does not declare module {module}"),
                ),
                [(name, range)] if name != &module => self.fail(
                    "E8002",
                    &file_name,
                    *range,
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
                    use_item.range,
                    format!("more than the {MAX_MODULES}-module v0 bound"),
                );
            }
            self.files.insert(
                module.clone(),
                ModuleFile {
                    source,
                    uses: child_uses.clone(),
                    functions,
                    other_declarations,
                },
            );
            for child in &child_uses {
                self.link(&file_name, child, depth + 1)?;
            }
            self.gray.remove(&module);
            self.done.insert(module.clone());
        }
        if let Some(file) = self.files.get(&module) {
            if !file.functions.contains(&use_item.item) {
                let message = match file.other_declarations.get(&use_item.item) {
                    Some(kind) => format!(
                        "module `{module}` exports functions only in v0; `{}` is a {kind} declaration",
                        use_item.item
                    ),
                    None => {
                        format!("module `{module}` has no item `{}`", use_item.item)
                    }
                };
                let code = if file.other_declarations.contains_key(&use_item.item) {
                    "E8011"
                } else {
                    "E8005"
                };
                self.fail(code, importer, use_item.range, message);
            }
        }
        Ok(())
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
