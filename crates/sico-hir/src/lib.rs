//! Untyped, deterministic HIR for successfully parsed RFC-0005 B source.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sico_lexer::{Token, TokenKind, lex};
use sico_parser::{DeclarationKind, parse};
use sico_source::{SourceFile, TextRange};

/// Stable preorder identity within one lowered module.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HirId(u32);

impl HirId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// HIR lowering never accepts a recovered or lexical-error tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LowerError {
    Lexical { count: usize },
    Syntax { count: usize },
}

/// Complete untyped module representation for the accepted B surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    pub id: HirId,
    pub declarations: Vec<Declaration>,
    source_map: BTreeMap<HirId, TextRange>,
}

impl Module {
    #[must_use]
    pub fn range(&self, id: HirId) -> Option<TextRange> {
        self.source_map.get(&id).copied()
    }

    #[must_use]
    pub fn source_map(&self) -> &BTreeMap<HirId, TextRange> {
        &self.source_map
    }

    /// Stable structural snapshot containing every semantic token kind.
    #[must_use]
    pub fn shape(&self) -> String {
        self.declarations
            .iter()
            .map(Declaration::shape)
            .collect::<Vec<_>>()
            .join(";")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    pub id: HirId,
    pub kind: DeclarationKind,
    pub name: String,
    pub range: TextRange,
    pub lines: Vec<Line>,
    pub token_tree: Vec<TokenTree>,
}

impl Declaration {
    fn shape(&self) -> String {
        let lines = self
            .lines
            .iter()
            .map(Line::shape)
            .collect::<Vec<_>>()
            .join(",");
        format!("{:?}:{}@{}[{lines}]", self.kind, self.name, self.id.get())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Line {
    pub id: HirId,
    pub kind: LineKind,
    pub depth: u16,
    pub range: TextRange,
    pub tokens: Vec<HirToken>,
}

impl Line {
    fn shape(&self) -> String {
        let tokens = self
            .tokens
            .iter()
            .map(|token| format!("{:?}", token.kind))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{}:{}:{:?}({tokens})", self.id.get(), self.depth, self.kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineKind {
    DeclarationHeader,
    FunctionSignature,
    Field,
    Invariant,
    Variant,
    Let,
    Return,
    Effects,
    Capabilities,
    Match,
    MatchArm,
    If,
    Using,
    TaskGroup,
    End,
    Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenTree {
    Token(HirToken),
    Group {
        open: HirToken,
        children: Vec<TokenTree>,
        close: HirToken,
        range: TextRange,
    },
}

impl TokenTree {
    pub fn flatten<'a>(&'a self, output: &mut Vec<&'a HirToken>) {
        match self {
            Self::Token(token) => output.push(token),
            Self::Group {
                open,
                children,
                close,
                ..
            } => {
                output.push(open);
                for child in children {
                    child.flatten(output);
                }
                output.push(close);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirToken {
    pub kind: TokenKind,
    pub text: String,
    pub range: TextRange,
}

#[derive(Clone, Debug)]
struct RawLine {
    range: TextRange,
    tokens: Vec<HirToken>,
}

/// Lowers every non-trivia token of a successful parse into deterministic HIR.
///
/// # Errors
///
/// Returns a typed error if source has any lexical or parser error. Recovered
/// trees are intentionally unable to produce HIR.
///
/// # Panics
///
/// Panics only if a successful parser result violates its internal AST or
/// balanced-delimiter invariants.
pub fn lower(source: &SourceFile) -> Result<Module, LowerError> {
    let parsed = parse(source);
    if !parsed.lex_errors().is_empty() {
        return Err(LowerError::Lexical {
            count: parsed.lex_errors().len(),
        });
    }
    if !parsed.errors().is_empty() {
        return Err(LowerError::Syntax {
            count: parsed.errors().len(),
        });
    }

    let raw_lines = semantic_lines(source);
    let mut next_id = 1_u32;
    let mut source_map = BTreeMap::from([(HirId(0), TextRange::up_to(source.len()))]);
    let mut declarations = Vec::new();
    for declaration in parsed.ast().unwrap().declarations() {
        let id = allocate(&mut next_id);
        source_map.insert(id, declaration.range);
        let selected: Vec<_> = raw_lines
            .iter()
            .filter(|line| {
                declaration.range.contains_range(line.range)
                    || (declaration.range.start() <= line.range.start()
                        && line.range.end() <= declaration.range.end())
            })
            .collect();
        let mut depth = 0_u16;
        let mut lines = Vec::new();
        let declaration_tokens: Vec<_> = selected
            .iter()
            .flat_map(|line| line.tokens.iter().cloned())
            .collect();
        for (index, raw) in selected.into_iter().enumerate() {
            let first = raw.tokens.first().map(|token| token.kind);
            if first == Some(TokenKind::End) {
                depth = depth.saturating_sub(1);
            }
            let line_id = allocate(&mut next_id);
            source_map.insert(line_id, raw.range);
            let kind = classify_line(index == 0, &raw.tokens);
            lines.push(Line {
                id: line_id,
                kind,
                depth,
                range: raw.range,
                tokens: raw.tokens.clone(),
            });
            if index > 0 && opens_block(&raw.tokens) {
                depth = depth.saturating_add(1);
            }
        }
        declarations.push(Declaration {
            id,
            kind: declaration.kind,
            name: declaration.name.clone(),
            range: declaration.range,
            lines,
            token_tree: token_trees(&declaration_tokens),
        });
    }
    Ok(Module {
        id: HirId(0),
        declarations,
        source_map,
    })
}

fn allocate(next: &mut u32) -> HirId {
    let id = HirId(*next);
    *next = next.checked_add(1).expect("source limits bound HIR ids");
    id
}

fn semantic_lines(source: &SourceFile) -> Vec<RawLine> {
    let lexed = lex(source);
    let mut result = Vec::new();
    let mut current = Vec::<HirToken>::new();
    for token in lexed.tokens().iter().copied() {
        if token.kind == TokenKind::Newline || token.kind == TokenKind::Eof {
            if !current.is_empty() {
                let start = current.first().expect("nonempty").range.start();
                let end = current.last().expect("nonempty").range.end();
                result.push(RawLine {
                    range: TextRange::new(start, end),
                    tokens: std::mem::take(&mut current),
                });
            }
            continue;
        }
        if matches!(token.kind, TokenKind::Whitespace | TokenKind::LineComment) {
            continue;
        }
        current.push(hir_token(source.text(), token));
    }
    result
}

fn hir_token(source: &str, token: Token) -> HirToken {
    HirToken {
        kind: token.kind,
        text: source[usize::from(token.range.start())..usize::from(token.range.end())].to_owned(),
        range: token.range,
    }
}

fn classify_line(header: bool, tokens: &[HirToken]) -> LineKind {
    if header {
        return LineKind::DeclarationHeader;
    }
    let first = tokens.first().map(|token| token.kind);
    match first {
        Some(TokenKind::Function | TokenKind::Async | TokenKind::Export) => {
            LineKind::FunctionSignature
        }
        Some(TokenKind::Invariant) => LineKind::Invariant,
        Some(TokenKind::Case)
            if tokens
                .last()
                .is_some_and(|token| token.kind == TokenKind::Colon) =>
        {
            LineKind::MatchArm
        }
        Some(TokenKind::Case) => LineKind::Variant,
        Some(TokenKind::Let) => LineKind::Let,
        Some(TokenKind::Return) => LineKind::Return,
        Some(TokenKind::Effects) => LineKind::Effects,
        Some(TokenKind::Capabilities) => LineKind::Capabilities,
        Some(TokenKind::Match) => LineKind::Match,
        Some(TokenKind::If) => LineKind::If,
        Some(TokenKind::Using) => LineKind::Using,
        Some(TokenKind::Task) => LineKind::TaskGroup,
        Some(TokenKind::End) => LineKind::End,
        Some(TokenKind::Identifier)
            if tokens.first().is_some_and(|token| token.text == "field") =>
        {
            LineKind::Field
        }
        _ => LineKind::Expression,
    }
}

fn opens_block(tokens: &[HirToken]) -> bool {
    tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Colon)
        && tokens.iter().any(|token| {
            matches!(
                token.kind,
                TokenKind::Function
                    | TokenKind::Match
                    | TokenKind::If
                    | TokenKind::Using
                    | TokenKind::Task
            )
        })
}

fn token_trees(tokens: &[HirToken]) -> Vec<TokenTree> {
    let mut roots = Vec::new();
    let mut stack = Vec::<(HirToken, Vec<TokenTree>)>::new();
    for token in tokens.iter().cloned() {
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => stack.push((token, Vec::new())),
            TokenKind::RightParen | TokenKind::RightBracket => {
                let (open, children) = stack.pop().expect("successful parse balances delimiters");
                let range = TextRange::new(open.range.start(), token.range.end());
                let group = TokenTree::Group {
                    open,
                    children,
                    close: token,
                    range,
                };
                if let Some((_, parent)) = stack.last_mut() {
                    parent.push(group);
                } else {
                    roots.push(group);
                }
            }
            _ => {
                let tree = TokenTree::Token(token);
                if let Some((_, children)) = stack.last_mut() {
                    children.push(tree);
                } else {
                    roots.push(tree);
                }
            }
        }
    }
    assert!(stack.is_empty(), "successful parse balances delimiters");
    roots
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use sico_source::SourceId;

    use super::*;

    #[test]
    fn all_b_sources_lower_deterministically_with_complete_source_maps() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-candidates/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 54);
        let mut snapshots = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            let module = lower(&source).unwrap();
            assert_eq!(module, lower(&source).unwrap());
            let ids: Vec<_> = module.source_map().keys().map(|id| id.get()).collect();
            assert_eq!(
                ids,
                (0..u32::try_from(ids.len()).unwrap()).collect::<Vec<_>>()
            );
            for (id, range) in module.source_map() {
                assert!(
                    source.span(*range).is_some(),
                    "{} id={}",
                    path.display(),
                    id.get()
                );
            }
            for declaration in &module.declarations {
                assert!(!declaration.lines.is_empty());
                for line in &declaration.lines {
                    for token in &line.tokens {
                        let start = usize::from(token.range.start());
                        let end = usize::from(token.range.end());
                        assert_eq!(token.text, source.text()[start..end]);
                    }
                }
                let mut tree_tokens = Vec::new();
                for tree in &declaration.token_tree {
                    tree.flatten(&mut tree_tokens);
                }
                let line_tokens: Vec<_> = declaration
                    .lines
                    .iter()
                    .flat_map(|line| &line.tokens)
                    .collect();
                assert_eq!(tree_tokens, line_tokens);
            }
            let relative = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            snapshots.push(format!("{relative} = {}", module.shape()));
        }
        let actual = format!("{}\n", snapshots.join("\n"));
        if std::env::var_os("SICO_DUMP_HIR").is_some() {
            print!("{actual}");
        } else {
            assert_eq!(actual, include_str!("../../../tests/hir/b-shapes.txt"));
        }
    }

    #[test]
    fn recovered_trees_cannot_lower() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-mutations/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 12);
        for (index, path) in paths.iter().enumerate() {
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            assert_eq!(lower(&source), Err(LowerError::Syntax { count: 1 }));
        }
    }

    #[test]
    fn top_level_execution_cannot_reach_hir() {
        let statement = SourceFile::from_text(
            SourceId::new(100),
            "statement.sico",
            "stdout.write(stdin.read_all())\n".to_owned(),
        )
        .unwrap();
        assert_eq!(lower(&statement), Err(LowerError::Syntax { count: 1 }));

        let labeled = SourceFile::from_text(
            SourceId::new(101),
            "labeled.sico",
            "script:\n  return input.stdin\nend script\n".to_owned(),
        )
        .unwrap();
        assert_eq!(lower(&labeled), Err(LowerError::Syntax { count: 1 }));
    }

    fn collect_sico(root: &Path, output: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_sico(&path, output);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "sico")
            {
                output.push(path);
            }
        }
    }
}
