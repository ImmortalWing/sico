# STEP-0221: M22 modular compiler frontend integration

> - status: complete (compiler frontend integration; lowering/codegen remain partial)
> - phase: M22 S3 to S4 handoff
> - completed: 2026-09-17
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/compiler.sico`, `selfhost/compiler_lexer.sico`, `selfhost/compiler_parser.sico`, `runner/sico-runner/tests/selfhost_compiler.rs`, `tools/validate-step-0221.ps1`

## 1. Result

The self-host compiler is no longer a single source file with an isolated
punctuation-free word probe. Its Script entry now links two Sico modules:

- `compiler_lexer.sico` owns the lossless token stream;
- `compiler_parser.sico` consumes that lexer stream line by line and emits the
  canonical declaration AST metadata used by the compiler entry;
- `compiler.sico` exposes bounded `--emit-tokens` and `--emit-ast-metadata`
  frontend modes while retaining the verified IR and codegen seams.

The Rust-built compiler Component is executed through the real runner. On the
three compiler sources themselves:

- lexer kind, UTF-8 byte spans and raw token bytes match the Rust lexer exactly;
- parser declaration kind/name/range/detail match public Rust `ModuleAst`
  metadata exactly;
- the existing six canonical-IR acceptances, same-length mutation refusal and
  fixed Core Wasm equality remain green.

This is the first evidence that the general lexer/parser path is linked into
the actual compiler Component rather than demonstrated only by standalone
tools.

## 2. Honest boundary

The integrated parser currently emits top-level declaration metadata. It does
not yet expose expression/statement nodes to the lowering module, so the
compiler's canonical IR and codegen support remain bounded seams within this
STEP. This STEP does not claim any of the 37 STEP-0220 Script artifacts are
guest-produced, nor A=B=C, packaging or budget closure.
