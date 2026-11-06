# M22 owner 裁决卷宗 · 重复 STEP 编号与登记异常处置包（2026-11-06）

- STEP: STEP-0576
- 日期: 2026-11-06
- 分支: `codex/m22-w2`，HEAD `a939862`
- 性质: **只读审计 + 新报告**。本卷宗不改任何既有文件（含 `docs/steps/README.md`）；处置执行待 owner 拍板后另行成片。
- 纪律: 不翻门、不改 gate 文本/证据等级；M22 保持 NO-GO；不做完成声明。
- 方法: 全部结论来自实际执行的只读命令（见 §6 命令日志）。扫描脚本 `target/tmp/m22-dossier-scan.js` / `target/tmp/m22-pipes.js`（本片新建的临时工具，不在交付物内、不影响编译）。

---

## 0. 摘要（一页可决）

| 项 | 实测 | 建议处置 | 风险 |
|---|---|---|---|
| README 表同号多行 | **20 组**（0051–0061 ×11、0111–0118 ×8、0175 ×1） | 见 §1/§2；19 组为同文件双语重复行（**合并/去重**），0175 为两份不同文件共用一号（**重编号**） | Markdown 表渲染出重复条目；STEP 编号唯一性（AGENTS §3）被破坏 |
| 未转义 `\|` 行 | **5 行**（0285 / 0328 / 0502 / 0520 / 0539） | 见 §3 逐行修复建议；其中 **0539 是缺收尾管道符**（截断行），与其余四行（内容内未转义管道）性质不同 | 表列错位，串列到相邻行 |
| 0440–0485 悬空引用 | **未消除**：46/46 号在 `docs/STATUS.md` 有引用、`docs/steps/` 无文件、README 无登记 | 见 §4；属跨分支历史引用，建议 STATUS 侧加注而非补登 | 引用指向不存在的步骤记录 |

表内总行数（`docs/steps/README.md`）= 481 条 `[STEP-xxxx]` 表行，唯一编号 461 个 ⇒ 20 个编号各出现两次。

---

## 1. 20 组同号多行 · 逐组对账

### 1.1 A 类：同一文件的双语重复行（19 组：0051–0061、0111–0118）

**性质**：每组两行指向**同一个** `docs/steps/STEP-xxxx-*.md` 文件（链接逐字节相同），一行是英文标题/状态、一行是中文标题/状态。内容差异 = 纯翻译，无信息增量。这类是早期（M5/M6/M12）建表时同一记录被登记两次的重复行，**不是两个独立步骤**。

对每组的两行（号 / 行号 / 链接 / 标题 / 阶段）对账如下。链接列全部相同 ⇒ 差异仅在标题语言与状态措辞。

| 组号 | 行 A（英文） | 行 B（中文） | 链接相同 | 内容差异性质 | 建议处置 + 理由 |
|---|---|---|---|---|---|
| 0051 | L31 | L82 | 是 | `Windows install/open/uninstall, native UI and package smoke` vs `集成 Windows Desktop Host`；状态同 `complete`、阶段同 `M5` | **合并**：保留一行（建议留中文行 L82，与全表中文标题口径一致），删英文行 L31。两行同指一个文件、无独立内容 |
| 0052 | L30 | L83 | 是 | `Windows/macOS/Linux adapter artifacts and parity evidence` vs `固定 desktop platform adapters/parity` | 同上 **合并**，留 L83 删 L30 |
| 0053 | L29 | L84 | 是 | `representative app, properties, startup baseline and M5 GO` vs `完成 M5 quality/exit audit` | **合并**，留 L84 删 L29 |
| 0054 | L28 | L85 | 是 | `Android threat/lifecycle/platform/packaging contract` vs `冻结 Android Host contract` | **合并**，留 L85 删 L28 |
| 0055 | L27 | L86 | 是 | `shared mobile Host core and typed JNI boundary` vs `实现 Mobile Host/JNI boundary contract` | **合并**，留 L86 删 L27 |
| 0056 | L26 | L87 | 是 | `content URI package/open/share/deep-link adapter` vs `实现 Android Intent/package ingestion contract` | **合并**，留 L87 删 L26 |
| 0057 | L25 | L88 | 是 | `Android permission, URI grant and app-private storage mapping` vs `映射 Android permission/storage` | **合并**，留 L88 删 L25 |
| 0058 | L24 | L89 | 是 | 状态不同：A=`complete-cross-check`，B=`complete`；标题 `Wasmtime Android cross-check and lifecycle supervision` vs `冻结 Android Runtime/lifecycle contract` | **合并**：留 L89，但状态措辞两行不一致（cross-check vs complete）⇒ 合并时取**更保守**的 `complete-cross-check` 或以文件 H1 为准；这是 19 组里唯一状态措辞冲突的一组，需 owner 确认取哪个 |
| 0059 | L23 | L90 | 是 | 状态不同：A=`complete-cross-check`，B=`complete`；`Android native touch, IME and accessibility adapter` vs `适配 Android native UI contract` | **合并**：留 L90，状态同 0058 取保守值 |
| 0060 | L22 | L91 | 是 | 状态同 `partial-runtime-evidence`；`same signed package Desktop/Mobile parity; Android runner pending` vs `验证 Desktop/Mobile core parity；Android runner deferred` | **合并**，留 L91 删 L22 |
| 0061 | L21 | L92 | 是 | 状态不同：A=`complete-audit / blocked`，B=`complete / NO-GO`；`security, performance and NO-GO Android runner audit` vs `完成 M6 host audit，保留外部 runner 阻塞` | **合并**：留 L92，状态两行措辞不同（blocked vs NO-GO）⇒ 合并时取文件 H1 实际裁决；第二组状态冲突，需 owner 确认 |
| 0111 | L141 | L152 | 是 | 状态不同：A=`complete-contract`，B=`complete`；标题粒度不同：A=`冻结 secure HTTP provider authority/transport contract`，B=`RFC-0037 accepted：http@0.2.0 身份、endpoint authority、rustls 选型（含本机 roundtrip 证据）、redirect 矩阵、secret 模型、全部 bounds`（B 明显更详尽） | **合并**：留信息更全的 L152，删 L141；B 行是 M12 HTTP 实际实现记录，A 行是较早的 contract 摘要 |
| 0112 | L142 | L153 | 是 | A=`成熟 TLS stack、证书与 hostname 验证`；B=`sico-http-provider（第 27 包）：确定性 rcgen CA + rustls 服务器/验证客户端；…`（B 更详尽，是真实 crate 落地记录） | **合并**，留 L153 删 L142 |
| 0113 | L143 | L154 | 是 | A=`endpoint authority、DNS pinning 与地址策略`；B=`严格 authority 规范化（IPv4 文本欺骗/IPv4-mapped/zone/大小写/percent 全拒绝）、IDNA 只认 canonical ASCII、…` | **合并**，留 L154 删 L143 |
| 0114 | L144 | L155 | 是 | A=`strict framing、bounded streaming 与 backpressure`；B=`严格传输分帧：重复/混合/非规范 Content-Length 拒绝、chunked 读取器…` | **合并**，留 L155 删 L144 |
| 0115 | L145 | L156 | 是 | A=`redirect reauthorization 与跨 origin secret stripping`；B=`opt-in ≤5 跳 redirect 引擎：冻结状态码矩阵、同 URL 环检测…` | **合并**，留 L156 删 L145 |
| 0116 | L146 | L157 | 是 | A=`Host-owned secret provider 与全链路 redaction`；B=`Host 侧 opaque secret 注册表：name+endpoint+policy 精确三元的 typed 拒绝…` | **合并**，留 L157 删 L146 |
| 0117 | L147 | L158 | 是 | 状态同 `complete-core`；A=`per-Store HTTP engine 与 connection lifecycle SDK core`；B=`per-Store HttpEngine：authority→pinning→地址门→grant 检查→TLS/明文交换→redirect 链全组合…` | **合并**，留 L158 删 L147 |
| 0118 | L148 | L159 | 是 | 状态同 `complete / GO-core`；A=`secure HTTP core audit；guest runner integration 与 Linux provider parity 待完成`；B=`10 项 exit gate：1-6/8 GO-core（合同+五层全部冻结并测试）；7 partial…` | **合并**，留 L159 删 L148（B 为 exit-audit 全量记录） |

**A 类统一建议**：19 组全部 **合并去重**（每组删 1 行、留 1 行），不需要重编号（编号本身唯一、只是行重复）。三组（0058 / 0059 / 0061）状态措辞两行不一致，合并时以文件 H1 的实际裁决为准，且 **不得**借机改任何 gate 文本或证据等级——只统一措辞、不改变结论。

### 1.2 B 类：两份不同文件共用一号（1 组：0175）

见 §2 专项。

---

## 2. STEP-0175 双文件同号专项

- 行 A：`docs/steps/README.md:191` → `./STEP-0175-language-v1-batch2.md`
  - 标题：`Language v1 batch 2 — for-loops, error propagation, numeric list elements (RFC-0046)`
  - 阶段 `M21 / M22 S0`，状态 `complete`
- 行 B：`docs/steps/README.md:192` → `./STEP-0175-language-v1-batch2-for-loops-error-propagation.md`
  - 标题：`Language v1 batch 2 — for-loops, error propagation, bare-literal record（与上一行同号两文件，均登记）`
  - 阶段 `M21 / M22 S0`，状态 `complete`

两份文件均存在（实测 `ls`，同日期 2026-11-04，5330 B / 4390 B）。两份头部实测对比：

| 维度 | `STEP-0175-language-v1-batch2.md` | `STEP-0175-language-v1-batch2-for-loops-error-propagation.md` |
|---|---|---|
| H1 | `... numeric list elements (RFC-0046)` | `... bare-literal record` |
| status | complete | complete |
| phase | M21 §3.2 / M22 S0（RFC-0046 accepted 2026-09-14） | M21 §3.2 / M22 S0（RFC-0046 accepted 2026-09-14） |
| completed | 2026-09-14 | 2026-09-14 |
| artifacts | RFC-0046 + `sico-{lexer,ir,semantics,codegen-wasm}` + 4 个 e2e 语料 + `language_batch2.rs` + `application-profile-v0.json` + `validate-step-0131.ps1` | RFC-0046 + `sico-{lexer,hir,parser,ir,semantics}` + `lower.rs` + `for-loop-words.sico` + `for_loop.rs` + `error_propagation.rs` |
| 正文覆盖 | 四项 RFC-0046 决策端到端（for-loops 全 iterable 集 / error propagation / numeric list / bare-literal record） | 仅 for-loops 降级细节 + error propagation |

**孰为权威判定**：两份都是同一天（2026-09-14）对同一 RFC-0046 的**真实执行记录**，但**覆盖面不同**：
- `batch2.md`（A）是**更完整的 batch-2 总览**（标题含 `numeric list elements`，artifacts 覆盖 4 个 e2e 语料 + codegen-wasm + 校验器，正文明确写「The four RFC-0046 decisions are executable end to end」）。
- `...-for-loops-error-propagation.md`（B）是**较早/较窄的子集记录**（artifacts 无 codegen-wasm、无 numeric list 语料，正文只讲 for-loops + error propagation 两项）。

⇒ **权威 = `STEP-0175-language-v1-batch2.md`（A）**：它是覆盖四项决策的完整 batch-2 记录，B 是其 for-loops/error-propagation 子集的早期切片。

**建议处置 = 保留 A 占用 STEP-0175，将 B 重编号为新的唯一号（建议 STEP-0175 → B 改为 STEP-0195 之后的下一个空号；实测 0195 已占用，见下方取号说明）**：
- A 文件名与 README 行 191 保持 `STEP-0175` 不变（它是权威完整记录）。
- B 文件重命名为 `STEP-0<新号>-language-v1-batch2-for-loops-error-propagation.md`，同步改文件内 H1 与 status 块、README 行 192 的链接与号。
- **取号约束**：B 的历史定位是 M21/M22 S0 早期片，重编号应落在 **M22 S0 邻近号段**以保持时序可读；但 0176–0194 段已被占用（实测 README 已登记到 0194，且 0195–0213 段也已占用）。⇒ 建议 B 取 **M22 早期段之后的第一个空号**，或由 owner 直接指定一个明确的历史保留号（例如 `STEP-0175b` 不符合 `STEP-xxxx` 数字格式，不建议）。**最终取号属 owner 裁决**，本卷宗只给出「重编号 + 保持 M22 S0 邻近段」的方向，不擅自定死具体数字以免与并行蜂群取号冲突。
- 理由：AGENTS §3 要求非平凡执行步唯一编号；两份文件是两个不同的 artifact 集与正文，不是翻译重复，不能像 A 类那样合并去重。

---

## 3. 5 行未转义 `\|` · 逐行修复建议

背景：Markdown 表格单元格内的 `|` 会切断列，必须写成 `\|`；即便在反引号代码跨内，GFM 表格解析仍按 `|` 分列，故代码跨内也须转义。下列 5 行实测管道符计数偏离正常值 5（`| Step | 状态 | 阶段 | 标题 |` = 5 个管道符）。

| 文件:行 | STEP | 实测管道符数 | 缺陷性质 | 修复建议（转义后的正确文本片段） |
|---|---|---|---|---|
| `docs/steps/README.md:321` | 0285 | 7（多 2） | 标题内 `` `||`/`!` `` 未转义 | 把 `` `||` `` 改为 `` `\|\|` `` ⇒ 全片断为 ``……` `\|\|`/`!` 零实测消费被排除……`` |
| `docs/steps/README.md:364` | 0328 | 7（多 2） | 标题内 `` `||`/`!` `` 未转义 | 同上：`` `||` `` → `` `\|\|` `` ⇒ ``……` `\|\|`/`!` 出 v0）……`` |
| `docs/steps/README.md:440` | 0502 | 6（多 1） | 标题内 `` `variant ok|error` `` 未转义 | `` `variant ok\|error` `` |
| `docs/steps/README.md:487` | 0520 | 6（多 1） | 标题内 `` `Result[I64|U64, NumericError]` `` 未转义 | `` `Result[I64\|U64, NumericError]` `` |
| `docs/steps/README.md:480` | 0539 | 4（少 1，且行尾无 `\|`） | **缺收尾管道符（截断行）**——与上四行性质不同 | 行尾 `……**M22 仍为 NO-GO**）` 后**补 ` \|`**（补回表格收尾管道符）。⚠️ 这一行不是「未转义管道」而是「缺管道」，修复动作是**补收尾符**而非转义 |

**建议处置**：4 行做 `\|` 转义、1 行（0539）补收尾 `|`；全部为纯 Markdown 转义/补符，**不改任何标题文字、状态、阶段或链接**。修复后全表每行管道符数应恒为 5（可用 §6 的 `m22-pipes.js` 复核，期望输出 `rows with unescaped-pipe cell count != 5: 0`）。

---

## 4. STEP-0440–0485 悬空引用现状复核

**结论：STEP-0573 之后未消除，46/46 全部仍为悬空。**

实测证据：
- `docs/STATUS.md` 中匹配 `STEP-04[4-8]\d` 的行共 **60** 行，覆盖 0440–0489 全号段（多为 changelog 的「updated: …（STEP-04xx …）」叙述引用）。
- 其中**真正的 Markdown 链接**只有 3 条且全部指向 **0486/0487/0488**（这三个文件存在、README 也已登记）：`[STEP-0486](./steps/STEP-0486-m22-w2-s5-codegen-wave.md)`、`[STEP-0487]…`、`[STEP-0488]…`。
- **0440–0485 号段**：`docs/steps/` 目录**逐个检查 46/46 无对应文件**（`STEP-0440`…`STEP-0485` 全部 NO FILE）；`docs/steps/README.md` 中匹配 `STEP-04[4-8]\d` 的登记行**只有 0486–0489，无任何 0440–0485**。
- ⇒ STATUS 对 0440–0485 的引用是**叙述性文字引用**（非链接），指向的步骤记录文件在当前分支不存在、README 也不登记。STEP-0570 已在 `docs/steps/STEP-0570-m22-doc-hygiene-audit.md:51` 如实登记该现象（原文：「STEP-0440–0485 号段在 STATUS 有引用但 steps/README 均无（跨分支历史）」）。**STEP-0573 的 117 行补登未触及该号段**（0573 补的是目录中已存在但未登记的文件；0440–0485 在目录中根本不存在，无法补登）。

**建议处置**：
1. **不在 README 补登**（无文件可链，补了就是死链）。
2. **不在 STATUS 删历史 changelog**（改写历史 changelog 违反「不篡改历史记录」纪律）。
3. 建议 owner 在 STATUS 相关 changelog 段落或 handoff-m22 加一条**加注**：说明 0440–0485 是跨分支历史号段、其记录文件不在当前分支、以 STATUS 叙述为唯一留痕——这与 STEP-0570 的登记一致，属登记异常的诚实留痕而非补造记录。
4. **不预留/不回收这些号**：它们是历史消费过的号，回收会造成新的编号歧义。

---

## 5. 汇总建议（供 owner 拍板）

| 序 | 事项 | 处置类型 | 是否需重编号 | 是否改 gate |
|---|---|---|---|---|
| 1 | 0051–0061、0111–0118 共 19 组双语重复行 | README **合并去重**（每组删 1 行） | 否 | 否（仅统一措辞，不触证据等级） |
| 2 | 0175 双文件同号 | B 文件 **重编号**，A 保留 0175 | **是**（B 取新唯一号，段位 owner 定） | 否 |
| 3 | 0285/0328/0502/0520 未转义管道 | README **`\|` 转义** | 否 | 否 |
| 4 | 0539 缺收尾管道 | README **补 `\|`** | 否 | 否 |
| 5 | 0440–0485 悬空引用 | STATUS/handoff **加注**，不补登不删史 | 否 | 否 |

执行注意（供后续成片）：
- 项 1/3/4 是纯 README 行级编辑，逐组删/改后须复跑 §6 的两个扫描脚本确认 dup=0（除 0175 保留的 1 组待项 2 处理）、pipe 异常=0。
- 项 2 重编号须同时改：文件名、文件内 H1、文件内 status 块、README 行 192 的链接与号；并复跑 `git diff --check`。
- 全部处置**不得**触碰 gate 文本、证据等级、STATUS 首条权威记录的结论；M22 保持 NO-GO。

---

## 6. 命令日志（实际执行）

| 命令 | 退出码 | 关键输出 |
|---|---|---|
| `git log --oneline -3` | 0 | `a939862 docs(m22): renumber W4 verdict-parity STEP-0557 -> STEP-0561 …`（确认 HEAD） |
| `git status --short` | 0 | 前两批未提交成果在列（未触碰） |
| `wc -l docs/steps/README.md` | 0 | `493 docs/steps/README.md` |
| `node target/tmp/m22-dossier-scan.js` | 0 | `=== DUP GROUPS (20) ===`；`total table rows=481 unique nums=461`；逐组打印 20 组两行的行号/链接/标题/阶段 |
| `node target/tmp/m22-pipes.js` | 0 | `rows with unescaped-pipe cell count != 5: 5` → L321 pipes=7、L364=7、L440=6、L480=4、L487=6 |
| `node -e "…pipeCount/endsWithPipe…"`（对 321/364/440/480/487） | 0 | L321=7/true、L364=7/true、L440=6/true、**L480=4/false**、L487=6/true（确认 0539 缺收尾管道） |
| `head -20` 两份 STEP-0175 文件 | 0 | 两份 H1/status/completed/artifacts 实测（见 §2 表） |
| `ls -la` 两份 STEP-0175 文件 | 0 | 5330 B / 4390 B，同日期 |
| `grep -oP '^\|\s*\[STEP-04[4-8]\d\]' docs/steps/README.md \| sort -u` | 0 | 仅 0486/0487/0488/0489 ⇒ 无 0440–0485 登记行 |
| `for n in 440..485; ls docs/steps/\| grep ^STEP-0$n` | 0 | `missing=46 / 46` ⇒ 0440–0485 全无文件 |
| `grep -c 'STEP-04[4-8][0-9]' docs/STATUS.md` | 0 | `60` |
| `grep -oP '\[STEP-04[4-8]\d\]\([^)]*\)' docs/STATUS.md \| sort -u` | 0 | 仅 0486/0487/0488 三条链接 ⇒ 0440–0485 全为叙述引用 |
| `grep -n '0440\|悬空' docs/steps/STEP-0570-m22-doc-hygiene-audit.md` | 0 | `:51` 登记「STEP-0440–0485 号段在 STATUS 有引用但 steps/README 均无（跨分支历史）」 |

**未运行**：`cargo`/`clippy`/测试、`validate-step-0124.ps1` 等。本片为纯文档只读审计 + 新增报告，不触碰任何编译代码、gate、契约、fixture，故按 AGENTS §8「窄文档改动不机械跑全工作区」执行，仅复跑只读扫描与 `git` 检查。若 owner 拍板后落地 README/STATUS 编辑，应在那一成片里补跑 `git diff --check` 与相应校验器。

---

## 7. 残余风险 / 未决

1. **0175 重编号的具体新号未定死**：为避免与并行蜂群取号冲突，本卷宗只给方向（保持 M22 S0 邻近段）不锁定数字；需 owner 指定。
2. **A 类三组状态措辞冲突（0058/0059/0061）**：合并去重时取哪个状态需以文件 H1 实际裁决为准，本卷宗未逐个打开这 11 个 M5/M6 文件核对 H1（超出本片「一页可决」范围）；建议执行片合并前逐个 `head` 确认，**不得**借合并改 gate 措辞。
3. **未验证 Markdown 渲染实际效果**：本片以管道符计数与 GFM 表格规则推断串列风险，未在渲染器中截图取证；结论基于 GFM 表格语义，可靠但非渲染器实测。
4. **0440–0485 是否在其他分支有文件**：本片仅在当前分支 `codex/m22-w2` 工作树核实「无文件」；未跨全部分支搜索其历史存在性（STEP-0570 称「跨分支历史」，本片未独立复核该断言，如实登记）。