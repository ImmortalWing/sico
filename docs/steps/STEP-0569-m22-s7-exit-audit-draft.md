# STEP-0569 m22-w2 **S7 出口审计草案（纯文档片）**

| 项 | 值 |
|---|---|
| 日期 | 2026-11-06（Asia/Shanghai） |
| 分支 / 树 | `codex/m22-w2` @ `a939862`；工作树含 1 个**非本片**的既有脏文件（`selfhost/match-component-coverage-v1.json`，4 处 verdict 由 `component-equal` 改 `FuelExhausted`）⇒ 本片不碰 |
| 片型 | **纯文档片（S7 出口审计草案）**：逐门起草门 1–7 的当前裁决 + 证据指针 + 剩余缺口；`crates/*`、`runner/*`、`selfhost/*`、`wit/*`、`tools/*`、`docs/plans|STATUS|handoff|ROADMAP|rfc|adr|steps README` **全部零改动** |
| 交付物 | 报告 = [`docs/reports/m22-exit-audit-draft-2026-11-06.md`](../reports/m22-exit-audit-draft-2026-11-06.md)；本 STEP 文档 |
| exit | **达成（草案目标）**：七门逐门裁决已成文；结论 **M22 overall NO-GO 保持**；不翻任何门、不改门文本与证据等级、不做完成声明 |
| 不翻门 | 门 1/5 GO（沿用 10-25 审计）、门 2 PARTIAL、门 3 NO-GO、门 4 NO-GO（不得沿用）、门 6 **GO 已失效（本草案收紧）**、门 7 不得给完成信用 —— **M22 整体仍 NO-GO** |
| 证据等级 | `internal-fixture`：全程只引用**已执行**的 STEP/STATUS 实测；本片自身仅复跑 1 个廉价探针（超时未完成，如实登记） |

## 1. 入口门槛（为何本片此时可做）

- AGENTS.md §10.9（R6）要求「新建出口审计逐门裁决 1–7，附证据矩阵和剩余缺口」。10-25 已有一份权威审计 [`m22-exit-audit-2026-10-25.md`](../reports/m22-exit-audit-2026-10-25.md)，但其覆盖止于 STEP-0543。
- STEP-0544–0561 产生了**大量新的已执行证据**（fuel 归因 + B3/B4 三刀负结果、W4 verdict parity、声音洞修复、发射词表补刀），且 STEP-0566 实测出**独立 CI 触发面断裂**这一门 6 关键事实 ⇒ 需要一份**叠加最新实测**的审计草案，供 owner 复审取材。
- 本片入口门槛 = ①不修改任何 gate 文本 / 证据等级 / 门状态（不翻门）；②只引用已执行证据，不引入新主张；③不确定处标「未核实」；④不碰评审者维护的文档与热文件；⑤不 commit / 不 push。

## 2. 草案裁决摘要（详见报告 §1）

| 门 | 裁决 | 一句话依据 |
|---|---|---|
| 1 L1 formatter/checker | **GO（限声明子集）** | formatter canary 30/30 + `full_source_exit: 0`；checker 电池 exit 0（10-25 审计，本片未重跑） |
| 2 L2 冻结语料逐字节 | **PARTIAL** | 组件轴 **36/396**；残差 331 `FuelExhausted` + 24 8MiB trap + 5 refused-guest；W4 verdict parity **147/150**（3 燃料墙） |
| 3 自举闭环 `A==B==C` + `.sapp` | **NO-GO** | guest 尚未产出组件 B；三阶段字节证据 / `.sapp` 打包 / `sico-app` 自编译 / hostile 全部未演示 |
| 4 性能预算 `B(S)` | **NO-GO** | 现有只有 IR 层观测（25.93 s）；`B(S)` 未实测 ⇒ cold/warm wall、fuel 读数、峰值内存、hostcall、M19 对照全缺 |
| 5 双实现登记 | **GO** | Rust 保持唯一 byte oracle（`sico-codegen-wasm` 未被 W2 改动）；两实现均未删除/降格 |
| 6 M0–M21 回归 + 独立 CI | **GO 已失效** | 旧锚点 run #52（SHA `9f0b7a3`）早于 STEP-0544–0561 全部改动；触发面在 STEP-0566 前是断的（`git ls-remote` 实测）⇒ 当前活动线无任何独立 CI |
| 7 声明↔执行证据映射 | **不得给完成信用** | §10.11 未闭合项（4 既有红、语料覆盖 0、整文件自举、M-g* 站点、W4 精确 parity）仍在 |

**贯穿性事实（报告 §2）**：最近三片实测**证伪**「fuel 墙在文本累加 / 索引 / 游标重扫」，静态可读路径解释不了 2.4e9 fuel ⇒ **fuel 墙根因未定位**，同时压门 2（331 站点不回绿）、门 3（B(S) 前置硬阻塞）、门 4（`B(S)` 无法实测）。

## 3. 本片实测证据（诚实记录：只跑成 1 项，且该项超时）

本片为纯文档片，自身只做**廉价复验**，不重跑整轮电池（历史全量 ledger ≈6114 s，超出本片预算）。

### 3.1 环境与预构建产物探测（实测，exit 0）

```
$ ls runner/sico-runner/target/debug/deps/selfhost_compiler-*.exe
runner/sico-runner/target/debug/deps/selfhost_compiler-3ee9bc92fdb5f988.exe
runner/sico-runner/target/debug/deps/selfhost_compiler-4fa1b02d73be6796.exe
$ ./target/debug/sico.exe --version
sico 0.1.0        (exit 0)
$ which powershell.exe
/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
```

⇒ 复跑工具链可用（预构建 `sico.exe` + 两个 `selfhost_compiler` 测试二进制 + PowerShell 均在位）。

### 3.2 W4 corpus-parity 探针复跑（**超时，未完成 —— 诚实登记**）

```
$ powershell.exe -NoProfile -File tools/probe-m22-corpus-parity.ps1
（后台运行，560 s 后被我方 timeout 掐断）
EXIT=124        # 124 = timeout(1) 的超时退出码，非探针自身结论
```

- **结果**：全量 150 项在 560 s 内**未跑完** ⇒ 本片**未能独立复验** W4 的 147/150 parity。报告中该数字**只作为「已执行证据」引用**（[STATUS 2026-11-05 STEP-0561 条](../../STATUS.md)、[STEP-0561](./STEP-0561-m22-w2-w4-verdict-parity.md)、[STEP-0558](./STEP-0558-m22-w2-w4-missing-field-soundness-fix.md)），**不作为本片的复验成果**。
- **附带观察（不作归因）**：该探针 150 项需 >560 s，而其中 3 项本身是 `scope-nesting-limit.sico` ×3 的燃料墙（FuelExhausted exit 125）⇒ **全量 parity 探针本身受燃料墙拖累**，与报告门 2 缺口 3 一致。**未核实**：未单测每一项耗时，不知 560 s 主要花在 3 个燃料墙上还是在 150 项总和。
- **未跑廉价子集**：探针支持 `-Limit N`（构建期子集，STEP-0561 记 12 项 + 2 项全 match）；本片为省时未再跑 `-Limit` 子集 ⇒ 子集侧**未独立复验**。

### 3.3 未跑项（如实登记，不包装）

| 验证器 | 为何未跑 | 影响 |
|---|---|---|
| 396 站点 ledger（`selfhost_compiler ... frozen_match_site_ledger`） | 历史全量 ≈6114 s（[STATUS 2026-11-05 STEP-0558 条](../../STATUS.md)），超本片预算 | 门 2 组件轴 36/396 + 331/24/5 残差**未独立复测**，只引用已执行记录 |
| `tools/report-m22-canary.ps1`（门 1 formatter canary） | 门 1 沿用 10-25 审计，本片不推翻 | 门 1 GO **未独立复验**，只做指针引用 |
| `tools/validate-step-0245.ps1`（门 1 checker 电池） | 同上 | 同上 |
| `tools/run-ci.ps1` + 独立 CI | 门 6 需干净已提交 SHA + push（owner 授权），本片不 commit/push | 门 6 无新证据；「GO 已失效」是基于 STEP-0566 实测的**推断性收紧**，非 owner 已裁决翻门 |
| `cargo test`（4 个既有红 `step0507/0519/0520/0526`） | 纯文档片不触碰 runner 测试；且本机可能缺 gcc ⇒ runner 重编译有失败风险 | 4 既有红**未复跑**，只引用 [STEP-0559](./STEP-0559-m22-w2-callshape-u64-list-get-frontend-gap.md) 的诊断 |
| `git diff --check` | 本片新建 2 个 md 文件，无空白错误风险；未跑 | 未跑项 |

## 4. 诚实收窄（本片不主张什么）

- 本片是**草案**，**不是权威裁决**；权威裁决仍是 10-25 审计。凡与 10-25 冲突处（仅门 6 一处），本草案**以更严者为准**并显式说明，但**不代替 owner 复审**。
- 门 1 / 门 5 的 GO 是**沿用**，非本片复验；门 2 的所有数字是**引用**已执行记录，非本片复测；门 3 / 门 4 的 NO-GO 是**引用** 10-25 审计的「未演示 / 未实测」事实。
- 门 6 的「GO 已失效」是本草案**基于 STEP-0566 `git ls-remote` 实测 + run #52 SHA 早于后续改动**这两条已执行事实做出的收紧推断；**未取得新 CI run**，故不宣称门 6 现为 NO-GO 的正式裁决，只宣称「10-25 的 GO 不可沿用」。
- fuel 墙根因**未定位**：本片只综合 STEP-0555/0556/STATUS 复核条的**已执行证伪结论**，不提出新的根因主张。
- 3 个 W4 燃料墙项、24 个 8MiB trap、5 个 refused-guest 均**未做归因**，只登记现象。
- STEP-0560 补的 `sico.list.length[U64]` 是否真闭合那 7 个站点：**未核实**（组件轴逐站差分未跑，发射能力 ≠ 站点闭合）。

## 5. 结论

- 七门逐门裁决已成文，**M22 overall 保持 NO-GO**，不翻任何门。
- 相对 10-25 审计，本草案的**唯一收紧**是门 6：旧锚点 run #52 覆盖不到 STEP-0544–0561 的实现改动，且触发面在 STEP-0566 前断裂 ⇒ 10-25 的门 6 GO **不可沿用**；重建门 6 GO 需在干净已提交候选 SHA 上重跑一键 CI 并取得该 SHA 的独立 `completed/success`。
- 通往 GO 的关键路径仍是：**定位 fuel 墙**（解 331 站点 + `B(S)` 前置）→ 门 3 自举闭环 + `.sapp` → 门 4 `B(S)` 完整预算 → 门 2 语料/bundle 收尾 → 门 6 干净 SHA 独立 CI → 门 7 证据映射 + 支持面登记表。