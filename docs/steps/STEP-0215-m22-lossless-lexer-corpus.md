# STEP-0215: M22 lossless Sico lexer over the frozen corpus

> - status: complete
> - phase: M22 S3 lexer closure
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/tokens.sico`, `runner/sico-runner/tests/selfhost_tokens.rs`, `tools/validate-step-0215.ps1`

## 1. Result

The punctuation-free `List[Text]` word probe is replaced by a lossless lexer
implemented in Sico. Each token is emitted as a length-delimited frame carrying
kind, UTF-8 byte start/end, byte length and exact raw token bytes. The surface
covers trivia, comments, strings/escapes, integers, every frozen keyword,
maximal-munch `==`/`<=`, punctuation, lexical error tokens and EOF.

The permanent Rust lexer oracle and the real runner compare every token of all
215 STEP-0214 sources. The result is 215/215 files green: kind, span and raw
bytes match, and concatenating non-EOF guest token bytes reconstructs each
source byte-exactly.

## 2. Bounded execution shape

The frozen corpus includes a 49,212-byte/1,314-line solver. Accumulating every
framed token through the current copy-on-write `List.append`/`Bytes.concat`
helpers exhausts the fixed 64 MiB guest arena. The differential therefore feeds
newline-terminated lexical segments and adjusts spans by the segment base.
RFC-0006 tokens cannot cross a physical newline (strings terminate before it;
line comments terminate at it; CRLF stays in one segment), so this preserves
the exact full-file token sequence while bounding each guest invocation. The
largest frozen physical line is 278 bytes.

This is not hidden as whole-file compiler readiness: the following AST/parser
slice must consume segments into bounded struct-of-arrays state and must not
recreate an unbounded framed token list.

## 3. Boundary

This STEP closes the lexer row of the STEP-0213 recovery sequence. It does not
claim a real AST, formatter/checker parity, general typed IR/codegen, A=B=C or
M22 GO.
