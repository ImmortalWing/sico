# M20–M21 代码质量审查（2026-09-20）

> - 审查对象：分支 `dev` @ `acb5113`；范围 = M20（cross-platform runtime / language v1 batch 1 / §13 完成度审计）与 M21（DX + 标准库 batch 2 + language v1 batch 2）
> - 配套文件：[`m22-quality-review-2026-09-20.md`](./m22-quality-review-2026-09-20.md)（M22 自举线与文档纪律；本报告的实测环境说明见其 §1.2）
> - 证据等级：`internal-audit` + `measured`（§2 的实测项已在本机执行）
> - 本报告为一次性审查产物，不对应任何 STEP 编号，不构成里程碑 GO/NO-GO 判定

## 1. 判定

| 里程碑 | 判定 |
|---|---|
| **M20** | **审计诚实、交付残缺。** `STEP-0170:3` 记 NO-GO、平台侧无越权声明（`docs/reports/desktop-platform-parity-v0.md:15-19`），但 §3.1 承诺的语言 v1 冻结清单里只落地了中缀 `==`；byte/text 访问、保留字迁移、for/闭包全部外溢到 M21，且 `SYNTAX.md`/`SEMANTICS.md` 停在 2026-07-17 未随语言变更同步。 |
| **M21** | **真四层实现，质量高于 M20 批次；但"配套面"欠账严重。** stdlib batch 2 + `for` + `expr?` + `List[I64/U64]` 在 parser/semantics/IR/codegen 四层都落地并有真 runner 逐字节证据；formatter、语料、保留字诊断三项基本没做，且 `STEP-0177:3` 的 6/6 GO 中 gate 3/4（诊断/LSP）证据不足。 |

基线需要说清楚，以免被读成"全线告急"：**Rust 侧根 workspace 在本机 MSVC 变体上 388/0 全绿**（§2），M20/M21 的缺陷集中在"配套面"（formatter、诊断、语料、gate）而不是内核崩溃。反过来，实测复现的失败里**唯一属于产品缺陷的一条**是 §3.1 的语料冻结完整性——它发生在文档主张"已闭合"的 M22 侧；其余红项是环境与门自身质量（§2 末两行、§3.9）。

## 2. 实测项（本机，2026-09-20）

环境：rustc/cargo 1.98.0（GNU 与 MSVC 双装）、`--locked`（前置 `cargo fetch --locked`，故 `-p sico-semantics` 与 fmt 用 `--offline` 通过，全量则放开 `--offline`）、wasmtime v46.0.1 前置件 SHA256 校验通过。
**GNU 目标在本机不可建**：`ring v0.17.14` 构建需 `dlltool.exe`（全盘无 MinGW binutils，`target/tooling/msys2-binutils/` 不存在），故 M20/M21/M22 的 runner 与 workspace 测试全部在 **`x86_64-pc-windows-msvc`** 上执行 —— 与文档记录的 "Windows x64 GNU" 证据不是同一 ABI 主机变体。

| 项 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | **PASS**（GNU，exit 0） |
| `cargo test --locked --offline -p sico-semantics`（M21 语义 oracle） | **PASS**，17 项（6 + 11），0 失败 |
| 根 workspace 全量 `cargo test --locked --workspace --all-targets --all-features --no-fail-fast`（MSVC，`SICO_TEST_WASMTIME` 已注入） | **PASS**：91 个测试二进制、**388 通过 / 0 失败 / 6 忽略**（6 项全部是 `generate_*` / `update_wasm_snapshots` 这类维护者手动生成器）；exit 0。注意 `--all-targets` 不含 doc-test，`--workspace` 不含 runner 独立 workspace |
| M22 八个 selfhost 套件（runner workspace，MSVC，`--no-fail-fast`） | **22 通过 / 1 失败**；唯一失败为 `bootstrap_bundle::frozen_corpus_manifest_is_complete_and_forms_a_canonical_bundle`（见 §3.1） |
| runner workspace 全量 `cargo test --locked --manifest-path runner/sico-runner/Cargo.toml --all-targets --no-fail-fast`（MSVC，默认并行） | **151 通过 / 4 失败**，exit 101。分解：1 条真实完整性缺陷（§3.1 语料）+ 2 条 `Command::new("python")` 落到 Windows Store 存根（`tests/runner.rs:1325,1380`；`SICO_PYTHON` 在 Rust 侧零消费者，只有 `run-ci.ps1:27-28` 用 PATH 前置绕过）+ 1 条进程级 RSS 断言被同二进制内并行兄弟测试污染（`:2580` 硬编码 +16 MiB 容差；`--test-threads=1` 下 PASS，`run-ci.ps1:57` 正是靠该参数绕开，而 AGENTS.md §8 给的 runner 命令漏了它） |
| `tools/validate-module-boundaries.ps1` | **PASS**：`MODULE_BOUNDARIES_OK packages=29 exceptions=4` |
| `tools/validate-step-0131.ps1` | **PASS**：`OK (matrix fixture, corpus paths, registry surface)` |
| `tools/validate-step-0156.ps1` | **PASS**：`OK (cross-host matrix, harness page, pinned engine)` + `UI renderer corpus OK (8/8)` |
| `tools/validate-step-0124.ps1`（M14–M25 规划合同） | **FAIL（实测复现）**：`tools/validate-step-0124.ps1:30` 用子串 `'*no STEP numbers reserved*'` 断言每份计划的状态行，而 `docs/plans/M22-compiler-self-host.md:3` 现写 "**No later STEP numbers are reserved.**" —— 多了 `later`/`are` 两个词就不匹配 → `throw "M22-compiler-self-host.md must keep implementation STEP numbers unreserved"`。措辞漂移由提交 `1c6c04b`（"extend M22 lowering coverage"）引入。**HEAD 上 AGENTS.md §8 规定的"文档改动最小校验"是红的。** |
| `tools/validate-step-0245.ps1`（M22 回归门） | **FAIL（环境前置，非代码）**：`:5` 硬钉 `RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-gnu'`，本机 GNU 无 `gcc.exe`/`dlltool.exe` → `windows-sys`/`ring` 构建失败 → `THREW: M22 checker/parser/compiler regression failed`。同一 cargo 内容在 MSVC 变体上按上行分拆执行。 |


## 3. 缺陷清单（按严重度）

### 3.1 【实测复现·最高优先】M14 语料与仓库内容不一致，fresh clone 上 CI 必红

- `selfhost/corpus-v0.json` 中 `tests/end-to-end/block-solver.sico` 条目为 `bytes=49212`、`source_sha256=c242220192ec…ab31`。
- 仓库该文件（`git cat-file -s dev:…` = **47898**，1314 行，**0 个 CR 字节**，纯 LF）。把该文件逐行改成 CRLF 后：大小恰为 **49212**，sha256 恰为 **c242220192ec…ab31**。
- 即：**该条目冻结的是某主机上 CRLF 工作副本，而不是仓库内容**。其余 214 条目字节与 sha 全部吻合（逐条实测）。
- 后果：按仓库自身 `.gitattributes`（`* text=auto eol=lf`）做干净检出（任何 fresh clone / CI）时，`runner/sico-runner/tests/bootstrap_bundle.rs:174` 的 `assert_eq!(entry.bytes, source.len())` 失败 → `tools/validate-step-0245.ps1` 与 `tools/run-ci.ps1` 第 5/6 步 **RED**。已在 HEAD 实测复现。
- 为什么别的套件没抓到：`selfhost_checker.rs` 读取语料源文件却**不校验 `source_sha256`**（对比 `bootstrap_bundle.rs:175` 校验），见 M22 报告 §4 P2。
- 修复：把该 fixture 规范化为 LF 并重跑 `tools/update-m22-corpus.ps1` 重冻结；语料完整性检查应改为读 `git cat-file`（仓库字节）而非工作树字节，或在 CI 增加"fresh clone 后 `git status` 必须干净"的断言。
- 关联风险：这个文件正是 M14/M18 的俄罗斯方块离线求解器验收 fixture（AGENTS.md §5/§6 指定的 acceptance oracle），漂移的是最关键的那个。

### 3.2 P0 正确性：`check` 通过但 `build` 拒绝（M14 定义性红线）

`sico-semantics` 的"区域内绑定随区域结束失效"机制是**空转**的：

- `crates/sico-semantics/src/lib.rs` 全文件中 `open_scopes` 只出现在 `:938`（声明）、`:943`/`:1105`/`:1110`（retain）、`:1011`/`:1063`（向已存在 scope 追加名字）——**没有任何 `open_scopes.push(...)`**，故该 vec 恒空。
- `region_close_map`（`:1157-1174`）只为 `While | If | Match` 建关闭映射，**不含 `LineKind::For`**；`:951` 的 For 分支注释还写着"loop variable binds … until the region closes"（`:952-955`）。
- 后果：`end if` / `end for` 之后继续读或 `set` 该区域内绑定的变量，`sico check` 放行，而 IR 侧移除 cell（`crates/sico-ir/src/lower.rs:1367`）→ `build` 类型化拒绝。同时违反 RFC-0046 D1 与 AGENTS.md §5（"不得出现未声明的 check/build/run 缺口"）。
- 影响面不止 M21 的 `for`：M14 的 `if`/`while` 同样无作用域回收。

### 3.3 P1：新语法缺 formatter 支持，`sico format` 会改写已接受源码

`crates/sico-format/src/lib.rs:180-197` 的 `opener_close` 分支只有 `Record/Enum/Capability/Resource/Interface/Match/If/Using/Task/Function`，**无 `While`、无 `For`**（While 是更早就缺）。因此 while/for 体不缩进、`end for` 不 dedent，格式化会改动已通过 check 的源码。`tests/formatter/policy.snap` 内亦无 while/for 用例。直接违反 RFC-0046 §6.5 的 formatter 幂等承诺。

### 3.4 P1：RFC-0046 契约偏离（已接受合同未落地）

1. **保留字诊断缺失**：RFC-0046 §4/§5 承诺 `for`/`in` 冲突标识符给带 span 的 E1xxx；全仓无 reserved 诊断（`crates/sico-lexer/src/lib.rs:387-388`、`crates/sico-diagnostics/src/lib.rs:31-147`），parser 只在 module/use/param 处校验 Identifier（`crates/sico-parser/src/lib.rs:486,520,522,854-887`），输出错位文案。
2. **limit+1 未闭合**：唯一预算 `MAX_PARSE_DEPTH = 256`（`crates/sico-parser/src/lib.rs:11`），但 `NestingLimitExceeded/MissingBlockClose/MismatchedClose`（`:164-172`）**未注册诊断码** → `sico check` 走 Debug 打印 + `EXIT_TOOL_ERROR`（`crates/sico-cli/src/lib.rs:1087-1098`），LSP 另造未注册码 `E1999`（`crates/sico-language-server/src/lib.rs:306`）。AGENTS.md §7 明确要求 limit+1/截断/尾随数据必须显式拒绝。
3. **RFC-0045 §4.5 "repeated builds byte-identical" 只测了 repeated *runs***，且第二次断言弱化为 `starts_with`（`runner/sico-runner/tests/stdlib_batch2.rs:88-100`）。
4. **STEP 记录自相矛盾**：`docs/steps/STEP-0175-language-v1-batch2-for-loops-error-propagation.md:26-29` 称 `for` 主语仅 `List[Text]`、`try` 只到 check 期；`docs/steps/STEP-0175-language-v1-batch2.md:13-19` 说法相反。代码证据（`crates/sico-ir/src/lower.rs:2538-2552`）表明 `try` 确实 lowers。两个同 STEP-0175 文件（M22 报告 §4 P1-E 已列）不只是编号重复，而且内容互相否证。支持矩阵也有重复行（`for-loops` 与 `for-loops-iterables` 等三对）。

### 3.5 P1：内存与健壮性（Rust 侧）

1. `text.format` 的模板扫描在 `stdlib.rs:1490-1503` 处对 `t[j]` 的 `I32Load8U` 前置虽有 `j < len`（`:1492-1495`），但 Wasm `I32And` **不短路**，故 `j == len` 时仍会发生 1 字节越界读（结果被丢弃）；只有当模板恰止于线性内存顶端才 trap。**此条我未构造 trap 用例，按疑点记录。**
2. `helper_dependencies` 每次调用 `Box::leak`（`crates/sico-codegen-wasm/src/lib.rs:508-514`）。注释声明"受闭合文法约束（11 op × ≤25 实例化）"，该界限只对**单次编译**成立；在 LSP/registry 这类长驻进程里泄漏随编译次数累积。准确说法是"每次编译泄漏若干字符串、进程生命周期内不回收"，不是无界单编译泄漏。
3. `sico check --json` 对非标量边界 span 直接 `.unwrap()` panic（`crates/sico-cli/src/lib.rs:1103` ← `crates/sico-diagnostics/src/lib.rs:173-183`）。
4. 三张 helper 表无交叉一致性测试，不一致即触发编译器 `unreachable!`（`stdlib.rs:41,104`）。
5. `sort` 为 O(n²)×O(len)（`stdlib.rs:1567-1650`），符合契约，但语料最大仅 4 元素，无规模/燃料证据。

### 3.6 P2：诊断/LSP 质量（M21 §3.3）

- LSP `KEYWORDS` 补全缺 `for`/`in`（也缺 `while/set/break/continue/try`），且其测试只断言"无重复"（`:819-838,1341`）不断言完整性。
- `action_hint` 仅 CLI explain 消费（`crates/sico-cli/src/lib.rs:926`），LSP/AI 诊断面不带 hint → M21 的"诊断质量"目标只完成一半。
- `lsp_range(...)?` 位于 `filter_map` 中，静默丢弃诊断；内部错误被压成常量串 `.map_err(|_| …invalid)`（`crates/sico-language-server/src/lib.rs:530-536,571-575`）。
- E2001 一码 11 用（语义粒度过粗）。
- `docs/rfc/RFC-0001…:66` 的激活码表仍停在 E1013。
- 正面：UTF-16/CRLF 处理本身正确——`sico-source` 的 `line_col` 显式拒绝 `\r\n` 中间位置，LSP 有 emoji（😀）用例。

### 3.7 P2：平台证据弱于措辞

- 未发现越权声明（M20 的整体结论是 NO-GO，登记正确）。
- 但 `tools/validate-step-0156.ps1:7-16`（cross-host matrix）只比对 fixture **自报的 hex** 与文件存在性，**不执行任何 runner**；native 侧实际只有 Windows+Edge 单机（`docs/evidence/m15/cross-host-matrix.json:5-9`），却被 `STEP-0177:18` 计入 CI 绿。
- 叠加 §2 的 ABI 事实：文档记录的 "Windows x64 GNU runtime-verified" 与本次 "MSVC 主机变体" 不可互换，`AGENTS.md §8` 要求报告精确平台。

### 3.8 P2：新语法边界用例缺口（实测可复算的最小集合）

`for`：空表 / 单元素 / 循环内 `break`·`continue`（List 与 Set 两种）/ 嵌套 `for` / 循环外引用变量（即 §3.2 的正反例）。
`text.compare`：前缀更短分支（`stdlib.rs:1326-1335`）。
`sort`：等键稳定性。
`format`：`{}`、末尾裸 `{`、下标 ≥100000（`:1476-1481`）、`arg_count = 0`。
`char_at`：索引 ≥2^32 守卫（`crates/sico-codegen-wasm/src/lib.rs:4755`）；`bytes.at` index = 2^64−1。
数值空表上的 `min`/`max`/`sort`。
结构性阻塞：新语法在 `syntax-candidates/`、`syntax-mutations/` 内**零条目**，而 `crates/sico-format/src/lib.rs:275,297` 把计数硬编码为 58/12 → 补语料会反向撞断既有测试，这是补测的真实阻碍。

### 3.9 P1：治理 gate 自身是子串匹配，HEAD 上已实测为红

- `validate-step-0124.ps1:30` 与 `validate-step-0245.ps1` 的 grep 门都把"合同仍然有效"编码成**计划/测试正文里的一个字面量**。这类断言只检查"字符串还在"，不检查"性质还在"，因此：正常的措辞修订会误红（§2 实测：M22 计划状态行改一个字即 throw），而死代码可以让它误绿（M22 报告 §4 P2 已列 `(116, 34, 65, 0)` 一例）。
- 后果不是 cosmetic：AGENTS.md §8 把 `validate-step-0124.ps1` 定为文档级改动的**最小**校验，它在 dev 的 HEAD 上就是红的 → 后续任何 M14–M25 规划改动要么撞上假失败，要么被跳过。
- 与 §2 的 `validate-step-0245.ps1` GNU 硬钉叠加后，dev 上"两条被文档指定的门"一条因措辞漂移红、一条因未文档化的 binutils 前置红，**没有一条能在 fresh clone 上按其宣称的方式跑通**。
- 修复方向：把子串改成结构化字段（如计划 front-matter 的 `step-reservation: unreserved`），并给 validator 加"自身失败消息含文件+行号"的约定；0245 的 `--toolchain` 变成可参数化并让 `run-ci.ps1` 显式报告跳过的步骤而非静默。
- 同一族的另一处：**复算命令本身未被文档化**。`run-ci.ps1:57` 用 `--test-threads=1` 跑 runner 测试（规避 §2 表里的进程级 RSS 断言），而 AGENTS.md §8 给读者的 runner 命令行没有这个参数；`run-ci.ps1:27` 用 PATH 前置喂 `python`，而测试从不读 `SICO_PYTHON`。照文档命令跑 → 3 条与产品无关的红。修法见 M22 报告 §4 P1-J。

## 4. 值得保留的部分

- `bytes.at/equal/compare/char_at`、`sort/min/max/format` 是**真实现**：`crates/sico-semantics/src/lib.rs:3474-3495` → `crates/sico-ir/src/lib.rs:1424-1428` → `crates/sico-codegen-wasm/src/lib.rs:4689-4787` + `stdlib.rs:1272-1400/1408-1720`，并有 `stdlib_batch2.rs` 的真 runner 逐字节断言。
- `for` 走 parser（`crates/sico-parser/src/lib.rs:742,768,929`）→ semantics（`:951-1015`）→ IR 脱糖（`crates/sico-ir/src/lower.rs:1102-1415`）真链路，codegen 无需新机制。
- `expr?` 真落地：lexer `:417`、semantics `:1038-1060`、`lower.rs:1817-1930` 复用 match/return；M22 报告曾担心的"只到 check 期"不成立（STEP-0175 的一份记录写错了，代码是对的）。
- 闭包/迭代器：M20 计划里承诺，实际从未落地——**并被 M21/M23 顺延**，没有伪装成已完成。
- 平台证据没有洗白：`STEP-0170` NO-GO 与 `desktop-platform-parity-v0.md` 的单机限定都是正确的诚实降级。

## 5. 建议（按杠杆排序）

1. **立刻重冻结 `block-solver.sico` 语料条目**（<0.5 天）：LF 规范化 + 重跑 `tools/update-m22-corpus.ps1`；否则 HEAD 在任何 fresh clone 上 CI 红，且这是唯一被实测复现的失败。
2. **修 §3.2 作用域空转**（0.5–1 天）：补 `open_scopes.push(...)`、`region_close_map` 加 For，并加"循环变量出域"正反例。这是当前唯一已知的 check/build 分歧类缺陷，直接决定 M14 能否 GO。
3. **formatter 补 While/For + 幂等语料**（0.5 天）。
4. **把 3 个未注册 ParseErrorKind 与 reserved-word 变成 E1xxx + action_hint**（1 天），去掉 CLI 的 Debug 泄漏与 LSP 的 `E1999`，同步 `RFC-0001` 激活表。
5. **健壮性小修三件**（0.5 天）：`syntax_check_json` 去 `unwrap`；`helper_dependencies` 改静态表；`text.format` 的载入前置独立分支（并用一个贴顶内存模板做 trap 用例证伪/证实 §3.5.1）。
6. **合并重复 STEP-0175 与重复矩阵行**，然后按 §3.8 补边界用例；同时把 `sico-format` 测试里硬编码的 58/12 改成从语料目录动态计数，否则补测无法进行。
7. **让 `selfhost_checker.rs` 也校验 `source_sha256`**，使 §3.1 这类漂移在 M22 侧也能被早发现（当前只有 `bootstrap_bundle` 一处把关）。
8. **平台口径统一**：明确 "Windows x64 GNU" 与 "MSVC 变体" 的证据关系，并把 binutils 写成 `docs/development` 的显式前置条件 + 提供 `tools/ensure-binutils.ps1`（现在只有 `ensure-python`/`ensure-wasmtime`，而 `run-ci.ps1:23` 对缺失目录静默跳过）。
9. **将 §3.9 的两处子串 gate 改成结构化断言**（<0.5 天）：`validate-step-0124.ps1:30` 的措辞漂移误红在 HEAD 上实测可复现，改一行匹配串即可复绿；同时把 `validate-step-0245.ps1:5` 的工具链变成参数并允许显式声明变体。

## 6. 主审亲自复核（不依赖子代理）的条目

§3.1（全部数值：47898/49212/0 CR/CRLF-变体 sha 逐一复算，214 条目逐条比对）、§3.2（`open_scopes` 无 push 全文件 grep 确认；`region_close_map:1157-1174` 无 For 直接读到）、§3.3（`opener_close:180-197` 直接读到无 While/For）、§3.5.2（`Box::leak` 与注释原文，严重度已下调）、§3.9（`validate-step-0124.ps1:30` 与 `M22-compiler-self-host.md:3` 原文逐字比对，并 `git log -S` 定位到引入提交 `1c6c04b`）、§2 全部实测输出（本次在同一次会话内直接执行并保留 stdout：`fmt`/`-p sico-semantics`/M22 八套件/根 workspace 91 二进制/4 个 validator；原始日志在本机 `.audit-logs/` 下，未提交，提交前会贴出关键计数）。

## 7. 未验证 / 遗留

- 根 workspace 全量测试与 4 个 PowerShell validator 的实测结果已回填至 §2 表格。
- 未跑：`cargo clippy --workspace --all-targets --all-features -- -D warnings`（GNU 路线本机不可建，MSVC 全量 clippy 未列入本次预算）；`tools/run-ci.ps1` 全 11 步同理未跑。因此"CI GREEN"在本机只能分解为逐条实测，不能整体主张。
- doc-test 未包含在 `--all-targets` 内，本次未单独执行 `cargo test --doc`。
- §3.5.1 的越界读未构造 trap 用例，保持"疑点"标签。
- Linux/macOS/Android 侧本次未执行任何 runner；M20 的 NO-GO 结论不因本审查改变。
- 本报告与 M22 报告均为一次性审查产物：不对应 STEP 编号，不更新 STATUS/ROADMAP，其结论需由后续带 STEP 的修复提交承接。
