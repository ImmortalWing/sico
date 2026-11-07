# STEP-0579 · M22 W2 · match 密度分账（STEP-0575 §4.4 延迟判据执行 + 文档治理只读评估）

> **片型** = **只读测量 + 文档片**（诊断）。`selfhost/*`、`runner/*`、`crates/*`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`crates/sico-ui-renderer-d2d` **零改动**。
> **唯一产物** = 本 STEP 文档（`docs/steps/STEP-0579-m22-match-density-ledger-accounting.md`）。
> **权威判据** = STEP-0575 §4.4「落地主刀前必跑的 match-密度分账」。
> **账本** = `selfhost/match-component-coverage-v1.json`（396 站点）——**只读，未改**（账本无 `match_density` 字段，本片用派生代理口径，见 §2）。
> **门状态** = 全部维持；M22 仍 **NO-GO**；本片**零门翻转、零生产代码改动、零限额变更、零账本改动、零既有文件改动**。

## 0. 一句话结论（可执行裁决）

**STEP-0575 §4.4 的「match 密度超配」判据在字面上成立**：耗尽桶（335 站点）match 密度中位数 **9**，全等桶（32 站点）**1**（9×），且耗尽分数随密度单调上升（密度 1 → 27.6%、密度 10–16 → 100%）。**但该「超配」几乎完全是函数尺寸伪影**：密度与函数行数 Spearman ρ = **0.883**，全等桶站点全部落在 span ≤ 71 行 / 密度 ≤ 3 的小函数里，耗尽桶横跨 span 29–1257；在**每个固定密度层**（1/2/3）全等站点函数平均 23–66 行、耗尽站点 104–171 行——**密度与尺寸在此数据中不可分离**。对照耗尽桶相对全体基线的顶半段超配倍数仅 **1.04×**，说明「9×」来自全等桶全是微函数，而非耗尽桶在「match 密集」上异常集中。结合 STEP-0556 §11 六刀模式（每把 O(n²) 重扫/累加刀回绿 0–1%）与 STEP-0575 §3.3（两具名源合计 <0.1%），**match 族对燃料墙的贡献 ≤ ~1%**。

**裁决：match 主刀（改 `cgs_match_hex` / `cgs_dispatcher_slot` / match 族左折叠 concat）不值得作为破墙主刀开工**；预测回绿 ~0–1%（六刀模式 + §3.3 口径）。**收敛结论**：下一刀只能走 **op 级燃料剖析**（需 owner 授权 runner 生产钩 + gcc 安装，最小变更清单见 §4.3）**或 owner D8 限额裁决**（STEP-0556 §11 选项 2）。**若 owner 坚持 §4.4 字面规则（超配 → 落地），则应落地主刀，但六刀模式预示回绿 ~0–1%，本片不主张该路径为主破墙刀。**

## 1. 背景与判据来源

- STEP-0562/0572 定位墙在发射器层、`fuel ~ IR²`；STEP-0572 §5 明确 **op 级 wasm-op 源未定位**（需 runner 生产 API 暴露 fuel 读数/epoch·opcode 钩）。
- STEP-0575 §3.2 判别实验钉死平方项随 **match 数**（`match_flat` n=2→4 恰 4.0×）；§3.3 收窄：两具名源（源 A = `cgs_match_hex` 每 match 调 `cgs_dispatcher_slot` 全 body 重扫；源 B = match 族左折叠 concat）相对总 fuel **均 <0.1%** ⇒ 与「墙 = 常数主导」一致。
- STEP-0575 §4.4（核心交付）：**「先对 `match-component-coverage-v1.json` 396 站点按 match 密度分桶，比较「耗尽 335」vs「全等 32」——若耗尽站点 match 密度不超配，则主刀预测回绿 ~0、不应开工；若超配才落地。」**本片执行该判据。
- STEP-0556 §11（决定性跨片模式）：六刀（B2 值索引 1.10%、B3 游标化、B4-D cell_slot 索引化、P1/C 线性装配）**回绿 0–1%** ⇒ **墙 = 常数主导（Sico→wasm 每操作开销 × 操作数），不是算法主导**；任何局部 O(n²) 优化预测 ≤2×。§134 裁「下一刀不再是任何单一重扫游标化/索引化」，需仪器化 profile 或结构预解析。

## 2. 方法与口径（代理口径 + 局限，必读）

### 2.1 密度代理（账本无该字段，派生）

- **代理口径**：站点密度 = **其所在函数的 match 语句数**（含嵌套）。函数 = 从 `^function\s+<name>\s*\(` 到下一函数名前的行区间；match 语句 = 行匹配 census 工具同一正则 `^(\s*)match\s+(.*?):\s*$`（`tools/census-m22-match-sites.py:42-44`）。
- **为何用此代理**：账本条目键为 `id,path,line,function,function_line,parameter_count,head,labels,case_shape,verdict,…`，**无 `match_density` 字段**；任务允许用「站点所在函数的 match 语句数」作代理。census 工具用同一正则枚举站点 ⇒ 口径与账本同源。
- **含嵌套（口径说明）**：全 `selfhost/*.sico` 共 **523** 条 match 行（含嵌套）；其中 `checked_add` 头 **419** 条；census 站点枚举用**顶层-only 规则**（`census-m22-match-sites.py:122-124`）得 **396** 站点 ⇒ 419 − 396 = **23 条嵌套 `checked_add` match** 不是站点但计入密度。本片密度为「函数内全部 match 语句」，含这 23 条嵌套（它们同样是 match 降低工作）。**局限**：census 正则不剥离字符串/注释；已扫描确认 `selfhost/*.sico` **无**「注释行以 `match` 开头」的误报（§8 扫描输出为空）。
- **函数尺寸辅助量**：`span` = 函数行数（用于检验「密度是否只是尺寸代理」）。

### 2.2 四桶口径（两读数，账本 vs 已校正）

- **账本文件口径**（`match-component-coverage-v1.json` 现文）：`ir-equal+component-other:FuelExhausted` **331** / `ir-equal+component-equal` **36** / `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` **24** / `ir-equal+component-refused-guest:unsupported codegen source shape` **5** = 396。
- **已校正口径（本片主用）**：STEP-0567 §2.3/§2.4 用 HEAD-fresh 固件实测 **MS-081..084（`parser.sico:fixed_literal_const_segment`）实为 `FuelExhausted`**，账本记 `component-equal` 为**偏乐观**；AGENTS.md §10（:211）据此订正为 **335 exhausted / 32 equal / 24 trap(8 MiB) / 5 refused-guest**。STEP-0572 §3.1 独立实测该函数 emit-only `ir_bytes=92057` 在 6.4e9 ceiling **exhausted**，佐证订正。
- **本片采用 335/32/24/5 为主口径**（AGENTS.md §10、STEP-0575 §4.4、任务前置描述一致），并把账本文件的 331/36 作为「账本现文口径」并列报告。**差异 = MS-081..084（4 站点，`fixed_literal_const_segment`，密度 4 / span 101）从 equal 翻为 exhausted。**

### 2.3 统计口径

- 每桶：样本数 n、密度均值/中位/分位（p25/p75）、span 均值/中位/最大。
- 超配判定：①§4.4 口径 = 耗尽 vs 全等**密度中位比**；②基线口径 = 耗尽桶在「密度 ≥ 全局中位(9)」的份额 vs 全体站点同份额（超配倍数 = 两者之比）；③剂量-反应 = 各密度带的耗尽分数。
- 混淆检验：密度↔span 的 Spearman ρ；**固定密度层**内 equal vs exhausted 的 span 均值分离；span 分层耗尽分数。

## 3. 结果（密度分账）

### 3.1 四桶密度分布（已校正 335/32/24/5）

| 桶 | n | 密度均值 | 密度中位 | 密度 p25 | 密度 p75 | 密度范围 | span 均值 | span 中位 | span 最大 |
|---|---:|---:|---:|---:|---:|---|---:|---:|---:|
| **耗尽 FuelExhausted** | 335 | 10.23 | **9** | 5 | 15 | 1–23 | 454.5 | 365 | 1257 |
| **全等 component-equal** | 32 | 1.44 | **1** | 1 | 2 | 1–3 | 29.8 | 29 | 71 |
| **trap 8 MiB** | 24 | 24 | 24 | 24 | 24 | 24 | 1126 | 1126 | 1126 |
| **refused-guest** | 5 | 5.8 | 7 | 4 | 7 | 4–7 | 83.2 | 106 | 106 |
| **全体** | 396 | 10.30 | 9 | 4 | 15 | 1–24 | 456.2 | 365 | 1257 |

（账本现文口径：耗尽 331 密度均值 10.31 / 中位 9；全等 36 密度均值 1.72 / 中位 1 / span 最大 101——因含 MS-081..084。）

### 3.2 剂量-反应（耗尽分数随密度，已校正）

| 密度带 | n | 耗尽 | 耗尽分数 |
|---|---:|---:|---:|
| (0,1] | 29 | 8 | **0.276** |
| (1,3] | 51 | 40 | **0.784** |
| (3,6] | 69 | 67 | **0.971** |
| (6,10] | 66 | 63 | **0.955** |
| (10,16] | 87 | 87 | **1.000** |
| (16,100] | 94 | 70 | 0.745* |

（*末带降至 0.745 因含 24 个正交的 8 MiB trap 站点（密度 24）；剔除 trap 后密度 ≥16 的耗尽分数 = 100%。）

### 3.3 超配判定

- **§4.4 口径**：耗尽密度中位 **9** vs 全等 **1** ⇒ **9× 超配**（字面成立）。
- **基线口径**：密度 ≥ 9（全局中位）的耗尽份额 53.7%（180/335）vs 全体 51.5%（204/396）⇒ **超配倍数 1.04×**（相对基线**无异常集中**）。
- **剂量-反应**：27.6% → 78.4% → 97.1% → 100%（单调，表面支持「match 密集 → 必耗尽」）。

### 3.4 混淆检验（决定判据是否可判定）

| 检验 | 结果 |
|---|---|
| 密度 ↔ span Spearman ρ（n=396） | **0.883**（密度 ≈ 尺寸代理） |
| 固定密度层 equal vs exhausted 的 span 均值 | 密度 1：23.4 vs 104.1；密度 2：33.2 vs 170.9；密度 3：66 vs 144.1（全等始终小 3–4×） |
| span 分层耗尽分数 | (0,50]→0%、(50,100]→75%、(100,200]→91.8%、(200,800]→100% |
| 全等桶函数集 | 全部 span ≤ 71 / 密度 ≤ 3（微函数） |

**判读**：全等桶 = 微函数（span 8–71）且只含密度 ≤3；耗尽桶 = 大函数（span 29–1257）。**密度梯度与尺寸梯度在此数据中完全重叠**——「超配 9×」由全等桶全是微函数驱动，非耗尽桶在 match 密集上相对基线的异常集中（基线口径仅 1.04×）。**§4.4 的密度判据无法分离「match 项」与「每操作常数 × 操作数（∝ 函数/闭包尺寸）」**。

### 3.5 独立判别证据（为何判据字面成立仍不落地）

- **六刀模式（STEP-0556 §11:140）**：每把消除 O(n²) 重扫/累加的刀回绿 **0–1%**（B2 值索引 1.10%、B3 游标化、B4-D cell_slot、P1/C 线性装配≈0）。主刀两机制与 B2 同属「消除一次重扫/累加」类 ⇒ 预测同档。
- **§3.3（STEP-0575:56）**：源 A + 源 B 合计 <0.1%（match_flat n=4 处）；真实站点密度更高（≤23），但剂量-反应已被 §3.4 归为尺寸伪影，故不推翻 <0.1% 量级。
- **STEP-0567 §3（:107）**：STEP-0560 的 `list.length[U64]` 发射能力**没有**让任何已知站点闭合。

## 4. 裁决

### 4.1 主刀是否值得开工

**不值得作为主破墙刀开工。** 理由：①§4.4 字面「超配」成立，但被 §3.4 证明为尺寸伪影，判据**不具备判别力**；②独立判别证据（六刀 0–1%、§3.3 <0.1%、STEP-0567 零闭合）一致指向 match 项 ≤ ~1%；③主刀两机制与已被证伪的 B2/B3/P1/C 同构（消除一次 O(n²) 重扫/累加）。**预测回绿 ~0–1%，不改变门状态、不推进出口。** 若 owner 坚持 §4.4 字面规则而落地主刀，本片不反对执行，但如实登记预测回绿 ~0–1% 且需 exit 单列「195 新开测回绿数 + 32 全等站点 component_sha256 逐字节不变 + 词表 source_sha256 刷新 + ledger 联动重测（>8 min）」（STEP-0575 §4.4 exit 口径）。

### 4.2 收敛结论

**下一刀只能二选一**（STEP-0556 §11:144-146 与 STEP-0572 §5 一致）：

1. **op 级燃料剖析**：需 owner 授权 runner 生产钩 + gcc 安装（§4.3 最小变更清单）。这是唯一能把「2.4e9 = ~1.2e6 wasm ops/输出字节」定位到具体指令/操作数族的路径（STEP-0556 §134 已排除静态读码可解释性）。
2. **owner D8 限额裁决**：常数主导 + 预解析成本过高时，owner 在「上调限额（fuel 1.6T vs 800e9 = 2×，STEP-0556 §11 选项 2）」与「收窄语料/闭包范围」间裁决。**不得由代理自行上调限额。**

（STEP-0556 §11 选项 1「结构预解析」为第三条已登记方向：每模块一次预解析 IR 为紧凑结构、替换全部逐字段 `cgj_member`/`cgj_value_text` 重读，操作数降 10–100×，但改动面大——几乎每个 `cgs_*` 签名 + 指纹 + 2h ledger。若 owner 选此，属独立主刀，不在本片范围。）

### 4.3 op 级燃料剖析最小变更清单（供 owner 审批）

| # | 文件 | 函数/位置 | 变更 | 读数口径 |
|---|---|---|---|---|
| 1 | `runner/sico-runner/src/lib.rs` | `PreparedProgram::run`（燃料/epoch 配置处） | **生产钩**：run 后暴露**已耗 fuel 读数**，和/或安装 **epoch/opcode 回调**（per-wasm-op 或 per-function 计费） | per-wasm-op fuel 计数；per-function fuel 归因（epoch-deadline 归因法） |
| 2 | `runner/sico-runner/tests/m22_fuel_profile.rs` | 既有 `#[ignore]` 判别臂（STEP-0572 §2 emit-only 注入 + `fuel_bracket`） | 在钩下运行探针，把 fuel 归因到 op 类别/函数 | 代表性站点集：`fixed_literal_const_segment`（全等→实耗）、`control_while_function_ir`（耗尽）、`scalar_ir`（trap）、`match_flat` 合成族（平方项隔离） |
| 3 | `selfhost/codegen.sico`、`selfhost/parser.sico` | 冻结 guest（只读） | **零改动**（探针输入） | 源 SHA 不变 |
| 4 | 工具链 | `gcc.exe`（mingw-w64） | **安装**（STEP-0572 §1:30 最小解除输入；`cargo build/test` 已可用，仅 clippy/check 与 cc-rs 重建需 gcc） | 使 clippy/check profile 与 `wasmtime-internal-fiber` 的 `build.rs` 可编译 |

> **授权边界**：#1 改 runner **生产**代码（STEP-0572 §2 明言「加钩需改 runner 生产代码，本片禁改」）；#4 装系统工具。两者均需 **owner 显式授权**，本片未做、也不在本片权限内。`crates/*`、`selfhost/*`、`docs/*` 零改动。

## 5. 诚实收窄

- **代理口径局限**：密度与 span 都是**函数体**量，而 fuel 是**依赖闭包**（函数 + 传递调用者）执行测得（STEP-0572 §2 emit-only 闭包抽取）——函数体 span 是闭包尺寸的**非精确代理**（18 个 span ≤130 行的耗尽函数可能闭包大，因调用大 helper）。**本片未测闭包尺寸**；因此「尺寸伪影」结论基于函数体量，闭包级复核需 op 级剖析或闭包尺寸测量（另片）。
- **判据非判别力为本文核心发现**：§4.4 字面「超配」与「不超配」均无法在此数据上分离 match 项；故本片给出「字面成立但判据失效」的双重结论，**最终开工决策属 owner**。
- **账本现文偏乐观**：MS-081..084 在 `match-component-coverage-v1.json` 记 `component-equal`，STEP-0567 §2.3 已确证实为 `FuelExhausted`；本片未改账本（禁改），以 335/32/24/5 为主口径并并列账本现文 331/36/24/5。
- **未跑项**：未运行任何 `cargo`/`clippy`/测试/`validate-step-0124`（纯只读测量 + 文档，AGENTS §8 不机械跑全工作区）；只跑只读扫描（PowerShell 账本/源分析 + `git status`/`git diff --check`）。
- **未翻任何门**：门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用——全部维持；M22 整体 **NO-GO**；零门翻转、零生产代码改动、零限额变更、零账本/既有文件改动。

## 6. 文档治理只读评估（STEP-0576 dossier 复核对账）

> 只读评估，**未重编号、未改表**（重编号属 owner 裁决项）。下列为当前 `dev` 工作树实测，与 STEP-0576（`codex/m22-w2 @ a939862`）差异已标注。

| 项 | STEP-0576 结论 | 本工作树实测（dev @ 7bcdc35） | 处置建议 |
|---|---|---|---|
| README 同号组数 | 20 组 | **26 组**（+6：0279–0283 五组、0338 一组） | 见下行 |
| — 19 组双语重复 | 0051–0061 ×11、0111–0118 ×8，逐字节同链、纯翻译无增量 | 同（19 组仍在） | **合并去重**（不重编号）；其中 0058/0059/0061 三组两行状态措辞不一致，合并以文件 H1 实际裁决为准、不得借机改 gate |
| — 0175 两份文件共用一号 | 保留 `STEP-0175-language-v1-batch2.md`（权威，RFC-0046 四项决策），另一份重编号 | 同；README 行 191/192 两行 | **重编号**（owner 裁决；新号保持 M22 S0 邻近段） |
| — 0279–0283（新增，本工作树） | 未在 m22-w2 分支见 | 五组每组两行，README **已标 `· codex/m22-w1-ci 线` vs `· dev 线`**（AGENTS §7 的 origin/dev 与 dev 线编号冲突，非静默重复） | **加注**（已带分支标注，保持）；不合并、不重编号（与两分支历史冲突）；集成留 M25（D14） |
| — 0338（新增，本工作树） | 未在 m22-w2 分支见 | 两份文件：`STEP-0338-m22-w2-guard-chain-host-call-lets.md` + `STEP-0338-m24-native-ui-fluent-d2d-renderer.md`（后者为未跟踪 `??`） | **加注或重编号**（owner 裁决；M24 轨占用 STEP-0330–0337，0338 为新冲突号，按 AGENTS §7 取号规则处理） |
| 5 行未转义 `\|` | 0285/0328/0502/0520/0539（行 321/364/440/480/487） | 同五号，README 行 326/369/445/485/492（+5，README 有增行）；0285 含 `` `\|\|` ``、0328 含 `` `Result[I64\|U64,…]` ``、0502 含 `` `variant ok\|error` ``、0520 含 `` `\|\|` ``、0539 **缺收尾 `\|`** | 前四行 `\|` **转义**；0539 **补收尾 `\|`**（表修复，非重编号） |
| 0440–0485 悬空引用 | STATUS 60 行引用、`docs/steps/` 46/46 无文件、README 仅登记 0486–0489、STATUS 真链接仅 0486/0487/0488 | 同：`docs/steps/STEP-04[4-8][0-5]-*.md` **0 文件**；0486–0489 存在；README 04xx 行仅 0486/0487/0488/0489 | **STATUS/handoff 加注**（不补登、不删史、不回收号；叙述性引用为历史记录） |

**本片建议（不改表）**：仅「19 组双语合并去重」与「5 行 `\|` 转义/补尾」两类属**非重编号的机械修复**，可在执行片落地；0175 重编号、0279–0283/0338 处理、0440–0485 加注均属 **owner 裁决**，本片只列清单不动手。

## 7. 改动文件

| 文件 | 改动 |
|---|---|
| `docs/steps/STEP-0579-m22-match-density-ledger-accounting.md` | **新建**（本文件，本片唯一产物） |
| `selfhost/match-component-coverage-v1.json`、`selfhost/*.sico`、`tools/*` | **零改动**（只读） |
| `runner/*`、`crates/*`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`crates/sico-ui-renderer-d2d` | **零改动**（本片未触） |

## 8. 复跑（命令与证据）

```powershell
# 只读测量（PowerShell 账本 + 源分析；无 cargo/编译/测试）
powershell -NoProfile -Command "…match-component-coverage-v1.json 四桶 Group-Object…"      # exit 0；331/36/24/5 精确
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\m22_density.ps1"             # exit 0；密度/span 连接 + 直方图 + 函数表
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\m22_confound.ps1"            # exit 0；ρ=0.883、固定密度层 span 分离、span 分层
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\m22_final.ps1"               # exit 0；已校正 335/32/24/5 + 剂量-反应 + 超配倍数
git diff --check                                                                            # 无本片引入的空白/冲突标记
git status --short                                                                          # 见下
```

**只读扫描事实**：`selfhost/*.sico` 共 523 条 match 行（含嵌套）、419 条 `checked_add` 头、396 顶层站点（census 顶层-only 规则）；注释行误报扫描 = 空；`git diff --check` 无本片引入异常。

**本片产物 = 仅 `docs/steps/STEP-0579-*.md` 一个新增文件**；`git status --short` 中其余改动（`runner/sico-runner/src/lib.rs`、`crates/sico-package/*`、`docs/STATUS.md`、`docs/ROADMAP.md`、`AGENTS.md`、`docs/steps/README.md`、`crates/sico-ui-renderer-d2d/`、`docs/steps/STEP-0338-m24-*.md`、`docs/evidence/m24/ui-frame-corpus.json` 等）均为**既有/并发改动**，本片未触、未覆盖、未还原。
