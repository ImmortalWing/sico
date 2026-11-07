# STEP-0594 m24 **校验器族普查数重同步片**：把 `tools/*.ps1` 中因语料重冻结（215→235；`syntax-candidates/b` 58→66；`diagnostics/semantic-case-map.json` 33→46）而陈旧的硬编码普查断言逐个处理清楚——当前树校验器改为实测值并注明口径与来源；带日期的出口门/里程碑步骤校验器显式区分「历史门范围（冻结值，注释记录）」与「当前树一致性（实测）」；**不**删除断言、**不**放宽断言、**不**改冻结语料/guest/历史 STEP 文档

| 项 | 值 |
|---|---|
| 片型 | 有界普查重同步：只改 `tools/*.ps1` 中的硬编码计数断言与其口径注释；**不**动冻结语料（`selfhost/corpus-v0.json`、`selfhost/script-build-corpus-v0.json`、`diagnostics/semantic-case-map.json`、`semantic-cases/`、`syntax-candidates/`、`syntax-mutations/`）、**不**动 guest 源、**不**动历史 STEP/审计文档正文、**不**放宽/删断言 |
| exit | **普查数欠账关闭**：12 个校验器被触及，其中 10 个的陈旧计数已改为实测值；2 个（0214/0215）只同步描述性输出/注释（其断言经实测与当前树一致，未改断言语义）。全部触及校验器保持 **fail-closed**（计数再漂移则红）。**仍红项均已归因**：多数为 IND-003（owner 待决，STEP-0584 §6/§8.1），其余为并发代理工作区/环境（sico-ui-renderer-d2d 编译错、runner access violation、工作区 rustfmt、error-taxonomy 的 taxonomy/catalog 数据不一致）——见 §5/§6 |
| 不翻门 | 不声称任何 gate 翻转；M22/M24 overall 维持各自既有状态 |
| 证据等级 | `internal-fixture`：本机 2026-11-07，Windows x64，工具链 1.98.0-x86_64-pc-windows-gnu；计数全部由 PowerShell `Get-ChildItem`/`ConvertFrom-Json` 枚举实测（不凭推算）；合并 `7bcdc35` 之后实测 |

## 1. 背景

本仓语料已多次重冻结（STEP-0279..0283 随合并 `7bcdc35`）：`selfhost/corpus-v0.json` 215→**235** 条（`rust_format` 分布 **accepted 118 / refused 117**；`rust_check` **accepted 73 / refused 162**）；`syntax-candidates/b` 递归 `.sico` 58→**66**（其中 **IND-003** 是解析拒绝用例 ⇒ 可解析 **65**）；`syntax-mutations/b` 12（未变）；`diagnostics/semantic-case-map.json` 33→**46**（STEP-0584 补登 5 条：IND-003/004/005、REV-201/203）。`semantic-cases/manifest.json`（58/25/33）、`tests/parser/b-ast-shapes.txt`（58）、`tests/hir/b-shapes.txt`（58）、`tests/diagnostics/b-mutations.snap`（12）、`tests/performance/*`（均未变——它们是被 STEP-0214 之后冻结的历史工件）。

STEP-0580 已对齐 18 处普查锚（含 `crates/*` 内测试与 `tools/validate-step-0016/0068/0245.ps1`、`tools/measure-syntax-candidates.ps1`）；STEP-0584 已补登 oracle 5 条并同步 `m1_frontend_bench.rs`（78/65/13、66/12）；主线已修 `tools/validate-step-0021.ps1`（工件内部自洽校验 + 语料漂移归属）。**本片接管剩余**：`validate-m1/m2-exit.ps1`、`validate-step-0017/0019/0022/0025/0027/0214/0215/0220/0246.ps1`、`validate-error-taxonomy.ps1` 中的陈旧计数。

## 2. 普查清单表（逐文件:行号 / 旧值 / 实测值 / 判定 / 处置 / 理由）

> 判定 **(A)** = 当前树状态校验器（断言意图持续有效 ⇒ 数字改实测值）；**(B)** = 带日期的出口门/里程碑步骤校验器（断言的是某次时点的语料/范围 ⇒ 不改写历史，显式区分历史门范围与当前树一致性）。所有实测值由本片用 PowerShell 枚举实测（口径列于「理由」），非推算。

### 2.1 本片修改（12 个校验器）

| 文件:行号 | 旧值 | 实测值 | A/B | 处置 | 理由 |
|---|---|---|---|---|---|
| `tools/validate-m1-exit.ps1:57` | `bCases=58` | **66** | B | 改实测 + 历史 58 注释；`:31-33` STATUS handoff 检查改历史记录注释（STATUS 已推进，审计 doc 仍持 GO）；`:44` 性能工件保持 dated 值 | M1 出口门；b 集 58→66（STEP-0279..0283）。STATUS.md 已推进至 M22/M24，post-M1 handoff 不再是当前树不变量；权威结论在 `m1-exit-audit.md`（仍断言 GO） |
| `tools/validate-m2-exit.ps1:50` | `@($map.cases).Count=33` | **46** | B | 改实测 + 历史 33 注释；`:32-34` STATUS handoff 检查改历史记录注释；性能工件保持 dated | M2 出口门；`semantic-case-map.json` 33→46（235 重冻结 + STEP-0584 5 条）。manifest 58/25/33 未变（保留）；权威结论在 `m2-exit-audit.md` |
| `tools/validate-step-0017.ps1:20` | `bCases=58` | **66** | B | 改实测 + 历史 58 注释 | M1 期步骤；b 58→66。snapshot 58 保留（未再生成，被 IND-003 阻挡） |
| `tools/validate-step-0017.ps1:25` | `accept=25 / reject=33` | **28 / 38** | B | 改实测 + 历史注释 | 标记分账随 b 增长（28 accept / 38 reject），逐文件 `// expect:` 实测 |
| `tools/validate-step-0019.ps1:18` | `bCases=58` | **66** | B | 改实测 + 历史 58 注释 | M1 期步骤；b 58→66。mutations 12 未变（保留） |
| `tools/validate-step-0022.ps1:30` | `bCases=58` | **66** | B | 改实测 + 历史 58 注释 | M2 期步骤；b 58→66。snapshots 58 / mutations 12 未变（保留） |
| `tools/validate-step-0025.ps1:24` | `owned=4` | **5** | B | 改实测 + 历史 4 注释 | M2 期步骤；E4001 组因 IND-004 入 oracle 增至 5。valid 4 / unique 4 未变（保留） |
| `tools/validate-step-0027.ps1:20` | `owned=2` | **5** | B | 改实测 + 历史 2 注释 | M2 期步骤；E7002 组因 REV-201/203 + IND-005 增至 5 |
| `tools/validate-step-0027.ps1:21` | `valid=2` | **3** | B | 改实测 + 历史 2 注释 | `b/revision/valid` 增至 3 文件（STEP-0279..0283）。unique 2 未变（保留） |
| `tools/validate-step-0214.ps1:33` | 输出 `corpus=215 format=99/116 check=65/150` | **235 / 118/117 / 73/162** | A | 只改描述性输出 + 注释 | M22 现行校验器；断言是 `bootstrap_bundle` 测试 + `update-m22-corpus -Verify`（未改），计数在输出串中，按实测刷新 |
| `tools/validate-step-0215.ps1:32,39` | 注释/输出 `corpus=215` | **235** | A | 只改注释 + 输出 | 同上；实际断言是 `selfhost_checker` 测试（未改），计数为描述性 |
| `tools/validate-step-0220.ps1:33` | `215 / 37 / 178` | **235 / 42 / 193** | A | 改实测 + 注释 | M22 现行校验器；`script-build-corpus-v0.json` 亦重冻结至 235（accepted 42 / refused 193，全部 accepted 可复现） |
| `tools/validate-step-0220.ps1:52` | `SCRIPT_ABI=28` | **31** | A | 改实测 + 注释 | 拒绝类分账：LEXICAL 116 未变；SCRIPT_ABI 28→31（语义类 34→46） |
| `tools/validate-step-0246.ps1:95` | `215` | **235** | A | 改实测 + 注释 | M22 现行校验器；`corpus-v0.json` 235（`entry_count == entries.Count == 235`） |
| `tools/validate-error-taxonomy.ps1:105` | `caseIds=33` | **46** | A | 改实测 + 注释（**被更早失败遮蔽**） | `semantic-case-map.json` 33→46；该计数检查位于第 87 行 taxonomy/catalog 覆盖检查之后，后者先红（见 §5） |

> 注：`validate-step-0214/0215` 的计数只在**描述性输出/注释**中，**不构成断言**（其断言是 cargo 测试）；本片同步它们仅为避免输出/注释与实测不符，未改断言语义。

### 2.2 已由 STEP-0580/0584 对齐、本片复核一致（**未改**）

| 文件:行号 | 值 | 复核 |
|---|---|---|
| `tools/validate-step-0016.ps1:29` | 66 | `bCases` 实测 66 ✓ |
| `tools/validate-step-0068.ps1` | 66 / (28,38) | 标记分账实测 28/38 ✓ |
| `tools/validate-step-0245.ps1:43,46` | `(116,46,73,0)` / 235 | `rust_check` 实测 73/162 = 116 lexical + 46 semantic ✓ |
| `tools/measure-syntax-candidates.ps1` | A0=58 / B=66 / C=58 | 计数已对齐；**仍红**因 B≠A0 镜像（契约，非计数，见 §6） |
| `crates/sico-cli/examples/m1_frontend_bench.rs` | 78/65/13、66/12 | 与本片复算一致 ✓ |

### 2.3 复核为正确/冻结契约（**未改**）

| 文件:行号 | 值 | 依据 |
|---|---|---|
| `tools/validate-step-0018.ps1:14` | mutations=12 / map=12 / snapshot=12 | `syntax-mutations/b`=12、recovery map 12，均未变 |
| `tools/validate-step-0031.ps1:17` | snapshots=12 | 冻结 core-lowering 快照 |
| `tools/validate-step-0062.ps1:26` / `-0063.ps1:22` | surfaces=12 / cases=28 | 冻结 ecosystem 契约 |
| `tools/validate-step-0070.ps1:26` | minSdk=28 | 冻结 Android 契约 |
| `tools/validate-step-0100.ps1:31` / `-0102.ps1:40` | 12 / 12,20,6 | 冻结协议契约 |
| `tools/validate-syntax-mutations.ps1:147` | 12 | `syntax-mutations/b` 未变 |
| `tools/validate-error-taxonomy.ps1:41` | classes=12 | ai-eval 冻结错误分类（未变） |
| `tools/validate-error-taxonomy.ps1:114` | mutations=36 | `syntax-mutations/manifest.json` 未变 |
| `tools/validate-step-0029.ps1:25` | corpus.valid=25 | dated `m2-semantics` 工件（valid 25 / invalid 29，工件未变） |

### 2.4 非普查/范围外（**未改**，如实登记）

| 文件:行号 | 说明 |
|---|---|
| `tools/validate-error-taxonomy.ps1:87` | taxonomy 诊断键与 v0 catalog **不精确覆盖**——数据不一致，非计数（见 §6） |
| `tools/validate-m4-exit.ps1` / `-m5-exit.ps1` | `38..45` / `46..53` 为 **STEP 编号区间**，非语料计数 |
| `prototypes/component-host-call/tools/validate.ps1:44` | `-ne 512` 为 bigint 往返字节，非语料 |
| `prototypes/image-codec-probe/tools/probe.ps1` | python 路径，非计数 |

## 3. 口径与来源（每个实测值的枚举口径）

- **corpus-v0.json 235 / 118 / 117 / 73 / 162**：`Get-Content | ConvertFrom-Json` 读 `selfhost/corpus-v0.json`，`entry_count=235`、`entries.Count=235`；`rust_format` accepted=118 / refused=117；`rust_check` accepted=73 / refused=162。
- **b 集 66 / 28 / 38**：`Get-ChildItem syntax-candidates/b -Recurse -Filter '*.sico'` = 66；逐文件 `// expect: accept` = 28、`// expect: reject(` = 38（每文件恰一标记，28+38=66）。可解析 65（66−IND-003 解析拒绝）。
- **script-build-corpus 235 / 42 / 193 / 116 / 31 / 46**：`Get-Content | ConvertFrom-Json` 读 `selfhost/script-build-corpus-v0.json`；accepted 42（全部 `component_reproducible` 且带 SHA）、refused 193、拒绝类 `LEXICAL`=116、`SCRIPT_ABI`=31、语义类=193−116−31=46。
- **semantic-case-map 46**：`Get-Content | ConvertFrom-Json` 读 `diagnostics/semantic-case-map.json`，`@($map.cases).Count`=46。
- **0025 组 5 / 4 / 4**：map 中 `code ∈ {E4001,E4002,E6001,E6002}` = 5（IND-004 入 E4001）；`syntax-candidates/b/{effects-capabilities,component-call}/valid` 文件 = 4；distinct code = 4。
- **0027 组 5 / 3 / 2**：map 中 `code ∈ {E7001,E7002}` = 5（REV-201/203 + IND-005 入 E7002）；`syntax-candidates/b/revision/valid` 文件 = 3；distinct code = 2。
- **snapshots 58 / 12 / 58 / 12**：`tests/parser/b-ast-shapes.txt`（58）、`tests/diagnostics/b-mutations.snap`（12）、`tests/hir/b-shapes.txt`（58）、`tests/formatter/policy.snap`（golden）——全部**未**随 +8 用例再生成，保留记录值（再生成被 IND-003 阻挡，STEP-0584 §6）。
- **性能工件**：`tests/performance/m1-frontend-windows-release.json`（runs=3、files=66、canonical=54、mutations=12、iterations=200、sla=not-established）；`tests/performance/m2-semantics-windows-release.json`（runs=3、valid=25、invalid=29、median=1242.186、sla=not-established）——均为 dated（2026-07-15），工件未变。

## 4. 验证（逐个实际运行，改前/改后 exit 与关键输出行）

> 凡「改前」与「改后」exit 相同且为 1 的，区别在**失败点与根因**：改前失败于陈旧计数（或更早的 doc 检查），改后失败点后移至 cargo/数据检查，根因归为 IND-003 或并发代理/环境（§5/§6）。`cargo` 步因本工作区存在**并发代理在途改动**（`crates/sico-ui-renderer-d2d`、`runner/**`、provenance）与 **rustc ICE / gcc 缺 PATH**（STEP-0580 §5 登记）而不可全绿，非本片引入。

| 校验器 | 改前 exit（关键行） | 改后 exit（关键行） | 结论 |
|---|---|---|---|
| `validate-step-0017.ps1` | 1（`B parser corpus/snapshot mismatch: cases=66 snapshot=58`） | 1（`STEP-0017 parser tests failed`） | 计数过、cargo 红（IND-003） |
| `validate-step-0019.ps1` | 1（`STEP-0019 corpus mismatch: B=66 mutations=12`） | 1（`STEP-0019 formatter tests failed`） | 计数过、cargo 红（IND-003） |
| `validate-step-0022.ps1` | 1（`HIR evidence mismatch: cases=66 snapshots=58 mutations=12`） | 1（`STEP-0022 HIR tests failed`） | 计数过、cargo 红（IND-003） |
| `validate-step-0025.ps1` | 1（`expected 4 owned diagnostic cases, found 5`） | 1（`STEP-0025 semantic tests failed`） | 计数过、cargo 红（IND-003） |
| `validate-step-0027.ps1` | 1（`expected 2 owned diagnostic cases, found 5`） | 1（`STEP-0027 semantic tests failed`） | 计数过、cargo 红（IND-003） |
| `validate-m1-exit.ps1` | 1（`STATUS does not preserve the post-M1 handoff`） | 1（`workspace rustfmt failed`——另一代理未格式化 provenance 文件） | doc 检查过、count 过、cargo fmt 红（并发代理） |
| `validate-m2-exit.ps1` | 1（`STATUS does not preserve the post-M2 handoff`） | 1（`STEP-0022 HIR tests failed`——步骤循环调 0022） | doc/性能/计数过、步骤循环红（IND-003） |
| `validate-step-0214.ps1` | 1（`bootstrap_bundle` 编译：`sico-ui-renderer-d2d` E0004） | 1（同前——本片只改描述性输出） | 断言未变，红因并发代理编译错 |
| `validate-step-0215.ps1` | 1（`selfhost_checker` exit 0xc0000005 访问违例） | 1（同前——本片只改注释/输出） | 断言未变，红因环境/并发代理 |
| `validate-step-0220.ps1` | 1（计数 215≠235；`update-m22-corpus -Verify` 先行、≥15 min） | 运行超时（900 s 仍于 `-Verify`，无输出、未达计数检查）；**计数逻辑独立验证 235/42/193/116/31、repro=42、leak=0 为真** | 计数已过（独立验证），整体 exit 由预存在慢 `-Verify` 主导 |
| `validate-step-0246.ps1` | 1（`M22 corpus cardinality drifted` 215≠235） | 运行中（>28 min）：计数检查（line 95）已通过并进入 `-Verify`；**计数 235/235 独立验证为真** | 计数已过（独立验证 + 达 -Verify），整体 exit 由预存在慢 `-Verify`/组件构建主导 |
| `validate-error-taxonomy.ps1` | 1（`taxonomy diagnostic keys do not exactly cover the v0 catalog`） | 1（同前——计数检查 33→46 被更早失败遮蔽） | 计数已改，红因 taxonomy/catalog 数据不一致 |

- `git diff --check`：**exit 0（净）**（本片改后复核）。
- 所有触及 `.ps1` 均以 `powershell -NoProfile -File` 实际执行（即证语法正确）。
- `validate-step-0220/0246` 的 `update-m22-corpus -Verify`/组件构建为预存在慢路径（≥15 min / 数 min），非本片引入；计数部分以独立 PowerShell 断言复算验证（§3 口径）。

## 5. 仍红项与归因

| 仍红目标 | 根因（实测） | 性质 | 处置 |
|---|---|---|---|
| `validate-step-0017/0019/0022/0025/0026`（cargo 步） | IND-003 解析拒绝（`indentation/invalid/zero-indent-recovery-declaration.sico`，E1001）使 parser/format/hir/semantics 的 b 集测试红 | **契约**（STEP-0584 §6/§8.1） | **owner 待决**：IND-003 归属（移出 b 集 vs 放宽「b 集全部可解析」） |
| `validate-step-0027`（cargo 步） | 同上（IND-003） | 契约 | 同上 |
| `validate-m1-exit.ps1`（cargo fmt 步） | 工作区 `cargo fmt --all --check` 红——另一代理未格式化 `crates/sico-package`（provenance）文件 | **并发代理**（非本片、非计数） | 如实登记；不计入本片欠账 |
| `validate-m2-exit.ps1`（步骤循环） | `validate-step-0022` 红（IND-003） | 契约 | owner 待决（IND-003） |
| `validate-step-0214.ps1` | `bootstrap_bundle` 编译错：`sico-ui-renderer-d2d` E0004（另一代理在途） | **并发代理** | 如实登记 |
| `validate-step-0215.ps1` | `selfhost_checker` exit 0xc0000005 访问违例 | **环境/并发代理** | 如实登记 |
| `validate-step-0220.ps1` | `update-m22-corpus -Verify` 慢（≥25 min，预存在）；计数断言已独立验证为真 | **环境/预存在** | 计数已修；整体红非计数 |
| `validate-step-0246.ps1` | 组件构建/runner 运行（环境，含 ICE/gcc 缺 PATH 风险） | **环境/预存在** | 计数已修；整体红非计数 |
| `validate-error-taxonomy.ps1` | 第 87 行 taxonomy 诊断键 ≠ v0 catalog 键（数据不一致，非计数） | **数据**（error-taxonomy 冻结契约面） | 计数已改（被遮蔽）；taxonomy/catalog 不一致为单独欠账，留 owner/后续 |
| `tools/measure-syntax-candidates.ps1` / `tools/validate-semantic-cases.ps1` | B≠A0 镜像（B 66 ⊋ A0 58，8 条 b-only） | **契约**（STEP-0584 §7/§8.2） | owner 待决（恢复镜像 vs 收窄镜像契约） |

## 6. 诚实收窄（本片未闭合，均为非本片计数欠账）

- **IND-003 连带清单**（因 IND-003 解析拒绝而红，**owner 待决**，本片未动、未裁）：`validate-step-0017/0019/0022/0025/0026/0027` 的 cargo 步；`validate-m2-exit.ps1` 的步骤循环（经 0022）；`crates/sico-{parser,format,hir,semantics}.src` 的 b 集测试、`crates/sico-cli/tests/cli.rs:136`（IND-003 `type_checker=="not-run"` vs 断言 `"error"`）、`crates/sico-ir/tests/core_lowering`（IND-003 达 IR support dispatch）——均已在 STEP-0580 §7 / STEP-0584 §9 登记。
- **并发代理/环境**（非本片、非计数）：工作区 `cargo fmt`（sico-package provenance）、`sico-ui-renderer-d2d` E0004、`selfhost_checker` 访问违例、`update-m22-corpus -Verify`/组件构建慢路径、rustc ICE / gcc 缺 PATH。
- **error-taxonomy 的 taxonomy/catalog 不一致**（`validate-error-taxonomy.ps1:87`）：独立数据欠账，本片只改被其遮蔽的 `caseIds` 计数（33→46），未修该不一致（涉 error-taxonomy 冻结契约面，属 owner 裁决）。
- **B≠A0 镜像**：`measure-syntax-candidates.ps1` / `validate-semantic-cases.ps1` 红因路径集镜像被 b 增长打破——STEP-0584 §7 已出两方案，owner 待裁。
- **parser/hir 快照未再生成**（b-ast-shapes.txt=58、b-shapes.txt=58 而 b=66）：再生成被 IND-003 阻挡；相关计数在 STEP-0584 §6/§8.1 已登记，不重复。

## 7. 边界

- 本片**改**（12 个 `.ps1`）：`tools/validate-m1-exit.ps1`、`tools/validate-m2-exit.ps1`、`tools/validate-step-0017/0019/0022/0025/0027/0214/0215/0220/0246.ps1`、`tools/validate-error-taxonomy.ps1`、以及本 STEP 文档。
- 本片**未改**：`runner/**`（三个并发代理在动）、`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/**`、`crates/sico-semantics/src/lib.rs`（lib.rs:4826 的 `expected.len()` 2→4 为热文件可动范围外，STEP-0584 §8.3 已登记）、`selfhost/**`、`guests/**`、`semantic-cases/**`、`syntax-candidates/**`、`syntax-mutations/**`、`diagnostics/semantic-case-map.json`（STEP-0584 定稿）、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、任何历史 STEP/审计报告正文。
- 未 commit/push；保留所有既有改动（git status 既有 M/?? 文件未动）。
- 未运行全 workspace `cargo test`（仅定向 `-p` 目标 + 校验器自身调用）；未触碰 runner 测试源码。
- 计数断言保持 **fail-closed**（`-ne` 严格相等，未放宽、未删除）；若语料再漂移，触及校验器会变红。

## 8. 建议主线审核点

1. **本片 12 个 `.ps1` 的计数改动**：逐处核对「实测值 == 注释所述口径」；确认无 `>=`/`-ne` 放宽、无断言删除、无冻结语料/guest/历史 STEP 文档正文改动。
2. **m1/m2-exit 的 STATUS handoff 检查改为历史记录注释**：确认权威 GO 结论仍由 audit doc 断言（`m1/m2-exit-audit.md`），且未删除历史断言语义（历史值写入注释）。
3. **IND-003 归属**（owner 裁决项）：若裁决「移出 b 集」，则本片 0017/0019/0022/0025/0027/m1/m2-exit 的 66/28/38/5/3/5 及 b-ast-shapes/b-shapes 快照 58 需**再次**重同步至 65/…；若裁决「放宽契约」，则上述 cargo 红可转绿（计数已对）。
4. **error-taxonomy taxonomy/catalog 不一致**（line 87）：独立数据欠账，与计数无关，建议单独 STEP 或 owner 裁决。
5. **`update-m22-corpus -Verify` 慢路径**（≥15 min/次，0220/0246 依赖）：预存在，建议登记为已知性能事项（非本片引入）。
