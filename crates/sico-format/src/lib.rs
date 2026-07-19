//! Canonical formatting for successfully parsed RFC-0005 B source.

#![forbid(unsafe_code)]

use sico_lexer::{Token, TokenKind, lex};
use sico_parser::parse;
use sico_source::SourceFile;

/// Formatting is deliberately unavailable for recovered/error trees.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatError {
    Lexical { count: usize },
    Syntax { count: usize },
}

#[derive(Clone, Copy, Debug)]
struct BlockState {
    close: TokenKind,
    match_arm_active: bool,
}

/// Produces the single canonical layout for valid B source.
///
/// Identifier, literal, keyword, and comment token text is never rewritten.
/// The formatter owns whitespace, line endings, indentation, and blank lines.
///
/// # Errors
///
/// Returns [`FormatError::Lexical`] or [`FormatError::Syntax`] instead of
/// emitting partial output when the source is not a successful parse.
pub fn format(source: &SourceFile) -> Result<String, FormatError> {
    let parsed = parse(source);
    if !parsed.lex_errors().is_empty() {
        return Err(FormatError::Lexical {
            count: parsed.lex_errors().len(),
        });
    }
    if !parsed.errors().is_empty() {
        return Err(FormatError::Syntax {
            count: parsed.errors().len(),
        });
    }

    let lexed = lex(source);
    let mut blocks = Vec::<BlockState>::new();
    let mut section_active = false;
    let mut lines = Vec::<String>::new();
    let mut current = Vec::<Token>::new();

    for token in lexed.tokens().iter().copied() {
        match token.kind {
            TokenKind::Newline => {
                emit_line(
                    source.text(),
                    &current,
                    &mut blocks,
                    &mut section_active,
                    &mut lines,
                );
                current.clear();
            }
            TokenKind::Eof => {}
            _ => current.push(token),
        }
    }
    if !current.is_empty() {
        emit_line(
            source.text(),
            &current,
            &mut blocks,
            &mut section_active,
            &mut lines,
        );
    }

    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn emit_line(
    source: &str,
    tokens: &[Token],
    blocks: &mut Vec<BlockState>,
    section_active: &mut bool,
    lines: &mut Vec<String>,
) {
    let content: Vec<_> = tokens
        .iter()
        .copied()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .collect();
    if content.is_empty() {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        return;
    }

    let comment_index = content
        .iter()
        .position(|token| token.kind == TokenKind::LineComment);
    let code = &content[..comment_index.unwrap_or(content.len())];
    let first = code.first().map(|token| token.kind);

    if first == Some(TokenKind::End) {
        let close = code.get(1).map(|token| token.kind);
        if blocks
            .last()
            .is_some_and(|block| Some(block.close) == close)
        {
            blocks.pop();
        }
        *section_active = false;
    }

    if first == Some(TokenKind::Case)
        && let Some(block) = blocks
            .iter_mut()
            .rev()
            .find(|block| block.close == TokenKind::Match)
    {
        block.match_arm_active = false;
    }

    let list_item = *section_active && first == Some(TokenKind::Identifier);
    if matches!(first, Some(TokenKind::Effects | TokenKind::Capabilities))
        || (first.is_some() && !list_item)
    {
        *section_active = false;
    }

    let match_arm_levels = blocks.iter().filter(|block| block.match_arm_active).count();
    let indent = blocks.len() + match_arm_levels + usize::from(list_item);
    let mut output = "  ".repeat(indent);
    output.push_str(&format_code(source, code));

    if let Some(index) = comment_index {
        let comment = token_text(source, content[index]).trim_end();
        if !code.is_empty() {
            output.push_str("  ");
        }
        output.push_str(comment);
    }
    lines.push(output);

    if first == Some(TokenKind::Case)
        && code
            .last()
            .is_some_and(|token| token.kind == TokenKind::Colon)
        && let Some(block) = blocks
            .iter_mut()
            .rev()
            .find(|block| block.close == TokenKind::Match)
    {
        block.match_arm_active = true;
    }

    if matches!(first, Some(TokenKind::Effects | TokenKind::Capabilities))
        && code.len() == 2
        && code
            .last()
            .is_some_and(|token| token.kind == TokenKind::Colon)
    {
        *section_active = true;
    }

    if let Some(close) = opener_close(code) {
        blocks.push(BlockState {
            close,
            match_arm_active: false,
        });
    }
}

fn opener_close(code: &[Token]) -> Option<TokenKind> {
    if code.last()?.kind != TokenKind::Colon {
        return None;
    }
    let first = code.first()?.kind;
    let direct = match first {
        TokenKind::Record => Some(TokenKind::Record),
        TokenKind::Enum => Some(TokenKind::Enum),
        TokenKind::Capability => Some(TokenKind::Capability),
        TokenKind::Resource => Some(TokenKind::Resource),
        TokenKind::Interface => Some(TokenKind::Interface),
        TokenKind::Match => Some(TokenKind::Match),
        TokenKind::If => Some(TokenKind::If),
        TokenKind::Using => Some(TokenKind::Using),
        TokenKind::Task => Some(TokenKind::Task),
        _ => None,
    };
    direct.or_else(|| {
        code.iter()
            .any(|token| token.kind == TokenKind::Function)
            .then_some(TokenKind::Function)
    })
}

fn format_code(source: &str, code: &[Token]) -> String {
    let mut output = String::new();
    for (index, token) in code.iter().copied().enumerate() {
        if index > 0 && needs_space(code[index - 1].kind, token.kind) {
            output.push(' ');
        }
        output.push_str(token_text(source, token));
    }
    output
}

fn needs_space(previous: TokenKind, current: TokenKind) -> bool {
    if matches!(
        current,
        TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::Comma
            | TokenKind::Dot
            | TokenKind::Colon
            | TokenKind::At
    ) || matches!(
        previous,
        TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot | TokenKind::At
    ) {
        return false;
    }
    if matches!(current, TokenKind::LeftParen | TokenKind::LeftBracket)
        && matches!(
            previous,
            TokenKind::Identifier
                | TokenKind::OkKeyword
                | TokenKind::ErrorKeyword
                | TokenKind::RightParen
                | TokenKind::RightBracket
        )
    {
        return false;
    }
    true
}

fn token_text(source: &str, token: Token) -> &str {
    &source[usize::from(token.range.start())..usize::from(token.range.end())]
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use sico_source::SourceId;

    use super::*;

    fn source(text: &str) -> SourceFile {
        SourceFile::from_text(SourceId::new(1), "test.sico", text).unwrap()
    }

    #[test]
    fn policy_normalizes_layout_comments_and_nested_indentation() {
        let input = "\r\nfunction  main ( )  returns  Int : // entry  \r\n\teffects :\r\n x.y\r\n\tmatch ( x ) :\r\ncase  Value :\r\nreturn  ok ( x +  1 ) // result\r\nend match\r\nend function\r\n\r\n";
        let expected = include_str!("../../../tests/formatter/policy.snap");
        let formatted = format(&source(input)).unwrap();
        assert_eq!(formatted, expected);
        assert_eq!(format(&source(&formatted)).unwrap(), formatted);
    }

    #[test]
    fn all_b_cases_preserve_ast_shape_and_are_idempotent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-candidates/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 54);
        for (index, path) in paths.iter().enumerate() {
            let original = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            let original_shape = parse(&original).ast().unwrap().shape();
            let formatted = format(&original).unwrap();
            let canonical = SourceFile::from_text(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                formatted.clone(),
            )
            .unwrap();
            assert_eq!(parse(&canonical).ast().unwrap().shape(), original_shape);
            assert_eq!(format(&canonical).unwrap(), formatted, "{}", path.display());
        }
    }

    #[test]
    fn all_b_mutations_are_rejected_without_output() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-mutations/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 12);
        for (index, path) in paths.iter().enumerate() {
            let invalid = SourceFile::from_bytes(
                SourceId::new(u32::try_from(index).unwrap()),
                path.display().to_string(),
                &fs::read(path).unwrap(),
            )
            .unwrap();
            assert_eq!(format(&invalid), Err(FormatError::Syntax { count: 1 }));
        }
    }

    #[test]
    fn rejected_top_level_execution_is_never_formatted() {
        assert_eq!(
            format(&source("stdout.write(stdin.read_all())\n")),
            Err(FormatError::Syntax { count: 1 })
        );
        assert_eq!(
            format(&source("script:\n  return input.stdin\nend script\n")),
            Err(FormatError::Syntax { count: 1 })
        );
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
