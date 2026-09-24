# STEP-0280: W1 片 D — 0-indent 语料先行 + W1 关闭

> - status: complete（语料先行定行为 + 方向决策 + 计数同步已落地；runner
>   测试仓本机不可重建——gcc/dlltool 缺失——集成链以 CI 裁决）
> - phase: M22 compiler self-host — W1 consolidation（[`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md) §5 item 4；计划 = STEP-0277）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机：oracle 对照 + checker 组件 235 条全量探针 235/235；CI：集成链全量）

## 1. Objective

按 STEP-0277 §5 片 D：0-indent corpus case 先行，实测 Rust oracle 与
guest 对「函数体内 0 缩进行」的行为，分歧再定 refused / structural 方向。
本片为零代码改动片（guest 行为不动，语料钉行为）。

## 2. 实测行为（probe 先行，全部本机实测）

| 形态 | Rust oracle | guest（改前=改后，本片未动代码） |
| --- | --- | --- |
| `return 42` 顶格（IND-001） | accept | accept ✅ |
| 缩进行 + 顶格 `let` + 缩进行（IND-002） | accept | accept ✅ |
| 函数体内顶格 `record X:`（IND-003） | E1001（recovery 声明，函数未闭） | E1001 ✅ |
| 顶格 `return stdout.write("x")` + `capabilities: none`（IND-004） | 拒（E2031/E4001/E4002，清单序首 = E2031） | **accept（跳过该行语义）** ❌ |
| 顶格 `return Model(text: loaded.text, ...)`（IND-005） | E7002 | **accept（跳过该行语义）** ❌ |

根因（STEP-0277 §2.4 实位确认）：guest parser/semantics 以
`same(raw, line)`（0-indent）区分「声明行 vs 语句行」，函数体内的顶格
非声明语句被静默跳过——不分析、不拒绝。oracle 则把缩进当惯例（check 层
不强制），顶格语句照常参与语义分析。

## 3. 方向决策：structural（登记，W2 执行）

- **拒绝对方向（typed refusal）被否**：guest 无法区分「oracle 会接受的
  顶格行」（IND-001/002，guest 现正确 accept）与「oracle 会拒绝的顶格行」
  （IND-004/005）——不做分析就分不出，做分析就是 structural 本身。
  一律拒绝会把当前正确的 2 条 accept 也打成 diverge，且凭空造一个
  oracle 没有的诊断码，违反 identity-level 契约。
- **structural 方向登记为 W2 内容**：guest parser 状态机 + semantics
  第二遍把「函数体内顶格非声明行」按语句处理（声明关键字行仍走 E1001
  recovery 优先）。与 records 重授权层同波次动 guest 架构，避免 W1
  合并债片越权扩大。
- 片 D 交付 = 行为钉死（5 条语料）+ 2 条 diverge 显式豁免（同 0279
  waiver 机制，必须保持 diverge 防腐烂）。

## 4. Changes

- 新语料 5 条（`syntax-candidates/b/indentation/`）：
  - `valid/zero-indent-return.sico`（IND-001）、
    `valid/zero-indent-mid-statement.sico`（IND-002）——strict accepted；
  - `invalid/zero-indent-recovery-declaration.sico`（IND-003）——strict
    E1001；
  - `invalid/zero-indent-undeclared-capability.sico`（IND-004，
    oracle 首码 E2031）、`invalid/zero-indent-stale-result.sico`
    （IND-005，oracle E7002）——**diverge 豁免**（guest 现 accept），
    W2 structural 修复后移出豁免。
- 语料重冻结 230 → 235（既有 230 条零变动，逐路径比对）。
- `bootstrap_bundle.rs`：entry_count 230 → 235。
- `selfhost_checker.rs`：条目数 230 → 235；`RECORD_DIVERGENCE_WAIVER` 与
  `INDENT_DIVERGENCE_WAIVER` 合并为 `DIVERGENCE_WAIVER`（10 条，注释按
  组标注关闭责任波次）；分区断言 (116, 36, 70, 8) → **(116, 37, 72, 10)**
  （+1 E1001 identity、+2 accepted、+2 waived unsupported）。

## 5. Validation（本机实测）

- oracle 期望 5 条全部 `sico check --json` 实取（§2 表）。
- **checker 组件 235 条全量探针：235/235 pass**（10 条豁免按 diverge
  语义核对；预构建 runner CLI，8 路并行，约 202 s）。
- `validate-step-0124.ps1`、`git diff --check` 绿。
- 本机不可运行（已知宿主限制）：runner 测试仓 cargo 构建（gcc/dlltool），
  集成链 235 条全量 + canary 以 CI 裁决（同 STEP-0272/0276/0278/0279 惯例）。

## 6. Exit test 对照（W1 item 4 + W1 整体出口）

- 0-indent 行为钉死：5 条语料（3 strict + 2 waived）✅；
  分歧方向决策登记（structural @ W2，拒绝对方向否決记录）✅；
  235 全量绿（本机探针）✅。
- **W1 整体关闭**（评估 §6 step 3）：item 1 legacy 退役 ✅（0278）、
  item 2 E7002 结构性 ✅（0279）、item 3 sha 门 ✅（0278）、
  item 4 缩进代理语料先行 ✅（本片）；215+ 语料全量绿（235 条，含
  rename-safety / 0-indent）✅；legacy 三文件与三单测不存在 ✅；
  全仓库无 `use lexer.|use tokens.|use declaration_parser.` ✅。
  canary 重钉属 CI 裁决项，如实登记。

## 7. Follow-ups

- **W2**（下一波次，R3 止损计数从 W2 起算）：guest records 重授权层 +
  S3/S4 re-baseline；同波次关闭 10 条 diverge 豁免（8 条 records +
  2 条 indent——indent 的 structural 修复见 §3）。
- 候选债（沿用 0279 登记，本片无新增）：关键字参数名 check/build 缝隙；
  guest `words()` 词对近似。
