# STEP-0277: W1 合并债偿还计划登记（legacy lexer/parser 合并 + 指纹规则 + sha 门 + 缩进代理）

> - status: complete（计划登记 + 实测侦察；实现分片随后按 §5 顺序落地，各自带 CI 裁决）
> - phase: M22 compiler self-host — W1 consolidation（[`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md) §5/§6）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: planning-only（实测引用盘点，无可执行声明）

## 1. Objective

把评估文档 §5「两种 R0 选项下都要偿还的合并债」items 1–4 登记为可执行的
W1 计划：机械性、语料守卫、**不改 lowering 语义**；suite + canary 必须保持绿。
前置（WIP 吸收）已由 STEP-0276 清空，本 STEP 是 W1 的第一刀。

## 2. 实测侦察（2026-09-24 工作树，本 STEP 新测）

### 2.1 Item 1 — 一个 lexer、一个 declaration parser（legacy 三件套退役）

- **引用面实测**：`lexer.sico`(117) / `tokens.sico`(459) / `declaration_parser.sico`(371)
  在 selfhost 源层**零 import**（grep `use lexer.|use tokens.|use
  declaration_parser.` 无命中）；只被三份各自的单测引用：
  `selfhost_lexer.rs`(1 test) / `selfhost_tokens.rs`(1 test) /
  `selfhost_declaration_parser.rs`(1 test)，三者都跑 `corpus-v0.json` 的
  215 语料子面。
- **校验器引用面**：`validate-step-0215.ps1`（钉 tokens.sico + selfhost_tokens.rs
  + 跑该测试）、`validate-step-0216.ps1` / `validate-step-0219.ps1`（钉
  declaration_parser.sico）、`validate-step-0245.ps1`（suite 列表含
  `--test selfhost_declaration_parser`）。集成链校验器（0221/0242）用的是
  `compiler_lexer.sico`，不受退役影响。
- **缺陷分支实证**：`tokens.sico:431-435`——`b==60`(`<`) 且 next 非 `=`
  时 kind/finish 均不赋值（裸 `<` 漏 lex），评估所述缺陷分支属实，随退役消灭。
- **覆盖衔接**：集成链 `selfhost_checker.rs` 已跑全量 215 语料
  （116 lexical / 34 identity / 65 accepted / 0 unsupported，STEP-0265
  登记）；退役丢失的仅 legacy 单测的三条**单位级**断言面（lossless 字段
  kind/start/end/text、module AST 对拍），由集成链的整拍断言 + 语料 sha 冻结
  接替——评估 P2「不维护两个 lexer」的裁决优先于单位级断言面保留。

### 2.2 Item 2 — 指纹规则（compiler_semantics.sico E7001/E7002）

- **E7001**（:576-580）：`commit` 实参计数 < 2 → 已是**结构性**检查
  （`missing_two_arguments`），非指纹，无需动。
- **E7002**（:582-590）：关键词指纹——`at(parts,1)=="Model"` +
  `word_after(parts,"text")=="loaded"` + `word_after(parts,"revision")=="model"`
  + `revision_guard==0` → E7002。按语料路径改写为结构性检查（revision
  绑定状态的变量级跟踪，owner 函数形参名重命名后不再漏检/误检），或连同
  rename-safety 语料登记为 declared-subset 限制——实现片按改写优先、
  语料守卫定夺。

### 2.3 Item 3 — selfhost_checker.rs sha 门

- **实测已存在**：`selfhost_checker.rs:159-168` 对**每个**语料入口断言
  `bytes` + `source_sha256`（`Sha256::digest`，sha2 导入在 :5），模式与
  `bootstrap_bundle.rs:189` 一致；`git log` 显示该门随 STEP-0246 批
  （0eaccf4）落地。评估 §5 item 3（review P0-5 follow-up）**已被后续 STEP
  实质完成**。→ W1 本片仅验证留证 + 登记关闭，无代码改动。

### 2.4 Item 4 — 缩进即语义代理（same(raw, line)）

- 实位：`compiler_parser.sico:489`（`syntax_missing_function_colon`）与
  :650（declaration/语句判别主循环）——`raw`（原始行）== `line`（trim 后）
  充当「declaration 从 0 缩进开始」的代理；:235 用
  `sico.text.leading_spaces` 计算偏移，说明链内已知缩进信息可取得。
- 债：0-indent 语料用例或 declared refusal。实现片先补 0-indent 语料
  case（corpus-v0.json 增条目 + checker 期望），若集成链行为与 Rust oracle
  分歧再定 refused/structural 方向——**语料先行定行为**，不先改代码。

## 3. Scope

包含：本计划登记；§2 实测侦察留档；索引（steps/plans 指针）与 STATUS
同步；`validate-step-0124.ps1` + `git diff --check`。
不含：任何代码/语料/测试改动（实现按 §5 分片，各片带 CI 裁决留档）。

## 4. 决定

- 实现顺序按 §5（先验证关闭 item 3 → item 1 退役 → item 2 E7002 → item 4
  语料先行），每片独立可回退；item 1 的文件删除与校验器改写同片提交，
  避免「文件已删、校验器还钉」的中间红态进 CI。
- 单位级断言面退役（§2.1）是评估已接受的代价，不再单独评审。

## 5. 实现分片计划（各片 = 独立落地 + suite/canary 绿 + CI 裁决）

1. **片 A（item 3 关闭）**：纯文档/验证——重跑 sha 断言存在性 + 登记
   STEP 关闭。零风险。
2. **片 B（item 1 退役）**：删 `selfhost/lexer.sico|tokens.sico|
   declaration_parser.sico` + 三份单测；改写 0215/0216/0219 为「退役登记」
   校验器（改跑集成链等价测试或固定文档存在性 + 退役注释）；0245 suite
   列表摘除 selfhost_declaration_parser；canary（0245/0261/0221）绿。
3. **片 C（item 2 E7002）**：compiler_semantics.sico 指纹改结构性；
   增 rename-safety 语料（owner 形参改名后 E7002 仍触发）；215 全量
   期望不变（除新语料）。
4. **片 D（item 4 缩进代理）**：0-indent corpus case 先行，行为分歧再定
   refused/structural；canary 重钉。

出口测试（W1 整体）：215+ 语料全量绿（含新增 rename-safety / 0-indent
条目）、canary 校验器全绿、legacy 三文件与三单测不存在、全仓库无
`use lexer.|use tokens.|use declaration_parser.` 引用。

## 6. Risks

- **本机跑不了 runner 测试仓**（ring 需 gcc）：片 B/C/D 的 suite 断言以 CI
  裁决（与 STEP-0272 同惯例）；片间保持「单中间红态不进 CI」。
- **历史校验器改写**：0215/0216/0219 是已完成 STEP 的钉；改写为退役登记
  时保留原输出签名语义（`STEP_0xxx_OK` 行改注明退役指向），run-ci 清单
  同步核对。
- **E7002 行为回归**：rename-safety 语料先钉期望再动语义；若结构性改写
  超出演变预算，退登记 declared-subset 限制（评估允许的 fallback）。
