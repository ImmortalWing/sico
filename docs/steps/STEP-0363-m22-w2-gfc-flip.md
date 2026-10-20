# STEP-0363 — M22: `gw_find_code_text` 翻绿（escape 奇偶 prev-index 化），list_get/chained_let/straight_let 三户连带翻绿

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，自降前置闭环）
> - 证据级别：internal-fixture / runtime-verized（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**0362 遗留门定位（T6–T8 截体）**：prev-byte 形式的循环尾
`set gfc_prev_byte = gfc_b` 本身是门（T6 去之即通；T7 前移位置亦通但
语义错）——根因不重考（已两次实证：同位置 cell set 的接收者与深度
组合触发 gw 机器未建模形状），**正解 = 语义等价改写**：以
`gfc_prev_idx`（前一字节下标 cell）替代 prev 字节缓存——循环尾
`set gfc_prev_idx = gfc_i`、转义判定改 `sico.bytes.at(src, gfc_prev_idx,
0)`（cell 游标实参 0362 门 1 已开）、命中转义时 `set gfc_prev_idx =
gfc_i` 复位（连续反斜杠奇偶正确）。T8 截体验证通过 + **oracle 字节全等**。

**连锁**：gfc 翻绿 → 含它的三个大闭包解锁——**list_get_match_ir /
chained_let_function_ir / straight_let_function_ir 全部 FLIP**（0361/0362
的 span 修复 + to_text 门 + gfc 自降低四片合力）。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_find_code_text_byte_exactly`——gfc 闭包（
find_text_from/next_index/gfc）+ straight_let 全 25 被调闭包两段字节全等。

## 2. 前沿更新

- 依赖闭包 census **76/127、0 DIFF**（净翻绿数不变：+4 新翻绿
  〔gfc/list_get/chained_let/straight_let〕与 4 户推进/换手相抵，
  histogram 逐格不变属健康流动）。
- EXPRESSION 26 户现为最大类（机器族自降主战场）；CALL-SHAPE 14 /
  CALL-ARGUMENT 8 户次之。
- 下一刀主线：gw 机器族自降（gw_rhs_packed/gw_condition_packed/scalar_ir
  等机器本体的 EXPRESSION 门）——机器本体翻绿即大片连锁。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0362 终值（76/127、0 DIFF）后动刀；
  每个子改动立即 census（0360 以来纪律）。
- 微复现：T6/T7/T8 截体定位；gfc 单函数 + 三户闭包 oracle 字节全等。
- `--test selfhost_compiler`：**69/69 绿**（含新固件两段字节全等）。
- 回归：fmt / validate-step-0262 / validate-step-0245（含 runner clippy，
  借主仓 msys2 gcc）/ report-m22-canary / validate-step-0124 -SelfTest /
  git diff --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=76 diffs=0 total=127；HIST
  CALL-ARGUMENT 8 / CALL-SHAPE 14 / EXPRESSION 26 / GWPACK-OTHER 1 /
  NON-DOMAIN 1 / STATEMENT 1。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
