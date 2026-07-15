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
    pub related: Option<TextRange>,
    pub anchor: RecoveryAnchor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    MissingFunctionClose,
    MissingMatchArmSeparator,
    MissingRecordClose,
    MissingTypeArgumentClose,
    MissingEnumClose,
    MissingCallClose,
    MissingCapabilityClose,
    MissingResourceClose,
    MissingUsingClose,
    MissingTaskClose,
    MissingInterfaceClose,
    MissingParameterListClose,
    MissingBlockClose(BlockKind),
    UnexpectedClose(BlockKind),
    MismatchedClose {
        expected: BlockKind,
        actual: BlockKind,
    },
    MissingClose(BlockKind),
    UnexpectedDelimiter(TokenKind),
    UnclosedDelimiter(TokenKind),
}

/// Deterministic point where parsing resumes after the root cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAnchor {
    NextDefinition,
    NextMatchArm,
    FunctionBody,
    FunctionClose,
    EndOfFile,
    Local,
}

impl RecoveryAnchor {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NextDefinition => "next-definition",
            Self::NextMatchArm => "next-match-arm",
            Self::FunctionBody => "function-body",
            Self::FunctionClose => "function-close",
            Self::EndOfFile => "end-of-file",
            Self::Local => "local",
        }
    }
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
    pub fn ast(&self) -> Option<&ModuleAst> {
        self.is_success().then_some(&self.ast)
    }

    /// Recovered declaration shape for editor/outline use. This must not be
    /// passed to M2 when [`Self::ast`] is `None`.
    #[must_use]
    pub const fn recovered_ast(&self) -> &ModuleAst {
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
    open_range: TextRange,
    name: Option<String>,
    top_level: bool,
}

#[derive(Clone, Debug)]
struct Line {
    start: usize,
    end: usize,
    significant: Vec<usize>,
}

/// Parses one validated source as RFC-0005 B syntax.
///
/// # Panics
///
/// Panics only if the lexer violates its internal guarantees that token ranges
/// are ordered, in bounds, and every non-empty line has a significant token.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn parse(source: &SourceFile) -> Parse {
    let lexed = lex(source);
    let tokens = lexed.tokens();
    let token_count = tokens.len().saturating_sub(1);
    let lines = lines(tokens, token_count);
    let mut starts = BTreeMap::<usize, SyntaxKind>::new();
    let mut finishes = BTreeMap::<usize, usize>::new();
    let mut empty_nodes = BTreeMap::<usize, Vec<SyntaxKind>>::new();
    let mut blocks = Vec::<OpenBlock>::new();
    let mut delimiters = Vec::<(TokenKind, TextRange, usize)>::new();
    let mut declarations = Vec::new();
    let mut errors = Vec::new();

    for line in &lines {
        validate_delimiters(tokens, line, &mut delimiters, &mut errors);
        if line.significant.is_empty() {
            continue;
        }
        let first = tokens[line.significant[0]].kind;

        if first == TokenKind::Case
            && blocks.iter().any(|block| block.kind == BlockKind::Match)
            && line
                .significant
                .last()
                .is_none_or(|index| tokens[*index].kind != TokenKind::Colon)
        {
            let anchor = tokens[*line.significant.last().unwrap()].range.end();
            let related = blocks
                .iter()
                .rev()
                .find(|block| block.kind == BlockKind::Match)
                .map(|block| block.open_range);
            push_recovery_error(
                &mut errors,
                &mut empty_nodes,
                line.end,
                ParseErrorKind::MissingMatchArmSeparator,
                TextRange::empty(anchor),
                related,
                RecoveryAnchor::NextMatchArm,
            );
        }

        recover_function_header(tokens, line, &mut delimiters, &mut errors, &mut empty_nodes);

        if first == TokenKind::End {
            if let Some(actual) = line
                .significant
                .get(1)
                .and_then(|index| close_kind(tokens[*index].kind))
            {
                if actual == BlockKind::Function {
                    recover_call_delimiter(
                        tokens,
                        line,
                        &blocks,
                        &mut delimiters,
                        &mut errors,
                        &mut empty_nodes,
                    );
                }
                let matching = blocks.iter().rposition(|block| block.kind == actual);
                match matching {
                    None if blocks.is_empty() => errors.push(ParseError {
                        kind: ParseErrorKind::UnexpectedClose(actual),
                        range: tokens[line.significant[0]].range,
                        related: None,
                        anchor: RecoveryAnchor::Local,
                    }),
                    None => {
                        let expected = blocks.last().unwrap().kind;
                        errors.push(ParseError {
                            kind: ParseErrorKind::MismatchedClose { expected, actual },
                            range: tokens[line.significant[0]].range,
                            related: blocks.last().map(|block| block.open_range),
                            anchor: RecoveryAnchor::Local,
                        });
                    }
                    Some(position) => {
                        while blocks.len() - 1 > position {
                            let missing = blocks.pop().unwrap();
                            push_recovery_error(
                                &mut errors,
                                &mut empty_nodes,
                                line.start,
                                missing_close_error(missing.kind),
                                TextRange::empty(tokens[line.significant[0]].range.start()),
                                Some(missing.open_range),
                                RecoveryAnchor::FunctionClose,
                            );
                            schedule_finish(&mut finishes, line.start);
                        }
                        let open = blocks.pop().unwrap();
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

        let candidate = opener(tokens, line);
        let starts_top_level = line.significant[0] == line.start
            && (matches!(
                first,
                TokenKind::Newtype
                    | TokenKind::Record
                    | TokenKind::Enum
                    | TokenKind::Capability
                    | TokenKind::Resource
                    | TokenKind::Interface
                    | TokenKind::Function
            ) || (matches!(first, TokenKind::Export | TokenKind::Async)
                && candidate.is_some_and(|(kind, _)| kind == BlockKind::Function)));
        if starts_top_level && blocks.len() == 1 && blocks[0].top_level {
            let missing = blocks.pop().unwrap();
            push_recovery_error(
                &mut errors,
                &mut empty_nodes,
                line.start,
                missing_close_error(missing.kind),
                TextRange::empty(tokens[line.significant[0]].range.start()),
                Some(missing.open_range),
                RecoveryAnchor::NextDefinition,
            );
            schedule_finish(&mut finishes, line.start);
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

        if let Some((kind, opener_index)) = candidate {
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
                open_range: tokens[opener_index].range,
                name,
                top_level,
            });
        }
    }

    for (kind, range, _) in delimiters.into_iter().rev() {
        errors.push(ParseError {
            kind: ParseErrorKind::UnclosedDelimiter(kind),
            range,
            related: None,
            anchor: RecoveryAnchor::EndOfFile,
        });
    }
    while let Some(open) = blocks.pop() {
        push_recovery_error(
            &mut errors,
            &mut empty_nodes,
            token_count,
            missing_close_error(open.kind),
            TextRange::empty(source.len()),
            Some(open.open_range),
            RecoveryAnchor::EndOfFile,
        );
        schedule_finish(&mut finishes, token_count);
    }

    let syntax = build_tree(
        source.text(),
        tokens,
        token_count,
        &starts,
        &finishes,
        &empty_nodes,
    );
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
    Line {
        start,
        end,
        significant,
    }
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

fn missing_close_error(kind: BlockKind) -> ParseErrorKind {
    match kind {
        BlockKind::Function => ParseErrorKind::MissingFunctionClose,
        BlockKind::Record => ParseErrorKind::MissingRecordClose,
        BlockKind::Enum => ParseErrorKind::MissingEnumClose,
        BlockKind::Capability => ParseErrorKind::MissingCapabilityClose,
        BlockKind::Resource => ParseErrorKind::MissingResourceClose,
        BlockKind::Using => ParseErrorKind::MissingUsingClose,
        BlockKind::Task => ParseErrorKind::MissingTaskClose,
        BlockKind::Interface => ParseErrorKind::MissingInterfaceClose,
        BlockKind::Match | BlockKind::If => ParseErrorKind::MissingBlockClose(kind),
    }
}

fn push_recovery_error(
    errors: &mut Vec<ParseError>,
    empty_nodes: &mut BTreeMap<usize, Vec<SyntaxKind>>,
    node_index: usize,
    kind: ParseErrorKind,
    range: TextRange,
    related: Option<TextRange>,
    anchor: RecoveryAnchor,
) {
    empty_nodes
        .entry(node_index)
        .or_default()
        .extend([SyntaxKind::ERROR, SyntaxKind::MISSING]);
    errors.push(ParseError {
        kind,
        range,
        related,
        anchor,
    });
}

fn recover_function_header(
    tokens: &[Token],
    line: &Line,
    delimiters: &mut Vec<(TokenKind, TextRange, usize)>,
    errors: &mut Vec<ParseError>,
    empty_nodes: &mut BTreeMap<usize, Vec<SyntaxKind>>,
) {
    let Some(function_index) = line
        .significant
        .iter()
        .copied()
        .find(|index| tokens[*index].kind == TokenKind::Function)
    else {
        return;
    };
    let Some(returns_index) = line
        .significant
        .iter()
        .copied()
        .find(|index| tokens[*index].kind == TokenKind::Returns)
    else {
        return;
    };

    if let Some(position) = delimiters.iter().rposition(|(kind, _, index)| {
        *kind == TokenKind::LeftParen && *index > function_index && *index < returns_index
    }) {
        let (_, related, _) = delimiters.remove(position);
        push_recovery_error(
            errors,
            empty_nodes,
            returns_index,
            ParseErrorKind::MissingParameterListClose,
            TextRange::empty(tokens[returns_index].range.start()),
            Some(related),
            RecoveryAnchor::FunctionBody,
        );
    }

    let colon_index = *line.significant.last().unwrap();
    if tokens[colon_index].kind == TokenKind::Colon
        && let Some(position) = delimiters.iter().rposition(|(kind, _, index)| {
            *kind == TokenKind::LeftBracket && *index > returns_index && *index < colon_index
        })
    {
        let (_, related, _) = delimiters.remove(position);
        push_recovery_error(
            errors,
            empty_nodes,
            colon_index,
            ParseErrorKind::MissingTypeArgumentClose,
            TextRange::empty(tokens[colon_index].range.start()),
            Some(related),
            RecoveryAnchor::FunctionBody,
        );
    }
}

fn recover_call_delimiter(
    tokens: &[Token],
    line: &Line,
    blocks: &[OpenBlock],
    delimiters: &mut Vec<(TokenKind, TextRange, usize)>,
    errors: &mut Vec<ParseError>,
    empty_nodes: &mut BTreeMap<usize, Vec<SyntaxKind>>,
) {
    let Some(function) = blocks
        .iter()
        .rev()
        .find(|block| block.kind == BlockKind::Function)
    else {
        return;
    };
    if let Some(position) = delimiters.iter().rposition(|(kind, range, _)| {
        *kind == TokenKind::LeftParen && range.start() > function.start
    }) {
        let (_, related, _) = delimiters.remove(position);
        push_recovery_error(
            errors,
            empty_nodes,
            line.start,
            ParseErrorKind::MissingCallClose,
            TextRange::empty(tokens[line.significant[0]].range.start()),
            Some(related),
            RecoveryAnchor::FunctionClose,
        );
    }
}

fn validate_delimiters(
    tokens: &[Token],
    line: &Line,
    stack: &mut Vec<(TokenKind, TextRange, usize)>,
    errors: &mut Vec<ParseError>,
) {
    for index in &line.significant {
        let token = tokens[*index];
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => {
                stack.push((token.kind, token.range, *index));
            }
            TokenKind::RightParen | TokenKind::RightBracket => {
                let expected = if token.kind == TokenKind::RightParen {
                    TokenKind::LeftParen
                } else {
                    TokenKind::LeftBracket
                };
                if stack.last().is_some_and(|(kind, _, _)| *kind == expected) {
                    stack.pop();
                } else {
                    errors.push(ParseError {
                        kind: ParseErrorKind::UnexpectedDelimiter(token.kind),
                        range: token.range,
                        related: None,
                        anchor: RecoveryAnchor::Local,
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
    empty_nodes: &BTreeMap<usize, Vec<SyntaxKind>>,
) -> SyntaxNode {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(rowan_kind(SyntaxKind::ROOT));
    for (index, token) in tokens.iter().copied().take(token_count).enumerate() {
        if let Some(kinds) = empty_nodes.get(&index) {
            for kind in kinds {
                builder.start_node(rowan_kind(*kind));
                builder.finish_node();
            }
        }
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
    if let Some(kinds) = empty_nodes.get(&token_count) {
        for kind in kinds {
            builder.start_node(rowan_kind(*kind));
            builder.finish_node();
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
                !parsed.ast().unwrap().declarations().is_empty(),
                "{}",
                path.display()
            );
            let relative = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            shapes.push(format!("{relative} = {}", parsed.ast().unwrap().shape()));
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

    #[test]
    fn b_mutations_recover_with_one_registered_root_cause() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-mutations/b");
        let expected = [
            (
                "MUT-001-missing-function-close.sico",
                ParseErrorKind::MissingFunctionClose,
                RecoveryAnchor::NextDefinition,
            ),
            (
                "MUT-002-missing-match-arm-separator.sico",
                ParseErrorKind::MissingMatchArmSeparator,
                RecoveryAnchor::NextMatchArm,
            ),
            (
                "MUT-003-missing-record-close.sico",
                ParseErrorKind::MissingRecordClose,
                RecoveryAnchor::NextDefinition,
            ),
            (
                "MUT-004-missing-type-argument-close.sico",
                ParseErrorKind::MissingTypeArgumentClose,
                RecoveryAnchor::FunctionBody,
            ),
            (
                "MUT-005-missing-enum-close.sico",
                ParseErrorKind::MissingEnumClose,
                RecoveryAnchor::NextDefinition,
            ),
            (
                "MUT-006-missing-call-close.sico",
                ParseErrorKind::MissingCallClose,
                RecoveryAnchor::FunctionClose,
            ),
            (
                "MUT-007-missing-capability-close.sico",
                ParseErrorKind::MissingCapabilityClose,
                RecoveryAnchor::NextDefinition,
            ),
            (
                "MUT-008-missing-resource-close.sico",
                ParseErrorKind::MissingResourceClose,
                RecoveryAnchor::NextDefinition,
            ),
            (
                "MUT-009-missing-using-close.sico",
                ParseErrorKind::MissingUsingClose,
                RecoveryAnchor::FunctionClose,
            ),
            (
                "MUT-010-missing-task-group-close.sico",
                ParseErrorKind::MissingTaskClose,
                RecoveryAnchor::FunctionClose,
            ),
            (
                "MUT-011-missing-interface-close.sico",
                ParseErrorKind::MissingInterfaceClose,
                RecoveryAnchor::EndOfFile,
            ),
            (
                "MUT-012-missing-parameter-list-close.sico",
                ParseErrorKind::MissingParameterListClose,
                RecoveryAnchor::FunctionBody,
            ),
        ];
        for (index, (name, expected_kind, expected_anchor)) in expected.into_iter().enumerate() {
            let path = root.join(name);
            let source = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(&path).unwrap(),
            )
            .unwrap();
            let parsed = parse(&source);
            assert!(parsed.lex_errors().is_empty(), "{name}");
            assert_eq!(parsed.errors().len(), 1, "{name}: {:?}", parsed.errors());
            assert_eq!(parsed.errors()[0].kind, expected_kind, "{name}");
            assert_eq!(parsed.errors()[0].anchor, expected_anchor, "{name}");
            assert!(parsed.errors()[0].related.is_some(), "{name}");
            assert!(parsed.ast().is_none(), "{name}");
            assert_eq!(parsed.syntax().text().to_string(), source.text(), "{name}");
            let kinds: Vec<_> = parsed
                .syntax()
                .descendants()
                .map(|node| node.kind())
                .collect();
            assert!(kinds.contains(&SyntaxKind::ERROR), "{name}");
            assert!(kinds.contains(&SyntaxKind::MISSING), "{name}");
        }
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
