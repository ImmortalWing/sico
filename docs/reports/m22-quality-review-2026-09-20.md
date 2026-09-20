# M22 代码与前进方案质量审查（2026-09-20）

> - 审查对象：分支 `dev` @ `acb5113`（含 M14→M25 全部在途工作，相对 `main` @ `5a1af08` 领先 37 提交 / +54451 行 / 383 文件）
> - 审查方式：静态代码与文档审阅 + 只读 git 取证 + **本机实测复算**（§1.2、§1.3）；关键指控由主审逐条复核（见 §6）
> - 证据等级：`internal-audit` + `measured`（实测在 `x86_64-pc-windows-msvc` 变体上完成，与文档记录的 "Windows x64 GNU" 非同一 ABI 主机，见 §1.3）
> - 姊妹文件：[`m20-m21-quality-review-2026-09-20.md`](./m20-m21-quality-review-2026-09-20.md)（M20/M21 语言与标准库，含同一实测环境说明）
> - 本报告为一次性审查产物，不对应任何 STEP 编号，不构成里程碑 GO/NO-GO 判定

## 1. 范围与方法

### 1.1 覆盖内容

| 层 | 路径 | 体量 |
|---|---|---|
| 自举前端 | `selfhost/*.sico` | 11938 行（12 文件，`parser.sico` 独占 7608 行） |
| 自举测试 oracle | `runner/sico-runner/tests/selfhost_*.rs` | 2602 行 |
| 冻结语料 | `selfhost/corpus-v0.json`、`selfhost/script-build-corpus-v0.json` | 215 源 / 37+178 构建判定 |
| Rust 内核 | `crates/sico-ir`、`sico-codegen-wasm`、`sico-cli`、`sico-automation-host` | ~37 文件 |
| Runner | `runner/sico-runner/src/`（`lib.rs`/`dap.rs`/`http2.rs`/`scheduler.rs`/`bootstrap.rs`/`win32.rs`） | 32 文件 |
| 过程文档 | `docs/steps/STEP-014x–0245`、`docs/plans/M1x–M2x`、`docs/rfc`、`docs/adr`、`docs/user-guide` | 240 条 STEP 记录 |

### 1.2 实测复算（2026-09-20，本机 Windows x64）

初稿曾因本机缺 pinned 工具链而无法验证；应用户要求补齐环境后完成实测。

| 步骤 | 结果 |
|---|---|
| `rustup toolchain install 1.98.0-x86_64-pc-windows-gnu`（+clippy/rustfmt） | 成功（rustc 1.98.0 / cargo 1.98.0） |
| `cargo fetch --locked`（根 + runner 两个 workspace） | 成功（此前 `--offline` 因 `cc v1.4.4` 未缓存而失败） |
| `tools/ensure-wasmtime.ps1` | 成功：下载并 SHA256 校验 wasmtime v46.0.1-mingw → `target/tooling/wasmtime-v46.0.1-x86_64-mingw/wasmtime.exe` |
| `cargo fmt --all -- --check` | **PASS** |
| `cargo test --locked --offline -p sico-semantics`（Rust 语义 oracle） | **PASS**：17 项（6 + 11），0 失败 |
| `cargo build --locked --offline --manifest-path runner/… --tests`（**GNU**） | **FAIL**：`ring v0.17.14` build script 需 `dlltool.exe`，本机无 MinGW binutils，`target/tooling/msys2-binutils/` 不存在 |
| 8 个 selfhost 套件 + `bootstrap_bundle`（**MSVC 变体**，`--no-fail-fast`） | **22 通过 / 1 失败**；唯一失败见 §4 P0-5 |
| 根 workspace 全量 `cargo test --locked --workspace --all-targets --all-features --no-fail-fast`（MSVC + `SICO_TEST_WASMTIME`） | **PASS**：91 二进制，**388 通过 / 0 失败 / 6 忽略**（忽略项全是 `generate_*`/`update_wasm_snapshots` 维护者生成器），exit 0 |
| runner workspace 全量 `cargo test --locked --manifest-path runner/sico-runner/Cargo.toml --all-targets --no-fail-fast`（MSVC，默认并行） | **151 通过 / 4 失败**，exit 101。四条失败原因各不相同，见 §4 P1-J（其中只有 1 条是真实完整性缺陷） |
| `tools/validate-module-boundaries.ps1` / `validate-step-0131.ps1` / `validate-step-0156.ps1` | **PASS**（`MODULE_BOUNDARIES_OK packages=29`、`OK (matrix fixture, corpus paths, registry surface)`、`OK (cross-host matrix, harness page, pinned engine)` + `UI renderer corpus OK (8/8)`） |
| `tools/validate-step-0124.ps1`（M14–M25 规划合同，AGENTS.md §8 指定的文档级最小校验） | **FAIL（实测复现）**：`:30` 以子串 `'*no STEP numbers reserved*'` 断言计划状态行，`plans/M22-compiler-self-host.md:3` 写作 "No later STEP numbers are reserved." → `throw "…must keep implementation STEP numbers unreserved"`。措辞漂移由 `1c6c04b` 引入。详见 §4 P1-I |
| `tools/validate-step-0245.ps1`（M22 回归门） | **FAIL（环境前置，非代码失败）**：`:5` 硬钉 GNU 工具链，本机 `gcc.exe`/`dlltool.exe` 缺失 → `error calling dlltool 'dlltool.exe': program not found` → `THREW: M22 checker/parser/compiler regression failed`。即 §1.3.1 的未文档化前置条件在这条门上实测撞墙 |

未执行：`tools/run-ci.ps1` 全 11 步（GNU 路线被 binutils 卡住）、workspace clippy（MSVC 全量成本过高）、`cargo test --doc`（不在 `--all-targets` 范围内）。

### 1.3 由实测暴露的环境与可复现性缺陷

1. **官方 CI 在任何 fresh clone 上不可能绿**：`ring` 同在根 workspace 依赖图（`cargo tree -i ring` → `sico-http-provider` → rustls/rcgen），故 GNU 下 `cargo test --workspace` 与 runner 测试一样建不起来。而 `tools/run-ci.ps1:23` 对 `target\tooling\msys2-binutils\mingw64\bin` 只做 `if (Test-Path)` 静默跳过；`tools/` 下只有 `ensure-python.ps1` 与 `ensure-wasmtime.ps1`，**没有任何脚本准备 binutils**，`docs/development/` 与 `docs/user-guide/INSTALLATION.md` 全文未出现 `dlltool`/`binutils`。STEP-0196 的"fresh-host 修复"因此留了未文档化、未自动化的前置条件。
2. **平台变体不可互换**：本次通过项取自 `x86_64-pc-windows-msvc`，STEP 记录写的证据是 "Windows x64 GNU"；按 AGENTS.md §8 两者不得混用。
3. 对初稿的修正：原先写的"9 月证据本机不可复算"部分成立但方向不同——障碍是 binutils 前置缺失（P1），不是测试本身。跑起来后：8 个 selfhost 套件全绿；把 runner workspace 跑全（`--all-targets`）后 151/4，四条中**只有 1 条是真实完整性缺陷**（§4 P0-5），另 3 条是 §4 P1-J 的隐式环境契约与并行污染；根 workspace 91 个二进制 388/0 全绿。
4. "fresh clone 上 CI 不可能绿"有**两条独立**理由，任一单独修好都不足以复绿：(a) 本条上方第 1 点的 GNU 前置缺失（实测：`validate-step-0245.ps1` 因 `dlltool.exe` 缺失直接 throw）；(b) §4 P0-5 的语料字节漂移，它在**任何**ABI 变体上都会让 `bootstrap_bundle` 红。
5. 另外仍然成立：STEP 记录引用的证据路径（如 `STEP-0168:7` → `target/evidence/m19/`）位于 `.gitignore:4` 排除的目录，fresh clone 拿不到；本机 `target/` 的构建产物与 `target/evidence/*` 停在 7 月 21 日。

## 2. 总体判定

一句话：**语义内核与安全边界是真工程；自举路线的推进形状已经在数学上不收敛，而文档层出现了同名词两套含义的失控。**

| 维度 | 评级 | 依据 |
|---|---|---|
| Rust 安全内核（runner/IR/bootstrap bundle） | **强** | §3.1 |
| Sico 语言内自举前端质量 | **弱（结构性）** | §3.2、§4 P0 |
| M22 阶段进度陈述诚实度 | **中上**（主动声明不主张，但阶段标签自相矛盾） | §4 P1-D |
| 过程纪律（STEP 唯一性/合同先行） | **弱** | §4 P1-E |
| 用户手册与可执行面一致性 | **失效** | §4 P1-F |
| 证据可复现性 | **有实测复现的破损** | §1.2、§1.3、§4 P0-5、P1-G、P1-I、P1-J |
| M23–M25 未来计划质量 | **合格**（真计划，但被 M22 标签污染而暂不可判伪） | §5.3 |

## 3. 分区评估

### 3.1 Rust 侧：内核扎实，外围堆积

扎实的部分（应明确保留）：

- `runner/sico-runner/src/bootstrap.rs` 是全仓质量最高的新模块：magic/count/单文件与总量上限、路径规范化、重复与乱序拒绝、SHA-256、`TrailingData`/`Truncated`/`IntegerOverflow` 全覆盖，`checked_add` + `get()`，无一处 `unwrap`；`tests/bootstrap_bundle.rs:83` 精确测到 limit 与 limit+1。
- 一般 CFG 降级是真结构化构造，不是特例拼装：`crates/sico-ir/src/lower.rs:1063-1424`（while/for 脱糖、if-else）使用 `new_block`/`seal_current`/`loop_stack` 与创建序 block id，`LineKind` 分派穷尽，未支持形态返回**类型化 `unsupported`**，无 `_ =>` 静默兜底；`expect("block bound")` 由 `MAX_BLOCKS_PER_FUNCTION = 100_000`（`lib.rs:18`）真实支撑。
- runner 包链接 fail-closed（纯净性/命名空间/arity），limiter 只按已 prepare 数量精确放宽（`lib.rs:1954-1958`）。
- 模块边界契约**没有被放宽**：`tests/architecture/module-boundaries.json` 只新增两个 host 包归属，零新增 exception。
- `STEP-0241` 把评审发现的 cell 遮蔽语义缺陷以 `E-SH-IR-CELL-SHADOW` 显式拒绝关闭，而不是藏 fallback——这是正确的处理方式。

缺陷（详见 §4）：`crates/sico-cli/src/test.rs` 的不可信输入无界读取；发现阶段吞掉 IO 错误造成"假绿"；`#[allow(clippy::too_many_lines)]` 新增约 20 处（12 处无理由），`emit_intrinsic` 已 568 行；15 份近乎相同的 compile_source 临时目录 harness 未抽公共模块。

### 3.2 Sico 侧自举前端：不是编译器，是形状枚举机

- `selfhost/checker.sico:29-41` 只有三段流水线（`compiler_lexer.has_error` → `compiler_parser.syntax_diagnostic` → `compiler_semantics.semantic_diagnostic`）。**未发现**源哈希、文件名字符串、魔数答案表或 base64 内嵌 blob——identity 确实由逐行状态机算出。所以就"是否查表应付测试"而言：**不是查表，但规则泛化半径约等于 1 个样本**。
- `selfhost/compiler.sico` 的 IR 路径是三套字节精确模板机：identity（`:455-504`）、const（`:505-545`）、add（`:546-615`），三处均把 `"source_name"` 写死为 `"identity.sico"`（`:475/:521/:578`）且要求 `sico.bytes.equal(stdin, 规范源码)`。main 总共只认 4 种规范形状（`:410-617`）。
- `acb5113` 提交信息所述属实：父提交把 `answer() -> 42` 的源码与 85 字节 Wasm blob 整体硬编码；该提交部分泛化出 `constant_core_hex`（`selfhost/compiler.sico:379-398`，动态编码函数名与常量），但模块头（`:384`）、函数体前缀（`:395`）、尾部（`:397`）仍是硬骨架，只覆盖零参数非负 Int 常量函数，其余形状 fail-closed 拒绝（`:451`）。
- 字节差分是**真实但范围过窄**：`runner/sico-runner/tests/selfhost_compiler.rs:309-333` 用 7 个字面量（含 63/64、127/128 SLEB 边界、624485、`i64::MAX`）逐字节比对 `sico_codegen_wasm::compile` 并经 wasmparser 校验，另有 3 个必拒用例（`:335-348`）。但比对对象是 **Core Wasm `compile`，不是 S5 出口要求的 Component 字节**；`:284` 的组件级比对只对含 "answer" 的单一夹具触发。
- `selfhost/script-build-corpus-v0.json` 冻结的是 **Rust** 构建结果（37 accepted / 178 refused）；`STEP-0220 §2` 已明确承认"Sico 编译器尚不能编译这 37 个源"。
- 存在**两套并行前端**且互不引用：7608 行单体 `parser.sico`（S3 时代，`selfhost_parser.rs` 1259 行测试伺候）与模块化 `compiler_lexer → compiler_parser → compiler_semantics → checker` 链（S2/S5 实际路径，`checker.sico:24-26`、`compiler.sico:24-25`）。selfhost 总量 11938 行里约 8.2k 行属前者。这是双份维护成本，且没有记录说明哪个是权威。

## 4. 缺陷清单（按严重度）

### P0 — 直接阻塞 S6（自举闭环）

**P0-1 checker 会拒绝自己的源码形状（假阳性，已亲自复核）**
`selfhost/compiler_semantics.sico:271-273`：任何"未缩进且以 `module ` 开头"的行 → 返回 `E8010`。但 Rust 侧 E8010 的作用域是**仅限 CLI 入口文件**——`crates/sico-cli/src/modules.rs:337-350` 的原文是 "the CLI entry file is the program root and does not declare a module"，非入口文件声明 module 是合法的（`use` 才构成链接边）。
后果：S2 checker 必然拒绝 `selfhost/compiler_lexer.sico:1`、`compiler_parser.sico:1`、`compiler_semantics.sico:1`。
未被发现的原因：`selfhost/corpus-v0.json` 中 `selfhost/` 路径出现次数为 **0**，215 语料里 module 形状仅 `tests/end-to-end/modules-report/math_util.sico` 一例。**"215/215 闭合"因此不包含自举前端自身。**

**P0-2 `List[record]` 不在可执行集（根因级）**
`docs/plans/M22-compiler-self-host.md:113` 自陈，`:56` 允许 struct-of-arrays 作为过渡形状；`STEP-0182:37`、`STEP-0183:18` 引 RFC-0046 D4 维持该限制。这正是并行数组、JSON 字符串拼接 AST、7608 行膨胀、逐形状模板比对的**唯一根因**。不解决它，通用 AST 在语言内不可表示，S4/S5 不存在"通用"出口判据。

**P0-3 fuel/栈悬崖在自举规模上从未测量**
默认 `fuel: 10_000_000`、`max_wasm_stack(4 MiB)`（`runner/sico-runner/src/lib.rs:243`、`:1446`）；而现有测试为逐行词法化一个 600 行文件已需 `1e9–5e9` fuel（`runner/sico-runner/tests/selfhost_compiler.rs:78`、`selfhost_checker.rs` 用 `1_000_000_000`，即默认值的 100–500 倍）。`ADR-0015 §8` 禁止失败后再抬限。**编译 ~2500 行以上自身源集所需的预算从未被绘制过。**

**P0-4 S5 目标与当前实现之间差整个 Component ABI**
S5 出口是冻结语料的 **Component 字节全等**（`plans/M22:62`），对应 `crates/sico-codegen-wasm/src/lib.rs:593` `compile_script_program` 的 canonical ABI / arena / custom section；当前只泛化了 Core Wasm 常量函数一条缝。

**P0-5 冻结语料记录的是某主机的 CRLF 工作副本，而非仓库内容（实测复现，HEAD 即失败）**
`selfhost/corpus-v0.json` 中 `tests/end-to-end/block-solver.sico` 条目为 `bytes=49212`、`source_sha256=c242220192ec…ab31`；而该文件在仓库里是 **47898 字节、1314 行、0 个 CR 字节（纯 LF）**。把该文件逐行改成 CRLF 后大小恰为 49212、sha256 恰为清单所记的值——即清单冻结的是 CRLF 副本。
按仓库自身 `.gitattributes`（`* text=auto eol=lf`）做干净检出时 `runner/sico-runner/tests/bootstrap_bundle.rs:174`（`assert_eq!(entry.bytes, source.len())`）必然失败：**实测 `3 passed / 1 failed`，cargo exit 101**。逐条比对 215 条目：仅这 1 条不符，其余 214 条字节与 sha 全对。
连锁后果：`tools/validate-step-0245.ps1` 与 `tools/run-ci.ps1` 的 runner 步骤在 HEAD 上不可能绿 → "S2 declared-subset GO / CI GREEN 11/11" 是**主机相关**证据。
为什么只有 `bootstrap_bundle` 抓到：`selfhost_checker.rs:146` 读源文件却不校验 `source_sha256`（见 §4 P2），所以 checker 套件对这个漂移完全无感。
受影响文件正是 M14/M18 的俄罗斯方块离线求解器验收 fixture（AGENTS.md §5/§6 指定的 acceptance oracle）。
修复：LF 规范化该 fixture + 重跑 `tools/update-m22-corpus.ps1` 重冻结；完整性检查改读 `git cat-file` 的仓库字节而不是工作树字节；并让 `selfhost_checker.rs` 同样校验 sha。

### P1 — 安全、正确性或纪律缺陷

**P1-A `sico test` 违背本仓不可信输入纪律**（`crates/sico-cli/src/test.rs`）
- `:245` 子进程 stdout 用 `read_to_end` **无上限**、`:249` `child.wait()` **无超时**、写 stdin 失败不 kill。同仓 `run.rs:36/216/432` 已示范 `MAX_GUEST_STDIN` + `take(limit+1)`；AGENTS.md §5/§7 明确 guest 输出为不可信数据。
- `:202` `let Ok(entries) = fs::read_dir(dir) else { return; }` 吞掉所有 IO 错误，递归无深度/软链/数量上限 → 权限失败子树会让"0 failed"成为假绿。
- 对 M22 无阻塞；对 `sico test` 面向不可信包是硬阻塞。

**P1-B 未审查的合并损伤**（`runner/sico-runner/src/lib.rs:1948`）
两条 doc 注释被挤成一行：`/// True when this generation owns the streaming stdin channel.    /// Instantiates every prepared package…`，且其文档对象 `owns_streaming_stdin` 已全仓不存在。同类痕迹还有 `lib.rs:1760 pub fn prepare_program_inner_probe`（零调用者，`pub` 使其绕过 `dead_code`）。

**P1-C unsafe 围栏留口子 + 帧字节上限被绕过**
`crates/sico-automation-host/Cargo.toml:39` 整 crate `unsafe_code = "allow"`，而 `#[deny(unsafe_code)]` 只挂在 `policy`/`synthetic` 模块（`src/lib.rs:24,26`），crate 根与 `audit.rs` 无覆盖。`win32.rs:81/126` 的 `width * 4`、`stride * height` 无帧字节上限，绕过了抽象宿主的 `max_frame_bytes`（`lib.rs:358`）。目前仅 example 可达，故列 P1 不列 P0。

**P1-D 阶段名两套含义（文档与 STATUS 互相矛盾，已亲自复核）**
- `docs/STATUS.md:5`、`docs/ROADMAP.md:5`、`docs/plans/README.md:28`、`plans/M22:3` 均称 "S6 未进入"；而 `docs/steps/STEP-0193-m22-s6-sico-lowering-handoff.md:3` 写 "**S6 opened**"（0194/0195/0197 同），dev 提交自 09-14 起大量用 "M22 S6" 描述 lowering。`plans/M22:65` 定义 S6 = 自举闭环。**同一 token 指两件事，导致 M23/M24 的"S7 之后"入口 gate 当前不可判伪。**
- S5 口径冲突：`STEP-0241:26` 称 S5 codegen "unentered"，STATUS/ROADMAP 称 "S5 partial"；而 `STEP-0192` 文件名带 "s5-sico-parser"（S5 定义是 L2 codegen，`plans/M22:62`）。

**P1-E STEP 唯一性铁律破裂**
- `docs/steps/STEP-0175-language-v1-batch2.md` 与 `STEP-0175-language-v1-batch2-for-loops-error-propagation.md` **同编号两条记录**（已复核存在）。
- `STEP-0241:29-32` 自认 origin 上 11 条记录曾复用 STEP-0213–0223 并被删除，提交 `a4e2325/f59e6ba/feafeb2/d4d3168` 一串 "re-applied"、`9aa92e5` "rebuilt … as re-validated"、`d383ce4/da26671` "repaired origin dev"。这些 ID 现在指向与历史不同的代码。
- `acb5113`（HEAD）无 STEP 记录、未同步 STATUS/ROADMAP，提交信息为第一人称将来时口吻（说"将替换"，实际已部分落地）——违反 AGENTS.md §3。
- 多远端（`origin` gitcode + `github`）双流合并是上述纪律破坏的主要来源。
- 索引缺口：STEP-0178–0194（17 条）在 `docs/steps/README.md` 无任何引用。

**P1-F 用户手册整段与可执行面矛盾（最后修改 2026-09-13）**
`docs/user-guide/LIMITATIONS.md`：
- `:30` "`sico test` 不是现有 CLI 子命令" — **假**：`crates/sico-cli/src/lib.rs:80` 分发 `test`，`crates/sico-cli/src/test.rs:1-14` 是 STEP-0140 落地的真实发现+执行器（`deny_unknown_fields`、`#![forbid(unsafe_code)]`）。
- `:21` "source-level debugging/DAP 尚未实现" — 与 M10 GO（minimal DAP 清单驱动子集，`runner/sico-runner/src/dap.rs`）冲突。
- `:43` "当前真实模型调用次数为 0" — 与 STEP-0128 的 2880 次真实 DeepSeek 调用冲突。
`LANGUAGE-BASICS.md:38,53` 仍描述 M14 之前的 codegen refusal，缺 `while/for/set/break`、Map/Set、位运算、`checked_mul`。
Linux 侧双向风险：`LIMITATIONS.md:11` 称无原生 runner，`STATUS.md:5` 称 M12 有 "Linux x64 native runtime evidence"（实为 WSL2）——措辞必须统一。

**P1-G 合同先行被追溯满足**
`ADR-0015` 接受日 2026-09-17（文件头 `:3`，git 09-18 00:07），而 M22 自举实现始于 09-14（`STEP-0193/0194` completed 2026-09-14）；`plans/M22:3` 自认入口条件 5 于 09-17 才关闭 → **实现早于架构冻结 3 天**。`RFC-0038` 现标 accepted，但 `STATUS.md:26` 记 STEP-0129 时仍为 "draft 待 owner 接受"，STEP-0130 起已在实现。RFC-0044/0045/0046 的接受方式是 owner 口头指令批量接受整套草案（`RFC-0040:4`、`RFC-0041:4` "D1–D4 accepted as drafted"），且 RFC-0046 与 STEP-0193 相隔约 4 分钟进入同一提交串。

**P1-H 证据分级标注覆盖不足**
STEP-0140–0245 共 101 条记录，仅 35 条带 evidence 类；M15–M20 的平台/出口审计（STEP-0155/0156/0158/0159/0162/0166/0168/0170）**全部无标签**，恰是最需要分级的运行时主张。215 语料的分级标注本身正确（`STEP-0214:5`、`STEP-0245:6`、`reports/m22-interim-audit.md:3,6` = internal-fixture / NO-GO）。STEP-0128 的 live-model 限定语规范，但原始 run 与 prompt packets 只在 gitignored `target/`（`reports/ai-eval-live-model-v1.md:24,53-59`），且未按 `ai-eval/runs/README.md` 自订政策记录 Sico 仓库 commit → "0.9095" 不可独立复现。

**P1-I 治理门把"合同仍有效"编码成字面量，HEAD 上实测为红**
`tools/validate-step-0124.ps1:30` 要求 M14–M25 每份计划正文含子串 `no STEP numbers reserved`；`docs/plans/M22-compiler-self-host.md:3` 写的是 "No later STEP numbers are reserved."（多 `later`/`are`）→ 直接 `throw`。措辞漂移由提交 `1c6c04b` 引入，本机实测复现。这条门正是 AGENTS.md §8 给"文档级 roadmap 改动"指定的最小校验，于是 **dev HEAD 上做任何 M14–M25 规划改动都会撞假失败**。同一模式的另一端见 §4 P2（`validate-step-0245.ps1` 的 `(116, 34, 65, 0)` 字面量 grep 门可被死代码满足）：子串断言既能误红也能误绿，因为它检查"字符串还在"而非"性质还在"。修法：状态改成结构化 front-matter 字段，validator 读字段。

**P1-J runner 全量测试的 4 条失败里只有 1 条是产品缺陷，另 3 条是"按仓库自己写的命令跑就会红"**（§1.2 实测，MSVC 变体，`--no-fail-fast`）
1. `bootstrap_bundle::frozen_corpus_manifest_…`：**真实完整性缺陷**，见 §4 P0-5。
2–3. `runner_cli_observes_real_windows_console_control`（`tests/runner.rs:1325`）与 `runner_watch_observes_console_control_while_idle`（`:1380`）：`Command::new("python")` 从 **PATH** 取解释器。`run-ci.ps1:27` 用 `ensure-python.ps1` 解析出 pinned CPython 后只是把它的目录前置到 `PATH`（STEP-0196:45-48 自述），`SICO_PYTHON` 这个变量**在 Rust 侧零消费者**（全仓 grep 只有 `run-ci.ps1:27-28` 与 STEP-0196 文本）。因此在 CI 包装脚本之外跑 AGENTS.md §8 给的命令，本机 `python` 落到 Windows Store 存根 → panic 消息是 Store 的 "Python was not found…"，看起来像 runner 失败。
4. `channel_relay_keeps_rss_independent_of_stream_size`（`:2580`）：断言 `rss_after <= rss_before + 16 MiB`，而 RSS 是**整进程**指标，同一 test binary 里并行跑的兄弟测试在分配。实测：并行全量 **FAIL**（`154144768 -> 189358080`，+35.2 MiB），`--test-threads=1` 全量 **PASS**（35/37，只剩上面两条 python），单独跑 **3/3 PASS**。`run-ci.ps1:57` 正是靠 `--test-threads=1` 绕开它；AGENTS.md §8 的 runner 命令行**漏了这个参数**。
→ 结论：M22 的"CI 绿"依赖一个包装脚本里的隐式环境契约（PATH 前置 + 串行执行），而这个契约既没写进 `docs/development`，也没写进 AGENTS.md 的复算命令，`SICO_PYTHON` 形同虚设。修法：测试改为读 `SICO_PYTHON`（缺省时 `skip`，与 `SICO_TEST_WASMTIME` 同一模式），把 RSS 断言改成 scheduler 自身计数或移入 `--test-threads=1` 专属分组，并把 §8 的 runner 命令补上 `--test-threads=1`。

### P2 — 债务（不阻塞，但会在下一步咬人）

- `selfhost/compiler_semantics.sico`：`semantic_diagnostic` 单函数 507 行（`:243-749`），用 18 条平行 `List[Text]` 模拟结构体（`:245-262`），`map_get`/`parameter_type`/`owner_count` 全线性扫 → O(n²)。新增一条语义规则只能往 500 行状态机尾部追加关键词指纹。
- 指纹式规则（改名即失效）：`compiler_semantics.sico:582-590` 的 E7002/E7001 依赖字面出现 `return Model` + `text`→`loaded` + `revision`→`model`（即 `syntax-candidates/b/revision/invalid/unchecked-stale-result.sico:18` 原词）；`:541/:552` 依赖类型名字面量 `"Stream"`。变量改名后检查静默消失。
- 缩进被当作语义代理：`compiler_semantics.sico:271`、`:384` 用 `same(raw, line)`（trim 前后相等）区分声明/语句 → 0 缩进源码会跳过 E2xxx–E8xxx 全部规则。
- 词法字母表窄于 Rust：`compiler_lexer.sico:290-328` 缺 `<`、`>`、`!`、`{}`，而 `LEXICAL` 桶只是 `tools/update-m22-corpus.ps1:87-95` 对 Rust 输出做 `lexical error` 子串匹配得到的合成桶——116/215 的"词法闭合"信息量很低，同属 Error 不等于同因。
- 复制：`compiler_lexer.sico` 与 `tokens.sico` 近乎整份重复，`tokens.sico:431-435` 残留裸 `<` 判 Error 的缺陷分支；`compiler_parser.sico:208-317` 与 `declaration_parser.sico:245-354` 逐行重复；`:592/:595` 用 `set x = x` 代替缺失的 `continue`。
- 测试驱动空洞断言：`runner/sico-runner/tests/selfhost_checker.rs:193-199` 兜底分支只断言"未被拒绝"；`:142-145` 只比对排序后**第一个** diagnostic id；`:146` 读源文件却从不校验 `source_sha256`（对比 `tests/bootstrap_bundle.rs:175` 做了校验）；`:209-222`、`:244-255`、`:277-314` 是手抄 (path, code) 元组；`:107-113` 断言自造码 `E-SH-SYNTAX-FUNCTION-COLON`，Rust 真实码位是 E1017（`crates/sico-diagnostics/src/lib.rs:426`）→ 缺冒号场景 identity 与 Rust **不一致**，只是 215 语料中无此形状。
- 校验脚本的 grep 门是"断言存在"而非"断言性质"：`tools/validate-step-0245.ps1` 要求测试文本包含 `(116, 34, 65, 0)` 与 `assert_eq!(entries.len(), 215)` 字面量——这些串可由死代码满足。（值得肯定的是同文件 `:36-40` 主动禁止 `syntax-candidates/`、`// expect:`、`source_sha256` 出现在语义模块里，是明确的反查表护栏；且该 validator 之后确实跑真实 cargo 测试与 clippy。）
- `crates/sico-ir/src/lib.rs:1388-1406` `boundary_kebab` 注释声称注入性，但 `_`→`-` 与大写→`-x` 使 `Foo_Bar` 与 `FooBar` 同名；runner `link_packages`（`lib.rs:1884-1892`）用 `position(…).unwrap_or(0)` 先到先得，无重复身份拒绝 → 应改为 check 阶段显式拒绝。
- 缺失用例：超尺寸 guest stdout、runner 挂死、`read_dir` 失败子树、目录软链环、`win32::capture` 真实路径帧上限、`boundary_kebab` 冲突、`break`/`continue` 嵌套形态（e2e 语料各仅 2 处）、`lower.rs:831` `self.nested` 无深度上限的深嵌套栈溢出。
- `AGENTS.md:16-19` 治理文档自身失效：仍写 M12 GO-core、M14–M18 无 STEP（实际 241 条 STEP、M15–M21 已 GO），最后修改 2026-09-04。

## 5. 前进方案质量判定

### 5.1 M22 路线：按当前路径实质阻塞，需重划片

- **诚实度**是这条线最大的优点：`STEP-0220`、`STEP-0241`、`STEP-0245`、`reports/m22-interim-audit.md:3`（NO-GO）都在主动声明"不主张什么"，`STATUS.md:5` 写的是 "S3/S4/S5 partial，S6 尚未进入"。未发现把 partial 洗成 complete 的迹象。
- 但**推进形状不收敛**：STEP-0213 起 33 个 STEP 只换来 checker 冻结子集闭合，S6 零进展；每步只新增一个语法形状的字节模板（`STEP-0222…0240` 的命名即自证）。这不是"还差几步"，而是"每步的收益恒定、问题的规模不固定"。
- **"S4/S5 partial" 命名过宽**：S4 的出口判据（typed IR 与 Rust IR 同形）目前只在 4 个规范形状上成立；把它称作 partial 会让人以为接近完成。
- 判定：**S1、S2（声明子集）为真实完成；S4/S5 应重标为"形状模板原型"；S6 按现路径不可达**，除非先解 P0-2（`List[record]` 或 SOA 规范化 ADR）与 P0-3（预算曲线）。

### 5.2 计划文档本身的质量

`plans/M22` 的切片、exit gates、non-goals、risks 结构完整，且 §7 risks 诚实点名了 `List[record]`；§8 把自举中遇到的语言摩擦登记为 v1 batch 3 候选并**明令不得在 M22 内实现**——这条纪律是对的，但也意味着自举被自己冻住的表面卡住（无中缀 `<`/`&&`，每条比较需 `Result` match 仪式，代码量倍增）。这是**已登记的自缚**，不是疏忽。

### 5.3 M23–M25：真计划，但被用于维持进度叙事

- 三份计划有实质内容：M23 入口 gate 显式等 M22 S7 并逐项要求 4 部分证据；M24 `:23-56` 给出容差/digest/limit+1/错误注入/急停的可测退出；M25 `:27-34` 明令不得把缺失 owner 输入转成暗示成功；三条均**未预留 STEP 号**（`plans/README.md:29-31`）→ 2026-09-17 注册它们**没有**违反 AGENTS.md §3"不得为排期好看而预留 STEP"。
- 真正的越界在两处：`docs/ROADMAP.md:5` 把主线写成"…随后 M23→M24→M25"（在 S6 未进入时给出顺序叙事），以及 `ROADMAP.md:305` 把 NO-GO 的 M20 未完成范围整体转嫁给 M23/M25（把未交付义务挪进未来计划）。

## 6. 主审亲自复核过的指控

以下不依赖子代理结论，由主审直接读文件确认：`compiler_semantics.sico:271-273` 的 module→E8010 规则与 `modules.rs:337-350` 的入口限定语义之差；`corpus-v0.json` 含 0 个 `selfhost/` 源；`compiler.sico:475/521/578` 三处 `source_name` 写死 `"identity.sico"`；`List[record]` 排除条款（`plans/M22:56,113`、`STEP-0182:37`、`STEP-0183:18`）；`STEP-0241:30` 编号复用自认；STEP-0175 双文件；`STEP-0193:3` "S6 opened"；`runner` 默认 fuel `10_000_000`（`lib.rs:243`）vs 测试 `1e9–5e9`（`selfhost_compiler.rs:78`）；`test.rs:202/245/249`；`lib.rs:1948` 注释挤压与 `lib.rs:1760` 死 API；`Cargo.toml:39` unsafe allow 与 `lib.rs:24,26` deny 范围；`crates/sico-cli/src/lib.rs:80` 确有 `test` 子命令而 `LIMITATIONS.md:30` 否认；`tools/run-ci.ps1` 全 11 步与 1.98.0 钉住；`target/` 证据停在 7-21；`.gitignore:4` 排除 `target/evidence`。
本轮实测另由主审亲自复算并交叉验证：`corpus-v0.json` 215 条目的 bytes/sha 逐条比对（214 吻合、1 条 CRLF 漂移）；`validate-step-0124.ps1:30` 与 `M22-compiler-self-host.md:3` 逐字比对并用 `git log -S` 定位引入提交（§4 P1-I）；`SICO_PYTHON` 全仓消费者 grep（仅 `run-ci.ps1:27-28` + STEP-0196 文本，Rust 侧 0，§4 P1-J）；RSS 断言的三态复现（并行 FAIL / `--test-threads=1` PASS / 单测 3/3 PASS，§4 P1-J）。

## 7. 修复建议（按杠杆排序）

1. **立刻修复 §4 P0-5 的语料冻结**（<0.5 天）：`block-solver.sico` 规范化为 LF + 重跑 `tools/update-m22-corpus.ps1`，并把 `bootstrap_bundle` 恢复为绿。这是本次实测中 HEAD 唯一真实的失败项，也是任何 fresh clone 上 CI 的第一道红灯；顺带让 `selfhost_checker.rs` 校验 `source_sha256`。
2. **停新增形状 STEP**（0.5 天决策）：现有 STEP-0213→0245 的边际收益曲线已证明该路径不收敛。改为先立 `List[record]` / 表达式形状的 batch-3 RFC，或一份"SOA 规范化是正式架构而非临时技巧"的 ADR。
3. **给 S4 一个通用性出口判据**（1–2 天）：以"能编译 `formatter.sico`（827 行真实代码）"作 canary，替代逐形状差分。通不过就明说 S4 未进入。
4. **修 P0-1 并在语料里加入自举前端自身**（0.5 天）：E8010 作用域改判为"仅入口文件"，把 `selfhost/*.sico` 纳入 checker 自测；这一步会立刻暴露泛化半径问题，是成本最低的真实度提升。
5. **画预算曲线**（1 天）：对 1k/4k/8k 行输入测 guest 前端的 fuel 与栈用量，在 S6 之前把 P0-3 变成数字而不是悬念；注意 `ADR-0015 §8` 禁止事后抬限。
6. **补 HEAD 提交的过程记录**（0.2 天）：给 `acb5113` 立 STEP，并在 validator 中强制 `selfhost/` 变更必须引用存在的 STEP-ID。
7. **统一阶段词汇**（1 天）：重标 STEP-017x–019x 中错挂的 S5/S6，合并 STEP-0175 双记录，索引 STEP-0178–0194；把 `validate-step-0241.ps1` 扩为"每 ID 唯一 + 全索引 + 阶段名与 `plans/M22` 定义一致"。
8. **修 P1-A/P1-B/P1-C 三处安全与卫生缺陷**（1 天）：`test.rs` 加 `take(limit+1)`+看门狗+`read_dir` 失败计入 fail；修 `lib.rs:1948`、删 `prepare_program_inner_probe`；automation-host 改 `#![deny(unsafe_code)]` 且仅 `win32` 模块 allow，并把 `max_frame_bytes` 前移到 win32 分配之前。
9. **补 `tools/ensure-binutils.ps1` 并把 binutils 写进 `docs/development` 前置条件**（0.5 天）：消除 §1.3.1 那个未文档化的 GNU 前置；同时让 `run-ci.ps1:23` 在缺失时报错而非静默跳过。
10. **重写用户手册为支持矩阵的投影**（1–2 天）：先修 `LIMITATIONS.md:11/21/30/43` 与 `LANGUAGE-BASICS.md:38/53`，再加机器校验（子命令清单 + 矩阵 JSON + e2e 语料）禁止手册与可执行面矛盾。同步更新失效的 `AGENTS.md §2`。
11. **证据持久化**（1 天）：`target/evidence/m19`、ai-eval run/packet 及仓库 commit SHA 以受控形式落入 `docs/evidence/`；STEP-0144 起每条记录强制 `evidence class:`。
12. **拆掉隐式环境契约**（0.5 天，§4 P1-J）：console-control 测试改读 `SICO_PYTHON`（缺省时按 `SICO_TEST_WASMTIME` 的现成模式 skip），RSS 断言改用 scheduler 自身计数或单列串行分组，并把 `--test-threads=1` 补进 AGENTS.md §8 的 runner 命令行。
13. **把 §4 P1-I 的子串 gate 变成结构化字段**（<0.5 天）：`validate-step-0124.ps1:30` 的措辞漂移误红在 HEAD 实测可复现，先复绿，再把 M14–M25 计划的 STEP 预留状态改为 front-matter 字段读取。
14. 中期债务：抽 `runner/sico-runner/tests/common` 收拢 15 份复制 harness；把 `boundary_kebab` 冲突变成 check 阶段 E8xxx 拒绝；补 §4 P2 缺失用例。

## 8. 遗留阻塞清单（不构成任何成功主张）

- ~~本机无 pinned 工具链 → 9 月证据不可复算~~ **已解除**：1.98.0 GNU+MSVC 双装、wasmtime/python 前置件已就绪，根 workspace 与 runner 套件均已在 §1.2 实测复算。剩余环境障碍改为一条且只有一条：**官方钉住的 GNU 变体缺 `gcc.exe`/`dlltool.exe`**（无脚本、无文档，§1.3.1），故本次全部通过项来自 MSVC 变体，**不能替代 GNU 证据**。
- 本次未跑：`run-ci.ps1` 全 11 步、workspace/runner clippy、`cargo test --doc`。不得由 §1.2 的部分绿灯推出"CI GREEN"。
- Linux 原生证据实为 WSL2，与 `LIMITATIONS.md:11` 未统一；Android/Harmony 仍为外部后置轨。
- M7 公网部署仍 blocked-external-evidence；M6 mobile NO-GO。
- M13 记录了既定缺口：B-repair 0.9095 达标但 0.833 < 0.850 target（`ADR-0012`）。
- M20 整体 NO-GO（`STEP-0170 §13`），语言 v1 冻结未完成；M22 S6/S7 未达成 → M23/M24 入口 gate 目前不可判真。
