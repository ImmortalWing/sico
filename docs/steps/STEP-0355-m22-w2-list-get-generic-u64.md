# STEP-0355 — M22: list.get match 机器泛型 `[U64]` 支持，u64_list_at 翻绿，MATCH-SHAPE 拒绝类清零

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，mid-layer 解锁片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**诊断**：census MATCH-SHAPE 17 户的公共守门人 `u64_list_at`（0354 登记的两个
新辅助亦被它闭包阻塞）微复现——`match sico.list.get[U64](values, idx)` 整身
match 函数（三参 `(values: List[U64], idx: U64, fallback: U64) returns U64`、
ok 臂返 binding、error 臂返第三参）被 `list_get_match_ir` 连续门槛拒：arity
只收 1/2 参、`get` 后直接要求 `(`（泛型 `[U64]` 落在族外）、声明元素写死
`Text`、return_kind 写死 string、发射写死 `sico.list.get` 名与 string ok 型。
oracle 转储钉死目标字节：intrinsic 名携带泛型（`sico.list.get[U64]`）、
result ok/return/project 三处为 `u64`、error 臂空指令块直接 return 参数 id。

**修改**（parser.sico `list_get_match_ir` 一处机器，+~90 行）：

- `get` 后可选泛型段识别：`[ <Text|U64> ]`（族外类型 CALL-TYPE；无泛型且
  元素 U64 → CALL-TYPE〔return-kind 门先至者记 TYPE-MISMATCH， Fixture 按
  实登记〕）；`(` 定位与 operand 扫描随泛型右移，既有 no-generic 路径零行为
  变化；
- arity 扩 3 参 + 声明第三段 `, p2: U64` 解析 + 声明元素按 `lgm_elem_kind`
  参数化（Text/U64）；
- 发射参数化：结果型/return_type/project ty 用 JSON 小写型（新局部
  `lgm_elem_json` = string/u64；`lgm_elem_kind` 保大写供词比较与泛型名），
  intrinsic 名按泛型 presence 拼 `[U64]`；
- U64 形状的 fallback 必须是参数（字符串字面量 fallback → CALL-TYPE）。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_list_get_generic_u64_match_byte_exactly`——`u64_list_at`
原文字节全等 + 三例 typed 拒绝守卫（`get[I64]` → CALL-TYPE、no-generic →
TYPE-MISMATCH、U64 形状字面量 fallback → CALL-TYPE）+ 拒绝后恢复正例。

## 2. 前沿更新

- **u64_list_at 翻绿**（微复现 229B 单函数字节全等 + census 闭包双证实）；
  依赖闭包 census **65→66/126、0 DIFF**。
- **MATCH-SHAPE 17→0 清零**——健康级联：原 17 户多为闭包含 u64_list_at 被其
  门代言，今各自推进到真实下一道门；现 histogram = CALL-ARGUMENT 34 /
  EXPRESSION 24 / GWCOND-EXPRESSION 1 / STATEMENT 1（新码 GWCOND-EXPRESSION
  首次出现 = gw 条件表达式门被探到，非回归）。两个 0354 新辅助如实推进到自
  身下一门（gw_unreachable_terminator → GWCOND-EXPRESSION：gw 体内字符串
  字面量 return；gw_deferred_tail_json → EXPRESSION：set-append 链）。
- 下一刀主线：gw 条件/表达式门（gw 体内字面量 return、set-append 表达式链）
  + CALL-ARGUMENT 34 户被调链（机器族自降）。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0354 终值（65/126、0 DIFF、HIST 四格）后动刀。
- `--test selfhost_compiler`：**65/65 绿**（含新固件正例 + 三拒绝守卫 + 恢复）。
- 回归：validate-step-0262 / validate-step-0245（含 runner fmt/clippy，借主仓
  msys2 gcc）/ report-m22-canary / validate-step-0124 -SelfTest / git diff
  --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=66 diffs=0 total=126；HIST
  CALL-ARGUMENT 34 / EXPRESSION 24 / GWCOND-EXPRESSION 1 / STATEMENT 1；
  MATCH-SHAPE 户空。
- R3 连续停滞数保持 0（parser 覆盖 65→66 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
