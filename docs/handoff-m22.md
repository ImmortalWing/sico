# M22 交接手稿（living handoff，合并基线：`dev` 线 STEP-0283 + `codex/m22-w1-ci` 线 STEP-0324 / M24 STEP-0337）

> - 更新：2026-10-20（`codex/m24-vision-ui` 合并入 `dev` 完成：两线事实并集，重复 STEP 编号并集登记未重编号）
> - 前一份：[`handoff-m22-line-tokens.md`](./handoff-m22-line-tokens.md)（STEP-0260 基线，已被 0278 版取代，同批删除）
> - 权威事实仍以 [`docs/steps`](./steps/README.md) / [`docs/STATUS.md`](./STATUS.md) 为准；本文只管「下一刀从哪下、哪些验证还没闭环」
> - 平行线注记（合并前原文保留）：`origin/dev` 已前进至 `550059a`，包含另一套 STEP-0279–0283 W1/W2 历史，与 `codex/m22-w1-ci` 线源码及 STEP 编号冲突，不得覆盖或隐式合并 —— **本次合并已按此要求逐项处理**（重复编号并集登记未重编号、语料保留 `dev` 线 235 条全集、`compiler_semantics.sico` 取 `dev` 已验证实现），**但仍不得把该线 CI 结论当作合并后 `dev` 的验证结果**。
> - 权威裁决：[STATUS](./STATUS.md)、[M22 计划](./plans/M22-compiler-self-host.md)、[STEP-0292](./steps/STEP-0292-m22-w1-independent-ci.md)、[STEP-0293](./steps/STEP-0293-m22-w2-u64-to-text-while-rhs.md) 与 [STEP-0294](./steps/STEP-0294-m22-w2-map-get-match-subject.md)。本手稿只提供恢复路径。

## 0. 当前状态

M22 处于 **R 阶段后半 + W2 已收官**：R0 已拍板（Option A，ADR-0017），
records v0 全封（STEP-0271..0275），**W1 合并债四片全清已关闭**
（0278/0279/0280），**W2 三片全落地已收官**（0281 indent structural /
0282 records 重授权 / 0283 E2031 identity）：**DIVERGENCE_WAIVER 清零
（机制留空表），分区 (116,46,73,0)——235 条语料全部 strict，本机零豁免
全量探针 235/235**，R3 止损计数停止。
**下一波次 = S3/S4 records re-baseline；W2 收尾小项 = record-nested
多行字面量 + EC-3 canonical IR JSON 快照。**

## 1. 本批已落地（自 fd802c4 / STEP-0262 起；0279–0283 五批已提交推送）
M22 整体 **NO-GO**。S1/S2 声明子集完成，S3/S4/S5 partial，S6/S7 未进入。R0 已接受 ADR-0017 Option A 与 RFC-0047。STEP-0292 对 `e8495f9` 的独立 Windows GNU [GitHub CI](https://github.com/ImmortalWing/sico/actions/runs/36087222284) 成功，**W1 在本隔离分支快照裁为 GO**。该裁决不覆盖 `origin/dev` 的并行历史。

STEP-0293 是 22-C 首个 R3 计数实现片：while 体 sico.u64.to_text RHS 与 Rust oracle byte-exact，错误参数类型 typed 拒绝；formatter canary 22/30，nearest_match 从 GWPACK-OTHER 移至 CALL-TARGET，整份 exit 122。cb371c5 的独立 Windows GNU CI 运行 36090403437 成功，S1 于本隔离分支裁为 GO。R3 连续停滞数 0。

STEP-0294 是 22-C S2：`sico.map.get[Text,U64]` 的受限 match 主语与 Rust oracle 字节一致；错误 Map/key 类型、参数数和泛型数 typed 拒绝。formatter canary 仍 22/30，`nearest_match` 前沿 CALL-TARGET → STATEMENT，整份 exit 122，R3 连续停滞数仍为 0。本机 0261/0262/0245 全绿，`5a6d0bb` 的独立 Windows GNU [CI 运行 36147743130](https://github.com/ImmortalWing/sico/actions/runs/36147743130) 成功，**S2 于本隔离分支裁为 GO**。

## 1. 证据与边界

| 检查 | 当前结果 | 证据边界 |
|---|---|---|
| 0264 | M22–M26 路线重划（R 阶段收敛门 + R3 止损预登记） | planning-only |
| 0265 | 重启评估（不回退，分层重启；W1/W2/W3 波次） | [`M22 restart assessment v1`](./plans/M22-restart-assessment-v1.md) |
| 0266/0268 | R1 canary/燃料仪器化实测；R0 推荐修订（Option A） | tools/report-m22-canary.ps1 等 |
| 0267/0269/0270 | ADR-0017 起草 → RFC-0047 草案 → accepted + EC-4 双普查冻结 | ADR/RFC 文档 + census json |
| 0271–0275 | records v0 全封：EC-1 语料/前端、ABI 扁平化 + `Module.records`、EC-2 拒绝面 + EC-5 矩阵、D5 `List[record]` 列式单态、for 迭代 + 深度核对 | record_types.rs + 11 个 end-to-end 语料；三 crate test/clippy/fmt 绿 |
| 0276 | WIP 吸收（`Map[K,V]` 参数面 +228 行）+ guest `records` 表发射修复（6 新函数） | 本机 CLI 微分：formatter 前缀 14/16、driver --emit-ir 16/16、拒绝面 3 例 |
| 0277 | W1 四债计划 + 实测侦察 | planning-only |
| 0278 | W1 片 A+B：sha 门验证关闭；legacy 三件套退役（删 947 行 + 3 单测；0215/0216/0219 改退役登记；0243-0245 suite 同步） | 本机引用清扫/ps1 语法/check 绿；CI 待裁决（随 0e4cbb9 已推） |
| 0279 | W1 片 C：E7002 关键词指纹 → 结构性判定（versioned payload + if `==` 深度配对守卫 + 任意行 apply）；rename-safety 语料 3 条（oracle 实取）；语料 215→230（吸收 12 条 record-* 漏冻结）；8 条 diverge 豁免 | 本机 230/230 探针绿（197s）；CI 待裁决 |
| 0280 | W1 片 D（末片）：0-indent 语料先行——实测 oracle 顶格语句照常语义分析、guest 静默跳过（false-accept）；structural 方向登记 W2（typed refusal 否决）；语料 230→235；waiver 合并 DIVERGENCE_WAIVER(10)；**W1 关闭** | 本机 235/235 探针绿（202s）；CI 待裁决 |
| 0281 | W2 片 1：indent structural 修复——semantics 第二遍顶格非声明行按语句处理（process_body_line + body 检查块搬出 if/else）；IND-005→strict E7002、IND-004→E4001（首码 E2031 待补）、waiver 10→9、分区 (116,38,72,9) | 本机 235/235 零回归（208s）；CI 待裁决 |
| 0282 | W2 主片：records 重授权——裸字段注册（bare_field_line）+ `field` 形放宽 parts>2（修 `List[Text]` 字段静默丢失）+ E2022 + local 类型表 + E2023 dot_before_equal + let 行 E2021(count_field_label)/E2010/E2001 + E2024(words==3) + E2011 dot_access_field（子串+词边界 at_word_boundary+引号外 outside_quotes）；误伤面三次实证修复；8 条豁免全关、waiver 9→1、分区 (116,45,73,1)；途中修复 0280 批次测试不自洽（语义臂补 E1001） | 本机 strict 235/235（263s）+ 12 条 record 逐一 12/12；selfhost_checker.rs rustfmt 绿；CI 待裁决 |
| 0283 | W2 收官片：IND-004 首码 E2031 identity——`use` 行注册 imported_roots + move/borrow 形注册 resource 型 local + E2031 body 检查（内建/形参/local/enum/record/nominal/imported 七重排除 + has_dot_call 词边界〔前字节非`.`〕+引号外+`(`）；两处误伤实证修复（`sico.text.encode` 中段组件 → at_word_boundary 收 `.` 前字节；resource 型 local 被 E2011 误扫 → E2011 仅扫 record 型）；DIVERGENCE_WAIVER 1→0（机制留空表）、分区 (116,46,73,0)——235 条全部 strict，**W2 收官** | 本机零豁免 235/235（296s）+ 关键 8 条 8/8；CI 待裁决 |
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

代码面汇总（0279–0283 批已提交）：`selfhost/compiler_semantics.sico`
（E7002 结构性 + records 全族：第一遍裸字段/field 放宽/E2022，第二遍
local 表/E2023/E2021/E2010/E2001 let 形/E2024/E2011，新 helper
bare_field_line/word_byte/letter_byte/dot_before_equal/count_field_label/
at_word_boundary/outside_quotes/dot_access_field/record_lacks_field）；
8 个新语料源（3 revision + 5 indentation）；
`selfhost/corpus-v0.json` + `script-build-corpus-v0.json` 重冻结（235 条）；
`runner/sico-runner/tests/selfhost_checker.rs`（235 + DIVERGENCE_WAIVER(1)
+ 分区 (116,45,73,1) + 语义臂补 E1001/E2021–E2024）；
`bootstrap_bundle.rs` 235；STEP-0279/0280/0281/0282/0283 文档 + README + STATUS。

## 2. 下一步

1. 按 [STEP-0291](./steps/STEP-0291-m22-nearest-match-design.md) 开始 S3：match 臂内嵌 if/return 的控制流降低，重新钉定 `nearest_match` 字节差分与下一个前沿，并取得本片独立 CI。
2. 合并 `origin/dev` 的并行历史前的三项处理（重复 STEP 编号 / 235 项语料 / `compiler_semantics.sico` 差异）**已于 2026-10-20 完成**，裁决见本文件顶部与 [`steps/README.md`](./steps/README.md) 表后登记；**CI 纪律不变**：不得把 `codex/m22-w1-ci` 线的 CI 结论声称为合并后 `dev` 的验证结果，合并后仍需重新取得独立 CI 裁决。
3. M23 的 RFC-0048/0049、M24 的 RFC-0050/0051/0052 与 ADR-0018、M26 的 RFC-0053/0054 仍各自等待 owner 接受；它们不阻挡本分支 22-C S3。

**本机已绿**：sico check 集成链；checker 组件构建成功；**235 条全量探针
零豁免（WAIVER 已清零）strict 235/235**（预构建 runner CLI 8 路并行约 263 s；驱动
`target/w1c-probe/run_corpus230.py`，结果 `results235.json`）；12 条
record 语料逐一核对 12/12；validate-step-0124 + git diff --check；
selfhost_checker.rs rustfmt --check 绿。

**待 CI 裁决（本机跑不了，勿在本地重试白等）**：
1. runner 测试仓全量（selfhost_checker 235 条 + waiver 首跑、
   bootstrap_bundle 235）——本机缺 gcc/dlltool（ring/windows-sys 编译
   失败，9-20 产物指纹与今构不一致会触发全量重编译），**不是代码问题**；
2. 改写后的 0215/0216/0219（首次以「退役登记 + selfhost_checker」模式跑 CI）；
3. 0243/0244/0245 suite 列表变更后的 canary；
4. selfhost_compiler / selfhost_parser 全量断言（0276 只实测了代表子集：
   formatter 14/16——`close_code`/`direct_close` 为既有 GWPACK-OTHER 诚实
   拒绝、非 gated；driver 99 例中 16 例 + 拒绝面 3 例）。

## 3. 下一刀（按序）

1. **S3/S4 records re-baseline**（W2 收官后的下一波次；SOA 不变式
   ADR-0016 在 re-baseline 前持续强制）。
2. W2 收尾小项：record-nested 多行字面量 + EC-3 canonical IR JSON 快照。
3. guest 降阶前沿（独立可做 R2 内容）：`close_code`/`direct_close`
   的 while-in-if（SKIP:GENERAL-WHILE-IF-REGION）。
4. 候选 declared-subset：parser 顶格行语法检查；match/try/E2001-E2020
   块（缩进行限定）扩到顶格行；E2031 裸调用（无点）与跨行调用近似.

## 4. 新发现（0279–0283 途中，已登记未扩大化）

- **0280 批次测试不自洽（0282 已修）**：zero-indent-recovery-declaration
  首码 E1001 落 `_` 臂（要求 Output），与 guest 实际拒绝矛盾；语义臂补
  E1001 关闭。教训：加语料后必须按分类逻辑复算分区，不能照抄旧数。
- **E2011 子串扫描的三重防线**：词边界（防 `lis**t.**append`）+ 引号外
  （防 `"input.txt"`）+ 弃用词邻接（防 `(origin, I64)` 误判）；字符串内
  转义引号未建模，登记 declared-subset 观察项。
- **关键字参数名缝隙**：参数命名 `record` 时 `sico check` 通过、script-v0
  lowering 报 `unsupported atomic expression`——check/build 暗隙候选，
  已规避（改 `record_name`），登记为 M14「check/build/run 无暗隙」gate 的
  候修项。
- **guest `words()` 词对近似**：字段访问判定用 `[P, text]` 词对近似
  oracle 的 `[P, Dot, text]` token 窗（`fresh . text`、
  `x.fresh.textual` 不可区分），当前冻结面无损，declared-subset 已写入
  STEP-0279 §2.1。
- **0-indent false-accept 类**：0281 结构性修复 + 0283 E2031 identity
  关闭，全部清零。
- **E2031 已知近似**：裸调用（无点）unresolved target 未实现（语料无
  首码用例）；跨行调用未建模——declared-subset 观察项。
- **CI 触发面缺口（2026-10-21 登记，阻塞合并后 CI 裁决）**：本仓唯一工作流
  `.github/workflows/m22-w1-ci.yml` 的触发条件是 `push: branches:
  [codex/m22-w1-ci]` 与手动 `workflow_dispatch`——**push 到 `dev` 不产生任何
  CI run**（GitHub API 实测：`head_sha=95b80cd` 的 workflow run
  `total_count=0`，`branch=dev` 历史 run 亦为 0）。⇒ 合并 `codex/m24-vision-ui`
  后的 `dev`（`95b80cd`，已推 origin/github）**没有 CI 证据**，M22 出口审计
  要求的 exact-SHA 独立 CI 裁决仍然缺；关闭条件 = 对 `dev` 手动 dispatch
  `M22 W1 independent CI`（workflow_dispatch 在所选 ref 上跑工作流，即合并后
  的树），或向 `codex/m22-w1-ci` 推提交走自动路径。本机无 `gh` CLI 且不代用
  owner 凭据，该步由 owner 执行。本条**不改任何 gate 数值与证据等级**。
- **待 owner 裁决、已挂起至 M22 完成的两项（2026-10-21 登记）**：
  ① **重复 STEP 编号 0279–0283**：两条并行线各指一份不同的执行记录，现为
  「两行并存 + 标注归属」登记、**未重编号**（登记见
  [`steps/README.md`](./steps/README.md) 表后）；重编号属 owner 裁决项
  （AGENTS.md §3「每个非平凡实现步需唯一 STEP」）。
  ② **`dev` 线既有红测的普查数口径**：语料 215→235 重冻结后，硬编码普查数
  未同步——9 个根工作区目标（sico-ai-tools / sico-cli / sico-format /
  sico-hir / sico-ir×2 / sico-lexer / sico-parser / sico-semantics）期望
  `syntax-candidates/b` = 58（实为 66）、能力面 (20,5)（实为 (23,5)）、33 条
  invalid（新增 IND-003 已越过 IR support dispatch）、revision oracle 4（实为 7）；
  runner `selfhost_formatter` 期望 accepted=99/refused=116（清单实为 118/117）。
  已在**合并前 `550059a`** 的独立 worktree 上实测复现同值 ⇒ **非合并引入**。
  两项按 owner 指示「等 M22 完成后再定口径」：在此之前**不改数、不扩大化、
  不掩盖失败**。

## 5. 环境备忘

- `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；cargo =
  `$USERPROFILE/.cargo/bin/cargo.exe`；CLI 在 `target/debug/sico.exe`。
- runner 测试仓本机不可 cargo 构建（gcc/dlltool 缺失；self-contained 的
  x86_64-w64-mingw32-gcc 无 `as`，dlltool 无法建 import lib——勿再试）。
- `sico check` 不能直查 module 文件：用入口
  `target/debug/sico.exe check selfhost/checker.sico`（会带上 parser 模块）。
- runner CLI 预构建可用：`runner/sico-runner/target/debug/sico-runner.exe`
  （`--fuel 5000000000 prog.component.wasm`；诊断码在 **stderr** 的
  outcome JSON `detail` 里，且引号是 `\"` 转义的）。
- checker 组件构建：`sico build --profile script-v0 selfhost/checker.sico`
  → `selfhost/checker.component.wasm`（CLI 拒绝覆盖，先删旧文件）。
- 语料冻结：改/加语料源后跑 `tools/update-m22-corpus.ps1`（本机无 rg——
  需临时把 `rg --files` 行换成 Get-ChildItem 枚举，生成后还原；枚举结果
  与 git tracked+untracked 集合核对一致即可）。corpus-v0.json 逐入口
  bytes+sha256 由 selfhost_checker 守卫。
- 语料探针驱动：`target/w1c-probe/run_corpus230.py`（断点续跑
  results235.json，8 路并行约 263–296s/235 条；WAIVER 集已清零，strict
  模式）。
- 分区复算：改语料/测试后先用 python 按分类逻辑独立复算
  (lexical, semantic, accepted, unsupported) 再写断言（0282 教训）。

## 6. 纪律提醒

- 不主动 commit/push（AGENTS.md §9）；0278 批（0e4cbb9）与 0279–0283
  五批均系 owner 显式指令进库并推送。
- 冻结合同（语言/WIT）动前走 RFC；跨组件架构动前走 ADR。
- W2 收官前 S3/S4 源层维持 SOA 不变式（ADR-0016 仍强制）。
## 7. 本机复核命令（`codex/m22-w1-ci` 线原 §3「本机复核命令」，合并后随文保留）

> 本节原为 `codex/m22-w1-ci` 线手稿的 §3（STEP-0283 文档中的「handoff §3.1」指向本节）；合并后编号为 §7，原编号保留在标题内以便交叉引用解析。

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
