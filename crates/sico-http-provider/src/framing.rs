//! Strict HTTP/1.1 transfer framing and streaming body budgets (M12
//! STEP-0114, RFC-0037 §7). Every parser has byte/item bounds; ambiguous
//! or malformed framing fails closed with typed errors.

/// Default per-direction body budget (RFC-0037 §7).
pub const DEFAULT_BODY_BUDGET: u64 = 64 * 1024 * 1024;
/// Host ceiling per direction.
pub const MAX_BODY_BUDGET: u64 = 1024 * 1024 * 1024;
/// Maximum chunk size accepted by the chunked parser.
pub const MAX_CHUNK_BYTES: usize = 64 * 1024;
/// Bound on header lines parsed from a chunked trailer section.
pub const MAX_TRAILER_LINES: usize = 64;

/// Typed framing failures (RFC-0037 §1 `protocol` class).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramingError {
    /// Content-Length present more than once with different values.
    ConflictingLength,
    /// Content-Length and `Transfer-Encoding: chunked` together.
    MixedFraming,
    /// Chunk size line malformed or oversized.
    BadChunkSize,
    /// Trailer section exceeded bounds or was malformed.
    BadTrailer,
    /// Stream ended before the declared body was complete.
    PrematureEof,
    /// A declared body exceeded the run/direction budget.
    BudgetExceeded,
}

impl core::fmt::Display for FramingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::ConflictingLength => "conflicting Content-Length values",
            Self::MixedFraming => "Content-Length and Transfer-Encoding both present",
            Self::BadChunkSize => "malformed or oversized chunk size",
            Self::BadTrailer => "malformed or oversized trailer section",
            Self::PrematureEof => "connection closed before body completed",
            Self::BudgetExceeded => "body exceeded its budget",
        };
        f.write_str(text)
    }
}

impl std::error::Error for FramingError {}

/// How the response body is framed, decided once from strict headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFraming {
    /// Fixed number of bytes.
    Length(u64),
    /// Chunked transfer coding.
    Chunked,
    /// Until connection close (bounded by the response budget).
    UntilClose,
    /// No body (204/304 or HEAD semantics).
    Empty,
}

/// Decides body framing from strict header views. `content_lengths` is
/// the list of raw Content-Length values (duplicates included);
/// `transfer_encoding` is the raw TE header value if present.
///
/// # Errors
///
/// [`FramingError::ConflictingLength`] for disagreeing duplicates,
/// [`FramingError::MixedFraming`] when both framing headers exist, and
/// [`FramingError::BudgetExceeded`] when the declared length exceeds
/// `budget`.
pub fn decide_framing(
    content_lengths: &[&str],
    transfer_encoding: Option<&str>,
    budget: u64,
) -> Result<BodyFraming, FramingError> {
    if let Some(te) = transfer_encoding {
        if te.trim().eq_ignore_ascii_case("chunked") {
            if !content_lengths.is_empty() {
                return Err(FramingError::MixedFraming);
            }
            return Ok(BodyFraming::Chunked);
        }
        return Err(FramingError::MixedFraming);
    }
    match content_lengths {
        [] => Ok(BodyFraming::UntilClose),
        [single] => {
            let text = single.trim();
            if text.is_empty()
                || !text.bytes().all(|b| b.is_ascii_digit())
                || (text.len() > 1 && text.starts_with('0'))
            {
                return Err(FramingError::ConflictingLength);
            }
            let length: u64 = text.parse().map_err(|_| FramingError::ConflictingLength)?;
            if length > budget {
                return Err(FramingError::BudgetExceeded);
            }
            Ok(BodyFraming::Length(length))
        }
        values => {
            let first = values[0].trim();
            if values.iter().any(|v| v.trim() != first) {
                return Err(FramingError::ConflictingLength);
            }
            Err(FramingError::ConflictingLength)
        }
    }
}

/// A strict incremental chunked-body reader: feed it socket bytes, get
/// payload bytes out. Bounds are checked before allocation.
#[derive(Debug)]
pub struct ChunkedReader {
    remaining_chunk: u64,
    finished: bool,
    seen_final_zero: bool,
}

impl ChunkedReader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            remaining_chunk: 0,
            finished: false,
            seen_final_zero: false,
        }
    }

    /// Parses one chunk-size line (without CRLF). Hex only, bounded,
    /// extensions after `;` are refused (RFC-0037: strict v1).
    ///
    /// # Errors
    ///
    /// [`FramingError::BadChunkSize`] on malformed or oversized sizes.
    pub fn chunk_size_line(&mut self, line: &str) -> Result<(), FramingError> {
        if self.finished {
            return Err(FramingError::BadTrailer);
        }
        if line.contains(';') {
            return Err(FramingError::BadChunkSize);
        }
        let size_text = line.trim();
        if size_text.is_empty()
            || size_text.len() > 8
            || !size_text.bytes().all(|b| b.is_ascii_hexdigit())
            || (size_text.len() > 1 && size_text.starts_with('0'))
        {
            return Err(FramingError::BadChunkSize);
        }
        let size = u64::from_str_radix(size_text, 16).map_err(|_| FramingError::BadChunkSize)?;
        if size > MAX_CHUNK_BYTES as u64 {
            return Err(FramingError::BadChunkSize);
        }
        if size == 0 {
            self.seen_final_zero = true;
            self.finished = true;
            return Ok(());
        }
        self.remaining_chunk = size;
        Ok(())
    }

    /// Consumes up to `input` bytes of chunk payload; returns how many
    /// payload bytes `input` contributes. `budget_consumed` is the
    /// cumulative payload so far and `budget` the direction budget.
    ///
    /// # Errors
    ///
    /// [`FramingError::BudgetExceeded`] when the chunk would overflow.
    pub fn chunk_data(
        &mut self,
        input: &[u8],
        budget_consumed: u64,
        budget: u64,
    ) -> Result<usize, FramingError> {
        if self.seen_final_zero {
            return Err(FramingError::BadTrailer);
        }
        let take = usize::try_from(self.remaining_chunk)
            .unwrap_or(usize::MAX)
            .min(input.len());
        if budget_consumed + take as u64 > budget {
            return Err(FramingError::BudgetExceeded);
        }
        self.remaining_chunk -= take as u64;
        Ok(take)
    }

    #[must_use]
    pub fn expecting_data(&self) -> bool {
        self.remaining_chunk > 0
    }

    /// Payload bytes of the current chunk that have not been consumed yet
    /// (0 unless [`Self::expecting_data`]).
    #[must_use]
    pub fn pending_chunk_bytes(&self) -> u64 {
        self.remaining_chunk
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        self.finished
    }

    #[must_use]
    pub fn trailer_expected(&self) -> bool {
        self.seen_final_zero
    }
}

impl Default for ChunkedReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates a trailer section: bounded line count, each line a bounded
/// `name: value` pair, no obs-folds.
///
/// # Errors
///
/// [`FramingError::BadTrailer`] on any violation.
pub fn validate_trailer(lines: &[&str]) -> Result<(), FramingError> {
    if lines.len() > MAX_TRAILER_LINES {
        return Err(FramingError::BadTrailer);
    }
    for line in lines {
        if line.len() > 2048 || line.starts_with(' ') || line.starts_with('\t') {
            return Err(FramingError::BadTrailer);
        }
        if line.bytes().any(|b| b < 0x20 || b == 0x7f) {
            return Err(FramingError::BadTrailer);
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(FramingError::BadTrailer);
        };
        if name.is_empty() || name.trim() != name || value.len() > 1024 {
            return Err(FramingError::BadTrailer);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_chunked_and_close_decide_cleanly() {
        assert_eq!(
            decide_framing(&["1024"], None, DEFAULT_BODY_BUDGET),
            Ok(BodyFraming::Length(1024))
        );
        assert_eq!(
            decide_framing(&[], Some("chunked"), DEFAULT_BODY_BUDGET),
            Ok(BodyFraming::Chunked)
        );
        assert_eq!(
            decide_framing(&[], None, DEFAULT_BODY_BUDGET),
            Ok(BodyFraming::UntilClose)
        );
        assert_eq!(
            decide_framing(&["0"], None, DEFAULT_BODY_BUDGET),
            Ok(BodyFraming::Length(0))
        );
    }

    #[test]
    fn smuggling_and_budget_cases_fail_closed() {
        assert_eq!(
            decide_framing(&["1024", "2048"], None, DEFAULT_BODY_BUDGET),
            Err(FramingError::ConflictingLength)
        );
        assert_eq!(
            decide_framing(&["1024", "1024"], None, DEFAULT_BODY_BUDGET),
            Err(FramingError::ConflictingLength)
        );
        assert_eq!(
            decide_framing(&["1024"], Some("chunked"), DEFAULT_BODY_BUDGET),
            Err(FramingError::MixedFraming)
        );
        assert_eq!(
            decide_framing(&["+1024"], None, DEFAULT_BODY_BUDGET),
            Err(FramingError::ConflictingLength)
        );
        assert_eq!(
            decide_framing(&["01024"], None, DEFAULT_BODY_BUDGET),
            Err(FramingError::ConflictingLength)
        );
        assert_eq!(
            decide_framing(
                &[&format!("{}", DEFAULT_BODY_BUDGET + 1)],
                None,
                DEFAULT_BODY_BUDGET
            ),
            Err(FramingError::BudgetExceeded)
        );
    }

    #[test]
    fn chunk_reader_accepts_a_canonical_sequence() {
        let mut reader = ChunkedReader::new();
        reader.chunk_size_line("4").unwrap();
        assert!(reader.expecting_data());
        let taken = reader.chunk_data(b"pong!", 0, DEFAULT_BODY_BUDGET).unwrap();
        assert_eq!(taken, 4);
        assert!(!reader.expecting_data());
        reader.chunk_size_line("0").unwrap();
        assert!(reader.finished());
        assert!(reader.trailer_expected());
        assert_eq!(validate_trailer(&["x-check: abc"]), Ok(()));
    }

    #[test]
    fn chunk_reader_refuses_ambiguity_and_overflow() {
        let mut reader = ChunkedReader::new();
        assert_eq!(
            reader.chunk_size_line("0x4"),
            Err(FramingError::BadChunkSize)
        );
        assert_eq!(
            reader.chunk_size_line("0010"),
            Err(FramingError::BadChunkSize)
        );
        assert_eq!(
            reader.chunk_size_line(&format!("{}", MAX_CHUNK_BYTES + 1)),
            Err(FramingError::BadChunkSize)
        );
        // Extensions refused (strict v1).
        assert_eq!(
            reader.chunk_size_line("4;ext=1"),
            Err(FramingError::BadChunkSize)
        );

        let mut reader = ChunkedReader::new();
        reader.chunk_size_line("10").unwrap();
        // Budget consumed nearly full: the chunk must overflow the gate.
        assert_eq!(
            reader.chunk_data(
                b"0123456789abcdef",
                DEFAULT_BODY_BUDGET,
                DEFAULT_BODY_BUDGET
            ),
            Err(FramingError::BudgetExceeded)
        );
        // Within budget it is accepted and advances the chunk.
        assert_eq!(reader.chunk_data(b"01234", 0, DEFAULT_BODY_BUDGET), Ok(5));
        // Data after final-zero is trailer abuse.
        let mut reader = ChunkedReader::new();
        reader.chunk_size_line("0").unwrap();
        assert_eq!(
            reader.chunk_data(b"z", 0, DEFAULT_BODY_BUDGET),
            Err(FramingError::BadTrailer)
        );
        assert_eq!(reader.chunk_size_line("4"), Err(FramingError::BadTrailer));
    }

    #[test]
    fn trailer_section_bounded() {
        assert_eq!(validate_trailer(&[]), Ok(()));
        let many: Vec<String> = (0..=MAX_TRAILER_LINES)
            .map(|i| format!("x-{i}: v"))
            .collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        assert_eq!(validate_trailer(&refs), Err(FramingError::BadTrailer));
        assert_eq!(
            validate_trailer(&["folded: yes\n more"]),
            Err(FramingError::BadTrailer)
        );
        assert_eq!(
            validate_trailer(&["no-colon"]),
            Err(FramingError::BadTrailer)
        );
        assert_eq!(
            validate_trailer(&[" space-name: v"]),
            Err(FramingError::BadTrailer)
        );
    }
}
