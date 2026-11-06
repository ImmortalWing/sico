# STEP-0566 — M22 CI 触发面修复（`.github/workflows/m22-w1-ci.yml`）

- 日期：2026-11-06（Asia/Shanghai；结构校验时刻 UTC `2026-11-06T09:34:19Z`）
- 分支 / 树：`codex/m22-w2` @ `a9398627e51c21af90b019b0e7caae6615f30d73`，工作树进入本片时干净
- 本片性质：**CI 触发面修复片（配置片）**。改 1 个 workflow 文件的**触发集 + 文件头注释 + workflow 名**；**零产品代码改动**（`crates/*`、`selfhost/*`、`runner/*`、`tools/*` 全部零改动）⇒ 不推进任何 M22 出口门、**M22 仍 NO-GO**、七门文本与证据等级未动、不翻门、不做完成声明。
- 报告：[`docs/reports/m22-ci-trigger-2026-11-06.md`](../reports/m22-ci-trigger-2026-11-06.md)
- 校验器（本片新建，**未入库**，放在 gitignored 的 `target/tmp/`）：`target/tmp/step0566_validate_workflow.py`

## 1. 入口门槛

- STATUS / handoff 登记的缺口 = 唯一工作流 `m22-w1-ci.yml` 的 `push` 触发只挂在 `codex/m22-w1-ci`，而该分支已从 GitHub 远端删除 ⇒ 活动线 `codex/m22-w2` 与 `dev` 的 push 均不产生 CI run ⇒ M22 出口审计要求的「exact-SHA 独立 CI」不可取得。
- 本片入口门槛 = ①该缺口在**当前树**可复现（远端 ref 列表里没有 `codex/m22-w1-ci`）；②改动面只在 `.github/workflows/`（本片是该目录的唯一写者）；③不触碰评审者维护的文档（STATUS / handoff / plans / ROADMAP / RFC / ADR / steps README）与热文件（`selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/.../selfhost_compiler.rs`）。三项均满足。

## 2. 实测证据（全部来自本机实际执行的命令，退出码如实记录）

### 2.1 缺口复现（远端 ref 普查）

```
$ git rev-parse HEAD
a9398627e51c21af90b019b0e7caae6615f30d73
$ git branch --show-current
codex/m22-w2
$ git ls-remote --heads github | grep -E "m22-w1-ci|m22-w2|dev"
a9398627e51c21af90b019b0e7caae6615f30d73  refs/heads/codex/m22-w2
615c6ef1b8c0821192c8dbbee8cffc17a773f760  refs/heads/dev
```

`refs/heads/codex/m22-w1-ci` **不在**远端 ref 列表里 ⇒ 旧触发目标确实已删除（可执行证据，非文档转述）。本地分支清单（`git branch -a`）同样只有 `codex/m22-w2` / `codex/m24-vision-ui-v2` / `dev` / `main` 与两条远端镜像。

### 2.2 既有工作流的作业步骤与运行环境（改前全文已读）

一个 job `windows-gnu`：`runs-on: windows-2025`、`timeout-minutes: 180`、3 个 step：

1. `actions/checkout@v4`；
2. `msys2/setup-msys2@v2`（MINGW64，装 `mingw-w64-x86_64-gcc` + `mingw-w64-x86_64-binutils`；随后断言 `dlltool.exe` 存在）；
3. `Run repository CI on a fresh runner`（PowerShell）：装 `1.98.0-x86_64-pc-windows-gnu`（含 clippy / rustfmt）⇒ 把 MSYS2 `mingw64\bin` 置于 `PATH` 前 ⇒ `cargo fetch --locked`（根工作区 + `runner/sico-runner` 独立工作区各一次）⇒ 调 `tools/run-ci.ps1` 并透传退出码。

`tools/run-ci.ps1`（**本片零改动**，只读）= 14 个内层步：`fmt` / `fmt (runner)` / `clippy (workspace)` / `clippy (runner)` / `build (runner debug)` / `test (runner core)`（串行、先跑，让环境敏感的 20 ms planning 预算断言在安静机器上量一次）/ `test (workspace)` / `test (runner suite)`（串行，`--skip prepared_program_reruns_are_isolated_and_survive_a_trap`，因已量过一次）/ `module boundaries` / `selfhost comment discipline` / `planning contract (M14-M25)` / `application-profile matrix` / `cross-host matrix + UI corpus` / `whitespace (git diff --check)`；逐步结果写入 `$GITHUB_STEP_SUMMARY`（匿名可读）。

### 2.3 改动（`git diff`，1 文件，+22 / −2）

```diff
-name: M22 W1 independent CI
+# ...文件头注释块（命名说明 + 触发决定）...
+name: M22 independent CI (W2 line)
 on:
   push:
     branches:
-      - codex/m22-w1-ci
+      - codex/m22-w2
   workflow_dispatch:
```

job id、runner 镜像、timeout、3 个 step 与被调脚本**逐字节不变**。

### 2.4 决定（写在文档里，非隐性）

- `push` → **加 `codex/m22-w2`**：活动线（本地与 `github/codex/m22-w2` 同在 `a939862`）⇒ 这是让出口审计的 exact-SHA 独立 CI 重新可得的唯一必要项。
- `workflow_dispatch` → **保留**：它是对非 W2 分支取 CI 裁决的唯一途径（例如 STEP-0544 用过的隔离 worktree 分支 `codex/m22-w2-step0544`，为保证「证据树 = 提交树」而离开主树），也是分支再次改名时的兜底。
- `dev` → **不加**（明确决定）。利弊：
  - **利**：`dev` 是 M22 交付描述里的最终集成目标；`dev` 上的红灯可能早发现跨线破坏。
  - **弊**：M22 计划 **D14** 已裁决「M22 的完成判据**不包含**与 `origin/dev` 的合并、不 rebase、不 merge，冲突历史登记为 M25 集成议题」⇒ `dev` 上的 run **不构成任何 M22 门的证据**；每轮 run 耗 `windows-2025` runner 约 1–2 小时（近期已裁决的 run：57m34s / 1h06m43s / 1h14m36s）⇒ 每个 `dev` push 都烧这段时间却没有门价值；`dev` 树还承载其他里程碑的在途工作 ⇒ 会把无关失败记到 M22 的 CI 账上，污染前几片逐 run 维护的裁决台账。
  - **结论 = 不加**。若 owner 之后要 `dev` 信号，更便宜且范围更清的做法是另开一个 workflow 文件、或按需 `workflow_dispatch` 到 `dev` ref；两者都是 owner 决定，本片不做。

### 2.5 文件名为何仍叫 `m22-w1-ci.yml`

`AGENTS.md` §2/§8、M22 计划的「独立 CI」条目、STEP 与 handoff 记录都按这个名字引用该文件，历史 run #22–#54 的裁决也都系于它 ⇒ 改名会断掉可追溯性、零行为收益。故**文件名不动**，改为在文件头注释里写明它现在是 W2 线的工作流。改名登记为 owner 选项，非本片改动。

### 2.6 校验（含先写会失败的测试）

环境事实：`python` 不在 `PATH`（Windows Store 别名），仓库固定解释器 `target/tooling/python-3.12.10/python.exe` 里**没有 PyYAML**：

```
$ target/tooling/python-3.12.10/python.exe -c "import yaml; print(yaml.__version__)"
ModuleNotFoundError: No module named 'yaml'   (exit 1)
$ powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/ensure-python.ps1
D:\SorftWare\sico-m22-bootstrap\target\tooling\python-3.12.10\python.exe   (exit 0)
```

⇒ 任务建议的 `yaml.safe_load` 校验**在本机无法照原样执行**（无 PyYAML；不私自装依赖）。改用纯标准库的**结构校验器**，并按「先写会失败的测试」纪律**先在旧文件上跑它**：

```
# ① 把 HEAD 版工作流写回工作区（validator 钉住旧触发集）
$ git show HEAD:.github/workflows/m22-w1-ci.yml > .github/workflows/m22-w1-ci.yml
$ python target/tmp/step0566_validate_workflow.py
AssertionError: ['name: M22 W1 independent CI', 'on:', 'jobs:']   → OLD-file-validator-exit=1   ✅ 会失败
# ② 恢复本片新文件
$ cp target/tmp/step0566_new_workflow.yml .github/workflows/m22-w1-ci.yml
$ python target/tmp/step0566_validate_workflow.py
WORKFLOW_STRUCTURE_OK
  top-level keys : ['name: M22 independent CI (W2 line)', 'on:', 'jobs:']
  push branches  : ['codex/m22-w2']
  dispatch       : present
  job            : windows-gnu / runs-on: windows-2025
  steps          : 3
→ NEW-file-validator-exit=0   ✅
$ git diff --check        → exit 0
$ git diff --stat -- .github/workflows/m22-w1-ci.yml
 1 file changed, 22 insertions(+), 2 deletions(-)
```

校验器断言：纯 LF、无 BOM、缩进 0 的键恰为 `name:` / `on:` / `jobs:`；`on:` 下有 `push:`（分支列表恰为 `codex/m22-w2` 一项）与 `workflow_dispatch:`；恰好一个 job `windows-gnu:` 带 `runs-on: windows-2025` / `timeout-minutes: 180` / `steps:`（3 个 step）且含 `run-ci.ps1` 调用；触发列表里没有残留的 `- codex/m22-w1-ci`。

**诚实收窄（校验强度）**：这是**结构校验**，不是 GitHub Actions schema 校验、也不是 YAML 解析器。改动面 = 4 行触发/命名 + 一段注释，其余行与产出过 run #22–#54 裁决的版本**逐字节相同** ⇒ schema 风险被限制在触发块，而该块的形状（`on.push.branches` 字符串列表、`on.workflow_dispatch` 空值键）与此前可运行版本一致。更强的 `actionlint` 本机未安装、也未下载。

### 2.7 工作区状态

```
$ git status --short
 M .github/workflows/m22-w1-ci.yml          ← 本片
 M selfhost/match-component-coverage-v1.json ← 他人（并发写者）既有改动，本片未触碰、原样保留
```

## 3. 未跑项（如实登记）

- **未触发、未观察任何 CI run**：本片不 commit、不 push（纪律要求）⇒ **无 run id、无 head_sha、无 conclusion**。绝不声称 CI 已跑。第一个真实裁决来自下一次获授权的 `codex/m22-w2` push，由持有推送权者登记。
- **未跑 `actionlint`**（本机未装，未下载）。
- **未跑 `tools/run-ci.ps1`**：workflow 体零改动，完整本机电池是多小时、机器敏感的运行，纯文档/配置改动不影响它。
- **未做 PyYAML 解析**：本机无 PyYAML，见 §2.6。
- **未改评审者文件**：`docs/STATUS.md`、`docs/handoff-m22.md`、`docs/plans/**`、`docs/steps/README.md`、`docs/ROADMAP.md`、`docs/rfc/**`、`docs/adr/**` 零改动 ⇒ STATUS 里那条 2026-10-21 的触发面登记**仍由评审者按其流程更新**，本片只提供证据文件（本 STEP + 报告）。

## 4. 生效条件与残余风险

- **生效条件**：本改动必须**提交到 `codex/m22-w2` 并推送到 `github`** 之后才生效；在那之前 `codex/m22-w2` 的 push 仍不产生 CI run（工作区里的一处未提交编辑对远端零影响）。
- **残余风险 1（分支再次改名即静默熄火）**：W1 就是这样灭的。兜底是 `workflow_dispatch`（保留），或未来再加一条触发分支；本片不做。
- **残余风险 2（timeout 余量）**：仍是单 job `windows-2025` / 180 分钟；近期完整 run 57–75 分钟 ⇒ 余量未变、够用；若套件涨到 ~3 小时，约束会从触发面移到 timeout（届时是独立议题）。
- **残余风险 3（未验证的远端行为）**：远端是否已配置分支保护 / required checks 指向旧的 run 名（`M22 W1 independent CI` / job `windows-gnu`），本机无从核实；workflow **名**已改为 `M22 independent CI (W2 line)`（job id `windows-gnu` 未变）。若 required-check 按名字绑定，owner 首次推送后需核对一次。
- **门状态不变**：本片是出口审计「exact-SHA 独立 CI」这条**证据路径**的修复，不是里程碑结果 ⇒ **M22 仍 NO-GO**，门 3 / 6 仍 NO-GO，其余门按 STATUS 既有口径，未做任何 GO/NO-GO 复审。
