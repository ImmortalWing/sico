# STEP-0327: M22 W2 parser 覆盖 +5 与 bytes.at cursor 值修复

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（完成计划 A 队列；R3 关键步）
> - completed: 2026-10-01
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

- 批量普查探针（16 个未试函数）发现 5 个直接翻绿；`decimal_at_most` 的翻绿
  暴露并修复 `while_bytes_at_local_src_packed` 的 cursor 实参 id 错误
  （cell-cursor 路径的 `sico.bytes.at` 第三实参误用 src read 的 id——
  探针字节差 `[25,25,27]` vs `[25,26,27]` 精确钉死）。
- 同包器 cursor 放宽：u64 cell 之外接受 u64 参数（无 read 指令、值 id 直用
  参数号；count 随之 3/4 变化）。字面量 cursor 曾实现并在探针中复现字节错，
  按纪律收窄为 typed 拒绝（CALL-ARGUMENT），留待独立字节验证后放开。
- 翻绿函数（冻结原文 + 4 被调前置）：`binding_parameter_index`、
  `parameter_type_word`、`script_result_return_json`、`decimal_at_most`、
  `return_expression_end`。

## Measured results（本机真实 runner）

- 5 函数对 Rust oracle 字节全等；`selfhost_compiler` 套件 **58/58** 绿。
- parser 逐函数覆盖 **12/115 → 17/115**（+5），canary 增长 →
  **R3 连续停滞计数归零**（自 STEP-0325 的 2/4 清零）。

## Honest residuals

- control_if 机器（无 while 函数）的宿主调用支持两轮实现后按纪律回滚
  （拒绝面前移但无字节验证用例；M3 类守卫操作数打包已验证可达，残余
  EXPRESSION 在首个 return 之后的路径，含 form=="ret" 对普通字面量终返的
  拒绝门）——设计与 comma-skip 修复要点的完整记录在案，is_digit_token 三
  函数仍为下一切片目标。
- 其余拒绝不变：function_declaration_* / declared_return_kind /
  binding_parameter_kind / argument_values → CALL-TARGET（user-call RHS）；
  word_span_offset / argument_instruction_count / fixed_literal_argument /
  is_canonical_magnitude → EXPRESSION。
- M22 NO-GO；前缀前沿 `lex_words`/CALL-TARGET 不变。
