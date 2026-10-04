# STEP-0345 — M22 W2 S3/S4: gw 条件宿主左操作数（chain 复用），5 函数翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

`gw_condition_packed` 的 I64/U64 接收者分支改为**先走 `chain_condition_packed`**
（STEP-0338 落地、字节已证的宿主左操作数路径：`sico.bytes.at` /
`sico.text.length` / `sico.bytes.length` 左值 + param/cell/字面量右值），
`SKIP:` 回落 `gw_compare_packed`（链上非宿主形状本就委托 gw_compare，无环）。
此前 gw 机器的 if/while 条件对宿主调用左操作数一律拒 EXPRESSION
（`gw_compare_packed` 左操作数 fallthrough 的 `operand_nonparam_kind` 空值门，
四个前沿函数同点位阵亡）。

**翻绿（parser 43→48/121，canary +5，R3 0）**：
- `is_canonical_magnitude`（text.encode 顶层 let + bytes.at 比较 ×5 + while，line_end
  四函数前缀字节全等）；
- 普查连锁（40 函数前缀，组合冻结逐字复刻）：pass1 `constant_argument_kind`、
  `fixed_literal_argument`、`comma_count`，pass2 `fixed_operation_operand_width`
  （需 pass1 三函数作被调）。剩余 77。

## 2. 前沿更新

- `find_text_from`/`parameter_name_start` EXPRESSION→**CALL-ARGUMENT**、
  `json_string_token` →CALL-ARGUMENT、`has_return_at` →CALL-ARGUMENT：宿主条件已通，
  剩嵌套调用形状（各自独立片）。
- 语料教训复刻：census 组合必须逐字复刻（15 函数自制组合缺 `decimal_at_most` 被调，
  oracle UNRESOLVED_CALL_TARGET 即刻暴露）。
- R3 0/4；M22 整体 NO-GO 不变。

## 3. 验证与预算

- `--test selfhost_compiler`：**58/58 绿**（is_canonical_magnitude 条目 + census40 前缀
  三条目 + census44 条目）。
- 完整 runner 工作区套件全绿（35 个测试目标 ok）；`cargo fmt` 绿；local bounds 绿
  （`gwc_chain`/`gwc_chain2` 为 gw_condition_packed 内新 cell，该函数非最忙函数，
  `general_while_function_ir` 保持 248/256，地板 8 不变）。
- `selfhost_probe_tmp.rs` 按纪律删除。
