//! Source text, file identity, byte ranges, and line indexing.

#![forbid(unsafe_code)]

pub use text_size::{TextRange, TextSize};

/// Maximum accepted source size in UTF-8 bytes.
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum accepted physical line size, including its newline bytes.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

/// Opaque identity assigned to a source in one compiler session.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates a session-local source identity.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the session-local integer representation.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A byte range associated with one source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    /// Source containing the range.
    pub source: SourceId,
    /// Half-open UTF-8 byte range.
    pub range: TextRange,
}

/// Human-facing 1-based Unicode scalar coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineCol {
    /// 1-based line.
    pub line: u32,
    /// 1-based Unicode scalar column.
    pub column: u32,
}

/// Source validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceError {
    /// Stable typed reason. Numeric diagnostic codes are assigned later.
    pub kind: SourceErrorKind,
    /// Authoritative range in the original byte input.
    pub range: TextRange,
}

/// Source validation reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceErrorKind {
    /// File exceeds [`MAX_SOURCE_BYTES`].
    SourceTooLarge,
    /// Bytes are not strict UTF-8.
    InvalidUtf8,
    /// A UTF-8 BOM occurs at byte zero.
    ByteOrderMark,
    /// A forbidden control or default-ignorable scalar occurs in source.
    ForbiddenCharacter(char),
    /// CR was not followed by LF.
    BareCarriageReturn,
    /// A physical line exceeds [`MAX_LINE_BYTES`].
    LineTooLong,
}

/// Validated Sico source and its shared line index.
#[derive(Clone, Debug)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    line_index: LineIndex,
}

impl SourceFile {
    /// Validates original bytes according to RFC-0006.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic encoding, control, newline, or size
    /// failure in byte order.
    pub fn from_bytes(
        id: SourceId,
        name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, SourceError> {
        validate_source_bytes(bytes)?;
        let text = String::from_utf8(bytes.to_vec()).map_err(|invalid| {
            let start = invalid.utf8_error().valid_up_to();
            error(SourceErrorKind::InvalidUtf8, start, start.saturating_add(1))
        })?;
        let line_index = LineIndex::new(&text);
        Ok(Self {
            id,
            name: name.into(),
            text,
            line_index,
        })
    }

    /// Validates an already decoded string according to RFC-0006.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic control, newline, or size failure in
    /// byte order.
    pub fn from_text(
        id: SourceId,
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Self, SourceError> {
        let text = text.into();
        Self::from_bytes(id, name, text.as_bytes())
    }

    /// Session-local source identity.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Display name supplied by the caller.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Valid UTF-8 source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Source length in UTF-8 bytes.
    #[must_use]
    pub fn len(&self) -> TextSize {
        text_size(self.text.len())
    }

    /// Whether this source is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Shared line index.
    #[must_use]
    pub const fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    /// Creates a source span after validating range ordering and boundaries.
    #[must_use]
    pub fn span(&self, range: TextRange) -> Option<Span> {
        let start = usize::from(range.start());
        let end = usize::from(range.end());
        (end <= self.text.len()
            && start <= end
            && self.text.is_char_boundary(start)
            && self.text.is_char_boundary(end))
        .then_some(Span {
            source: self.id,
            range,
        })
    }
}

/// Index of UTF-8 byte offsets for line starts.
#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<TextSize>,
    len: TextSize,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut starts = vec![TextSize::from(0)];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(text_size(index + 1));
            }
        }
        Self {
            starts,
            len: text_size(text.len()),
        }
    }

    /// Number of logical lines. Empty source has one line.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        u32::try_from(self.starts.len()).unwrap_or(u32::MAX)
    }

    /// Returns the 1-based coordinate for a valid scalar boundary.
    #[must_use]
    pub fn line_col(&self, text: &str, offset: TextSize) -> Option<LineCol> {
        let offset = usize::from(offset);
        if offset > usize::from(self.len) || !text.is_char_boundary(offset) {
            return None;
        }
        let bytes = text.as_bytes();
        if offset > 0
            && offset < bytes.len()
            && bytes[offset - 1] == b'\r'
            && bytes[offset] == b'\n'
        {
            return None;
        }
        let line_index = self
            .starts
            .partition_point(|start| usize::from(*start) <= offset)
            .saturating_sub(1);
        let start = usize::from(self.starts[line_index]);
        let column = text[start..offset].chars().count() + 1;
        Some(LineCol {
            line: u32::try_from(line_index + 1).ok()?,
            column: u32::try_from(column).ok()?,
        })
    }

    /// Converts a 1-based scalar coordinate back to a UTF-8 byte offset.
    #[must_use]
    pub fn offset(&self, text: &str, position: LineCol) -> Option<TextSize> {
        let line = usize::try_from(position.line.checked_sub(1)?).ok()?;
        let target_column = usize::try_from(position.column.checked_sub(1)?).ok()?;
        let start = usize::from(*self.starts.get(line)?);
        let mut end = self
            .starts
            .get(line + 1)
            .map_or(text.len(), |offset| usize::from(*offset));
        if end > start && text.as_bytes()[end - 1] == b'\n' {
            end -= 1;
            if end > start && text.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
        }
        if target_column == text[start..end].chars().count() {
            return Some(text_size(end));
        }
        text[start..end]
            .char_indices()
            .nth(target_column)
            .map(|(relative, _)| text_size(start + relative))
    }
}

fn validate_source_bytes(bytes: &[u8]) -> Result<(), SourceError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(error(
            SourceErrorKind::SourceTooLarge,
            MAX_SOURCE_BYTES,
            bytes.len(),
        ));
    }
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(error(SourceErrorKind::ByteOrderMark, 0, 3));
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(invalid) => {
            let start = invalid.valid_up_to();
            let end = start + invalid.error_len().unwrap_or(1).min(bytes.len() - start);
            return Err(error(SourceErrorKind::InvalidUtf8, start, end));
        }
    };

    let mut line_start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        let end = index + ch.len_utf8();
        if ch == '\r' {
            if chars.peek().is_none_or(|(_, next)| *next != '\n') {
                return Err(error(SourceErrorKind::BareCarriageReturn, index, end));
            }
        } else if is_forbidden_source_character(ch) {
            return Err(error(SourceErrorKind::ForbiddenCharacter(ch), index, end));
        }
        if ch == '\n' {
            if end - line_start > MAX_LINE_BYTES {
                return Err(error(SourceErrorKind::LineTooLong, line_start, end));
            }
            line_start = end;
        }
    }
    if text.len() - line_start > MAX_LINE_BYTES {
        return Err(error(SourceErrorKind::LineTooLong, line_start, text.len()));
    }
    Ok(())
}

fn is_forbidden_source_character(ch: char) -> bool {
    matches!(ch, '\0'..='\u{0008}' | '\u{000B}'..='\u{000C}' | '\u{000E}'..='\u{001F}' | '\u{007F}')
        || is_default_ignorable(ch)
}

fn is_default_ignorable(ch: char) -> bool {
    matches!(
        ch,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}'
    )
}

fn error(kind: SourceErrorKind, start: usize, end: usize) -> SourceError {
    SourceError {
        kind,
        range: TextRange::new(text_size(start), text_size(end)),
    }
}

fn text_size(value: usize) -> TextSize {
    TextSize::from(u32::try_from(value).expect("RFC-0006 source limit fits u32"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(text: &str) -> SourceFile {
        SourceFile::from_text(SourceId::new(1), "test.sico", text).unwrap()
    }

    #[test]
    fn strict_utf8_bom_and_controls_are_rejected() {
        let invalid =
            SourceFile::from_bytes(SourceId::new(0), "bad", &[b'f', 0xFF, b'o']).unwrap_err();
        assert_eq!(invalid.kind, SourceErrorKind::InvalidUtf8);
        assert_eq!(usize::from(invalid.range.start()), 1);
        assert_eq!(
            SourceFile::from_bytes(SourceId::new(0), "bom", &[0xEF, 0xBB, 0xBF, b'f'])
                .unwrap_err()
                .kind,
            SourceErrorKind::ByteOrderMark
        );
        assert!(matches!(
            SourceFile::from_text(SourceId::new(0), "nul", "a\0b")
                .unwrap_err()
                .kind,
            SourceErrorKind::ForbiddenCharacter('\0')
        ));
        assert!(matches!(
            SourceFile::from_text(SourceId::new(0), "invisible", "a\u{200B}b")
                .unwrap_err()
                .kind,
            SourceErrorKind::ForbiddenCharacter('\u{200B}')
        ));
    }

    #[test]
    fn bare_cr_is_rejected_and_crlf_is_one_line_break() {
        assert_eq!(
            SourceFile::from_text(SourceId::new(0), "bad", "a\rb")
                .unwrap_err()
                .kind,
            SourceErrorKind::BareCarriageReturn
        );
        let file = source("a\r\nb");
        assert_eq!(file.line_index().line_count(), 2);
        assert_eq!(
            file.line_index().line_col(file.text(), TextSize::from(3)),
            Some(LineCol { line: 2, column: 1 })
        );
        assert_eq!(
            file.line_index().line_col(file.text(), TextSize::from(2)),
            None
        );
    }

    #[test]
    fn utf8_byte_and_scalar_coordinates_roundtrip() {
        let file = source("// 😀\n中");
        let expected = [(0, 1, 1), (3, 1, 4), (7, 1, 5), (8, 2, 1), (11, 2, 2)];
        for (byte, line, column) in expected {
            let position = LineCol { line, column };
            assert_eq!(
                file.line_index()
                    .line_col(file.text(), TextSize::from(byte)),
                Some(position)
            );
            assert_eq!(
                file.line_index().offset(file.text(), position),
                Some(TextSize::from(byte))
            );
        }
        assert_eq!(
            file.line_index().line_col(file.text(), TextSize::from(4)),
            None
        );
    }

    #[test]
    fn every_scalar_boundary_roundtrips_across_newline_forms() {
        let file = source("α😀\r\n中\nx");
        for (byte, _) in file
            .text()
            .char_indices()
            .chain([(file.text().len(), '\0')])
        {
            if byte > 0
                && byte < file.text().len()
                && file.text().as_bytes()[byte - 1] == b'\r'
                && file.text().as_bytes()[byte] == b'\n'
            {
                continue;
            }
            let offset = text_size(byte);
            let position = file.line_index().line_col(file.text(), offset).unwrap();
            assert_eq!(
                file.line_index().offset(file.text(), position),
                Some(offset)
            );
        }
    }

    #[test]
    fn source_and_line_limits_are_enforced_at_limit_plus_one() {
        let line = vec![b'a'; MAX_LINE_BYTES];
        assert!(SourceFile::from_bytes(SourceId::new(0), "line-at-limit", &line).is_ok());
        let too_large = vec![b'a'; MAX_SOURCE_BYTES + 1];
        assert_eq!(
            SourceFile::from_bytes(SourceId::new(0), "large", &too_large)
                .unwrap_err()
                .kind,
            SourceErrorKind::SourceTooLarge
        );
        let long_line = vec![b'a'; MAX_LINE_BYTES + 1];
        assert_eq!(
            SourceFile::from_bytes(SourceId::new(0), "line", &long_line)
                .unwrap_err()
                .kind,
            SourceErrorKind::LineTooLong
        );

        let physical_line = format!("{}\n", "a".repeat(1023));
        let at_source_limit = physical_line.repeat(MAX_SOURCE_BYTES / physical_line.len());
        assert_eq!(at_source_limit.len(), MAX_SOURCE_BYTES);
        assert!(
            SourceFile::from_text(SourceId::new(0), "source-at-limit", at_source_limit).is_ok()
        );
    }

    #[test]
    fn spans_require_in_bounds_scalar_boundaries() {
        let file = source("中");
        assert!(
            file.span(TextRange::new(TextSize::from(0), TextSize::from(3)))
                .is_some()
        );
        assert!(
            file.span(TextRange::new(TextSize::from(1), TextSize::from(3)))
                .is_none()
        );
        assert!(
            file.span(TextRange::new(TextSize::from(0), TextSize::from(4)))
                .is_none()
        );
    }
}
