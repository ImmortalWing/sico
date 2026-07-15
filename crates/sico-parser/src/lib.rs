//! Lossless parser for the accepted B labeled-block grammar.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sico_lexer::{LexError, Token, TokenKind, lex};
use sico_source::{SourceFile, TextRange, TextSize};
use sico_syntax::{GreenNodeBuilder, SyntaxKind, SyntaxNode, root};

/// B block identity carried by both opener and `end <kind>`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockKind {
    Record,
    Enum,
    Capability,
    Resource,
    Interface,
    Function,
    Match,
    If,
    Using,
    Task,
}

impl BlockKind {
    const fn syntax(self) -> SyntaxKind {
        match self {
            Self::Record => SyntaxKind::RECORD_DECL,
            Self::Enum => SyntaxKind::ENUM_DECL,
            Self::Capability => SyntaxKind::CAPABILITY_DECL,
            Self::Resource => SyntaxKind::RESOURCE_DECL,
            Self::Interface => SyntaxKind::INTERFACE_DECL,
            Self::Function => SyntaxKind::FUNCTION_DECL,
            Self::Match => SyntaxKind::MATCH_BLOCK,
            Self::If => SyntaxKind::IF_BLOCK,
            Self::Using => SyntaxKind::USING_BLOCK,
            Self::Task => SyntaxKind::TASK_BLOCK,
        }
    }

    const fn declaration(self) -> Option<DeclarationKind> {
        match self {
            Self::Record => Some(DeclarationKind::Record),
            Self::Enum => Some(DeclarationKind::Enum),
            Self::Capability => Some(DeclarationKind::Capability),
            Self::Resource => Some(DeclarationKind::Resource),
            Self::Interface => Some(DeclarationKind::Interface),
            Self::Function => Some(DeclarationKind::Function),
            Self::Match | Self::If | Self::Using | Self::Task => None,
        }
    }
}

/// Minimal semantic AST declaration kind. No M2 judgment is represented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    Newtype,
    Record,
    Enum,
    Capability,
    Resource,
    Interface,
    Function,
}

/// Top-level declaration shape used by later semantic lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    pub kind: DeclarationKind,
    pub name: String,
    pub range: TextRange,
}

/// Minimal semantic module shape.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModuleAst {
    declarations: Vec<Declaration>,
}

impl ModuleAst {
    #[must_use]
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Stable compact representation used by AST snapshots and outline tests.
    #[must_use]
    pub fn shape(&self) -> String {
        self.declarations
            .iter()
            .map(|declaration| format!("{:?}:{}", declaration.kind, declaration.name))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Happy-path structural parser error. Recovery-specific errors are extended in
/// STEP-0018 without changing successful tree shapes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub range: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedClose(BlockKind),
    MismatchedClose {
        expected: BlockKind,
        actual: BlockKind,
    },
    MissingClose(BlockKind),
    UnexpectedDelimiter(TokenKind),
    UnclosedDelimiter(TokenKind),
}

/// Parser output keeps source, lexical, syntax, and semantic-shape layers apart.
#[derive(Clone, Debug)]
pub struct Parse {
    syntax: SyntaxNode,
    ast: ModuleAst,
    lex_errors: Vec<LexError>,
    errors: Vec<ParseError>,
}

impl Parse {
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }

    #[must_use]
    pub const fn ast(&self) -> &ModuleAst {
        &self.ast
    }

    #[must_use]
    pub fn lex_errors(&self) -> &[LexError] {
        &self.lex_errors
    }

    #[must_use]
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    #[must_use]
    pub fn is_success(&self) -> bool {
        self.lex_errors.is_empty() && self.errors.is_empty()
    }
}

#[derive(Clone, Debug)]
struct OpenBlock {
    kind: BlockKind,
    start: TextSize,
    name: Option<String>,
    top_level: bool,
}

#[derive(Clone, Debug)]
struct Line {
    end: usize,
    significant: Vec<usize>,
}

/// Parses one validated source as RFC-0005 B syntax.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn parse(source: &SourceFile) -> Parse {
    let lexed = lex(source);
    let tokens = lexed.tokens();
    let token_count = tokens.len().saturating_sub(1);
    let lines = lines(tokens, token_count);
    let mut starts = BTreeMap::<usize, SyntaxKind>::new();
    let mut finishes = BTreeMap::<usize, usize>::new();
    let mut blocks = Vec::<OpenBlock>::new();
    let mut delimiters = Vec::<(TokenKind, TextRange)>::new();
    let mut declarations = Vec::new();
    let mut errors = Vec::new();

    for line in &lines {
        validate_delimiters(tokens, line, &mut delimiters, &mut errors);
        if line.significant.is_empty() {
            continue;
        }
        let first = tokens[line.significant[0]].kind;

        if first == TokenKind::End {
            if let Some(actual) = line
                .significant
                .get(1)
                .and_then(|index| close_kind(tokens[*index].kind))
            {
                match blocks.pop() {
                    None => errors.push(ParseError {
                        kind: ParseErrorKind::UnexpectedClose(actual),
                        range: tokens[line.significant[0]].range,
                    }),
                    Some(open) if open.kind != actual => {
                        errors.push(ParseError {
                            kind: ParseErrorKind::MismatchedClose {
                                expected: open.kind,
                                actual,
                            },
                            range: tokens[line.significant[0]].range,
                        });
                        schedule_finish(&mut finishes, line.end);
                    }
                    Some(open) => {
                        schedule_finish(&mut finishes, line.end);
                        if open.top_level
                            && let (Some(kind), Some(name)) = (open.kind.declaration(), open.name)
                        {
                            declarations.push(Declaration {
                                kind,
                                name,
                                range: TextRange::new(open.start, tokens[line.end - 1].range.end()),
                            });
                        }
                    }
                }
            }
            continue;
        }

        if blocks.is_empty() && first == TokenKind::Newtype {
            let start_index = line.significant[0];
            starts.insert(start_index, SyntaxKind::NEWTYPE_DECL);
            schedule_finish(&mut finishes, line.end);
            if let Some(name) =
                identifier_after(source.text(), tokens, &line.significant, start_index)
            {
                declarations.push(Declaration {
                    kind: DeclarationKind::Newtype,
                    name,
                    range: TextRange::new(
                        tokens[start_index].range.start(),
                        tokens[line.end - 1].range.end(),
                    ),
                });
            }
            continue;
        }

        if let Some((kind, opener_index)) = opener(tokens, line) {
            let top_level = blocks.is_empty() && kind.declaration().is_some();
            let node_start = if top_level {
                line.significant[0]
            } else {
                opener_index
            };
            starts.insert(node_start, kind.syntax());
            let name = kind.declaration().and_then(|_| {
                identifier_after(source.text(), tokens, &line.significant, opener_index)
            });
            blocks.push(OpenBlock {
                kind,
                start: tokens[node_start].range.start(),
                name,
                top_level,
            });
        }
    }

    for (kind, range) in delimiters.into_iter().rev() {
        errors.push(ParseError {
            kind: ParseErrorKind::UnclosedDelimiter(kind),
            range,
        });
    }
    while let Some(open) = blocks.pop() {
        errors.push(ParseError {
            kind: ParseErrorKind::MissingClose(open.kind),
            range: TextRange::empty(open.start),
        });
        schedule_finish(&mut finishes, token_count);
    }

    let syntax = build_tree(source.text(), tokens, token_count, &starts, &finishes);
    Parse {
        syntax,
        ast: ModuleAst { declarations },
        lex_errors: lexed.errors().to_vec(),
        errors,
    }
}

fn lines(tokens: &[Token], token_count: usize) -> Vec<Line> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, token) in tokens.iter().copied().take(token_count).enumerate() {
        if token.kind == TokenKind::Newline {
            result.push(line(tokens, start, index + 1));
            start = index + 1;
        }
    }
    if start < token_count {
        result.push(line(tokens, start, token_count));
    }
    result
}

fn line(tokens: &[Token], start: usize, end: usize) -> Line {
    let significant = (start..end)
        .filter(|index| !tokens[*index].kind.is_trivia())
        .collect();
    Line { end, significant }
}

fn opener(tokens: &[Token], line: &Line) -> Option<(BlockKind, usize)> {
    let last = *line.significant.last()?;
    if tokens[last].kind != TokenKind::Colon {
        return None;
    }
    let first = line.significant[0];
    let first_kind = tokens[first].kind;
    let direct = match first_kind {
        TokenKind::Record => Some(BlockKind::Record),
        TokenKind::Enum => Some(BlockKind::Enum),
        TokenKind::Capability => Some(BlockKind::Capability),
        TokenKind::Resource => Some(BlockKind::Resource),
        TokenKind::Interface => Some(BlockKind::Interface),
        TokenKind::Match => Some(BlockKind::Match),
        TokenKind::If => Some(BlockKind::If),
        TokenKind::Using => Some(BlockKind::Using),
        TokenKind::Task => Some(BlockKind::Task),
        _ => None,
    };
    if let Some(kind) = direct {
        return Some((kind, first));
    }
    line.significant
        .iter()
        .copied()
        .find(|index| tokens[*index].kind == TokenKind::Function)
        .map(|index| (BlockKind::Function, index))
}

fn close_kind(kind: TokenKind) -> Option<BlockKind> {
    Some(match kind {
        TokenKind::Record => BlockKind::Record,
        TokenKind::Enum => BlockKind::Enum,
        TokenKind::Capability => BlockKind::Capability,
        TokenKind::Resource => BlockKind::Resource,
        TokenKind::Interface => BlockKind::Interface,
        TokenKind::Function => BlockKind::Function,
        TokenKind::Match => BlockKind::Match,
        TokenKind::If => BlockKind::If,
        TokenKind::Using => BlockKind::Using,
        TokenKind::Task => BlockKind::Task,
        _ => return None,
    })
}

fn identifier_after(
    text: &str,
    tokens: &[Token],
    significant: &[usize],
    opener: usize,
) -> Option<String> {
    significant
        .iter()
        .copied()
        .skip_while(|index| *index != opener)
        .skip(1)
        .find(|index| tokens[*index].kind == TokenKind::Identifier)
        .map(|index| {
            let range = tokens[index].range;
            text[usize::from(range.start())..usize::from(range.end())].to_owned()
        })
}

fn validate_delimiters(
    tokens: &[Token],
    line: &Line,
    stack: &mut Vec<(TokenKind, TextRange)>,
    errors: &mut Vec<ParseError>,
) {
    for index in &line.significant {
        let token = tokens[*index];
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => stack.push((token.kind, token.range)),
            TokenKind::RightParen | TokenKind::RightBracket => {
                let expected = if token.kind == TokenKind::RightParen {
                    TokenKind::LeftParen
                } else {
                    TokenKind::LeftBracket
                };
                if stack.last().is_some_and(|(kind, _)| *kind == expected) {
                    stack.pop();
                } else {
                    errors.push(ParseError {
                        kind: ParseErrorKind::UnexpectedDelimiter(token.kind),
                        range: token.range,
                    });
                }
            }
            _ => {}
        }
    }
}

fn schedule_finish(finishes: &mut BTreeMap<usize, usize>, index: usize) {
    *finishes.entry(index).or_default() += 1;
}

fn build_tree(
    text: &str,
    tokens: &[Token],
    token_count: usize,
    starts: &BTreeMap<usize, SyntaxKind>,
    finishes: &BTreeMap<usize, usize>,
) -> SyntaxNode {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(rowan_kind(SyntaxKind::ROOT));
    for (index, token) in tokens.iter().copied().take(token_count).enumerate() {
        if let Some(kind) = starts.get(&index) {
            builder.start_node(rowan_kind(*kind));
        }
        let start = usize::from(token.range.start());
        let end = usize::from(token.range.end());
        builder.token(
            rowan_kind(SyntaxKind::token(token.kind as u16)),
            &text[start..end],
        );
        if let Some(count) = finishes.get(&(index + 1)) {
            for _ in 0..*count {
                builder.finish_node();
            }
        }
    }
    builder.finish_node();
    root(builder.finish())
}

fn rowan_kind(kind: SyntaxKind) -> rowan::SyntaxKind {
    rowan::SyntaxKind(kind.raw())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use sico_source::SourceId;

    use super::*;

    #[test]
    fn all_b_canonical_cases_parse_losslessly_without_semantic_rejection() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-candidates/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 54);
        let mut designed_accept = 0;
        let mut designed_reject = 0;
        let mut shapes = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let bytes = fs::read(path).unwrap();
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &bytes,
            )
            .unwrap();
            let parsed = parse(&source);
            assert!(
                parsed.is_success(),
                "{}: {:?}",
                path.display(),
                parsed.errors()
            );
            assert_eq!(
                parsed.syntax().text().to_string(),
                source.text(),
                "{}",
                path.display()
            );
            assert!(
                !parsed.ast().declarations().is_empty(),
                "{}",
                path.display()
            );
            let relative = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            shapes.push(format!("{relative} = {}", parsed.ast().shape()));
            if source.text().contains("// expect: accept") {
                designed_accept += 1;
            } else if source.text().contains("// expect: reject(") {
                designed_reject += 1;
            }
        }
        assert_eq!((designed_accept, designed_reject), (25, 29));
        let snapshot = format!("{}\n", shapes.join("\n"));
        if std::env::var_os("SICO_DUMP_SHAPES").is_some() {
            print!("{snapshot}");
        } else {
            assert_eq!(
                snapshot,
                include_str!("../../../tests/parser/b-ast-shapes.txt")
            );
        }
    }

    #[test]
    fn representative_nested_blocks_have_stable_node_kinds() {
        let text = "function main() returns Text:\n  if true:\n    match true:\n      case true:\n        return \"yes\"\n    end match\n  end if\nend function\n";
        let source = SourceFile::from_text(SourceId::new(1), "nested.sico", text).unwrap();
        let parsed = parse(&source);
        assert!(parsed.is_success(), "{:?}", parsed.errors());
        let kinds: Vec<_> = parsed
            .syntax()
            .descendants()
            .map(|node| node.kind())
            .collect();
        assert!(kinds.contains(&SyntaxKind::FUNCTION_DECL));
        assert!(kinds.contains(&SyntaxKind::IF_BLOCK));
        assert!(kinds.contains(&SyntaxKind::MATCH_BLOCK));
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
