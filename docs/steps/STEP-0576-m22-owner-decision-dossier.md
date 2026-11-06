# STEP-0576: M22 owner 裁决卷宗 — 重复 STEP 编号与登记异常处置包

> - status: report-complete（只读审计 + 新报告；处置执行待 owner 拍板）
> - phase: M22 文档治理
> - date: 2026-11-06
> - owners: autonomous-agent
> - branch: `codex/m22-w2` @ `a939862`
> - deliverables: [`docs/reports/m22-owner-decisions-dossier-2026-11-06.md`](../reports/m22-owner-decisions-dossier-2026-11-06.md)
> - discipline: 只读 + 新报告；不改任何既有文件（含 `docs/steps/README.md`）；不翻门、不改 gate 文本/证据等级；M22 保持 NO-GO；不做完成声明

## 1. 性质

AGENTS §3 要求非平凡执行步唯一编号。STEP-0573 实测 `docs/steps/README.md` 存在 **20 组同号多行**与 **5 行未转义 `|`**；重编号属 owner 裁决。本片产出一份「一页可决」卷宗，把处置选项、`文件:行` 定位与理由摆全，**只出卷宗不执行**（改 README/STATUS 待 owner 拍板后另行成片）。

## 2. 实测结论（全部来自实际执行的只读命令）

- README 表 `[STEP-xxxx]` 行 **481** 条、唯一编号 **461** 个 ⇒ **20 组同号**。
- **19 组为同一文件的双语重复行**（0051–0061 共 11 组、0111–0118 共 8 组）：每组两行链接逐字节相同、一行英文标题一行中文标题，纯翻译无信息增量 ⇒ 建议**合并去重**，不需重编号。其中 0058/0059/0061 三组两行**状态措辞不一致**（cross-check vs complete、blocked vs NO-GO），合并时以文件 H1 实际裁决为准、不得借机改 gate。
- **1 组为两份不同文件共用一号**（0175）：`STEP-0175-language-v1-batch2.md`（权威，覆盖 RFC-0046 四项决策）与 `STEP-0175-language-v1-batch2-for-loops-error-propagation.md`（for-loops/error-propagation 子集早期切片）⇒ 建议**保留前者占 0175、后者重编号为新的唯一号**（具体新号属 owner 裁决，方向 = 保持 M22 S0 邻近段）。
- **5 行管道符异常**（0285/0328/0502/0520/0539）：四行是标题代码跨内未转义 `|`（`` `||` `` ×2、`` `variant ok|error` ``、`` `Result[I64|U64,…]` ``）⇒ 建议 `\|` 转义；**0539 性质不同**——缺收尾管道符（4 个管道、行尾无 `|`，截断行）⇒ 建议补收尾 `|`。
- **0440–0485 悬空引用未消除**：`docs/STATUS.md` 有 60 行引用该号段，但 `docs/steps/` 目录 **46/46 无对应文件**、README 无登记（README 该号段仅 0486–0489）。STATUS 中真正的链接只有 0486/0487/0488，0440–0485 全为叙述性文字引用 ⇒ 与 STEP-0570:51 登记一致，STEP-0573 的 117 行补登未触及（这些号在目录中无文件、无法补登）。建议 **STATUS/handoff 加注**，不补登、不删史、不回收号。

## 3. 出口

卷宗交付 = [`docs/reports/m22-owner-decisions-dossier-2026-11-06.md`](../reports/m22-owner-decisions-dossier-2026-11-06.md)（含 §1 逐组对账、§2 0175 专项、§3 逐行修复建议、§4 悬空引用复核、§5 汇总建议、§6 命令日志、§7 残余风险）。本片零既有文件改动。

## 4. 诚实收窄

- **不执行任何处置**：README/STATUS 编辑、0175 重编号均待 owner 拍板后另片落地；本片只读。
- **0175 新号未定死**（避免与并行蜂群取号冲突）；**A 类三组状态措辞冲突**未逐个打开 11 个 M5/M6 文件核对 H1（超出「一页可决」范围，建议执行片合并前逐个 `head`）。
- **未跑 cargo/clippy/测试/validate-step-0124**：纯文档只读审计 + 新报告，不触碰编译代码/gate/契约/fixture，按 AGENTS §8 不机械跑全工作区；只复跑只读扫描与 git 检查。
- **Markdown 渲染效果未截图取证**：以管道符计数 + GFM 表格语义推断串列风险，可靠但非渲染器实测。
- **0440–0485 跨分支历史存在性未独立复核**：仅在当前分支工作树核实「无文件」，未逐分支搜索；STEP-0570 的「跨分支历史」断言如实转述、未独立验证。

## 5. 命令与证据

`git log --oneline -3`（0，确认 `a939862`）、`wc -l docs/steps/README.md`（0，493）、`node target/tmp/m22-dossier-scan.js`（0，`DUP GROUPS (20)`、`rows=481 unique=461`）、`node target/tmp/m22-pipes.js`（0，5 行异常：L321=7/L364=7/L440=6/L480=4/L487=6）、`node -e pipeCount/endsWithPipe`（0，L480=4/false 证实 0539 缺收尾）、`head -20`/`ls -la` 两份 0175（0，H1/status/artifacts 实测）、`grep -oP '^\|\s*\[STEP-04[4-8]\d\]' README`（0，仅 0486–0489）、`for 440..485 ls`（0，`missing=46/46`）、`grep -c 'STEP-04[4-8][0-9]' STATUS`（0，60）、`grep -oP '\[STEP-04[4-8]\d\]\(…\)' STATUS`（0，仅 0486/0487/0488）、`grep -n 0440 STEP-0570`（0，`:51`）。全部退出码 0，无失败命令。

**M22 保持 NO-GO**；本片不推进任何出口门。