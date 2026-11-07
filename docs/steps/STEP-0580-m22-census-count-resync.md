# STEP-0580 m22 **普查数重同步片**：把硬编码源码/语料计数断言对齐到合并后真实冻结集（`syntax-candidates/b` 58→66、`revision` 4→7、能力面 (20,5)→(23,5)、`(25,33)`→`(28,38)`、`selfhost_formatter` 99/116→118/117、`corpus` 基数/分区 215→235 / `(116,34,65,0)`→`(116,46,73,0)`）⇒ 9 个根工作区目标中的 **3 个整绿**（sico-ai-tools / sico-lexer / sico-ir-flow），其余**仍红但根因已改判为非普查**（见 §7 诚实收窄）

| 项 | 值 |
|---|---|
| 片型 | 有界集成修复：只对齐「硬编码普查数」断言与其口径注释；**不**改冻结语料（`selfhost/corpus-v0.json`、`selfhost/corpus-w1/`、`selfhost/*-v0.json`）、**不**改 guest 源、**不**放宽断言 |
| exit | **部分达成**：所有普查锚已按实测值更新（§3 表）；`sico-ai-tools`/`sico-lexer`/`sico-ir-flow_lowering` 由红转绿，两个 ps1 普查门（0016/0068）复跑 `OK`；**其余仍红**，但根因均为**非普查**数据/语义缺口（§7） |
| 不翻门 | 不声称任何 gate 翻转；M22 overall 维持 NO-GO |
| 证据等级 | `internal-fixture`：本机 2026-11-07，Windows x64，`cargo test --locked --offline`，工具链 1.98.0-x86_64-pc-windows-gnu；合并 `7bcdc35` 之后实测 |

## 1. 背景

`dev` 与 `origin/codex/m22-w2` 已合并（merge commit `7bcdc35`）。合并后：
- `selfhost/corpus-v0.json` = **235 条**，其中 `rust_format` 分布 **accepted 118 / refused 117**（PowerShell `ConvertFrom-Json` 实测）。
- `syntax-candidates/b` 递归 `.sico` 枚举 = **66**（`find` 等价于各测试的 `collect_sico`，无 exclude）；此前硬编码 **58**。
- `syntax-candidates/b/revision` = **7**（此前 4）；`syntax-mutations/b` = 12（未变）；`semantic-cases`（A0）= 58、`syntax-candidates/c` = 58。

该欠账在合并前 `dev` 线即已登记（`docs/STATUS.md` §226、`docs/handoff-m22.md` §995–1003：「9 个根工作区目标 + runner `selfhost_formatter`」），**非本次合并引入**。

## 2. 普查定位：全部硬编码计数断言

在根 workspace 各 crate 的 `tests` 与 `src` 内 `#[cfg(test)]`、runner workspace 的 `tests`、`tools/*.ps1` 普查数门中逐个枚举并判断语义。结论：**18 处普查锚**（应更新，见 §3）+ **若干契约常量**（不得改，见 §4）。

## 3. 普查清单表（18 处普查锚，全部已更新）

| # | 文件:行号 | 旧值 | 新值 | 口径（推导） | 类别 |
|---|---|---|---|---|---|
| 1 | `crates/sico-cli/tests/cli.rs:109` | 58 | **66** | `syntax-candidates/b` 递归 `.sico` 枚举 | 普查锚 |
| 2 | `crates/sico-cli/tests/cli.rs:137` | (25,33) | **(28,38)** | `// expect: accept` 标记分账（28 accept / 38 reject） | 普查锚 |
| 3 | `crates/sico-ai-tools/src/lib.rs:889` | 58 | **66** | 同 #1 | 普查锚 |
| 4 | `crates/sico-ai-tools/src/lib.rs:911` | (25,33) | **(28,38)** | 同 #2 | 普查锚 |
| 5 | `crates/sico-lexer/src/lib.rs:711` | 58 | **66** | 同 #1 | 普查锚 |
| 6 | `crates/sico-format/src/lib.rs:295` | 58 | **66** | 同 #1 | 普查锚 |
| 7 | `crates/sico-hir/src/lib.rs:403` | 58 | **66** | 同 #1 | 普查锚 |
| 8 | `crates/sico-parser/src/lib.rs:1189` | 58 | **66** | 同 #1 | 普查锚 |
| 9 | `crates/sico-semantics/src/lib.rs:4886` | 58 | **66** | 同 #1 | 普查锚 |
| 10 | `crates/sico-semantics/src/lib.rs:4831` | 4 | **7** | `syntax-candidates/b/revision` 递归枚举 | 普查锚 |
| 11 | `crates/sico-semantics/src/lib.rs:4877` | (2,2) | **(3,4)** | revision 的 accept/reject 分账 | 普查锚 |
| 12 | `crates/sico-ir/tests/flow_lowering.rs:117` | (20,5) | **(23,5)** | accept 标记 28 个 case 经 `lower_core` 分账（23 lowered / 5 typed-refused） | 普查锚 |
| 13 | `runner/sico-runner/tests/selfhost_formatter.rs:81` | 99 | **118** | `corpus-v0.json` `rust_format == "accepted"` | 普查锚 |
| 14 | `runner/sico-runner/tests/selfhost_formatter.rs:89` | 116 | **117** | `rust_format == "refused"` | 普查锚 |
| 15 | `tools/measure-syntax-candidates.ps1:101` | 58（全部） | **A0=58 / B=66 / C=58** | 每根 `.sico` 计数（B 单独 66） | 普查锚 |
| 16 | `tools/validate-step-0016.ps1:27` | 58 | **66** | `syntax-candidates/b` 计数 | 普查锚 |
| 17 | `tools/validate-step-0068.ps1:21` | 58 / (25,33) | **66 / (28,38)** | 针字符串 + `STEP_0068_OK` 输出行 | 普查锚 |
| 18 | `tools/validate-step-0245.ps1:43,46` | (116,34,65,0) / 215 | **(116,46,73,0) / 235** | 冻结语料基数与分区（116 lexical + 46 semantic + 73 accepted + 0 unsupported = 235） | 普查锚 |

## 4. 契约常量（**未改**，经复核为正确或冻结事实）

| 位置 | 值 | 依据 |
|---|---|---|
| `crates/sico-ai-tools/src/lib.rs:863` | `tasks.len(), 12` | `ai-eval/tasks/repair.json` B 任务 = 12（实测），冻结夹具计数 |
| `crates/sico-hir/src/lib.rs:468` | `paths.len(), 12` | `syntax-mutations/b` = 12（实测） |
| `crates/sico-format/src/lib.rs:322` | `paths.len(), 12` | 同左 |
| `crates/sico-semantics/src/lib.rs:4510/4600/4672/4753/4826` | `expected.len(), 9/8/4/10/2` | `semantic-case-map.json` 前缀过滤计数（现况匹配） |
| `crates/sico-semantics/src/lib.rs:4522/4612/4684/4763` | `paths.len(), 16/14/8/16` | 各语义组 `.sico` 计数（现况匹配） |
| `tools/validate-step-0016.ps1:23` | `contractCases == 21` | `tests/lexical/contract-v0.json` 冻结契约 |
| `tools/validate-step-0068.ps1:17,25` | `29 用例` / `test count == 12` | AI tooling 冻结契约 |
| `runner/sico-runner/tests/selfhost_checker.rs:141,258` | `entries.len(), 235` / `(116,46,73,0)` | 已随 235 重冻结（未动，绿） |

## 5. 实测口径

- **b 目录 66**：`find syntax-candidates/b -name '*.sico' -type f`（等价于各测试 `collect_sico` 的递归 `.sico` 枚举，无 exclude）。逐文件标记统计：**28 个 `// expect: accept`、38 个 `// expect: reject`**，每文件恰一标记。
- **revision 7**：`syntax-candidates/b/revision` 递归枚举 = 7（3 valid/accept + 4 invalid/reject）。
- **能力面 (23,5)**：`cargo test -p sico-ir --test flow_lowering` 实测 `left: (23, 5)`。
- **corpus 235 / 118 / 117**：PowerShell `ConvertFrom-Json` 读 `selfhost/corpus-v0.json`，`entry_count=235`、`rust_format` accepted=118、refused=117。
- **A0/C = 58**：`semantic-cases` 与 `syntax-candidates/c` 递归枚举均 58。
- **各 crate 计数**：以 `cargo test --locked --offline -p <crate> --lib <filter>` 实测断言失败时的 `left` 值（§6），不凭估计。

## 6. 修复与逐条验证

改动文件（13，均非冲突避让清单所列）：
`crates/sico-cli/tests/cli.rs`、`crates/sico-ai-tools/src/lib.rs`、`crates/sico-lexer/src/lib.rs`、`crates/sico-format/src/lib.rs`、`crates/sico-hir/src/lib.rs`、`crates/sico-parser/src/lib.rs`、`crates/sico-semantics/src/lib.rs`、`crates/sico-ir/tests/flow_lowering.rs`、`runner/sico-runner/tests/selfhost_formatter.rs`、`tools/measure-syntax-candidates.ps1`、`tools/validate-step-0016.ps1`、`tools/validate-step-0068.ps1`、`tools/validate-step-0245.ps1`。每处只改常数与口径注释（注释写明冻结事实与 STEP-0279–0283 依据），未放宽断言、未删除断言、未改冻结语料/guest 源。

| 目标 | 命令 | 结果 |
|---|---|---|
| sico-lexer lib | `cargo test --locked --offline -p sico-lexer --lib` | **8 passed / 0 failed**（绿） |
| sico-ai-tools lib | `cargo test --locked --offline -p sico-ai-tools --lib` | **12 passed / 0 failed**（绿） |
| sico-ir flow_lowering | `cargo test --locked --offline -p sico-ir --test flow_lowering` | `cumulative_capability...` **绿**（4 passed / 1 failed = core_lowering，见 §7） |
| validate-step-0016 | `powershell -File tools/validate-step-0016.ps1` | `STEP_0016_OK contract=21 b_cases=66` exit 0 |
| validate-step-0068 | `powershell -File tools/validate-step-0068.ps1` | `STEP_0068_OK inspect=66 clean=28 diagnosed=38` exit 0 |
| 根工作区 fmt | `cargo fmt --all -- --check` | **exit 1，但仅因其它代理的 `provenance_budget.rs`/`provenance_install.rs`**（未跟踪、非本片文件）；本片 13 文件全部 fmt 净 |
| runner fmt | `cargo fmt --all --manifest-path runner/sico-runner/Cargo.toml -- --check` | 同左（仅其它代理 provenance 文件） |
| `git diff --check` | `git diff --check` | **exit 0**（净） |
| runner `selfhost_formatter` | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_formatter` | **未运行**：rustc 1.98.0 windows-gnu 触发 dep-graph 序列化 ICE（`rustc_middle/src/dep_graph/serialized.rs:352`），且 gcc/dlltool 不在 PATH（handoff §5 已登记该环境限制）。计数以等价枚举复算 = **118/117**（§5） |

sico-format / sico-hir / sico-parser / sico-semantics / sico-cli / sico-ir-core_lowering 的普查锚虽已更新，但对应测试在计数断言通过后**仍红**，根因**非普查**（见 §7）。

## 7. 诚实收窄：仍红项与待裁决项（根因已改判为非普查）

普查锚更新本身是正确的（§3 各新值均实测），但 `syntax-candidates/b` 从 58→66 的增长**同时**打破了数个**非计数**契约，这些不是「硬编码普查数」问题，本片未修（避免越界与弱化断言）：

| 仍红目标 | 根因（非普查） | 性质 |
|---|---|---|
| sico-format `all_b_cases_preserve_ast_shape...`（lib.rs:306） | **IND-003**（`indentation/invalid/zero-indent-recovery-declaration.sico`）为**解析错误**用例（`// expect: reject(MISSING_FUNCTION_CLOSE)`，E1001），`parse().ast()` 为 `None` | 数据组成：b 集含一个解析拒绝用例，违反「b 集全部可解析」契约 |
| sico-parser `all_b_canonical_cases_parse_losslessly...`（lib.rs:1205） | 同上，`MissingFunctionClose` | 同上 |
| sico-hir `all_b_sources_lower_deterministically...`（lib.rs:415） | 同上（lower 依赖 parse） | 同上 |
| sico-semantics `remaining_b_groups_do_not_receive_unowned_diagnostics`（lib.rs:4903） | 同上 | 同上 |
| sico-ir `all_thirty_three_invalid_cases_stop_before_ir_support_dispatch`（core_lowering.rs:753） | **IND-003 到达 IR support dispatch**（`lower_core` 返回非 `Semantic`）——即 handoff §995 所记「新增 IND-003 已越过 IR support dispatch」 | 语义/IR 行为；其 `rejected == 33` 计数同样过期（实为 38）但**在 IND-003 解决前不可达**，本片未动（避免「改了也白改」） |
| sico-cli `check_matches_all_58_semantic_oracles`（cli.rs:128） | **IND-003/004/005、REV-201/203 五个拒绝用例的 case id 未登记在 `diagnostics/semantic-case-map.json`**（oracle 41 条，缺这 5 条）→ `expected[case]` 索引 panic | 数据完整性：新用例未随 b 集增长登记入 oracle |
| sico-semantics `revision_cases_match_the_registered_oracle`（lib.rs:4856） | **REV-201/203 未登记在 oracle**（同上） | 同上 |
| `tools/measure-syntax-candidates.ps1:133` | **B ≠ A0 镜像**：b 集含语法专属组（indentation 5 例等）而 A0 无 ⇒ 路径集不镜像 | 数据组成：镜像契约被 b 集增长打破（计数已修，镜像仍红） |
| `tools/validate-semantic-cases.ps1`（若红） | 同上（B/C 须镜像 A0，B=66 ≠ A0=58） | 契约校验器（manifest 驱动，非硬编码计数），未运行、未修改 |
| `crates/sico-cli/examples/m1_frontend_bench.rs:20,45,57` | `paths.len(), 70` / `(iteration+1)*58` / `"canonical_files": 58` 过期（实为 78 / 66 / 66） | 例程（非 test/src/ps1），**超出本片范围**，未改；建议后续一并修 |

## 8. 结论与建议下一步

- 本片把 18 处普查锚全部对齐到实测值；`sico-ai-tools` / `sico-lexer` / `sico-ir-flow_lowering` 三个目标**整绿**，两个 ps1 普查门复跑 `OK`。**普查数欠账已关闭**。
- **红测欠账未整体关闭**：剩余 7 个根/runner 目标 + 2 个 ps1 仍红，根因均为 **IND-003 解析拒绝用例**与 **oracle 缺 5 条 case id**（+ B≠A0 镜像），属**数据/语义**而非普查数。建议下一步（需 owner 裁决，因涉数据组成或契约放宽）：
  1. **IND-003**：裁决 `syntax-candidates/b` 是否应含解析拒绝用例——若否，移入 `syntax-mutations/` 或拒绝语料目录（数据）；若是，更新 `format/hir/parser/semantics` 四测试与 `core_lowering` 以处理解析拒绝用例（契约）。
  2. **oracle 5 条**（IND-003/004/005、REV-201/203）：用 `sico check --json` 实测各诊断并补 `diagnostics/semantic-case-map.json`（数据）。
  3. **B≠A0 镜像**：裁决 `measure-syntax-candidates.ps1` 与 `validate-semantic-cases.ps1` 的镜像契约是否仍适用（b 集已含语法专属组）。
  4. **m1_frontend_bench.rs** 例程 58/70/66 随 1–3 一并收口。

## 9. 边界

- 本片未触碰 `runner/sico-runner/tests/bootstrap_bundle.rs`、`crates/sico-package/src/provenance.rs`、`crates/sico-ui-renderer-d2d/**`（冲突避让）。
- 未 commit/push；未改动 `docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`（共享登记表由主线统一更新）。
- 未运行 `validate-step-0245.ps1` 全文（其 cargo 步含 `bootstrap_bundle`，属冲突避让；仅静态核对其两处字符串检查已满足）。
- `runner/sico-runner/tests/selfhost_formatter.rs` 计数以等价枚举复算；测试本体因工具链 ICE 未执行。
- 工作区大量既有他人改动（provenance、ui-renderer-d2d、M24 step 等）未动；本片 13 文件为唯一新增修改。
