# STEP-0320: M22 W2 parser 覆盖扩展——line_end 翻绿

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（`parser.sico` 逐函数差分覆盖扩展）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

继 STEP-0319 的逐函数覆盖口径，把 `line_end`（while + if/return + user-call
消费者）翻绿：其形状由 formatter 轮次建立的 while/if 机器覆盖，被调函数
`is_word`/`word_at`/`next_index`（已翻绿）同文本重排前置。
`selfhost_compiler.rs` 的重排子集测试增加 `line_end` 用例并抽出
`parser_function_text` 辅助。

## Measured results（本机真实 runner）

- parser 每函数覆盖 **8/115 → 9/115**（+1，canary 增长，R3 保持 0）。
- 同轮探针（临时测试，已删除）确认 line_end 下游仍拒：
  `matching_close`/`find_body_end`/`count_params`/`fn_entry`/
  `count_until_end_function`/`expression_shape_depth` →
  `ERR:E-SH-IR-CALL-TARGET`；`return_shapes` → `CALL-ARGUMENT`；
  `json_string_token` → `EXPRESSION`。共性缺口 = **user-call RHS /
  参数位 user call 形状**，即下一切片目标。
- `selfhost_compiler` 套件 54/54 绿。

## Honest residuals

- 前缀前沿 `lex_words`/CALL-TARGET 不变；M22 NO-GO；S5/S6/S7 缺口不变。
- 下游函数需要的不止 user-call RHS：`json_string_token` 另含
  `text.encode`/`bytes.at` 类 let-RHS 宿主调用（EXPRESSION 缺口）。
