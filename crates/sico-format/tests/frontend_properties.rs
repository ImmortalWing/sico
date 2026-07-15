use sico_format::format;
use sico_lexer::lex;
use sico_parser::{MAX_PARSE_ERRORS, parse};
use sico_source::{SourceFile, SourceId};

#[test]
fn deterministic_arbitrary_bytes_never_escape_source_contract() {
    let mut random = XorShift64::new(0x5eed_a11c_e5ee_d123);
    for case in 0..4_096_u32 {
        let length = usize::try_from(random.next() % 512).unwrap();
        let bytes: Vec<_> = (0..length)
            .map(|_| random.next().to_le_bytes()[0])
            .collect();
        if let Ok(source) = SourceFile::from_bytes(SourceId::new(case), "fuzz-bytes", &bytes) {
            exercise_validated_source(&source);
        }
    }
}

#[test]
fn deterministic_valid_utf8_and_delimiters_are_bounded_and_repeatable() {
    const ATOMS: &[&str] = &[
        "a", "value", "计算", "café", " ", "\t", "\n", "\r\n", "(", ")", "[", "]", ":", ",", ".",
        "+", "-", "=", "<=", "// note", "\"text\"", "function", "end", "if", "match", "case",
        "return",
    ];
    let mut random = XorShift64::new(0x51c0_f022_0021);
    for case in 0..4_096_u32 {
        let count = usize::try_from(random.next() % 128).unwrap();
        let mut text = String::new();
        for _ in 0..count {
            let index = usize::try_from(random.next()).unwrap() % ATOMS.len();
            text.push_str(ATOMS[index]);
        }
        let source = SourceFile::from_text(SourceId::new(case), "fuzz-utf8", text).unwrap();
        exercise_validated_source(&source);
    }

    let source = SourceFile::from_text(
        SourceId::new(9_000),
        "many-delimiters",
        ")\n".repeat(MAX_PARSE_ERRORS * 20),
    )
    .unwrap();
    assert_eq!(parse(&source).errors().len(), MAX_PARSE_ERRORS);
}

fn exercise_validated_source(source: &SourceFile) {
    let lexed = lex(source);
    assert!(lexed.errors().len() <= sico_lexer::MAX_LEX_ERRORS);
    let parsed = parse(source);
    assert!(parsed.errors().len() <= MAX_PARSE_ERRORS);
    assert_eq!(parsed.syntax().text().to_string(), source.text());
    if parsed.is_success() {
        let formatted = format(source).unwrap();
        let canonical =
            SourceFile::from_text(SourceId::new(10_000), "canonical", formatted.clone()).unwrap();
        assert!(parse(&canonical).is_success());
        assert_eq!(format(&canonical).unwrap(), formatted);
    }
}

struct XorShift64(u64);

impl XorShift64 {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}
