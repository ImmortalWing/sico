# STEP-0278: W1 片 A+B — legacy lexer/parser 三件套退役 + sha 门验证关闭

> - status: complete（实现已落地；runner 测试仓本机不可重建——gcc/dlltool 缺失——
>   集成链语料差分以 CI 裁决）
> - phase: M22 compiler self-host — W1 consolidation（[`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md) §5 items 1+3；计划 = STEP-0277）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机：引用清扫 + 语法检查 + 集成链 check；CI：215 语料全量差分）

## 1. Objective

按 STEP-0277 §5 的分片顺序执行 W1 的片 A（item 3 sha 门验证关闭）与片 B
（item 1 legacy 三件套退役）。机械性变更，不改 lowering 语义。

## 2. Changes

### 2.1 片 A（item 3）— sha 门验证关闭，零代码改动

- 实测 `selfhost_checker.rs:159-168` 对 corpus-v0.json **每个**入口断言冻结
  `bytes` + `source_sha256`（sha2，`format!("{:x}", Sha256::digest(&source))`），
  与 `bootstrap_bundle.rs:189` 同模式；`git log` 证实随 STEP-0246 批
  （0eaccf4）落地，早于评估 §5 item 3 的登记——**债已在还，本片验证留证
  并登记关闭**。

### 2.2 片 B（item 1）— legacy 三件套退役

- **删除源文件**：`selfhost/lexer.sico`(117) / `selfhost/tokens.sico`(459) /
  `selfhost/declaration_parser.sico`(371)。退役前终态引用面实测（STEP-0277
  §2.1）：selfhost 源层零 import，评估所述 `tokens.sico:431-435` 裸 `<`
  缺陷分支随之消灭。
- **删除单位级单测**：`runner/sico-runner/tests/selfhost_lexer.rs` /
  `selfhost_tokens.rs` / `selfhost_declaration_parser.rs`（各 1 test，跑
  corpus-v0.json 子面）。丢失的三条单位级断言面由集成链整拍
  （`selfhost_checker` 215 全量：116 lexical / 34 identity / 65 accepted /
  0 unsupported）+ 语料 sha 冻结接替——评估 P2「不维护两个 lexer」的
  既定取舍。
- **历史校验器改写为退役登记**（原钉文件存在 → 现钉文件不存在 + 历史
  锚仍在 + 集成链接替）：
  - `validate-step-0215.ps1`：改跑 `--test selfhost_checker`；
    输出签名 `STEP_0215_OK ... lexer=integrated-retired ... retired-by=STEP-0278`。
  - `validate-step-0216.ps1` / `validate-step-0219.ps1`：同模式。
- **canary suite 列表同步**：`validate-step-0243/0244/0245.ps1` 摘除
  `--test selfhost_declaration_parser`。
- 历史 STEP 文档（0182/0183/0184/0191/0215/0216/0219）与
  `docs/reports/m22-interim-audit.md` 保持原样——它们是当时事实的记录，
  退役事实由本 STEP 登记。

## 3. Validation（本机实测）

- 引用清扫：全仓库 `--include=*.ps1/*.rs/*.sico` 已无 legacy 目标活引用
  （唯一 grep 命中为 `crates/sico-automation-host/tests/corpus.rs` 注释里
  的英文短语 "single-use tokens."，误报）。
- 六个改写/更新的 `.ps1` 全部通过 PSParser 语法检查。
- 集成链自洽：`sico check` 对 `compiler.sico`（根）/ `checker.sico` /
  `formatter.sico` 全 ok——退役后源集语法 + 语义无破损。
- 本机不可运行（已知宿主限制，非本片引入）：runner 测试仓构建需
  gcc/dlltool（ring/windows-sys），`validate-step-0221.ps1` 等含 cargo
  的校验器本机必红，**215 语料全量差分与 canary 以 CI 裁决**（与
  STEP-0272/0276 同惯例）。
- `validate-step-0124.ps1`、`validate-module-boundaries.ps1`、
  `git diff --check` 绿。

## 4. Exit test 对照（W1 item 1/3）

- legacy 三文件与三单测不存在 ✅；全仓库无 `use lexer.|use tokens.|
  use declaration_parser.` 引用 ✅；历史锚（corpus-v0.json + STEP 文档）
  在位（改写后校验器守卫）✅；sha 门逐入口断言存在且随 CI 每跑 ✅；
  215 语料全量绿 = CI 裁决项（本机不可执行，如实登记）。

## 5. Follow-ups

- 片 C（item 2）：E7002 关键词指纹改结构性（`compiler_semantics.sico:582-590`）
  + rename-safety 语料，下一片。
- 片 D（item 4）：0-indent 语料先行定 `same(raw, line)` 行为，再定
  refused/structural。
- W1 整体出口（评估 §6 step 3）：四片全绿 + canary 重钉后，W1 关闭，
  进 W2（R0 = Option A records 重授权层）。
