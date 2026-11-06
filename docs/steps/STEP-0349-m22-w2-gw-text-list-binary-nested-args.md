# STEP-0349 — M22 W2 S3/S4: gw text.join/list.append 多 token 实参 + 参数跨度终点修正，lex_words 翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

`gw_text_list_binary_packed`（`sico.text.join` / `sico.list.append` 共用打包器）
三处修复：

1. **多 token 第二参**：arity 门原要求第二参单 token（`next_index(text_idx)==
   close`），`list.append(words, sico.bytes.utf8_decode(...))` 嵌套调用实参一律
   CALL-SHAPE@8444。放宽为"第二参存在且范围内**顶层无逗号**"（括号深度扫描，
   保住三参拒绝语义——既有 corpus 的
   `append(output, value, value)`/`join(items, "\n", "x")` CALL-SHAPE 期望不受
   影响，首版简单放宽曾破两测试，深度扫描版恢复）。
2. **参数跨度终点修正**：两参原先把自身起点当 range end 传给 `gw_rhs_packed`
   （`line_end_offset` 参数），单 token 原子自算跨度无恙，但嵌套调用发出零宽
   range（guest `end:3012` vs oracle `end:3048`）。现按"参数最后一个 token 的
   偏移+宽度"计算（arg1 至逗号、arg2 至闭括号各一扫描游标），与 STEP-0347
   slice 参数已证模式一致。
3. lex_words 其余形状已由前几片覆盖：List[Text] append 字面量实参（既有）、
   match ok 臂 append 嵌套调用（本片）、`return sico.text.split_lines("...")`
   调用返回（既有 return 位调用）、深层 if/else 嵌套（既有）。

**翻绿（parser 54→55/121，+1，R3 0）**：`lex_words`（模块词法器，55766 字节
全等）。census fixpoint 无额外免费翻转（51 函数验证前缀，1 轮收敛）。

## 2. 前沿更新

- **entry 区 match 保留门**：函数首语句 match 仍拒 STATEMENT
  （gw_match_open_pack entry 门，0348 有意保留）；合成 L3 证实该缺口独立于
  lex_words。属 T2 之外的 CFG 边角，暂记不扩。
- straight 机器 `return sico.text.split_lines` 已通（L2 字节等）——sef/scc
  return 位调用族比先前 I_* 探针所示更宽（此前 I_utf8_decode return 已通）。
- 剩余类不变：json 构造器类 6 函数（CALL-ARGUMENT）、RETURN-COUNT 两函数、
  u64_list_at MATCH-SHAPE、gw_decimal_value（缺被调）、gw_* 自举层、T2。
- M22 整体 NO-GO 不变（S5/S6/S7 未动）。

## 3. 验证与预算

- `--test selfhost_compiler`：**61/61 绿**（新增 STEP-0349 测试：L1 字面量
  append、L2 return 位 split_lines、L3 match 臂嵌套调用 append、lex_words
  逐字冻结；两处既有 corpus 的三参 CALL-SHAPE 拒绝回归验证保留）。
- `selfhost_local_bounds` 绿（`gw_text_list_binary_packed` 新增约 14 cell
  ——双扫描游标对 + 深度扫描三元组，仍非最忙函数）。
- 完整 runner 工作区套件全绿（35 个测试目标 ok，0 失败）；`cargo fmt --check` 绿；
  `validate-module-boundaries.ps1` OK；`git diff --check` 绿。
- 探针方法：awk 行号标记（CALL-SHAPE→8444 单门）+ MISMATCH 范围差分定位
  零宽 range；`selfhost_probe_tmp.rs` 按纪律删除。
