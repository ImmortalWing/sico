# STEP-0354 — M22: gw 机器尾 match 双臂 return 收口（unreachable join），gw_instr_count 翻绿，RETURN-COUNT 拒绝类清零

> - 状态：implementation complete locally（本机全量回归绿：0262/0245/canary/0124/diff-check；independent CI pending）
> - 里程碑：M22 compiler self-host（S6 全源差分推进，gw 机器族自降片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**诊断**：census RETURN-COUNT 23 户的大头 `gw_instr_count`（0353 登记）微复现二分
——while 后 after 区尾 match 双臂皆 `return` 的形状被拒 RETURN-COUNT；根因 = gw
机器 `end match` 处理器无条件把 join 块建成可继续块并把 `gcur_term` 置 false，
函数尾随之没有终结点，epilogue 的 `gcur_term` 门（parser.sico 原 12446 行）拒
RETURN-COUNT。oracle 转储（微复现探针）证明该形状的 join 块**保留但终结子为
`{"kind":"unreachable"}`**（range = match 头跨度，与可达 join 相同）。

**修改**（parser.sico gw 机器一处表 + 两处手术 + 三个抽出的辅助函数，机器本体
净局部数反降 248→245〔头寸地板 8→11〕）：

- 新登记簿 `gunreach`：unreachable join 块 id 表；终结子解析链查表发射
  `{"kind":"unreachable"}` 由抽出辅助 `gw_unreachable_terminator` 承担；
- `end match` 处理器：双臂皆终结（ok 臂终结 flag + `gcur_term`）**且**函数尾
  紧随（跳 nl 后窥见 `end function`，抽出辅助 `gw_tail_close_probe`）时，join
  块登记 unreachable、`gcur_term` 保持 true（epilogue 门原样通过）；非函数尾
  维持旧行为（可达 join，死后续代码落入其中——oracle 字节实证此路径本就一
  致）；
- 延迟块发射整段抽出为 `gw_deferred_tail_json`（0343 登记纪律「gw 加宽须抽
  辅助函数」；途中修复 oracle 规则「参数非 cell 不可 set」——辅助内累加用
  局部 `gdt_blocks` 承接参数）；
- 非双臂终结 / 非函数尾形状零行为变化（全部既有冻结固件 64/64 原位绿）。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_tail_match_both_arms_return_byte_exactly`——尾 match
双臂 return（unreachable join 形状）、尾 match 后死后续 return（可达 join 形
状）、`gw_instr_count` 原文 + `next_index` 闭包三例字节全等；族外 op
（`U64.bit_and`）主语保持 CALL-TARGET 拒绝守卫 + 拒绝后恢复正例。

## 2. 前沿更新

- **gw_instr_count 翻绿**（微复现 + census 闭包字节全等双证实）；连带新翻绿
  6 户：`fixed_operation_operand_atom_idx/check/value`、
  `fixed_operation_literal_count`、`chained_operand_value`（自身形状即本闸门
  或闭包内被调解锁）+ 新辅助 `gw_tail_close_probe`（本片自降即翻绿）；
  依赖闭包 census **58→65（分母 123→126，含三个新辅助）、0 DIFF**。
- **拒绝类地图**：RETURN-COUNT **23→0 清零**（两片连清，0353 清 CALL-TARGET
  后本片清 RETURN-COUNT）；现 histogram = CALL-ARGUMENT 32 / MATCH-SHAPE 17 /
  EXPRESSION 11 / STATEMENT 1（数字上升 = 原 RETURN-COUNT 户与两个新辅助推进
  到各自下一道门，非回归；新辅助 `gw_unreachable_terminator`/
  `gw_deferred_tail_json` 拒 MATCH-SHAPE，为 mid-layer 自降后续片留口）。
- 头寸：`general_while_function_ir` 245 locals（限 256，headroom 11，较本片
  前基线 248 反降——抽辅助函数纪律生效）。
- 下一刀主线：gw 机器族自降剩余户（R3 多返回 if 收口、match 主语族缺口、
  两个新辅助的 MATCH-SHAPE 门）与 CALL-ARGUMENT 32 户被调链解锁。

## 3. 验证与预算

- 基线复核：本片开工前 validate-step-0262 全绿（selfhost_compiler 63/63、
  parser 2/2、local bounds 1/1、STEP_0262_OK、canary FORMATTER-CLOSED）。
- 探针纪律：闭包 census 复现 0353 冻结数字（58/0/histogram 逐格一致）后
  才动刀；微复现 A/B/C/C3/R1/R2 六个形状逐一对 oracle；探针文件
  （scratch_census_probe.rs 与 target/tmp/census-0354/）按纪律删除。
- `--test selfhost_compiler`：**64/64 绿**（含三例新冻结字节全等 + 族外拒绝
  守卫）。
- `selfhost_local_bounds`：general_while_function_ir **245 locals（headroom
  11）**——途中 256 顶格过界，按 0343 纪律抽三个辅助函数后反降（本片前 248）。
- 回归：validate-step-0262 / validate-step-0245（含 runner fmt/clippy，clippy
  借主仓 `E:\github\sico\target\tooling\msys2-binutils` gcc 通过）/
  report-m22-canary（FORMATTER-CLOSED 30/30）/ validate-step-0124 -SelfTest /
  git diff --check 本片重跑**全绿**。
- census 终值（探针按纪律删除）：flipped=65 diffs=0 total=126；HIST
  CALL-ARGUMENT 32 / MATCH-SHAPE 17 / EXPRESSION 11 / STATEMENT 1；
  RETURN-COUNT 户空。
- R3 连续停滞数保持 0（canary 30/30 与 parser 覆盖 58→65 双增长）；M22 整体
  NO-GO 不变（S6/S7 未进入）。
