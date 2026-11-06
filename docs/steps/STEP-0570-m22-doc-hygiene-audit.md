# STEP-0570 · M22 文档/工件卫生审计（只读审计片）

| 字段 | 值 |
|---|---|
| 片编号 | STEP-0570 |
| 分支 / HEAD | `codex/m22-w2` / `a9398627e51c21af90b019b0e7caae6615f30d73` |
| 片型 | **只读审计片（documentation-only audit）**：零源码、零 gate、零既有文件改动 |
| 日期 | 2026-11-06 |
| 产物 | `docs/reports/m22-doc-hygiene-2026-11-06.md`（discrepancies 清单）、本文件 |
| 门影响 | **无**。M22 仍 **NO-GO**、七门一律不变；本片不翻门、不改证据等级、不做完成声明 |

## 1. 入口门槛

- 审计对象 = 5 类文档/工件一致性：AGENTS.md §10.1.2 标 `[x]` 的两个工具、§10.10/§10.11 与 STATUS 的对齐、`tools/validate-step-0124.ps1` 校验的 steps README 表 vs `docs/steps` 实际文件、工作树 untracked/dirty 卫生风险、重复 STEP 编号（0279–0283、0557/0561）。
- 前置：只读，不需要 runner 重编译；只允许新建本片两个文件，**不得编辑** STATUS / handoff / plans / steps README / ROADMAP / RFC / ADR / 既有 STEP / 热点源码。

## 2. 实测证据（命令 + 退出码 + 关键输出）

所有命令在 `D:/SorftWare/sico-m22-bootstrap` 下、Git Bash 执行；PowerShell 脚本用 `powershell.exe -NoProfile`。`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`。

| # | 命令 | 退出码 | 关键输出 / 判读 |
|---|---|---|---|
| 1 | `git status --short` | 0 | 初始输出为空；但后续带 `--untracked-files=all` 的复查见 #14 ⇒ 派单「工作树干净」在审计窗口内已不成立（并发片写入） |
| 2 | `git rev-parse HEAD` | 0 | `a9398627e51c21af90b019b0e7caae6615f30d73`（审计全程 HEAD 未前进） |
| 3 | `ls tools/validate-m22-s5-dev.ps1 tools/m22-s5-fingerprint.ps1` | 0 | 两文件均存在（13992 B / 6542 B，mtime 11-06 17:40 / 17:38） |
| 4 | `git cat-file -e HEAD:tools/validate-m22-s5-dev.ps1` | **128** | `fatal: path '...' exists on disk, but not in 'HEAD'`；`m22-s5-fingerprint.ps1` 同 ⇒ **untracked，不在 HEAD** |
| 5 | `git ls-files tools/validate-m22-s5-dev.ps1 tools/m22-s5-fingerprint.ps1` | 0 | 空输出 ⇒ 确认未跟踪 |
| 6 | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -List` | 0 | 列出 `fast 6 steps` / `probes 3 steps` / `firmware 3 steps` / `all 15 steps`，与 `AGENTS.md:176` 时长标注一致 |
| 7 | `sha256sum selfhost/codegen.sico` | 0 | `013f2d7e894994b3734646c6c9996c14e2e0d999a7fd37a37ef3ac5dc0854d0f` |
| 8 | `grep -o '"source_sha256"[^,]*' selfhost/emitter-vocabulary-v1.json` | 0 | 含 `"source_sha256": "013f2d7e…"` ⇒ 与 #7 一致 ⇒ `AGENTS.md:338` 的「必红」前提已消失 |
| 9 | `python --version` / `python3 --version` / `py --version` | 9009 / 127 / 127 | 本机无 Python（仅 Microsoft Store 别名）⇒ census `--check` 无法实跑（见 §4） |
| 10 | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0124.ps1` | 0 | `STEP_0124_OK milestones=M14-M26 steps=unreserved layers=...,compiler-self-host,...` |
| 11 | README 表 vs `docs/steps` 对账（`ls` 去重 + `grep -o 'STEP-0[0-9][0-9][0-9]'` + `comm`） | 0 | 文件 452 / 唯一号 451；README 唯一号 339；README 引用但文件缺失 = 0；文件存在但 README 未登记 ≈ 112（含 0555/0556/0558/0559/0560/0561/0566、0175/0355/0404 等） |
| 12 | 22 个 STEP 号的存在性 + H1 + README 行抽查（逐号 `ls` + `head -1` + `grep -n`） | 0 | 11 个已登记号（0279/0280/0281/0282/0283/0486/0534/0535/0544/0547/0551）标题与文件 H1 全部一致；6 个 M22 W4 号 + 0566 未登记；`STEP-0175` 有两个文件 |
| 13 | README 相对链接存在性（`grep -oE '\]\(\./STEP-...\)'` + 逐条 `[ -e ]`） | 0 | 无 MISSING ⇒ README 表无悬空链接 |
| 14 | `git status --porcelain --untracked-files=all` | 0 | 9 条：3 M（`.github/workflows/m22-w1-ci.yml`、`selfhost/match-component-coverage-v1.json`、`tools/probe-m22-corpus-parity.ps1`）+ 6 ??（`docs/reports/m22-ci-trigger-2026-11-06.md`、`docs/steps/STEP-0566-*.md`、`selfhost/corpus-parity-map-v1.json`、`tools/m22-s5-fingerprint.ps1`、`tools/profile-m22-fuel-wall.ps1`、`tools/validate-m22-s5-dev.ps1`）⇒ 并发 STEP-0565/0566 产物 |
| 15 | `git log --oneline -8 HEAD` / `git log --all --diff-filter=A --name-only` | 0 | `a939862` = `docs(m22): renumber W4 verdict-parity STEP-0557 -> STEP-0561 (collision with m24 picker STEP-0557)`；`--all` 下存在两套 `STEP-0557-*.md`（m22-w2 verdict-parity 与 m24 picker） |
| 16 | `git merge-base --is-ancestor 9f49ef2 HEAD` / `9344bde HEAD` / `a939862 HEAD` | 1 / 1 / 0 | 记录重编号的 `9f49ef2`（plans）与 `9344bde`（归档删除）**不是** HEAD 祖先 ⇒ 本分支落后这两个收尾 commit |
| 17 | 全仓 grep `STEP-0557`（`docs/`、`tools/`、`AGENTS.md`） | 1（无命中） | `docs/`、`tools/`、`AGENTS.md` 内零残留 0557 引用；`tools/probe-m22-corpus-parity.ps1:6` 已用 STEP-0561 ✔ |
| 18 | `git diff --check` | 0 | 无空白错误 |
| 19 | `"$HOME/.cargo/bin/cargo.exe" fmt --all -- --check` | 0 | `fmt (workspace)` 绿 ⇒ `AGENTS.md:339` 所述「`fmt (runner)` 红」在当前树不成立（runner 侧未单独跑，见 §4） |
| 20 | `./runner/sico-runner/target/debug/deps/selfhost_compiler-3ee9bc92fdb5f988.exe --list` | 0 | 191 个测试（STATUS 的 STEP-0558 条记 182+4 ⇒ 其间新增约 5 个，总数未更新） |
| 21 | 同上二进制 `-- --exact sico_compiler_lowers_the_step0{507,519,520,526}_codegen_functions_byte_exactly` | 0（套件级）/ **4 failed** | `test result: FAILED. 0 passed; 4 failed; ... 187 filtered out; finished in 9.96s`；panic 原因：step0507 = `cgs_result_return_hex must compile through the guest frontend`；step0519 = `cgs_value_ty ... ERR:E-SH-IR-CALL-SHAPE` ⇒ 与 `STATUS.md:3`「剩余 pre-existing 红 = step0507/0519/0520/0526」一致 |
| 22 | `sed -n '3595,3601p' runner/sico-runner/src/lib.rs` | 0 | 第 3599 行确为 `"malformed result: {name} exceeds 8 MiB"` ⇒ `AGENTS.md:203` 行号引用仍准 |

## 3. 结论（discrepancy 清单，详见报告 §F）

1. **§10.1.2 的 `[x]` 先于版本化**：两个工具实测**存在**（推翻派单假设的「不存在」），但都是 untracked、不在 HEAD `a939862`；工具头部自称 STEP-0565，而 `docs/steps/` 无 STEP-0565 记录 ⇒ 完成声明缺版本化与执行记录支撑。
2. **§10.10 滞后**：末条停在 STEP-0551（10-28），STATUS 顶部 4 项已完成工作（0555/0556/0558/0560 + 0560 收口）未收；handoff 缺「0560 后续收口」条。
3. **§10.11 三条 `[ ]` 过期或部分过期**：`:338`（词表 SHA）已被实测推翻；`:339`（脏文件）大部分已消失、`fmt` 实测绿；`:337` 成因从「没写」变为「没提交」。
4. **steps README 登记缺口**：约 112 个已存在文件未登记（含 6 个 M22 W4 号 + 0566）；被登记的 11 个抽查号标题全部与文件 H1 一致；README 无悬空链接；STEP-0440–0485 号段在 STATUS 有引用但 steps/README 均无（跨分支历史）。
5. **工作树卫生**：并发 STEP-0565 + STEP-0566 产物并存于工作树 ⇒ `git add -A` 式收口会跨片混提；本片两个新文件是唯一应随 STEP-0570 提交的内容。
6. **重复编号**：0279–0283 本分支内唯一、跨分支（`origin/dev`）重复（已知、已声明不得隐式合并）；0557→0561 重编号已落地但只在 commit message，STEP 文档/plans 无记录且记录它的两个 commit 不是 HEAD 祖先；另发现 `STEP-0175` 同号两文件。

## 4. 诚实收窄与未跑项

- **未实跑** `tools/census-m22-emitter-vocabulary.py --check`：本机无 `python`（#9）。对 `AGENTS.md:338` 的推翻依据是 SHA256 逐字节比对（#7/#8），**不是**该 census 的实际 `EMITTER_VOCABULARY_GREEN` 输出。
- **未实跑** `tools/validate-m22-s5-dev.ps1 -Set fast`：其 6 步含 3 个 `python` 步，在本机必失败 ⇒ 只跑 `-List`（#6）验证可用性与档位时长。
- **未跑** `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check`（`fmt (runner)`）：只跑了 workspace 侧（#19）⇒ 对 `AGENTS.md:339` 的「`fmt (runner)` 红」只证伪了 workspace 侧，runner 侧未测。
- **未重编译 runner**、未跑 `tools/run-ci.ps1`、未跑完整 `selfhost_compiler` 套件（只跑 4 个已知红测试，10 s，#21）。
- **未修改任何被审计文件**、未创建被审计工具（遵守「只做对账登记」）、未 commit / push。
- 派单前提「工作树干净」经实测不成立（#14）；本片一律以实测为准并标注时间，未据旧前提下任何结论。