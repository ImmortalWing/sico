# STEP-0276: W1 前置 gate — 在途 WIP 吸收（`Map[K,V]` 参数面）+ selfhost `records` 表发射修复

> - status: complete（WIP 已吸收登记；STEP-0272 selfhost 微分回归已修复并实测 byte-exact）
> - phase: M22 compiler self-host — W1 合并债前置 gate（owner 指令 2026-09-24「完成M22，途中发现问题要反馈和回顾」）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机 CLI 微分复现；runner 测试仓本机不可重建——ring 需 gcc——`selfhost_compiler`/`selfhost_parser` 全量断言以 CI 裁决）

## 1. Objective

中断会话留下的在途 WIP（`selfhost/parser.sico` +228 行 `Map[K,V]` 参数面 +
`selfhost_compiler.rs` `dump_nm_region_ir` 探针）是 W1 合并债的前置依赖。吸收审计
发现 WIP 本身质量良好，但 STEP-0272 给 `sico_ir::Module` 增加 `records` 表后，
guest 侧 `scalar_ir` 未同步发射 records 段，含脚手架 record 的 selfhost 微分自
STEP-0272 起即红。本 STEP = 先修 records 发射，微分转绿后把 WIP 与修复一起吸收
登记。

## 2. 审计回顾（发现 → 处置）

- **发现 1（回归）**：`Module.records`（serde `skip_serializing_if=BTreeMap::is_empty`，
  字段在 `import_signatures` 之后）入 schema 后，`scalar_ir` 手工拼 JSON 未发
  records 段；实测含脚手架 record 的语料 guest 输出 = oracle − 393 字节（恒定），
  无 record 语料不受影响。→ **本 STEP 修复**（§3）。
- **发现 2（观察项）**：WIP 参数面（`I64`/`List[I64]`）byte-exact、identity 三次
  重跑确定性匹配；早前一次 68KB 垃圾输出 + 一次挂起**不可复现**，登记为观察项，
  不阻塞吸收。
- 结论：不原样吸收 WIP；records 修复微分全绿后随 WIP 一起登记（= 本 STEP）。

## 3. Changes

### 3.1 guest `records` 表发射（`selfhost/parser.sico`，+262 行新函数 + scalar_ir 两处接线）

- 新增 `record_scalar_kind`（Int/Bool/Text/I64/U64/Float64/Bytes → serde snake_case
  名）、`record_named_type`（首字节 ident 判定）、`record_close_bracket`（`[...]`
  配平；既有 `matching_close` 只配 `(...)`）、`record_map_comma`（Map 顶层逗号）、
  `record_type_json`（递归类型编码：`list`/`map`/`named`/标量，契约 =
  `sico_ir::Type` serde `tag="kind", content="data", rename_all="snake_case"`，
  不支持的字段类型 typed 拒绝 `ERR:E-SH-IR-RECORD-TYPE`，不静默猜）、
  `records_table_json`（扫顶格 `record Name:`…`end record`，`field x: T` 与 bare
  `x: T` 两种字段面均收，字段按声明序；record 名按 BTreeMap 字典序——用
  `sico.list.sort`（byte-order `sico.text.compare`）对 `"Name":[...]` 条目排序，
  常量引号前缀保证与裸名字典序一致）。
- `scalar_ir` 顶部调用 `records_table_json` 并把 `ERR:` 上抛；尾部组装：
  records 非空时 `],"records":{...}}`，**空表整个省略**（与 STEP-0272 前旧字节
  兼容，冻结序列化不变式保持）。
- 实现中触发的两处 guest 静态拒绝已修：Sico cell 函数级作用域（Map 分支局部名
  与 List 分支冲突 → 改名 `map_open_idx`/`map_close_idx`）、`sico.text.concat`
  只收 2 参（拆嵌套）。

### 3.2 在途 WIP 吸收（同文件，+228 行，审计确认为高质量）

- `count_params`：`[...]` 括号深度计数，Map 参数值类型里的逗号不再误计形参
  （`seen_bracket` 配平，`[` 深度 0 才计逗号）。
- `parameters_ir` / `binding_parameter_index` / `binding_parameter_kind` /
  `parameter_kind_by_id` / `declared_return_kind`：`Map[K,V]` 形参/绑定面分支
  （键值 kinds 经 `scalar_type_kind`，形状不合法 `ERR:E-SH-IR-PARAMETERS`）。
- `runner/sico-runner/tests/selfhost_compiler.rs` +7 行：`dump_nm_region_ir`
  探针测试（nearest_match 前缀 oracle dump，println 型，无断言面变化）。

## 4. Validation（本机实测）

微分框架 `target/w1-probe`：oracle = `sico-ir` `dump_oracle_ir` 测试（Rust
canonical IR）；guest = 预构建 sico-runner CLI + 当场 `sico build --profile
script-v0` 的 compiler 组件（4 模块：compiler/compiler_parser/compiler_lexer/parser）。
HEAD 对照 `target/w1-probe-head`（HEAD `parser.sico` 组件）用于归因。

- **formatter 前缀微分（16 项）**：16 项中 14 项 byte-exact OK——含
  `nearest_match`（修复前 HEAD 对照 = MISMATCH −393B，实证回归即 records 段）；
  小语料 identity/keep_count/keep_size/keep_flag/answer/add 全 OK（空表省略
  零回归实证）。
- **HEAD 归因对照**：`close_code`/`direct_close` 在 HEAD 组件同样
  `ERR:E-SH-IR-GWPACK-OTHER`（诚实拒绝）→ 非本次回归，不在
  `selfhost_compiler` 断言集内（断言前沿至 format_code 前缀止），登记为 W1
  合并债范围内的既有 guest 前沿。
- **parser_driver 侧**：summary 模式 INPUT 输出与测试期望串 byte-exact；
  `--emit-ir` 16 个代表语料（常量/全标量参数/字符串转义/unicode/固定位运算/
  嵌套与递归调用/if-else/while/Bool-Text 管线/多参混合）16/16 byte-exact；
  拒绝面 3 例精确（`E-SH-IR-EXPRESSION` 非规范常量绑定、`E-SH-IR-PARAMETER-TYPE`
  形参形状、`E-SH-IR-CALL-TARGET` 未声明调用）。INPUT 的 `--emit-ir`（for/while/
  match 组合）走 `GWPACK-OTHER`，同因非 gated（summary 断言不变）。
- 本机限制：runner 测试仓不可重建（ring 需 gcc），`selfhost_compiler` 19 项 +
  `selfhost_parser` 99 例全量断言以 CI 裁决；探针目录已清理（框架可由
  §4 命令重建）。
- `git diff --check`、`validate-step-0124.ps1`、`validate-module-boundaries.ps1`
  绿（记录见 §5）。

## 5. Follow-ups

- **W1 合并债本体**（legacy lexer/parser 合并）前置已清：parser.sico 工作树
  无在途未定 WIP，可动。
- guest 前沿推进序不变：`close_code`/`direct_close`（gw 机）→
  `normalize_source`/`main`；`selfhost_parser` 99 例剩余 81 例（已测 16 例覆盖
  全部特征族）随下一片 STEP 或 CI 全量裁决。
- 观察项延续：68KB 垃圾输出 / 挂起一次各一，不可复现；若再现，按 R1 canary
  仪器化数据另行诊断。
