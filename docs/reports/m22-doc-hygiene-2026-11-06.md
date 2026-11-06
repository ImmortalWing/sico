# M22 文档/工件卫生审计（STEP-0570 · 只读审计片 · 2026-11-06）

- 分支 `codex/m22-w2`，审计锚点 HEAD = `a9398627e51c21af90b019b0e7caae6615f30d73`。
- 片性质：**只读对账**。本片不创建任何被审计对象、不修改任何既有文件、不改 gate 文本与证据等级、不翻门；M22 保持 **NO-GO**。
- 本片只新建本文件与 `docs/steps/STEP-0570-m22-doc-hygiene-audit.md`。
- 重要前提更正：派单写「工作树干净」，实测**不干净**（并发 STEP-0565/0566 片在审计窗口内写入了工作树，见 §D）。本报告一律以实测 `git status` 为准，并逐条标注证据来源与时间。

---

## A. AGENTS.md §10.1.2 标 `[x]` 的「分层策略落地」所列两个工具

### A.1 结论（与派单假设相反）

派单假设「`tools/validate-m22-s5-dev.ps1` 与 `tools/m22-s5-fingerprint.ps1` 实测不存在」。**实测：两个文件在审计时的工作树里都存在且可运行**；真正的 discrepancy 是**它们是未跟踪文件（untracked），不在 HEAD `a939862` 里**，而 AGENTS.md 已在 `a939862` 上把这项标成 `[x]`（完成）。即「完成声明先于版本化落地」，与 §10.2「可复现验证器必须版本化」的口径冲突。

| 断言位置 | 断言 | 实测 |
|---|---|---|
| `AGENTS.md:336`（`- [x]` 分层策略落地） | `tools/validate-m22-s5-dev.ps1`（`-Set fast\|probes\|firmware\|all`）已入库 | 文件存在（13992 B，mtime 2026-11-06 17:40）；`git cat-file -e HEAD:tools/validate-m22-s5-dev.ps1` ⇒ `fatal: ... exists on disk, but not in 'HEAD'`；`git ls-files` 该路径为空 ⇒ **untracked** |
| `AGENTS.md:336` | `tools/m22-s5-fingerprint.ps1`（`target`/`__pycache__`/`*.pyc` 三排除）已入库 | 文件存在（6542 B，mtime 2026-11-06 17:38）；同样 **untracked**，不在 HEAD |
| `AGENTS.md:176`（§10.1.2 表格） | 开发期子集工具 = `tools/validate-m22-s5-dev.ps1` | 同上：存在但未跟踪 |
| `AGENTS.md:179` | 续跑指纹用 `tools/m22-s5-fingerprint.ps1` | 同上：存在但未跟踪 |

### A.2 工具内容与断言一致性（已读文件正文）

- `tools/validate-m22-s5-dev.ps1:3` 自述 `M22 S5 development-tier battery (STEP-0565, owner decision AGENTS.md 10.1.2)`；`:10-13` 四档 `fast ~1 min / probes ~12 min / firmware ~28 min / all ~2.7 h`，与 `AGENTS.md:176` 的时长标注一致；`:33` 指纹排除项 = `target` / `__pycache__` / `*.pyc`，与 `AGENTS.md:179` 一致。
- 实跑 `-List`（命令见 STEP 文档）退出码 0，列出 `fast 6 steps` / `probes 3 steps` / `firmware 3 steps` / `all 15 steps`，与「同一命令、同一顺序、只改节奏」的自述一致。
- 工具头部把 owner 决策归到 **STEP-0565**；`docs/steps/` 内**无 `STEP-0565-*.md` 执行记录**（实测 `ls docs/steps/ | grep 056` ⇒ 只有 0560/0561/0566），`docs/STATUS.md` 也没有 STEP-0565 条目 ⇒ **执行记录悬空**。

### A.3 建议处置（本片不执行）

1. 由 STEP-0565 的 owner 片把两个工具与 `docs/steps/STEP-0565-*.md` 一起提交，使 `AGENTS.md:336` 的 `[x]` 有版本化支撑；在那之前 `[x]` 应降级表述为「已落地于工作树、待入库」，或补 `docs/steps/` 记录。
2. 本片**不创建、不提交**这两个文件（遵守派单「只做对账登记」）。

---

## B. AGENTS.md §10.10 / §10.11 与 STATUS 最新条目的不一致

### B.1 §10.10 已完成清单滞后于 STATUS（4 条已完成工作未登记）

`AGENTS.md` §10.10 末条 = **STEP-0551（B2 值索引，2026-10-28）**。`docs/STATUS.md` 顶部 2026-11-05/06 的 6 条 `> - updated:` 已登记了 4 项新的已完成/已测量工作，§10.10 一条未收：

| STATUS 条目 | 主题 | §10.10 是否登记 |
|---|---|---|
| `STATUS.md:3`（2026-11-06） | STEP-0560 后续收口：注释纪律 RED→GREEN（`selfhost/parser.sico:11364-11365` 措辞改写）+ MS-225 ledger 行号位移收口（`LEDGER_CURRENT sites=396`） | 未登记 |
| `STATUS.md:4`（2026-11-06） | STEP-0560 W4 发射词表 C 补刀：`sico.list.length[U64]` 完整 collection-helper 发射（codegen +97 行，新钉 1048 B 逐字节等 oracle） | 未登记 |
| `STATUS.md:5`（2026-11-05） | STEP-0558 W4 声音洞修复：record 构造缺字段必拒（`ERR:E-SH-IR-CONSTRUCT-FIELD`，parity 146→147/150） | 未登记 |
| `STATUS.md:6-8`（2026-11-05） | STEP-0561 / STEP-0556 / STEP-0555：W4 verdict parity 146/150、B4-C 线性装配负结果、B3/B4 三刀负结果 | 未登记 |

附带：`docs/handoff-m22.md:3-8` 已有 STEP-0559/0558/0561/0556/0555/0560 的条目，**但缺 `STATUS.md:3` 的「STEP-0560 后续收口（注释纪律 + MS-225 ledger）」那条** ⇒ handoff 与 STATUS 顶部也不完全对齐。

### B.2 §10.11 未完成清单里 3 条已被实测推翻（checkbox 仍为 `[ ]`）

| 位置 | §10.11 断言（`[ ]`） | 实测证据 | 判读 |
|---|---|---|---|
| `AGENTS.md:338` | `emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` 仍是 `da0fec5f…`，当前 `codegen.sico` 是 `3d8705b1…` ⇒ `--check` 必判 `EMITTER_VOCABULARY_RED` | `sha256sum selfhost/codegen.sico` = `013f2d7e894994b3734646c6c9996c14e2e0d999a7fd37a37ef3ac5dc0854d0f`；`selfhost/emitter-vocabulary-v1.json` 内已有 `"source_sha256": "013f2d7e…"` ⇒ **指纹已刷新、两者一致** | **断言过期**：该 `[ ]` 项实质已闭合（`STATUS.md:4` 的 STEP-0560 `--update` 刷新可佐证）。注意本片**未能**实跑 `census-m22-emitter-vocabulary.py --check` 复核 `EMITTER_VOCABULARY_GREEN`，原因见 STEP 文档「未跑项」（本机 `python` 不可用） |
| `AGENTS.md:339` | 主树脏文件：`selfhost/codegen.sico`（step0545 修复副本）、`selfhost_compiler.rs`（旧草稿，`fmt (runner)` 红，权威版 436 行）、`AGENTS.md §10`、`tools/` 两个新工具未提交 | ①实测 `git status` 中**无** `selfhost/codegen.sico`、`selfhost_compiler.rs`、`AGENTS.md` 的改动；②`runner/sico-runner/tests/selfhost_compiler.rs` = **12284 行**（不是 436 行版，但已随 `a939862` 提交，非脏文件）；③`cargo fmt --all -- --check` 退出码 **0** ⇒ `fmt` 并不红；④`tools/` 两个新工具确实仍未入库（见 §A） | **大部分过期**：脏文件状态已随提交消失，只剩「`tools/` 两个新工具未入库」这半句仍成立 |
| `AGENTS.md:337` | 可续跑封装器仍只在 `target/tmp/`，未版本化到 `tools/` | `tools/validate-m22-s5-dev.ps1` 已实现逐档状态文件（`target/tmp/m22-s5-dev/<set>.state.json`）+ 指纹闸门 + PID 锁（工具正文 `:24-31`），但**文件本身 untracked** ⇒ 「未版本化」这半句仍成立，成因从「没写」变成「没提交」 | **部分成立**，建议改写指向 §A 的入库动作 |

### B.3 数字口径一致性抽查（§10.10/§10.11 vs 当前实测）

- 组件轴：`AGENTS.md:201` 写「组件轴 36/396」，与 `STATUS.md:5` 的「组件轴 36/396」一致 ✔。
- 既有 runner 红：`AGENTS.md` §10.11 未逐条列；`STATUS.md:3` 列 `step0507/0519/0520/0526`。本片用预构建测试二进制实跑这 4 个测试 ⇒ **4 failed / 187 filtered out**，与 STATUS 一致 ✔（命令与输出见 STEP 文档）。
- 测试总数漂移：STATUS（STEP-0558 条）写 `182 passed / 4 failed`，当前二进制 `--list` 计 **191** 个测试 ⇒ 其间新增约 5 个测试，STATUS 未更新总数（轻微，非错误）。
- 8 MiB 结果上限行号：`AGENTS.md:203` 引 `runner/sico-runner/src/lib.rs:3599`，实测该行确为 `"malformed result: {name} exceeds 8 MiB"` ✔ 行号仍准。

---

## C. `tools/validate-step-0124.ps1` 校验的登记（steps README 表）与 `docs/steps` 实际文件对账

### C.1 校验器实跑

`powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0124.ps1` ⇒ 退出码 **0**，输出 `STEP_0124_OK milestones=M14-M26 steps=unreserved layers=...,compiler-self-host,...`。该脚本只检查「ROADMAP 里引用的 `STEP-0125`–`STEP-0299` 号必须有 `docs/steps/` 执行记录」⇒ **它不校验 steps README 表本身**，所以 README 表与实际文件的漂移不归它管（这正是本项审计的价值）。

### C.2 总量对账

- `docs/steps/` 实际 `STEP-*.md` 文件 = **452**（去重后的唯一 STEP 号 = 451，因 `STEP-0175` 有两个文件）。
- `docs/steps/README.md` 表内引用的唯一 STEP 号 = **339**。
- ⇒ **至少 112 个已存在的 STEP 文件未在 README 表登记**（含 M22 相关的 STEP-0555/0556/0558/0559/0560/0561/0566，以及 STEP-0175/0355/0404 等）。
- README 表引用但文件缺失 = **0**（逐条 `](*.md)` 相对链接存在性检查，无 MISSING）。STATUS 里引用的 05xx 号全部有文件。

### C.3 抽查：22 个 STEP 号的存在性与标题对账

| STEP | 文件存在 | 文件内 H1（截断） | README 表登记 | 标题一致 |
|---|---|---|---|---|
| 0103 | **否** | — | 仅在散文「STEP-0103–0110 完成 M11」提及，非表行 | n/a（非表行，无悬空链接） |
| 0175 | 是（**两个文件**：`STEP-0175-language-v1-batch2.md`、`STEP-0175-language-v1-batch2-for-loops-error-propagation.md`） | `# STEP-0175: Language v1 batch 2 — for-loops, error propagation, bare-literal record` | 未登记 | n/a |
| 0279 | 是 | `# STEP-0279: M14–M26 milestone gate audit and route synchronization` | 是（:242，phase M14–M26 governance） | 一致 ✔ |
| 0280 | 是 | `# STEP-0280: M22 W1 C/D — E7002 结构检查与零缩进差分` | 是（:243） | 一致 ✔ |
| 0281 | 是 | `# STEP-0281: M22–M26 单步执行卡与过时 M22 队列修正` | 是（:244） | 一致 ✔ |
| 0282 | 是 | `# STEP-0282: M22 W2 测量基线冻结（执行卡 22-B）` | 是（:245） | 一致 ✔ |
| 0283 | 是 | `# STEP-0283: M22 W1 前置裁决——本机全量 CI 绿 + 校验器 BOM 修复` | 是（:246） | 一致 ✔ |
| 0355 | 是 | `# STEP-0355 — M22: list.get match 机器泛型 [U64] 支持…` | **未登记** | n/a |
| 0404 | 是 | `# STEP-0404 — M22: 顶层化手术续——fall-through 展开 + elif 链抽辅助…` | **未登记** | n/a |
| 0486 | 是 | `# STEP-0486 m22-w2 S5 component-codegen wave opening` | 是（:299） | 一致 ✔ |
| 0534 | 是 | `# STEP-0534 m22-w2 S5 W2 族 match 构造的真值测量片（拍 1）…` | 是（:354） | 一致 ✔ |
| 0535 | 是 | `# STEP-0535 m22-w2 S5 W2 族 match 构造的组件层发射刀（M-g4a）…` | 是（:353） | 一致 ✔ |
| 0544 | 是 | `# STEP-0544 · M22 W2 S5 · fuel 归因测量片…` | 是（:344） | 一致 ✔ |
| 0547 | 是 | `# STEP-0547 · M22 W2 · 步骤全量审查与执行重排片…` | 是（:339） | 一致 ✔ |
| 0551 | 是 | `# STEP-0551 m22-w2 B2 值索引片…` | 是（:340） | 一致 ✔ |
| 0555 | 是 | `# STEP-0555 m22-w2 B3 负结果：游标化与模块索引未回绿…` | **未登记** | n/a |
| 0556 | 是 | `# STEP-0556 m22-w2 B4 候选 C 负结果：线性装配…` | **未登记** | n/a |
| 0558 | 是 | `# STEP-0558 m22-w2 W4 声音洞修复：record 构造缺字段必拒…` | **未登记** | n/a |
| 0559 | 是 | `# STEP-0559 m22-w2 CALL-SHAPE 根因收窄…` | **未登记** | n/a |
| 0560 | 是 | `# STEP-0560 · M22 W2 · 发射词表 C 补刀：sico.list.length[U64]…` | **未登记** | n/a |
| 0561 | 是 | `# STEP-0561 m22-w2 W4 拒绝语料 verdict parity（前端 150 项）` | **未登记**（且正文 H1/内文仍自称 STEP-0561，见 §E） | n/a |
| 0566 | 是 | `# STEP-0566 — M22 CI 触发面修复（.github/workflows/m22-w1-ci.yml）` | **未登记** | n/a |

- **11 个被登记的 STEP（0279/0280/0281/0282/0283/0486/0534/0535/0544/0547/0551）标题与文件 H1 全部一致**；**11 个抽查号（含 6 个 M22 W4 相关号）未在 README 登记**。
- 另发现一处**结构性空洞**（非本片职责、仅登记）：`docs/STATUS.md` 引用了 STEP-0440–STEP-0485 共约 46 个号（如 `STATUS.md:124` 的 STEP-0448），但 `docs/steps/` 内 **STEP-0440–0485 号段没有任何文件**（该号段第一个文件是 STEP-0486）；README 表同样从 0486 起跳 ⇒ STATUS 有条目、steps 无执行记录、README 表无行，三处一致地缺这一段。这与 §E 的跨分支编号冲突同源（该号段内容在 `origin/dev` 一线）。

### C.4 建议处置

1. 给 README 表补 6 个 M22 W4 行（0555/0556/0558/0559/0560/0561）+ 0566 行——由评审者统一维护（本片不改）。
2. 对 STATUS 引用但 `docs/steps/` 无文件的号段（0440–0485 等）做一次一次性「并集登记 or 标注为跨分支历史」，呼应 `docs/reports/m22-review-2026-10-26.md:91` 的 G3 结论（STEP-0279–0283 重复编号需一次性并集登记）。

---

## D. 工作树 untracked/dirty 清单与候选卫生风险

派单称「工作树干净」。实测（`git status --porcelain --untracked-files=all`，审计时点）**共 9 条**，HEAD 仍 = `a939862`（未前进）：

| 路径 | 状态 | mtime | 归属（据文件自述/内容） | 是否该进候选提交 |
|---|---|---|---|---|
| `.github/workflows/m22-w1-ci.yml` | M（+24/−） | 11-06 17:35 | STEP-0566「CI 触发面修复」 | 属 STEP-0566 片，应与其 STEP 文档**同片提交**，不应混入他人候选 |
| `selfhost/match-component-coverage-v1.json` | M | 11-06 17:28 | STEP-0566 伴随工件 | 同上 |
| `tools/probe-m22-corpus-parity.ps1` | M（+234/−） | 11-06 17:36 | STEP-0566（探针扩展） | 同上 |
| `docs/reports/m22-ci-trigger-2026-11-06.md` | ?? | 11-06 17:34 | STEP-0566 报告 | 同上 |
| `docs/steps/STEP-0566-m22-ci-workflow-trigger.md` | ?? | 11-06 17:36 | STEP-0566 记录 | 同上 |
| `selfhost/corpus-parity-map-v1.json` | ?? | 11-06 17:34 | STEP-0566 工件 | 同上 |
| `tools/m22-s5-fingerprint.ps1` | ?? | 11-06 17:38 | **STEP-0565**（§A） | 属 STEP-0565 片，须与 `docs/steps/STEP-0565-*.md` 同片提交 |
| `tools/validate-m22-s5-dev.ps1` | ?? | 11-06 17:40 | **STEP-0565**（§A） | 同上 |
| `tools/profile-m22-fuel-wall.ps1` | ?? | — | STEP-0565/0566 伴随探针 | 同上，提交前需确认归属片 |

**卫生风险（候选提交）**：

1. **跨片混提风险**：工作树同时含 STEP-0565 与 STEP-0566 两片的产物；若某片收口时 `git add -A`，会把另一片的半成品（尤其 `tools/profile-m22-fuel-wall.ps1` 无 STEP 记录）一起带进候选。建议每片只 `git add` 自己清单内的路径。
2. **`[x]` 先于版本化**：`AGENTS.md:336` 的 `[x]` 依赖的 3 个 untracked 工具（§A + `profile-m22-fuel-wall.ps1`）若最终被丢弃，§10.1.2 表格与 `:336` 都会成为悬空引用。
3. **工件目录**：`target/tmp/` 下有 12 项（`step0562`、`step0566_new_workflow.yml`、`step0566_old_workflow.yml`、`step0566_validate_workflow.py`、`b3`、`ledger-*.log`、`ms225-rerun.log`、`mcv1.prefix.json` 等）。`target/` 已被忽略 ⇒ 不会进候选，但 §10.11 `AGENTS.md:337` 要求「可续跑封装器必须版本化」⇒ 其中被 STEP 文档引用为证据的脚本应择机提升到 `tools/`（本片不执行）。
4. 本片自己的两个新文件（`docs/reports/m22-doc-hygiene-2026-11-06.md`、`docs/steps/STEP-0570-m22-doc-hygiene-audit.md`）是唯一应随 STEP-0570 提交的内容。

本片未跑 `tools/run-ci.ps1`（会重编译 runner，成本高且与只读审计无关）；已跑 `git diff --check`（退出码 0）与 `cargo fmt --all -- --check`（退出码 0）。

---

## E. 重复 STEP 编号现状（0279–0283、0557/0561）

### E.1 0279–0283：**分支间重复，本分支内唯一**

- 本分支（`codex/m22-w2`，HEAD `a939862`）内 `docs/steps/` 对 0279–0283 各有**恰好一个**文件，README 表 242–246 行各登记一行，**文件名与 H1 一致**（见 §C.3 表）⇒ 本分支内无重复。
- 重复来自**跨分支**：`docs/handoff-m22.md:11` 明写「`origin/dev` 已前进至 `550059a`，包含另一套 STEP-0279–0283 W1/W2 历史，与本分支源码及 STEP 编号冲突。不得覆盖或隐式合并」；`docs/reports/m22-review-2026-10-26.md:91`（G3）也要求「STEP-0279–0283 的重复编号需一次性并集登记」；`docs/steps/STEP-0292:13`、`STEP-0293:72` 都有同样的冲突声明。
- 处置建议：维持「不得覆盖或隐式合并」，并在评审者侧做一次并集登记（哪一套是权威、另一套标注为 `origin/dev` 历史）。本片不改。

### E.2 0557/0561：**已由 `a939862` 重编号，但工作树内遗留 3 类不一致**

- `git log` 证实 `a939862` = `docs(m22): renumber W4 verdict-parity STEP-0557 -> STEP-0561 (collision with m24 picker STEP-0557)` ⇒ **0557 → 0561 的重编号已在本分支落地**，起因是与 M24 轨的 `STEP-0557-m24-native-file-open-save-picker.md` 撞号（后者在 `codex/m24-vision-ui-v2` 一线，`git log --all` 可见两套 `docs/steps/STEP-0557-*.md`）。
- 本分支 `docs/steps/` 内：`STEP-0557-*` 文件**已不存在**，`STEP-0561-m22-w2-w4-verdict-parity.md` 存在 ⇒ 重命名完成 ✔。
- **遗留不一致（本片登记，不改）**：
  1. `docs/steps/STEP-0561-m22-w2-w4-verdict-parity.md:1` 的 H1 = `# STEP-0561 m22-w2 W4 拒绝语料 verdict parity（前端 150 项）`，但 `:73` 的表格仍写「W4 verdict parity（前端 150 项） | **本片** | STEP-0561」⇒ 自指 OK；但文件内**没有任何地方记录「本文件原名 STEP-0557、因与 M24 撞号而重编号」**⇒ 重编号历史只存在于 commit message，未落入 STEP 文档或 README。
  2. `docs/plans/` 内 grep `0557` **零命中** ⇒ 计划文档没有登记这次重编号；记录重编号的两个 commit `9f49ef2`（plans 登记）与 `9344bde`（gitcode origin 归档删除记录）经 `git merge-base --is-ancestor` 实测**不是 HEAD 的祖先** ⇒ 本分支落后于那两个收尾 commit。
  3. `tools/probe-m22-corpus-parity.ps1:6` 仍写「Extends the STEP-0561 verdict-parity probe」——这一处与重编号一致 ✔；`STATUS.md:6`、`handoff-m22.md:6`、`STEP-0558:5/10/12/46` 引用的都是 STEP-0561 ✔ 无残留 0557 引用（全仓 grep `STEP-0557` 在 `docs/`、`tools/`、`AGENTS.md` 内零命中）。
- 处置建议：把「原号 STEP-0557 → 现号 STEP-0561（与 M24 picker 撞号）」补进 `docs/steps/STEP-0561-*.md` 的编号说明段（多数 M22 STEP 文档都有「编号说明」小节，如 STEP-0491），并在 README 表补 0561 行时一并注明。

### E.3 另发现的同号文件重复（本分支内）

- `STEP-0175` 有两个文件：`STEP-0175-language-v1-batch2.md`（5330 B）与 `STEP-0175-language-v1-batch2-for-loops-error-propagation.md`（4390 B），两者都存在且 mtime 相同（Nov 4 15:47），**README 表未登记 0175** ⇒ 无法从 README 判断哪个是权威。建议评审者择一保留或合并，并补登记。

---

## F. 汇总（discrepancy 清单速查）

| # | 类型 | 定位 | 一句话结论 | 建议处置 |
|---|---|---|---|---|
| A | 完成声明先于版本化 | `AGENTS.md:336` / `:176` / `:179` | 两个工具实测存在于工作树但 untracked，不在 HEAD `a939862` | 由 STEP-0565 片连同其 STEP 记录入库；此前 `[x]` 表述应加「待入库」限定 |
| A | 执行记录悬空 | `tools/validate-m22-s5-dev.ps1:3` 引 STEP-0565 | `docs/steps/` 无 `STEP-0565-*.md`，STATUS 无该条 | 补 STEP-0565 执行记录 |
| B1 | §10.10 滞后 | `AGENTS.md` §10.10 末条 = STEP-0551 | STATUS 顶部 4 项已完成工作（0555/0556/0558/0560 + 0560 收口）未进 §10.10 | 评审者补 §10.10 条目 |
| B1 | handoff 缺条 | `docs/handoff-m22.md:3-8` | 缺 `STATUS.md:3` 的「STEP-0560 后续收口」条 | 评审者补 |
| B2 | §10.11 断言过期 | `AGENTS.md:338` | 词表 `source_sha256` 已与当前 `codegen.sico`（`013f2d7e…`）一致，`[ ]` 描述的必红前提已消失 | 评审者关闭该项（并注明本片未实跑 `--check`，见 STEP 文档未跑项） |
| B2 | §10.11 断言过期 | `AGENTS.md:339` | codegen/selfhost_compiler/AGENTS 均非脏文件、`fmt` 绿（退出码 0），只剩「`tools/` 两工具未入库」半句成立 | 评审者收窄该项到 §A 的入库动作 |
| B2 | §10.11 部分成立 | `AGENTS.md:337` | 可续跑封装器已写成工具但 untracked ⇒ 「未版本化」仍成立，成因变化 | 与 §A 合并处置 |
| C2 | 登记缺口 | `docs/steps/README.md` 表 | 452 个文件 / 339 个表行 ⇒ 约 112 个文件未登记（含 6 个 M22 W4 号 + 0566） | 评审者补表行 |
| C3 | 跨分支空洞 | `docs/steps/` STEP-0440–0485 号段 | STATUS 引用该号段但 steps 无文件、README 表无行（该号段首个文件为 0486） | 一次性并集登记 or 标注跨分支历史（呼应 m22-review G3） |
| D1 | 跨片混提风险 | 工作树 9 条改动（STEP-0565 + STEP-0566 两片并存） | `git add -A` 会把他人半成品带进候选 | 每片只 add 自己清单内路径 |
| D3 | 工件未版本化 | `target/tmp/` 12 项（step0562/step0566_*/ledger-*） | 被引用为证据的脚本仍在 `target/tmp/` | 择机提升到 `tools/` |
| E1 | 跨分支重复编号 | STEP-0279–0283（本分支 vs `origin/dev`） | 本分支内唯一，跨分支两套历史 | 维持「不得覆盖/隐式合并」，并做并集登记 |
| E2 | 重编号未落文档 | `a939862`；`STEP-0561-*.md:1/:73`；`docs/plans/` | 0557→0561 只在 commit message，STEP 文档/plans 无记录；记录它的两个 commit 不是 HEAD 祖先 | 在 0561 文档补「编号说明」，并追平 `9f49ef2`/`9344bde` |
| E3 | 同号两文件 | `docs/steps/STEP-0175-*.md` ×2 | 两个 0175 文件并存且 README 未登记 0175 | 择一保留或合并并补登记 |

## G. 明确未跑 / 未做（诚实收窄）

- 未实跑 `tools/census-m22-emitter-vocabulary.py --check`（本机 `python`/`python3`/`py` 均不可用，只有 Microsoft Store 别名）⇒ §B2 对 `AGENTS.md:338` 的推翻是**基于 SHA256 逐字节比对**（`sha256sum selfhost/codegen.sico` vs JSON 内 `source_sha256`），不是基于该 census 的实际 GREEN 输出。
- 未实跑 `tools/validate-m22-s5-dev.ps1 -Set fast`（其 6 步里有 `python` 步，必失败于本机环境）⇒ 只跑了 `-List` 验证工具可用性与档位时长。
- 未重编译 runner、未跑 `tools/run-ci.ps1`、未跑完整 `selfhost_compiler` 套件（只跑 4 个已知红测试 = 10 s）。
- 未修改任何被审计文件、未创建被审计工具、未 commit / push。