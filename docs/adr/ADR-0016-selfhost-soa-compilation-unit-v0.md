# ADR-0016: Self-host SOA frontend and compilation-unit boundary v0

> - status: accepted (owner directive “完成M22”, 2026-09-20), with amendment A1 below (2026-10-22)
> - date: 2026-09-20
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -
> - depends: RFC-0039, RFC-0046, ADR-0015, M22 plan

## Context

M22 already accepts struct-of-arrays (SOA) as the transition representation
while `List[record]` remains outside the executable language set.  The earlier
implementation nevertheless split lowering into a standalone `parser.sico`
program and kept three exact-source IR templates in `compiler.sico`.  It also
treated every checker input as a CLI entry, so the self-host path had no way to
distinguish an entry source from a legal imported `module` source.

The representation and compilation-unit boundary must be frozen before S3/S4
can converge without changing the language surface or weakening ADR-0015.

## Decision

1. M22 v0 uses SOA throughout its Sico-written token, declaration, semantic,
   lowering and codegen state.  Every logical row has parallel arrays with
   identical lengths; a mismatch, missing row, overflow or limit+1 input is a
   typed refusal.  No array lookup may silently substitute a semantic value.
2. `selfhost/parser.sico` is the single reusable lowering module.  A thin
   `parser_driver.sico` owns only the Script ABI used by the real-runner test.
   `compiler.sico` imports the same `lex_words` and `scalar_ir` functions; it
   must not retain exact-source IR templates as a fallback.
3. S2 remains a single-entry checker contract: a standalone source declaring
   `module` returns E8010.  L2 compilation receives a bounded, ordered source
   bundle with exactly one entry and zero or more imported modules.  Imported
   files must declare the matching module identity and pass RFC-0039 duplicate,
   cycle, traversal, depth and count gates.  Entry and module roles are data in
   the compilation unit, not inferred from source spelling.
4. Frozen source identity is computed from canonical UTF-8 LF bytes.  Host
   checkout CRLF materialization never changes a manifest digest.  Invalid
   UTF-8 and non-CRLF carriage returns fail closed.
5. The Rust parser, IR verifier and deterministic backend remain independent
   permanent oracles.  This ADR neither permits a Rust compile hostcall nor
   weakens ADR-0015 `A == B == C` Component-byte closure.
6. The canary order is fixed: the six existing scalar IR shapes, then
   `formatter.sico`, the complete self-host source bundle, the 37 accepted
   Script corpus entries, deterministic Component codegen, and finally the
   ADR-0015 bootstrap harness.  A typed refusal or trap at an earlier canary
   blocks later completion claims.

## Accepted amendments

- **A1 — 组件级 canary 次序按"先广度后尺度"重排（2026-10-22，owner 授权）**。决策 6 的
  canary 次序在**组件级**上细化为：

  `六个既有标量 IR 形状 → formatter.sico 组件级 → 37 条 accepted Script 语料的组件级差分 → 完整 self-host source bundle 组件级 → ADR-0015 闭合 harness`

  - **变更点**：原决策 6 把"完整 self-host source bundle"排在"37 条 accepted Script 语料"**之前**，
    并把"确定性 Component codegen"单列为一个晚于 bundle 的梯级。本修订把该梯级拆解为它自身的
    三个子梯级，并按**先广度、后尺度**排列。
  - **理由**：(a) `formatter.sico` 的 IR 面已字节全等（2026-10-20 出口审计记 canary 30/30
    CLOSED），因此它的组件级失配可**单独归因于 codegen**，是 codegen 正确性最干净的单点测试；
    (b) 37 条 accepted 语料以小体量覆盖广度，其脚本 ABI 组（stream / task / http / fs）是
    最大未知面，宜在尺度梯级之前处理；(c) 完整 bundle（≈1.02 MB）同时含**尺度**与**全表面**
    两个维度，是最贵梯级，应最后处理。
  - **阻断纪律不变**：早梯级的 typed refusal 或 trap 阻断后梯级的完成声明（原文保留）。
  - **本修订不改**：任何出口门、`A == B == C` 合同、证据等级或支持面主张。
  - **登记**：owner 于 2026-10-22 会话中授权该会话"全权拍板"，据此登记为 A1；执行记录见
    [STEP-0514](../steps/STEP-0514-m22-w2-s5-s6-closure-ladder.md)，阶梯全文见
    [M22 计划 §3.3](../plans/M22-compiler-self-host.md)。STEP-0486 §2.4 的原表述按
    [该 STEP 的对齐注记](../steps/STEP-0486-m22-w2-s5-codegen-wave.md)保留原句以便追溯。

## Consequences

SOA remains verbose but is now a deliberate bounded architecture rather than a
temporary convention.  The parser and compiler share one lowering path, so a
shape cannot pass a parser-only suite while the compiler silently uses a
template.  Adding `List[record]` later remains a separate RFC and is not an M22
prerequisite.

M22 remains NO-GO until S5 Component bytes, S6 bootstrap/package/budget
evidence, and S7 regression audit pass.  This decision closes an architecture
ambiguity; it does not itself close those gates.

## Validation

STEP-0246 validates canonical manifest regeneration, source SHA checks, the
module/driver split, the unified compiler lowering path, exact scalar IR
baselines and the absence of the former exact-source fallback.

## Links

- [`M22 plan`](../plans/M22-compiler-self-host.md)
- [`ADR-0015`](./ADR-0015-compiler-bootstrap-closure-v0.md)
- [`STEP-0246`](../steps/STEP-0246-m22-soa-convergence-baseline.md)

