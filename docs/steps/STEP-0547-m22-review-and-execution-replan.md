# STEP-0547 · M22 W2 · 步骤全量审查与执行重排片（`docs/reports/m22-review-2026-10-26.md`）

> **本片性质** = **审查/文档片**（`crates/` / `runner/` / `selfhost/` / `wit/` **零改动**）⇒ 不跑整轮电池，只跑相关校验器 + `git diff --check`。
> **不推进任何门**：门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4 与门 7 不得沿用 GO、M22 overall NO-GO 全部不变。
> **本片产出** = 一次跨 54 个 STEP 的复盘 + 新的执行顺序（P0–P6）+ 治理项 G1–G6，替代「一片一形状」的推进方式。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| 审查基线是**最新工况树** | ✅ | `E:/github/sico-w2`（branch `codex/m22-w2`），HEAD `4449efa` = STEP-0544，工作树含未提交 `AGENTS.md` §10 与 B1 游标修复 |
| 有可复核的**站点账本** | ✅ | `selfhost/match-component-coverage-v1.json` 396 站点（32 全等 / 140 FuelExhausted / 5 refused-guest / 219 closure-too-large） |
| 有**已执行**的规模读数 | ✅ | STEP-0516（S1/S2/S3）、STEP-0544（fuel 归因）、STEP-0545（B1，未提交于 scratch 树）、STEP-0546（B2，scratch 树未提交） |
| 不改代码、不动他人未提交改动 | ✅ | 本片只新增 `docs/reports/m22-review-2026-10-26.md` + 本记录 + 登记表行 + `AGENTS.md §10.1.3` |
| 不占用他人在飞编号 | ✅ | 0545/0546 已被 `sico-m22-step0545` / `sico-m22-step0546` 两棵 scratch 树占用，本片取 0547 |

## 1. 本片方法

1. 以**源码与已提交 STEP**为一手证据，不采信计划里的自述（`AGENTS.md` §1 第 4 条）。
2. 对每个「最近三片为何零回绿」的异常，回溯到**被测量的量**（fuel 二元组、墙钟、verdict 直方图）而不是结论文字。
3. 用**量级账**判断路线是否可达（输出 1 MiB ⇒ 拷贝次数 × 单次成本），而不是等下一个站点实验失败。
4. 文档与可执行证据冲突时取**更低支持声明**；本轮未执行的验证一律写明未做。

## 2. 发现（全部带定位）

| # | 发现 | 定位 |
|---|---|---|
| F1 | 发射端把输出累加成 **hex 文本**，用左折叠 `sico.text.concat` 追加 ⇒ 输出 N 字节需 O(N²) 拷贝 | `selfhost/codegen.sico`：194 functions / **512** 处 `sico.text.concat`（**461** 处自左折叠）/ **0** 处可变 byte；`cbsh_out`×37、`cccsh_out`×32、`clgh_out`×25、`cgpr_text`×25；`cgs_core_code_hex:6412` |
| F2 | STEP-0516 的「1 MiB 可行」只对 **S3（一次 concat 两个既有大操作数）**成立；发射端只能做 S1/S2，实测上限 **128 KiB / 256 KiB** | `docs/steps/STEP-0516-…-w1-scale-feasibility.md`：S1 128 KiB = 541,471,259 fuel、256 KiB `arena-exhausted`；S2 256 KiB = 1,085,117,073 fuel、384 KiB `arena-exhausted`；S3 操作数来自 stdin（≥1 MiB 源字面量撞 `source contract error LineTooLong at bytes 453..1049045`） |
| F3 | 0544→0546 三片都在优化**查表**，但二次项主体仍是**拷贝** ⇒ 0545 的 396 站点 verdict 逐条未变（32/140/5/219），0546 只把可达指令数推到 256（n=512 仍 `FuelExhausted` 94.31 s） | STEP-0544 X 族仅 `a+b·n+c·n²` 相容；STEP-0545 c 系数 0.6×、二次项仍占 76%–97%；STEP-0546 账本 36/136/5/219 |
| F4 | 219 个 `closure-too-large` 是**夹具自设上限**（64 函数 / 262,144 B），不是 guest 能力边界 ⇒ 从未开测 | `runner/sico-runner/tests/selfhost_compiler.rs:9286-9287`、`:9434` |
| F5 | **语言面今天已有线性装配**，无需 RFC/新内建：`list.append` 命中逻辑版本就地补位（摊还 O(n)），`text.join` 先总长后一次分配（O(total)） | `crates/sico-codegen-wasm/src/stdlib.rs:606-630`、`:635-641`、`:597-600` 注释、`:670-` `emit_join`；`crates/sico-ir/src/lib.rs:1463`、`:1477` |
| F6 | 治理债：§10（213 行）只在 w2 未提交，`sico-m22-step0546` 的 STEP 引用 §10.1/§10.2 **悬空**；两棵 scratch 树与主树各持分叉 `codegen.sico`；STEP-0279–0283 重复编号；电池三坑（`__pycache__` 指纹、`include_str!` mtime、封装器未入库）；CI 20 ms 抖动未隔离 | `E:/github/sico-w2/AGENTS.md`（未提交）、`sico-m22-step0545/0546`、`tools/validate-m22-s5*.ps1`、`runner/sico-runner/tests/runner.rs:1471`、run #55 failure |

## 3. 重排（摘要；全文见报告 §5）

| 阶段 | 一句话 | 片数 |
|---|---|---|
| P0 尺度预检 | 同一 payload 比较「左折叠 concat」与「分片 list + join」在 64 KiB / 256 KiB / 1 MiB 的 fuel/wall/trap | 1 |
| P1 装配重写 | hex 段落改分片 + 一次 `text.join`，字节输出不变 | 1–2 |
| P2 批量复算 | 夹具上限提到 128 fn / 1 MiB，一次重跑 396 站点 | 1 |
| P3 具名残差 | `refused-guest` 5 站 + `checked_sub/mul/div` + T3–T20 中确实出现的族 | 3–6 |
| P4 语料与 bundle | W3-a/W3-b/W4/W5 | 4–8 |
| P5 自举与预算 | `A==B==C`、`.sapp`、hostile limit+1、`B(S)` 全套 + M19 对照 | 3–5 |
| P6 出口 | 干净唯一候选、M0–M21 回归、独立 CI、七门复审 | 2–3 |

**决策门**：D1（P0 后）—— 分片+join 在 1 MiB 内绿则进 P1；仍红则转 RFC 或按 R3 止损。D2（P2 后）—— 若 219 站回绿 <25%，判结构性，三选一（继续 / 合同变更 / 止损）。

## 4. 需要 owner 裁决的三件事

1. **是否立刻执行 P0**（1 片，只加测试、不改 product 代码）。
2. **ADR-0017 Option A（记录化重基线）是否显式挂起到 M25 之后**：现在重基线会重写冻结 S 的源文本，作废 149/149 census 与全部逐站 `source_sha256`/`ir_sha256`/`component_sha256`。
3. **G1（§10 入库）与 G2（退役两棵 scratch 树）是否与本片同批提交**：这两项不做，P1 会在三份分叉的 `codegen.sico` 上施工。

## 5. 验证

| 检查 | 命令 | 结果 |
|---|---|---|
| 计划合同校验 | `.\tools\validate-step-0124.ps1` | 见运行记录 |
| 空白/行尾 | `git diff --check` | 见运行记录 |

**未跑**：整轮电池 `tools/validate-m22-s5.ps1`（本片为零代码改动的文档片）、任何 `cargo` 测试、P0 尺度预检（尚未执行）。

## 6. 诚实残差

- P0 的「分片+join 在 1 MiB 内可行」目前是**基于 STEP-0516 S3 线性度 + `emit_join` 实现**的推断，**尚未实测**；这正是 P0 要先跑的原因。
- 3.3 的量级账是估算（按 hex 表示 2 MiB 文本、逐字节 concat 计），不是读数。
- P0–P6 的片数是**工作量级估计**，不是时间承诺（沿用 §10.1.1 措辞）。
- 本片未复核 396 站点的逐条判据，只读账本给出的 verdict 直方图。
- 门状态一律未变；M22 仍是 NO-GO。
