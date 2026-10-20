# STEP-0353 — M22: gw match 主语家族扩 `checked_mul`/`checked_div`，gw_decimal_value 翻绿，CALL-TARGET 拒绝类清零

> - 状态：implementation complete locally / independent CI pending（本机 runner clippy 缺 gcc 环境阻塞，CI 裁决）
> - 里程碑：M22 compiler self-host（S6 全源差分推进）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**诊断**：census 唯一 CALL-TARGET 守门人 `gw_decimal_value`（0351 登记）微复现
二分定位——`match U64.checked_mul(acc, U64.literal(10))` 形状（while 体内、
error 臂早退）被拒；根因 = gw match 主语路由（`gw_match_open_pack` 的
is_checked 判定 + `gw_checked_add_subject_packed` 的 op 名检查）只认
`checked_add`/`checked_sub`（STEP-0324 家族），`checked_mul`/`checked_div`
（STEP-0134 落地、Result[T, NumericError] 同型）落在族外拒 CALL-TARGET。

**修改**（parser.sico 三处、+18 行，无新机器）：

- `gw_match_open_packed`：I64/U64 双分支的 op 名判定各扩 `checked_mul`/
  `checked_div` 两个 else 档；
- `gw_checked_add_subject_packed`：op 名检查同步扩族；
- 发射路径零改动——`gca_op_name` 原样进 IR op 字段、result_ty 按
  `Result[kind, NumericError]` 构造，与 oracle 对
  `checked_mul`/`checked_div` 的发射天然一致（字节差分证实）。

**测试固件同步**（selfhost_compiler.rs，同 scoped 变更）：

- STEP-0324 钉的 `bad_op` 拒绝固件用的正是 `checked_mul` 主语（彼时族外
  → CALL-TARGET）；本片扩族后该固件转为成功降低——固件按新前沿改写：
  正例档新增 `checked_mul` cell×literal 字节全等用例（gw_decimal_value
  真实形状），`bad_op` 改用真族外 op `U64.bit_and`（CALL-TARGET 守卫保留）；
- 测试头注释登记 0353 扩族事实。

## 2. 前沿更新

- **gw_decimal_value 翻绿**（微复现 2298B 闭包 guest 降低 + census 闭包字节
  全等双证实）；依赖闭包 census **58/123 翻绿、0 DIFF**（57→58）。
- **拒绝类地图**：CALL-TARGET **1→0 清零**；现 histogram = RETURN-COUNT 23 /
  CALL-ARGUMENT 21 / MATCH-SHAPE 12 / EXPRESSION 8 / STATEMENT 1。
- 下一刀主线不变：多返回 while（gw_instr_count 3 返回形状，RETURN-COUNT
  23 户的大头）与 gw 机器族自降。

## 3. 验证与预算

- `sico check selfhost/checker.sico`：ok。
- 微复现探针（CLI 直跑，临时）：`checked_mul` while 主语 / `bytes.at` 三参 /
  嵌套 if-else 数字链三形状隔离，唯 checked_mul 拒（修复后全过；
  `checked_add(u64, text.encode(...))` 类型错仍如实 CALL-TYPE）。
- census 探针（临时，按纪律已删）：flipped=58 refused=65 diffs=0 total=123。
- `--test selfhost_compiler`：**63/63 绿**（含新 checked_mul 正例 + bit_and
  拒绝守卫）。
- `validate-step-0262.ps1` / `validate-step-0245.ps1`：本片重跑全绿
  （M22 回归 bootstrap_bundle + selfhost_checker + selfhost_compiler）；
  runner clippy 环境阻塞同 0352 记录（CI 裁决）。
- 探针按纪律删除；`git diff --check` 绿。
