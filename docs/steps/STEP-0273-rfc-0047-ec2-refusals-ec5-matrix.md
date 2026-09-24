# STEP-0273: RFC-0047 EC-2 semantic refusals + EC-5 matrix row (records exit-corpus closure, D5除外)

> - status: complete (E2023/E2024 refusal surface landed; EC-5 matrix row registered; D5 `List[record]` remains gated with an explicit layout decision carried to the next slice)
> - phase: records implementation (RFC-0047) — M22 路线（owner 指令 2026-09-24「完成M22，途中发现问题要反馈和回顾」）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture

## 1. Objective

owner 目标「完成 M22」的 records 收口段：EC-2 语义面两处置信缺口（实测发现的，非计划内）+ EC-5 机器矩阵行登记，把 RFC-0047 v0 封闭集在 check/build/run 三态上钉死。D5 `List[record]` 是独立的下一片（布局决策见 §5）。

## 2. Measured findings（实测发现，反馈项）

1. **`set p.x = …` 误报型拒绝**：set 语句语义只取 tokens[1] 作 cell 名，`.x` 被静默忽略，报 `E2001 expected Point, found I64`——拒绝是对的但诊断撒谎（把不可变字段写说成类型不匹配）。
2. **`a == b`（record 相等）静默放行**：语义层中缀 `==`/`<=` 分支推断两侧后**丢弃结果、无条件返回 Bool**，record 比较 check 绿——D3 明确 v0 拒绝，这是 check/build 真缺口（build 未必过，但语义层不该沉默）。
3. **EC-5 矩阵校验器的 step 头是冻结锚**（`STEP-0131..0203` 尾锚定 modules/packages 片），records 行只能带行级 step——已按此登记。

## 3. Changes

- `crates/sico-semantics/src/lib.rs`：
  - `set <cell>.<field>` 形状识别 → **E2023 FIELD_IMMUTABLE**（字段 v0 不可变，行动提示 = 整记录替换；误导性 E2001 不再出现）；
  - 中缀 `==`/`<=` 任一侧为名义 record 类型 → **E2024 RECORD_COMPARISON**（每比较单诊断，落在第一个 record 侧；字段级比较需显式组合）。
- EC-2 语料 +3：`record-refusal-field-set.sico`（E2023）、`record-refusal-equality.sico`（E2024）、`record-copy-on-write.sico`（COW 别名语义 accept + run → `record-cow-ok`：别名不受整记录替换影响）。
- `runner/sico-runner/tests/record_types.rs`：+2 拒绝测试、+1 run 测试（11 个测试全接线）。
- EC-5：`tests/language-matrix/application-profile-v0.json` 登记 `records-v0-user-records` 行（check/build/run true，行级 step STEP-0271..0273；`List[record]` 如实注记未执行）。

## 4. Validation（本机实测）

- E2023/E2024 各单诊断、span 正确；COW 语料 build+runner 实跑 `record-cow-ok`；
- `cargo test -p sico-semantics -p sico-ir` 全绿（语义 11/11、IR 全套）；
- `validate-step-0131.ps1`（矩阵校验器）绿；`git diff --check` 绿；
- 既有 9 语料零回归（复测在 STEP-0272 验证后未动相关路径）。

## 5. D5 决策反馈（下一片的输入）

`List[record]` 执行需要定**元素存储布局**，两条路：

- **ADR-0016 列式（真并行数组）**：每个字段一列，append/get 要维护 K 列——与「SOA 成本身份不变」的承诺最诚
  实（RFC D4 原文「lowers to exactly the parallel-array representation ADR-0016 mandates」），但 list helper 全家（empty/append/get/length）要按列重写，for 迭代的 record 绑定要按列重组字段图——工作量最大；
- **行式（每元素 K 槽连续）**：复用现有单表 helper 骨架（stride=K），工作量约为列式三分之一，但与 ADR-0016 的字面承诺有偏差，re-baseline 时 SOA 对照燃料曲线会有常量差异。

RFC-0047 D4 的字面文本指向列式；按「文档与证据冲突取低声明、先合同后实现」的纪律，**下一片默认按列式实现**（除非 owner 拍板行式），并补 `record-list-of-records.sico` 的 run 解锁 + for 迭代语料。

## 6. Follow-ups

- D5 列式 `List[record]`（本 STEP §5 决策）+ `List[List[record]]` 深度预算核对 + D5 gate 语料解锁；
- EC-3 canonical IR JSON 快照（records 表序列化 byte-exact 对拍 Rust oracle）随 D5 一起；
- M22 主线：records 收口后按 ADR-0017 进 W1（legacy lexer/parser 合并债，**在途 WIP parser.sico +228 行须先落地为独立 STEP 或吸收**），随后 S3/S4 源层以 records 重写（re-baseline，R3 计数重启）；
- 途中问题已全部按 owner 要求即时反馈（§2、§5）。
