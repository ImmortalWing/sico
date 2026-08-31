//! Lossless tokenization and lexical diagnostics following RFC-0006.

#![forbid(unsafe_code)]

use sico_source::{SourceFile, TextRange, TextSize};
use unicode_ident::{is_xid_continue, is_xid_start};
use unicode_normalization::UnicodeNormalization;

/// Maximum identifier token size in UTF-8 bytes.
pub const MAX_IDENTIFIER_BYTES: usize = 1024;
/// Maximum non-identifier token size in UTF-8 bytes.
pub const MAX_TOKEN_BYTES: usize = 1024 * 1024;
/// Maximum emitted tokens including trivia and excluding EOF.
pub const MAX_TOKENS: usize = 1_000_000;
/// Maximum emitted lexical diagnostics.
pub const MAX_LEX_ERRORS: usize = 100;

/// Lossless lexical token category.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u16)]
pub enum TokenKind {
    Whitespace,
    Newline,
    LineComment,
    Identifier,
    Underscore,
    Integer,
    String,
    True,
    False,
    Async,
    Await,
    Borrow,
    Call,
    Capabilities,
    Capability,
    Case,
    Effects,
    Else,
    End,
    Enum,
    ErrorKeyword,
    Export,
    From,
    Function,
    Group,
    If,
    Ignore,
    Interface,
    Invariant,
    Let,
    Match,
    Move,
    Newtype,
    None,
    OkKeyword,
    Record,
    Resource,
    Return,
    Returns,
    SelfKeyword,
    Spawn,
    Task,
    Try,
    Using,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,
    Dot,
    At,
    Plus,
    Minus,
    Equal,
    EqualEqual,
    LessEqual,
    Error,
    Eof,
}

impl TokenKind {
    /// Whether the parser may skip this token without losing source bytes.
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Newline | Self::LineComment)
    }
}

/// One token and its half-open UTF-8 byte range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub range: TextRange,
}

/// Typed lexical failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub range: TextRange,
}

/// Lexical failure reason. Stable E1xxx codes are assigned in STEP-0018.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LexErrorKind {
    IdentifierNotNfc,
    IdentifierTooLong,
    TokenTooLong,
    UnexpectedCharacter(char),
    UnterminatedString,
    UnknownStringEscape(char),
    TokenLimitExceeded,
}

/// Complete lossless lexer output.
#[derive(Clone, Debug)]
pub struct Lexed {
    tokens: Vec<Token>,
    errors: Vec<LexError>,
    errors_truncated: bool,
}

impl Lexed {
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    #[must_use]
    pub fn errors(&self) -> &[LexError] {
        &self.errors
    }

    #[must_use]
    pub const fn errors_truncated(&self) -> bool {
        self.errors_truncated
    }

    /// Reconstructs all original bytes for inputs that did not hit token limit.
    #[must_use]
    pub fn reconstruct(&self, source: &str) -> Option<String> {
        let mut result = String::with_capacity(source.len());
        let mut expected = 0;
        for token in self
            .tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Eof)
        {
            let start = usize::from(token.range.start());
            let end = usize::from(token.range.end());
            if start != expected || end > source.len() {
                return None;
            }
            result.push_str(&source[start..end]);
            expected = end;
        }
        (expected == source.len()).then_some(result)
    }
}

/// Tokenizes one validated source file.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn lex(source: &SourceFile) -> Lexed {
    let text = source.text();
    let mut cursor = 0;
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut errors_truncated = false;

    while cursor < text.len() {
        if tokens.len() >= MAX_TOKENS {
            push_error(
                &mut errors,
                &mut errors_truncated,
                LexErrorKind::TokenLimitExceeded,
                cursor,
                cursor,
            );
            break;
        }
        let start = cursor;
        let rest = &text[cursor..];
        let Some(first) = rest.chars().next() else {
            break;
        };

        let kind = if first == ' ' || first == '\t' {
            cursor += rest
                .bytes()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count();
            TokenKind::Whitespace
        } else if first == '\n' {
            cursor += 1;
            TokenKind::Newline
        } else if first == '\r' {
            cursor += 2;
            TokenKind::Newline
        } else if rest.starts_with("//") {
            cursor += rest.find(['\r', '\n']).unwrap_or(rest.len());
            TokenKind::LineComment
        } else if first == '_' || is_xid_start(first) {
            cursor += first.len_utf8();
            while cursor < text.len() {
                let Some(next) = text[cursor..].chars().next() else {
                    break;
                };
                if next == '_' || is_xid_continue(next) {
                    cursor += next.len_utf8();
                } else {
                    break;
                }
            }
            let word = &text[start..cursor];
            let kind = keyword(word);
            if kind == TokenKind::Identifier {
                if word.len() > MAX_IDENTIFIER_BYTES {
                    push_error(
                        &mut errors,
                        &mut errors_truncated,
                        LexErrorKind::IdentifierTooLong,
                        start,
                        cursor,
                    );
                }
                if !word.nfc().eq(word.chars()) {
                    push_error(
                        &mut errors,
                        &mut errors_truncated,
                        LexErrorKind::IdentifierNotNfc,
                        start,
                        cursor,
                    );
                }
            }
            kind
        } else if first.is_ascii_digit() {
            cursor += rest.bytes().take_while(u8::is_ascii_digit).count();
            TokenKind::Integer
        } else if first == '"' {
            cursor += 1;
            let mut terminated = false;
            while cursor < text.len() {
                let Some(next) = text[cursor..].chars().next() else {
                    break;
                };
                if next == '"' {
                    cursor += 1;
                    terminated = true;
                    break;
                }
                if matches!(next, '\r' | '\n') {
                    break;
                }
                if next == '\\' {
                    let escape_start = cursor;
                    cursor += 1;
                    if cursor == text.len() {
                        break;
                    }
                    let Some(escaped) = text[cursor..].chars().next() else {
                        break;
                    };
                    cursor += escaped.len_utf8();
                    if !matches!(escaped, '"' | '\\' | 'n' | 'r' | 't' | '0') {
                        push_error(
                            &mut errors,
                            &mut errors_truncated,
                            LexErrorKind::UnknownStringEscape(escaped),
                            escape_start,
                            cursor,
                        );
                    }
                } else {
                    cursor += next.len_utf8();
                }
            }
            if !terminated {
                push_error(
                    &mut errors,
                    &mut errors_truncated,
                    LexErrorKind::UnterminatedString,
                    start,
                    cursor,
                );
            }
            TokenKind::String
        } else {
            let (kind, width) = punctuation(rest).unwrap_or((TokenKind::Error, first.len_utf8()));
            cursor += width;
            if kind == TokenKind::Error {
                push_error(
                    &mut errors,
                    &mut errors_truncated,
                    LexErrorKind::UnexpectedCharacter(first),
                    start,
                    cursor,
                );
            }
            kind
        };

        if cursor - start > MAX_TOKEN_BYTES && kind != TokenKind::Identifier {
            push_error(
                &mut errors,
                &mut errors_truncated,
                LexErrorKind::TokenTooLong,
                start,
                cursor,
            );
        }
        tokens.push(Token {
            kind,
            range: range(start, cursor),
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        range: range(cursor, cursor),
    });
    Lexed {
        tokens,
        errors,
        errors_truncated,
    }
}

fn keyword(word: &str) -> TokenKind {
    match word {
        "_" => TokenKind::Underscore,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "async" => TokenKind::Async,
        "await" => TokenKind::Await,
        "borrow" => TokenKind::Borrow,
        "call" => TokenKind::Call,
        "capabilities" => TokenKind::Capabilities,
        "capability" => TokenKind::Capability,
        "case" => TokenKind::Case,
        "effects" => TokenKind::Effects,
        "else" => TokenKind::Else,
        "end" => TokenKind::End,
        "enum" => TokenKind::Enum,
        "error" => TokenKind::ErrorKeyword,
        "export" => TokenKind::Export,
        "from" => TokenKind::From,
        "function" => TokenKind::Function,
        "group" => TokenKind::Group,
        "if" => TokenKind::If,
        "ignore" => TokenKind::Ignore,
        "interface" => TokenKind::Interface,
        "invariant" => TokenKind::Invariant,
        "let" => TokenKind::Let,
        "match" => TokenKind::Match,
        "move" => TokenKind::Move,
        "newtype" => TokenKind::Newtype,
        "none" => TokenKind::None,
        "ok" => TokenKind::OkKeyword,
        "record" => TokenKind::Record,
        "resource" => TokenKind::Resource,
        "return" => TokenKind::Return,
        "returns" => TokenKind::Returns,
        "self" => TokenKind::SelfKeyword,
        "spawn" => TokenKind::Spawn,
        "task" => TokenKind::Task,
        "try" => TokenKind::Try,
        "using" => TokenKind::Using,
        _ => TokenKind::Identifier,
    }
}

fn punctuation(rest: &str) -> Option<(TokenKind, usize)> {
    let pair = rest.get(..2);
    if pair == Some("==") {
        return Some((TokenKind::EqualEqual, 2));
    }
    if pair == Some("<=") {
        return Some((TokenKind::LessEqual, 2));
    }
    Some(match *rest.as_bytes().first()? {
        b'(' => (TokenKind::LeftParen, 1),
        b')' => (TokenKind::RightParen, 1),
        b'[' => (TokenKind::LeftBracket, 1),
        b']' => (TokenKind::RightBracket, 1),
        b':' => (TokenKind::Colon, 1),
        b',' => (TokenKind::Comma, 1),
        b'.' => (TokenKind::Dot, 1),
        b'@' => (TokenKind::At, 1),
        b'+' => (TokenKind::Plus, 1),
        b'-' => (TokenKind::Minus, 1),
        b'=' => (TokenKind::Equal, 1),
        _ => return None,
    })
}

fn push_error(
    errors: &mut Vec<LexError>,
    truncated: &mut bool,
    kind: LexErrorKind,
    start: usize,
    end: usize,
) {
    if errors.len() < MAX_LEX_ERRORS {
        errors.push(LexError {
            kind,
            range: range(start, end),
        });
    } else {
        *truncated = true;
    }
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange::new(size(start), size(end))
}

fn size(value: usize) -> TextSize {
    TextSize::from(u32::try_from(value).expect("validated source fits u32"))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::Value;
    use sico_source::{LineCol, SourceErrorKind, SourceId};

    use super::*;

    fn source(text: &str) -> SourceFile {
        SourceFile::from_text(SourceId::new(1), "test.sico", text).unwrap()
    }

    #[test]
    fn token_golden_covers_rfc_families_and_maximal_munch() {
        let text = "async function _name(a: Int) returns Result[Int, Error]: @ + - = == <= 001 \"A\\n\\\"B\\\\\" true false _\nend function";
        let file = source(text);
        let lexed = lex(&file);
        assert!(lexed.errors().is_empty(), "{:?}", lexed.errors());
        let significant: Vec<_> = lexed
            .tokens()
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .map(|token| token.kind)
            .collect();
        assert_eq!(
            significant,
            vec![
                TokenKind::Async,
                TokenKind::Function,
                TokenKind::Identifier,
                TokenKind::LeftParen,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::RightParen,
                TokenKind::Returns,
                TokenKind::Identifier,
                TokenKind::LeftBracket,
                TokenKind::Identifier,
                TokenKind::Comma,
                TokenKind::Identifier,
                TokenKind::RightBracket,
                TokenKind::Colon,
                TokenKind::At,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Equal,
                TokenKind::EqualEqual,
                TokenKind::LessEqual,
                TokenKind::Integer,
                TokenKind::String,
                TokenKind::True,
                TokenKind::False,
                TokenKind::Underscore,
                TokenKind::End,
                TokenKind::Function,
                TokenKind::Eof,
            ]
        );
        assert_eq!(lexed.reconstruct(file.text()).as_deref(), Some(text));
    }

    #[test]
    fn unicode_identifiers_require_xid_and_nfc() {
        let accepted = source("计算 café _结果2");
        let lexed = lex(&accepted);
        assert!(lexed.errors().is_empty());
        assert_eq!(
            lexed
                .tokens()
                .iter()
                .filter(|token| token.kind == TokenKind::Identifier)
                .count(),
            3
        );

        let decomposed = source("cafe\u{301}");
        assert_eq!(
            lex(&decomposed).errors()[0].kind,
            LexErrorKind::IdentifierNotNfc
        );
        let emoji = source("😀name");
        assert!(matches!(
            lex(&emoji).errors()[0].kind,
            LexErrorKind::UnexpectedCharacter('😀')
        ));
    }

    #[test]
    fn invalid_strings_punctuation_and_trivia_make_local_errors() {
        let cases = [
            ("\"unterminated", LexErrorKind::UnterminatedString),
            ("\"bad\\u{41}\"", LexErrorKind::UnknownStringEscape('u')),
            ("name： Text", LexErrorKind::UnexpectedCharacter('：')),
            ("a\u{00A0}b", LexErrorKind::UnexpectedCharacter('\u{00A0}')),
            ("/* comment */", LexErrorKind::UnexpectedCharacter('/')),
        ];
        for (text, expected) in cases {
            let file = source(text);
            let lexed = lex(&file);
            assert_eq!(lexed.errors()[0].kind, expected, "{text}");
            assert_eq!(lexed.reconstruct(file.text()).as_deref(), Some(text));
        }
    }

    #[test]
    fn identifier_and_token_limits_report_without_losing_bytes() {
        let at_identifier_limit = "a".repeat(MAX_IDENTIFIER_BYTES);
        assert!(lex(&source(&at_identifier_limit)).errors().is_empty());
        let identifier = "a".repeat(MAX_IDENTIFIER_BYTES + 1);
        let file = source(&identifier);
        let lexed = lex(&file);
        assert_eq!(lexed.errors()[0].kind, LexErrorKind::IdentifierTooLong);
        assert_eq!(
            lexed.reconstruct(file.text()).as_deref(),
            Some(identifier.as_str())
        );

        let comment = format!("//{}", "a".repeat(MAX_TOKEN_BYTES - 2));
        let file = source(&comment);
        let lexed = lex(&file);
        assert!(lexed.errors().is_empty());
        assert_eq!(usize::from(lexed.tokens()[0].range.len()), MAX_TOKEN_BYTES);

        let too_long = format!("{comment}a");
        assert_eq!(
            SourceFile::from_text(SourceId::new(1), "too-long-token", too_long)
                .unwrap_err()
                .kind,
            SourceErrorKind::LineTooLong
        );
    }

    #[test]
    fn token_and_diagnostic_count_limits_are_exact_and_bounded() {
        let at_limit = "a\n".repeat(MAX_TOKENS / 2);
        let lexed = lex(&source(&at_limit));
        assert_eq!(lexed.tokens().len(), MAX_TOKENS + 1);
        assert!(lexed.errors().is_empty());
        assert_eq!(
            lexed.reconstruct(&at_limit).as_deref(),
            Some(at_limit.as_str())
        );

        let over_limit = format!("{at_limit}a\n");
        let lexed = lex(&source(&over_limit));
        assert_eq!(lexed.tokens().len(), MAX_TOKENS + 1);
        assert_eq!(lexed.errors()[0].kind, LexErrorKind::TokenLimitExceeded);
        assert!(lexed.reconstruct(&over_limit).is_none());

        let errors = "!".repeat(MAX_LEX_ERRORS + 1);
        let lexed = lex(&source(&errors));
        assert_eq!(lexed.errors().len(), MAX_LEX_ERRORS);
        assert!(lexed.errors_truncated());
        assert_eq!(lexed.reconstruct(&errors).as_deref(), Some(errors.as_str()));
    }

    #[test]
    fn contract_json_cases_execute_against_source_and_lexer() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/lexical/contract-v0.json");
        let contract: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(contract["positive"].as_array().unwrap().len(), 7);
        assert_eq!(contract["negative"].as_array().unwrap().len(), 14);

        for case in contract["positive"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            if id == "LEX-P007" {
                continue;
            }
            let text = case["source"].as_str().unwrap();
            let file = source(text);
            let lexed = lex(&file);
            assert!(lexed.errors().is_empty(), "{id}: {:?}", lexed.errors());
            assert_eq!(
                lexed.reconstruct(file.text()).as_deref(),
                Some(text),
                "{id}"
            );
            if id == "LEX-P006" {
                for point in case["points"].as_array().unwrap() {
                    let byte = u32::try_from(point["byte"].as_u64().unwrap()).unwrap();
                    let expected = LineCol {
                        line: u32::try_from(point["line"].as_u64().unwrap()).unwrap(),
                        column: u32::try_from(point["column"].as_u64().unwrap()).unwrap(),
                    };
                    assert_eq!(
                        file.line_index()
                            .line_col(file.text(), TextSize::from(byte)),
                        Some(expected)
                    );
                }
            }
        }

        for case in contract["negative"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            match id {
                "LEX-N001" | "LEX-N002" => {
                    let bytes: Vec<u8> = case["bytes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|value| u8::try_from(value.as_u64().unwrap()).unwrap())
                        .collect();
                    assert!(SourceFile::from_bytes(SourceId::new(1), id, &bytes).is_err());
                }
                "LEX-N004" | "LEX-N006" | "LEX-N012" => {
                    assert!(
                        SourceFile::from_text(
                            SourceId::new(1),
                            id,
                            case["source"].as_str().unwrap()
                        )
                        .is_err()
                    );
                }
                "LEX-N013" | "LEX-N014" => {}
                _ => {
                    let file = source(case["source"].as_str().unwrap());
                    assert!(!lex(&file).errors().is_empty(), "{id}");
                }
            }
        }
        assert_eq!(
            contract["limits"]["source_bytes"],
            sico_source::MAX_SOURCE_BYTES
        );
        assert_eq!(
            contract["limits"]["line_bytes"],
            sico_source::MAX_LINE_BYTES
        );
        assert_eq!(contract["limits"]["identifier_bytes"], MAX_IDENTIFIER_BYTES);
        assert_eq!(contract["limits"]["token_bytes"], MAX_TOKEN_BYTES);
        assert_eq!(contract["limits"]["tokens"], MAX_TOKENS);
        assert_eq!(contract["limits"]["diagnostics"], MAX_LEX_ERRORS);
    }

    #[test]
    fn all_b_canonical_files_are_lossless_and_lexically_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../syntax-candidates/b");
        let mut paths = Vec::new();
        collect_sico(&root, &mut paths);
        paths.sort();
        assert_eq!(paths.len(), 58);
        let mut token_count = 0;
        for (id, path) in paths.iter().enumerate() {
            let bytes = fs::read(path).unwrap();
            let file = SourceFile::from_bytes(
                SourceId::new(u32::try_from(id).unwrap()),
                path.display().to_string(),
                &bytes,
            )
            .unwrap();
            let lexed = lex(&file);
            assert!(
                lexed.errors().is_empty(),
                "{}: {:?}",
                path.display(),
                lexed.errors()
            );
            assert_eq!(lexed.reconstruct(file.text()).as_deref(), Some(file.text()));
            token_count += lexed.tokens().len() - 1;
        }
        assert!(
            token_count > 2_668,
            "trivia-inclusive token count must exceed old metric"
        );
    }

    #[test]
    fn source_failures_remain_distinct_from_lexical_failures() {
        let invalid = SourceFile::from_text(SourceId::new(0), "bad", "a\rb").unwrap_err();
        assert_eq!(invalid.kind, SourceErrorKind::BareCarriageReturn);
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
