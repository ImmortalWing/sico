# STEP-0565 · M22 W2 · 电池分层工具落地片（`tools/validate-m22-s5-dev.ps1` 与 `tools/m22-s5-fingerprint.ps1`）

**日期**：2026-11-06
**分支**：`codex/m22-w2` @ `a9398627e51c21af90b019b0e7caae6615f30d73`
**片性质**：**验证基础设施片（tooling）**。`crates/*`、`selfhost/*.sico`、`runner/sico-runner/src` **零改动**；不推进任何出口门；**M22 整体仍 NO-GO、七门不变**。

## 1. 入口门槛

AGENTS.md §10.1.2 与 §10.2 声称「分层策略落地」已完成并标 `[x]`，点名两个工具：

- `tools/validate-m22-s5-dev.ps1`（开发期子集，`-Set fast|probes|firmware|all`）
- `tools/m22-s5-fingerprint.ps1`（修正后的指纹：`target` / `__pycache__` / `*.pyc` 三项排除）

实测 `tools/` 下只有 `validate-m22-s5.ps1`，**这两个文件不存在** ⇒ 文档与事实不符，本片收口：把两个工具真正落地并实测，而不是改文档去迁就缺失。

同族前置纪律（本片必须遵守、且工具必须承压）：

- **单实例**（handoff-m22 2026-10-24）：两个电池实例并发争用同一 `target/` 与同一日志 ⇒ 这种运行的绿灯不可引用。
- **自污染**（AGENTS.md §10.11 A、STEP-0545 §6.1）：电池自己往 `target/` 写日志与状态 ⇒ 指纹若不排除 `target`，每跑一次就作废上一次的绿。
- **退出码管道**（STEP-0504 / handoff-m22 §957）：验证步骤的失败必须走真正的进程退出码，不能用 PowerShell 变量当返回值。

## 2. 交付物

| 文件 | 性质 | 内容 |
|---|---|---|
| `tools/m22-s5-fingerprint.ps1` | 新建 | 树指纹：递归哈希仓库根全部文件，排除 `target` 目录、`__pycache__` 目录、`*.pyc` 文件；`-AsJson` / `-SaveFile` / `-TestFile` / `-SelfTest` 四个入口；exit 0 匹配、2 不匹配、3 无存档、1 工具错误 |
| `tools/validate-m22-s5-dev.ps1` | 新建 | 分层子集 runner：`-Set fast\|probes\|firmware\|all`，`-Fresh` 丢弃状态，`-List` 列拍不执行；逐拍落盘状态文件 + 指纹闸门 + 单实例 PID 锁 |

两者都不改 `tools/validate-m22-s5.ps1` 本体语义（本片零改动该文件）。

### 2.1 `validate-m22-s5-dev.ps1` 的四档划分

拍名、命令、相对顺序**逐项取自 `validate-m22-s5.ps1` 的 15 拍**，只做档位切分，不新造拍、不改命令行：

| 档 | 拍数 | 包含的拍 | 目标量级（§10.1.2） |
|---|---|---|---|
| `fast` | 6 | `fmt (workspace)` / `fmt (runner)` / `comment-discipline` / `match-site-census` / `emitter-vocabulary-census` / `git-diff-check` | ≈ 1 min |
| `probes` | 3 | `probe-frontend-frontier` / `probe-ir-plan-frontier` / `match-site-coverage` | ≈ 12 min |
| `firmware` | 3（去重后 1） | `firmware-selfhost` | ≈ 28 min |
| `all` | 15 | 上述全部 15 拍，按 `validate-m22-s5.ps1` 原顺序 | ≈ 2.7 h |

`-List` 实测输出确认四档划分与拍名无误。

### 2.2 续跑与指纹闸门

- 状态文件 `target/tmp/m22-s5-dev/<set>.state.json`，**每拍结束即原子落盘**（写 `.tmp` 再 `Move-Item -Force`），记录 `schema` / `set` / `fingerprint` / `fileCount` / `startedAt` / 逐拍 `Name`/`Command`/`Seconds`/`Exit`/`FinishedAt`。
- 复跑同一档时，状态里 `Exit = 0` 的拍**跳过不重跑**；`Exit != 0` 的拍**重跑**。
- 状态文件里存的指纹与当前树指纹不一致 ⇒ **拒绝复用旧绿**，exit 2，打印两侧指纹与 `-Fresh` 提示。
- 指纹由 `tools/m22-s5-fingerprint.ps1` 子进程计算（同一套排除规则，单一实现）。
- 单实例：`target/tmp/m22-s5-dev/battery.lock` 存 PID；锁存在且该 PID 仍活 ⇒ exit 2，提示先终止陈旧实例。

## 3. 实测证据

环境：Windows + Git Bash；`cargo` = `~/.cargo/bin/cargo.exe`；`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；PowerShell 5.1.26100.9818。所有命令均为干净命令调用，退出码逐一登记。

### 3.1 指纹工具（实测）

| 命令 | 退出码 | 关键输出 |
|---|---|---|
| `powershell -File tools/m22-s5-fingerprint.ps1 -SelfTest` | 0 | `FINGERPRINT_SELF_TEST_GREEN files=1 fp=be209e43…`（自检覆盖：同树两次同值、改内容变值、`__pycache__` 被排除） |
| `powershell -File tools/m22-s5-fingerprint.ps1` | 0 | `FINGERPRINT 8e5a1525… files=1895`；紧接着复跑同值 ⇒ **确定性成立**，耗时 ~3 s |
| `… -AsJson` | 0 | `{"Fingerprint":"2822c665…","FileCount":1903,…}` |
| `… -SaveFile <p>` 然后 `… -TestFile <p>` | 0 | `FINGERPRINT_MATCH 2822c665… files=1903` |
| `… -TestFile <伪造 deadbeef>` | **2** | `FINGERPRINT_MISMATCH saved=deadbeef current=2822c665…` |
| `… -TestFile <不存在的路径>` | **3** | `FINGERPRINT_NO_SAVED_STATE` |

`fileCount` 随树变化（1895 → 1903）是因为电池与临时探针在 `target/tmp` 之外的文件数变了；`target/tmp` 内的写入**不**改变指纹（见 §3.4 反证）。

### 3.2 单实例锁（实测，拒绝路径）

后台 fast 档运行中，另起一个 probes 档 ⇒ 立即被拒：

```
DEV_BATTERY_LOCKED pid=20900 lock=D:\SorftWare\sico-m22-bootstrap\target\tmp\m22-s5-dev\battery.lock
another instance of validate-m22-s5-dev.ps1 is running; terminate it before starting one
PROBES_EXIT=2
```

### 3.3 fast 档（实测，两轮）

**第一轮**（`-Set fast -Fresh`，8m56s 墙钟）：逐拍 exit 落盘正确，但汇总块全部打印 `exit=-` ⇒ **本片自造缺陷 #1**：汇总读的 `$done` 哈希只在加载旧状态时填充，本轮执行的拍没有写回 ⇒ 修法 = `Invoke-DevStep` 落盘后同步 `$done[$name] = $record`。这是 STEP-0504「验证基础设施自身的缺陷必须写进记录」的同一族（**汇总的红绿与真实结果同时出现**），已修并复跑。

**第二轮**（修后 `-Set fast -Fresh`，仍在跑时记录，随后补记）：

| 拍 | 秒 | exit |
|---|---|---|
| `fmt (workspace)` | 2 | 0 |
| `fmt (runner)` | 5 | **1** |
| `comment-discipline` | 0 | 0 |
| `match-site-census` | 0 | 0 |
| `emitter-vocabulary-census` | 523 | 0 |
| `git-diff-check` | 0 | 0 |

汇总块（修缺陷 #1 后）实测正确：`DEV_SUBSET_RED 1/6 set=fast fingerprint=2822c665…`，只列 `fmt (runner)` 一条红，其余五拍逐拍 exit 正确打印 ⇒ **逐拍落盘 + 汇总 + 跳过逻辑三者一致**。

**唯一红 = 既有红**：`fmt (runner)` exit 1：`runner/sico-runner/tests/selfhost_compiler.rs:12012` 一处 `println!` 未过 rustfmt（`EMITTER... EMIT_ONLY functions=…` 行）。该文件是**他人热文件**，本片只读不写 ⇒ 如实登记，不在本片修。

**本片观测到一条并发输入风险（不是工具缺陷，登记为分层工具的使用前提）**：`emitter-vocabulary-census` 在第一轮判红（`EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S`，524 s），第二轮判绿（`EMITTER_VOCABULARY_CURRENT`，523 s），而 `selfhost/emitter-vocabulary-v1.json` 与 `selfhost/codegen.sico` 在两轮之间**均未变**（mtime 15:41 / 15:52，都早于两轮）。变的是该 census 的**另一个输入** `selfhost/match-component-coverage-v1.json`（账本），mtime **17:53:13** = 两轮之间 ⇒ 该账本在本片跑电池期间被**他人并发改写**（`git status` 显示 ` M`）⇒ census 重算的 artifact 随账本变化 ⇒ 第一轮「不再描述冻结 S」为**当时真实判定**，第二轮「现行」同样是**当时真实判定**，两者不矛盾，但**并发改账本时电池结果不可引用**。这与 handoff-m22 的单实例纪律同族：**电池要求独占机器与独占输入**，本片的单实例 PID 锁只防本脚本的并发实例，防不了他人改输入文件 ⇒ 登记为使用前提，不在本片扩大锁的范围。

### 3.4 指纹闸门（实测）

第一轮 fast 结束后我改了 `validate-m22-s5-dev.ps1`（缺陷 #1）⇒ 树指纹变化 ⇒ 第二轮必须 `-Fresh`。反向验证：`target/tmp` 内的 `fp-test.json` / `fp-stale.json` / `probes-run.out` / 日志 / 状态文件的写入**从未**改变指纹值（1895 与 1903 两次读数都发生在这些文件存在期间，指纹只随 `target/tmp` **之外**的文件变化）⇒ **自污染修复承重**，实测成立。

### 3.5 probes / firmware / all 档：**未跑完，如实登记**

**probes 档（本片实测，部分阻塞）**

| 拍 | 结果 | 证据 |
|---|---|---|
| `probe-frontend-frontier` | **未完成（阻塞）** | 通过 dev runner 跑 `-Set probes` ⇒ 该拍跑了 **23+ 分钟仍无进展**（历史单点 681 s ≈ 11 min）。实测探针 PowerShell 进程**存活但无任何子进程**（15 s 两次采样均 `NO_CHILDREN`），宿主上**无 cargo / sico / rustc** 在跑 ⇒ 进程**卡死（wedged）**，非在算。**根因 = 宿主争用**：本片跑电池期间同 swarm 的其他代理并发产出 STEP-0562/0564/0566/0568/0569/0570 且改写 `selfhost/match-component-coverage-v1.json`（§3.3），同一 `target/` 被多方争用。处置：终止卡死实例 + 清孤儿锁（持锁 PID 11784 实测已死 ⇒ 锁为孤儿，已清），**不伪装绿**。 |
| `probe-ir-plan-frontier` | **绿（绕开 runner 直跑取计时）** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` ⇒ `PLAN_FRONTIER_UNCHANGED`（8/8 用例 OK：3 planned + 5 refused 逐条与期望一致），**exit 0，60 s**（历史 35 s，偏慢 ⇒ 同一争用）。**注意**：这是**绕开 dev runner 直跑**取的计时，不是 `-Set probes` 那一轮跑出来的（那一轮卡死在第一步）⇒ 计时成立、但它不是分档 runner 的完整一轮证据。 |
| `match-site-coverage` | **未跑** | 排在前两拍之后 ⇒ runner 卡死在第一拍，本片窗口内**未执行**。 |

**firmware 档（环境阻塞，测不了）**

`cargo check --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` 实测失败：`error occurred in cc-rs: failed to find tool "gcc.exe": program not found`。本机 `gcc` 不在 PATH、`target/tooling/` 只有 python 与 wasmtime ⇒ **runner 测试二进制无法重链接**。`runner/sico-runner/target/debug/deps/selfhost_compiler-3ee9bc92fdb5f988.exe`（11-05 14:31）虽在，但 cargo 要先过 build script 的 cc-rs ⇒ 绕开 cargo 直接跑既有二进制**不等于** `firmware-selfhost` 拍（该拍命令是 `cargo test …`）⇒ **不伪装绿，登记为「本机测不了」**。

**all 档（未跑）**

含 `ci-one-key`（历史 ~2600 s）+ 两个 STEP 验证器 + clippy，量级 ~2.7 h；且它包含跑不了的 `firmware-selfhost`（上文）⇒ 本片窗口内不可能取到完整计时。

### 3.6 计时对账（§10.1.2 目标 vs 实测）

| 档 | 目标 | 实测 | 判读 |
|---|---|---|---|
| `fast` | ≈ 1 min | 其余 5 拍合计 **7 s**；`emitter-vocabulary-census` 单拍实测 **523 s**（第二轮，无本机 cargo 争用） | **不达标，如实登记**：单是该拍就 8.7× 于整档目标。该拍要向 Rust oracle 逐站点问 canonical IR（约 8–9 min 量级）⇒ **§10.1.2 的「fast ≈ 1 min」在当前树上无法成立**。要达标只能把该拍移出 fast（改成 probes 的首拍）或换更便宜的口径——两者都改判据节奏，**须 owner 裁决，本片不擅自改**。 |
| `probes` | ≈ 12 min | `probe-ir-plan-frontier` 直跑 **60 s 绿**；`probe-frontend-frontier` **卡死 23+ min 未完成**；`match-site-coverage` **未跑** | **未取到完整分档计时**：runner 那一轮卡死在第一步（宿主争用）⇒ 只有绕开 runner 直跑的 `probe-ir-plan-frontier` 60 s 一个实测点。12 min 目标**无法用本片证据证实或证伪** |
| `firmware` | ≈ 28 min | **测不了**（缺 gcc，§3.5） | STEP-0549 历史单点 `firmware-selfhost` 2,061 s ≈ 34 min ⇒ 28 min 目标与本机历史量级相符，**但本机现在重编译不了 ⇒ 无法复现** |
| `all` | ≈ 2.7 h | 未跑 | 与 `validate-m22-s5.ps1` 15 拍同量级，历史记录支持；本片未实测 |

## 4. 与 §10.1.2 自述的逐条对账

| §10.1.2 / §10.2 的声称 | 本片实测 | 结论 |
|---|---|---|
| `tools/validate-m22-s5-dev.ps1` 已落地（§10.2 标 `[x]`） | 本片前 `tools/` 下**不存在**该文件 | **文档与事实不符** ⇒ 本片补齐 |
| `tools/m22-s5-fingerprint.ps1` 已落地（§10.2 标 `[x]`） | 同上，**不存在** | **文档与事实不符** ⇒ 本片补齐 |
| `-Set fast\|probes\|firmware\|all` 四档 | `-List` 实测四档齐全，拍名与命令逐项取自 `validate-m22-s5.ps1` | **成立** |
| fast ≈ 1 min / probes ≈ 12 min / firmware ≈ 28 min | fast 其余 5 拍 7 s、`emitter-vocabulary-census` 单拍 523 s ⇒ fast **不达标**；probes 仅一个绕开 runner 的 60 s 实测点；firmware 本机测不了 | **量级声称未经本片完整证实**，§3.6 逐档登记 |
| 指纹排除 `target` / `__pycache__` / `*.pyc` 三项是承重件 | `-SelfTest` 实测 `__pycache__` 排除生效；`target/tmp` 写入不改变指纹（§3.4） | **成立** |
| 指纹变了就拒绝复用旧绿 | 状态指纹 ≠ 树指纹 ⇒ exit 2 + `-Fresh` 提示；`-TestFile` 不匹配 ⇒ exit 2 | **成立** |
| 分层只改节奏，不降判据 | 拍名 / 命令 / 顺序逐项同 `validate-m22-s5.ps1`；失败仍 exit 1；开发期子集显式打印「NOT exit evidence and closes no gate」 | **成立** |
| 可续跑封装器入库（§10.2 仍标 `[ ]`） | 状态文件 + 逐拍落盘 + 指纹闸门已进 `tools/`（不再是 `target/tmp` 里的临时脚本） | **本片补齐该条的前提**；但 §10.2 的 `[ ]` 文本本片**不改**（评审者统一维护文档） |

## 5. 诚实收窄

- **本片不改任何 gate 文本、不翻门、不改证据等级**；M22 仍 NO-GO；不 commit、不 push。
- `tools/validate-m22-s5.ps1` 本体**零改动**（`git diff --stat` 空）。
- 本片新建的只有三个文件：两个工具 + 本 STEP 记录。工作树里其余脏文件（`.github/workflows/m22-w1-ci.yml`、`selfhost/match-component-coverage-v1.json`、`tools/probe-m22-corpus-parity.ps1` 及多个 `STEP-056x`/`m22-*-2026-11-06.md`）**均为同 swarm 其他代理的并发产物，本片未触碰**。
- `fmt (runner)` 红（`selfhost_compiler.rs:12012` 一处 `println!` 未过 rustfmt）**为既有红**，该文件是他人热文件，本片只读不写、未修。
- `emitter-vocabulary-census` 的红→绿翻转**不是工具缺陷也不是门翻转**，是它的账本输入被他人并发改写（§3.3）⇒ 已归因为「电池结果要求独占输入」的使用前提。
- **fast 档的「≈ 1 min」目标在当前树上不成立**：单 `emitter-vocabulary-census` 一拍就 523 s。本片**没有**为了凑目标而把该拍挪出 fast（那会改拍结构），也**没有**改判据 ⇒ 目标与实现的差距如实留给 owner 裁决。
- **probes 档没有完整一轮的绿**：runner 那一轮卡死在第一步（宿主争用，已终止 + 清孤儿锁），唯一实测点 `probe-ir-plan-frontier` 是**绕开 runner 直跑**取的 ⇒ 它证明该拍本身绿，但**不等于** `-Set probes` 完整一轮跑通。
- **firmware 档本机测不了**（缺 gcc ⇒ cc-rs 失败）⇒ 本片**没有**任何 firmware 绿灯，也**没有**绕开 cargo 跑既有二进制冒充该拍。
- **all 档未跑**（~2.7 h 且含跑不了的 firmware 拍）。
- AGENTS.md §10.2 那条 `[x]` 在「两个工具文件存在」这一项上**曾是错的**；本片把文件补齐并实测，但**没有**把 §10.1.2 / §10.2 的文字改写为「本片已验证」（文档统一维护归评审者）。

## 6. 未跑项（诚实登记）

- `probes` 档**完整一轮**：**未跑通**（`probe-frontend-frontier` 卡死，`match-site-coverage` 未执行）。
- `probe-frontend-frontier` 与 `match-site-coverage` 的本片计时：**未取到**。
- `firmware` 档：**本机测不了**（`cc-rs` 找不到 `gcc.exe`）。
- `all` 档完整 15 拍：**未跑**。
- fast 档在**独占机器 + 独占输入**条件下的量级：**未复跑取证**（两轮都撞上他人并发的 cargo / 账本改写）。
- 指纹在 `.git` 上的处理：`.git` 是 worktree 指针文件（60 B），本片按普通文件哈希；**未做跨 worktree 一致性验证**。
- 单实例锁只防本脚本并发实例，**不防**他人改输入文件或他人进程争用 `target/` ⇒ 这两类并发风险**未在本片中设防**，只登记为使用前提。
