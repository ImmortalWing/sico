# STEP-0319: M22 W2 parser 逐函数差分覆盖（重排子集）

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（`parser.sico` 语料差分覆盖扩展；owner 指令 2026-09-30「完成M22，可用子代理」执行线）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective

canary 度量从前缀（连续函数序列）扩展为逐函数（冻结函数原文的重排子集差分）：
现有族已能降低的 parser.sico 函数立即翻绿计入，无需等待 `lex_words` 整体降低。

## Changes

- `selfhost_compiler.rs` 新增
  `sico_compiler_lowers_parser_rearranged_function_subsets_byte_exactly`：
  取 `PARSER_SOURCE` 中该函数的精确原文（`function <name>` 起至
  `end function` 含换行），guest 与 Rust oracle 对同一文本逐字节对比。
  首批三个函数：
  - `next_index` — 返回臂 `checked_add` match 族；
  - `word_at` — `list.get` 返回 match 族（参数索引）；
  - `valid_escape_byte` — control-if 守卫链族（六个 `I64.equal` 守卫返回）。
- 度量口径登记：parser 覆盖计数 = 逐函数差分通过数（重排子集、原文文本、
  前置被调函数同文本重排）；连续前缀前沿另行记录且不受本步影响。

## Measured results（本机真实 runner）

- parser 每函数覆盖 **5/115 → 8/115**（+3）。
- canary 增长（当前阶段 = 冻结 selfhost 源集的逐函数字节差分进度）→
  **R3 连续停滞计数归零**（STEP-0318 的 1/4 撤销依据：计数为"连续"，非累计）。
- `selfhost_compiler` 套件 54/54 绿（53 + 1 新）。
- 同轮探针（临时测试，已删除）确认后续切片输入：
  `is_digit_token`/`is_string_token` 拒
  `ERR:E-SH-IR-EXPRESSION`（缺 `text.encode`/`bytes.at`/`text.length`
  的 let-RHS 形状）；`line_end`/`matching_close` 拒
  `ERR:E-SH-IR-CALL-TARGET`（缺 user-call RHS 形状）。

## Honest residuals

- 连续前缀前沿未动：`lex_words`/CALL-TARGET（本步不触碰 lex_words）。
- 重排子集依赖被调函数同在子集内且自身可降低：`line_end` 依赖自身未降低
  的形状，故 `matching_close` 等含 `line_end` 的子集暂不可翻。
- M22 维持 NO-GO；S5/S6/S7 缺口不变；独立 CI 待 GitHub 可达。
