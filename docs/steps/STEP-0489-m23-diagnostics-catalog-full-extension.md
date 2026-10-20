# STEP-0489 m23 诊断协议目录全量扩展（实现态对拍闭包）

> - status: implementation complete locally（未提交快照；本片关闭 M23 出口审计登记的 `catalog.json` / `semantic-case-map` 全量协议扩展欠账）
> - date: 2026-10-20
> - owners: autonomous-agent
> - depends: [RFC-0001](./../rfc/RFC-0001-diagnostics-protocol-v0.md)、[RFC-0047](./../rfc/RFC-0047-record-types-v0.md)、[RFC-0048](./../rfc/RFC-0048-language-v1-batch3-infix-comparison-logical-and.md)、[RFC-0049](./../rfc/RFC-0049-language-v1-batch3-let-bound-if-match.md)、[M23 出口审计](./../reports/m23-exit-audit-2026-09-30.md)
> - gate: M23 出口审计 §4 residual #4（诊断目录/case-map 全量协议扩展）——本片关闭该项；M23 整体出口状态不变（item 3 仍为 owner 凭据外部门，STEP-0328/0329 独立 CI 仍为外部门）

## 1. 定位

M23 出口审计（2026-09-30）把三项残余写得很清楚，其中唯一可在本地闭合的是：

> `diagnostics/catalog.json`/semantic-case-map 的全量协议扩展仍欠（E2023/E2024/E2031 同批先例，登记为独立于本里程碑的后续收口项）。

实测确认这不是"文档没跟上"的小事，而是**协议事实与编译器实现已经分叉**：

- 编译器实现了 8 个语义码却从未登记进目录（E2021–E2027、E2031），目录声明的唯一事实源地位失效；
- 其中 3 个码的变化值**只存在于人读文本**里，机器通道根本拿不到：

| 码 | 修复前的机器通道 | 缺陷 |
|---|---|---|
| E2024 | `arguments = {type}`，文本里才有算符 | 比较算符（`==`/`<`/…）不可被机器复原 |
| E2025 | `arguments = {}`，文本里才有算符与改写式 | 交换形算符与非交换改写式全部丢失 |
| E2026 | 同一码两条文本（if 臂 / match 臂） | 一个 code 对应两个默认短消息，目录只能有一个模板 |

RFC-0001 §1 要求"同一个 `code` 永远表示同一类根因"，§7 要求"模板占位符必须与 `required_arguments` 完全一致"。这三条同时被违反，说明缺口在**协议形状**层面，不是文案层面。

## 2. 交付物

| 项 | 位置 | 说明 |
|---|---|---|
| 目录追加 | `diagnostics/catalog.json` | 追加 8 个已实现语义码（E2021–E2027、E2031），append-only 不动既有 41 个 code；模板与 `required_arguments` 按 RFC-0001 §7 同形 |
| 实现态语料 | `diagnostics/implementation-cases/`（新目录） | 8 个最小拒绝源 + 每组 README 证明行 + `manifest.json`；用**当前可执行语法**写，可被真实编译器复现 |
| case map 登记 | `diagnostics/semantic-case-map.json` | 追加 8 条（case id / source / code / key / arguments / expected_message） |
| 协议校验器扩展 | `tools/validate-diagnostics.ps1` | 接受第二个 case 源根（实现态语料）、README 行按源目录就近查找、实现态清单与分组计数校验；"目录项必须被 case 覆盖"的规则**不放宽** |
| 实现态对拍校验器 | `tools/validate-step-0489.ps1`（新） | 构建 `sico-cli`，对每个实现态 case 跑 `sico check --json`，断言 code / key / argument 集合 / argument 值 / 渲染后短消息与目录**逐字节一致** |
| 编译器对齐 | `crates/sico-semantics/src/lib.rs` | E2024 补 `operator` 参数并把类型名纳入消息；E2025 传 `operator` + `rewrite`；E2026 统一 match 臂文案为与 if 臂同一条 |
| 证据链修复 | `tools/validate-step-0328.ps1`、`tools/validate-step-0329.ps1` | 第二遍 formatter 幂等检查原走 stdin；Windows PowerShell 5.1 的重定向 StandardInput 会发出 `[Console]::OutputEncoding` 的 UTF-8 BOM，`sico format -` 因此以 `source contract error ByteOrderMark at bytes 0..3` 拒绝输入（0328 抛"second formatter pass failed"，0329 得到空第二遍并误报"非幂等"）。改为字节精确文件重定向后两条校验器在本机复跑均 exit 0——与 `tools/validate-step-0262.ps1` 的同模式修复一致 |
| 文档 | `diagnostics/README.md`、`docs/rfc/README.md`、`docs/user-guide/LANGUAGE-BASICS.md`、`docs/plans/M23-language-v1-batch-3.md`、`docs/ROADMAP.md` | 目录计数与范围边界；RFC-0048/0049 索引状态（draft → accepted）；用户指南拒绝面；计划 §10 残余闭合表；路线图 M23 状态 |

## 3. 同形对齐逐条

| 码 | 目录模板 | `required_arguments` | 编译器对齐 |
|---|---|---|---|
| E2021 | `duplicate field: {field}` | `field` | 已同形 |
| E2022 | `record {record} declares no fields` | `record` | 已同形 |
| E2023 | `field {record}.{field} is immutable; replace the whole record with set` | `record`,`field` | 已同形 |
| E2024 | `records of type {type} do not support {operator}; compare fields explicitly` | `type`,`operator` | **改**：补 `operator`；类型名进消息（此前只在 arguments 里） |
| E2025 | `non-atom operand in a swap-form {operator} condition; evaluate the operand into a binding first or rewrite as {rewrite}` | `operator`,`rewrite` | **改**：由空 arguments 改为两者齐备 |
| E2026 | `let-bound arm must be exactly one expression` | — | **改**：match 臂文案统一为与 if 臂同一条 |
| E2027 | `let-bound if requires an else arm` | — | 已同形 |
| E2031 | `unresolved call target {callee}` | `callee` | 已同形 |

E2024 / E2026 的文案变化属 RFC-0001《Compatibility》允许的"不改变含义的文案修正"；E2024/E2025 的 arguments 变化属"原本只存在于人读文本的变化值补进机器通道"，code 含义与根因类别未变。

## 4. 为什么用增量语料而不是扩 P0 语料

`semantic-cases/` 的 33 个非法 case 是**设计态**语法（`fn main() -> T`、`record Feature … end`），不是当前可执行语法——它们承载 code/key 的**设计映射**，无法证明"编译器真的发出这个码"。

扩 P0 语料会连带触发：58 → 66 个 case、174 → 198 个 program、`syntax-candidates/A|B|C` 三套镜像与 `semantic-cases/manifest.json` 计数重算——这是对冻结语料的大面积改动，与"补一个协议登记"不成比例。

因此沿用仓库既有先例（STEP-0280 的 `corpus-w1` 五条 SHA 冻结增量语料），新建**实现态增量语料**目录：冻结语料零改动，新语料由真实编译器复现。

## 5. 诚实性

- 本片**不**改动 P0 语料、`syntax-candidates` 镜像、`syntax-mutations` 清单与任何冻结快照；
- 本片**不**新增语言面，**不**改 IR / codegen / Runtime / Host；
- 本片**不**触碰 M22 关键路径：batch-3 表面仍是自托管前端声明的**不相交子集**（0 个中缀算符、0 个 let 绑定形），M22 差分字节稳定，无 re-baseline 义务；
- **E8xxx 不进目录**：RFC-0001 把 `E8000–E8999` 定为 reserved 分区，RFC-0039 的模块/包码属于 CLI 链接通道身份，不是源码级编译器诊断；登记它们要改分区状态（新协议主版本 + RFC），故显式留在目录外并写入 `diagnostics/README.md` 的范围说明；
- **E1014–E1016 仍未登记**：它们的 case map 绑定冻结的 mutation / candidate 语料，扩表是另一个有界步骤，不在本片静默扩表（已登记为独立欠账）；
- 证据等级 `internal-fixture`，Windows x64（`1.98.0-x86_64-pc-windows-gnu`）本机真实 CLI；**独立 CI 未跑**（本机不可达 GitHub Actions）。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 协议校验 | `tools/validate-diagnostics.ps1` | `DIAGNOSTICS_OK catalog=49 semantic_cases=41 syntax_mutations=12 top_level_cases=2 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=106`，exit 0 |
| 实现态对拍 | `tools/validate-step-0489.ps1` | 8/8 case 由真实编译器复现（code / key / arguments / 渲染消息逐字节一致），exit 0 |
| M23 item 1 校验器 | `tools/validate-step-0328.ps1` | `STEP-0328 validator OK (matrix, e2e corpus, refusal corpus, formatter idempotence)`，exit 0（BOM 修复后复跑） |
| M23 item 2 校验器 | `tools/validate-step-0329.ps1` | `STEP-0329 validator OK (matrix, e2e corpus, refusal corpus, nesting budget, formatter)`，exit 0（BOM 修复后复跑） |
| 冻结语料未动 | `tools/validate-semantic-cases.ps1` | `SEMANTIC_CASES_OK cases=58 valid=25 invalid=33 A0=58 B=58 C=58 programs=174`，exit 0 |
| 模块边界 | `tools/validate-module-boundaries.ps1` | `MODULE_BOUNDARIES_OK model=openjdk-style packages=29 exceptions=4`，exit 0 |
| 根工作区回归 | `cargo fmt --all -- --check` + `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` + `cargo test --locked --offline --workspace --all-targets --all-features` | fmt 绿、clippy 绿、workspace 测试全绿（含 `check_matches_all_58_semantic_oracles`） |
| 空白纪律 | `git diff --check` | 净 |

未跑：runner 独立工作区全量（本步不触及 runner/codegen 行为，仅诊断消息与协议登记）、独立 CI（本机不可达 GitHub Actions）、live-model 评测（owner 凭据外部门，与本片无关）。

## 7. 后续

- **M23 剩余两项均为外部门**：item 3 裸字面量复称需要 owner 凭据的 live-model 语料腿；STEP-0328/0329 的独立 CI 裁决需要可达的 CI。本片不改变这两项的状态，也不把 M23 读作整体 GO。
- **M22 关系**：本片不改变 M22 的任何 gate（门 3 仍 NO-GO），也不引入自举 re-baseline 义务。
- **后续有界欠账**：E1014–E1016 的目录登记（需扩 syntax case map）。
