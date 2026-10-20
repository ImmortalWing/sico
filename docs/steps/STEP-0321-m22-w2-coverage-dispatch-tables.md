# STEP-0321: M22 W2 覆盖扩展——parser 分发表 + compiler_parser 破零

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（逐函数差分覆盖扩展；Flash 普查产物的首轮抽样验证）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

按普查（「M22 剩余函数形状普查」产物）提名的无循环目标探针翻绿：

- parser.sico：`scalar_type_kind`、`fixed_op_name`（无循环分发表，带
  `is_word`/`word_at`/`next_index`/`line_end` 被调前缀，探针与正式测试同文本）。
- compiler_parser.sico **破零**：`same`（字节比较委托）、`add`/`sub`
  （match 主语 checked 算术）standalone 字节全等——新测试
  `sico_compiler_lowers_compiler_parser_function_subsets_byte_exactly`。

## Measured results（本机真实 runner）

- parser 逐函数覆盖 **9/115 → 11/115**；compiler_parser **0/32 → 3/32**；
  canary +5，R3 保持 0；套件 55/55 绿。
- 同轮探针确认仍拒：`has_return_at`/`lossless_tokens`/`append_metadata` →
  EXPRESSION（return 位/let-RHS 的本地调用与 concat 流水形状）；
  `constant_argument_kind`/`expression_shape_depth` → CALL-TARGET。
  普查结论"3 个纯直线函数可直接翻"被部分修正：仅 `same` 直线翻绿，
  `lossless_tokens`（return 位本地调用）与 `append_metadata`（let+set
  concat 流水）仍拒——普查为只读判断，本探针为准。

## Honest residuals

- 前缀前沿 `lex_words`/CALL-TARGET 不变；M22 NO-GO；S5/S6/S7 缺口不变。
