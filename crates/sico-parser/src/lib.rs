//! Lossless parser for the accepted B labeled-block grammar.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sico_lexer::{LexError, Token, TokenKind, lex};
use sico_source::{SourceFile, TextRange, TextSize};
use sico_syntax::{GreenNodeBuilder, SyntaxKind, SyntaxNode, root};

/// Maximum combined block or delimiter nesting accepted by the parser.
pub const MAX_PARSE_DEPTH: usize = 256;
/// Maximum parser diagnostics retained for one source.
pub const MAX_PARSE_ERRORS: usize = 100;

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
    While,
    For,
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
            Self::While | Self::For => SyntaxKind::WHILE_BLOCK,
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
            Self::Match | Self::If | Self::While | Self::For | Self::Using | Self::Task => None,
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
    /// RFC-0039: a single-line `module <name>` declaration (STEP-0143).
    Module,
    /// RFC-0039: a single-line `use <module>.<item>` declaration (STEP-0143).
    /// The name carries the normalized `<module>.<item>` path.
    Use,
}

/// RFC-0039 §2.3/§2.4 (STEP-0147): structured payload for the declaration
/// forms whose identity is more than a name. Absent (`None`) everywhere the
/// pre-existing surface is unchanged.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum DeclarationDetail {
    #[default]
    None,
    /// `use pkg <name> version <n> [expose <interface>]`: the resolved
    /// package dependency and the user WIT interface it must expose.
    Package {
        package: String,
        version: u64,
        expose: Option<String>,
    },
    /// `interface <name> version <n>:`: the boundary identity spelling; the
    /// version-less corpus form stays accepted with `None` semantics.
    InterfaceVersion(u64),
}

/// Top-level declaration shape used by later semantic lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    pub kind: DeclarationKind,
    pub name: String,
    pub range: TextRange,
    pub detail: DeclarationDetail,
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
    UnexpectedTopLevel,
    /// RFC-0039: `module` declarations are exactly `module <name>` (E1014).
    InvalidModuleDeclaration,
    /// RFC-0039: `use` declarations are exactly `use <module>.<item>` or the
    /// RFC-0039 §2.3 package form (E1015).
    InvalidUseDeclaration,
    /// RFC-0039 §2.4 (STEP-0147): `interface <name> version <n>:` is the only
    /// versioned spelling and `<n>` must be a positive integer (E1016).
    InvalidInterfaceVersion,
    NestingLimitExceeded {
        limit: usize,
    },
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
    version: Option<u64>,
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
    let mut depth_limit_exceeded = false;
    // RFC-0033 rejects Candidate C without reserving `script` as a keyword.
    // This recovery state suppresses one diagnostic per body line and resumes
    // at the explicit close or the next declaration.
    let mut rejected_script_block = false;

    for line in &lines {
        let errors_before_line = errors.len();
        if validate_delimiters(tokens, line, &mut delimiters, &mut errors, &mut empty_nodes) {
            depth_limit_exceeded = true;
            break;
        }
        if line.significant.is_empty() {
            continue;
        }
        let first = tokens[line.significant[0]].kind;

        if rejected_script_block {
            if is_rejected_script_close(source.text(), tokens, line) {
                rejected_script_block = false;
                continue;
            }
            if starts_top_level_declaration(tokens, line) {
                rejected_script_block = false;
            } else {
                continue;
            }
        }

        if blocks.is_empty()
            && errors.len() == errors_before_line
            && is_rejected_script_opener(source.text(), tokens, line)
        {
            push_parse_error(
                &mut errors,
                ParseError {
                    kind: ParseErrorKind::UnexpectedTopLevel,
                    range: tokens[line.significant[0]].range,
                    related: None,
                    anchor: RecoveryAnchor::NextDefinition,
                },
            );
            rejected_script_block = true;
            continue;
        }

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
                    None if blocks.is_empty() => {
                        push_parse_error(
                            &mut errors,
                            ParseError {
                                kind: ParseErrorKind::UnexpectedClose(actual),
                                range: tokens[line.significant[0]].range,
                                related: None,
                                anchor: RecoveryAnchor::Local,
                            },
                        );
                    }
                    None => {
                        let expected = blocks.last().unwrap().kind;
                        push_parse_error(
                            &mut errors,
                            ParseError {
                                kind: ParseErrorKind::MismatchedClose { expected, actual },
                                range: tokens[line.significant[0]].range,
                                related: blocks.last().map(|block| block.open_range),
                                anchor: RecoveryAnchor::Local,
                            },
                        );
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
                            let detail = match open.version {
                                Some(version) if kind == DeclarationKind::Interface => {
                                    DeclarationDetail::InterfaceVersion(version)
                                }
                                _ => DeclarationDetail::None,
                            };
                            declarations.push(Declaration {
                                kind,
                                name,
                                range: TextRange::new(open.start, tokens[line.end - 1].range.end()),
                                detail,
                            });
                        }
                    }
                }
            } else if blocks.is_empty() && errors.len() == errors_before_line {
                push_parse_error(
                    &mut errors,
                    ParseError {
                        kind: ParseErrorKind::UnexpectedTopLevel,
                        range: tokens[line.significant[0]].range,
                        related: None,
                        anchor: RecoveryAnchor::NextDefinition,
                    },
                );
            }
            continue;
        }

        let candidate = opener(tokens, line);
        let starts_top_level = starts_top_level_declaration(tokens, line);
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
                    detail: DeclarationDetail::None,
                });
            }
            continue;
        }

        // RFC-0039 (STEP-0143): single-line `module <name>` declarations.
        if blocks.is_empty() && first == TokenKind::Module {
            let start_index = line.significant[0];
            starts.insert(start_index, SyntaxKind::MODULE_DECL);
            schedule_finish(&mut finishes, line.end);
            if line.significant.len() == 2
                && tokens[line.significant[1]].kind == TokenKind::Identifier
            {
                let name_index = line.significant[1];
                declarations.push(Declaration {
                    kind: DeclarationKind::Module,
                    name: token_text(source.text(), tokens[name_index]),
                    range: TextRange::new(
                        tokens[start_index].range.start(),
                        tokens[line.end - 1].range.end(),
                    ),
                    detail: DeclarationDetail::None,
                });
            } else if errors.len() == errors_before_line {
                push_parse_error(
                    &mut errors,
                    ParseError {
                        kind: ParseErrorKind::InvalidModuleDeclaration,
                        range: tokens[start_index].range,
                        related: None,
                        anchor: RecoveryAnchor::NextDefinition,
                    },
                );
            }
            continue;
        }

        // RFC-0039 (STEP-0143): single-line `use <module>.<item>` declarations,
        // extended in STEP-0147 with the §2.3 package form
        // `use pkg <name> version <n> [expose <interface>]`.
        if blocks.is_empty() && first == TokenKind::Use {
            let start_index = line.significant[0];
            starts.insert(start_index, SyntaxKind::USE_DECL);
            schedule_finish(&mut finishes, line.end);
            let shaped = line.significant.len() == 4
                && tokens[line.significant[1]].kind == TokenKind::Identifier
                && tokens[line.significant[2]].kind == TokenKind::Dot
                && tokens[line.significant[3]].kind == TokenKind::Identifier;
            if shaped {
                let module_index = line.significant[1];
                let item_index = line.significant[3];
                let name = format!(
                    "{}.{}",
                    token_text(source.text(), tokens[module_index]),
                    token_text(source.text(), tokens[item_index])
                );
                declarations.push(Declaration {
                    kind: DeclarationKind::Use,
                    name,
                    range: TextRange::new(
                        tokens[start_index].range.start(),
                        tokens[line.end - 1].range.end(),
                    ),
                    detail: DeclarationDetail::None,
                });
            } else if let Some(declaration) =
                parse_use_package(source.text(), tokens, &line.significant)
            {
                declarations.push(declaration);
            } else if errors.len() == errors_before_line {
                push_parse_error(
                    &mut errors,
                    ParseError {
                        kind: ParseErrorKind::InvalidUseDeclaration,
                        range: tokens[start_index].range,
                        related: None,
                        anchor: RecoveryAnchor::NextDefinition,
                    },
                );
            }
            continue;
        }

        if let Some((kind, opener_index)) = candidate {
            if blocks.is_empty()
                && kind.declaration().is_none()
                && errors.len() == errors_before_line
            {
                push_parse_error(
                    &mut errors,
                    ParseError {
                        kind: ParseErrorKind::UnexpectedTopLevel,
                        range: tokens[line.significant[0]].range,
                        related: None,
                        anchor: RecoveryAnchor::NextDefinition,
                    },
                );
            }
            if blocks.len() >= MAX_PARSE_DEPTH {
                push_recovery_error(
                    &mut errors,
                    &mut empty_nodes,
                    line.start,
                    ParseErrorKind::NestingLimitExceeded {
                        limit: MAX_PARSE_DEPTH,
                    },
                    tokens[opener_index].range,
                    None,
                    RecoveryAnchor::EndOfFile,
                );
                depth_limit_exceeded = true;
                break;
            }
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
            let version = if kind == BlockKind::Interface {
                interface_version(source.text(), tokens, &line.significant, opener_index).or_else(
                    || {
                        if errors.len() == errors_before_line {
                            // A `version` token present but malformed is a
                            // typed parse error (E1016); a version-less
                            // opener stays the accepted corpus form.
                            let has_version = line
                                .significant
                                .iter()
                                .copied()
                                .skip_while(|index| *index != opener_index)
                                .skip(1)
                                .any(|index| {
                                    tokens[index].kind == TokenKind::Identifier
                                        && token_text_is(source.text(), tokens[index], "version")
                                });
                            if has_version {
                                push_parse_error(
                                    &mut errors,
                                    ParseError {
                                        kind: ParseErrorKind::InvalidInterfaceVersion,
                                        range: tokens[opener_index].range,
                                        related: None,
                                        anchor: RecoveryAnchor::NextDefinition,
                                    },
                                );
                            }
                        }
                        None
                    },
                )
            } else {
                None
            };
            blocks.push(OpenBlock {
                kind,
                start: tokens[node_start].range.start(),
                open_range: tokens[opener_index].range,
                name,
                version,
                top_level,
            });
        } else if blocks.is_empty() && errors.len() == errors_before_line {
            push_parse_error(
                &mut errors,
                ParseError {
                    kind: ParseErrorKind::UnexpectedTopLevel,
                    range: tokens[line.significant[0]].range,
                    related: None,
                    anchor: RecoveryAnchor::NextDefinition,
                },
            );
        }
    }

    if depth_limit_exceeded {
        while blocks.pop().is_some() {
            schedule_finish(&mut finishes, token_count);
        }
    } else {
        for (kind, range, _) in delimiters.into_iter().rev() {
            push_parse_error(
                &mut errors,
                ParseError {
                    kind: ParseErrorKind::UnclosedDelimiter(kind),
                    range,
                    related: None,
                    anchor: RecoveryAnchor::EndOfFile,
                },
            );
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
        TokenKind::While => Some(BlockKind::While),
        TokenKind::For => Some(BlockKind::For),
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
        TokenKind::While => BlockKind::While,
        TokenKind::For => BlockKind::For,
        TokenKind::Using => BlockKind::Using,
        TokenKind::Task => BlockKind::Task,
        _ => return None,
    })
}

fn is_rejected_script_opener(text: &str, tokens: &[Token], line: &Line) -> bool {
    line.significant.len() == 2
        && token_text_is(text, tokens[line.significant[0]], "script")
        && tokens[line.significant[1]].kind == TokenKind::Colon
}

fn starts_top_level_declaration(tokens: &[Token], line: &Line) -> bool {
    if line.significant[0] != line.start {
        return false;
    }
    let first = tokens[line.significant[0]].kind;
    matches!(
        first,
        TokenKind::Newtype
            | TokenKind::Record
            | TokenKind::Enum
            | TokenKind::Capability
            | TokenKind::Resource
            | TokenKind::Interface
            | TokenKind::Function
            | TokenKind::Module
            | TokenKind::Use
    ) || (matches!(first, TokenKind::Export | TokenKind::Async)
        && opener(tokens, line).is_some_and(|(kind, _)| kind == BlockKind::Function))
}

fn is_rejected_script_close(text: &str, tokens: &[Token], line: &Line) -> bool {
    line.significant.len() == 2
        && tokens[line.significant[0]].kind == TokenKind::End
        && token_text_is(text, tokens[line.significant[1]], "script")
}

fn token_text_is(text: &str, token: Token, expected: &str) -> bool {
    let start = usize::from(token.range.start());
    let end = usize::from(token.range.end());
    &text[start..end] == expected
}

fn token_text(text: &str, token: Token) -> String {
    let start = usize::from(token.range.start());
    let end = usize::from(token.range.end());
    text[start..end].to_owned()
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

/// RFC-0039 §2.4 (STEP-0147): the opener line of an interface block may
/// carry the boundary version — `interface <name> version <n>:` — as five
/// significant tokens. Returns `None` for the version-less corpus form and
/// for anything else (malformed versions are refused by the caller).
fn interface_version(
    text: &str,
    tokens: &[Token],
    significant: &[usize],
    opener: usize,
) -> Option<u64> {
    let rest: Vec<usize> = significant
        .iter()
        .copied()
        .skip_while(|index| *index != opener)
        .skip(1)
        .collect();
    if rest.len() != 4
        || tokens[rest[1]].kind != TokenKind::Identifier
        || !token_text_is(text, tokens[rest[1]], "version")
        || tokens[rest[2]].kind != TokenKind::Integer
    {
        return None;
    }
    let digits = token_text(text, tokens[rest[2]]);
    if digits.starts_with('0') {
        return None;
    }
    digits.parse::<u64>().ok().filter(|version| *version > 0)
}

/// RFC-0039 §2.3 (STEP-0147): `use pkg <name> version <n> [expose
/// <interface>]`. The `pkg`, `version` and `expose` markers are contextual:
/// they are only special inside this line shape, so existing sources using
/// them as identifiers are unaffected.
fn parse_use_package(text: &str, tokens: &[Token], significant: &[usize]) -> Option<Declaration> {
    let rest: Vec<usize> = significant.iter().copied().skip(1).collect();
    let package_form = rest.len() == 4 || rest.len() == 6;
    if !package_form
        || tokens[rest[0]].kind != TokenKind::Identifier
        || !token_text_is(text, tokens[rest[0]], "pkg")
        || tokens[rest[1]].kind != TokenKind::Identifier
        || tokens[rest[2]].kind != TokenKind::Identifier
        || !token_text_is(text, tokens[rest[2]], "version")
        || tokens[rest[3]].kind != TokenKind::Integer
    {
        return None;
    }
    if rest.len() == 6
        && (tokens[rest[4]].kind != TokenKind::Identifier
            || !token_text_is(text, tokens[rest[4]], "expose")
            || tokens[rest[5]].kind != TokenKind::Identifier)
    {
        return None;
    }
    let package = token_text(text, tokens[rest[1]]);
    if package.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let digits = token_text(text, tokens[rest[3]]);
    if digits.starts_with('0') {
        return None;
    }
    let version = digits.parse::<u64>().ok().filter(|version| *version > 0)?;
    let expose = if rest.len() == 6 {
        Some(token_text(text, tokens[rest[5]]))
    } else {
        None
    };
    let start = significant.first().copied()?;
    let end = significant.last().copied()?;
    Some(Declaration {
        kind: DeclarationKind::Use,
        name: package.clone(),
        range: TextRange::new(tokens[start].range.start(), tokens[end].range.end()),
        detail: DeclarationDetail::Package {
            package,
            version,
            expose,
        },
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
        BlockKind::Match | BlockKind::If | BlockKind::While | BlockKind::For => {
            ParseErrorKind::MissingBlockClose(kind)
        }
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
    if push_parse_error(
        errors,
        ParseError {
            kind,
            range,
            related,
            anchor,
        },
    ) {
        empty_nodes
            .entry(node_index)
            .or_default()
            .extend([SyntaxKind::ERROR, SyntaxKind::MISSING]);
    }
}

fn push_parse_error(errors: &mut Vec<ParseError>, error: ParseError) -> bool {
    if errors.len() == MAX_PARSE_ERRORS {
        return false;
    }
    errors.push(error);
    true
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
    empty_nodes: &mut BTreeMap<usize, Vec<SyntaxKind>>,
) -> bool {
    for index in &line.significant {
        let token = tokens[*index];
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => {
                if stack.len() >= MAX_PARSE_DEPTH {
                    push_recovery_error(
                        errors,
                        empty_nodes,
                        *index,
                        ParseErrorKind::NestingLimitExceeded {
                            limit: MAX_PARSE_DEPTH,
                        },
                        token.range,
                        None,
                        RecoveryAnchor::EndOfFile,
                    );
                    return true;
                }
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
                    push_parse_error(
                        errors,
                        ParseError {
                            kind: ParseErrorKind::UnexpectedDelimiter(token.kind),
                            range: token.range,
                            related: None,
                            anchor: RecoveryAnchor::Local,
                        },
                    );
                }
            }
            _ => {}
        }
    }
    false
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
        assert_eq!(paths.len(), 58);
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
        assert_eq!((designed_accept, designed_reject), (25, 33));
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
    fn top_level_execution_candidates_fail_closed_and_recover_at_declarations() {
        let statement = source(
            "stdout.write(stdin.read_all())\nfunction main() returns Int:\n  return 1\nend function\n",
        );
        let parsed = parse(&statement);
        assert_eq!(parsed.errors().len(), 1);
        assert_eq!(parsed.errors()[0].kind, ParseErrorKind::UnexpectedTopLevel);
        assert_eq!(parsed.errors()[0].anchor, RecoveryAnchor::NextDefinition);
        assert_eq!(parsed.recovered_ast().shape(), "Function:main");

        let labeled = source(
            "script:\n  return input.stdin\nend script\nfunction main() returns Int:\n  return 1\nend function\n",
        );
        let parsed = parse(&labeled);
        assert_eq!(parsed.errors().len(), 1);
        assert_eq!(parsed.errors()[0].kind, ParseErrorKind::UnexpectedTopLevel);
        assert_eq!(parsed.recovered_ast().shape(), "Function:main");

        let missing_close = source(
            "script:\n  return input.stdin\nfunction main() returns Int:\n  return 1\nend function\n",
        );
        let parsed = parse(&missing_close);
        assert_eq!(parsed.errors().len(), 1);
        assert_eq!(parsed.recovered_ast().shape(), "Function:main");
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

    #[test]
    fn parser_depth_and_diagnostic_limits_are_exact_and_lossless() {
        let at_limit = nested_blocks(MAX_PARSE_DEPTH);
        let file = source(&at_limit);
        let parsed = parse(&file);
        assert!(parsed.is_success(), "{:?}", parsed.errors());
        assert_eq!(parsed.syntax().text().to_string(), at_limit);

        let over_limit = nested_blocks(MAX_PARSE_DEPTH + 1);
        let file = source(&over_limit);
        let parsed = parse(&file);
        assert_eq!(parsed.errors().len(), 1);
        assert_eq!(
            parsed.errors()[0].kind,
            ParseErrorKind::NestingLimitExceeded {
                limit: MAX_PARSE_DEPTH
            }
        );
        assert!(parsed.ast().is_none());
        assert_eq!(parsed.syntax().text().to_string(), over_limit);

        let at_delimiter_limit = format!(
            "function main() returns Int:\n  return {}1{}\nend function\n",
            "(".repeat(MAX_PARSE_DEPTH),
            ")".repeat(MAX_PARSE_DEPTH)
        );
        assert!(parse(&source(&at_delimiter_limit)).is_success());
        let over_delimiter_limit = format!(
            "function main() returns Int:\n  return {}1{}\nend function\n",
            "(".repeat(MAX_PARSE_DEPTH + 1),
            ")".repeat(MAX_PARSE_DEPTH + 1)
        );
        let parsed = parse(&source(&over_delimiter_limit));
        assert_eq!(parsed.errors().len(), 1);
        assert!(matches!(
            parsed.errors()[0].kind,
            ParseErrorKind::NestingLimitExceeded { .. }
        ));
        assert_eq!(parsed.syntax().text().to_string(), over_delimiter_limit);

        let unexpected = ")\n".repeat(MAX_PARSE_ERRORS + 50);
        let parsed = parse(&source(&unexpected));
        assert_eq!(parsed.errors().len(), MAX_PARSE_ERRORS);
        assert!(
            parsed
                .errors()
                .iter()
                .all(|error| error.kind
                    == ParseErrorKind::UnexpectedDelimiter(TokenKind::RightParen))
        );
    }

    fn nested_blocks(depth: usize) -> String {
        let mut text = String::from("function main() returns Int:\n");
        for _ in 1..depth {
            text.push_str("if true:\n");
        }
        text.push_str("return 1\n");
        for _ in 1..depth {
            text.push_str("end if\n");
        }
        text.push_str("end function\n");
        text
    }

    fn source(text: &str) -> SourceFile {
        SourceFile::from_text(SourceId::new(9_999), "limit-test.sico", text).unwrap()
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
