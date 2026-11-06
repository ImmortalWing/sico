# STEP-0348 — M22 W2 S3/S4: gw after 区 match/if 放行，json_string_token 翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

gw 机器放行 **after 区（while 关闭后的尾段块）的 match 与 if 语句**——删除
两处保守拒绝门：

1. `gw_match_open_pack` 的 `region == "after"` 硬门（原 10759；entry/header
   门保留）。
2. `general_while_function_ir` if 处理器的 `region == "after"` 硬门（原 11300）。

**非对称证据链**（本片判定依据）：块装配完全 region 无关——case 臂处理沿用
当前 region 字符串、end-match 只封 join 块（11612 段无 region 检查）、end-if
恢复到 "join"（join 区对 if/match 本就放行）、收尾装配接受
after/entry/join + cur_term（12015 段）。while 处理器此前已放行 after
（11209），if/match 的拒绝是历史保守而非结构约束。删门不新增 cell
（match 门在 gw_match_open_pack 自身函数内，if 门是纯删分支），
`general_while_function_ir` 保持 248/256。

**翻绿（parser 53→54/121，+1，R3 0）**：`json_string_token`（尾段
match + if 包裹 match + concat/decode 实参，0346/0347 三片铺路的收官函数，
22763 字节全等）。census fixpoint 无额外免费翻转（47→50 函数验证前缀，
1 轮收敛；lex_words 需更多被调与自身形状）。

## 2. 前沿更新（census r1 实测，remaining 67）

- `lex_words`：CALL-TARGET → **CALL-SHAPE**（json_string_token 被调已解，
  自身词法循环新形状）。
- json 构造器类 CALL-ARGUMENT 不变：summarize、fixed_operation_instruction_json、
  read_local_instruction_json、call_instruction_json、intrinsic_instruction_json、
  record_named_type。
- **RETURN-COUNT**：fixed_operation_operand_atom_idx、gw_instr_count。
- **MATCH-SHAPE**：u64_list_at。
- **CALL-TARGET**：gw_decimal_value（缺被调 gw_instr_count，RETURN-COUNT 类）。
- M22 整体 NO-GO 不变（S5/S6/S7 未动）。

## 3. 验证与预算

- `--test selfhost_compiler`：**60/60 绿**（新增 STEP-0348 测试：A1 after 区
  match、A2 after 区 if/else、A3 if+match 组合（json 尾段形状）、
  json_string_token 逐字冻结文本，均 next_index 单被调前缀）。
- `selfhost_local_bounds` 绿（gw 248/256 不变）。
- 完整 runner 工作区套件全绿（35 个测试目标 ok，0 失败）；`cargo fmt --check` 绿；
  `validate-module-boundaries.ps1` OK；`git diff --check` 绿。
- `selfhost_probe_tmp.rs` 按纪律删除。
