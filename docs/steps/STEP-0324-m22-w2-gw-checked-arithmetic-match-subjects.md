# STEP-0324: M22 W2 gw 机器接受 checked_add/checked_sub match 主语

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（完成计划 A1 首族；`lex_words` 入场区形状）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

general-while 机器的 in-body `match` 主语此前仅接受 `sico.map.get` 与
`sico.bytes.slice`；本步新增第三类主语 `I64/U64.checked_add`（后泛化至
`checked_sub`），操作数为 参数/环境 cell/字面量（含负数字面量经
`fixed_literal_operand_json`）。

- 新函数 `gw_checked_add_subject_packed`：镜像既有主语打包器合同
  （`value\ncount\ninstrs`），发射操作数指令 + `checked_add/checked_sub`
  指令（range=[receiver 起, 闭括号+1)，操作名透传源词）。
- `gw_match_open_pack`：主语检测（I64/U64 + `.` + add/sub）、选包器第三
  分支、ok_ty 按接收者映射 i64/u64（u64 时 match 临时 cell 沿用
  `result-u64` 惯例）。
- 测试 `sico_compiler_lowers_gw_checked_arithmetic_match_subjects_byte_exactly`：
  if 分支内 set 臂 checked_add（cell+literal）、checked_sub（cell+param+
  嵌套 if——matching_close 形状）、`matching_close` 冻结原文整函数；
  typed 拒绝（跨类参数 → CALL-TYPE、checked_mul → CALL-TARGET）+ 恢复全等。

## Measured results（本机真实 runner）

- 4 个隔离探针 + 整函数 `matching_close` 对 Rust oracle 字节全等；
  parser 逐函数覆盖 **11/115 → 12/115**（canary +1，R3 保持 0）；
  套件 **56/56** 绿（55 + 1 新测试；共享路径 map.get/slice in-body match
  无回归）。

## Machine facts（供后续切片复用）

- `parameter_type_word` 对含 `List[Text]`/`Map[...]` 参数的签名走错
  （把 `List` 当完整类型后找不到 `,` 返回空）——参数类型检查应使用
  `parameter_kind_by_id`（正确跳过泛型参数、返回裸 kind）。
- `operand_nonparam_kind` 对字面量接收词（`U64`）也返回非空——cell 与
  字面量的判别必须**先看边界**（单词=cell，多词=literal 形式）。

## Honest residuals

- `match` 直接位于 while 体首（无 if 包裹，探针 S2）仍 STATEMENT——
  循环体区域的 match 开闭受限，留待下一切片。
- `find_body_end`/`count_params`/`fn_entry`/`count_until_end_function`/
  `expression_shape_depth` 仍 CALL-TARGET（user-call 聚合 RHS 等其它形状）、
  `return_shapes` CALL-ARGUMENT、`json_string_token` EXPRESSION。
- M22 NO-GO；前缀前沿 `lex_words`/CALL-TARGET 不变。
