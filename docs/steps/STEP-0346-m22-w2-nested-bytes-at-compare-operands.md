# STEP-0346 — M22 W2 S3/S4: 链式比较条件接受嵌套 bytes.at 操作数（双操作数 + 参数源分发），5 函数翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

`chain_condition_packed`（gw 机器链式比较条件路径）两处操作数扩展：

1. **左操作数参数源分发**：`sico.bytes.at(...)` 左值的第一个参数此前只接受
   cell（`while_bytes_at_local_src_packed` 硬门 CALL-ARGUMENT@6741）。现按语句区
   同款规则分发——arg1 是 bytes **参数**时走 `while_bytes_at_rhs_packed`
   （pack 布局 kind/value/count/json，与 local_src 的 count/value/kind/json 不同，
   分支解析）。bisect 全部 CALL-ARGUMENT 前沿函数（has_return_at /
   parameter_name_start / find_text_from / 合成 M1）同门 6741 汇聚。
2. **右操作数嵌套调用**：右值此前只有 param / cell / 字面量（单指令）三形态。
   新增嵌套 `sico.bytes.at` 分支（arg1 参数/局部双分发同左），闭合索引改由
   嵌套调用的 `matching_close` 推导（原 `fixed_operation_operand_width` 路径保留
   于非嵌套分支）。`find_text_from` 的
   `I64.equal(at(src,i,0), at(needle,j,0))` 双嵌套操作数一次通过。

免费翻转（fixpoint census，47 函数验证前缀、2 轮收敛）：`return_expression_start`
（被调 has_return_at 翻绿后 CALL-TARGET 类即解）、`local_binding_index`。

**翻绿（parser 48→53/121，+5，R3 0）**：has_return_at、parameter_name_start、
find_text_from（bisect 三函数）+ return_expression_start、local_binding_index（census）。

## 2. 前沿更新（census fixpoint 后的拒绝类地图）

- **CALL-TARGET**：`json_string_token`（自身 intrinsic 白名单缺口，候选
  utf8_decode/slice/encode，待 bisect）；`gw_decimal_value`（被调 gw_instr_count
  未翻）；`lex_words`（被调 json_string_token 未翻）。
- **CALL-ARGUMENT（非 ccp 位点）**：`summarize`、`fixed_operation_instruction_json`、
  `read_local_instruction_json`、`call_instruction_json`、`intrinsic_instruction_json`、
  `record_named_type`（json 构造器类，多为 straight 机器位点）。
- **EXPRESSION**：`word_span_offset`、`argument_instruction_count`、`parameters_ir`
  （let/RHS 一般调用表达式）；合成 M4（return 位双调用）、M5（straight let-RHS
  bytes.at）同类。
- **RETURN-COUNT**：`fixed_operation_operand_atom_idx`、`gw_instr_count`。
- **MATCH-SHAPE**：`u64_list_at`；合成 M0（match slice ok 臂 return 调用）同类。
- **STATEMENT**：gw concat RHS（0344 packer 只接了 sef/scc；合成 M3 证实 gw 缺位）。
- **gw_* 自举层（~20）+ 机器函数 + scalar_ir**：census 前缀缺被调（oracle
  UNRESOLVED_CALL_TARGET），按叶子逐层解锁，非独立缺口。
- census ORACLE-REFUSE ≠ 机器缺口：组合前缀缺被调者的预期表现，逐轮吸收。
- M22 整体 NO-GO 不变（S5/S6/S7 未动）。

## 3. 验证与预算

- `--test selfhost_compiler`：**58/58 绿**（新增 STEP-0346 块：4 函数被调前缀
  累积链 ×5 函数 + M1 最小回归卫兵）。
- `selfhost_local_bounds` 绿：chain_condition_packed 新增绑定未触及最忙函数，
  `general_while_function_ir` 保持 248/256，地板 8 不变。
- 完整 runner 工作区套件全绿（35 个测试目标 ok，0 失败）；`cargo fmt --check` 绿；
  `validate-module-boundaries.ps1` OK；`git diff --check` 绿。
- 探针方法：paren 锚定提取器 + awk 行号标记（CALL-ARGUMENT 全部汇聚 6741，
  单门单刀）；`selfhost_probe_tmp.rs` 按纪律删除。
