# STEP-0573: steps README registry backfill (登记欠账补齐)

> - 状态：complete（docs/steps/README.md 登记补齐 + 机械校验全绿）
> - 阶段：M22 文档卫生批次（承 STEP-0570 的登记欠账注记）
> - started: 2026-11-06
> - owners: m22-bootstrap sub-agent (steps-README writer)

## 1. 任务与范围

STEP-0570 实测发现 README 表只登记到 STEP-0061 区段 + 0562–0570 批次，0325–0561 之间大量已存在 STEP 文件未入表。本片补齐登记欠账。**唯一写入文件 = `docs/steps/README.md`**；不动任何 STEP 文件本身，不动 STATUS/handoff/plans/ROADMAP/rfc/adr（评审者专属）。

## 2. 未登记清单来源（集合差，实测）

命令（Git Bash，`docs/steps/` 下）：

```
ls docs/steps/*.md | grep -v '^README'        # → 460 个 STEP 文件（含两份 0175）
ls *.md | grep -v '^README' | sed 's/^STEP-0*\([0-9]*\)-.*/\1/' | sort -n | uniq   # 唯一号段枚举
grep -oP '\(\./\KSTEP-\d+-[^)]+' README.md | sort -u                              # 表已有链接目标
comm -23 <磁盘文件> <表链接>                                                              # 集合差
```

实测结果：

- 磁盘 STEP 文件 = **460**（459 个唯一 STEP 号，因 **0175 有两份文件**）。
- 补登前 README 表已有链接目标 = **343**（唯一 342）。
- 集合差 = **117 个未登记文件**（= 459 − 342；两份 0175 都未登记）。
- 与 STEP-0570「约 110 个」的估计一致，精确值为 117。

未登记文件按号段分布：

| 号段 | 个数 | 归属 |
|---|---|---|
| 0131–0141 | 11 | M14 gap-closing / acceptance oracles / M14 exit |
| 0142–0150 | 9 | M15 kickoff/modules、M16/M17 contract、M15 consumer、M16 synthetic adapter |
| 0155/0156/0158/0159/0160/0162 | 6 | M15 web host、M16 Windows real path、M15/M16 exit audits、M17 session audit、M15 UI renderer |
| 0163–0168 | 6 | M18–M20 planning、M18 portfolio、M18 exit、M17 re-audit、M19 execution |
| 0169–0174 | 6 | M20 language v1 batch 1、M20 exit、M21 planning、M19 pre-gate seam、M22 planning、stdlib batch 2 |
| 0175（两份）/0176/0177 | 4 | M21 language v1 batch 2、list monomorphs、M21 exit |
| 0178–0194 | 17 | M22 S1/S1.5/S2/S3/S4/S4→S5 缺陷链/S5/S6 早期片 |
| 0342–0351 | 10 | M22 W2 S3/S4 lowering 片 |
| 0354–0365 | 12 | M22 W2 S4 lowering / 诊断 / 回退片 |
| 0370/0382–0391 | 11 | M22 W2 S4 lowering / 诊断片 |
| 0394–0408 | 13 | M22 W2 S4 lowering / 容量攻坚片 |
| 0410–0423 | 8 | M22 W2 S4 深嵌手术 / RFC 预算表修订片 |
| 0555/0556/0558/0559/0560/0561 | 6 | M22 W2 B3/B4 负结果、W4 修复与 parity、发射词表 C 补刀 |

## 3. 补登规则（推断，非编造）

- **表格格式**：`| [STEP-xxxx](./文件) | 状态 | 阶段 | 标题 |`，沿用既有列序与分隔行。
- **插入位置**：三张表均为**降序**排列，故按号段插到正确降序锚点之前——0131–0194 插在 `STEP-0195` 行前（表 1）、0342–0423 插在 `STEP-0352` 行前（表 2）、0555–0561 插在 `STEP-0547` 行前（表 3）。
- **标题**：取各文件**首行 H1**（`# ` 之后），去掉 `STEP-xxxx` 前缀冗余后作为标题列；不逐文件深读正文（工时纪律）。
- **状态列**：从文件头部「状态 / status / 片型 / exit」行推断——
  - 明确 `complete` → `complete`；
  - `implementation complete locally / independent CI pending`（中文行「本机全量回归…independent CI pending」）→ 原样保留为 `implementation complete locally / independent CI pending`；
  - `reverted` / `negative result` / `diagnostic in progress` → 如实标为 `reverted（…）` / `negative result …` / `diagnostic in progress`，**不美化**；
  - audit 文件带 verdict 的（如 0158/0159/0165/0166/0170/0177）→ 标 `complete (audit verdict …)`，保留真实 verdict（GO / NO-GO / 部分）；
  - planning-only 文件 → `complete (planning)`；
  - **无明确状态行的标 `recorded`**（仅 0423，见 §5），**不编造 complete**。
- **阶段列**：从头部「里程碑 / phase」行推断——`里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）` → `M22 W2 S3/S4`；`phase: M14 (…)` → `M14`；`phase: M22 S4→S5` → `M22 S4→S5`；推断不了的标 `?`（本批未出现，全部可从头部行或标题推断）。
- **保留既有欠账注记**但更新数字（见 README 第 5 行），并写明集合差口径与两份 0175 的处理。

## 4. 实测结果（补登后）

| 校验项 | 命令 | 结果 |
|---|---|---|
| 磁盘文件 ↔ 表链接全覆盖 | `_coverage.py`（`os.listdir` vs 表内 `./STEP-*.md` 链接集合差） | files on disk **460** / unique links **460** / registered-but-missing **0** / on-disk-but-unregistered **0** |
| 链接目标存在 + 列数 + 空列 | `_verify.py`（逐行正则 + `os.path.isfile`） | BAD **0**（新增行）；仅 5 行**既有**行因单元格内含未转义 `|` 被判列数异常（见 §6，非本片引入，未改动） |
| 重复行 | `_verify.py`（按链接/按号聚合） | 同号多行 = 19 组**既有**行（0051–0061、0111–0118 各 2 行 + 0175 两份文件 2 行）；**新增行之间无重复**。同号两行**内容不同**（不同 milestone 语境的不同 status/title，见 §6），故保留 |
| 标题 ↔ H1 抽查 | `_spotcheck.py`，抽查 **30** 行 | **28** 行标题与文件 H1 逐字一致（归一化后子串匹配）；2 行为已知特例（0370 前缀差异、0423 H1 缺失），见 §5 |
| 仓库校验器 | `powershell -File tools/validate-step-0124.ps1` | `STEP_0124_OK …`，**EXIT=0** |
| 行尾/空白 | `git diff --check docs/steps/README.md` | **EXIT=0** |
| 编码/行尾 | `file` + 字节统计 | UTF-8 无 BOM、**CRLF=0**、结尾 LF |

补登行数：脚本插入 **116 行** + 手工补 1 行（0175 第二份文件）= **共 117 行**。README 相对 HEAD `+128` 行 = 本片 117 行 + 上一批已在工作树的 0562–0570 批次 9 行 + 欠账注记 1 行（`git diff` 无删除行）。

## 5. 两处抽查特例（如实登记）

- **STEP-0370**：标题列 = `gw 机器入口区 match 支持（gw_match_open_pack entry 门摘除），STATEMENT 拒绝类清零`，H1 = `STEP-0370 — M22: gw 机器入口区 match 支持（…）`。两者正文逐字一致，仅 H1 带编号/里程碑前缀（本批统一去掉该前缀）→ 抽查脚本的朴素子串判断误报，人工核对为一致。
- **STEP-0423**：文件 H1 是 `# Sico project status`（该文件实为**会话状态日志**，无真正的步骤 H1，首部只有 `> - updated: …` 追加行）。按纪律**不编造状态**，标为 `recorded`，标题列写明「文件为会话状态日志，H1 缺失」。

## 6. 残留（不属本片范围，登记给评审者）

- **既有同号多行 19 组**：0051–0061（11 组）、0111–0118（8 组）在表中各出现 2 行，**内容不同**（如 STEP-0061 一行为 `complete-audit / blocked`、另一行为 `complete / NO-GO`；STEP-0111 一行为 `complete-contract`、另一行为 `complete / RFC-0037 accepted`）——是不同 milestone 语境的历史登记，非本片引入，本片**未删改**（删除会动他人内容、且会丢失某一语境的 status/title）。0175 的两行对应磁盘上**两份真实文件**，均登记。
- **既有 5 行列数异常**：STEP-0285 / 0328 / 0502 / 0520 / 0539 的单元格内含未转义 `|`（如 `**register** as **draft**（卡 23-B）| …`），使机械列数计数 ≠ 4。**既有内容，本片未改动**；渲染通常正常，但严格的 Markdown 表格解析可能串列。留给评审者决定是否转义。
- 本片为**纯文档登记**，不改任何门文本/证据等级，M22 保持 NO-GO，不做完成声明。未跑全 workspace 构建/测试（文档改动不影响编译产物）；已跑本批相关校验器 `validate-step-0124.ps1`（EXIT=0）与 `git diff --check`（EXIT=0）。