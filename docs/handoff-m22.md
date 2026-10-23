# M22 交接手稿（living handoff，STEP-0389 本机基线）

> - 更新：2026-10-19（STEP-0469 implementation；当前状态见 §0，census 141/146、0 DIFF，红户 5）；工作目录 `E:\github\sico-w2`（git worktree），分支 `codex/m22-w2`（STEP-0352–0469 未提交，工作树 = e963872 + 百余片）。**探针基建在 `target/tmp/`（持久基座；禁放 C:\Users\...\Temp）**：extract_closure.py（file-order 版）/ closures.py / cmp_probe.sh（400B 燃料、POSIX stdin + cygpath DUMP_SRC、printf 字节对拍）/ run_dump.sh / dump_oracle.rs / run_census.sh / scratch_census_probe.rs（2560MiB）/ instr_cs_0436.py / surgery_0465_l/r.py / surgery_0469.py；parser.sico 备份链 `target/tmp/parser_backup_*.sico`（最新 0469）。
> - `origin/dev` 已前进至 `550059a`，包含另一套 STEP-0279–0283 W1/W2 历史，与本分支源码及 STEP 编号冲突。不得覆盖或隐式合并。STEP-0330–0337 为 M24 并行轨（worktree `sico-m24`，提交 `095227b`）占用；M22 线自 STEP-0338 起编。
> - **2026-10-20 23:48 更新**：本工作树出现**并行 M23 会话**产物（`tools/validate-step-0489.ps1`，M23 residual closure A）→ **STEP-0489 已被 M23 占用**；M22 线下一片从 **STEP-0490** 起取号。两会话同树并行，改动面必须互不重叠。
> - **2026-10-21 更新**：并行 M23 会话**先占用了 STEP-0490**（`docs/steps/STEP-0490-m23-item3-bare-literal-preparation.md`）→ M22 线本片**顺延为 STEP-0491**。取号前必须先核对 `docs/steps/` 与本手稿；两会话改动面仍不重叠（M23 触 `docs/ROADMAP.md` / `docs/plans/M23-*` / M23 普查与 bare-literal 测量工具，未触 `selfhost/parser.sico` 与 M22 计划/手稿）。
> - **2026-10-23 更新（批次提交 + 独立 CI 已裁决）**：owner 授权「一次提交推送」⇒ **STEP-0506…STEP-0521 共 16 个切片已一次提交（`a5e0b65`）并推送到 `github` 远端的 `codex/m22-w1-ci`**（fast-forward `50cc4a3 → a5e0b65`）⇒ **独立 CI run #28 = `success` ✅**（<https://github.com/ImmortalWing/sico/actions/runs/39886240283>，head_sha `a5e0b65`，`status: completed` / `conclusion: success`；前两轮 run #26 / #27 亦均 `success`）⇒ 族级出口判据的「独立 CI 绿」一项**对上一批已达成**。**STEP-0522 随后单独提交（`8bdd58d`）并推送到同一 `codex/m22-w1-ci` 分支** ⇒ **独立 CI run #29 = `success` ✅**（<https://github.com/ImmortalWing/sico/actions/runs/39907445663>，head_sha `8bdd58d`，`status: completed` / `conclusion: success`）⇒ 该项**对 STEP-0522 也已达成**；**STEP-0523 的推送已触发 run #30 = success ✅**（run id `39915083272`）⇒ **本片（STEP-0524）的推送会触发下一轮**，按仓库既有做法**由下一片的提交回填该轮裁决**（不为此再推一轮）。**三个他人遗留文件未纳入提交、原样保留在工作区**：`cl_argument_instruction_count.sico`、`crates/sico-codegen-wasm/tests/scratch_dump.rs`、`tools/fix-selfhost-comment-words.py`。**工作区当前只剩本片（STEP-0524）的实现与文档（未提交）**。
> - **2026-10-22 更新（规划片 STEP-0514）**：owner 指示"只负责规划、尽量细化、执行是另一个对话" ⇒ **M22 剩余工作的闭合阶梯已写入 [M22 计划 §3.3](./plans/M22-compiler-self-host.md)**（7 个 slice ≈ 75–125 片；关键路径 `W1 → W2 → W3 → W5 → W6 → W7`）。**执行会话从该节开始读**；本手稿只提供恢复路径。
> - **执行线的三条硬规则**（来自 §3.3）：①**不预留 STEP 号** —— slice 只用标识（W1–W7 / T1–T20 / G1–G2 / R1–R3），开工时按本手稿的取号规则顺取（**STEP-0514 已被规划片占用 ⇒ 执行线下一片从 STEP-0515 起**，取号前仍须核对 `docs/steps/` 与本手稿；M24 占 0330–0337、M23 占 0489/0490）；②**W1 是纯测量片**（只测量、不实现、不推进任何门、不放宽任何 typed 契约），其**三档判读**（D6 阈值：≤6× 可行 / ≤16× 临界 / >16× 不可行）是决策枢纽，判"不可行"即触发止损 T-R3-a（停止 W2/W3 顺序推进、按 D7 优先级转 ADR/RFC 修订流、升级给 owner）；③**canary 次序已由 ADR-0016 A1 冻结**（`formatter 组件级 → 37 语料组件级 → bundle 组件级 → harness`）⇒ 早梯级的 refusal/trap **阻断**后梯级的完成声明；**D1–D3 差异已全部裁决关闭**，无待 owner 决策项。
> - 权威裁决：[STATUS](./STATUS.md)、[M22 计划](./plans/M22-compiler-self-host.md)、[STEP-0292](./steps/STEP-0292-m22-w1-independent-ci.md)、[STEP-0293](./steps/STEP-0293-m22-w2-u64-to-text-while-rhs.md) 与 [STEP-0294](./steps/STEP-0294-m22-w2-map-get-match-subject.md)。本手稿只提供恢复路径。

## 0. 当前状态
**STEP-0514（2026-10-22，planning complete，纯规划无实现）**：**M22 剩余工作的闭合阶梯已定稿**（[STEP-0514](./steps/STEP-0514-m22-w2-s5-s6-closure-ladder.md) → [M22 计划 §3.3](./plans/M22-compiler-self-host.md)）。**本片不推进任何出口门；M22 仍为 NO-GO。**
- **量化基线（实测）**：`A = R(S)` 组件 **1,024,613 B**（core 1,024,161 / **code 1,003,250** / **data 16,487** / type 3,685 / 组件级 419）vs guest 已发射过的最大组件 **1,025 B** ⇒ **落差约 1000 倍**；冻结源集 S = 9 文件 / 29,088 行 / 1.30 MB；S 内宿主内建 **20 族**（`text.concat` 2,882 次最多）而 guest **只闭合 1 个**（`sico.bytes.length`）；语料 **37 accepted / 178 refused**（LEXICAL 116 / SCRIPT_ABI 28 / 散码 34）。
- **阶梯**（7 slice ≈ 75–125 片，关键路径 `W1 → W2 → W3 → W5 → W6 → W7`）：W1 规模可行性**纯测量**（三档判读）→ W2 20 族四拍流水线 → W3 **组件级广度 rung**（W3-a `formatter.sico` 组件级 → W3-b 37 条 accepted 语料组件级差分）→ W4 178 条拒绝三元组一致 → W5 **组件级尺度 rung**（5 文件 bundle ≈1.02 MB + 9 文件全集组件级全等）→ W6 ADR-0015 `A == B == C` + `.sapp` + 预算 + 证据清单 → W7 快照 + 独立 CI + 全量回归 + 新出口审计 + `origin/dev` 裁决 + 门 4 补强 + 支持面登记表。
- **owner 三决策（2026-10-22）**：①同意插纯测量片并前置；②授权**合格跑之前**重基线化限额（失败后上调作废该 attempt）；③`origin/dev` 冲突历史**推迟到门 6 收尾**。
- **追加授权"全权拍板" ⇒ D4–D15 已全部裁决关闭**（执行会话直接遵守，不再回头请示）：**D4** 组件级 canary 次序冻结为 `formatter 组件级 → 37 语料组件级 → bundle 组件级 → harness`（**先广度后尺度**），登记为 **[ADR-0016 A1](./adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)**（STEP-0486 §2.4 已加对齐注记，原句保留以便追溯）；**D5** W1 探针**不进 `selfhost/`**（避免扰动冻结源集 S 与 A 的基线字节）⇒ 放 runner 测试侧独立 fixture，探针组件**不得进入 A/B/C 链**；**D6** W1 判读阈值固定（≤6× 近线性＝可行 / ≤16×＝临界 / >16×＝不可行）；**D7** 不可行分支优先级 = 二进制输出通道 > 流式分块发射 > 降中间表示成本 > 加限额；**D8** 限额重基线候选 = arena 4 GiB / host 5120 MiB / fuel 1.6T / timeout 180 s；**D9** **W2 禁止并行开族**（共享 `parser.sico`/`codegen.sico`）⇒ W2 是最不可压缩的一段；**D10** 族降级后若 S 内仍有未覆盖调用点形状 ⇒ **不得降级**；**D11** 语料一致性探针 = `tools/probe-m22-corpus-parity.ps1`（只读）+ 冻结期望 `selfhost/corpus-parity-v0.json`；**D12** S 喂入 = 单块 ≤ 64 KiB / 总量 ≤ 2 MiB / 超限 fail-closed；**D13** 无 fuel 读数出口时登记为**上界**并显式标注，**不阻塞 GO**；**D14** `origin/dev` = **不 rebase 不 merge**，冲突历史转 M25 集成议题；**D15** 门 7 前必须产出**支持面登记表**（族 × 子集边界 × 语料覆盖 × 未覆盖形状）。
- **执行会话入口** = [M22 计划 §3.3](./plans/M22-compiler-self-host.md)；取号 = **STEP-0515 起顺取**（开工时核对 `docs/steps/` 与本手稿）；**执行线第一片 = W1（纯测量）**，其后才是 W2 的 T1–T20。
**STEP-0502（2026-10-21，complete，独立 CI GO）**：**run 体发射的"指令段刀" = guest 发射的指令段与 Rust backend 实际发射逐字节相同**（[STEP-0502](./steps/STEP-0502-m22-w2-s5-run-instruction-segment.md)）。**本片不推进门 3**。
- **实测**：guest 推出的指令段 = **182 hex（91 B）**，与 backend **实际发射**的指令段**逐字节相同**（对照读 oracle 的**生产输出**，不是复刻）。
- **guest 侧**：`selfhost/codegen.sico` 新增 **28 个函数**（算术 / 十进制 / 编码 / 布局 / 槽位 / 记录 / 发射七组）；`selfhost/compiler.sico` 新增 **`--emit-run-instr-hex`** 驱动模式。
- **两个新编码助手**：`cgs_sleb_hex`（**有符号** LEB128 ⇒ `64` 编码为 `c000` 而非 `40`）与十进制解析（**至多 10 位**，保证不溢出且落在 i64 正半区）。
- **两个新固件钉**：`sico_compiler_emits_the_run_instruction_segment_byte_exactly`（逐字节等同）+ `sico_compiler_refuses_the_run_instruction_segment_outside_the_subset`（子集外 typed 拒绝）。
- **支持子集**（超出即 `ERR:E-SH-CG-IR-EMIT`）：`project`（基为**已声明 record 的形参**）/ `const_i64` / `const_u64` / `construct`（已声明 record）/ `variant ok|error`（tag 取**冻结的 canonical ABI 判别值** ok=0 / error=1）。
- **槽宽交叉校验（fail-closed）**：guest 用"声明类型"计数而 oracle 用"推断布局" ⇒ 本片在 `project`/`construct` 上**显式校验两者相等**，不等即 typed 拒绝，**不静默取一侧**。
- **踩到并修正 4 条 guest 前端真约束 + 2 条自造缺陷**（详见 §5 第 10/11/14 条与哨兵纪律条）：`record` 是保留关键字不得作形参名；**同函数内 `let` 无块作用域**（两个循环体各自 `let` 同名也会撞）；`U64.equal` 返回 **Bool** 不可再包一层；"**找不到**"与"**总数**"不能共用 0 哨兵；**布尔标志必须用中性名字 + 明确默认值**（写反的那次被字节对拍当场抓出）。
- 回归：全源 census 字节等同（**自举形状前沿未破**）+ 固件 **9 + 99 + 1 + 2 = 111/111** + 前端前沿探针 **18/18** + 计划层前沿探针 **8/8** + **一键 CI 12/12** + 行尾审计（3 文件纯 LF、零 CRLF、零 BOM）。
- **诚实收窄**：**本片只发射指令段**——`run` 体的 **Script 结果区返回 + dispatcher 尾声（178 hex）**在本片**尚未**发射 ⇒ 整段 run 体还没有逐字节等同；`variant` 的 `error` 臂右对齐**只有源码依据**（参考形态不触发）。**后续已闭合**：该尾段已由 [STEP-0503](./steps/STEP-0503-m22-w2-s5-run-body-result-return.md) 发射（整段 `run` 体 **360 hex** 与 backend 实际发射**逐字节相同**）。**门 3 仍 NO-GO**。
- **独立 CI 裁决**：已**提交并推送**（owner 授权）= 提交 `ee71719` → `github/codex/m22-w1-ci`（fast-forward `a730ac6..ee71719`）；独立 CI run **39556951856**（run #24，`event: push`，head_sha `ee71719`）= `status: completed` / **`conclusion: success`**，job `windows-gnu` 06:41:12Z → 07:09:29Z、job 内 6 步全 success、**内层 12/12 步全 PASS** ⇒ 本片闭合。**首轮一键 CI 那处环境敏感红点**（`prepared_program_reruns_are_isolated_and_survive_a_trap` 的 warm median 预算断言 22.16 ms vs 20 ms）经该裁决确认在**独立 runner 上通过** ⇒ **预算常量与断言均未改动**、不需重新基线化。
**STEP-0503（2026-10-21，complete，独立 CI GO）**：**run 体发射的"结果区返回刀" = guest 推出整段 `run` 体并与 Rust backend 实际发射逐字节相同**（[STEP-0503](./steps/STEP-0503-m22-w2-s5-run-body-result-return.md)）。**本片不推进门 3**。
- **实测**：guest 推出的 **Script 结果区返回 + dispatcher 尾声 = 178 hex（89 B）**，与 backend **实际发射**的同一段**逐字节相同**；**顺带把整段 run 体打通**——新增的 `--emit-run-body-hex` 输出 **360 hex（180 B）**，与 oracle 的整段 `run` 体**逐字节相同**（对照读 oracle 的**生产输出**，不是复刻；oracle 生产代码**零改动**）。
- **guest 侧**：`selfhost/codegen.sico` 新增 **24 个函数**（冻结事实 8 + IR 导航 9 + 交叉校验 1 + 发射 6）；`selfhost/compiler.sico` 新增 **`--emit-run-body-hex`** 驱动模式。
- **两类事实必须分清**：**组件级冻结事实由 guest 携带**（结果区 32 B、分配器索引 1、`ScriptOutput` 字段名/顺序/flat 种类/结果区偏移 8-12-16-20-24/flat 槽偏移 0-2-4 —— 与 STEP-0487 携带冻结核心模块段同性质，**不由 IR 派生**）；**IR 派生事实由 guest 重算**（判别值槽 = 变体值首槽；载荷基槽 = 判别值槽+1；`return_area`/`copy_destination` = dispatcher 槽 +1/+2）。
- **交叉校验（fail-closed）**：字段名/顺序/flat 种类/槽数任一与冻结事实不符即 typed 拒绝 `ERR:E-SH-CG-IR-EMIT`，**不静默取一侧**。
- **支持子集**：`run` 单块 + `return` 一个值 + 该值由 `variant ok` 产生 + 载荷由 `construct` 产生 + 字段恰好 3 个且与冻结 `ScriptOutput` 记录逐位一致。**`error` 臂与 oracle 的双载荷（`has_ok && has_error`）形态一律拒绝**（参考形态不触发 ⇒ `ScriptError` 结果区偏移**无实测真值**，不拿源码推导冒充支持）。
- **两个新固件钉**：`sico_compiler_emits_the_run_body_byte_exactly`（整段 360 hex 逐字节等同 + 尾段 178 hex 固化断言）+ `sico_compiler_refuses_the_run_body_outside_the_subset`（`ERR:E-SH-CG-IR-PLAN` / `ERR:E-SH-CG-IR-EMIT`）。
- **踩到并修正 1 条自造缺陷**（详见 §5 第 15 条）：把 `call <index>` 写成了「立即数的 LEB」而不是「操作码 + 立即数」⇒ **`cg_leb_hex` 是裸立即数编码器、不是指令编码器**。
- **顺带纠正 STEP-0501 §4.3 两处表述偏差**：dispatcher 尾声实为 **6 字节**（原表列了 7 项）；`i32.store8` 的 memarg 是 **align+offset 两个 LEB** ⇒ `3a 00 00` 是 **3 字节**。**真值 hex 本身未变**。
- 回归：全源 census 字节等同（**自举形状前沿未破**）+ 固件 **9 + 101 + 1 + 2 = 113/113** + 0245 exit 0 + 0262 `STEP_0262_OK` + 前端前沿探针 **18/18** + 计划层前沿探针 **8/8** + **一键 CI 12/12** + 行尾审计（4 文件纯 LF、零 CRLF、零 BOM）。
- **诚实收窄**：**本片只覆盖参考形态**（oracle 只接受 Script v0 入口签名，其它形态根本无法取真值）；**整组件逐字节等同是 STEP-0504 的事**（用派生 run 体替换 STEP-0487 的冻结 run 体 hex）。**门 3 仍 NO-GO**。
- **下一刀的切点已经找准**（**已由 STEP-0504 实施**）：`selfhost/codegen.sico` 的 `cg_run_body_hex_minimal()` 原先返回**一条冻结常量** = `15`（局部声明组数 21 的 LEB128）+ STEP-0499 的局部声明段 + STEP-0503 的 360 hex run 体；**两段当时都已经能 IR 派生**（`cgs_local_hex` / `cgs_run_body_hex`）⇒ STEP-0504 = 把这条冻结常量换成 `cg_leb_hex(组数) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`，替换后 `cg_core_module_hex()` 输出的整个核心模块都变成 IR 派生的 ⇒ 参考形态的**整个组件**逐字节等同。**该函数与它承载的冻结常量现已从 guest 源码删除**。
- **独立 CI 裁决**：已**提交并推送**（owner 授权）= 提交 `7cf2ece` → `github/codex/m22-w1-ci`（fast-forward `ee71719..7cf2ece`）；同一次提交带上了 **STEP-0502 的独立 CI 裁决回填**（owner 偏好一次提交推送、不拆两轮独立 CI）。独立 CI run **39572167889**（run #25，`event: push`，head_sha `7cf2ece`）= `status: completed` / **`conclusion: success`**，job `windows-gnu` 08:49:22Z → 09:11:57Z、job 内 6 步全 success、**内层 12/12 步全 PASS** ⇒ 本片闭合。**独立 CI 证明的是本片成立，不是门 3 成立**。
**STEP-0504（2026-10-21，implementation complete locally；本机全绿，未提交未推送）**：**派生组件刀 = 用 IR 派生的 run 体替换 STEP-0487 的冻结 run 体常量 ⇒ 参考形态整组件逐字节等同**（[STEP-0504](./steps/STEP-0504-m22-w2-s5-derived-component.md)）。**本片不推进门 3**。
- **切点**：STEP-0487 留在 guest 里的**冻结 run 体常量**（`15` 组数 LEB + 42 B 局部声明 + 180 B code = **223 B**）被**删除**，换成**现算**的 `cg_leb_hex(组数) + cgs_local_hex(ir) + cgs_run_body_hex(ir)`（三段分别来自 STEP-0499 / STEP-0499 / STEP-0502+0503）⇒ **字节不变、来源从「写死」变成「现算」**。
- **guest 侧**：`selfhost/codegen.sico`（2624 → 2623 行）删 `cg_run_body_hex_minimal()` 与死代码 `cg_minimal_fixture_source()`，新增 `cg_run_body_hex_from_ir(ir)`，`cg_core_module_hex` / `cg_component_hex` / `component_hex_for_ir` 全部改为**吃 IR**；`selfhost/compiler.sico`（977 → 982 行）的 `--emit-component-hex` 从「逐字节比对冻结 fixture 源码」改为「**词法 → IR → 发射**」，既有 typed 契约（`unsupported codegen source shape`）**未放宽**。
- **组件字节解剖（实测，从 guest 实际产出的组件解析）**：组件 **981 B**；核心模块 **530 B**（段 {1,3,5,6,7,10}，**无 data 段** ⇒ 与 STEP-0501 §3.2 互证 ⇒ **run 体是唯一的 IR 相关部分**）；code section 4 个函数 = `run` **223 B（IR 派生）** + `alloc` 84 B + `realloc` 72 B + `post` 18 B（后三个**冻结**）。
- **「发射跟着 IR 走」是字节级差分实测、不是断言**：第二个参考形态 `ECHO_COMPONENT_FIXTURE_SOURCE_EXIT7`（只差 `I64.literal(0)` → `I64.literal(7)`）与参考形态的组件**都**与 oracle `compile_script_program` 的**生产输出逐字节相同**、**都**通过 `wasmparser::Validator`，且两者**必须不同**——实测**只差 1 个字节**：组件字节 **567**（= 核心模块 205 = run 体 75 = run 体 code 32）= `42`（`i64.const` 操作码）后的 **SLEB 立即数**（`00` vs `07`），上下文 `02 21 06 20 03 21 07 42 00`。
- **新增「IR 只能有一个函数」守卫**（`cgj_array_item(ir, functions, 1) != 0` 即拒绝）：guest 携带的**函数段是冻结的**（run/alloc/realloc/post 四个），backend 遇到第二个源函数会**多发一个核心函数**，冻结函数段装不下 ⇒ 若不拒绝 guest 会**静默丢掉**那个函数、产出「看似合法、语义错误」的组件 ⇒ 按 ADR-0016 决策 6 **typed 拒绝**。
- **两个新固件钉**：`sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly`（两个变体都 == oracle 生产输出 + wasmparser 验证 + 两者必须不同）+ `sico_codegen_guest_refuses_the_component_emission_outside_the_subset`（`IDENTITY_SOURCE` 与「echo + 多一个未用函数」两个实例）；STEP-0487 的既有组件钉仍绿。
- 回归：全源 census 字节等同（**自举形状前沿未破**）+ 固件 **9 + 103 + 1 + 2 = 115/115**（selfhost_compiler 由 101 增至 103）+ **本机验证电池 11/11 BATTERY_GREEN**（`fmt (runner)` 00:04 / one-key CI **12 步 CI GREEN** 05:08 / 0245 00:56 / 0262 02:17 / 前端前沿探针 **18/18** 02:08 / IR 计划层前沿探针 **8/8** 00:25 / 0124 00:00 / 0131 00:01 / 0156 00:00 / `git diff --check` 00:00 / 行尾审计 00:01）+ 固件 **115/115**（**LF 修正后的树上复测同值**）。
- **踩到并修正 2 条自造缺陷**（详见 [STEP-0504](./steps/STEP-0504-m22-w2-s5-derived-component.md) §5.3 与 §5 的**验证基础设施**纪律）：**(a)** `selfhost/codegen.sico` 的**工作副本整文件是 CRLF**（2623 CRLF / 0 LF，**全仓唯一**）—— `.gitattributes` 的 `* text=auto eol=lf` 会在**入库时**归一化成 LF ⇒ **提交内容不受影响**，但**本机证据跑的字节 ≠ CI 检出的字节** ⇒ 字节级 CRLF→LF 转换（内容零改动，105,860 → 103,237 B）并**整轮重跑电池**；**(b)** 电池的**行尾审计步骤实测检出 (a) 并返回 1，但汇总仍打印 `BATTERY_GREEN`** —— 根因是退出码管道（步骤体用 PowerShell 变量 `$bad` 当返回值，而 `Run-Step` 读的是**上一个原生命令**的 `$LASTEXITCODE`）⇒ 拆成独立 `audit-lines.ps1` 用 `powershell -File` 调用。**这两条都要写进记录**：第一条是产物卫生，第二条是**验证基础设施缺陷**（同族于 STEP-0497 的「本地绿灯可能是空跑」）。
- **独立 CI 裁决**：已**提交并推送**（owner 授权）= 提交 `579645b` → `github/codex/m22-w1-ci`（fast-forward `7cf2ece..579645b`），远端 head 确认 = `579645b6d2096a1d31b0514d441aff380054da62`；独立 CI run **39585258425**（run #26，`event: push`，head_sha `579645b`）= `status: completed` / **`conclusion: success`**，job `windows-gnu` 10:31:54Z → 10:59:49Z、job 内 6 步全 success、**内层 12/12 步全 PASS** ⇒ **本片闭合**。**独立 CI 证明的是本片成立，不是门 3 成立**。
- **诚实收窄**：①**本片只覆盖参考形态**（oracle 只接受 Script v0 入口签名）；②**组件里仍有 758 B 是冻结骨架**（类型/导入/别名/导出段 + 三个 helper 体）⇒ guest 现在是「**冻结骨架 + IR 派生的 run 体**」，**不是**通用组件发射器，IR 相关字节只占 223/981 ≈ 22.7%；③**`error` 臂仍无实测真值**（承自 STEP-0503）⇒ 继续 typed 拒绝；④**ADR-0015 的 `A == B == C` 没有被本片闭合**（那需要**三段 runner 执行** A=进程内编译 / B=执行 A / C=执行 B 且三者逐字节等同，本片只提供了「guest 组件 == backend 组件」这一条**内容腿**且仅对参考形态成立）。**门 3 仍 NO-GO**。
- **下一刀的范围（owner 已拍板 2026-10-21）**：**STEP-0505 = 把组件发射扩到非参考形态，先取 `error` 臂的真值**——写一个返回 `error(...)` 的 Script-profile fixture 让 oracle **真正发射**，把 `ScriptError` 的结果区偏移从「冻结推导」升级为「**实测**」（这是三条候选里**唯一「只要写一个 fixture 就有真值」**的：Script profile 的 `run` 返回 `Result[ScriptOutput, ScriptError]`，返回 `error(...)` 的 `run` **是合法入口**，backend 会照发）；多函数 / data 段模块要先把 guest 的**函数段**与 **data 段**做成派生的，放到 `error` 臂之后；再往后才是 37 语料 / formatter / 全源 canary 阶梯 + ADR-0015 `A == B == C` + `.sapp`。
**STEP-0516（2026-10-23，measurement complete locally；本机实测绿，未提交未推送）**：**S5-W1 规模可行性测量片（纯测量）= 1 MiB 级字节组装在合格限额内【可行】**（[STEP-0516](./steps/STEP-0516-m22-w2-s5-w1-scale-feasibility.md)）。**本片不推进任何出口门；M22 仍为 NO-GO**。
- **为什么这一片先做**：计划 §3.3.3 把 W1 称为整条闭合阶梯的**决策枢纽**——固定点要求 guest 发射 **≈1,024,613 B** 组件而 guest 已发射过的最大组件是 **1,025 B**（落差 ≈1000 倍）；若 1 MB 级发射在合格限额内做不到，W2/W5/W6 的**实施方式**都要改，且判「不可行」即触发止损 T-R3-a。
- **探针落点遵守 D5**：探针源在**测试进程内生成**、组件由 `sico_cli` 单独编译、**不进 `selfhost/`**、**不进 A/B/C 链**、**不进自举形状前沿**；测量用**与合格跑相同的限额**（fuel 800 B / timeout 120 s / runner 默认 2560 MiB 内存上限）。
- **三种组装策略实测**（Runtime **无** fuel 耗量读数出口、**无** guest 峰值内存读数出口 ⇒ 按 D13 记**所需上限**＝上界，**显式标注**、**不阻塞 GO**；失败按 `arena-exhausted`（注册在 `unreachable` 上的 Wasm trap）/ `fuel-exhausted`（typed）**分类**不合并）：**S1 增量 `sico.text.concat`（O(n²)）** 最大成功 **128 KiB**，256 KiB ⇒ `arena-exhausted`；**S2 分块 `sico.arena.push_chunk`（chunk 表每次 push 重建 ⇒ O(n²)）** 最大成功 **256 KiB**，384 KiB ⇒ `arena-exhausted`；**S3 线性对照（一次 `concat` 两个大操作数，O(n)）** **1 MiB = 66 ms / fuel 上界 24,117,785**、**4 MiB = 133 ms / fuel 上界 96,469,529**。同一个 256 KiB 输出，S2 比 S3 贵 **180 倍**燃料 ⇒ **撞墙的是策略的二次方行为，不是规模**。
- **D6 三档判读 = 可行**：1 MiB 在 (arena ≤ 2 GiB, fuel ≤ 800 B, wall ≤ 120 s) 内达成 ✅（66–78 ms 多轮观测，fuel 上界 24,117,785 = 限额的 **0.003%**），且 256 KiB→1 MiB 增长 **1.8×–2.3× ≤ 6×**（**墙钟是机器相关的观测值** ⇒ 文档与钉都按**区间**记录，钉**不钉墙钟**、只钉结构事实与阈值余量）⇒ **T-R3-a 止损不触发**，W2/W3 顺序推进**继续**。
- **附带一条实施约束（落进 W2 的族实现方式）**：**字节组装必须是线性的**——现有 guest codegen 的增量拼接形状在 1 MiB 量级**必然**撞 arena；计划 D7 把「②二进制通道」排在「①流式/分块」之前，但**实测表明只分块不够**（S2 也是二次方）⇒ W2 的 codegen 刀**不得**沿用现有增量拼接形状。**这条不是泛泛提醒，而是对现有代码的直接诊断**：guest 现在的 `cg_leb_hex` / `cg_hex_byte` / `cg_section_hex`（`selfhost/codegen.sico`）就是**层层嵌套的增量拼接**（= 实测失败的 S1）⇒ 按现状 guest 在 **128 KiB** 输出量级就耗尽 arena，而目标 hex 缓冲是 **2,049,226 B**（≈2 MiB，**16 倍**）⇒ **不先改组装路径，W5 在规模上根本走不到**。**M5 折算**：目标只需**常驻 ≈3,381,223 B**（canonical IR JSON ≈1.3× + hex 缓冲 2,049,226 B）⇒ 只比「S3 已实测成功的 256 KiB」大 **12.9×**，远在可行区内。
- **撞到并修掉 4 条自造缺陷**：①**内存上限按计划字面写成 2 GiB ⇒ 全部 `MemoryLimit`**——计划「arena 2 GiB」是 **guest arena**（`canonical::ARENA_LIMIT`，编译期常量，活在 guest 线性内存**里面**），而 runner 的 `memory_bytes` 是**线性内存上限**，必须覆盖**声明值 33,792 页 = 2,214,592,512 B**，否则 wasmtime 无法满足声明最小值 ⇒ **STEP-0497 失败形态**；②**S3 的大操作数写成源字面量 ⇒ 测成源契约**（`LineTooLong at bytes 453..1049045`）⇒ 改从 **stdin** 进；③**`sico.list.empty[Text]()` 不是合法调用目标**（bracket 单态只声明 `I64`/`U64`）⇒ 改 `sico.text.split_lines("")`；④`Point` 未 `Clone` + `min_fuel` 重复定义。
- **出口**：1 条测量钉（3 条结构化断言：S3 达 1 MiB 且增长 ≤ D6 可行阈值 / S1 仍撞 arena 且到不了 1 MiB / 实测内存下限 == 声明值）+ 证据文件 `target/tmp/step0516/scale-measurement.txt`（30 行）。
- **未测项**：M2（通用 bytes→hex 吞吐 —— runtime **没有** hex 内建，只能测 guest 自写 hex 循环 ⇒ 属实现面）、M3（大对象共驻）、M4（Rust 侧墙钟）⇒ 留后续片。
- **下一刀**：按计划 §3.3.5 的族模板继续 **W2**（20 个宿主内建族四拍：真值测量 → 前端刀 → codegen 刀 → 收口片；**D9 禁止并行开族**），**族序按 S 内调用点降序**（T1 `sico.text.concat` 2,882 处最多）。**本片给 W2 的实施约束**：codegen 刀必须按**线性组装**设计。

**STEP-0523（2026-10-23，closed；本机全绿；已提交 `957887c` 并推送到 `codex/m22-w1-ci`；**独立 CI run #30 = success ✅**）**：**W2 族 T1 的 G-d 刀（第一刀）= 单语句 `return` 的通用操作数 ⇒ 前端路由复用 `let` 绑定路由的操作数降低器**（[STEP-0523](./steps/STEP-0523-m22-w2-s5-w2-t1-gd-single-return-route.md)；**族仍未闭合**）
- **真值（生产输出，非设计）**：改前 7 个「函数体只有一条 `return`、表达式是 `sico.text.concat`」的形状**全部**在前端被拒（`ERR:E-SH-IR-EXPRESSION`），而 Rust 后端**全部**能降 ⇒ **改后 6 个形状逐字节等同**（组件 1124 / 1113 / 1112 / 1177 / 1177 / 1208 / 1260 B）。
- **落点**：`selfhost/parser.sico` **重塑 1 个函数** `direct_text_concat_return_ir`（22342 → **22304 行**，**−38 行**；函数数 174 → 174）⇒ 路由**不再自己抄一份操作数规则**，改为复用 `let` 绑定路由一直在用的 `straight_text_concat_packed`。
- **失败语义保持不变**：降低器的 typed 身份被路由**转成 `SKIP`** ⇒ 最终仍是原来的 `ERR:E-SH-IR-EXPRESSION` ⇒ **没有偷偷换身份**。
- **自举形状前沿没有推进**：重塑后的路由与 `straight_text_concat_packed` 改前改后**都**在前沿内 ⇒ 新形状是**复用**既有形状得到的。
- **本片最重要的发现 = G-d 其实是两个缺口**：S 内那 10 个调用点逐个定位后，剩下的 4 个**全部**在 `if` 块里，而它们的拒绝**不在前端** —— 前端**已经**逐字节降下来了，拒的是**组件发射**（guest 的发射器只支持单块直线代码 ⇒ **任何** `if` 都撞上它）⇒ 拆成 **G-d1**（前端，✅ 本片闭合，6 / 10）与 **G-d2**（**发射端分支缺口**，⏳ 未闭合，4 / 10）；**G-f**（调用型操作数带两个实参）仍在前端 ⇒ S 内 1 个调用点（**⚠️ 口径订正（STEP-0524）**：这是 **G-d 子集**的计数 —— **全 S** 的这种形状（G-f 面）实测是 **117** 个调用点，见 STEP-0524 §5）。
- **订正一条过期事实**：STEP-0518 的「guest 前端拒前向引用的用户函数调用」**实测不成立** ⇒ 注释已改，闭包清单改成**从文件自动计算**。
- **出口**：**2 条新钉** + **真值表扩到 24 行**（H / I / J / R 翻成「逐字节等同」，新增 6 行，引入 `GuestVerdict::CodegenRefused`）+ **前沿探针 34 → 38** + 固件 **145 → 147**（**147 passed / 0 failed**）+ 本机验证电池 **17/17 `BATTERY_GREEN`**（`target/tmp/step0523/battery.ps1`）。
- **诚实收窄**：**T1 族仍未闭合**（**G-d2** 4 个 + **G-f** 1 个调用点 ⇒ 按 **D10 不得降级**）；**语料覆盖仍 0 条**；**整文件自举仍未关闭**；**发射端零改动**；**门 3 未动，M22 仍 NO-GO**。
- **下一刀**：**G-d2（发射端的分支支持）** —— T1 族剩下的最大硬阻塞，但它**不是 T1 专属**：guest 的发射器只支持单块直线代码 ⇒ 建议**先当独立条目登记进计划**再决定是否单开一片；随后 **G-f（两实参调用）**（改的是 `let` 路由共用的降低器 ⇒ 单独成片 + 整表复跑）⇒ 之后才是 **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + **独立 CI** 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`）。
**STEP-0524（2026-10-23，implementation complete locally；本机全绿，随本片提交并推送；独立 CI 裁决待下一批回填）**：**W2 族 T1 的 G-f 刀 = 调用型操作数带多个实参 ⇒ 前端逐字节等同**（[STEP-0524](./steps/STEP-0524-m22-w2-s5-w2-t1-gf-multi-argument-call-operand.md)；**族仍未闭合**）
- **真值（生产输出，非设计）**：S 的真实形状 `sico.text.concat(at(items, U64.literal(1)), sico.text.concat(".", at(items, U64.literal(2))))` 的 Rust canonical IR = **3513 B**、生产组件 = **1229 B**；改前 guest typed 拒绝（`ERR:E-SH-IR-EXPRESSION`）⇒ 改后**逐字节相同**；另 3 个变体（两个形参 **3036/1185**、三个实参 **3189/1193**、两个字面量实参 **3120/1211**）**全部逐字节等同**。**Rust 前端把 `U64.literal(1)` 降成 `const_u64`**（range 只覆盖那个数字）—— 从 oracle 读出，**不是**设计。
- **落点 = 纯插入**：`selfhost/parser.sico` **新增 1 个函数** `stc_call_operand_packed`（**138 行**）+ 把 `straight_text_concat_packed` 调用分支的 `else`（原来直接 `ERR:E-SH-IR-CALL-ARITY`）**换成一次分派**（+17 行）⇒ 文件 22304 → **22458 行**（**+154**），函数数 174 → **175**；**单实参路径一个字节都没动** ⇒ `let` 路由 6 行与 1 实参调用行**逐字节不变**（真值表实测）——该降低器是**全族热点**，故只加路径、不改旧路（最坏退回「1 个实参照旧、多个实参照旧被拒」，**不连累既有形状**）。
- **失败语义不变**：typed 身份透传 ⇒ 路由转 `SKIP` ⇒ 仍 `ERR:E-SH-IR-EXPRESSION`；**实参 0 个仍 `ERR:E-SH-IR-CALL-ARITY`**（不静默放宽）。**自举形状前沿没有推进**。
- **撞到并修掉 2 条自造缺陷**：①实参区间**左闭右开** ⇒ 改成「`)` 的下一个词 == 区间末端」；②**臂内嵌套 `match` 不被 guest 的通用 CFG 路由支持**（`ERR:E-SH-IR-STATEMENT`）⇒ 尾部改成「计数器 + 纯 `return`」⇒ 顺带把 `straight_text_concat_packed` 那份**手工维护**的闭包清单改成**自动计算**（`parser_closure_source`）。
- **出口**：**2 条新钉** + **真值表 24 → 29 行**（Z 行翻成「逐字节等同」；新增 3 行逐字节等同 + 1 行 `I64.literal(-n)` 实参仍前端 typed 拒绝 + **1 行「实参本身是嵌套调用」仍前端 typed 拒绝（BF 行）**）+ **前沿探针 38 → 39** + 固件 **147 → 149**（**149 passed / 0 failed**）。
- **独立条目登记**：**G-d2（发射端分支支持）已登记为非 T1 的独立计划条目**（入口门槛 + 出口判据写全）——它要的是**发射器的新特性**（多块发射 + Wasm `block/if/else/br` + 分支值槽），**任何**用分支的族都会撞上它。
- **S 内 G-f 普查（新增证据）**：G-f 面 = `sico.text.concat` 的操作数是**用户函数调用**、被调函数返回 `Text`、实参 ≥ 2 个 ⇒ S 内 **117** 个调用点（其中 **95** 个的实参全部落在新路径认领的集合内、**22** 个带**嵌套调用实参**）⇒ **订正** STEP-0523 记的「S 内 1 个调用点」（那是 **G-d 子集**口径）⇒ **订正** STEP-0523 记的「S 内 1 个调用点」（**G-d 子集**口径）⇒ **新登记 G-f2** = 调用型操作数的**实参本身是嵌套调用**（前端形状缺口；真值表 **BF** 行钉住：Rust canonical IR **3635** / 生产组件 **1267**，guest 拒）。
- **诚实收窄**：**T1 族仍未闭合**（**G-d2** 4 个调用点 + **G-f2** 22 个调用点 ⇒ 按 **D10 不得降级**）、**未认领** `I64.literal(-n)` 实参（S 内不存在）与**嵌套调用实参**、**语料覆盖仍 0 条**、**整文件自举仍未关闭**、**发射端零改动**、oracle 生产代码 **零改动**、**门 3 未动，M22 仍 NO-GO**。
- **下一刀**：**G-f2（嵌套调用实参）** —— 本片**新登记**的 T1 前端形状缺口（S 内 22 个调用点）⇒ 它要的是**操作数降低器能递归地降一次调用作为实参**；其后是 **G-d2（发射端分支支持）** —— 已作为**独立条目**登记进计划（不是 T1 的某一拍）⇒ 决定是否单开一片；其后才是 **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + **独立 CI** 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。
**STEP-0522（2026-10-23，closed；本机全绿；已提交 `8bdd58d` 并推送到 `codex/m22-w1-ci`；**独立 CI run #29 = success ✅**）**：**W2 族 T1 的 G-b 刀 = 转义字面量解码 + 空字面量（G-e）⇒ 组件与 Rust 后端生产输出逐字节等同**（[STEP-0522](./steps/STEP-0522-m22-w2-s5-w2-t1-escaped-literal-decoder.md)；登记表 [m22-support-surface-t1-text-concat.md](./reports/m22-support-surface-t1-text-concat.md)）。**本片是 T1 族的收口前置片之一（不是收口片 X）；族仍未闭合（G-d 在 S 内真实存在）；不推进门 3**。
- **真值（生产输出，非设计）**：Rust 后端（`sico build --profile script-v0`）对 `"tab\there"` + `"nl\nhere"` 的静态 data 段 = **15 B** `74616209686572656e6c0a68657265`（`tab<TAB>here` + `nl<LF>here`）⇒ **后端放的是解码后的字节**（TAB=`09`、LF=`0a`），不是源码里的两个字符 `\t`；**改前 guest 对同一 fixture 是 typed 拒绝**（`ERR:E-SH-CG-IR-EMIT`），**改后逐字节相同**（1197 B）。
- **核心改动 = 字面量的身份换成它的解码后字节**（hex 字节对）：`cgs_literal_text` → `cgs_literal_hex`；`cgs_data_offset` 改为按**字节**去重与累计；`cgs_data_hex` 直接用 hex ⇒ 与后端 `script_data`（`value.as_bytes()` 做 key）**逐字节同构**（否则「同一字节两种拼法」会被当成两个字面量 ⇒ 偏移错 ⇒ **静默错字节**）。
- **新增 4 个函数**：`cgj_string_hex`（JSON 串 → 解码后字节的 hex；不可解码 ⇒ `ERR:E-SH-CG-IR-EMIT`）/ `cgj_escape_hex`（`\"`→`22` `\\`→`5c` `\n`→`0a` `\r`→`0d` `\t`→`09`）/ `cgs_is_const_string`（把**空字面量**与**不是字面量**分开）/ `cgs_hex_bytes`（按 2 步长计对 ⇒ **不引入除法**，因为 `U64.checked_div` 这条路由**不在** guest 前端的路由梯里，实测 `ERR:E-SH-IR-CALL-TARGET`）。
- **顺带闭合 G-e（普查时新发现）**：空字面量 `""` 此前也被拒（同一个哨兵把「空」和「不是字面量」混为一谈）⇒ 后端对它的处理是「进 `data_offsets`、贡献 0 字节、指针 = 当前 data 长度」，guest 改后**完全一致**。
- **S 内普查（口径：文本调用点，`target/tmp/step0522/census_s_literals.py`）**：**2,943** 个 `sico.text.concat(` 调用点（STEP-0521 记 2,941；差的 2 个是**本片新增**的两处 concat）；按**操作数**计 5,888 个 = 名字 3,308 / 调用或成员 1,004 / **带转义的字面量 952**（⇒ **G-b** 硬阻塞） / 普通字面量 619 / **空字面量 5**（⇒ **G-e** 硬阻塞）⇒ **G-d** 仍是硬阻塞 ⇒ 按 **D10 不得降级**。
- **自举形状前沿反而被拉回**：`cgs_literal_text` 曾在前沿外（`ERR:E-SH-IR-CALL-ARITY`）⇒ 搬到**字节域**后不再需要任何 text 形状的宿主调用 ⇒ 8 个新增/改动函数**全部**能被 guest 逐字节降低 ⇒ **未推进形状边界**。
- **落点**：`selfhost/codegen.sico` **新增 5 个**（`cgs_is_const_string` / `cgs_literal_hex` / `cgj_string_hex` / `cgj_escape_hex` / `cgs_hex_bytes`）+ **重塑 3 个**（`cgs_data_offset` / `cgs_const_string_hex` / `cgs_data_hex`）+ **删 2 个**（`cgs_literal_text` / `cgs_text_bytes_hex` 死代码）⇒ 文件 4303 → **4413 行**（**+110 行**），函数数 149 → **152**；`runner/sico-runner/tests/selfhost_compiler.rs` **新增 4 条钉** + **改** STEP-0521 的族级真值表钉（L / O / Q 三行 ⇒ 逐字节等同 **12 → 15** 行、组件层 typed 拒绝 **3 → 0** 行；`GuestVerdict::CodegenRefused` 变体随之无行使用 ⇒ 删除，否则 `dead_code` 会让 clippy `-D warnings` 变红）；`tools/probe-m22-frontend-frontier.ps1` **新增 3 条用例**（31 → **34**）；oracle 生产代码与 `selfhost/parser.sico` **零改动**。
- **撞到并修掉 3 条自造缺陷**：①`\u0000` 只消费了 2 个字节（4 个 `0` 被当字面量 ⇒ guest data 段多 `30 30 30 30`）⇒ **差分测出来**（两侧都 1429 B 但字节不同）⇒ 改成连 4 个数字一起消费；②形参名用了保留字 `end` ⇒ Rust 前端拒 `unsupported atomic expression at bytes 94054..94057` ⇒ 改名 `close`；③哨兵歧义 ⇒ 改用 `ERR:E-SH-CG-IR-EMIT` 标记解码失败。
- **出口**：**4 条新钉** + 固件 **141 → 145**（**145 passed / 0 failed**）+ 前端前沿探针 **34/34** + 本机验证电池 **16/16 `BATTERY_GREEN`**（`target/tmp/step0522/battery.ps1`；**16 步** —— STEP-0521 的两条足迹审计被本片的两条替换）。
- **诚实收窄**：①**T1 族仍未闭合**（**G-d** 在 S 内真实存在 + 本片独立 CI 待跑）⇒ 族级出口判据**五项缺一不算闭合**；②**语料覆盖仍 0 条**；③**整文件自举仍未关闭**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）⇒ **门 3 未动，M22 仍为 NO-GO**；④**未测边界**：源码字面量里**裸控制字符**（如裸 TAB）的 IR 逐字节等同**未测未认领**（数据段字节仍正确）；⑤**已知风险**：`cgs_data_offset` 每个 `const_string` 扫一遍整个模块 ⇒ **O(字面量数²)** ⇒ **W5 的 1 MB 级规模必须重测**；⑥guest codegen 仍只认两个内建；⑦oracle 生产代码与 `selfhost/parser.sico` **零改动**；**未提交未推送**。
- **下一刀**：**G-d（单语句 `return` 的非形参操作数，前端）** —— S 内 10 个调用点的硬阻塞（能力已存在，缺的是可达性）。做完 ⇒ **T1 收口片 X**（真值表复跑 + D15 登记表定稿 + 前沿探针 + 电池 + **独立 CI** 回填）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。

**STEP-0521（2026-10-23，implementation complete locally；本机全绿；已随 STEP-0506…0521 批次提交 `a5e0b65` 并推送；**独立 CI run #28 = success ✅**）：**W2 族 T1 族级真值表与支持面登记（D15）= 20 行真值表钉进固件 + 修正一处静默错字节的模块级 data 段偏移**（[STEP-0521](./steps/STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)；登记表 [m22-support-surface-t1-text-concat.md](./reports/m22-support-surface-t1-text-concat.md)）。**本片是 T1 族的收口前置，不是收口片 X；族仍未闭合；不推进门 3**。
- **为什么先做它**：前三拍（真值 T / 前端刀 F / codegen 刀 C）+ 共享前置（cell）都只钉**一个形状** ⇒ 「族支持到哪一步」从来只是一串逐片声明，不是**一张表**；D15 又要求每族闭合时登记「族 × 子集边界 × 语料覆盖 × 未覆盖形状」⇒ **先把族级事实钉死，再谈收口**。
- **族级真值表（20 行，四轴实测：Rust 前端 IR / Rust 后端组件 / guest 前端 / guest 组件）**：**逐字节等同 12 行** / **组件层 typed 拒绝 3 行**（L / O / Q：字面量含 JSON 转义）/ **前端 typed 拒绝 4 行**（H / I / J / R：函数体只有一条 `return` 且操作数不是两个形参）/ **修正前静默错字节 1 行**（S：两个函数各带一个字面量 ⇒ 组件**同长但字节不同**，数据指针差 1）⇒ 整张表**钉进固件**（18 行 + 2 行两条钉）。
- **方法学复现**：「某形状被拒绝」≠「该形状没实现」—— H / I / J 被拒是**路由门**（`body_statement_count = max(let_count, set_count) > 0`），不是能力缺失：加一条无关语句后**同样两个操作数**（K / N / M）就逐字节降下来了 ⇒ 前沿探针新增 **3** 条用例（28 → **31**），三条全判 `lowered`。
- **修正的到底是什么（G-a / G-c）**：后端 `script_data` 收**整个模块**的字面量（函数 → 块 → 指令）并**对重复字面量去重**；guest 的 `cgs_data_offset` 却只扫**当前函数** ⇒ 前面的函数也带字面量时偏移偏小 ⇒ **长度与 oracle 相同、字节不同**（字节 612 处 `00` vs `01`）⇒ **这是 fail-closed 纪律唯一不允许的结果**（静默发出错误字节）；修正后与 `cgs_data_hex` **同构**（模块级遍历 + 去重 + 首次出现解析）⇒ 同时修掉**重复字面量被误拒**（G-c）。
- **自举形状前沿反而被拉回**：修正前这两个函数带**嵌套宿主调用**（`sico.bytes.length(sico.text.encode(...))`）⇒ guest 前端报 `ERR:E-SH-IR-CALL-SHAPE` ⇒ **在前沿之外**；按 STEP-0515 的既有修法拆成临时变量后，`cgs_data_offset`（28,205 B）与 `cgs_const_string_hex`（9,963 B）**现在都能被 guest 逐字节降低**（新钉）⇒ **未推进形状边界**。**`cgs_literal_text` 仍在前沿之外**（`ERR:E-SH-IR-CALL-ARITY`）⇒ G-b 的实现必须同时解决它。
- **S 内普查（口径：文本出现次数，`target/tmp/step0521/census_s.py`）**：**2,941** 个 `sico.text.concat(` 调用点；**1,576** 个的操作数是字面量（⇒ **G-b** 是固定点硬阻塞）；**10** 个落在没有 `let`/`set` 的函数体里且**没有一个**是形参 ×2（⇒ **G-d** 是硬阻塞）⇒ 按 **D10 不得降级**。**口径提示**：STEP-0520 记的 **2,857** 是 **IR 级**计数，与本表的**文本**计数口径不同、**不矛盾**。
- **落点**：`selfhost/codegen.sico` **重塑 1 个函数** `cgs_data_offset` + **改 1 处** `cgs_const_string_hex` + **改 1 段注释** `cgs_data_hex` ⇒ 文件 4278 → **4303 行**（**+25 行**），函数数 149 → 149；`runner/sico-runner/tests/selfhost_compiler.rs` **新增 3 条钉**；`tools/probe-m22-frontend-frontier.ps1` **新增 3 条用例**；**新增** `docs/reports/m22-support-surface-t1-text-concat.md`（D15 登记表）；oracle 生产代码与 `selfhost/parser.sico` **零改动**。
- **撞到并修掉 2 条自造缺陷**：①**普查数字与钉住数字打架**（转写进证据文件的那一行形状描述与探针源码不一致 ⇒ 期望值 2446/1126，实测 **2485/1137**）⇒ 改成**在钉的同一 fixture 里实测**再写进钉（**证据文件的数字必须与产生它的那份源码同源**）；②**电池里的一个「假红」**：前沿探针的 `MOVED` 实际是 CLI 的 `source cache failure: Io(...)` **环境抖动**（不是语义移动）⇒ 单独复跑 `FRONTIER_UNCHANGED` ⇒ 整轮重跑电池；**探针脚本把「环境抖动」与「语义移动」混标成 `MOVED`** 是**已知弱点**（登记为改进项）。
- **出口**：3 条新钉 + 固件 **138 → 141**（**141 passed / 0 failed**）+ 前端前沿探针 **31/31** `FRONTIER_UNCHANGED` + 本机验证电池 **17/17 `BATTERY_GREEN`**（`target/tmp/step0521/battery.ps1`，17 步）。
- **改动面自证**：`target/tmp/step0521/audit_footprint.py` 把本片 **3 处**改动逐条反向撤销 ⇒ `STEP0521_FOOTPRINT_GREEN edits_reversed=3`（重建 **4278** 行 == STEP-0520 记录的前态，**+25 行**）；**独立见证**：重建文件再喂给 **STEP-0520 的审计脚本**（独立写的 11 处反向撤销）⇒ **3980 / 4278 / +298** 与 STEP-0520 当时记录值**逐个相同**。
- **诚实收窄**：①**T1 族仍未闭合**（**G-b** / **G-d** / **独立 CI**）⇒ 族级出口判据**五项缺一不算闭合**；②**语料覆盖仍 0 条**；③**整文件自举仍未关闭**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）⇒ **门 3 未动，M22 仍为 NO-GO**；④**已知风险**：`cgs_data_offset` 现在每个 `const_string` 扫一遍整个模块 ⇒ 偏移推导 **O(字面量数²)** ⇒ **W5 的 1 MB 级规模必须重测**（STEP-0516 的「组装必须线性」教训在此同样适用）；⑤guest codegen 仍只认两个内建；⑥oracle 生产代码与 `selfhost/parser.sico` **零改动**；**未提交未推送**；**独立 CI 尚未运行**。
- **下一刀（已由 STEP-0522 实施）**：**G-b（转义字面量，全族共享前置）** —— guest 要能反转义 JSON 字符串（`\\` `\"` `\n` `\r` `\t` `\u0000`），且 `cgs_literal_text` 必须**同时**被拉回自举形状前沿内；**G-d（单语句 `return` 的非形参操作数，前端）** —— S 内 10 个调用点的硬阻塞（能力已存在，缺的是可达性）。G-b 已闭合（G-e 一并闭合），**下一刀 = G-d** ⇒ 之后是 **T1 收口片 X**（真值表复跑 + 登记表定稿 + 前沿探针 + 电池 + **独立 CI**）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。

**STEP-0520（2026-10-23，implementation complete locally；本机全绿，未提交未推送）**：**W2 族共享前置（cell 支持）= `write_local` / `read_local` ⇒ 整组件与 Rust 后端生产输出逐字节等同（1114 B）**（[STEP-0520](./steps/STEP-0520-m22-w2-s5-w2-shared-local-cell-support.md)）。**本片不是 T1 族的第 4 拍，而是 W2 全部 20 个族的共享前置**；**族仍未闭合**（缺收口片 X 与独立 CI）；**不推进门 3**。
- **为什么必须先做它（D10 实测）**：S 内 `sico.text.concat` 调用点 **2,857** 个里 **2,843（99.5%）** 落在**带 cell 的函数**里 ⇒ **T1 族不得降级**；且 `let` 绑定一旦被 `set` 赋值，Rust 前端就把它物化成 **cell** ⇒ **任何**族都会撞同一堵墙 ⇒ **cell 支持必须先于任何族的 codegen 刀**。
- **后端真值（生产代码 + oracle 生产输出，非设计）**：`LocalLayout::plan`（`crates/sico-codegen-wasm/src/lib.rs:1812`）槽位顺序 = **形参（不声明）→ 各指令结果（每槽一组；unit 结果 0 槽 0 组）→ cells（每 cell 的 flat 类型一组）→ dispatcher（1×i32）→ Script scratch（2×i32）**；`WriteLocal` / `ReadLocal` = 逐槽 `local.get(源)` + `local.set(cell)`；oracle 组件反汇编（`target/tmp/step0520/dump_core.py`）⇒ `join` 的 locals = **5 组 (1,i32) + 1 组 (2,i32)**，槽位 `a`=0,1 `b`=2,3 concat 结果=4,5 **cell x=6,7** dispatcher=**8** scratch=9,10。
- **落点**：`selfhost/codegen.sico` **新增 8 个函数**（`cgs_is_unit_ty` / `cgs_cell_flat_hex` / `cgs_cell_ty` / `cgs_cell_width` / `cgs_cell_slots` / `cgs_cell_slot` / `cgs_write_local_hex` / `cgs_read_local_hex`）+ **重塑 4 个**（`cgs_function_local_hex` unit 结果 0 组 + cell 组 / `cgs_value_slot` unit 结果不占槽 / `cgs_dispatcher_slot` unit 结果 + cell 宽度 / `cgs_op_hex` 的 `write_local` / `read_local` 路由）⇒ 文件 3979 → **4277 行**（**+298 行**），函数数 141 → 149。
- **四道闸门全 fail-closed**：cell 类型 ∈ {bool, i64, u64, string, bytes}（**独立窄函数** `cgs_cell_flat_hex`，与后端 `lower_local_types` **逐类型对齐** —— 通用 `cgs_flat_hex` 在 `Result[I64|U64, NumericError]` 上与后端**差一槽**，故不可复用）/ cell 下标存在 / 源宽 == cell 宽 ⇒ 越界即 `ERR:E-SH-CG-IR-FLAT` / `ERR:E-SH-CG-IR-EMIT`，**不静默取近似**（ADR-0016 决策 6）。
- **出口达成**：参考形状 canonical IR **2630 B == 2630 B**（逐字节）+ 组件 **1114 B == 1114 B**（逐字节）+ oracle 与 guest 两侧**都过**真实 `wasmparser::Validator`；**cell 读**形状（2 写 2 读）组件 **1138 B** 逐字节等同 ⇒ `read_local` 真被发射；**I64 cell** 形状组件 **1116 B** 逐字节等同 ⇒ cell 子集不止 Text；既有 4 个无 cell 形状仍 **1094 B** 逐字节等同（不退化）；**自举形状前沿**：本片 **12** 个 codegen 函数经 guest 自己逐字节降低 == Rust 前端 ⇒ **未推进形状边界**。
- **撞到并修掉 3 条自造缺陷**：①**`U64.equal(<Bool>, 0)` ⇒ 前端 `E2001 expected U64, found Bool`**（`equal` 是**定宽数值**方法，Sico **没有** `Bool.equal`）⇒ 改用仓库既有的 if/else 标志惯用法；②**同一函数里两个同名 `let` ⇒ `cell redeclared`**（前端把一般 CFG 里的每个 `let` 都物化成 cell ⇒ **`let` 名字函数内唯一**）⇒ 改名并把该规则写进静态检查脚本；③**足迹审计脚本自己的还原语义错了三轮**（少 7 行 / 多 4 行 / 少 1 行）⇒ **靠独立见证发现**：把重建文件喂给 **STEP-0519 的审计脚本**，要求它给出与 STEP-0519 当时**记录值完全相同**的 3745 / 3980 / +235 ⇒ **自证脚本本身也会说谎**。
- **出口**：**6** 条新钉 + 固件 **132 → 138**（**138 passed / 0 failed**）+ 前端前沿探针 **28/28** `FRONTIER_UNCHANGED`（本片**不动前端前沿**）+ 本机验证电池 **17/17 `BATTERY_GREEN`**（`target/tmp/step0521/battery.ps1`；本片自己的 16 步电池曾在前沿探针上因 CLI 的 `source cache failure: Io(...)` **环境抖动**判红，单独复跑即 `FRONTIER_UNCHANGED`，落地后整轮重跑全绿 —— 详见 [STEP-0521 §4.5](./steps/STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)）。
- **改动面自证**：`target/tmp/step0520/audit_footprint.py` 把本片 **11 处**改动逐条反向撤销并重建出本片之前的文件 ⇒ `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（重建 **3980** 行 == STEP-0519 记录的前态行数，**+298 行**）；`check_functions.py` 再核对被重塑的 4 个函数与前态**逐函数行数相同**（60 / 66 / 54 / 55）；`check_guest.py` 核对纯 LF 无 BOM / 声明顺序 / `let` 名字唯一。
- **诚实收窄**：①**T1 族仍未闭合**（缺**收口片 X** 与**独立 CI**）⇒ **族级出口判据五项缺一不算闭合**；②**当前可达的 cell 子集是 Text 与 I64** —— `Bytes` / `List[Text]` 被**前端**挡在前（`ERR:E-SH-IR-EXPRESSION`），**记录型 cell 的前端 IR 与 Rust 前端不一致**（是**前端**差异，本片**没有**认领、也没算成组件层成果）；③guest codegen 仍只认两个内建（其它 helper 型内建仍 fail-closed）；④STEP-0517 的**三个**前端未闭合形状仍 typed 拒绝；⑤**语料覆盖仍 0 条**；⑥**整文件自举仍未关闭**（`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；⑦oracle 生产代码与 `selfhost/parser.sico` **零改动**；**未提交未推送**；**独立 CI 尚未运行**；**门 3 仍 NO-GO**。
- **下一刀**：W2 族 T1 的**第 4 拍（收口片 X）** —— 钉住族级真值表（组件 1094 B + **1114 B / 1138 B / 1116 B** 的 cell 形状 / helper 索引推导 / 条目条件化）+ 把未闭合形状逐条登记进**支持面登记表（D15）** + 前沿探针期望值确认 + 固件电池 + **独立 CI**（提交后）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。

**STEP-0519（2026-10-23，implementation complete locally；本机全绿，未提交未推送）**：**W2 族 T1 codegen 刀（C）= `sico.text.concat` ⇒ 整组件与 Rust 后端生产输出逐字节等同（1094 B）**（[STEP-0519](./steps/STEP-0519-m22-w2-s5-w2-t1-codegen-knife.md)）。**本片是 W2 族第 3 拍；族仍未闭合（缺收口片 X 与独立 CI）；不推进门 3**。
- **发射真值（实测 oracle 生产输出，非设计）**：type 段 6 条 ⇒ **type 5 = concat helper** = `60047f7f7f7f027f7f`；function 段 6 条且 **`type_index` = 自身下标**；**export 段不变**；code 段 6 个 body ⇒ helper body **42 B** = `01027f`（1 组 × 2 个 i32 局部）+ `200120036a2205`（总长 `a_len + b_len` → `local.tee 5`）+ `10 <alloc>`（arena 分配）+ `21 04`（结果 pointer）+ 两段 `MemoryCopy` + `200420050b`（返回 `(ptr, len)`）；**helper 的 core function index = 模块函数数 + 3**（后端 `helper_count = import_count + module.functions.len()`；`used_helpers` 按名字排序 ⇒ `sico.text.concat` 恒为 `i = 0`；Script profile 的 `import_count = 0`）。
- **落点**：`selfhost/codegen.sico` **新增 4 个结构化扫描函数**（`cgs_instruction_is_text_concat` / `cgs_block_has_text_concat` / `cgs_function_has_text_concat` / `cgs_module_has_text_concat`，93 行）+ **新增 `cg_concat_helper_body_hex`**（9 行）+ **搬家 `cgs_function_count`**（函数体**逐字节不变**，上移到 `cgs_intrinsic_hex` 之前）+ **重塑 4 个**（`cgs_intrinsic_hex` 新增 concat 分支、`cgs_core_types_hex`/`cgs_core_funcs_hex`/`cgs_core_code_hex` 三段条件化）⇒ 文件 3744 → **3979 行**（**+235 行**）。
- **四道闸门全 fail-closed**：名字 / 实参个数 = 恰好 **2** / 实参类型 = 两个 `string` / 结果槽数 = 恰好 **2** ⇒ 任一不过即 `ERR:E-SH-CG-IR-EMIT`，**不静默取近似**（ADR-0016 决策 6）。**元数边界仍在语义层**（1/3 实参被 Rust 前端自己拒 `stdlib intrinsic arity`）⇒ guest **不自造**元数检查。
- **「模块用没用到 concat」= 结构化扫描**（`functions` → `blocks` → `instructions` → `operation.op == "intrinsic"` → `operation.data.name == "sico.text.concat"`），**不是子串搜索**；每级一个函数、**每函数只有一个 `while`**（flag 模式 + early return，**不用嵌套 `continue`**）。
- **helper 条目条件化**：不用 concat 的模块三段**保持三条目** ⇒ 既有 host-call 组件 **1025 B 逐字节不变**（新钉）。
- **出口达成**：整组件 **1094 B == 1094 B**（与 oracle 生产输出**逐字节相同**）+ oracle 与 guest 两侧**都过**真实 `wasmparser::Validator`；**helper 索引是推导的**（三函数模块 ⇒ 索引 6，组件仍逐字节等同 ⇒ 新钉）；**自举形状前沿**：本片 10 个 codegen 函数经 guest 自己逐字节降低 == Rust 前端 ⇒ **未推进形状边界**（新钉）。
- **撞到并修掉 3 条自造缺陷**：①**直线体里 `let` 跟在 `set` 之后 ⇒ `ERR:E-SH-IR-CALL-TYPE`**（6 个最小变体实测 ⇒ 直线体必须**所有 `let` 在任何 `set` 之前**；同一写法**在 `if` 分支内部可降** ⇒ 规则**逐路由**、不可外推）⇒ 把 `let` 上移；②**准备片的反汇编脚本把 locals 段丢了**（helper body 一度被读成 39 B 而非 42 B）⇒ 靠**与 guest 已冻结常量比对**发现（三个 arena body 全差一个 locals 前缀）⇒ 真值改从后端 `emit_concat` 源码逐条对出 ⇒ **反汇编工具本身也要被交叉验证**；③**「callee 必须先声明」逼出一次函数搬家**（concat 分支需要模块函数数）⇒ `cgs_function_count` 原样上移（**本片不是纯插入**，是 codegen 刀的真实成本，记此备查）。
- **出口**：3 条新钉 + **翻转** 1 条 STEP-0517 真值钉（`sico_codegen_guest_refuses_the_text_concat_component_outside_the_subset` → `oracle_text_concat_component_truth_is_pinned`）+ 固件 **129 → 132**（**132 passed / 0 failed**）+ 前端前沿探针 **28/28** `FRONTIER_UNCHANGED`（codegen 刀**不动前端前沿**）+ 本机验证电池 **11/11 `BATTERY_GREEN`**（`fmt-runner` 3s / `ci-one-key` 430s / 0245 148s / 0262 145s / 前端前沿 193s / 计划层 25s / 0124 1s / 0131 3s / 0156 0s / diff-check 0s / 行尾 0s，行尾审计 **26** 个文件，含本片新增的 STEP-0519 文档）。
- **改动面自证**：本片**不是**纯插入 ⇒ `target/tmp/step0519/audit_footprint.py` 把本片的**九处改动逐条反向撤销**并重建出本片之前的文件 ⇒ `STEP0519_FOOTPRINT_GREEN edits_reversed=9`（**+235 行**，换算成 LF 口径即 **3744 → 3979 行**）；`target/tmp/step0519/audit_codegen_delta.py` 再机械检查：纯 LF 无 BOM、`cgs_function_count` **只声明一次**且**函数体逐字节不变**、本片新增函数**全部先于首次使用声明**。
- **诚实收窄**：①**族仍未闭合**（缺**收口片 X** 与**独立 CI**）⇒ **T1 族 3/4 拍完成**（族级出口判据**五项缺一不算闭合**）；②guest codegen **只认一个 helper 型内建**（其它 helper 型内建仍 fail-closed）；③STEP-0517 的**三个**前端未闭合形状仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`；④**语料覆盖仍 0 条**；⑤**整文件自举仍未关闭**；⑥oracle 生产代码与 `selfhost/parser.sico` **零改动**；**已随 STEP-0506…0521 批次提交 `a5e0b65` 并推送到 `codex/m22-w1-ci`**（**独立 CI run #28 已触发、尚未出结论** ⇒ 族级出口判据的「独立 CI 绿」一项仍 pending）；**门 3 仍 NO-GO**。
- **下一刀**：W2 族 T1 的**第 4 拍（收口片 X）** —— 钉住族级真值表（组件 1094 B / helper 索引推导 / 条目条件化）+ 把未闭合形状逐条登记进**支持面登记表（D15）** + 前沿探针期望值确认 + 固件电池 + **独立 CI**（提交后）⇒ 之后才允许开 **T2 族**（`sico.text.length`，S 内 318 处）。

**STEP-0518（2026-10-23，implementation complete locally；本机全绿，未提交未推送）**：**W2 族 T1 前端刀（F）= `return sico.text.concat(<形参>, <形参>)` ⇒ canonical `intrinsic` IR 与 Rust 前端逐字节等同**（[STEP-0518](./steps/STEP-0518-m22-w2-s5-w2-t1-frontend-knife.md)）。**本片是 W2 族第 2 拍；族仍未闭合；不推进门 3**。
- **落点**：`selfhost/parser.sico` 新增**专用顶层路由** `direct_text_concat_return_ir`（**+146 行**，含 2 行注释）+ `scalar_ir` 路由梯接线（**+24 行**，插在 `direct_host_call_let_ir` 之后、`fixed_guard_chain_ir` 之前）= **+170 行 / −0 行**、**纯插入、零既有行重排**（**自证**：把两块还原后行级 diff ⇒ `removed_lines=0`，`target/tmp/step0518/audit_pure_insertion.py`）。
- **不新造机器**：复用既有 `parameters_ir` / `intrinsic_instruction_json` / `return_expression_start` / `return_expression_end` / `binding_parameter_index` / `binding_parameter_kind` / `is_intrinsic_path` / `matching_close` / `gw_find_code_text`。**为什么不调用 13 实参的 `straight_text_concat_packed`**：本形状两个操作数**都是形参** ⇒ canonical IR 就是**一条 `intrinsic`**（与 STEP-0512/0515 的 `direct_host_call_return_ir` / `direct_host_call_let_ir` **完全同族**），且那个机器的 13 实参调用点**从未被自举前沿证明过**可降（`straight_cell_function_ir` / `straight_environment_function_ir` 不在任何闭包钉里）⇒ 放弃是**为了让闭包可自证**。
- **支持面 = 恰好一个形状**：**2 个 `Text` 形参**、声明返回 `Text`、函数体**唯一语句**是 `return sico.text.concat(<Text 形参>, <Text 形参>)`；**闸门全 `SKIP`**（形参数 / 声明返回类型 / 体内语句数 / 调用路径 / 调用形状 / 两个操作数的类型）⇒ **既有拒绝身份不变**。
- **出口达成**：guest canonical IR **2295 B == 2295 B**（与 Rust 前端**逐字节相同**；钉住 intrinsic 形状 `{"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}`、range **374..396**、terminator `{"kind":"return","data":2}`）。
- **没有静默放宽**：STEP-0517 测到的**四个**未闭合形状里**一个**被认领，其余**三个**（两个字面量实参 / 混合实参对 / 嵌套宿主调用）**仍** typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（固件钉 + 前沿探针新增 2 条用例）。
- **自举形状前沿**：新路由闭包 **20** 个函数经 guest 自己逐字节降低 == Rust 前端（新钉）⇒ **未推进形状边界**。**自举陷阱已绕开**：新路由代码里**不能**出现 `return sico.text.concat(...)`（那正是它要开启的形状 ⇒ 循环依赖）⇒ 末尾 `let` 累加 + `return` 局部变量（`direct_host_call_let_ir` 同款，已被 STEP-0515 前沿钉证明可降）。
- **撞到并修掉 3 条自造缺陷**：①**纯插入审计脚本方向写反** ⇒ 报告「删了 170 行」（恰是本片新增的行）⇒ 修正 `before=reconstructed` / `after=current`，只有 `delete`/`replace` 才算删除 ⇒ `removed=0`（**自证脚本本身也会说谎**，必须与「本片声称做了什么」交叉核对）；②**新路由若调用 13 实参的机器 ⇒ 闭包无法自证**；③**新路由的代码不能自己用 `return sico.text.concat(...)`**（循环依赖）。
- **出口**：2 条新钉 + 1 条邻接钉更新 + 前沿探针**翻转** 1 条期望值、新增 2 条（**26 → 28** 条，`FRONTIER_UNCHANGED`）+ 固件 **127 → 129**（**129 passed / 0 failed**）+ 本机验证电池 **11/11 `BATTERY_GREEN`**（`fmt-runner` 3s / `ci-one-key` 426s / 0245 135s / 0262 149s / 前端前沿 186s / 计划层 25s / 0124 0s / 0131 1s / 0156 0s / diff-check 0s / 行尾 0s，行尾审计 **25** 个文件）。
- **诚实收窄**：①**组件层仍 fail-closed 拒绝**（`cgs_intrinsic_hex` 只认 `sico.bytes.length`）⇒ **T1 族 2/4 拍完成、尚未闭合**（族级出口判据**五项缺一不算闭合**）；②**语料覆盖仍 0 条**（S 内 2,882 个调用点一个都没覆盖）；③两个字面量实参 / 混合实参对 / 嵌套宿主调用**仍** typed 拒绝；④**整文件自举仍未关闭**；⑤oracle 生产代码与 `selfhost/codegen.sico` **零改动**；**未提交未推送**；**独立 CI 尚未运行**；**门 3 仍 NO-GO**。
- **下一刀**：W2 族 T1 的**第 3 拍（codegen 刀 C）** —— `cgs_intrinsic_hex` 认 `sico.text.concat`（**四道闸门全 fail-closed**：名字 / 实参个数 = 2 / 实参类型 = `string` / 结果槽数 = 2），并**发射 helper 函数体**（arena 分配 + 两段 `MemoryCopy`，**线性**）⇒ 出口 = 整组件 **1094 B** 与 oracle 生产输出**逐字节相同** + 过真实 `wasmparser::Validator`。

**STEP-0517（2026-10-23，measurement complete locally；本机实测绿，未提交未推送）**：**W2 族 T1 真值测量片（`sico.text.concat`）= 前端 return 位置不是「缺能力」而是「缺可达性」**（[STEP-0517](./steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)）。**本片是计划 §3.3.5 的**第 1 拍（真值测量片 T）**；不推进任何出口门；M22 仍为 NO-GO**。
- **规则表（10 个变体实测）**：let 绑定位置（形参 ×2 / cell ×2）⇒ guest **逐字节** == Rust canonical IR（**2295 B**）；**return 位置 + 体内另有 `let`** ⇒ **2295 B 逐字节**；**return 位置 + `let`+`set`** ⇒ **2630 B 逐字节**；**return 位置 + 体内只有一条 `return`**（形参 ×2 / 字面量 ×2 / 形参+字面量 / 嵌套 concat）⇒ **typed 拒绝 `ERR:E-SH-IR-EXPRESSION`**；oracle **生产组件** = **1094 B**（let 绑定 / return+let）/ **1114 B**（return+let+set）/ 1124 / 1133 / 1113 B。
- **关键发现（决定下一刀形态）**：**guest 早就实现了 T1 的 return 位置下降**——`selfhost/parser.sico:9974` 的 `straight_cell_function_ir` **本来就有** return 位置的 concat 分支，只要函数体**除 return 外还有一条语句**，同一个 `return sico.text.concat(a, b)` 就**逐字节等同**。**缺口是可达性不是能力**：路由梯（`:20579`–`:20583`）把整个 `straight_cell_*` 家族门控在 `body_statement_count = max(let_count, set_count) > 0` 上 ⇒ **体内只有一条 return 时该分支根本不被调用** ⇒ 落到标量 return 分析 ⇒ `ERR:E-SH-IR-EXPRESSION`。
- **下一刀因此被压缩**：T1 前端刀（F）**不需新造 concat 机器**（`straight_text_concat_packed` `:8929` / `gw_text_concat_packed` `:5808` 两台**既有**机器已能处理 N 个实参与嵌套调用）⇒ 只须为「体内只有一条 return、且该 return 是 `sico.text.concat`」开一条**专用顶层路由**（与 `direct_host_call_return_ir` / `direct_host_call_let_ir` 同族，**纯插入、零既有行重排**）。
- **组件层尚未开工**：`cgs_intrinsic_hex` 只认 `sico.bytes.length`（STEP-0513）⇒ `--emit-component-hex` 仍 fail-closed 拒绝 `unsupported codegen source shape`。**与 STEP-0513 不同**：`sico.text.concat` 必须**发射一个 helper 函数体**（core func 5：arena 分配 + 两段 `MemoryCopy`）+ 调用点（`call <helper>`）⇒ **线性拷贝**，正落在 STEP-0516 的「线性组装」约束内（guest 现有增量拼接路径**不得**沿用）。
- **T1 子集边界（D15 登记）**：名字 = 仅 `sico.text.concat`；实参个数 = 恰好 **2**；实参类型 = 两个 `Text`；结果槽数 = **2**（pointer, length）；**语料覆盖 0 条**（S 内 2,882 个调用点一个都没覆盖）。**元数边界在语义层**：1 个 / 3 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity` ⇒ guest **不得**自造元数检查。
- **撞到并修掉 3 条自造缺陷**：①**探针写死机器绝对路径**（`E:/github/sico-w2/...`）⇒ 独立 CI 跑在 Linux + 不同工作目录会红 ⇒ 改用 `CARGO_MANIFEST_DIR` **上溯两级**推导；②**wasmparser API 版本差异**（`ImportSection` 迭代器无 `ty`/`module`/`name` 字段、`type_index` 实为 `type_id`）⇒ 去掉对 import 段的依赖；③**只量了「位置」漏了「可达性」**——初版结论「guest 未实现 T1 的 return 位置」**是错的**（源码里本来就有该分支）⇒ 补 **g1 / g2** 两个决定性变体（一测即逐字节等同）⇒ **教训**：「某形状被拒绝」**不等于**「该形状没实现」，**必须用「加一条无关语句」这类最小扰动探测可达性**，否则会把「可达性缺口」误报成「能力缺口」、把下一刀工作量估错一个数量级。
- **出口**：5 条固件钉（**122 → 127**）+ 前沿探针新增 **3** 条（**23 → 26** 条，`FRONTIER_UNCHANGED`）+ 10 组真值工件（`target/tmp/step0517/`）+ core module code 段**反汇编**（计划 §3.3.5 第 1 拍要求的第三项）；固件 **127/127**、本机验证电池 **11/11 `BATTERY_GREEN`**（`fmt-runner` 3s / `ci-one-key` 426s / 0245 130s / 0262 151s / 前端前沿 172s / 计划层 25s / 0124 1s / 0131 1s / 0156 0s / diff-check 0s / 行尾 0s，行尾审计 **24** 个文件）。
- **诚实收窄**：①**组件层未实现**（前端闭合 ≠ 族闭合：族级出口判据**五项缺一不算闭合**）；②**语料覆盖 0 条**；③「体内只有 return」的 return 位置、嵌套 concat、字面量实参的 return 位置**仍** typed 拒绝；④**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；⑤guest 源码与 oracle 生产代码**零改动**；**未提交未推送**；**独立 CI 尚未运行**；**门 3 仍 NO-GO**。


**STEP-0515（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**宿主调用（前端刀 · let 绑定位置）= `let n = sico.bytes.length(data)` ⇒ canonical `intrinsic` IR 与整组件逐字节等同**（[STEP-0515](./steps/STEP-0515-m22-w2-s5-host-call-let-binding.md)）。**本片只闭合宿主调用线的第二个位置；不推进门 3**。
- **编号说明**：**STEP-0514 已被规划片占用**（`docs/steps/STEP-0514-m22-w2-s5-s6-closure-ladder.md`，该片 §7 写明「**STEP-0514 已被本规划片占用 ⇒ 执行线下一片从 STEP-0515 起**」）⇒ 本片按该规则顺取 **STEP-0515**；下面 STEP-0513 条目里那条「下一刀 = STEP-0514」的原文**保留**，并在其后追加了改正说明。
- **真值（读自 oracle 生产输出的 `host-c` 工件）**：Rust 前端把**直线型 `let` 绑定直接传播进 SSA 值** ⇒ let 绑定位置的 canonical IR 与 return 位置**同形**：一条 `intrinsic`（`sico.bytes.length`、`arguments:[0]`、`ty`=`u64`）、结果 id 1、terminator `{"kind":"return","data":1}`、**无 local**（没有 `write_local`、没有 `locals`）；**只有 range 平移**：指令 range **369..392**（return 位置是 368..391，两处都正好是**调用表达式的文本跨度**）、函数 range 319..417（return 位置 319..405）；canonical IR **2215 B**、整组件 **1025 B**（过真实 `wasmparser::Validator`）⇒ **不需要新的 local 机器**。
- **落点**：`selfhost/parser.sico` 新增**专用顶层路由** `direct_host_call_let_ir`（**+230 行**）+ `scalar_ir` 路由梯接线（**+24 行**，放在 `direct_host_call_return_ir` 之后、`fixed_guard_chain_ir` 之前）+ 嵌套宿主调用拆解（**+2 行**）——合计 **+256 行 / −0 行**（与 pre-0514 备份逐行比对）⇒ **纯插入、零既有行重排**；复用既有机器（`parameters_ir` / `binding_parameter_index` / `binding_parameter_kind` / `word_span_offset` / `return_expression_end` / `intrinsic_instruction_json` / `gw_find_code_text`）⇒ **不新造机器**。
- **支持面 = 恰好一个形状**：`function size(data: Bytes) returns U64:` + 函数体 `let n = sico.bytes.length(data)` + `return n`（1 个 `Bytes` 形参、声明返回 `U64`、绑定名任意、`return` 的操作数**就是**那个绑定名）。
- **没有静默放宽**：路由对不认领的形状一律 `SKIP` ⇒ 三实参 `sico.bytes.at`、let 里 `return` 别的名字、let 后还有第二条语句、let 里别的内建（`sico.text.length`）**仍** typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（STEP-0512 的邻接形状钉已按此更新）。
- **出口达成**：guest canonical IR **2215 B == 2215 B**（与 Rust 前端**逐字节相同**，含 range 369..392 与「无 local」断言）；guest 发射的**整个组件 == oracle 生产输出逐字节相同**（**1025 B**）且过真实 `wasmparser::Validator`。
- **自举形状前沿**：新路由连同其 **19 个闭包函数**（共 20 个，按文件顺序）经 guest 自己逐字节降低 == Rust 前端（新钉）⇒ **未推进形状边界**。
- **撞到并修掉的 5 条自造缺陷**：①**guest 词表里没有 `=`**——词法器的 `is_punct_byte` 覆盖 `+ - . ( ) , : [ ]` 而**不含 61** ⇒「`let` 后有 `=`」这道闸门**不能**做词级检查（既有机器 `fixed_guard_chain_ir` 因此也是从绑定名直接跳到接收者）⇒ 改为**字节级**检查（绑定名末尾与表达式起点之间必须存在 `=` 字节）；②`U64.equal(x, I64.literal(1))` 类型不匹配（编译器报 `expected U64, found I64`）⇒ 涉及 `I64` 值的比较一律用 `I64.equal`；③**跳过缩进的字节循环把已找到的偏移覆盖成了 `source_len`**（游标与结果共用一个变量）⇒ 游标与结果**分开**；④**自举形状前沿越界**：单实参内建机器（`gw_intrinsic_call_packed`）要求**单词实参**，而新代码写了 `sico.bytes.length(sico.text.encode(...))`（**宿主调用吃宿主调用**）⇒ guest typed 拒绝 `ERR:E-SH-IR-CALL-SHAPE` ⇒ 按 STEP-0507 实测的形状规则**先落临时变量**；⑤**STEP 号冲突**（见上）。
- **前沿探针**：**新增 1 条用例** `host-call-in-let-binding`（期望 `lowered`）⇒ **22 → 23 条**（不是翻转既有期望）。
- **回归**：固件 **122 passed / 0 failed**（STEP-0513 的 119 + 本片 3 条新钉）。
- **工具侧观察（如实记录）**：`sico run` 走 runner **默认限额**（fuel = 10 M）；602 B 的 Script-profile fixture 在 **let 绑定位置**超限（`resource-limit.fuel`），而 96 B 的同形状通过 ⇒ **CLI 默认燃料对 ~600 B 的 guest 编译输入本就临界**（return 位置也只勉强通过），本片多做的字节扫描只是把它推过线；**权威路径不受影响**（runner 测试用 fuel = 800 G）。**不改任何预算常量**（只降并发、不改断言与预算的既定口径）。
- **诚实收窄**：①只覆盖**一个内建 / 一个实参类型 / 两个位置**（`sico.text.length`、多实参内建、cell 实参、字面量实参、多语句函数体仍拒）；②**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；③oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。
- **下一刀的范围（已用生产输出定界）**：**其余宿主调用形状**——①`sico.text.length` 等**其它单实参内建**（前端 + 组件两层都要扩）；②三实参 `sico.bytes.at`（backend 组件真值 **1053 B** 已在 STEP-0511 测得）；③cell / 字面量实参；再往后是本线更大的缺口：**整文件自举**（guest 不能整体降 `codegen.sico`）、S6 bootstrap 闭环（ADR-0015 `A == B == C` + `.sapp`）、S7 复审 ⇒ **门 3 未动**。

**STEP-0513（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**宿主调用（codegen 刀）= `intrinsic` 算子 ⇒ 整组件 1025 B 与 backend 生产输出逐字节等同**（[STEP-0513](./steps/STEP-0513-m22-w2-s5-host-call-component.md)）。**本片只闭合组件层；不推进门 3**。
- **真值（读自 oracle 生产输出的 core module code 段）**：`size` 函数体 **36 B** 逐段 = locals `03 017e 017f 027f`（3 组）+ dispatcher 外壳 + **`2001 ad 2102`** + 直接返回（`20 02 0f …`）⇒ **`sico.bytes.length` = `local.get <长度槽>` / `i64.extend_i32_u` / `local.set <结果槽>`**；**长度槽 = `Bytes` 形参 (pointer, length) 对的第二个槽**（pointer 是第一个槽 0），**结果槽 = 指令自己的平面槽**（参数对占 2 槽 ⇒ 结果落在槽 2）。
- **动手前的核对（结论：缺的只是操作数字节）**：guest 既有的**槽位派生**（`cgs_value_slot`：参数在前、指令结果按 IR 顺序）、**局部声明**（`cgs_function_local_hex`）、**dispatcher 外壳与直接返回**（STEP-0507 已逐字节等同过）全部自动给出与 backend 相同的槽位与组数 ⇒ 本片**全部**改动 = 「`intrinsic` 这条操作数怎么写成 Wasm 字节」。
- **实施**：`selfhost/codegen.sico` 新增 `cgs_intrinsic_hex`（**+63 行**）+ `cgs_op_hex` 的 `intrinsic` 分支（**+3 行**）——**纯插入、零既有行重排**；**闸门全 fail-closed**：名字必须是 `sico.bytes.length`、实参恰好一个且类型必须是 `bytes`、结果槽数必须是 1 ⇒ 其余一律 `ERR:E-SH-CG-IR-EMIT`（**不静默取近似**）。
- **出口达成**：guest 对该 fixture 发射的**整个组件 = 1025 B**，与 oracle `compile_script_program` 的**生产输出逐字节相同**并过真实 `wasmparser::Validator`（新钉 `sico_codegen_guest_emits_the_host_call_component_byte_exactly`）。
- **自举形状前沿**：新 codegen 函数连同被调桩经 guest 自己逐字节降低 == Rust 前端（新钉）⇒ **未推进形状边界**；新代码里**没有**「宿主调用吃宿主调用」的嵌套（两个被调助手先落临时变量再拼接）。
- **IR 计划层早就认 `intrinsic`**（`cgpo_intrinsic_text`）⇒ 两个前沿探针**零改动**（前端 22/22、计划层 8/8）。
- **回归**：固件 **119 passed / 0 failed**（STEP-0512 的 117 + 本片 2 条新钉）；被改写的 `cgs_op_hex` 仍被 STEP-0509 的前沿钉覆盖且**仍绿**。
- **诚实收窄**：①组件层**只覆盖一个内建 / 一个实参类型**（`sico.text.length`、多实参内建、`string`/`list` 实参、cell 实参 ⇒ 仍 typed 拒绝）；②**IR 层**的支持面仍是 STEP-0512 的那一个形状，本片**没有**扩大它；③**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；④oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。
- **下一刀的范围（已用生产输出定界）**：**STEP-0514 = 宿主调用的 let 绑定位置**（`let n = sico.bytes.length(data)` + `return n`）。关键实测：backend 对这个形状的 canonical IR **与 return 位置同形**（**一条 `intrinsic`、result 1、terminator 返回 1** ⇒ 绑定被直接传播、**不建 local**），只有 **range 平移**（369..392 vs 361..384）⇒ **不需要新的 local 机器**，前端只需在「函数体是 `let` + `return`」时认得同一台机器并取**实参调用**的区间；组件真值 **1025 B** 已在 STEP-0511 测得（`host-c`）。**【STEP-0515 追记】** ①**编号改正**：该「下一刀」的号 **STEP-0514 已被规划片占用**（`docs/steps/STEP-0514-m22-w2-s5-s6-closure-ladder.md`）⇒ 实际执行号是 **STEP-0515**（**原句保留**以便追溯）；②**range 数字订正**：return 位置的指令 range 实为 **368..391**（原记的 361..384 是 `return` **关键字**的偏移，**不是**调用表达式的跨度；真值以 `host-a` / `host-c` 两份 canonical IR 工件为准）⇒ 真实平移是 **369..392 vs 368..391**，两处都正好是**调用表达式的文本跨度**。

**STEP-0512（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**宿主调用（前端刀）= 直线型函数体里的 `sico.bytes.length` ⇒ canonical `intrinsic` IR 逐字节等同**（[STEP-0512](./steps/STEP-0512-m22-w2-s5-host-call-frontend.md)）。**本片只闭合 IR 层；不推进门 3**。
- **落点（实测）**：`selfhost/parser.sico` 新增**专用顶层路由** `direct_host_call_return_ir`（**+121 行**，与 `direct_bytes_equal_encode_ir` / `record_construct_function_ir` 同族）+ `scalar_ir` 路由梯接线（**+24 行**）——**纯插入、零既有行重排**；复用既有机器（`parameters_ir` / `binding_parameter_index` / `binding_parameter_kind` / `return_expression_start` / `return_expression_end` / `intrinsic_instruction_json` / `gw_find_code_text`）⇒ **不新造机器**（STEP-0511 §2.1 的判断成立：缺的是接线不是能力）。
- **支持面 = 恰好一个形状**：`function size(data: Bytes) returns U64:` + 函数体**唯一语句** `return sico.bytes.length(data)`（1 个 `Bytes` 形参、声明返回 `U64`、实参是形参）。
- **出口达成**：guest 把该 fixture 降成的 IR 与 **Rust 前端 canonical IR 逐字节相同**（**2215 B == 2215 B**，含 `"op":"intrinsic"` / `"name":"sico.bytes.length"` / `"arguments":[0]`）；STEP-0511 的 oracle 钉按其**承诺**把 guest 半边从 typed 拒绝**翻成逐字节比较**（组件仍 1025 B + 过真实 `wasmparser::Validator`）。
- **没有静默放宽**：路由对不认领的形状一律 `SKIP` ⇒ **三实参 `sico.bytes.at`** 与 **let 绑定位置**仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（新钉 `sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused`）。
- **自举形状前沿**：新函数的 **21 个闭包函数**经 guest 自己逐字节降低 == Rust 前端（新钉 `sico_compiler_lowers_the_step0512_parser_function_byte_exactly`）⇒ **本片没有推进 guest 前端的形状边界**；**本片改动了 `scalar_ir`（路由梯接线）⇒ 单独复核它的 158 个函数的闭包：实测仍 FLIP ⇒ 前沿没有回退**（口径说明：`target/tmp/census-latest.txt` 的「149/149 全 FLIP」是 STEP-0502/0503 时期的旧快照，不能当本片证据）。
- **撞到并修掉的 2 条自造缺陷**：①**把「函数体第一条语句」取错了位置**——初版用「参数表右括号之后的 token」⇒ 拿到的是签名里的 `returns` 而不是 `return` 语句 ⇒ 路由永远 `SKIP`；**怎么发现的**：临时把 14 道 `SKIP` 闸门换成可区分的 `ERR:E-SH-IR-DHCR-NN` 标记（**测完即摘**）⇒ 落在闸门 5；**修法**：改用**签名行末**（`line_end(words, body_from)`）定位；②**新 guest 代码里的宿主调用嵌套**——初稿把 `sico.u64.to_text(x)` 直接嵌进 `sico.text.concat(...)`（宿主调用吃宿主调用）⇒ 按 STEP-0507 实测的形状规则改为先落 `__nc_` / `__h1` 临时变量。
- **前沿探针**：`host-call-in-straight-line-body` 的期望值**显式翻转** `refused:ERR:E-SH-IR-EXPRESSION` ⇒ `lowered`（前沿**移动**，不是新增用例）。
- **回归**：固件 **117 passed / 0 failed**（STEP-0511 的 114 + 本片 3 条新钉）。
- **诚实收窄**：①**组件层仍拒**——`selfhost/codegen.sico` 的 `cgs_op_hex` **不认 `intrinsic` 算子** ⇒ 该形状在组件层仍 typed 拒绝 ⇒ **整组件 1025 B 的逐字节等同是下一刀（STEP-0513，codegen 刀）**；②只覆盖**一个内建 / 一个位置 / 一种实参**（`sico.text.length`、多实参内建、cell 实参、字面量实参、let 绑定位置全仍拒）；③**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；④oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。

**STEP-0511（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**宿主调用（真值测量片）= 直线型函数体里的 `sico.*` 调用 ⇒ 组件 1025 B 真值钉 + 补丁落点定界**（[STEP-0511](./steps/STEP-0511-m22-w2-s5-host-call-truth.md)）。**本片只测量、不实现；不推进门 3**。
- **真值（全部读自 backend 生产输出）**：backend 对该 fixture **真的发射**（整组件 **1025 B**，过真实 `wasmparser::Validator`），canonical IR 形状 = **一条 `intrinsic` 算子**（`{"name":"sico.bytes.length","arguments":[0]}`、`ty` = `u64`），guest 前端**仍 typed 拒绝** `ERR:E-SH-IR-EXPRESSION`（fail-closed）。
- **三个变体一次量完**：单实参 `sico.bytes.length`（oracle 1025 B）、**三实参** `sico.bytes.at`（oracle **1053 B**）、**let 绑定**位置（oracle 1025 B）⇒ backend 侧**都**发射、guest 侧**都**拒 ⇒ 缺口是**整类**的，不是个例。
- **补丁落点（下一刀不必再摸索）**：缺口**不是**「没有机器」——guest 已有 `intrinsic_instruction_json` 与**单实参**版 `gw_intrinsic_call_packed`；缺的是 **`scalar_ir` 的 return 表达式链里没有宿主调用分支**：`sico.bytes.length(data)` 掉进 `I64.literal`/`U64.literal` 的固定字面量档 ⇒ 无人认领 ⇒ `ERR:E-SH-IR-EXPRESSION`；codegen 侧另需 **`cgs_op_hex` 认 `intrinsic` 算子**。⇒ **下一刀 = 前端刀 + codegen 刀两片**。
- **出口**：真值钉 `oracle_host_call_in_a_straight_line_body_truth_is_pinned`（组件长度 + 真实校验器 + canonical IR 形状 + guest 的 typed 拒绝）+ 前沿探针新增 1 条（21 → **22** 条）。
- **回归**：固件 **114 passed / 0 failed**（STEP-0510 的 113 + 本片 1 个真值钉）。
- **诚实收窄**：**guest 源码零改动**、只覆盖一个 fixture / 一个**单实参**内建（三实参与 let 绑定位置的 backend 真值已测得、但不在本片范围）、**整文件自举仍未关闭**、oracle 生产代码零改动、**独立 CI 尚未运行**（本片未提交未推送）、**门 3 仍 NO-GO**。

**STEP-0510（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**调用位置的调用（收口片）= 用户调用在 return 表达式位置 ⇒ IR 与整组件逐字节等同；并纠正一条与证据冲突的记录**（[STEP-0510](./steps/STEP-0510-m22-w2-s5-call-in-return.md)）。**本片不推进门 3**。
- **出口达成**：guest 把「`helper2` 的 return 表达式是 `helper(...)`、且 `run` 在 construct 字段位调用 `helper2`」这个 fixture 降成的 IR 与 **Rust 前端 canonical IR 逐字节相同**（**2742 B == 2742 B**），且发射的**整个组件 = 1075 B**，与 oracle `compile_script_program` 的**生产输出逐字节相同**并过真实 `wasmparser::Validator`（新钉 `sico_compiler_lowers_the_call_in_a_return_position_byte_exactly` + `sico_codegen_guest_emits_the_call_in_a_return_component_byte_exactly`）。对照变体（`helper2` 不被 `run` 调用）同样逐字节相同（IR 2613 B / 组件 1067 B）⇒ 结论**不依赖**调用嵌套。
- **撞到的核心事实（文档纠偏）**：STEP-0507 §7.2 起头的一条「`return <user call>` ⇒ `ERR:E-SH-IR-CALL-ARGUMENT`」被 STEP-0508 / STEP-0509 一路抄进 5 处同步文档，说「表达式位置（return）的调用前端仍不降 ⇒ typed 拒绝」——**实测不成立**：18 个形状（12 位置 + 6 函数体变体）全部 **lowered**；且 **STEP-0507 自己的固件钉里 `cgs_alloc_index` 就是 `return cgs_function_count(bytes)`**，**至今仍绿** ⇒ 这个形状**从未被拒绝过**。已按证据改正 STEP-0507 §7.2、STEP-0509 §5.1、handoff 的 STEP-0507/STEP-0509 条目、ROADMAP / STATUS / plan / steps-README 相应记录（保留原句以便追溯）。
- **实测前沿（12 个形状）**：return 位置（实参是字面量 / 形参 / 零参 / 双参 / 实参是调用）、let 绑定位置、construct 字段位（双参）、被调者返回 `Text`（双槽）⇒ **全部 lowered**；**语句位置的裸调用**：Rust 前端自己拒（`non-Unit expression statement`）而 guest 能降 ⇒ **分歧**，参考实现拒绝的形状不进支持面；**宿主调用（`sico.*`）** 在 return / let / 实参位置 ⇒ guest **typed 拒绝**（`ERR:E-SH-IR-EXPRESSION` / `ERR:E-SH-IR-CALL-ARGUMENT`）。
- **下一刀的范围（实测钉准）**：宿主调用不是「一律不支持」，而是**按函数体路径**分叉——直线型函数体（`let n = sico.text.length(name)` + `return n`）**拒**，函数体里有 `while` / `if` 就**能降** ⇒ 下一刀 = **直线型函数体路径的宿主调用支持**（前端）+ 随之而来的 `Operation::Intrinsic` 组件发射（codegen）；backend 侧已具备内建发射能力（`emit_intrinsic` 覆盖 `sico.bytes.length` 等）⇒ 真值侧不缺。
- **前沿探针登记**：`tools/probe-m22-frontend-frontier.ps1` 新增 **3 条**用例（`user-call-in-return-position` = `lowered`、`user-call-in-let-binding` = `lowered`、`host-call-in-return-position` = `refused:ERR:E-SH-IR-EXPRESSION`）⇒ 前沿 18 → **21** 条，`FRONTIER_UNCHANGED`。
- **回归**：固件 **113 passed / 0 failed**（STEP-0509 的 111 + 本片 2 个新钉）。
- **自造缺陷 #1**：追加临时探针时用**过期**的文件长度当截断基准，把 `runner/sico-runner/tests/selfhost_compiler.rs` 的**尾部**一起截掉 ⇒ 修法 = `git show HEAD:<file>` 取基线 + `difflib.SequenceMatcher` 的 opcodes 把「未提交增量」与「截断」分开（最后一个 `replace` 就是截断点 ⇒ 用 HEAD 的完整尾部替换）⇒ 重建 243348 B / 5381 行，两处 `argument_values_at` 落点正确，**固件全量 113 passed** 验证重建正确。**验证基础设施自身的缺陷必须写进记录**（同族于 STEP-0497 / STEP-0504）。
- **自造缺陷 #2**：新钉里一行写不完的 `.expect(...)` 不合 rustfmt ⇒ **电池第 1 步 `fmt-runner` 就红了**（exit=1）⇒ 一条 `cargo fmt` 修好 ⇒ 重跑整轮电池。**正面证据**：电池把「新增代码没过格式化」挡在第一步，没让它混进后面 10 步的结论。
- **自造缺陷 #3（留给下一片的操作纪律）**：追加最后一个临时探针后**忘了摘**，电池就在**带着探针**的树上跑完 ⇒ `validate-0245` 报 **114 passed** 而清理后是 **113** ⇒ **裁剪基准要用「内容标记」（探针标记）而不是「长度」**（缺陷 #1 的同一个错法），且**电池结果必须与清理后的树一一对应**：固件计数与单独复跑不一致时先怀疑「树脏了」。
- **诚实收窄**：①本片**没有改动任何 guest 源码**（`selfhost/*.sico` 零改动）——能力早已存在，本片做的是**钉住 + 纠偏**；②只覆盖**一个 fixture**（§1.3 表格是形状级实测，不是逐个字节钉）；③语句位置的裸调用存在**分歧**（guest 比参考实现宽松）⇒ 不作支持声明；④**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；⑤oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。
**STEP-0509（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**调用位置的调用（codegen 刀）= 整组件 1025 B 与 backend 生产输出逐字节等同**（[STEP-0509](./steps/STEP-0509-m22-w2-s5-call-component.md)）。**本片不推进门 3**。
- **出口达成**：guest 对「construct 字段值位置上的调用」fixture 发射的**整个组件 = 1025 B**，与 oracle `compile_script_program` 的**生产输出逐字节相同**并过真实 `wasmparser::Validator`（新钉 `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly`）。
- **真值（逐段解剖，全部读自 backend 生产输出）**：差异**只**落在 `run` 的 body 上——局部声明 **21 组 → 22 组**（多出的正是两条新指令结果各一组 `01 7e`）、code 多 **6 字节** = `20 08`（推实参槽）+ `10 00`（调用）+ `21 09`（存结果槽）；type / function / export / global 段、data 段与其余四个函数体**逐字节相同**。
- **逐指令规则**：实参指令**先**发射（`42 07 21 08`）⇒ 调用时推**实参值的槽**（`20 08`）⇒ `call <Wasm 索引>` ⇒ 结果槽 `21 09`；**目标索引** = IR 的 `call.data.function`（**声明 id**、1-based）在**模块函数表**里的 **0-based 位置**（Script profile 的 import 数为 0）；结果槽**从高到低**落位（对齐 backend 的 `result.iter().rev()`）。
- **动手前的核对（本片的关键判断）**：guest 既有的**槽位派生**（`cgs_value_slot`：参数在前、指令结果按 IR 顺序）**自动**给出实参槽 8 与结果槽 9；**局部声明派生**（`cgs_function_local_hex`）**自动** 21 → 22 组；**构造成员查槽**（`cgs_construct_hex`）**自动**让 `exit_code` 指向结果槽 9 ⇒ 三者都与 backend 实测**一致**、**都不需要改** ⇒ 本片**全部**改动只是「`call` 这条操作数怎么写成 Wasm 字节」。
- **实施**：`selfhost/codegen.sico` 新增 `cgs_function_position`（IR 声明 id ⇒ 模块函数表里的 1-based 位置）与 `cgs_call_hex`（实参槽升序 `local.get` → `call` → 结果槽从高到低 `local.set`），`cgs_op_hex` 增加 `call` 分支。
- **自举形状前沿**：本片新增/改写的 3 个 codegen 函数（`cgs_function_position` / `cgs_call_hex` / `cgs_op_hex`）**一次通过**新钉 `sico_compiler_lowers_the_step0509_codegen_functions_byte_exactly`（逐个与 Rust 前端 canonical IR 逐字节等同）——形状落在 STEP-0507 已登记的边界内。
- **回归**：固件 **111 passed / 0 failed**（STEP-0508 的 109 + 本片 2 个新钉）。
- **本机验证电池 11 步**：**首轮 11/11 全绿**（`fmt-runner` 00:02 / one-key CI **13 步 CI GREEN** 05:08 / 0245 00:52 / 0262 02:17 / 前端前沿探针 **18/18** 02:14 / IR 计划层前沿探针 **8/8** 00:28 / 0124 00:00 / 0131 00:01 / 0156 00:00 / `git diff --check` 00:00 / 行尾审计 00:00 ⇒ `LINE_ENDING_AUDIT_GREEN 15 files`）——与 STEP-0508 的 **10/11** 不同，本轮**没有**复现宿主 Windows 提交限额那个环境敏感项（`os error 1455`），因此**不需要**降并发复跑取证。
- **诚实收窄**：①组件层**只**闭合**一个 fixture**（调用在 construct 字段值位置、实参 `I64.literal(...)`、被调者返回**标量**）；②**表达式位置**（return）与**语句位置**的调用**前端**仍不降 ⇒ typed 拒绝 → **【STEP-0510 实测纠正（2026-10-22）】后半句是错的**：用户调用直接做 return 表达式**早已**能逐字节降低（实测 12 个形状 + 6 个函数体变体全部 lowered；`selfhost/codegen.sico` 的 `cgs_alloc_index` 就是这个形状，STEP-0507 固件钉至今仍绿）；**真正未闭合**的是**宿主调用（`sico.*`）在直线型函数体里的表达式位置**（`ERR:E-SH-IR-EXPRESSION` / `ERR:E-SH-IR-CALL-ARGUMENT`）⇒ **下一刀**（见 [STEP-0510](./steps/STEP-0510-m22-w2-s5-call-in-return.md)）；③被调者返回 named/result/option ⇒ packed checked-result 路径未实现 ⇒ typed 拒绝；④目标索引的派生假定 **Script profile 的 import 数为 0**；⑤**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；⑥oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。
**STEP-0508（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**调用位置的调用（前端刀）= construct 字段值位置上的调用与 Rust 前端逐字节等同**（[STEP-0508](./steps/STEP-0508-m22-w2-s5-call-in-expression.md)）。**本片不推进门 3**。
- **出口达成**：guest 把 `ScriptOutput(exit_code: helper(I64.literal(7)))` 降成的 IR 与 **Rust 前端**的 canonical IR **逐字节相同**（新固件钉 `sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly`；2198 B == 2198 B）。
- **真值（全部读自 backend 生产输出）**：`call.data.function` = 被调者的**声明 id（1-based）**（实测 `1` = `helper`，**不是** 0-based 序号）、`call.data.arguments` = 实参 value id（实测 `[3]`）、`call.ty` = 被调者声明返回类型（`i64`）、`call.range` = 被调者名字起点→右括号之后（600..622）；**id 记账** = 实参指令先发射（各占 1 个 id）⇒ 调用占下一个 id ⇒ 字段引用调用的 id ⇒ construct ⇒ variant。**组件级真值 1025 B 已测得**（同形无调用 fixture 1017 B，差 8 B）⇒ **留给下一刀**。
- **前端刀**：`selfhost/parser.sico` 新增 `construct_field_call_count`（字段值是不是调用形状、消耗几个 id）+ `construct_field_call_ir`（实参指令 + 调用指令的 JSON）；`construct_lowering` 的字段值路径分成「调用 / 既有形状」两支，`cl_next_result` 按**实际消耗**推进；`construct_lowering` 回传**真实 construct id**（`<指令>\n<id>`）供 `rcf_*` / `rvf_*` 读取；**实参 base** 拆成 `argument_values_at(..., base)`（字段位用**字段当前 id**、return 表达式仍用「参数个数」⇒ 旧签名保留为包装、行为不变）。
- **过程中撞到并修掉的 3 处真实偏差**：**(a)** `function_declaration_ordinal` 本就返回 **1-based 声明 id**，我又加一 ⇒ guest 发 `2`、Rust 发 `1`；**(b)** 实参 base 写死成「参数数」⇒ 字段位置必须用**字段当前的结果 id**（guest `[1]` vs Rust `[3]`）；**(c)** construct 的 id 被调用方用「参数数+字段数」另算 ⇒ 调用字段多消耗 id 时**整体错位**（guest variant 落 5、Rust 6）。
- **自造缺陷 #1（本片撞到并修回，已记录在案）**：新加的 3 行注释写在 `construct_lowering` **函数体内部** ⇒ **整份 `parser.sico` 自宿退化**（`ERR:E-SH-IR-STATEMENT`）。**根因**：guest 的词法器 `lex_words` **不跳过 `//` 注释**——注释里的**每个词**都进词表 ⇒ 函数体**内部**的注释词被**语句扫描器**当成源码语句（落到「无法识别的语句」兜底），函数**之间**的注释词被**结构扫描**（统计 `let`/`set`/`match`/`if`/`while`）当成源码词 ⇒ 改变模块路由对该函数的分类。**定位链（可复现）**：①按「**传递被调闭包**」逐个喂 guest（guest 从**输入词表**查被调者的声明返回类型，所以闭包必须自带被调者）⇒ 收敛到 `construct_lowering`；②给 guest 的**每个** `ERR:E-SH-IR-STATEMENT` 站点打**源码行号**标记（临时诊断，已还原）⇒ 命中**语句分派兜底**（不是任何形状检查）⇒ 说明遇到了**不认识的语句词**；③把兜底改成回报该词 + 所属函数 ⇒ 回报 **`construct_lowering` / `The`** ⇒ 罪魁是**注释里的第一个词**。**证据**：整份 2.1 万行 `parser.sico` 里**函数体内部的注释只有我新加的这 3 行**。**修法**：注释**移到函数体之外** + `argument_values_at` 注释里的 `return` 一词换成 `expression`（`return` 是结构扫描词）⇒ 逐个闭包**全部 exit=0**、整份自宿恢复。
- **本线新增硬约束**：`selfhost/parser.sico` 的注释**只能写在函数之间**，且不得出现 17 个结构扫描词（`let`/`set`/`match`/`if`/`while`/`for`/`return`/`end`/`function`/`record`/`enum`/`case`/`break`/`continue`/`use`/`module`）。校验器：**受版本控制的** `tools/validate-selfhost-comment-discipline.py`（只读，报告「函数体内注释数」+「注释里的违禁词」），并**已接入一键 CI**（`tools/run-ci.ps1` 的 `selfhost comment discipline` 步 ⇒ 一键 CI 由 12 步变 13 步）。
- **自举形状前沿**：本片**没有**推进 guest 前端的形状边界——8 个新增/改写函数（`construct_field_call_count` 15 依赖 / `construct_field_call_ir` 28 / `argument_values_at` 19 / `argument_values` 20 / `argument_instructions` 24 / `construct_lowering` 42 / `record_construct_function_ir` 51 / `record_variant_function_ir` 53）**全部落在既有子集内**，逐个闭包 exit=0。
- **回归**：两个既有闭包钉（`argument_instructions` / `checked_arithmetic_match_ir`）的函数清单补入 `argument_values_at` 后转绿；固件 **109 passed / 0 failed**（STEP-0507 的 108 + 本片新钉 1）。
- **本机验证电池 11 步**：首轮 **10/11 绿**，唯一红步**是环境敏感项**（**不是**代码缺陷）：`validate-0245` 的 3 个 `selfhost_checker` 用例报 `Launch("页面文件太小，无法完成操作。 (os error 1455)")` —— 宿主 Windows 提交限额耗尽（一个 python 常驻 **5.45 GB**、msedge 1.3 GB；每个 guest 预留 2 GiB 线性内存），**不是**诊断分区不匹配（期望值仍是 `E3002`/`E2001`/`LEXICAL`）⇒ **只降并发**（`RUST_TEST_THREADS=1`，**不改断言、不改预算常量**）复跑 ⇒ **`validate-step-0245.ps1` exit=0**（内含固件 **109 passed** + clippy）；CI 其余步全 PASS。
- **诚实收窄**：①本片**只闭合 IR 层**——组件发射**仍 typed 拒绝**（guest 的 codegen 没有「`call` 指令」这一种操作数 ⇒ `cgs_op_hex` 不认 `call`）⇒ **整组件 1025 B 的逐字节等同是下一刀**；②只覆盖**一个 fixture**（实参是 `I64.literal(...)`）；③被调者的返回类型必须**标量**（named/result/option ⇒ typed 拒绝）；④未声明的被调者 ⇒ typed 拒绝（不静默当作别的形状）；⑤**表达式位置**（return）与**语句位置**的调用路径返回类型仍**写死** `{"kind":"u64"}`、**未被本片改动**也未被钉；⑥**整文件自举仍未关闭**（`codegen.sico` 的 `List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）；⑦oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。
- **工件**：`target/tmp/step0508/`（未跟踪）——`closure_of.py`（传递被调闭包计算）/ `comment_scan.py`（注释纪律只读校验，**已提升为受版本控制的 `tools/validate-selfhost-comment-discipline.py`**）/ `variants.py`（形状二分）/ `tag_errors.py` / `tag_word.py`（临时诊断，已还原）/ `cl_*.sico`（逐函数闭包）。**临时探针已从测试文件移除**（`probe_step0508_closure_lowering`）。
**STEP-0507（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**多函数模块的组件级派生 = 组件四段全部派生 ⇒ 整组件 1017 B 逐字节等同**（[STEP-0507](./steps/STEP-0507-m22-w2-s5-multi-function-component.md)）。**本片不推进门 3**。
- **出口达成**：guest 对双函数 fixture（`helper(value: I64) returns I64: return value` + `run(...)`）发射的**整个组件 = 1017 B**（**单槽 + 多槽两个 fixture**），与 oracle `compile_script_program` 的**生产输出逐字节相同**（新固件钉 `sico_codegen_guest_emits_the_multi_function_component_byte_exactly`，含 `wasmparser::Validator` 校验）。
- **派生 vs 冻结**：**types / functions / exports / code / 静态 data 段 / 分配器核心索引**改为**从 IR 派生**；**组件骨架**（WIT 记录、导入/别名/导出段、三个 helper 体、内存与 globals 上限）继续冻结。**删掉的冻结常量**：`cg_core_types_hex` / `cg_core_funcs_hex` / `cg_core_exports_hex`。
- **本片新增的派生点 = 分配器核心索引**：backend `alloc_index = import_count + module.functions.len()`（Script profile 无 import ⇒ **= 模块函数数**）⇒ guest 的冻结常量 1 改为 `cgs_alloc_index(bytes)`（单函数 1、双函数 2），并线程化到 `cgs_result_list_hex` / `cgs_result_field_hex` / `cgs_result_error_field_hex` / `cgs_result_return_hex`（ok/error 两臂）。
- **实测的局部布局规则（任意 Script 函数）**：参数不写进声明；每条指令结果的每个 slot **单独一组**；dispatcher `01 7f`；Script scratch 对 `02 7f`。
- **非 `run` 函数的返回**：**逐 slot `local.get` + `return`**（与 `run` 的结果区返回是**两条路径**，别混）。
- **自举形状前沿（新证据，下一刀直接用）**：用「同签名桩」把 `codegen.sico` 的单函数隔离出来交给 guest 前端降低，实测出 **5 类被拒形状**：`return <user call>`（`ERR:E-SH-IR-CALL-ARGUMENT`）、return 位置的 `concat(<literal>, <user call>)`（`ERR:E-SH-IR-EXPRESSION`）、**call 的实参是另一个 call**（`ERR:E-SH-IR-CALL-ARITY`）、`match <host call>`（`ERR:E-SH-IR-CALL-SHAPE`）、host 套 host（`ERR:E-SH-IR-GWPACK-OTHER`）⇒ **一律先过 cell**（`let` 取值 → `set` 拼接 → `return` cell）。本片据此把 **4 个新函数 + 4 个既有函数**改写（新钉 `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`，21/21 逐字节比对，**发射字节不变**）。
- **固件**：**108 passed / 0 failed**（STEP-0504/0506 的三个组件钉仍绿）。
- **本机验证电池 11 步**：首轮 **9/11 绿**，2 步红**都是环境敏感项**（**不是**代码缺陷，别去追）：①`ci-one-key` 的 `prepared_program_reruns_are_isolated_and_survive_a_trap` **20 ms warm-median 预算断言**（并发下实测 20.626 ms，**单跑 18.625 ms 通过**；该断言自己就是 M19 计划 §3.1 登记的 env-sensitive 断言，本片不碰 runner 热路径）；②`validate-0245` 的 `Launch(... os error 1455)`（宿主 Windows 提交限额耗尽：一个 python 常驻 5.7 GB、空闲物理内存仅 1.7 GB）⇒ **`RUST_TEST_THREADS=1`（只降并发）下 `validate-step-0245.ps1` exit=0**（内含固件 108 passed + clippy），CI 其余 **11 步全 PASS**。
- **诚实收窄**：①只覆盖**一个 fixture**（helper 未被调用）；②「**表达式位置的调用**」仍未实现（前端 `ERR:E-SH-IR-CONSTRUCT-VALUE`）⇒ 下一刀 → **【STEP-0510 实测纠正（2026-10-22）】**：这里指的是 **construct 字段值位置**（STEP-0508 已闭合）；**用户调用直接做 return 表达式**这个位置**从来就不是拒绝**（本片自己的 `cgs_alloc_index` 钉至今仍绿）⇒ 别再把两者混为一谈；③非 `run` 函数返回 `Result`/`Option` ⇒ typed 拒绝（packed checked-result 返回未实现）；④多 block 的通用函数未支持；⑤**整文件自举仍未关闭**（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）⇒ 前沿按**函数**推进；⑥oracle 生产代码零改动；**独立 CI 尚未运行**（本片未提交未推送）；**门 3 仍 NO-GO**。
- **工件**：`target/tmp/step0507/`（未跟踪）——`closure.py` / `stub_prefix.py` / `probe/*.sico`（前沿测量用）。**临时探针已从测试文件移除**（`--emit-parts` 诊断模式与 `probe_step0507_parts`）。
**STEP-0506（2026-10-22，implementation complete locally；本机全绿，未提交未推送）**：**`error` 臂组件刀 = 前端枚举 case + codegen `error` 臂 ⇒ 整组件逐字节等同**（[STEP-0506](./steps/STEP-0506-m22-w2-s5-error-arm-component.md)）。**本片不推进门 3**。
- **出口达成**：guest 对 `error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "boom"))` 的 Script-profile `run` 发射的**整个组件 = 939 B**，与 oracle `compile_script_program` 的**生产输出逐字节相同**（新固件钉 `sico_codegen_guest_emits_the_error_arm_component_byte_exactly`，含 `wasmparser::Validator` 校验）。
- **前端刀**：`selfhost/parser.sico` 新增 `record_field_type_word` / `enum_case_declared` / `enum_case_end` / `enum_case_instruction_json` / `construct_enum_case_ir` —— 枚举 case 操作数降级为**空载荷 `variant` op**，guest IR 与 Rust IR 逐字节相同（新固件钉 `sico_compiler_lowers_the_enum_case_operand_as_a_variant_byte_exactly`）。
- **codegen 刀**：`selfhost/codegen.sico` 新增 `cgs_enum_variant_hex` / `cgs_enum_case_tag` / `cgs_result_arm_slots` / `cgs_abi_error_field_*` / `cgs_result_scalar8_hex` / `cgs_result_error_field(s)_hex` / `cgs_literal_text` / `cgs_data_offset` / `cgs_const_string_hex` / `cgs_list_holds` / `cgs_data_hex` / `cgs_text_bytes_hex` / `cgs_arena_base` / `cgs_data_section_hex`；`cg_core_globals_hex` / `cg_post_body_hex` 的 arena base 从**冻结常量改为派生**。
- **组件级事实（实测）**：核心模块段号 {1,3,5,6,7,10} → **{1,3,5,6,7,10,11}**（新增 **data 段 12 B** = `0b 0a 01 00 41 00 0b 04 62 6f 6f 6d`，字面量在内存 0）⇒ **arena base = `align_up(4,8)` = 8**（出现在 globals[0] 初值与 `cabi_post_run` 的第一条常量）。⇒ **「参考形态无 data 段」只对 ok 臂成立**。
- **根因三条（实测）**：**(a)** guest 源码把 error 臂的**载荷基槽**算在 **ok 臂的 cell** `crrh_tag` 上（该 cell 在 error 路径从未赋值 ⇒ 0）⇒ base = 6 而不是 16 ⇒ 三个载荷槽整体错位（**标记实验证明运行时执行当前源码，不是组件缓存陈旧**）；**(b)** 缺 data 段派生；**(c)** arena base 是冻结常量。
- **顺带修掉**：测试文件里 **53 处字符曾被按 CP936 误解码**（em-dash / 箭头变成 `U+95B3` + `?`，并吞掉行尾空格或换行）⇒ 已按 `HEAD` 逐行对齐复原（**47 个 em-dash + 6 个箭头**，与 `HEAD` 的非 ASCII 清单完全一致）；三个改动文件复核**纯 LF、零 CRLF、零 BOM**。
- **回归**：固件 **selfhost_compiler 106 passed / 0 failed**（删掉 8 个临时探针后）+ **本机验证电池 11/11 BATTERY_GREEN**（`fmt (runner)` / one-key CI **12 步 CI GREEN** / 0245 / 0262 / 前端前沿探针 / IR 计划层前沿探针 / 0124 / 0131 / 0156 / `git diff --check` / 行尾审计）；**第一次电池曾红过一次（如实记录）**：CI 与 0245 的 `selfhost_checker` 报 `Launch("页面文件太小，无法完成操作。 (os error 1455)")` —— 这是**宿主 Windows 提交限额耗尽**（每个 guest 预留 2 GiB 线性内存，当时空闲虚拟内存仅 ~7.8 GB，用户的 ComfyUI 进程常驻 ~3.5 GB），**不是诊断分区不匹配**；同测试串行重跑 **9/9 绿**，空闲内存恢复后整轮 **11/11 绿**。本片**零改动** `selfhost/checker.sico` ⇒ 记为**环境敏感项**（与 STEP-0484 登记的 warm median 红点同类），不写成"全绿即无风险"+ oracle 生产代码**零改动**。
- **诚实收窄**：①**只覆盖一个 fixture**；②组件里 IR 相关字节 = run 体 **169 B**（局部声明 **35 B** + code **134 B**）+ data 段 **12 B** = **181 B / 939 ≈ 19.3%**，其余仍是冻结骨架 ⇒ guest 仍不是通用组件发射器；③**重复字面量**与**多函数模块**仍 typed 拒绝；④**独立 CI 尚未运行**（本片未提交未推送）；⑤**门 3 仍 NO-GO**（37 语料 / formatter / 全源 canary / ADR-0015 `A == B == C` + `.sapp` 全部未做）。
- **下一刀**：**多函数 / data 段模块**（guest 的**函数段**目前是冻结的四个；backend 的 `script_data` 按「模块内函数顺序 → 块顺序 → 指令顺序」收集字面量，多函数下的 data 段顺序要按这条规则派生）⇒ 之后才是 37 语料 → formatter → 全源 canary → ADR-0015 的阶梯。
**STEP-0505（2026-10-21，implementation complete locally；本机全绿，未提交未推送）**：**`error` 臂真值测量片 = 把 `ScriptError` 结果区偏移从「冻结推导」升级为「实测」**（[STEP-0505](./steps/STEP-0505-m22-w2-s5-error-arm-truth.md)）。**本片只测量、不实现**（与 STEP-0501 同规）。
- **真值存在**：Rust backend 对返回 `error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "boom"))` 的 Script-profile `run` **真的发射组件**（**939 B**，通过 `wasmparser::Validator`）⇒ 不再是「没有实测真值所以拒绝」。
- **实测的 `ScriptError` 结果区偏移**：`code` → **结果区 8**（`i32.store8`，**1 字节**）、`message` → **结果区 12（指针）/ 16（长度）**（`i32.store` align=2）。
- **实测的段与布局**：`error` 臂**结果区返回段 = 59 B（118 hex）**（ok 臂 89 B），尾部仍是 **6 字节 dispatcher 尾声**；`error` 臂**局部声明段 = 17 组 / 18 槽**（17 i32 + 1 i64），ok 臂 21 组 / 22 槽（19 i32 + 3 i64）⇒ **槽位计划随载荷变**。
- **结构规则差异**：`error` 臂 = 1 个 i32 标量（`code`）+ 1 个 list（`message`），**没有** i64 字段（段里不含 `37 03`），同样**没有**运行时分派（`has_ok && has_error` 为假 ⇒ 静态选 `error` 记录）⇒ **新规则：结果区里的标量 i32 字段用 `i32.store8` 存 1 字节**（**实测**，非推导）。
- **guest 当前拒绝点已精确定位：不在 codegen，而在 IR 构造阶段**——`--emit-locals-hex` / `--emit-run-body-hex` 返回 `ERR:E-SH-IR-CONSTRUCT-VALUE`，`--emit-component-hex` 返回 `unsupported codegen source shape`。
- **根因**：Rust 前端把**枚举 case** 降级为 **空载荷的 `variant` op**（`{"op":"variant","data":{"name":"ScriptErrorCode.InvalidInput","payload":[]}}`），而 guest 的 `construct` 字段值只支持 `const_string` / 字段投影 / `I64`·`U64` 字面量 ⇒ **缺「枚举 case 操作数」这一种形状**（guest 的点分分支**会**被走进——`record_named_type` 只校验标识符形态——但 `record_field_kind` **只扫 `record` 声明** ⇒ 对 enum 返回**空串** ⇒ 分支守卫不成立）。
- **新固件钉** ``oracle_error_arm_result_area_truth_is_pinned``（局部声明 + 整段 run 体 + 结果区段 + 三个 store + 无 i64 store + 6 字节尾声 + 59 B 长度，**全部读自 backend 生产输出**）；固件 **9 + 104 + 1 + 2 = 116/116**；**本机验证电池 11/11 BATTERY_GREEN**（mt (runner) 00:01 / one-key CI **12 步 CI GREEN** 05:10 / 0245 00:58（内含固件 **104 passed**）/ 0262 02:13 / 前端前沿探针 02:11 / IR 计划层前沿探针 00:25 / 0124 00:00 / 0131 00:01 / 0156 00:00 / git diff --check 00:00 / 行尾审计 00:01 ⇒ LINE_ENDING_AUDIT_GREEN）。
- **诚实收窄**：**本片只测量、不实现**——guest 仍 typed 拒绝、**不支持** `error` 臂；真值只覆盖这一个 fixture；槽位索引（10/16/17/18/20/21）是**实测字节事实、不是分配规则**（下一片必须**重算**槽位计划，不能把数字写死进 guest）；`i32.store8` 存 1 字节是实测但未推广到其它标量；oracle 生产代码零改动。**门 3 仍 NO-GO**。
- **下一刀的切点已经找准（两刀）**：**前端刀** = `construct_lowering` 增加「枚举 case 操作数」形状（基名是**已声明 enum**，不是 record）；**codegen 刀** = `cgs_result_return_hex` 增加 `error` 臂分支（按 §2.4 的规则）+ 槽位计划产出 `error` 臂的局部声明段。出口 = guest 对该 fixture 发射的**整个组件**与 backend 生产输出**逐字节相同**。
**STEP-0501（2026-10-21，complete，独立 CI GO）**：**run 体发射的"真值测量刀" = 逐字节实测 + 规则表 + 下一片可对拍切点**（[STEP-0501](./steps/STEP-0501-m22-w2-s5-run-body-truth.md)）。**本片不实现发射，不推进门 3**。
- **只测量与登记**：guest 侧**仍然不会**发射 run 体字节；STEP-0487 的 `--emit-component-hex` 仍是冻结的最小 echo 骨架（run 体是硬编码 hex）。
- **取值方式 = 读 oracle 的生产输出**（`compile_script_program` 真正产出的组件 → wasmparser → 导出 `run` 的 core module 函数体 → 跳过局部声明后的 code 字节），**不复刻发射器**；oracle 生产代码**零改动**（读取器写在测试文件内，与 STEP-0496/0499 同规）。
- **实测**：参考形态 `run` 体 code = **360 hex = 180 B**；体内部有**唯一命中**的分界标记 **`41201001`**（`i32.const 32` = 结果区大小 + `call 1` = 分配器）⇒ **指令段 182 hex（91 B）** / **终止符+尾声 178 hex（89 B）**。
- **规则表**（STEP-0501 §4）：dispatcher 外壳 / 块分派 / `project`（base 槽 + 字段槽偏移）/ `const_i64` / `construct`（**按 op 的 fields 顺序**连续拷贝）/ `variant`（tag + 载荷拷贝 + **清零未激活臂**；`error` 臂**右对齐**）/ Script 结果区返回 / dispatcher 尾声。
- **槽位真值与 STEP-0499 互证**：形参 0–3、结果 4–22、dispatcher 23、scratch 24/25，i64 槽 8/13/19 —— 与 STEP-0499 实测的局部声明段 `017f…027f` **逐字节相同**。
- **对门 3 关键的事实**：参考形态的核心模块**无 data 段**（段号 {1,3,5,6,7,10}）⇒ **run 体是组件唯一的 IR 相关部分** ⇒ run 体一旦 IR 派生且逐字节等同，**整个组件**就能逐字节等同。
- **新固件钉** `oracle_run_body_emission_table_is_pinned`（整段真值 + 标记唯一性 + 两段切分一起钉在生产输出上）+ 下一片直接可用的 `oracle_run_instruction_segment_hex`。
- **分解已切好**：STEP-0502 = guest 发射**指令段**（**无组件级依赖**）→ STEP-0503 = guest 发射 **Script 结果区返回**（需五项冻结的组件级事实：分配器索引 1、结果区 32 B、载荷起始偏移 8、`ScriptOutput` 字段 @0/8/16、`ScriptError` @0/4）→ STEP-0504 = 用派生 run 体替换 STEP-0487 的冻结 run 体 hex。
- **诚实收窄**：真值只覆盖 **Script profile 的参考形态**（`compile_script_program` 只接受 Script v0 入口签名，其它形态 oracle 根本不发射组件、无法取真值）；"error 臂右对齐"、"清零未激活臂"、"construct 按 op 的 fields 顺序"三条**只有源码依据**（参考形态不触发），下一片实现前应各自再取真值。
- **本机验证全绿**：真值钉 **ok** + 固件 **9 + 97 + 1 + 2 = 109/109** + 0245 exit 0 + 0262 `STEP_0262_OK` + 前端前沿探针 **18/18** + 计划层前沿探针 **8/8** + **一键 CI 12/12 全 PASS**（exit 0）+ 改动文件纯 LF、零 CRLF、零 BOM。首次跑一键 CI 时 `test (runner)` 曾红在**已登记的环境敏感**预算断言（`prepared_program_reruns_are_isolated_and_survive_a_trap`）——本片新钉是**纯 Rust**（不跑 guest、不产生 2 GiB 预留）、失败二进制源码与 `HEAD` 逐位相同，**机器空闲时重跑即 12/12 全绿**，**预算常量与断言均未改动**。**独立 CI 裁决已出（GO）**：本片已**提交并推送**（owner 授权）= 提交 `a730ac6` → `github/codex/m22-w1-ci`（fast-forward `ee2317f..a730ac6`）；独立 CI run **39549599204**（run #23，head_sha `a730ac6`）= `conclusion: success`，05:26:51Z → 05:55:15Z，内层 **12/12** 步全 PASS ⇒ **本片闭合**（该次推送同时带上了 STEP-0499 的独立 CI 裁决回填）。
**STEP-0499（2026-10-21，complete，独立 CI GO）**：**发射侧第一刀 = guest 槽位/局部声明计划，与 Rust backend 实际发射的 local declaration section 逐字节相同**（[STEP-0499](./steps/STEP-0499-m22-w2-s5-slot-plan.md)）。**本片不推进门 3**。
- **为什么把发射拆成两刀**：run 体的每条指令都写着**槽位索引**（`local.get <slot>` / `local.set <slot>`），槽位错一位字节就全错，且**错法与"指令顺序错了"无法区分** ⇒ 先钉中间层，而且**中间层本身就是可对拍的字节**（局部声明段），不是文字描述；下一片一旦对不上，能立刻区分"槽位算错了"还是"指令发射写错了"。
- **guest 侧**：`selfhost/codegen.sico` 新增 `cgs_run_function` / `cgs_flat_hex` / `cgs_record_hex` / `cgs_local_hex`；`selfhost/compiler.sico` 新增 **`--emit-locals-hex`** 驱动模式。
- **对照方式 = 读 oracle 的生产输出**：`runner/sico-runner/tests/selfhost_compiler.rs` 的 `oracle_run_locals_hex(source)` 让 `sico_codegen_wasm::compile_script_program` **真正产出**组件 → `wasmparser` 打开 → 找 core module 导出 `run` 的函数索引 → 取该函数体的局部声明分组 → 重新编码 ⇒ 比 STEP-0496 的"测试文件内复刻计划生成器"更强（对拍的是"guest 的槽位分配 == backend 真的发射出来的槽位分配"）。
- **实测**：guest 输出 `017f017f017f017f017e017f017f017f017f017e017f017f017f017f017e017f017f017f017f017f027f`（84 hex = 21 组 = 19 个指令槽（16 × i32 + 3 × i64）+ dispatcher + scratch 对），与 oracle 实际发射**逐字节相同**（新固件钉 `sico_compiler_emits_the_run_local_declaration_plan_byte_exactly`；钉住 21 组 / 84 hex / 结尾 `017f027f`）。
- **关键决策：计划直接承载为局部声明字节**（`017f`=i32 槽 / `017e`=i64 槽 / `027f`=scratch 对），不做"槽位类型列表 → 再编码"的二次转换 ⇒ 少一层可出错的转换。
- **两个 typed 身份**（ADR-0016 决策 6，越界形状 fail-closed、禁止静默回退）：`ERR:E-SH-CG-IR-PLAN`（IR 里没有 `run` / 缺 `blocks`/`instructions`/`ty`）、`ERR:E-SH-CG-IR-FLAT`（值落在扁平子集之外，或 `run` 声明了局部 cell）。
- **与 oracle 的精确分歧（guest 更窄的一侧）**：oracle 的 `flat_ir_types` 对 `named(N)` 还会把**未声明**的内建 record（`ScriptInput` / `ScriptOutput` / `ScriptError` / `HttpResponse` / `Http2Response`）按已注册布局扁平，guest 一律 typed 拒绝；`list[record]` 同理（backend 在 record 全为 i64 时接受 2 × i32）。即 guest 在这两处**更窄**，方向是 fail-closed，不构成静默近似。
- **顺带修正一处规范与实现矛盾**：`docs/development/script-component-emission-v0.md` §3 原称 `ScriptError` 因 code 是 enum 故不可扁平、恒用 WIT 布局 —— **与实现不符**（`flat_ir_types` 把 `ScriptErrorCode` 映射成 1 个 i32 槽 ⇒ `ScriptError` **可**扁平，用**模块自己声明的布局** flat `[i32,i32,i32]`，字段字节偏移 0 与 8；WIT 布局（偏移 0 与 4）是 §6 **结果区**布局）。已按纪律在同一范围内改正。
- **编号说明**：并行 M23 会话在 `36f0ee9` 提交并占用 **STEP-0498**（`STEP-0498-m23-item3-corpus-freeze-and-d3-record.md`），本片（未入库）**让位重编号为 STEP-0499** —— STEP 编号是跨并行 worktree 的**共享序列**，已提交的取号优先（见 §5 的「协作纪律」条）。
- 回归：全源 census 字节等同（**自举形状前沿未破**）+ 固件 **96 + 2 + 9 + 1 = 108/108** + 前端前沿探针 **18/18** + 计划层前沿探针 **8/8** + **完整验证电池 BATTERY_GREEN**（0245 / 0262 / 两个前沿探针 / 一键 CI）+ 行尾审计（改动文件纯 LF、零 CRLF、零 BOM）。**一键 CI 自本片起是 12 步**（新增 `fmt (runner)`，见 §5 的「验证覆盖」纪律）。**已登记的环境敏感红点**：`prepared_program_reruns_are_isolated_and_survive_a_trap` 的 warm median 预算断言在**本机**偶发红（单独跑 5/5 过；整套件序列包含本片新测试 3 过 4 红、跳过则 3/3 过）——该用例的**产物与 HEAD 逐位相同**、该断言**本就登记为环境敏感**，权威裁决是独立 CI。**独立 CI 裁决已出（GO）**：本片已**提交并推送**（owner 授权）= 提交 `ee2317f` → `github/codex/m22-w1-ci`（fast-forward `0980034..ee2317f`）；独立 CI run **39535461745**（head_sha `ee2317f`）裁决 `conclusion: success`，job `windows-gnu` 内层 **12/12** 步全 PASS ⇒ 上面的环境敏感断言在**独立 runner 上通过**，**预算常量与断言均未改动**、不需要重新基线化。**门 3 仍 NO-GO**：run 体字节发射、37 语料/formatter/全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。
**STEP-0497（2026-10-21，complete，独立 CI GO）**：**修复 M23 会话交接的 CI 红点 = 脚本组件 guest 内存"声明下界 > effective ceiling"**（[STEP-0497](./steps/STEP-0497-m22-w2-ci-arena-ceiling-repair.md)）。**本片不推进门 3**。
- **红点**：`sico-app-cli::run_executes_a_script_package_end_to_end` 在 CI（run 39454624507 / 39457030463）红，错误逐字 `forcing trap when growing memory to 2214592512 bytes`；本机在 `SICO_TEST_WASMTIME` 已设时**逐字复现**（修复前），修复后 **ok**。
- **归因比 M23 归因报告更完整——真正绑定的是包 manifest 的 64 MiB，不是宿主 2 GiB**：`effective_limits = min(包 manifest 请求, 宿主)`，而 `sico-app-cli::script_limits()` 继承 `RuntimeLimits::default()` 的 64 MiB ⇒ **只抬宿主上限，用例实测仍红**；宿主 2 GiB 是第二个独立缺口（同样低于声明下界）。
- **断点**：`b9f652b`（最后一次绿 CI）时 `MEMORY_PAGES = 1024` = 64 MiB，与包 manifest **恰好相等**；`c50b3fd`（STEP-0402/0421/0454/0484）把 `MEMORY_PAGES` 2048 → 33792、`ARENA_LIMIT` 128 MiB → 2 GiB、宿主授权 128 MiB → 2 GiB **一次改完、未同步包 manifest** ⇒ 配对断裂，断裂后第一个 CI 运行即红。
- **修法（取"不改变发射字节"的一侧）**：`MEMORY_PAGES` 不动（避免重生成 `tests/wasm/artifacts.hex` 快照、guest 冻结常量与全部发射字节）；包 manifest 限值抬到 `SCRIPT_MEMORY_BYTES` = 33,792 页 = **2,214,592,512 B**（声明线性内存，min == max）；宿主上限 2 GiB → **2.5 GiB**（与 runner 路径 `RunnerLimits::default()` 同值）；低于声明下界一律 **typed refusal**（`sico-package::build_script_v1` 拒绝 + `sico-runtime::PackageRuntimeError::MemoryCeilingBelowDeclared`）。
- **四条不变量守卫**（codegen 字节级 memory section / package 层 / runtime 层 / runner 层）全部为常量或字节比较、**不依赖 wasmtime** ⇒ 修掉"本地绿灯是空跑"（`SICO_TEST_WASMTIME` 未设时启动缝用例直接 return）的假阴性。
- **模块边界未破**：`runtime` 模块不允许依赖 `compiler` ⇒ `sico-runtime` 不新增 `sico-codegen-wasm` 生产依赖，改经 `sico-package`（`application` 模块，已登记例外）转出 `SCRIPT_MEMORY_BYTES`。
- **RFC-0029 增补 STEP-0497 修正案**：写清 STEP-0454 的 2 GiB = **arena 上限**，声明线性内存 = 2,214,592,512 B（min == max），并**登记设计债**（min == max ⇒ "更严格的包/Host 限额"条款对脚本组件不可实现）。
- 回归：红点转绿 **ok** + 守卫 8/15+4/4/1 全绿 + 一键 CI（`tools/run-ci.ps1`）。**已提交并推送（owner 授权）**：提交 `7e5ad3c`，推送 `github/codex/m22-w1-ci`（fast-forward `6808569..7e5ad3c`）；**独立 CI run 39524849306 裁决 `conclusion: success`**（job `windows-gnu` 全 success，01:01:29Z → 01:28:08Z）⇒ 红点在**独立 runner** 上确认转绿，本片**闭合**。
**STEP-0496（2026-10-21，implementation complete locally）**：**guest IR 摄取第一刀 = IR JSON 扫描器 + `run` 发射计划提取**（[STEP-0496](./steps/STEP-0496-m22-w2-s5-ir-ingestion-plan.md)）。
- **guest 侧**：`selfhost/codegen.sico` 新增字节扫描器（`cgj_ws`/`cgj_string_end`/`cgj_container_end`/`cgj_value_end`/`cgj_span_text`/`cgj_value_text`/`cgj_ident_text`/`cgj_member`/`cgj_object_keys`/`cgj_array_item`）与计划提取（`cgp_type_text`/`cgp_op_text`/`cgp_id_list`/`cgp_field_list`/`cgp_run_plan`）；`selfhost/compiler.sico` 新增 **`--emit-ir-plan` 驱动模式**。
- **计划文本**（五类行，逐行 LF）：`param <id> <name> <type>` / `inst <result> <type>` / `op <operation>` / `term return <value-id>` / `record <record> <field> <type>`；类型文本 `unit|bool|int|i64|u64|float64|string|bytes` / `list(T)` / `named(N)` / `result(ok,error)`；操作文本 `copy`/`const_int`/`const_i64`/`const_u64`/`const_bool`/`const_string`/`const_bytes`/`call`/`intrinsic`/`project`/`construct`/`variant`。
- **oracle 侧不改生产代码**：计划生成器写在测试文件里（`plan_type_text`/`plan_op_text`/`plan_id_list`/`plan_text`）；guest 从扫描到的 IR 推出的文本与 oracle 从 `sico_ir::Module` 推出的文本**逐字节相同**（新固件钉 `sico_compiler_extracts_the_run_emission_plan_byte_exactly`，4 个 fixture）。
- **四类 typed 拒绝各有身份**（ADR-0016 决策 6）：`ERR:E-SH-CG-IR-PLAN`（无 `run`/结构缺失）、`ERR:E-SH-CG-IR-TYPE`（类型超子集）、`ERR:E-SH-CG-IR-OP`（操作超子集）、`ERR:E-SH-CG-IR-TERMINATOR`（终止符非 `return`）；新负钉 4 例。
- **两条格式决策**：名字剥引号并校验"重新编码后字节相同"（必须是纯标识符，否则 fail-closed）；字面量载荷保留 canonical JSON 原样（guest 永不反转义；Rust 侧用 `serde_json::to_string` 重序列化 ⇒ 两侧必然同字节）。
- **本片踩到的三条**（两条真缺陷 + 一条新纪律，已写入 §5 纪律表第 10–11 条）：① **`from` 是 Sico 保留关键字**（lexer `TokenKind::From` + lowerer 的 `unsupported atomic expression` 兜底）→ 形参/局部名不得取语言关键字；② **同函数内 `let` 无块作用域**（不同 `if` 分支里的 `let` 也共用命名空间 → `unsupported cell redeclared`）；③ 重写文件时手抄**冻结 hex 常量**出错（已按 `HEAD` 还原，diff 为纯新增零删除）。
- **燃料成本显式化**：计划模式最小燃料门槛 **10M<cap≤20M**，**超出 runner CLI 默认 10M**（纯 IR 模式为 5M<cap≤10M）→ 新探针 `tools/probe-m22-ir-plan-frontier.ps1` 直接驱动 `sico-runner` 并写明 800B 预算，不把成本藏在 CLI 默认值后面。
- **诚实收窄**：只提取名为 `run` 的函数；类型只支持 11 类；操作只支持 12 类；终止符只支持 `return`；多块（分支）函数被终止符检查拒绝。**本片只做摄取与计划，没有任何 run 体字节发射**——STEP-0487 的 `--emit-component-hex` 仍是冻结的最小 echo 骨架。
- 回归：全源 census 字节等同 + 固件 **107/107** + 0245 exit 0 + 0262 `STEP_0262_OK` + 规划合同 0124 `STEP_0124_OK` + 前端前沿探针 **18/18** + **计划层前沿探针 8/8**。**门 3 仍 NO-GO**：run 体发射、37 语料/formatter/全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。
**STEP-0495（2026-10-21，implementation complete locally）**：**变体 `ok(...)` / `error(...)` = 闭合 Script-profile 三类形状中的第 3 类**（[STEP-0495](./steps/STEP-0495-m22-w2-s5-result-variant.md)）。
- `ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))` → 投影×2 + `const_i64` + `construct`（result 4）+ `variant ok`（result 5，payload `[4]`），terminator `{"kind":"return","data":5}`，**与 Rust oracle 逐字节相同**（新固件钉，4 个 fixture）。
- **关键决策**：沿用 STEP-0494 的 helper 家族（顶层浅嵌套），新增 `type_word_json` / `declared_result_arm_type_json` / `construct_field_count` / `construct_lowering` / `payload_type_json` / `record_variant_function_ir`；`Result[T,E]` 的类型 JSON 走**旁路通道**（同 STEP-0493 教训：绝不让 `declared_return_kind` 返回 JSON）。
- **payload 类型必须校验**（否则产出 `ty` 与 payload 不符的畸形 IR）：与声明分臂不一致 → `ERR:E-SH-IR-VARIANT-TYPE`；形状不支持 → `ERR:E-SH-IR-VARIANT-PAYLOAD`；函数未声明 `Result[...]` → `SKIP` → 落回 `call` 路径 `ERR:E-SH-IR-CALL-TARGET`（与 oracle 的 `E2031 unresolved call target ok` 同裁决）。
- **oracle 真值修正**：`return_type` **恒按声明顺序** `ok` 在前 `error` 在后，**不随实际构造的分臂交换**（首版对调了，由字节对拍当场抓出）。
- **本片暴露并修复一条容量边界（比变体本身更重要）**：全模块 canonical IR 从 HEAD 的 8,231,922 B 增长到 **8,608,383 B**，**越过 Script v0 单通道 8 MiB 冻结上限（8,388,608 B）** → 全源 census 取不回 IR，结局从 `Output` 变 `Trap("malformed result: stdout exceeds 8 MiB")`。根因是这条 census 通道从 STEP-0491 起只剩 ~1.9% 余量。修复 = `selfhost/compiler.sico` 新增 **`--emit-ir-split` 驱动模式**（IR 按字节中点切成两半走 stdout/stderr，拼接即完整 IR；拒绝仍按同一 typed identity 走 `ScriptError`，不隐藏）。**默认单通道路径在全模块规模上仍不可用**，这一限制已登记，未被掩盖。
- **本片踩到的三条**（两条真缺陷 + 一条新语法纪律）：① `type_word_json` 判定顺序错（`record_scalar_kind` 对未知词返回 **ERR 串**而非空串，命名类型因此被判成错误）→ 所有 `Result[...]` 形状统一报 `ERR:E-SH-IR-RECORD-TYPE`；② `return_type` 两臂对调；③ **match 分支绑定与 `let` 单元共用命名空间**（`case ok(x)` 之后再 `let x` → `unsupported cell redeclared`）→ 已写入 §5 纪律表第 9 条。
- **诚实收窄**：只支持 `Result` 的两个**内建**变体 `ok`/`error`（`some(...)` 在当前 Sico 语言里**不存在**，oracle 自己报 `E2031`）；payload 只支持投影 / `I64/U64.literal` / 字符串字面量 / record 构造；`Result` 分臂类型只支持标量或命名类型；只支持"函数体唯一一条 `return`"。
- **Script-profile 前沿钉重钉**：STEP-0488 钉的 `ERR:E-SH-IR-CALL-TARGET` typed 拒绝 → 改为**逐字节等同**（`sico_compiler_lowers_the_script_profile_echo_fixture_byte_exactly`）。**STEP-0488 缺口表的前端三项（投影 / 构造 / 变体）至此全部闭合**。
- 回归：全源 census 字节等同（`--emit-ir-split` 双通道，31.25s）+ 固件 **105/105** + 0245 exit 0 + 0262 `STEP_0262_OK` + 前沿探针 **18/18**。**门 3 仍 NO-GO**：guest IR 摄取、run 体发射、37 语料/formatter/全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。
**STEP-0494（2026-10-21，implementation complete locally）**：**record 构造 `construct` op = 闭合 Script-profile 三类形状中的第 2 类**（[STEP-0494](./steps/STEP-0494-m22-w2-s5-record-construct.md)）。
- `ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0))` → 每个字段值先各自降低成一条指令（result 1/2/3），construct 引用这些 id（result 4），terminator `{"kind":"return","data":4}`，**与 Rust oracle 逐字节相同**（新固件钉，344 B fixture）。
- **关键决策：走 helper 家族，不改 `scalar_ir` 的深层 return 分析**（拆内联分析链会落在 guest 自举的形状前沿上）。新增三个**顶层浅嵌套**函数：`construct_field_comma` / `construct_field_value_ir` / `record_construct_function_ir`；结果 id 记账显式（`rcf_next_result`）。
- 接入点在 helper 分派链**最前** → "确实是构造但字段不被支持"给出明确 typed 拒绝，而不是掉进 `call` 路径报 `ERR:E-SH-IR-CALL-TARGET`。`script-profile-run` 拒绝身份**不变**（外层 `ok(...)`）。
- **本片踩到三条真约束**（已写入本手稿 §5 的"语法与语义纪律"表）：返回类型必须写 `returns X:`（`-> X:` **无语法诊断**但语义模型静默丢函数 → 调用点 `E2031`）；同函数内 `let` 不可重名；token 计数边界看最后一个 token 的后继索引。
- **诚实收窄**：oracle 的构造用通用递归降低字段值（任意表达式）且 lower 阶段不校验字段名存在，guest 两者都更严。
- **下一片（变体 `ok(...)` / `error(...)`）的 oracle 真值已测并写进 STEP-0494 §8**：`Result` 类型 JSON = `{"kind":"result","data":{"ok":T,"error":E}}`；`variant` op = `{"op":"variant","data":{"name":"ok"|"error","payload":[<ids>]}}`，**payload 是 id 列表**；`variant` 的 `ty` 是**整个 Result 类型**；嵌套顺序 = payload 先降（construct result 4）再降 variant（result 5）；**result id 起点 = `parameter_count`**（无形参 → 第一个指令 result 0）；variant range = 变体表达式跨度。
- **一条重要的事实修正**：`some(...)` 连 **Rust oracle 都报 `E2031 unresolved call target some`** —— `Option` 的 `some` 变体在当前 Sico 语言里**不存在**，所以 STEP-0488 缺口表里那一行**不是缺口**；**缺口 2 的实际内容只有 `ok` / `error` 两个 `Result` 变体**。
- 回归：全源 census 字节等同 + 固件 102/102 + 0245 exit 0 + 0262 `STEP_0262_OK` + 前沿探针 15/15。**门 3 仍 NO-GO**。
**STEP-0493（2026-10-21，implementation complete locally）**：**命名返回类型管道 = 闭合 record 构造/变体的前置缺口 1**（[STEP-0493](./steps/STEP-0493-m22-w2-s5-named-record-return-type.md)）。
- `returns <已声明 record>` 现在降低为 canonical `return_type` = `{"kind":"named","data":"<Name>"}`；直返同名形参时 instructions 空、terminator `{"kind":"return","data":0}`（返回形参 id），**与 Rust oracle 逐字节相同**（新固件钉）。
- **关键决策：不动标量 kind 通道**。`declared_return_kind` 的 19 处调用点全部把返回值当**裸标量 kind** 用（`is_word(kind,"u64")` / `"ty":{"kind":"`+kind+`"}` / `length(kind)==0` 门闸）；让它对命名类型返回 JSON 会把 JSON 片段拼进 IR 字段 → 畸形 IR。故走**旁路**：新增 `record_declared` / `declared_return_named_type` / `binding_parameter_named_type` / `named_return_shape_gate`，19 处调用点零改动。
- **修掉一个真实的 fail-closed 违规**：改动前"命名返回类型 + 语句体"（while 形态）输出**畸形 IR** `"return_type":{"kind":""}`（ADR-0016 决策 6）→ 形态闸门把它收敛成 typed 拒绝 `ERR:E-SH-IR-RETURN-TYPE`。helper 家族按标量 kind 拼 `return_type`，所以闸门必须落在**第一个 helper 调用之前**。
- **诚实收窄**：只支持"命名返回类型 + 直返同名形参"；带 `let`/`set`/`match`/`if`/`while` 的命名返回类型函数一律拒绝（**Rust oracle 能降低其中一部分**，guest 缺多指令结果 id 记账）；未声明 record 的形参类型 oracle 也接受 → guest 更严，同为收窄。
- **下一片（record 构造 `construct` op）的 oracle 真值与剩余两项缺口已写进 STEP-0493 §8**（构造实参的子表达式降低 / 多指令结果 id 记账）。
- 回归：全源 census 字节等同 + 固件 100/100 + 0245 exit 0 + 0262 `STEP_0262_OK` + 前沿探针 12/12。**门 3 仍 NO-GO**。
**STEP-0492（2026-10-21，implementation complete locally）**：**投影泛化到任意已声明 record（解除 `ScriptInput` 硬编码）**（[STEP-0492](./steps/STEP-0492-m22-w2-s5-generic-record-projection.md)）。
- `parameters_ir` 的命名形参类型改报**实际声明的记录名**；投影分支的基名类型判定放宽为 `record_named_type`；字段存在性/标量判定仍由 `record_field_kind` 承担（fail-closed 不放松）。
- Rust oracle 真值已测（159 B fixture）+ 新固件钉逐字节等同。**连带修正一个真实的测试耦合缺陷**：隔离函数组合测试的 Rust oracle 先报 `E2031 unresolved call target record_named_type` → 组合是**显式依赖清单**，新增依赖必须同步登记。
- **下一片（record 构造）的 oracle 真值与三项前置缺口已写进 STEP-0492 §7**（命名返回类型 / 构造实参的子表达式降低 / 多指令结果 id 记账）。
- 回归：全源 census 字节等同 + 固件 98/98 + 0245 exit 0 + 0262 `STEP_0262_OK` + 前沿探针 9/9。**门 3 仍 NO-GO**。
**STEP-0491（2026-10-21，implementation complete locally）**：**S5 前端 record 字段投影闭合 = Script-profile 第 1 类形状打通**（[STEP-0491](./steps/STEP-0491-m22-w2-s5-frontend-record-field-projection.md)）。
- `input.stdin`（`input: ScriptInput` + 已声明 record + 标量字段）→ canonical `project` op，**与 Rust oracle 逐字节相同**（新固件钉 `sico_compiler_lowers_script_input_field_projection_byte_exactly`）；range = "." + 字段名 token 对。
- fail-closed：缺 record / 字段不存在 / 非 `ScriptInput` 形参 / 非标量字段 → typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（新固件钉 3 例）。
- **本片最硬的发现 = guest 自举降级机器有"形状前沿"**：深层块内 `match`、深层块内 `set` 的嵌套 intrinsic 右值、尾部 `match` 主语取循环体内赋值局部 → 全源对拍由 exit 0 变 `ERR:E-SH-IR-CALL-SHAPE`。后续刀必须沿用已证形状（详见 §4 的形状前沿表）。
- 修掉两个真缺陷：跨块读取块作用域 `let` → 扫描游标归零死循环（fuel 耗尽）；Sico 源码**无 `#` 注释语法**（词法错误）。
- 探针随推进重钉：8/8 判定一致（投影 → lowered + 2 例负钉）。全源 census 字节等同 + 固件 85/97 + 0262 `STEP_0262_OK`。
- **下一刀 = record 构造**（`ScriptOutput(stdout: ..., ...)`）→ 变体（`ok`/`some`）→ guest IR 摄取 → run 体发射 → canary 阶梯。门 3 仍 NO-GO。
**STEP-0488（2026-10-20，diagnosis/registration）**：**IR 驱动发射的前置缺口 = guest 前端不能降低 Script-profile 形态**（record 投影 / `ok(...)`·`some(...)` 变体 / record 构造）——**缺口 1 已由 STEP-0491 闭合**。
- 证据：探针 `tools/probe-m22-frontend-frontier.ps1`（6/6，`FRONTIER_UNCHANGED` exit 0 = 前沿未移动）+ 固件前沿钉（typed 拒绝身份稳定）。record **声明**已支持且 IR 的 `"records"` 表已按 canonical 格式产出。
- 发射算法规格已写死（局部槽位计划 / op 发射 / Script 返回 / Canonical ABI 冻结事实 / `user_records` 优先级）+ 目标字节 = 981 B（run 体 223 B 逐条反汇编）。
- 下一刀顺序 = 前端 record 投影 → record 构造 → 变体 → guest IR 摄取 → run 体发射 → canary 阶梯。门 3 仍 NO-GO。
- **证据链修复**：`tools/validate-step-0262.ps1` 的 stdin 在 Windows PowerShell 5.1 上被 .NET 注入 UTF-8 BOM（guest `source_len` 68→71 → range 全体右移 3 → 冻结 fixture 对拍失败）。改字节精确文件重定向后 `STEP_0262_OK` 恢复；guest lowering 与 fixture 均无误。同模式脚本（canary / budget / corpus / 0246..0261）待同样改造。
**STEP-0487（2026-10-20，implementation complete locally）**：**S5 骨架 = guest 发射完整 Component 981B 字节等同 Rust（census 不回归；电池 82/82）**。
- 布局 11 段文档化；载体 `--emit-component-hex` + 固件对拍；骨架诚实（LEB 实算 + run 体 = IR 槽位）；只覆盖最小冻结源形。
- 剩余段有序：IR run 体 → 数据段 → 助手表 → 辅助函数 → transport → canary 阶梯。警示：e2e run 体现撞 typed 门，0488 IR 输入取自已证面。
- 备份 backup_0500_codegen.sico。census-latest 已同步。
**STEP-0486（2026-10-20，波次登记片）**：**S5 component-codegen 波次合同冻结**（[STEP-0486](./steps/STEP-0486-m22-w2-s5-codegen-wave.md)）——门 3 实质 = 自举编译器只产 IR 不产 Component 字节；Sico 侧 RFC-0011 发射逐字节对齐 Rust backend 为最后大山；canary 顺序与合同五件已冻结，首个实现 STEP = 骨架对拍。
**STEP-0485（2026-10-20，出口审计片）**：**M22 NO-GO（[复审报告](./reports/m22-exit-audit-2026-10-20.md)）**——门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO；预算记录落地（25.93s/800B/2GiB+8GiH/120s）；主欠 = 组件 B（A==B==C + .sapp）+ 提交快照 CI。R3 停滞 0。下一 = 门 3。
**STEP-0484（2026-10-20，implementation complete locally，census 149/149 diffs=0 HIST {} + 全源降低 exit 0 = S6 达成）**：**arena treadmill 落地（RFC-0029 amendment）**。
- 设计 = perm/scratch 双区 + enter/exit_scratch + push_chunk；scalar_ir 走查 13 累加器改单圈 join；固件钉反转 = 全源 exit 0 字节对拍。
- 怪癖入册：gw_rhs_packed 特定位置禁 // 注释（27 户雪崩实锤）。CLI 无 hostcall 旗 = 已知余量。
- 电池全绿。备份 parser_backup_0498。census-latest = census-0501。下一 = S7 + 出口审计。
**STEP-0483（2026-10-19，implementation complete locally，census 148/149、diffs=0）**：**general_while 翻绿 = 子代理双杀（gca 左臂 pieces 门 + gtc 提前 return）**。
- 方法论入册：id-only 对照会被巧合重叠骗过 → (id,op,range) 三元组；合成复现不可靠 → census 闭包唯一可信。
- 红户 1 = scalar_ir 抵 arena 墙（typed 门全清零实锤；唯一 blocker = treadmill）。
- 电池全绿。备份 parser_backup_0494。census-latest = census-0494。
**STEP-0482（2026-10-19，implementation complete locally，幻影 +23→+2，电池全绿）**：**子代理猎杀双实录**。
- 代理 A = gla 调用形 count 欠账修复（+23→+2）；代理 B = gla 字面臂 pieces 追加化（中性保留）。
- 残余 +2 钉在 `set gcur_re = gw_decimal_value(word_at(gem_f, U64.literal(7)))` / block 231 边界；gla 两臂已净 → 嫌疑 gws write 段 / terminator。
- **教训 = Agent 中断频发，子代理工作必须备份+验证幂等**。
- 备份 parser_backup_0491。census-latest = census-0490。
**STEP-0481（2026-10-19，诊断片，零源码变更，电池全绿）**：**幻影语句 = gws unpack 区嵌套用户调用 let**（`let gws_value = gw_decimal_value(word_at(gws_parts, U64.literal(1)))`）。
- +23 = 该 RHS 嵌套调用链（multi_pack→wcr 递归→三行转四行）count 组装 vs 实际发射不符；嫌疑 = multi_pack 尾部 gwc 段 emit_count 累加。
- 下片 = 该形状 count 逐行审计。备份 parser_backup_0487 = 当前树。
**STEP-0480（2026-10-19，诊断片，零源码变更，电池全绿）**：**幻影槽实锤 = 结果号重编号（非漏发）**。
- v6 对照：指令内容/范围逐条全同；唯一差 = set 语句边界结果号跳变 877→901（幻影 23 槽）= RHS pack count 虚报 +23（源 ~408324 set 语句）。
- "漏发"假象入册：id 跳空致内建计数差，非真实漏发。
- 下片 = 该语句 RHS pack count 审计。备份不变（parser_backup_0487 = 当前树）。
**STEP-0479（2026-10-19，诊断片，census 147/149、diffs=1 维持，电池全绿）**：**漏发链追击 = 首 gap 定界语句级**。
- 内建计数对比 + 结果号-范围对照（跨函 ID 碰撞陷阱入册：result id 每函数重置，对照键须带函数序）。
- 首 gap（886→901 十五槽）= gws set-write 语句边界（u64_list_at 写路径）；guest 多前进 15 槽 = pack count 虚高或守卫跳过。
- 下片 = 函数序感知对照 → 漏发语句源范围 → packer 守卫审计。
**STEP-0478（2026-10-19，implementation complete locally，census 147/149、diffs=1 维持）**：**gcp list.length 臂自引用实参 bug 两行修复 + DIFF 逐差定位法成型**。
- 宽窗对拍法（DIFF-AT ±700B）一击 = gcp_gll/gcp_gl cell 分支实参串变量（intr_id 应为 r_base；复制模板未换名）。
- 次差移位 = guest 漏发 ~27 内建（concat -20/to_text -6/list.length -1；首 gap 十五槽于 gws unpack 区）= 下片目标。
- 电池全绿（81/81 + local-bounds）。备份 target/tmp/parser_backup_0487.sico。census-latest = census-0478。
**STEP-0477（2026-10-19，implementation complete locally，typed 前线全贯通，census 147/149、diffs=1）**：**全源守卫钉反转 = walk 达内存墙**。
- ①hostcall 传输墙（stdin*64+64MiB）实锤 → 探针/守卫同升 8GiB；②text.length/bytes.length 调用形实参机器扩展被 STEP-0385 钉驳回（合同 = 源级 hoist）→ 回退 + 14809 双 hoist（248 顶板）；
- **残余 DIFF 户已 pinpoint**：gws 区 list.length 条件操作数 intrinsic args [283] vs [282]（SSA 自引用差一）+ 级联；下片一击目标。
- 电池全绿（钉反转后）。备份 target/tmp/parser_backup_0486.sico。census-latest = census-0477。
**STEP-0476（2026-10-19，implementation complete locally，census 147/149、0 DIFF，红户 2 = CALL-SHAPE〔更深〕）**：**gcp sico 右操作数委托门两修**。
- ①gcp_rd（0465 委托）漏 gcp_close_idx/gcp_r_done 门（三兄弟臂全有独缺 = 0465 遗漏）→ 补两行；②gcp text.length 臂单 token 限制 → 尾部条件 hoist 减法。
- **事故实录 = parser.sico 外部清零（92 万 NUL）→ 备份链 0483 恢复 + 重放零丢失**。
- 钉第七次重钉。电池全绿。备份 target/tmp/parser_backup_0484.sico。census-latest = census-0476。
**STEP-0475（2026-10-19，implementation complete locally，census 147/149、0 DIFF，红户 2 = EXPRESSION〔更深〕）**：**to_text 调用实参门开瓶 = gw_ut_chain_packed gut_open off-by-one（+4→+5）**。
- 诊断教训入册：段界 bound 下 token dump 漂移假象——绑定点 dump 为正器。
- walk 推进过 concat/append 装配区；钉第六次重钉。
- 电池全绿。备份 target/tmp/parser_backup_0482.sico。census-latest = census-0475。
**STEP-0474（2026-10-19，implementation complete locally，census 147/149、0 DIFF，红户 2 = CALL-SHAPE〔更深〕）**：**gca/gcc/to_text 门族三修 + 第四户翻绿**。
- ①gca 左右操作数 sico.* 委托臂（R 侧 is_word "sico" 总闸）；②gcc 增 list.length/text.split_lines/u64.to_text 三臂；③split_lines_emit 整体重写（单尾 return 契约；首版双臂 return 触门实锤）；④to_text 调用实参经 helper gw_ut_chain_packed（逗号守卫保 SHAPE 钉）。
- 固件：wcr 闭包清单 +2；钉第五次重钉。
- 电池全绿。备份 target/tmp/parser_backup_0480.sico。census-latest = census-0474b。
**STEP-0473（2026-10-19，implementation complete locally，census 146/148、0 DIFF，红户 2 = EXPRESSION〔更深〕）**：**general_while 机器本体门面四连破**。
- 四修：①wcr 委托宽度覆盖；②gla 调用形实参臂（wcr 三行契约 + 字面守卫）；③fli → fixed_literal_const_segment 段变体（seg_end 参数化，嵌套实参 literal 门灭）；④multi_pack 臂尾堕穿修复（显式打包 return）。
- 固件伴生：wcr 闭包清单 +2；遗留 caller 标签清算。
- 下一门精确定位：gca sico.* 操作数臂（match 主语 13625/14125）。
- 电池全绿。备份 target/tmp/parser_backup_0478.sico。census-latest = census-0473。
**STEP-0472（2026-10-19，implementation complete locally，census 145/147、0 DIFF，红户 2）**：**gw_match_open_pack 翻绿 = gw_condition 裸 bool 形参守卫臂 + gw_cond_next 打包契约扩展**。
- @gwcond 标记一击 = `if cur_term:`（bool 形参直接守卫；oracle = branch-on-param SSA 零指令）。
- 三件套：①gw_condition param 臂（零指令 pack，parts[3]=gnext 覆盖）；②helper gw_cond_next；③两消费点 let 中性改调。
- 顺带第二户翻绿（145）；红户 2 = CALL-ARGUMENT（general_while/scalar_ir 更深）；钉第三次重钉。
- 电池全绿。备份 target/tmp/parser_backup_0476.sico。census-latest = census-0472。
**STEP-0471（2026-10-19，implementation complete locally，census 143/146、0 DIFF，红户 3）**：**gw_slice_subject_packed 翻绿 = 嵌套 match 成功臂 return 摊平**。
- @gwterm 标记一击 = 函数尾成功 return 埋于双层 match 嵌套（error 臂 return 正常、成功臂不设终止位）→ 摊平为 set + 单尾 return（flod 范式）。
- 连带：其余 3 户门前移 RETURN-COUNT→CALL-TARGET；全源守卫钉再重钉（第二次）。
- 0245 fmt 故障实锤 = 中断会话遗留 scratch 探针文件（非 fmt 回归）。
- 电池全绿。备份 target/tmp/parser_backup_0474.sico。census-latest = census-0471。
**STEP-0470（2026-10-19，implementation complete locally，census 142/146、0 DIFF，红户 4，EXPRESSION HIST 清零）**：**control_while/scalar_ir EXPRESSION 户门面瓦解 = 条件内嵌调用形操作数门族**。
- 诊断链：@fn 全库标记 → **decline 污染实锤**（fli 标记使 control_if 拒值失精确 → decline 不破案，探针伪影入册；正解 = 只标 gw-usercall 族 + gcp 7 站〔不触 decline 路径〕）→ GCP-LKIND + 条件 token 8 连词转储一击 = ①9780 括号包裹调用形操作数（gcp 不剥括号）→ 去括号；②9921 调用形操作数直形 → hoist 至 cell（sws_recv_next / swl_close_next）。
- 教训入册：**extract_closure 必须在源码手术后重跑**（过期闭包探针伪影第二例）。- 收尾：control_while 250 locals 超地板 → 减法 shave 2（死 let 删除 + 单用 let 内联）；全源守卫钉重钉 RETURN-COUNT。

- control_while 户 IDENTICAL 翻绿；scalar_ir 换挡 RETURN-COUNT（预登记大函数类）；红户 4 = RETURN-COUNT 全。
- 电池全绿（0262/0245/canary 30/30/0124/diff-check）。备份 target/tmp/parser_backup_0471.sico。census-latest = census-0470。

**STEP-0469（2026-10-19，implementation complete locally，census 141/146、0 DIFF，红户 6→5）**：**fli/sef 门族修复 = helper 抽取（0343 纪律）+ 减法式源重构三案**。
- ① `fixed_literal_or_delegated_packed` helper 落地（0468 回退的内联守卫整体搬出；scc return-else + sws_rhs 两处单行调用）→ fli 门（sico.* 调用形直达 fli 守卫缺失族）瓦解；helper 首版 if/else 内 return 形态触发 guest RETURN-COUNT 门（scc/control_if 连带红）→ **guest 单尾 return 契约**入册，改 set + 单尾 return 修复。
- ② sef GWCOND 门 = 自体 while 条件内嵌 `sico.bytes.length` + 循环内 `sico.bytes.at` set → 减法重构（`find_text_from(src, "let ", cursor)` 单调用等价替换 21 行重试循环）。
- ③ gws 关键字锚定碰撞实锤 = guest 搜 "let" 命中签名形参 `let_count`（range -32 DIFF）→ 针改 `let `/`set `。
- sef 闭包对拍 IDENTICAL；红户 5 = EXPRESSION 2（control_while/scalar_ir）/ RETURN-COUNT 3（gw_slice_subject/gw_match_open/general_while）；全源前线 = EXPRESSION；电池全绿（0262/0245/canary 30/30/0124/diff-check）。
- 备份 target/tmp/parser_backup_0469.sico（手术前后双备份；0468 = 0458 净态）。census-latest = census-0469b。

**STEP-0467（2026-10-18，诊断片，源码净 = 0465 委托态复证）**：**条件路径字面 EXPRESSION 站全灭**。
- ocl/swc 全 0 站；chain/gcp 已验静默——残余 GWCOND 的精确 EXPRESSION 经变量透传，发射函数未定。
- 0468 = 全库字面 EXPRESSION 站 @fn 标记一轮（exit-code 铁律 + 避开 control_if decline 精确串〔只标非 control_if 站〕）一击指认。
- 委托态保留（无害有证）；红户 6 不变；电池继承全绿。基线 census-latest = census-0465。备份 target/tmp/parser_backup_0467.sico。

**STEP-0466（2026-10-18，诊断片，源码净 = 0465 委托态复证）**：**委托生效实证 + 残余门再定位**。
- chain 全读：仅认领左=sico.length/at 宿主的条件，其余委托 gcp → "chain 首败"假设排除。
- **RDELEG2 触发标记实证右委托点火且 gw_rhs_packed 成功**（0465"未触发"系标记挂 ERR 分支误读——委托成功路径不显示）。
- 残余门 = GWCOND 仍在；左右非 sico else 全否（LKIND/RKIND 静默）→ 残余 EXPRESSION 在更深透传（operand_cell_or_literal_json / 其他条件路径）。
- 委托保留（无害有证 = census 0 DIFF）。0467 = operand_cell_or_literal_json 逐站插桩。
- 红户 6 不变；电池继承 0465 全绿。基线 census-latest = census-0465。备份 target/tmp/parser_backup_0466.sico。

**STEP-0465（2026-10-18）**：**gcp sico.* 操作数双侧委托落地（无害实锤）+ 首败转移实证**。
- 委托：左（深度感知扫描至逗号）+ 右（条件闭括号锚定）→ gw_rhs_packed 打包 + 4 行契约解包；base 对齐 gcp_r_base。census 139/145、0 DIFF = 委托对绿户零影响 ✓。
- RDDELEG 标记实证右委托未触发 → 首败已不在 gcp_r cell 分支：真首败候选 = **chain_condition_packed 认领路径**（U64/I64 起首条件先经 chain，非 SKIP 返回先于 gcp）。
- 电池全绿（0262 81/81 / 0245 / canary 30/30 / 0124 / diff-check）。基线 census-latest = census-0465。备份 target/tmp/parser_backup_0465.sico。
- 0466 = chain_condition 路径转储（2 EXPRESSION 站 + SKIP 判定）定位 sef 现首败。

**STEP-0464（2026-10-18，根因片，源码净 = 0458 验证态复证）**：**sef 门根因 = gcp 比较操作数不收 sico.* 内建调用形**。
- 决定性插桩：`r:sico ~ fn:straight_environment_function_ir`（name_idx 正确）——右操作数 = `sico` = 比较条件内嵌 sico.* 调用；实锤宿主 = general_while 自体 `while U64.less_than(gbsi, sico.list.length(gwt_entries))`（14305，同形 3 处）。
- gcp 操作数三级解析（param→callee→cell）对 `sico` 全落空 → GCPX5。**纯形状缺口、组合无关、单测可复现**——sef 案的"组合敏感"框架撤回（fli 案仍开放）。
- 0465 = wcr-W2 式委托修复（gcp 两操作数加 sico 分支 → gw_rhs_packed 打包；span = pos0 至逗号 / pos1 至条件闭括号）；预期解锁 sef + general_while + 级联。
- 红户 6 不变；电池继承全绿。备份 target/tmp/parser_backup_0464.sico。

**STEP-0463（2026-10-18，诊断/基线锁定片）**：**binding_parameter_index 审计 = 本地正确 + 双案并案**。
- 审计结论：binding_parameter_index 逐行正确（三元扫描 + List+3/Map+5；sef 参数表应命中 body_end=4）——0462"param 查找失效"降级为上游嫌疑。
- GCPX5 到达路径剩两候选：①gcp_r_idx 错位（operand width 误算）②name_idx 指向非 sef 函数（扫描目标错位 = 0391 级真 bug 形态：查找输入被 walk 上下文污染）。
- **fli/sef 组合敏感双案并案** = "walk 上下文污染查找输入"类（查找函数本地全正确、输入在特定模块上下文错位）。
- 0464 微复现 = 探针壳矩阵（同签名 + while 操作数组合〔cell,param〕/〔param,param〕/〔cell,cell〕 × 前驱 let 数）定真凶；若②实锤 = guest 机器内 name_idx 传链 bug（修复面可能一处瓦解多门）。
- 基线锁定：census 139/145、0 DIFF；电池全绿（0262 81/81 / 0245 / canary 30/30 / 0124 / diff-check）。备份 target/tmp/parser_backup_0463.sico。红户 6。

**STEP-0462（2026-10-18，诊断片，源码净 = 0458 验证态复证）**：**sef 门根因级定位 = 右操作数 param 查找失效**。
- 逐站排除（GCPX1-6 + chain×2 + cell-branch 全插桩）：GCPX5 = gcp **右操作数 cell 分支**（operand_nonparam_kind = ""）。
- 右操作数 = `body_end`（sef 的真参数）→ 到 cell 分支唯一路径 = `binding_parameter_index(words, name_idx, "body_end")` 意外未命中 → 级联落空。**根因指向 param 查找在 sef while 上下文失效**（0391 级真 bug 首次 pinpoint 到具体查找调用）。
- 0463 = binding_parameter_index 微复现（探针壳 = 同签名 + 同 while 形 + 变体矩阵：参数位置/前驱 let 数/名字长度），修 guest 或 sef 改写。
- 插桩全撤净态复证；红户 6 不变；电池继承全绿。备份 target/tmp/parser_backup_0462.sico。

**STEP-0461（2026-10-18，诊断片，源码净 = 0458 验证态复证）**：**sef 门窄化 = gw_compare_packed 的 cell-kind 检查拒**。
- 插桩实证：失败条件 = U64.less_than（cond 首词 U64）；发射点上文 = `set gcp_ckind = "u64" / else: return EXPRESSION`——某 cell 操作数的 cell_kinds 值 ≠ "u64"。
- 嫌疑精化 = sef_body_idx（`let sef_body_idx = body_from` 别名 let 的 kind 跟踪错位）或 cell 索引错位（sef 特定态）。
- 0462 首刀 = 该 else 点单站 kind 值转储（安全后缀法）→ 修 guest kind 跟踪 或 sef 侧改写。
- 红户 6 不变；census 139/145、0 DIFF；电池继承全绿。备份 target/tmp/parser_backup_0461.sico。
- 战略备忘：六红户中 sef 最接近突破（门已窄化到单点检查）；fli 挂账（5 片未破）；RETURN-COUNT 3 户为大函数手术。

**STEP-0460（2026-10-18，侦察/诊断片，源码净 = 0458 验证态复证）**：**RETURN-COUNT 三户手术量实测 + sef GWCOND 门定位**。
- RETURN-COUNT 三户实测 = gss 391 行 / gw_match_open 273 / general_while 941——0390 单返回模板逐户成本高，暂缓。
- sef GWCOND 门插桩（GWCOND 发射点 token 转储 = 安全非 decline 串）：失败条件 = **sef 主 while `U64.less_than(sef_body_idx, body_end)`**（最标准形）→ 真凶在 gw_condition 内部 operand 解析透传（gw_condition 字面 EXPRESSION 站 = 0，全经 callee 透传）。
- 0461 首刀 = gw_condition 的 operand 解析 callee 逐层转储（从 operand_cell_or_literal_json / binding_parameter_kind 起；其 passthrough 的 EXACT EXPRESSION 会触发 general_while GWPACK 支 = 反噬风险，故逐层单测其闭包而非全库插桩）。
- 源码净态复证；电池按 0458 继承。备份 target/tmp/parser_backup_0460.sico。红户 6 不变。

**STEP-0459（2026-10-18，实验负结果片，源码净 = 0458 验证态复证）**：**fli 门 hoisting 假设证伪**。
- fli_magnitude 深嵌 let 上提函数顶（语义等价）→ 全源仍 EXPRESSION：门不随 let 嵌套位置移动。排除清单 += let 嵌套深度。
- fli 门候选剩余：值/使用点依赖（decimal_at_most 调用形）或 control_if 透传链上的接收器形。0460 = control_if set/let 处理器 EXPRESSION 位点逐点插桩（K 编号 + 绑定名，避开 decline 精确串 "ERR:E-SH-IR-EXPRESSION" 本体——只改其他 11 个非 decline 位点）。
- 复证：fli 闭包 IDENTICAL；电池按 0458 继承。备份 target/tmp/parser_backup_0459.sico。
- 战略备忘更新：fli 已猎 5 片（0451/52/53/58/59）；若 0460 不破，红户 6 中先转 GWCOND/sef + RETURN-COUNT 3 户（模板成熟 = 单返回/条件化），fli 挂账 arena 复用轨后批量瓦解。

**STEP-0458（2026-10-18）**：**2GiB arena 常态固化 + stub v2 复证 fli_magnitude + ddmin 严格分类教训**。
- 2GiB 正式常态（0454 RFC 同步面已绿）：census 139/145、0 DIFF 复证；电池全绿（0262 81/81 含 EXPRESSION 钉）。
- stub 矩阵 v2（exit-code 分类）复证盲化 fli_magnitude 即推进门（→GWPACK-OTHER）——fli_magnitude 在门路径上成立。
- ddmin 教训：**分类必须严格匹配目标错误**——宽松（非 CLEAN 即毒）被 extras 自身 CALL-TARGET 伪影污染（缺 callee 先死自身从未达 fli）；"MINIMAL POISON = at_dispatch" 为伪结论（trivial 同名对照 CLEAN + 双闭包合并 CLEAN 双证伪）。
- fli 门最小复现 = 全源本身；error 经 control_if 路径透传（非 general_while let 处理器 GWPACK 支，0452 tagged-run 实证未命中）。
- 红户 6 = scalar_ir/control_while EXPRESSION 2 / sef GWCOND 1 / general_while+gw_match_open+gw_slice RETURN-COUNT 3。基线 census-latest = census-0458。备份 target/tmp/parser_backup_0458.sico。
- 0459 路线：结构化接收器转储穿透 control_if 透传链（GWPACK-OUT 临时哨兵或非字符串通道）或 fli 侧改写绕过（fli_magnitude 使用点仅 2 处 decimal_at_most，候选 = 重写为已建模形）。

**STEP-0457（2026-10-18，纠错片）**：**分类器 bug 实锤——0455/0456 发现撤回**。
- 根因：guest walk 成功时输出 canonical JSON **内嵌输入源**（parser.sico 函数体含 "ERR:E-SH-IR-*" 字面量）→ 对 stdout grep 错误模式 = 必然假阳性。**正分类器 = exit code**（0 净 / 122 domain 读 stderr）。
- 撤回清单：0455 stub 矩阵的 fli_magnitude 定位（break 条件即假阳性）；0456 全节（偏移依赖/头翻转/序数排除/组合敏感统一解释全无效）。
- 复测复证：fli 闭包 6 连跑（exit-code 分类）全 CLEAN，与 cmp IDENTICAL / census FLIP 一致。
- 存续结论：0451 arena 实验（host 结构化判读可信）+ 0454 RFC 第三次修订 + 全源 typed 前线死于 fli EXPRESSION（组合敏感为真实现象，0391 型状态说复活）。
- **探针纪律第四条入册：guest 输出含错误字面量，禁止文本模式分类，一律 exit code / 结构化 outcome**。
- census 139/145、0 DIFF（同一性）；电池继承全绿。备份 target/tmp/parser_backup_0456.sico。
- 0458 = stub 矩阵 v2（exit-code 分类 + 2GiB 重启用）重定位 fli 门 → 门形手术。

**STEP-0456（2026-10-18，发现片→已撤回）**：~~偏移依赖根因类实锤——组合敏感族统一解释~~（分类器假阳性所致，见 STEP-0457）。

**STEP-0455（2026-10-18，诊断片，源码净 = 0454 绿态复证）**：**fli let 级 stub 矩阵法定型 + 门定位 = fli_magnitude**。
- stub 矩阵（逐 call-let 盲化 + 全源探针复跑）一击定位：盲化 `fli_magnitude = word_at(words, fli_value_idx)` 即清门。发射链 = 用户调用臂 → multi_pack（word_at 的 2 参形 list+index 与 list.append 同形）在 fli 前驱状态下拒；gic 的同款 word_at let 绿 = 组合敏感态实锤再例。
- 基线漂移观察：裸 CLI 探针报 GWPACK-OTHER、固件 harness（源+尾换行）报 EXPRESSION → 门位对输入微差敏感；**守卫钉（EXPRESSION）脆弱性待 0456 复核**（若漂移稳定存在，钉应改 starts_with 类宽匹配或固定输入形态）。
- census 139/145、0 DIFF（同一性）；电池继承 0454 全绿。备份 target/tmp/parser_backup_0455.sico。红户 6 = scalar_ir/control_while EXPRESSION 2 / sef GWCOND 1 / general_while+gw_match_open+gw_slice RETURN-COUNT 3。
- 0456 路线：fli_magnitude 门形手术（multi_pack 2 参 u64-index 容忍臂 或 fli 侧改写）+ 守卫钉漂移复核。

**STEP-0454（2026-10-18）**：**RFC-0029 第三次修订落地 = guest arena 2GiB——census 139/145（+1）、0 DIFF，NON-DOMAIN 清零**。
- 同步面：codegen ARENA_LIMIT 2048MiB / MEMORY_PAGES 33792；runtime HostLimits 2048MiB；canonical tests 页数 33792；runner 默认 2.5GiB（**组件最低 2GiB + 余量必需**——ceiling 等值触发 MemoryLimit 实锤）；固件 guest-run 2560MiB ×2；全源守卫燃料 800B；census 探针 2560MiB。
- RFC-0029 amendment 段（STEP-0454）含 0451 实验阶梯结论：全源实需 ∈ (512MiB, 2GiB]；客内上限编译固化（host 加码无效）；treadmill 复用仍登记为长期正解。
- 全源守卫第十钉：arena trap → fli EXPRESSION（typed 前线钉）。红户 6 = EXPRESSION 2 / GWCOND 1 / RETURN-COUNT 3（NON-DOMAIN 3 户 arena 解锁 = 1 翻绿 + 2 转 typed）。
- 电池全绿：fmt×2 / 0262(81/81) / 0245 / canary(30/30) / 0124 / diff-check。基线 census-latest = census-0454。备份 target/tmp/parser_backup_0453.sico（parser 净态延续）。
- 0455 路线：fli 门（全源 typed 前线首门；2GiB 下 let 级二分或子 packer 臂扩展）→ 其后 GWCOND/sef + RETURN-COUNT 3；arena 复用（treadmill）长期项仍在册。

**STEP-0453（2026-10-18，诊断片，源码净 = 0448 绿态复证）**：**fli 双重身份 = 内存饕餮 + 组合敏感 typed 门**。
- 矛盾消解：fli 编译本身内存饕餮——512MiB 下 fli 编译中途 arena 耗尽（trap 死于 fli 内）；2GiB 下活到其 unmodelable let（typed）。两观察一致，无悖论。
- 5 实参 user-call 建模悖论：error_arm 同款 word_span_offset 5 参在 general_while 路线绿、fli 同路线拒 → 非 arity 形问题；嫌疑 = 前驱状态致 gw_rhs_packed 臂守卫误射（0391 型第三实例 + 内存放大器复合体）。
- 前驱毒化合成探针 = 前驱 + fli 闭包；前驱亦调后定义 callee → CALL-TARGET。0454 修正 = 前驱 + fli 闭包 + 全部后随函数（≈ 全源重排），或 2GiB 全源 let 级二分。
- census 138/145、0 DIFF（同一性）；电池继承全绿。备份 target/tmp/parser_backup_0453.sico。
- **战略备忘**：fli 猎杀已 3 片（0451-0453）未破；若 0454 再不破，建议转 arena RFC 第三次修订（2GiB 正式化 + 守卫重钉），把全源 typed 前线批量暴露后再统一猎杀（内存墙后的门可能成片且与组合态相关，逐片猎杀效率低）。

**STEP-0452（2026-10-18，诊断片，全净回退 = 0448 绿态复证）**：**fli 门 = 变量透传 EXPRESSION**。
- 省内存前置截断法失败：fli 有 file-order 后定义的 callee → 前缀 CALL-TARGET（file 序 ≠ callee-safe）。
- 2GiB + GWPACK-OTHER 绑定插桩复跑 = **裸 EXPRESSION 无后缀** → 发射点 = 变量透传 return（`return <pack>`；字面值插桩不可达）——fli 的拒模 let-RHS 经子 packer 透传。
- fli let-call 清单登记：word_span_offset 5 实参 ×3 / next_index 六连 / ARITY 类存在（746-1538 行）但非当前门。
- **保留态决策**：2GiB 不可留存（组件最低内存抬升 → 低内存固件批量实例化失败）→ 全回退复证绿态（cmp 验证）。
- census 138/145、0 DIFF（同一性）；电池继承全绿。备份 target/tmp/parser_backup_0452.sico。
- 0453 路线：2GiB 实验态下 **topo 序前缀截断**定位 fli 具体 let（extract_closure 序安全）→ 子 packer 臂扩展（word_span_offset 5 参形或实际形）；arena RFC 第三次修订并行候选。

**STEP-0451（2026-10-18，实验/诊断片，全净回退 = 0448 验证态）**：**arena 上限实验 = 512MiB 纯内存墙实锤 + 全源 typed 前线 = fli**。
- 实验：客内 arena 2GiB（ARENA_LIMIT 2048MiB / MEMORY_PAGES 33792）→ 全源 walk **越过 trap、推进至 `EXPRESSION@fixed_literal_const_instruction`**；512MiB 下 4GiB host 内存无效 = 客内组件上限编译固化（host memory_bytes 不可超越）。
- 结论：全源实需 ∈ (512MiB, 2GiB]；treadmill（单调增长）关切仍有效，但 S6 输入有界 → arena 提升可解当下，复用留后续加固。
- 实验全回退（canonical.rs 复原 / scratch_arena_probe 删除 / parser 净态 cmp 验证）——arena 正式提升 = RFC-0029 第三次修订片（预算表全同步 + 体积阶梯实验定最小值 + 固件钉板）。
- census 138/145、0 DIFF（同一性）；电池继承全绿。备份 target/tmp/parser_backup_0450.sico（= 当前净态）。
- 0452 二选一：① fli 门手术（typed 前线续推；general_while let-RHS 形 = gw_rhs_packed 臂不全）；② arena RFC 第三次修订（512→N 全同步，N 由阶梯实验定）。推荐先做①（typed 前线每推进一步，全源在最终 arena 下的编译就更近）。

**STEP-0450（2026-10-18，诊断片，源码净 = 0448/0449 终态）**：**control_while 组合特异失败定位 = gw_rhs_multi_pack（k=70）**。
- topo 序前缀截断二分（callee-safe）：k≤69 = CALL-TARGET（截断断 callee 的预期伪影），k=70 加入 multi_pack → 首现 EXPRESSION。
- multi_pack 单探 census 绿、组合 EXPRESSION = **0391 型跨函数走查状态类第三实例**（scalar_ir 的 functions_json/source_cursor/gnext 累积或机器内状态跨函数未复位）。
- **战略评估**：组合特异门单点猎杀成本递增（每片 1-2 函数）；M22 GO 的硬闸门 = S6 全源降低（现抵 arena trap）——**arena 复用轨（treadmill 正解）的边际收益高于逐门猎杀**（一次修复解锁 NON-DOMAIN 3 户 + 全源推进 + 可能瓦解多个组合门）。建议 0451 起切换 arena 轨：guest 主循环按函数复用 arena（RFC-0029 amendment 正解；runner/host 侧改动 + 固件钉板更新）。
- census 138/145、0 DIFF（同一性）；电池继承全绿。备份 target/tmp/parser_backup_0450.sico。
- 备选：若 arena 轨受阻，组合门续猎（multi_pack EXPRESSION = 机器内 let-RHS 形在特定前驱状态下的变体）。

**STEP-0449（2026-10-18，诊断片，源码净 = 0448 终态）**：**插桩反噬实锤 + control_if 语法补全**。
- **普查实证**：31/31 if-only 函数 census 全绿（含 16-if 的 fixed_literal_argument、9-if 的 is_punct_byte）——control_if 真实语法远宽于 K12 单点印象：guard-if 经 **DECLINE 路径**（control_if 返回精确 "ERR:E-SH-IR-EXPRESSION" → dispatch 判 SKIP:CONTROL-IF-DECLINED → general_while 重试）编译。K12 拒的只是 ret-形特例。
- **插桩反噬定律入册**：凡参与 `is_word` 精确匹配的错误串（decline 检查、SKIP 判定等）**禁加后缀**——本片 K-tag 致 decline 失配、伪故障 is_declaration_start（其真实 control_if 直建模条件 = I64/U64 起首；is_word 条件走 decline 后由 general_while 编译 ✓）。
- **control_while 闭包特异失败在册**：成员函数单探全绿、组合走查 EXPRESSION（净态复证）；general_while 的 ERR 为变量透传（`return gif_pack`）不可安全插桩 → 0450 用组合函数二分（topo 序前缀截断安全）定位真凶函数。
- census 138/145、0 DIFF（同一性）；电池继承 0448 全绿。备份 target/tmp/parser_backup_0449.sico。
- 0450 路线：control_while 组合二分 → 真凶函数 → 门形手术；GWCOND/sef + RETURN-COUNT 2 户随后；arena 轨（NON-DOMAIN 3）候 treadmill 正解。

**STEP-0448（2026-10-18）**：**ret-if-else 门修复 = trivial-match 重路由法定型——走查推进两函数（is_ident_byte → type_json → fixed_literal_const_instruction）**。
- Dispatch 实锤：if-only 体（if>0 且 match=0）被 control_if **无条件认领**；其 ret 形契约 = 双 return 臂 + else（收尾 epilogue 查 sif_ret1_done + sif_ret2_done + has_else）；单臂 guard-if 不可表达；两臂梯的 else 臂嵌套 if 亦不受理（STATEMENT 实证）。
- 修法定型 = **恒真 match 包裹**（`probe=U64.literal(0); match U64.checked_add(probe, 0): case ok(_): 原体 / case error(_): 缺省`）→ if>0 且 match>0 → general_while（guard-if 本建模；error_arm 先例行）。语义透明、census 字节对拍担保。
- 已落地：is_ident_byte（7 守卫）、type_json（guard+let 尾）。走查推进至 fli = 0449 主刀（fli 的 let-RHS 形被 control_if set/let 拒）。
- census 138/145、0 DIFF 维持；全源仍抵 arena trap（钉兼容）；红户 7 = EXPRESSION 1（fli 深层）/ GWCOND 1 / NON-DOMAIN 3 / RETURN-COUNT 2。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0448。备份 target/tmp/parser_backup_0448.sico。
- 0449 路线：fli let-RHS 形（control_if set/let 位点 K 编号 + 绑定名 dump）；同类 if-only 函数（valid_escape_byte/is_punct_byte/decimal_digit 等）按包裹法批量候选；RETURN-COUNT 2 户随后。

**STEP-0447（2026-10-18，诊断片，源码净 = 0446 终态）**：**control_while EXPRESSION 门定位 = control_if ret-if-else 形（K12）**。
- 插桩链：全库 EXPRESSION @fn（119 站）→ `@control_if_function_ir` → 12 站点 K 编号 → **K12 = control_if 的 `end if` 处理：ret 形 if 带 else（`if sif_form == "ret"` 且 `sif_has_else`）→ EXPRESSION**——control_if 机器不建模"return 臂 + else"组合（0399 门族 control_if 容量实例）。
- 0448 修法二选一：① guest 侧 control_if 扩展（ret-if-else 建模 = 臂 return 值经 phi/序列化；机器改动风险中）；② 输入侧 else-drop fall-through 模板（0404；**碰撞风险须先验证 = 目标 if 的搜索游标上下文**〔0441 教训〕；优先选无 end-while/end-if 前驱的站点）。
- census 138/145、0 DIFF 维持（同一性）；电池继承 0446 全绿。备份 target/tmp/parser_backup_0447.sico。
- 0448 后队列：RETURN-COUNT 2（gw_match_open_pack/gw_slice_subject；0390/0392 单返回模板）+ sef GWCOND + NON-DOMAIN 3（arena 轨）。

**STEP-0446（2026-10-18）**：**CALL-SHAPE 6 户门面瓦解——census 138/145，全源回抵 arena trap**。
- S5 编号 + receiver-token 转储定位首败输入 = `find_keyword_skip_end` **自身**的 `next_index(fks_pos, sico.bytes.length(src))` 嵌套实参（wcr 段推进宽度失配）。
- 修复：实参 hoist 至 cell（`fks_next_pos`）；**自宿主嵌套实参纪律升级 = 全库准则**（新增 guest 代码一律 cell 形实参）。
- 门面瓦解实证：control_if **IDENTICAL 翻绿**；find_keyword_skip_end / general_while / scalar_ir 全绿；control_while 门换挡 EXPRESSION、gw_match_open_pack 换挡 RETURN-COUNT（更深 typed 门）。
- census **138/145（+2）**、0 DIFF；红户 7 = EXPRESSION 1（control_while）/ GWCOND 1（sef）/ NON-DOMAIN 3 / RETURN-COUNT 2。全源守卫第九钉（CALL-SHAPE→arena trap 回归 = typed 前线二次贯通）。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0446。备份 target/tmp/parser_backup_0446.sico。
- 0447 路线：control_while EXPRESSION 门（插桩绑定名）+ gw_match_open_pack/gw_slice_subject RETURN-COUNT（0390/0392 单返回模板）+ sef GWCOND；NON-DOMAIN 3 户待 arena 轨。

**STEP-0445（2026-10-18）**：**wcr V3 门修复 = 委托守卫（门换挡 ARGUMENT→SHAPE）**。
- 根因：0444 委托成功后**直线落入用户调用解析**（callee-found 与 delegation-done 两路径共享后续段，缺 else/守卫）；首败 = V3（scl_uc_open 检查 = 落穿症状）。
- 修复：`scl_uc_done` 旗标 + 用户调用解析整块 `if scl_uc_done == 0` 守卫（委托成功设 1 跳过）。
- 手术纪律：嵌套 end-match 定位两次错置（MissingBlockClose×2）+ cell redeclared（重复插入）→ 三触礁整文件回退 backup_0445 → **锚定宽度 match 开口行深度法**（opens+closes 双侧计数、depth 归零为界）一次成型。教训入册：结构化插入必须以目标 match 开口行锚定 + 双侧深度计数。
- 全源守卫第八钉（CALL-ARGUMENT→CALL-SHAPE）。census 136/145、0 DIFF 维持；红户 8 = CALL-SHAPE 6（wcr 段形检查新门 = wcr 对其输入 segment 的 open/tail/逗号检查，下一片 token-dump 定位具体 segment 形）/ GWCOND 1 / NON-DOMAIN 1 / RETURN-COUNT 1。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0445。备份 target/tmp/parser_backup_0445.sico（= 0444 净态）。
- 0446 路线：wcr CALL-SHAPE 门 token-dump（其 5 个 SHAPE 位点编号 + segment 首词转储）；预期同 V 系列逐位推进。

**STEP-0444（2026-10-18）**：**wcr W2 门开瓶 = sico.* 内建实参委托 gw_rhs_packed**。
- 插桩链：W2 位点 atom 转储 = `sico` → 路径转储 = `sico.bytes.*` 内建调用实参（while-call RHS 的 segment 形）。
- 修复：W2 用户调用分支 fallback——`sico` atom 且 function_declaration_index 查无 → 委托 `gw_rhs_packed` 打包整个 segment（4 行 pack 解包 = instr/kind/value/advance；span = segment 首末词字节界；新变量 scl_ud_* 前缀避让既有 scl_uc_pack〔= 用户定义 callee 的递归调用点〕）。
- 固件伴生：wcr 闭包钉测试函数清单扩 33 函数（wcr 新增 callee gw_rhs_packed + 其独占闭包全量入列；期望 IR 仍同源推导，钉语义不变）。
- census 136/145、0 DIFF 维持（6 户门面未动 = 首败位点仍在 W3+）。残余在册：wcr 的 W3（用户调用 open 检查）/W4/W5（其余操作数槽位）逐位形开瓶 = 0445；GWCOND/sef + RETURN-COUNT（gw_slice_subject）随后。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0444。备份 target/tmp/parser_backup_0444.sico（= 0443 净态）。

**STEP-0443（2026-10-18）**：**census 回绿 136/145、0 DIFF（scc 翻绿）——read_local 语义 + helper 窗口双修复**。
- 0442 残余根因双击：①helper 窗口 off-by-one（"end while" 中 while 的前三字节是 "nd "〔空格在 end 与 while 之间〕→ 查 pos-4 "end " 四字节含空格；0442 版 helper 实为惰性）；②**cell 实参 read_local 语义**（oracle 对 local 实参先发射 read_local 再用其 SSA 值；append/less_than 两 packer cell 分支补发射 + kind 取 cell_kinds 实值——list/u64 append 的 read ty 实锤此要求）。修复链击 diff 三级逐级对拍清零（end-while 碰撞 → read_local 缺失 → read kind 错）。
- 全库 ARGUMENT 插桩（66 站 @fn）定位 control_if 新门 = **`while_call_rhs_packed`**（0444 主刀面）；local-bounds 双 shave（general_while gdecl_colon + control_while sws_rhs_multi_next 内联，后者顺带还原条件直形——gw_condition 本建模）。
- census **136/145、0 DIFF**；红户 8 = CALL-ARGUMENT 6（while_call_rhs 门面）/ GWCOND 1（sef）/ NON-DOMAIN 1 / RETURN-COUNT 1（gw_slice_subject）。全源守卫第七钉（arena trap → CALL-ARGUMENT 回归 typed = 全源 typed 前线推进到 while_call_rhs）。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0443b。备份 target/tmp/parser_backup_0443.sico（= 0442 净态）+ 当前树 = 备份 + 0443 全部改动。
- 0444 路线：while_call_rhs_packed ARGUMENT 门逐户开瓶（其内部 operand 形解析；插桩 token-dump 法直接套用）；GWCOND/sef + RETURN-COUNT 随后；arena 轨仍候 treadmill 正解。

**STEP-0442（2026-10-18，如实登记 census diffs=1 红态）**：**less_than 值 packer + 碰撞 #2 helper——GWPACK-OTHER 族灭**。
- `gw_u64_less_than_value_packed`（oracle 实锤 less_than 值形 = `less_fixed`/bool；`fixed_operation_instruction_json` 现成发射器；param/cell/literal 三形参）+ gw_rhs_packed 调度臂；三 sites 实参先 hoist 至 cell（嵌套 next_index 实参 packer 不收）。
- **`find_keyword_skip_end` helper**（命中后查前三字节 == "end"，非则 +1 重搜）+ 6 站（general_while 的 while/if、control_if 的 if、control_while 的 while/if、match_open 的 match）——碰撞 #2（end-X vs X）首个通用正解；control_if/while 实证推进（走进 packer 层）。
- 战果：census **135/145（+1 翻绿；两新 packer 自身闭包全字节等同 = 分母 +2 绿）**；GWPACK-OTHER 3→0 族灭；红户重组 = CALL-ARGUMENT 6（packer 参数解析新门：走查推进后 packer 被更多语句命中、非原子实参暴露）/ GWCOND 1 / NON-DOMAIN 1 / RETURN-COUNT 1。
- **残余 = straight_cell 单户 DIFF**（外层 while block start 偏 -8 = 碰撞 #2，但**非已铺 6 站** = 其路径另有未辨识 while-find；已排疑：全库 `find_text_from(src,"while"` 仅 2 处且已铺；候选 = 变体 needle 搜索或 block-range 链上游）。下片 empirical 定位法 = 对 helper 加一次性转储（命中时 concat 返回偏移+needle）确认 scc 路径是否经 helper。
- 备份 target/tmp/parser_backup_0441.sico（= 0440c 净态）；当前树备份未及（0442 状态 = parser_backup_0441 + 本片全部改动；下片开工先补 cp）。
- fmt 过；0262/0245/canary/0124 待 scc 残余修复后跑（现 census 红，电池预期红）。
- 下片（0443）：scc 残余定位修复 → census 回绿 → 电池 → CALL-ARGUMENT 6 户逐户（插桩法 = packer 返回带 atom 名的 ARGUMENT 错误）。

**STEP-0441（2026-10-18，诊断片，源码净 = 0440c 终态）**：**GWPACK-OTHER 3 户开瓶 + 条件化路线正式作废**。
- 插桩指认：straight_cell `scc_return_multi` + control_if/control_while 共享 `swr_multi` ×2 = u64.less_than 值 RHS 无 packer（0435 同族，当时三处修复属侥幸无碰撞上下文）。
- **条件化修法撞结构性碰撞 #2**：source 任何重排移动搜索游标链 → `find_text_from(src,"while",cursor)` 命中前驱 `end while` 的 while 子串 → while-block range.start 偏 -8 → DIFF 三户全中（已回退净态）。碰撞类 #2（end-while vs while）登记潜伏清单——同理 end if/end match 前缀隐患。
- **正解换挡**：guest 侧 u64.less_than 值 RHS packer（镜像 0439 append packer：cmp 指令发射 + 4 行 pack，kind "u64"；source 零重排 → 无游标链扰动）= 0442 主刀；预期一次解开 3 户（+ 0435 三处的隐性同款债可一并迁回真值形）。
- census 134/143、0 DIFF 维持（同一性）；电池继承 0440 全绿。备份 target/tmp/parser_backup_0441.sico。
- 0442 后队列：GWCOND-EXPRESSION 1（sef）+ RETURN-COUNT 2 + NON-DOMAIN 3（多属 arena 衍生）→ 然后 arena 复用轨（treadmill 正解）。

**STEP-0440（2026-10-18）**：**census 回绿 134/143、0 DIFF——typed 前线全贯通，主墙换挡 arena treadmill**。
- let_count 碰撞修复三案两败一胜：①尾空格变体（23 站）→ RANGE 48 崩（非空格跟随者系统性落空）回退；②core `find_text_from` 边界检查 → general_while fuel 循环（碰撞匹配在部分走查路径承重 = 行为耦合实锤）回退；③**外科重试法胜**：sef 单点 let 偏移重验证循环（命中后查 +3 字节 == 空格，非则 +1 重搜）。
- 战果：census **134/143、0 DIFF**；sef 门推进至 GWCOND-EXPRESSION（更深 typed 门）；**全源走查抵 arena trap（UnreachableCodeReached）**——typed 拒绝全清，0365 族地位复原。
- 全源守卫第六钉（CALL-TYPE → arena trap 回归钉，含 0365 注释）。红户 9 重组 = GWCOND-EXPRESSION 1 / GWPACK-OTHER 3 / NON-DOMAIN 3 / RETURN-COUNT 2。
- **战略换挡登记**：M22 下一主墙 = **arena treadmill**（512MiB 单调 per-run arena，最深 walker 耗尽触发 realloc 裸 unreachable）——正解 = 自宿主循环按函数复用 arena（RFC-0029 amendment 已登记，独立轨道；推进该线 = guest 主循环结构改动，非 parser.sico 手术）。
- 潜伏碰撞清单（其余 `find_text_from` 关键字站点：cle/nir/casm/lgm/swh/sww 的 let/if/while/return）登记在案——现被更深门掩盖，浮现时按外科重试法模板处理。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0440c（target/tmp/）。备份 target/tmp/parser_backup_0440.sico。
- 下片二选一：①arena 复用轨（guest 主循环按函数复用 arena——大改动，RFC 层级）；②余 9 红户 typed 门继续（GWCOND-EXPRESSION/GWPACK-OTHER/RETURN-COUNT 逐户，预期 NON-DOMAIN 3 户在 arena 修复前先转 typed 或持续 trap）。

**STEP-0439（2026-10-18，如实登记 census diffs=1 红态）**：**U64 append 专属 packer 落地 + typed 门全清实证 + 潜伏缺陷曝光**。
- packer：`gw_list_append_u64_rhs_packed`（80 行，镜像 gla/gic；参数/槽位引用循 while_bytes_at 字节先例；字面量第二参先发射 const）；调度臂 off-by-one 修复（gla_name_idx = receiver+5）。
- **typed 门全清实证**：scalar_ir 闭包与全源走查均越过全部 typed 拒绝，直抵 **arena trap（UnreachableCodeReached = treadmill 类）**——6 户 CALL-TYPE 门链瓦解。
- **新潜伏缺陷（census 红根因）**：走查器语句关键字 = `find_text_from` 纯文本搜索，与标识子串碰撞——straight_environment 形参 literally 名 `let_count`，首语句 let 的搜索命中参数名（write_local range.start 偏 -32，guest 121375 vs oracle 121407）。此前被 CALL-TYPE 门掩盖。**修复路线** = 词感知锚定（word_span_offset 从 fn_start 词锚）或 "let " 尾空格匹配；同类隐患 = 一切 `*_count` 形参名 vs let/set/if/match/while 关键字（scalar_ir 有全部五个，现被 trap 掩盖幸未 DIFF）。
- packer 本体无辜：其自身闭包字节等同（census 分母 +1 绿）。**census 134/143、diffs=1（红，下片修）**。fmt 过；0262 待潜伏缺陷修复后重钉（全源现已抵 arena trap）。
- 基线 census-latest = census-0439b（target/tmp/）。备份 target/tmp/parser_backup_0439.sico。
- 下片（0440）：潜伏碰撞修复（词感知锚定为正解；回归面 = scalar_ir 等五 count 形参函数的预期翻绿/新 DIFF 暴露）→ census 回绿 → 0262 重钉 → 电池。

**STEP-0438（2026-10-17）**：**泛型 append 修复面攻坚**。
- host 实锤：无注解 `sico.list.append` **仅推断 Text 元素**——U64 单元格批量 E2001 回弹，迭代修复器 `fix_iter_0438.py`（build-fix 循环）自动重泛型化 ~60 站；Text 单元格安全去泛型化。
- guest 前端泛型容忍落地：gla 开括号计算跳 `[T]` 括号对（set 修正 gla_open）+ gw_rhs_packed 调度臂泛型形内建名选择（`sico.list.append[U64]` 保 oracle 字节一致）。
- 门推进 CALL-SHAPE→**CALL-TYPE**（gla list-kind 检查拒 U64 元素 append）——**U64 元素 append 专属 packer = 下片正解**（gla 只建模 text-binary 形）。
- local-bounds 地板事故：12 处嵌套摊平中 4 处误入 general_while → busiest 254 > 248 红线 → 按 0343 回退该 4 处（其自身编译容忍嵌套形，census 无损失）。
- 全源守卫第五次重钉 CALL-TYPE；census 133/142、0 DIFF 维持（CALL-SHAPE 6→CALL-TYPE 6 = 门细化）。
- **环境事件**：Windows 清理 C:\Users\...\Temp 全盘（/tmp 基建再失）——**target/tmp 确认为持久基座**（全部探针/脚本/历史 census 幸存）。纪律更新：一切基建落 target/tmp，禁 /tmp。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0438c（target/tmp/）。
- 下片（0439）：U64 元素 append 专属 packer（gw_list_append_u64_rhs_packed 模式，镜像 gla 但 kind=U64 直值发射；或 gla 参数化 kind 检查）+ RETURN-COUNT 2 户 + general_while 内 3 终结子站点抽辅助。

**STEP-0437（2026-10-14，事故回退 + 诊断双收片）**：
- **诊断双收（token-dump 插桩两连中）**：①输入 `sico.list.append[U64](...)` 泛型形被 gw_rhs_packed 的 `is_intrinsic_path(list,append)` 臂误路由至 gla（gla1 open 检查拒——`[U64]` 使 gla_open 落在 `[`）；②输入 `sico.u64.to_text(u64_list_at(...))` 嵌套实参形经 gic3（单实参检查）拒——**15 处 locals_json 构建站点全部登记**（12 处机器函数内 + 3 处 general_while 终结子构建器）。
- **事故与回退**：手术两触礁——general_while local 头寸 256/256 顶板（+4 let 即破 MAX_LOCALS_PER_FUNCTION）→ 改内联回收 → 脚本三元优先级 bug（`('%s' % x if c else '%s' % y)` 缺右括号）6 行受损 + 误修 2 无关行 = 三败 → 整文件回退 backup_0435（实 = 0434 终态）+ 重放 0435 三条件化 + 0436 gic_open 摊平 = **精确复原 census-0436 态**（133/142、HIST 6/1/2 一致）。
- **教训双登记**：①**开片备份禁跳**（0436/0437 连跳两次引发回退链；备份一律 `cp` 到 /tmp/probe/ 双份）；②**local 顶板函数手术必须预查头寸**（general_while 全库唯一 256/256；0343"抽辅助不长个"纪律强制）。
- 电池全绿（0262 免重钉）；基线 census-latest = census-0437b。备份 /tmp/probe/parser_backup_0437.sico（= census-0436 态）。
- 下片（0438）路线已定：两输入形手术——12 处非顶板站点直接摊平（fix_lu 模式）；general_while 内 3 站点走 0343 抽辅助（终结子构建器独立成 helper，不长个）；`sico.list.append[U64]` 泛型形 = 独立修复面（或改输入形为 `sico.list.append` 非泛型〔需 host 类型推断支持〕，或 gla/gic 加泛型容忍——前者优先，属 guest 前端 packer 增强，需 RFC 层级判断）。

**STEP-0436（2026-10-14，诊断片 + 1 处清债）**：**CALL-SHAPE 6 户共门追击**。
- 方法论再升级：**全局函数名插桩**（208 个 CALL-SHAPE 站点批量加 `@fn` 后缀，instr_cs_0436.py 脚本化可逆）——一击指认 6 户共门发射器 = `gw_text_list_binary_packed`；二次精插桩（gla 5 站点区分后缀）= gla1（open 检查）；前缀截断（topo 序安全）定失败输入函数 = gw_rhs_multi_pack；复核时发射器前移到 `@gw_intrinsic_call_packed`→gic3（单实参检查拒）——**门接力随每次读数前移，当前断点 = 经 gic 路由的某输入调用实参数 ≠1**（multi_pack 体内 length/split_lines 全单参合规，疑点在路由层 receiver 形再认，0437 续）。
- 清债：gic_open 5 重 next_index 链摊平（0436 实修 1 处，0395 遗漏幸存者 +1）。
- census 133/142 维持、0 DIFF；HIST = CALL-SHAPE 6 / NON-DOMAIN 1 / RETURN-COUNT 2；全源前沿与钉一致（免重钉）。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0436。备份 /tmp/parser_backup_0436.sico（= 0435 终态）。
- 工具：instr_cs_0436.py（on/off）入 /tmp/probe；dd_gmo2/3、dd_grp、swap_grp、prefix-truncation 脚本全部入 /tmp/probe。
- 下片（0437）：gic3 断点细查（输入 receiver 形 vs gic 路由条件的错位；候选 = multi_pack 的 `sico.text.split_lines` 误路由或嵌套实参形）+ RETURN-COUNT 2 户（0390/0392 机器单返回模板）。

**STEP-0435（2026-10-14，跨日续片）**：**less_than 值 RHS 门族全清**。
- 插桩法批量开瓶（修正 grep 交替序陷阱：`E-SH-IR-[A-Z-]+` 在 leftmost 位置抢走 `GWPACK-OTHER:name` 的匹配）：control_if/control_while 共享 `sws_rhs_multi`、straight_cell 自有 `scc_rhs_multi`——与 0434 `gws_multi` **同族三门同源**（`let x = U64.less_than(next_index(..), ..)` 值形无 packer），条件化同模修复（if/while 计数不变）。
- **GWPACK-OTHER HIST 清零（3→0）**；三户门推进 GWPACK-OTHER→CALL-SHAPE（更深未绿）；census 133/142 维持、0 DIFF。
- 红户 9 重组 = **CALL-SHAPE 6**（control_if/control_while/straight_cell/straight_environment/fixed_guard_chain/scalar_ir）/ **RETURN-COUNT 2**（gw_match_open_pack/gw_slice_subject_packed）/ NON-DOMAIN 1。
- 全源前沿 = CALL-SHAPE 与钉一致（0262 免重钉直接过）；电池全绿（fmt/0262 81/81/0245/canary 30/30/0124/diff-check）。基线 census-latest = census-0435。备份 /tmp/parser_backup_0435.sico。
- 下片 = CALL-SHAPE 6 户逐户（插桩法套 CALL-SHAPE 发射点；候选首发 = fixed_guard_chain_ir〔最小〕或 scalar_ir〔解锁面最大〕）+ RETURN-COUNT 2（0390/0392 老族回归视野，机器单返回模板）+ NON-DOMAIN 1 重定。

**STEP-0434（2026-10-13）**：**插桩定位法 + gw_rhs_packed 翻绿 + census 燃料校正**。
- 方法论：形状启发式三击不中（嵌套深度扫描漏真凶）后启用**临时插桩**——guest let/set 处理器的 GWPACK-OTHER 分支 concat 绑定名（改 guest 源码 → 重建 → 探针读绑定名 → 撤插桩）。一击给中 `gws_multi`。**此法为后续所有 GWPACK-OTHER/CALL-SHAPE 户的标准开瓶器，收益永久。**
- 门根：`let gws_multi = U64.less_than(cell, param)` 作**值 RHS**——gw_rhs_packed 机器不建模 u64.less_than 值形（全库无第二例、无 less_than packer）。修法 = 条件化（`if U64.less_than(...)` 直走已建模的 gw_condition_packed；if 计数不变、路由不变）。
- 卫星：gw_match_open_pack 4 处嵌套摊平（2 重 next_index 链 ×2 + gw_decimal_value(word_at) ×2）+ u64.to_text dispatch 5 重 next_index 链（0395 遗漏幸存者）。
- **census 124→133/142、0 DIFF（+9 翻绿）**——关键 = **census 探针燃料 20B→400B**（0413 事故重建时取自 wire log 旧版；FuelExhausted 面具下 9 户已绿）。红户 21→**9**：CALL-SHAPE 3 / GWPACK-OTHER 3 / NON-DOMAIN 1 / RETURN-COUNT 2。
- 固件：全源守卫第四次重钉（GWPACK-OTHER→CALL-SHAPE）+ 守卫燃料 120B→400B；0262 恢复 81/81。
- 电池全绿：fmt/0262/0245/canary(30/30)/0124/diff-check。基线 census-latest = census-0434b（/tmp/probe/）。备份 /tmp/parser_backup_0434.sico。
- 下片（红户 9）：gw_match_open_pack（RETURN-COUNT→更深）+ straight_environment CALL-SHAPE + control_if/control_while/straight_cell GWPACK-OTHER ×3 + NON-DOMAIN 1 + RETURN-COUNT 2。插桩法直接套用。

**STEP-0433（2026-10-13）**：**CALL-SHAPE 族灭片**。
- 门根 = 实参位嵌套调用（0383 已知族新实例， topo 重排后显形）：`sico.u64.to_text(U64.literal(0))`（内建套内建）×2、`sico.u64.to_text(next_index(..))` ×2，分布于 gwf_l_operand / gwf_r_operand / gw_rhs_split_lines_emit / gw_match_open_pack——全部提升 cell（let 原子形 + set 引用），共 7 站点。
- 战果：**census 121→124/142、0 DIFF（+3 翻绿）**；HIST = CALL-SHAPE 15→**1**（族灭）/ GWPACK-OTHER 3 / NON-DOMAIN 3→14（12 户走进更深 walker 撞 arena——treadmill 类，RFC-0029 amendment 正解 = 自宿主循环按函数复用 arena，另行跟踪）。
- gw_match_open_pack 门推进 CALL-SHAPE→GWPACK-OTHER（下片主刀）；残余 typed 面 = CALL-SHAPE 1 + GWPACK-OTHER 3。
- 全源前沿守卫三钉连续剧：0365 trap → CALL-SHAPE（0432）→ GWPACK-OTHER（0433）——每次推进即重钉，守卫语义不变。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线快照 census-latest = census-0433（/tmp/probe/）。备份 /tmp/parser_backup_0433.sico。
- 下片 = gw_match_open_pack GWPACK-OTHER 手术（最大单户，解锁面未知）+ 余 CALL-SHAPE 1 户 + GWPACK-OTHER 3 户（control_if/control_while/straight_cell）；NON-DOMAIN 14 户待 treadmill 正解（独立轨道）。

**STEP-0432（2026-10-13）**：**parser.sico 全量拓扑重排**。
- SCC 收缩 + 稳定 Kahn（callees-first，独立函数保持原相对序）；首版边向反转（1123 前向对）立即整文件回退修复——拓扑方向纪律：边 = 调用者→被调，Kahn 源 = 无入边 = 顶层调用者，故输出需 reverse。
- 前向对 23→**4**：余 4 对全部 = gw_rhs_packed 互递归簇环内边（gw_text_concat→gw_text_list_binary / gw_map_put→gw_rhs / gw_text_list_binary→gw_rhs / gw_rhs→multi_pack），排序不可消除；**环边前向调用的门形 = CALL-SHAPE**（比 RHS 位置前向的 STATEMENT 宽容）。
- census **121/142、0 DIFF**；HIST = CALL-SHAPE 15 / GWPACK-OTHER 3 / NON-DOMAIN 3（trap 户 14→3——前沿近全 typed 化，可手术性大增；下片主刀面 = 15 户 CALL-SHAPE 逐族消解）。
- 固件钉板重钉两处（0262 红 → 绿）：①is_ident_byte 前缀锚 `lex_words` → `record_named_type`（拓扑序 lex_words 沉底 141/142；测试意图不变，期望 IR 仍同源推导字节精确）②全源前沿守卫 0365 trap 钉 → CALL-SHAPE typed 钉（越过 0365 族 = 预期推进；0365 族缺陷仍登记未修）。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线快照 census-latest = census-0432（/tmp/probe/）。备份 /tmp/parser_backup_0432.sico = 0431 终态。
- 下片 = CALL-SHAPE 15 户逐族（gw_rhs 簇环边 4 对优先：环消除候选 = 把 helper 回调 dispatcher 的调用点改经 dispatcher 形参注入或提前化）+ GWPACK-OTHER 3 户 + 余 trap 3 户（treadmill 正解另行跟踪）。

**STEP-0431（2026-10-13）**：**前向引用门实锤 + 重排手术**。
- 机制：guest 前端要求被调函数先于调用者声明；RHS 用户函数调用前向 → STATEMENT（A/B 受控实证：while_bytes_at_rhs_packed 唯一前向 callee intrinsic_instruction_json 位置 swap 翻转门）。
- extract_closure.py 双 bug 修复：①`sorted(seen)` 字母序 emit = 0429/0430 大量"组合/路由敏感门"探针的**伪门根源** ②`e-1` 截断丢 `end function`（无空行邻接函数受害；重排手术意外触发暴露）。**教训：本线历史上一切 closure 探针结论需按新提取器复拍才算数**。
- 手术（纯移动，无语义变化）：is_intrinsic_path + intrinsic_instruction_json（22+17 前向调用方）移至 earliest-caller（5814）前；gw_find_code_text（6 户）移至 2409 前——全库前向对 **39→23**。
- 前沿族复拍（真实门）：while_bytes_at_rhs_packed / gw_rhs_at_dispatch / while_bytes_at_local_src **IDENTICAL 翻绿**；gw_rhs_multi_pack / gw_rhs_packed → GWPACK-OTHER（真 typed 门）；gw_rhs_split_lines_emit → CALL-SHAPE。
- census **121/142、0 DIFF**；HIST 换挡 GWPACK-OTHER 16→3 / NON-DOMAIN 1→14（走进更深 walker 撞 arena trap = RFC-0029 amendment treadmill 类，正解 = 自宿主循环按函数复用 arena，另行跟踪）。census-latest = census-0431（/tmp/probe/）。
- **事故**：target/tmp 自 13:12 起被外部进程反复整体清空——探针基建全失，已从 session wire log 逐文件恢复，工具链迁至 **/tmp/probe/**（extract_closure/closures/cmp_probe/run_dump/dump_oracle/run_census/scratch_census_probe），禁再依赖 target/tmp。
- cmp_probe 三修复：POSIX stdin + cygpath DUMP_SRC 分流；400B 燃料对齐 census；printf 字节对拍复原（sed 版破坏字节等长）。
- 电池全绿：fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check。基线快照 census-latest = census-0431。下片 = 余 23 前向对清零（预计又一批翻绿）+ multi_pack/rhs_packed 的 GWPACK-OTHER 真门 + split_lines_emit CALL-SHAPE。

**STEP-0430（2026-10-13，诊断/机制发现片，源码净 = 0428 验证态）**：
- **路由定律实锤**：scalar_ir 分发按语句计数——if_count>0 且 match_count>0（或 match_count>1）强转 general_while；否则按 while 计数分 general_while/control_while。0429 的"组合依赖门"正式更正为**路由敏感门**（语句增删改计数 → 换机器 → 换门；解释 0391/0420 全部历史之谜）。
- 函数级 ddmin（22 探针）：收敛 = 22 函数全集（find_text_from = extract_closure 死重）；每成员删除即 callee 断裂首错替代——门在 while_bytes_at_rhs_packed 本体走查槽。
- 语句级 ddmin：前 44 语句可删（门不在前半体）；尾区结果因截体改路由**不可靠**（尾部残块被 control_while 认领；STATEMENT 发射点全库 ~85 处跨 ~10 机器，错误串相同 ≠ 同门）。工具 bug 修复（trial[-1] 强制回加致无穷循环）+ resume 支持；ddmin/stmt_ddmin 入 target/tmp。
- **general_while 状态机结构读出**：region（header/body/after/join）+ gcur_term 块终止位；set/let 统一入口；gregion=header 或 gcur_term 时 set → STATEMENT。**下片主攻假设** = 无 while 体走查中块终止状态与后继 set/let 的交互（0422 control_if set 门未移动笔记同族）。
- 下片正解 = 静态走查 general_while 状态机 × rhs_packed 语句序列（按区域/终止位逐步模拟），或对 guest 前端加错误位置标注（发射点带函数名+行号，收益永久，需动 guest 源码）。**计数路由定律下一切"删语句找门"法先天不可靠，禁再用。**

**STEP-0429（2026-10-13，诊断负结果片）**：while_bytes_at_rhs_packed STATEMENT 门专项——then 臂 return 摊平 ×2 + 巨型 7 层嵌套 concat return 摊平**三门手术全不响应**；绿的 local_src 度量对照证门**非体量聚合、非 then 臂 return 形态、非嵌套表达式深度**（该假设正式撤销）；截体二分再证无效（stub/half/quarter 全先撞 CALL-ARGUMENT——模块内第二个潜伏门暴露）；成员单探 23/23 绿 + 顺序重排仍 STATEMENT = "组合依赖门"（已被 0430 更正为路由敏感门）。源码回退 = 0428 验证态（byte 级 = census-0428 树，121/142 与电池继承成立）；diff-check 净。

**STEP-0428（2026-10-13）**：gw_rhs_at_param_arm 抽取（at_dispatch param 臂 12 行 → helper；caller 空串信号 + 长度守卫 sibling-if 形）——census 120/141→**121/142**、0 DIFF（+1 翻绿 +1 分母）。门接力实锤：at_dispatch/at_param_arm 闭包残余 STATEMENT 全部继承自 **while_bytes_at_rhs_packed**（general_while 机器本体；while_bytes_at_local_src_packed IDENTICAL 对照）——gw_rhs 族本体现全部 guest-clean，下一道门在 while 机器族。fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check 全绿。基线快照 census-latest = census-0428。

**STEP-0427（2026-10-13）**：gw_rhs_error_arm 整臂抽取（error 臂 165 行 12 层级联 → helper；本体对拍 IDENTICAL；caller 空串信号 + 长度守卫；__h218 顶零初始化补 let）+ ok/error 双守卫 sibling-if 摊平——census 119/140→120/141、0 DIFF（+1 翻绿 +1 分母）。探针纪律补记：cmp_probe/run_dump 的 DUMP_SRC 必须绝对路径（cargo 测试 CWD 陷阱）。gw_rhs_packed 闭包残余门 = at_dispatch/multi_pack 本体（callee 继承，下片目标 = at_dispatch STATEMENT → multi_pack 内 gws_second/gws_lit/gwf 残余）。fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check 全绿。基线快照 census-latest = census-0427。

**STEP-0426（2026-10-13）**：gw_rhs_ok_arm helper 抽取（ok 臂；fall-through 语义 = `let gow_r` + 单 flat 长度守卫）——census 118/139→119/140、0 DIFF（+1 翻绿 +1 分母）。fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check 全绿。基线快照 census-latest = census-0426。下片 = gw_rhs_packed 余臂（error 臂/直返调度残块）→ multi_pack 内 gwsdf/gws_lit/gwf 残余 → GWPACK-OTHER 16 户逐户。

**STEP-0425（2026-10-13）**：gw_rhs_utf8_decode_arm 整臂抽取（级联 79 行 → helper，模板②式）——census 117/137→118/139、0 DIFF；级联 helper 顶层 5 深未触发门（0399 门判据 = 臂 return 非纯深度，再印证）。fmt/0262/0245/canary/0124/diff-check 全绿。

**STEP-0424（2026-10-13，补记）**：gwf_r_operand 操作数析取 helper（模板③右操作数，param 臂异名约束同 0423）——与 0425 合计 +1 绿 +2 分母、0 DIFF。fmt/0262/0245/canary/0124/diff-check 全绿。原片漏载 STATUS/handoff，0426 片补记。

**STEP-0423（2026-10-13）**：模板③首执行（gwf_l_operand 操作数析取，early-return 全 flat）——census 117/137、0 DIFF 维持；模板机械执行路径打通（gwf_r/gwsdf 同法续）。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0423](./steps/STEP-0423-m22-w2-gwf-l-operand.md)。

**STEP-0421（2026-10-13）**：RFC-0029 第二次修订（512MiB）——trap 类再瓦解，NON-DOMAIN 4→1，GWPACK-OTHER 16 成新主导族；treadmill 关切入 RFC（正解 = 自宿主循环按函数复用 arena）。census 117/136、0 DIFF。fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0421](./steps/STEP-0421-m22-w2-arena-512-rfc.md)。

**STEP-0416（2026-10-13）**：at_dispatch 腐蚀区整块重写（自给化 + 直返 + 链顶层化）——census 117/136 维持无回归；腐蚀区手术标准流程定型。下片 = encode/length 臂 + gwsdf 级联 + gwf 操作数对。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0416](./steps/STEP-0416-m22-w2-at-dispatch-rewrite.md)。

**STEP-0415（2026-10-13）**：at_dispatch 潜伏腐蚀实锤（语义碰巧正确/guest 形状拒）；清理行级删除两轮失败整文件回退 0414 验证态（教训 = 整块重写禁行删）。

**STEP-0414（2026-10-13）**：split_lines 直返化（tail 折入 emit + else-drop 链顶层化）——census 117/136、0 DIFF 维持。下片 = encode/length 臂同法 + fallback 级联。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0414](./steps/STEP-0414-m22-w2-sl-direct-return.md)。

**STEP-0413（2026-10-13）**：嵌套消解模板库定型（early-return/pack-helper/操作数析取/级联逐级的四模板；"条件分支内含 callee+错误守卫不可 flat 表达"关键认知）——源码净。

**STEP-0412（2026-10-13）**：split_lines 分支抽辅助（raw-pack 与四行 pack 同构验证）——census 117/136、0 DIFF 维持。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0412](./steps/STEP-0412-m22-w2-split-lines-extract.md)。

**STEP-0411（2026-10-13）**：嵌套 ERR-guard 手术定型（分支对抽辅助；提升纯调用语义变化实锤放弃）——census 116→117/135、0 DIFF。节奏 = 每片一处 guard ≈ +1 绿。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0411](./steps/STEP-0411-m22-w2-err-guard-extract.md)。

**STEP-0410（2026-10-13）**：gw_rhs 双臂抽辅助（315/65 行各成 helper）——**gw_rhs 本体翻绿，census 115→116/134、0 DIFF**。下片 = helper 内嵌套 ERR-guard 逐段处理 + GWPACK-OTHER 13 户 + 4 户 trap。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0410](./steps/STEP-0410-m22-w2-gw-rhs-extract.md)。

**STEP-0409（2026-10-13）**：侦察收敛——16 红户三桶化（9 STATEMENT〔gw_rhs 主门〕/ 3 GWPACK-OTHER / 4 trap 最深端）；census 燃料 120B。下片 = gw_rhs 巨臂抽辅助。

**STEP-0408（2026-10-13）**：gw_rhs 18 处 fall-through 展开（span 限定实证）——门根非 eligible ifs，乃 elif/guard 结构；下片逐段抽辅助。census 115/132 维持；fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0408](./steps/STEP-0408-m22-w2-gw-rhs-flatten.md)。

**STEP-0407（2026-10-13）**：gw_rhs 六处 to_text 提升——CALL-SHAPE 10→1，前沿换挡 GWPACK-OTHER 12（gw_rhs 本体 STATEMENT = 12 户公共门，下片单点手术）。census 115/132 维持；fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0407](./steps/STEP-0407-m22-w2-gw-rhs-hoist.md)。

**STEP-0406（2026-10-13）**：census 燃料 60B（7 户脱 FuelExhausted 面具）+ 6 站点 matching_close 嵌套链摊平——census 115/132 维持（门推进 partial 如实登记）。真前沿 = CALL-SHAPE 10 / GWPACK-OTHER 3 / NON-DOMAIN 4（17 户，0404 手工纪律逐户续做）。fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0406](./steps/STEP-0406-m22-w2-census-fuel-mc-flatten.md)。

**STEP-0405（2026-10-13）**：flatten_if.py 全库批跑灾难（census 115→13 雪崩）立即整文件回退净——工具标注半手工、禁无人值守批跑（负结果登记于 0404 文档 §6）。

**STEP-0404（2026-10-13）**：fall-through 展开 + elif 链抽辅助——gcc/gca 翻绿，RETURN-COUNT 清零，**census 111→115/132、0 DIFF**。残余 17 户（CALL-SHAPE 7/GWPACK-OTHER 3/NON-DOMAIN 7）逐户同法。fmt/0262(81/81)/0245/canary/0124/diff-check 全绿。详见 [STEP-0404](./steps/STEP-0404-m22-w2-fallthrough-flatten.md)。

**STEP-0403（2026-10-13）**：CALL-SHAPE 续清（to_text(next_index) 提升）——**gow 翻绿，census 110→111/130、0 DIFF**。下片 = gcc/gca/gw_rhs/starts_with 顶层化手术 + GWPACK-OTHER 3 + NON-DOMAIN 7。详见 [STEP-0403](./steps/STEP-0403-m22-w2-call-shape-sweep.md)。

**STEP-0402（2026-10-13）**：RFC-0029 预算表修订（guest 内存 128→256MiB）——0365 trap 类瓦解。**census 107→110/130、0 DIFF，casm 本体翻绿，CALL-SHAPE 12 成新主导族**。fmt/0262(81/81)/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0402](./steps/STEP-0402-m22-w2-arena-256-rfc.md)。下一刀 = CALL-SHAPE 12 户 + GWPACK-OTHER 3 + NON-DOMAIN 5。

**STEP-0401（2026-10-13）**：casm trap 根因实锤 = arena 128MiB 耗尽（0365 值损坏诊断更正）；下片 = RFC-0029 预算表正式修订（0351 模式）。详见 [STEP-0401](./steps/STEP-0401-m22-w2-arena-trap-root-cause.md)。

**STEP-0400（2026-10-13）**：wcr 巨大 if 顶层化——**RETURN-COUNT 19 户族灭，census 104→107/130、0 DIFF**；前沿收敛为单一类 = 0365 wasm trap（NON-DOMAIN 17）。selfhost_compiler 81/81；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0400](./steps/STEP-0400-m22-w2-wcr-flatten.md)。下一刀 = casm trap 专项（0365 正体）。

**STEP-0399（2026-10-13）**：深度阈值标定——函数顶层嵌套 if 深度 ≥2 = STATEMENT（while 内容量充足）；wcr/gow/scc 共同形态实锤；下片 = 块感知巨大-if 反守卫变换器。

**STEP-0398（2026-10-13）**：前沿收敛诊断——残余户（gow/scc/GWPACK-OTHER 2/RETURN-COUNT 19）汇聚为同一类：机器走查函数深嵌结构容量。下片转攻深嵌容量或抽辅助拆体（0343 纪律）。

**STEP-0397（2026-10-13）**：gow 部分推进（两处门形清除，户未翻绿 partial 登记；深嵌 if 截体伪影记录，下片 callee 守卫反查）。census 104/129、0 DIFF 维持；fmt/0262(80/80)/0245/canary/0124/diff-check 全绿。详见 [STEP-0397](./steps/STEP-0397-m22-w2-gow-partial.md)。

**STEP-0396（2026-10-13）**：match 主语形 69 站点全库折叠——cam 翻绿、CALL-SHAPE 3→1（独苗 gow）、casm 入 0365 trap 类。**census 103/129→104/129、0 DIFF**。selfhost_compiler 80/80；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0396](./steps/STEP-0396-m22-w2-match-subject-fold.md)。下一刀 = gow（末户 CALL-SHAPE）+ casm trap 专项 + RETURN-COUNT 19 户 + GWPACK-OTHER 2 户。

**STEP-0395（2026-10-13）**：gls 自降低翻绿——直行机器 bytes.length 操作数形三分化（仅 match 主语形已建模），sweep2 提升形态回退 + 编码纪律登记。**census 102/129→103/129、0 DIFF，CALL-SHAPE 3**。selfhost_compiler 79/79；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0395](./steps/STEP-0395-m22-w2-gls-selflowering.md)。下一刀 = CALL-SHAPE 3 户（疑似同族操作数形）+ GWPACK-OTHER 2 + RETURN-COUNT 19。

**STEP-0394（2026-10-13）**：straight_text_concat_packed 翻绿（两处嵌套调用整形）；wcr 截体法证伪（路由干扰）待 oracle 对照法。**census 101/129→102/129、0 DIFF，CALL-SHAPE 4**。selfhost_compiler 78/78；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0394](./steps/STEP-0394-m22-w2-stc-flip.md)。下一刀 = CALL-SHAPE 4 户 + wcr oracle 对照 + GWPACK-OTHER 2 户。

**STEP-0393（2026-10-13）**：泄漏假设撤销——首错语义实证（走查停在第一个失败函数）；wcr 本体真门槛 = RETURN-COUNT。下片 = wcr 前向截体 + oracle IR 逐块对照。

**STEP-0392（2026-10-13）**：RETURN-COUNT 追查续——IF-TAIL 假设证伪 + 截体二分未收束 + 误删行事故整文件恢复（纪律：revert 一律整文件备份恢复）；0393 换向 oracle IR 对照审查 general_while 终结子装配。

**STEP-0391（2026-10-13）**：RETURN-COUNT 顺序依赖实锤——位置矩阵（wcr 末位⇒STATEMENT/非末位⇒RETURN-COUNT）锁定跨函数状态泄漏（general_while 毒化后继 straight 机器的找 return 扫描）；嫌疑 = list.append 引用传递。下片最简 A-after-B 复现 + 变异二分。详见 [STEP-0391](./steps/STEP-0391-m22-w2-return-count-order-dependence.md)。

**STEP-0390（2026-10-13）**：RETURN-COUNT 门槛标定（诊断，源码净）——单返回假设证伪，真触发器 = 机器本体深度组合（双重 while + 深嵌 match 臂 return + 多返回）；下片截体二分。详见 [STEP-0390](./steps/STEP-0390-m22-w2-return-count-ladder.md)。

**STEP-0389（2026-10-13）**：gw RHS 机器学 `sico.list.empty[U64]()`（41 处全库初始化形状的公共门）+ gcc 单返回续。**census 99/128→101/129、0 DIFF，user_call_expression_json 翻绿，GWPACK-OTHER 独苗 control_if_function_ir**。selfhost_compiler 77/77；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0389](./steps/STEP-0389-m22-w2-gw-list-empty-rhs.md)。下一刀 = RETURN-COUNT 19 户（机器族本体逐户重构）+ CALL-SHAPE 7 户 + control_if 独苗。

**STEP-0388（2026-10-12）**：坏形状全库清扫（sweep2=71 bytes.length(encode) 提升 / sweep1=53 嵌套 concat 提升）。**census 98/128→99/128、0 DIFF，CALL-SHAPE 28→7 簇瓦解，RETURN-COUNT 19 成新主导族**。selfhost_compiler 76/76；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0388](./steps/STEP-0388-m22-w2-bad-shape-sweep.md)。下一刀 = RETURN-COUNT 19 户批量单返回重构（gic 模板）+ CALL-SHAPE 残余 7 户。

**STEP-0387（2026-10-12）**：RETURN-COUNT 簇清零——gw_intrinsic_call_packed 单返回重构（R3 纪律）。**census 97/128→98/128、0 DIFF，RETURN-COUNT 清零**。selfhost_compiler 75/75；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0387](./steps/STEP-0387-m22-w2-return-count-cluster.md)。下一刀 = CALL-SHAPE 28 户（gw_ok_wrap 续攻）+ control_if_function_ir（GWPACK-OTHER 独苗）。

**STEP-0386（2026-10-12）**：CALL-SHAPE 簇源码整形——七户翻绿（gdt/nir/aic 族/environment_call_arguments），嵌套调用与 const 实参提升 + bytes.length 入 gw 调度（gw_intrinsic_call_packed 推进至 RETURN-COUNT）。**census 90/128→97/128、0 DIFF，CALL-SHAPE 37→27，RETURN-COUNT 回潮 2 户**。selfhost_compiler 74/74；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0386](./steps/STEP-0386-m22-w2-call-shape-reshaping.md)。下一刀 = RETURN-COUNT 2 户 + CALL-SHAPE 27 户 + control_if_function_ir（GWPACK-OTHER 独苗）。

**STEP-0385（2026-10-12）**：GWPACK-OTHER 簇瓦解——sico.text.length 入 gw RHS 调度（委托既有 gw_intrinsic_call_packed）+ 两处 text.length(user_call) 实参提升绕行。**census 89/128→90/128、0 DIFF，GWPACK-OTHER 34→1、CALL-SHAPE 5→37 级联，straight_environment_literal_count 翻绿**。selfhost_compiler 73/73；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0385](./steps/STEP-0385-m22-w2-gw-text-length-rhs.md)。下一刀 = CALL-SHAPE 37 户门族细分（新主导族）。

**STEP-0384（2026-10-12）**：GWPACK-OTHER 门族第一刀——门形 = gw_rhs_packed 回 EXPRESSION 且绑定非 "out"；新打包器 gw_list_sort_rhs_packed（arity 守卫 + oracle range 对拍修复）+ text.join 委托既有 binary 打包器 + records_table_json 嵌套调用拆 set 绕行。**census 88/127→89/128、0 DIFF，records_table_json 翻绿，GWPACK-OTHER 35→34**。selfhost_compiler 72/72；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0384](./steps/STEP-0384-m22-w2-gw-sort-join-rhs.md)。下一刀 = GWPACK-OTHER 34 户逐族微复现。

**STEP-0383（2026-10-12）**：CALL-ARGUMENT 27 户攻坚收官——门形微复现（user/builtin 外呼 + intrinsic 实参 = CALL-ARGUMENT；intrinsic 外呼 + intrinsic 实参按操作数分化〔concat 拒 / bytes.equal-inline 收〕→ 提升只动前者）+ 源码变换链（flatten 重结合 / family-1 hoist / 支配安全 let→set reuse）+ 30 户回归嫁接回退。**census 82→88/127、0 DIFF，CALL-ARGUMENT 27→0 清零、EXPRESSION 0、CALL-SHAPE 16→4、GWPACK-OTHER 1→35**。固件燃料 20B→40B + 前沿断言改 GWPACK-OTHER；selfhost_compiler 71/71；fmt/0262/0245/canary(30/30)/0124/diff-check 全绿。详见 [STEP-0383](./steps/STEP-0383-m22-w2-call-argument-hoist.md)。下一刀 = GWPACK-OTHER 35 户门族细分。

**STEP-0370（2026-10-12）**：入口区 match 支持（gw_match_open_pack entry 门摘除，一处 −3 行）+ 0368/0369 工件更正（n 系探针漏 end function，结论作废；探针纪律补 sico check 前置）。**census 76/127、0 DIFF，STATEMENT 类清零**。selfhost_compiler 70/70；fmt/0262/0245/canary/0124/diff-check 全绿。详见 [STEP-0370](./steps/STEP-0370-m22-w2-entry-match.md)。

**STEP-0365（2026-10-12）**：casm wasm trap 定位前哨——trap 独立复现（exit 125，22 函数闭包探针留存 target/tmp/probe_casm.sico）；裸行截体二分被不平衡切口混淆（信号不可信），静态扫描排除无兜底 helper；推断 trap 在 casm 机器自身执行（oracle 发射的 unreachable 路径被实输入走到）。源码净（76/127、0 DIFF）。平衡截体定位语句 348–350 + oracle-IR 反查实锤：8 个 unreachable 全为双臂 return 尾 match 的死 join（正常发射），trap = Result 判别式被上游损坏的下游症状——**与 0364 值编号损坏同根因类（gw 机器大/深输入状态损坏）**；0368 新增数据点：损坏随上下文三态（无 match 前缀=正确 / match+浅嵌套=STATEMENT / 三层嵌套+match=值损坏）——指向 match 细胞存在时打包器状态交互；0369 已锁定浅端根因：gw 机器入口区顶层 match（gw_match_open_pack entry 门 + 摘门后的 region/join 次道门）——n5/n6 对照实证；下片做 entry-match 的 region/join 机器扩展（0341 登记 (b) 项的正体），顺带核 n2 嵌套 to_text 差异。详见 [STEP-0365](./steps/STEP-0365-m22-w2-casm-trap-diagnosis.md)。

**STEP-0364（2026-10-12）**：嵌套调用实参分支第二次复测——span 修复后同一字节发散原样复现（gdt result-60 `?`/args[60,0]/值跳 425）→ span 假设证伪，根因在嵌套分支的值编号/计数消费与外层实参循环相互作用（深上下文未建模形状类）；整片回退净（76/127、0 DIFF、逐格一致）。**登记：勿第三次直落该分支，先以 gdt 截体专项解值编号根因。** 另登记：checked_arithmetic_set_match_ir census 现报 wasm trap（NON-DOMAIN，缺陷类待修——应 typed 拒绝）。详见 [STEP-0364](./steps/STEP-0364-m22-w2-nested-arg-second-revert.md)。

**STEP-0363（2026-10-12）**：gw_find_code_text 翻绿（T6–T8 截体定位循环尾 set 门 → gfc_prev_idx 语义等价改写，字节全等）——连锁解锁 **list_get_match_ir / chained_let_function_ir / straight_let_function_ir 三户大闭包全 FLIP**。census 76/127、0 DIFF；selfhost_compiler 69/69；fmt/0262/0245/canary/0124/diff-check 全绿。详见 [STEP-0363](./steps/STEP-0363-m22-w2-gfc-flip.md)。

**STEP-0362（2026-10-12）**：gw_find_code_text 自降低门链微复现——straight_let 远距拒绝根因 = helper 自身拒绝毒化含它的全部闭包（新教训：新增 helper 必须立即微复现自降低）；截体 T1–T5 逐门定位并重构（bytes.at cell 游标 / 前向反斜杠奇偶 / prev-byte 扁平化）；helper 推进至下一道 EXPRESSION 门（语义正确、不冻结）。**census 72/127→76/127、0 DIFF**（+4 连锁翻绿无回归）；0262（68 项）/0245/canary/0124/diff-check 全绿。详见 [STEP-0362](./steps/STEP-0362-m22-w2-gfc-selflowering-gates.md)。

**STEP-0361（2026-10-12）**：0360 两处潜伏 span 缺陷正解修复 + to_text 门落地——(A) gcursor 从声明冒号起；(B) `gw_find_code_text` helper（字面量/注释感知）替换全部 26 处 fn_end 搜索；(C) `gw_text_concat_packed` 新增 `u64.to_text` 实参分支。**census 68/126→72/127、0 DIFF**；CALL-ARGUMENT 36→7，三户原拒绝户推进至 EXPRESSION 真实下一门；selfhost_compiler 68/68；fmt/0262/0245/canary/0124/diff-check 全绿。详见 [STEP-0361](./steps/STEP-0361-m22-w2-to-text-and-span-fixes.md)。

**STEP-0360（2026-10-12）**：gw 机器两处潜伏 span 缺陷实锤——A：gcursor=fn_start 致函数名含 "let" 者（straight_let/chained_let）语句关键字命中函数名子串（write_local span −188B 实锤）；B：fn_end 字节搜索命中函数体内 `"end function"` 字符串字面量（−212B 实锤，list_get_match_ir 等）。修复尝试：A 单独有效；B 的 token 走线 helper 证伪（中间词漂移 → 31 DIFF），全量回退净（68/126、0 DIFF、逐格一致）。**下片正解方向已登记**（A：词边界/token 偏移定位；B：end-function 处理器惰性求值 + init gcur_re 依赖处理），修复后 to_text 门按同法重落地。详见 [STEP-0360](./steps/STEP-0360-m22-w2-gw-span-defects-diagnosis.md)。

**STEP-0359（2026-10-12）**：to_text 分支单落地复测（不带嵌套支持）→ 同 3 例 DIFF → 第二次回退（census 复测 68/126、0 DIFF 净）。机制定位：三户真实基线 = CALL-ARGUMENT 拒绝，分支使其 exit 0 但 span 偏 212B（straight_let result-10 write_local start 42178→41990，隔离形状 byte-exact）——疑点在多语句链 gcursor 推进与分支 word_span_offset 相互作用。 census 探针已增强（DIFF 行带闭包清单）。详见 [STEP-0359](./steps/STEP-0359-m22-w2-to-text-branch-second-revert.md)。

**STEP-0358（2026-10-12）**：concat 实参 `u64.to_text` 族探针——隔离形状（to_text(cell)/to_text(嵌套调用)）均达 byte-exact，但冻结语料 3 例 DIFF → 按 0-DIFF 纪律**整片回退**；census 复测 68/126、0 DIFF、histogram 与 0357 终值逐格一致。登记发现：深上下文（match 细胞+嵌套 while+if 块）嵌套调用实参解析发散（guest 实参 value 0/越界 `?`），根因待 gdt 截体二分；门本身可解。详见 [STEP-0358](./steps/STEP-0358-m22-w2-concat-to-text-probe-reverted.md)。

**STEP-0357（2026-10-12）**：0356"words 截断"发现更正（探针脚本工件误判，机器全程喂全量 words）+ 右操作数用户调用分支闭环（0356 漏置 `gcp_r_done`，一行修复，ccR byte-exact）。**chained_operand_check 翻绿，census 67→68/126、0 DIFF**（EXPRESSION 24→14 / CALL-ARGUMENT 36 / CALL-SHAPE 7 首次出现）。selfhost_compiler 67/67；fmt/0262/0245/canary/0124/diff-check 全绿。**探针纪律教训**：sed 改写探针脚本后必须校验输出文件字节数（本片工件根源）。详见 [STEP-0357](./steps/STEP-0357-m22-w2-compare-right-call-fix.md)。


**STEP-0356（2026-10-12）**：gw 条件右操作数两分支——泛型 `sico.list.length[U64]`（直接发射，绕过 gw_intrinsic_call_packed 的 +5 走位）+ 用户调用右操作数（镜像左路径）；新局部 `gcp_r_done` 门保护 width 收尾。**gw_unreachable_terminator 翻绿，census 66→67/126、0 DIFF，GWCOND-EXPRESSION 1→0 清零**（histogram CALL-ARGUMENT 34 / EXPRESSION 24 / STATEMENT 1）。selfhost_compiler 66/66；fmt/0262/0245/canary/0124/diff-check 本片重跑全绿。**途中诊断发现（下片首选攻击点）**：callee-last 模块中机器上下文 words 截断——探针在 callees 后时 `sico.list.length(words)`=86（≈单函数量，被调声明不可见），探针置于模块首位时 =207（全量、被调解析成功）；左右用户调用操作数 callee 解析同征，非本片引入（0355 基线同征），根因 = scalar_ir 喂给机器的 words 来源审查（疑似按函数/游标切片）。`gw_deferred_tail_json` 仍拒 EXPRESSION = 该上下文为其实际下一门。详见 [STEP-0356](./steps/STEP-0356-m22-w2-gw-compare-generic-operands.md)。
**STEP-0353（2026-10-05）**：gw match 主语家族扩 `checked_mul`/`checked_div`（微复现二分定位 gw_decimal_value 的 CALL-TARGET 根因 = 0324 族外；路由 + op 名检查三处扩族、发射零改动）；STEP-0324 拒绝固件同步（checked_mul 转正例、族外守卫改 bit_and）。**gw_decimal_value 翻绿；census 58/123、0 DIFF；CALL-TARGET 拒绝类清零**（histogram：RETURN-COUNT 23/CALL-ARGUMENT 21/MATCH-SHAPE 12/EXPRESSION 8/STATEMENT 1）。0262 OK、selfhost_compiler 63/63、0245 M22 回归绿（clippy 环境阻塞同 0352）。详见 [STEP-0353](./steps/STEP-0353-m22-w2-checked-mul-div-subjects.md)。下一刀 = 多返回 while 形状（gw_instr_count 3 返回，RETURN-COUNT 23 户大头）。

**STEP-0352（2026-10-05）**：gw 调用链机器 `gw_call_chain_packed` 落地（吸收在途 WIP + 三处修复：双臂 `let gcc_out` 重声明提升、call 分支 `(` 定位改向 bound 扫描〔根因〕、pieces 合并条件 `==1`→`>0` 修链指令丢失）。**word_span_offset 翻绿；依赖闭包 census 57/123 翻绿、0 DIFF；canary 30/30 FORMATTER-CLOSED、R3 0**；分母修正 parser.sico=123 函数。0245 M22 回归 63/63 绿（runner clippy 缺 gcc 环境阻塞，CI 裁决）；runner fmt HEAD 预存漂移随片清理。详见 [STEP-0352](./steps/STEP-0352-m22-w2-gw-call-chain.md)。

**STEP-0341（2026-10-03）**：gw 机器 **match 栈**落地——单槽 `match_open` 改为 `gmt_stack`/`gmt_depth` 进出栈（嵌套 open 时 push 外层 `gmt_p_pack`，`end match` 时 pop 恢复、depth==0 才清 open）；`scalar_ir` 梯级门 match>1 且 while==0 的 CONTROL 拒绝删除并改为路由 gw。`count_params` 翻绿（while 体内 list.get match + if-then 内嵌套 checked_sub match，line_end 前缀字节全等）→ parser **35/120**。**二分钉死的剩余缺口**：(a) match 臂内 if/else 双臂 return（R3 形状 → STATEMENT；find_body_end/count_until_end_function 的真实形状——gfr 未建模"后支配 return 的 if 收口"，case 处理器需接受 gcur_term=true 的臂尾；翻绿后 fn_entry 连锁解锁）；(b) 无 while 函数的顶层 match（header 区 STATEMENT）。探针矩阵：while 内 match 臂 return（R2）✓、嵌套 match+臂内 return（R4）✓ 均已通。

**STEP-0340（2026-10-03）**：`gw_starts_with_condition_packed` 落地——gw 机器 if 条件接受 `sico.text.starts_with(<expr>, "<literal>")`（parser.sico 最普遍条件形状，185 处，此前拒 SCICOND）。接收者 param（零指令）/cell（read_local）/用户调用（gw_rhs_packed 复用）+ 字面量参（scalar_const const_string）+ bool intrinsic；**id 记账：intrinsic result = const_id + 1**（total = recv_count + 2）。5 函数翻绿（is_punct_byte/type_json/declared_return_kind/binding_parameter_kind/parameter_kind_by_id）。附带钉死：嵌套用户调用实参 let-RHS 已通；declared_return_kind 的 CALL-TARGET = 缺 scalar_type_kind 被调，非机器缺口。

**STEP-0339（2026-10-03）**：全量函数普查 4 pass 至不动点（单独 → 4被调前缀 → 11函数前缀 → 14函数前缀，oracle 字节对比）——**9 函数新翻绿**（is_declaration_start、literal_value_index、list_aware_parameter_index、is_intrinsic_path、record_scalar_kind、record_close_bracket、record_map_comma、function_declaration_index、function_declaration_ordinal），全经既有机器、无新手术，冻结语料按普查原组合登记。**分母修正**：实测函数头 0319 起 117、0325 起 119、本片起 120；历史 "/115" 为陈旧口径。**前沿地图**（不动点）：深层 CALL-TARGET 传递阻塞于 mid-layer；`has_return_at` 的 CALL-TARGET 实为缺 `is_ident_byte` 被调；`json_string_token` 维持 while+match CONTROL；`u64_list_at` MATCH-SHAPE、`type_json` SCICOND。普查探针已按纪律删除；方法：all_function_names() 逐个 + oracle_ir 对比，前缀递增逼近闭包。

**STEP-0338（2026-10-03）**：守卫链机器（`fixed_guard_chain_ir`）三加宽落地——宿主调用 let（守卫间/前 `let` 经 `gw_rhs_packed` 复用打包 + `write_local` 入条件块前缀 + cell 三列表注册，cell local = cell 下标）、条件打包器 `chain_condition_packed`（双 SKIP 后第三 fallback：`sico.text.length`/`sico.bytes.length`/`sico.bytes.at` 左值 + param/cell/字面量右值，其余形状委托字节已证 `gw_compare_packed`；守卫与 final 两处接线）、有 cell 时发射 `"locals"` 数组。`while_bytes_at_local_src_packed` 字面量 cursor 形态（form 2）按 oracle 转储落地（STEP-0327 遗留疑点裁决）。`is_digit_token`/`is_string_token`/`is_ident_byte` 翻绿，canary +3，R3 0，套件 58/58、runner 全量绿。**宽度约定已钉死**：`fixed_operation_operand_width` 返回 token 数，`atom+width` = 操作数自身 `)` 之后的下一个 token——一切操作数走位依赖此约定（`bytes.at` 字面量游标的逗号 = 游标+width 本身）。**存疑标记**：同函数的 "sico" 规则（1db4624 移植件）对 `sico.<f>.<m>` 以 +4 判 `(`（实际 open 在 +5），当前不在已证路径，修复/验证前不得扩用。会话从 `1db4624`（中断会话 WIP 快照：scalar_ir gw-decline fallback + rustfmt 修正）续起——该快照承载 is_ident_byte 的 control_if-declined→gw 翻绿路径，勿回退。

M22 整体 **NO-GO**。S1/S2 声明子集完成，S3/S4/S5 partial，S6/S7 未进入。R0 已接受 ADR-0017 Option A 与 RFC-0047。STEP-0292 对 `e8495f9` 的独立 Windows GNU [GitHub CI](https://github.com/ImmortalWing/sico/actions/runs/36087222284) 成功，**W1 在本隔离分支快照裁为 GO**。该裁决不覆盖 `origin/dev` 的并行历史。

STEP-0293 是 22-C 首个 R3 计数实现片：while 体 sico.u64.to_text RHS 与 Rust oracle byte-exact，错误参数类型 typed 拒绝；formatter canary 22/30，nearest_match 从 GWPACK-OTHER 移至 CALL-TARGET，整份 exit 122。cb371c5 的独立 Windows GNU CI 运行 36090403437 成功，S1 于本隔离分支裁为 GO。R3 连续停滞数 0。

STEP-0294 是 22-C S2：`sico.map.get[Text,U64]` 的受限 match 主语与 Rust oracle 字节一致；错误 Map/key 类型、参数数和泛型数 typed 拒绝。formatter canary 仍 22/30，`nearest_match` 前沿 CALL-TARGET → STATEMENT，整份 exit 122，R3 连续停滞数仍为 0。本机 0261/0262/0245 全绿，`5a6d0bb` 的独立 Windows GNU [CI 运行 36147743130](https://github.com/ImmortalWing/sico/actions/runs/36147743130) 成功，**S2 于本隔离分支裁为 GO**。

## 1. 证据与边界

| 检查 | 当前结果 | 证据边界 |
|---|---|---|
| STEP-0292 fresh-runner CI | `e8495f9` 的完整 `run-ci.ps1` 作业 success | W1 快照；Windows 2025 GNU |
| STEP-0293 fresh-runner CI | 修复版 `cb371c5` 的完整 `run-ci.ps1` 作业 success（[运行 36090403437](https://github.com/ImmortalWing/sico/actions/runs/36090403437)） | S1 本片；Windows 2025 GNU |
| STEP-0294 fresh-runner CI | 最终 `5a6d0bb` 的完整 `run-ci.ps1` 作业 success（[运行 36147743130](https://github.com/ImmortalWing/sico/actions/runs/36147743130)） | S2 本片；Windows 2025 GNU |
| `selfhost_checker` | 9/9，原 215 项 + W1 五项 Rust/guest 对照 | Windows x64 GNU 真实 runner；internal-fixture |
| S1 新增 compiler 用例 | while 体 `u64.to_text` canonical IR byte-exact，双参数与错误类型 parameter/local typed 拒绝 | 真实 runner；internal-fixture |
| S2 新增 compiler 用例 | literal/parameter/local Text key 字节相等，extra arg/generic、wrong Map/key type typed 拒绝，拒绝后恢复 | 真实 runner；internal-fixture |
| `validate-step-0262.ps1` | compiler 23/23、parser 2/2、local-bounds 1/1、22 函数 IR 快照、现行前沿均绿 | S2 本机复跑；gw 最繁忙函数 240 locals |
| `validate-step-0261.ps1` | 同上及 17 函数历史快照绿 | S2 本机复跑 |
| `validate-step-0245.ps1` | semantic 6+11、bundle 4、checker 9、compiler 23、runner clippy 全绿 | S2 本机复跑 |
| `report-m22-canary.ps1` | 22/30，`nearest_match` / `STATEMENT`，exit 122 | S2 当前树；栈高水位仍未测得 |

## 2. 下一步

0. **STEP-0382 后基线**：CALL-ARGUMENT 27 户机器本体自降（gw_rhs_packed/gw_intrinsic_call_packed/scalar_ir 等——翻任一即大片连锁）+ CALL-SHAPE 16 户；嵌套调用实参专项停车留档（0380 登记，可重开）。runner clippy 本机需 `$env:Path` 前置主仓 msys2 binutils。
3. M23 的 RFC-0048/0049、M24 的 RFC-0050/0051/0052 与 ADR-0018、M26 的 RFC-0053/0054 仍各自等待 owner 接受；它们不阻挡本分支 22-C S3。

## 3. 本机复核命令（PowerShell，仓库根目录）

```powershell
git status --short
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$binutils = Join-Path (Get-Location) 'target\tooling\msys2-binutils\mingw64\bin'
$env:Path = "$binutils;$env:Path"
.\tools\validate-step-0245.ps1
.\tools\validate-step-0261.ps1
.\tools\validate-step-0262.ps1
.\tools\report-m22-canary.ps1
.\tools\validate-step-0124.ps1 -SelfTest
git diff --check
```

STEP-0261/0262 会重复执行较慢的 compiler/parser 差分；只在源码或当前前沿变化后重跑。S6 还必须实测 guest memory/stack 预算，不能将缺失的栈高水位写成已有证据。

## 4. guest 自举降级的形状前沿（STEP-0491 实测，2026-10-21）

`selfhost/parser.sico` 自身被 guest 逐函数降低并与 Rust oracle 逐字节对拍（`sico_guest_arena_accommodates_full_parser_selfhost_input`）。因此**新代码必须同时能被 guest 自己降低**——否则全源对拍从 exit 0 变成 typed 拒绝 `ERR:E-SH-IR-CALL-SHAPE`。下列形状**实测降不动**：

| # | 形状 | 症状 | 已证可用的规避形状 |
|---|---|---|---|
| 1 | 深层嵌套块内的 `match`（while 体 → else → 5 层 if → match） | `ERR:E-SH-IR-CALL-SHAPE` | 把 checked 加法放进 `match` 主语：`match U64.checked_add(<偏移>, sico.bytes.length(sico.text.encode(<名字>))):`——与 `scalar_ir` 内既有的 `match U64.checked_add(range_start, sico.bytes.length(sico.text.encode(range_lexeme))):` 同形 |
| 2 | 深层块内 `set` 的右值是嵌套 intrinsic 调用 | `ERR:E-SH-IR-CALL-SHAPE` | 嵌套 intrinsic 只出现在 `match` 主语里，不再单独作 `set` 右值 |
| 3 | 函数尾部 `match` 主语取**只在循环体内赋值**的局部 | `ERR:E-SH-IR-CALL-SHAPE` | 不在尾部做该加法；在嵌套块内一次算完 |
| 4 | 跨块读取块作用域 `let` | 静默取到 0 → 游标归零死循环（fuel 耗尽） | `let` 声明必须提到与使用处同层或更外层 |
| 5 | 源码里写 `#` 注释 | 词法错误 `UnexpectedCharacter('#')`（整树编译失败） | Sico 源码**无注释语法**，一律不写 |

> 资产价值：guest 前端的自举可降级性是**形状前沿**而非功能开关。`docs/development/script-component-emission-v0.md` 记录的是 **Rust 侧**发射算法，不代表 guest 侧能照抄同样的 Sico 写法；后续刀（record 构造 / 变体 / IR 摄取）必须先在本表里找已证形状再落笔。

## 5. guest 前端的语法与语义纪律（STEP-0494 实测 / STEP-0495 补第 9 条 / STEP-0496 补第 10–11 条 / STEP-0499 补第 12 条 / STEP-0501 补第 13 条 / STEP-0502 补第 14 条 / STEP-0503 补第 15 条 / STEP-0504 补第 16 条，2026-10-21）

下列**不是**形状问题，而是 Sico 前端的语法/语义硬约束；踩中时的症状具有欺骗性（第一条几乎无声）。

| # | 纪律 | 症状 | 正确写法 |
|---|---|---|---|
| 6 | 返回类型必须写 `returns X:` | 写成 `-> X:` **没有语法诊断**（lexer 把它当普通标点），但语义模型会**静默丢掉该函数**（`sico-semantics` 的 `parse_function` 用 `position(TokenKind::Returns)?` 定位返回类型，失败即 `None`），于是**每个调用点**报 `E2031 unresolved call target`。`sico outline` 仍会列出该函数（HIR 收得到），只有语义模型没有它 | `function f(...) returns Text:` |
| 7 | 同一函数内 `let` 不可重名（**互斥分支也不行**） | 编译报 `unsupported cell redeclared` | 顶层声明一次，分支内只 `set` |
| 8 | "表达式恰好 N 个 token"的边界 | 边界写成"倒数第二个 token"会让形态永远落空（表现为 `ERR:E-SH-IR-CONSTRUCT-VALUE` 之类的笼统拒绝） | 用**最后一个 token 的后继索引**与段尾比较 |
| 9 | **match 分支的 `case ok(x)` / `case error(x)` 绑定与函数的 `let` 单元共用同一个命名空间**（第 7 条的另一半） | 编译报 `unsupported cell redeclared at bytes <offset>`——**即使两者分处不同语句**（STEP-0495 在 `selfhost/compiler.sico` 里 `case ok(split_tail): ...` 之后又 `let split_tail = ...` 即中招） | match 绑定用**专名**（`split_tail_len_value`），别和 `let` 单元同名 |
| 10 | **形参 / 局部名不得取语言关键字** | 编译报 `cannot lower <文件>: unsupported atomic expression at bytes <offset>`——**报错只给入口文件名，不给真正出问题的那一行**（本例 range 实际落在被导入模块 `selfhost/codegen.sico` 里）。根因：`crates/sico-lexer/src/lib.rs` 的关键字表（`"from" => TokenKind::From` 等）+ `crates/sico-ir/src/lower.rs` 的 `_ => Err(unsupported_error("atomic expression", token.range))`——关键字 token 在 lowerer 里一律走"原子表达式"兜底分支 | 用**中性名**（`from` → `start` / `from_off`）。常见雷区：`from` / `to` / `type` / `use` / `module` / `task` / `try` / `new` / **`record`**（STEP-0502 实测：形参名取 `record` ⇒ `unsupported atomic expression`，报错范围正好落在该形参名上，但**报错只给入口文件名**） |
| 11 | **同函数内的 `let` 没有块作用域**（第 7 条的第三种形态） | 编译报 `unsupported cell redeclared at bytes <offset>`——**两个 `let` 分处不同 `if` 分支也会撞**（STEP-0496 在 `cgp_type_text` 的 `named` 分支与 `result` 分支各写了一个 `let cgpt_data` 即中招），**两个 `while` 循环体里各写一个同名 `let` 同样会撞**（STEP-0502 的 `cgs_variant_hex` 先后两段循环都声明了 `cvh_item` / `cvh_value` / `cvh_ty` / `cvh_count`） | 分支内用**带分支前缀的名字**（`cgpt_data` → `cgpt_result_data`），或把声明提到分支之前；同一函数内所有 `let` 名必须**全局唯一** |
| 12 | **扁平判定有固定次序：模块自己声明的 record 优先，其次内建名；且"不可扁平的自声明 record"等于"未声明"** | 次序写反时，一个字段不可扁平的自声明 record 会被误判成 1 × i32（落进 `ScriptErrorCode` 分支）。根因在 oracle 侧：`register_user_records` 对不可扁平的 record **不注册**（`flattenable = false` ⇒ 不入 `user_records`），因此它继续落到内建名分支。症状不是报错，而是"某个 fixture 的局部声明字节差一组"，且只在该 fixture 上出现 | 先查模块声明的 record，查不到再判内建名；不可扁平 ⇒ 返回空 ⇒ 一律 typed 拒绝（`ERR:E-SH-CG-IR-FLAT`），**不近似** |
| 13 | **脚本 profile 有两套布局，不能混用** | 症状是"结果区字段整体偏移 4 字节"：按 8 字节 flat 槽算出来的字段偏移，套到**结果区**上就全错位。具体两套：**局部变量 / 槽位**用 `flat_ir_types` 的 **8 字节槽**（i32 也占 8 字节，`byte_offset = slot_start × 8`）；**结果区**用**冻结的 WIT 布局**（i32 占 4 字节，`i32.store` 的 ptr/len 落在 offset+0 / offset+4），`result_payload_offset = 8` | 局部用 flat 槽模型；结果区用冻结的 WIT 字段偏移表（`ScriptOutput` = stdout@0 / stderr@8 / exit_code@16；`ScriptError` = code@0 / message@4）。**两者不可互相推导**（`script-component-emission-v0.md` §3 的 flat 规则 vs §6 的结果区布局） |
| 14 | **`U64.equal(a, b)` 的返回值是 `Bool`，不是 `U64`** | 编译报 `E2001 expected U64, found Bool`——把它当 U64 再包一层（`U64.equal(U64.equal(x, y), false)`）即中招。**取否定没有内建写法** | 要判否定就**写一个取反的助手函数**（STEP-0502 的 `cgs_decimal_bad`）；布尔量直接作 `if` 条件（`if csh_sign:`）；若要用 U64 承载布尔，就**只在一处**做 Bool→U64 转换，之后一律用 `U64.equal(flag, 1)` 读 |
| 15 | **`cg_leb_hex` 是裸立即数编码器，不是指令编码器** | STEP-0503 实测：把 `call <index>` 写成 `cg_leb_hex(index)` ⇒ 输出是 `4120` 后面直接跟 `01`，**少了 `call` 的操作码 `10`** ⇒ 整段尾段**每一处都少 1 字节**。症状具有欺骗性：guest 侧**不报错**、字段顺序与槽位全对，只在逐字节对拍时表现为"每处少 1 字节" | 凡「操作码 + 立即数」形式的指令都要有**同形的助手函数**（`cgs_local_get_hex` = `"20" + cg_leb_hex(slot)`、`cgs_local_set_hex` = `"21" + cg_leb_hex(slot)`、`cgs_i32_const_hex` = `"41" + cgs_sleb_hex(v)`）。`call` 原先**没有**这样的助手 ⇒ 本片补 `cgs_frozen_alloc_call_hex()` = `"10" + cg_leb_hex(index)`。纪律：**`cg_leb_hex` 只能出现在"已经写了操作码"的位置**；`local.get`/`local.set`/`i32.const` 已有助手，`call` 之前漏了 |
| 16 | **冻结的骨架装不下 backend 会多发的东西时，必须 typed 拒绝，不能静默丢弃** | STEP-0504 实测：guest 携带的**函数段是冻结的**（`run`/`alloc`/`realloc`/`post` 四个）。backend 遇到模块里的**第二个源函数**时会**多发一个核心函数**（函数段与 code section 都要变），而冻结的函数段**装不下**它 ⇒ 若不处理，guest 产出的组件会**合法通过 wasmparser 验证、却少了那个函数**——一个「看似合法、语义错误」的组件。症状具有欺骗性：**没有任何报错**，字节对拍也不会自己发现（对照的是同形参考形态，不是多函数形态） | 先数一数**骨架的容量**（本例 = `cgj_array_item(ir, functions, 1)` 必须为 0），装不下就**返回空 ⇒ typed 拒绝**，并让新固件钉覆盖这个形状（`sico_codegen_guest_refuses_the_component_emission_outside_the_subset` 的「echo + 多一个未用函数」实例）。纪律：**每引入一条冻结骨架，就要同时写清它的容量边界与越界时的 typed 拒绝**；「backend 会多发而我装不下」是 silent-drop 的典型来源 |

> 另一条**判定顺序**纪律（不属于语法，但在同一片里踩中并当场抓出）：`record_scalar_kind` 对**未知词返回的是 ERR 串**（`ERR:E-SH-IR-RECORD-TYPE`）而**不是空串**——所以"先看返回值长度、为 0 才当命名类型"的写法会把**命名类型判成错误**。正确顺序：先看是否以 `ERR:` 开头，是则转命名类型判定，否则才是标量。

> 再一条**宿主/包限值契约**纪律（STEP-0497 实测，不属于语法但同属"违反时症状具有欺骗性"）：**任何执行路径的 effective guest 内存上限必须 ≥ 脚本组件声明的下界**（`SCRIPT_MEMORY_BYTES` = 33,792 页 = 2,214,592,512 B，声明时 min == max）。症状是**启动即 trap**（`forcing trap when growing memory to <声明下界> bytes`），既不是降低失败也不是宿主内存不足，且**每台机器都复现**。致命之处在于 effective = **min(包 manifest, 宿主)**：两处都可能低于下界，**只修一处仍红**（STEP-0497 实测：只把宿主 2 GiB 抬到 2.5 GiB，用例仍然红，因为真正绑定的是包 manifest 继承的 64 MiB）。因此该契约必须写成**可执行守卫**（codegen 字节级 / package / runtime / runner 四层，均不依赖 runtime 二进制），而不是口头约定——否则"本地绿灯是空跑"（`SICO_TEST_WASMTIME` 未设时启动缝用例直接 return）会再次掩盖红点。

> 再一条**协作纪律**（STEP-0499 实测，不属于语法但同属"违反时症状具有欺骗性"）：**STEP 编号是跨并行 worktree 的共享序列**。本片在并行 M23 会话提交 `36f0ee9`（占用 STEP-0498）**之前**就把自己的片取号为 STEP-0498；当时核对 `docs/steps/` 无冲突，但 M23 的提交**先落地**。症状**不报错**，只表现为"同一个编号在 `docs/steps/` 下出现两份记录"（已入库的那份有提交，另一份还是未跟踪文件）。纪律：**取号之后到提交之前，只要同树有并行会话落地提交，就必须重新核对编号**；冲突时**已提交的一方保留，未入库的一方让位重编号**（本片让位为 STEP-0499），并在自己的 STEP 记录里写明让位原因与让位前的原编号。

> 再一条**验证覆盖**纪律（STEP-0499 实测，不属于语法但同属"违反时症状具有欺骗性"）：**根工作区的 `cargo fmt --all -- --check` 不覆盖 `runner/sico-runner`**——后者是**独立 Cargo 工作区**，不在根 `Cargo.toml` 的 `members` 里（根只有 28 个 `crates/*` 与 `pilots/*`）。`tools/run-ci.ps1` 的 11 步**没有** `fmt (runner)` 步骤（只有 `clippy (runner)`，而 clippy 不查排版），唯一的 runner fmt 检查在 `tools/validate-step-0245.ps1` 里，**0245 不在 run-ci 的 11 步内**，CI 工作流 `.github/workflows/m22-w1-ci.yml` 又只调用 `run-ci.ps1` ⇒ **runner 工作区的 rustfmt 违规能同时通过本机一键 CI 与独立 CI**。症状具有欺骗性：同一棵树上 run-ci 的 `fmt` 报 PASS、0245 的 runner fmt 报 FAIL，看起来像"两个工具互相矛盾"。STEP-0499 实测到此缺口并已按 rustfmt 输出对齐修好本片代码；**owner 拍板后本片已修缺口本身**——`tools/run-ci.ps1` 增加第 12 步 `fmt (runner)`（与既有 `clippy (runner)` 同形），**自本片起独立 CI 的内层步数为 12**（此前两轮历史 run 的 11/11 是历史事实，裁决证据不变，见 [STEP-0500](./steps/STEP-0500-m23-exit-verdict-independent-ci-green.md) §2 的「步数口径说明」）。

> 再一条**哨兵与布尔标志**纪律（STEP-0502 实测，不属于语法但同属"违反时症状具有欺骗性"）：**"找不到"与"总数"不能共用同一个 0 哨兵**——STEP-0502 曾用"空 id 当哨兵"的技巧想一次拿到槽位总数，但查值函数的"找不到"返回也是 0，于是总数恒为 0，表现为"整个指令段 typed 拒绝"。同类的还有**布尔标志的语义写反**：把"相等"命名为 `mismatch` 并在相等时置 1，读取时又按"不相等"理解 ⇒ **值明明相等却走拒绝分支**。这两条都被**字节对拍当场抓出**（前缀逐字节相同、在 `variant` 处整体拒绝），但症状是"笼统的 typed 拒绝"而不是定位信息 ⇒ 纪律：**哨兵值与语义标志都必须带中性名字 + 明确默认值**，且查找失败与统计总量要分成两个函数。

> 再一条**验证基础设施**纪律（STEP-0504 实测，不属于语法但同属"违反时症状具有欺骗性"）：**验证器自己的失败必须能被验证器之外的机制看见**。本片的行尾审计步骤**实测检出**了一个真实缺陷（`selfhost/codegen.sico` 的工作副本整文件是 CRLF，2623 CRLF / 0 LF），并**返回了 1**——但电池汇总仍打印 `BATTERY_GREEN`。根因是退出码管道：步骤体里用 PowerShell 变量作返回值（`$bad` = 1），而 `Run-Step` 读的是 `$LASTEXITCODE`——那是**上一个原生命令**的 0。症状具有欺骗性：**汇总的绿色与真实的失败同时出现**，若不逐行读步骤输出就会把红灯读成绿灯。纪律：①**每个验证步骤的失败必须走一个真正的进程退出码**（拆成独立 `.ps1` 后用 `powershell -File` 调用），不要用 PowerShell 变量当返回值；②**审计类步骤（行尾、BOM、字节级比对）必须真的检查工作副本**，不能只依赖 git 归一化后的提交内容——`.gitattributes` 的 `* text=auto eol=lf` 会在**入库时**把 CRLF 归一化成 LF ⇒ **提交内容不受影响，但本机证据跑的字节 ≠ CI 检出的字节**，两者必须一致才算证据成立。

> 这四条加上 §4 的形状前沿，共同构成"新代码必须能被 guest 自己降低"的**全部已知约束**。STEP-0494 因为前三条各返工一轮（详见 [STEP-0494](./steps/STEP-0494-m22-w2-s5-record-construct.md) §5），STEP-0495 补齐第 9 条（详见 [STEP-0495](./steps/STEP-0495-m22-w2-s5-result-variant.md) §5），STEP-0496 补齐第 10–11 条（详见 [STEP-0496](./steps/STEP-0496-m22-w2-s5-ir-ingestion-plan.md) §6），STEP-0499 补齐第 12 条与上面那条协作纪律（详见 [STEP-0499](./steps/STEP-0499-m22-w2-s5-slot-plan.md) §8），STEP-0501 补齐第 13 条（两套布局，详见 [STEP-0501](./steps/STEP-0501-m22-w2-s5-run-body-truth.md) §4.4），STEP-0502 扩写第 10–11 条并补齐第 14 条（详见 [STEP-0502](./steps/STEP-0502-m22-w2-s5-run-instruction-segment.md) §5），STEP-0503 补齐第 15 条（**`cg_leb_hex` 是裸立即数编码器、不是指令编码器**，详见 [STEP-0503](./steps/STEP-0503-m22-w2-s5-run-body-result-return.md) §5），STEP-0504 补齐第 16 条（**冻结骨架装不下 backend 会多发的东西时必须 typed 拒绝、不能静默丢弃**，详见 [STEP-0504](./steps/STEP-0504-m22-w2-s5-derived-component.md) §6）与上面那条**验证基础设施**纪律（详见 [STEP-0504](./steps/STEP-0504-m22-w2-s5-derived-component.md) §5.3），所以本表的扩充应与每片实现同步。
