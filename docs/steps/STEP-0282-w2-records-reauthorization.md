# STEP-0282: W2 主片 — guest records 重授权（8 条豁免关闭）

> - status: complete（guest 修复 + 豁免关闭已落地；runner 测试仓本机不可
>   重建——gcc/dlltool 缺失——集成链以 CI 裁决）
> - phase: M22 compiler self-host — W2（guest re-authorization；R3 止损计数沿用）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机：checker 组件 235 条全量探针 strict 模式 235/235；CI：集成链全量）

## 1. Objective

关闭 STEP-0279 登记的 8 条 records 豁免（record-basic 被误拒 + 7 条
record-refusal 首码不齐）。oracle 语义（`crates/sico-semantics/src/lib.rs`）
已读实：E2021 = 构造器重复命名字段；E2011 = 未知字段名访问；E2022 = 空
record 声明；E2023 = `set` 行目标带 Dot（纯形状）；E2024 = record 类型
参与 `==`/`<=` 比较；E2010 = 构造缺字段；E2001 = 形参类型不匹配。

## 2. Changes（`selfhost/compiler_semantics.sico`）

第一遍（声明扫描）：

- **裸字段注册**：record 体内非 `field `/`invariant `/`end ` 开头的行
  （`x: I64` 形，words ≥ 2）按 name=at(0)、type=at(1) 注册进
  field_owners/names/types（新 helper `bare_field_line` 做正向排除）。
- **`field ` 形放宽 parts==3 → parts>2**：`field arguments: List[Text]`
  经 words() 切成 4 个词，旧条件静默丢字段（ScriptInput.arguments
  从未注册）；放宽后以 at(2) 近似类型（`List`），近似可接受。
- **E2022**：`end record` 清 active_record 之前
  `owner_count(field_owners, active_record)==0` → 拒。

第二遍（body 检查，`process_body_line==1` 块，payload 扫描之后）：

- **local 类型表** `local_names/local_types`（函数头与 end function 处
  重置）：`let x = RecordCtor(...)` 形（at(2) ∈ record_names）登记
  x→RecordCtor；查类型顺序 parameter_type 优先、local 表兜底。
- **E2023**：`set ` 行 ∧ 含 `=` ∧ 新 helper `dot_before_equal`（首个
  `=` 前出现 `.`）→ 拒。`set shifted = Point(x: shifted.x, ...)` 的
  `=` 在 `.` 前 → 不误伤。
- **let 行构造器检查**（ctor = at(2) ∈ record_names）：先 E2021（新
  helper `count_field_label` 数 `name:` 出现次数 >1），后 E2010
  （missing_field_label 逐字段）；**E2001 let 形**：callee=at(2)
  有单形参期望、actual=at(3) 解析（参数∪local）∈ nominal/record 且
  不同 → 拒。
- **E2024**：`if ` 行 ∧ words==3（`==`/`<=` 被 words() 丢弃，
  `[if,left,right]`）∧ 任一侧解析类型 ∈ record_names → 拒。
  words==5 的 `fresh.revision == old.revision` 形天然排除。
- **E2011 未知字段访问**：新 helper `dot_access_field`（子串匹配 root
  + 边界 `at_word_boundary` + **引号外** `outside_quotes` + `.` +
  字母起头的候选词），对 record 型 local 与函数形参逐 root 扫描：
  候选 ∉ record_names ∧ record 无此字段 → 拒。

## 3. 误伤面与排除条件（探针裁决，全部实证）

- `lis**t.**append` 类子串误伤（root "t" 单字符）→ `at_word_boundary`
  （前一字节须非词字节）。
- `"input.txt"` 字符串字面量误伤（root "input"）→ `outside_quotes`
  （按 `"` 计数判引号内）。
- `(origin, I64)` 词邻接误判 → 弃用词邻接方案，改子串+边界+引号三
  重判定。

## 4. 结果（8 条豁免 → strict）

| 条目 | oracle | 修复后 guest |
| --- | --- | --- |
| record-basic | accept | accept ✅（自 E2011 误拒中恢复） |
| duplicate-field | E2021 | E2021 ✅ |
| empty | E2022 | E2022 ✅ |
| equality | E2024 | E2024 ✅ |
| field-set | E2023 | E2023 ✅ |
| missing-field | E2010 | E2010 ✅ |
| nominal | E2001 | E2001 ✅ |
| unknown-access | E2011 | E2011 ✅ |

- `DIVERGENCE_WAIVER` 9 → **1**（只剩 IND-004 的 E2031 first-identity）。
- 分区 (116, 38, 72, 9) → **(116, 45, 73, 1)**。
- **发现并修复 STEP-0280 批次遗留的测试不自洽**：零_indent
  `zero-indent-recovery-declaration`（首码 E1001）此前落 `_` 臂（要求
  Output），与 guest 实际拒绝矛盾——`selfhost_checker.rs` 语义码臂补
  E1001（guest 拒 E1001 已由探针实证）。

## 5. Validation（本机实测）

- `sico check selfhost/checker.sico` ok；`sico build --profile script-v0`
  成功。
- **checker 组件 235 条全量探针：strict（WAIVER=1）235/235 pass**
  （约 263 s）；此前 waiver 模式 235/235、单条 12 条 record 逐一核对
  11/12 → 修 boundary 后 12/12。
- `selfhost_checker.rs` 经 rustfmt --check；分区断言按 corpus-v0.json
  独立复算 (116,45,73,1) 一致。
- 本机不可运行（已知宿主限制）：runner 测试仓 cargo 构建（gcc/dlltool
  缺失，9-20 产物指纹与今构不一致需全量重编译），集成链全量以 CI 裁决。

## 6. Follow-ups（W2 收尾）

- IND-004 的 E2031 first-identity（unresolved call target）——唯一
  剩余豁免，关闭即 W2 收官。
- 候选 declared-subset：parser 顶格行语法检查；match/try 块的顶格扩展。
- E2011 子串扫描的已知近似：字符串内转义引号未建模、record 型 root
  重名跨函数不串扰（local 表按函数重置）——语料未覆盖，登记为
  declared-subset 观察项。
