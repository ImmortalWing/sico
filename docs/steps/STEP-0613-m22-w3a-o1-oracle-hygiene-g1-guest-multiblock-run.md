# STEP-0613 — M22 W3-a：oracle multi-block-`run` 卫生（O1）+ guest 最小 multi-block-`run` 发射（G1）

- Date: 2026-11-09
- Track: M22 compiler self-host, goal **W3-a**（`AGENTS.md` §10.7 / `docs/plans/M22-compiler-self-host.md`）
- Slice type: **实现片（CRITICAL PATH）** — `crates/sico-codegen-wasm/src/lib.rs`（oracle 卫生 1 行）、`selfhost/codegen.sico`（本波唯一热文件写入方：本片）、`runner/sico-runner/tests/selfhost_compiler.rs`（钉测试）。
- Worktree: `D:/SorftWare/sico-m22-w3a`，分支 `codex/m22-w2-step0613`，HEAD at start `e93f59adcce30ba4bcab88c90b991830a1d466ef`（`e93f59a`）。
- **未 commit / 未 push**（owner-only 授权）。**未翻任何门，M22 overall 仍 NO-GO。**

> **重要前置声明（诚实收窄）**：本片**无法运行 `cargo` 验证**——宿主机**未安装 Rust 工具链**（`cargo.exe`/`rustc.exe`/`rustup` 在 C:/D:/E:/F: 全盘搜索不到，无 `~/.cargo`/`~/.rustup`，无 crate registry 缓存，无 WSL）。主仓 `D:/SorftWare/sico/target/debug` 有 2026-11-04..07 的构建产物，说明工具链此前存在、现已移除。**本片全部代码/测试改动均未执行红→绿、未跑 bootstrap 守卫、未跑 fmt/clippy**。改动基于对 oracle 地面真值（冻结源码 + 既有逐字节钉）的静态推理，字节级正确性有强论证（§4），但**自举形状前沿（bootstrap statement frontier）风险未实测**（§7）。**任何「完成」断言一律不成立；本片交付 = 未验证的候选实现 + 证据链 + 阻塞报告。**

## 1. Goal

W3-a 的真挡板 = **guest 无法发射 multi-block `run` 函数**（STEP-0612 订正：此缺口为 **guest-only**、在既有契约内、**不需要 RFC/ADR**；Rust oracle 早已发射整份 `formatter.sico` 的 branched `run`，`oracle_bytes=28459`）。本片做两步：

- **O1（oracle 卫生）**：`helper_dependencies` 补 `is_utf8 → utf8_decode` 依赖（1 行），消除 STEP-0610 fixture 的「stdlib helper layout」伪信号（STEP-0612 §2）。
- **G1（guest 最小 multi-block `run`）**：lift 4 个具名 guest 挡点（STEP-0612 §4），使**最小 `if` + 两臂 `return` 的 `run`**（字节锚 = `IS_UTF8_GATE_FIXTURE_SOURCE`）逐字节等同 oracle。完整 formatter（28459 B）是 **G2，非本片**。

## 2. O1 — oracle 卫生（`crates/sico-codegen-wasm/src/lib.rs`）

- **真值（读自 oracle）**：`sico.bytes.is_utf8` 是**内联**发射器（`lib.rs:4499-4508`），其体调用 `helper("sico.bytes.utf8_decode")`（`lib.rs:4506`）。`helper_dependencies`（`lib.rs:510-556`）原先**没有** `is_utf8` 条目，默认臂返回 `&[]` ⇒ 只用 `is_utf8` 而从不直接调 `utf8_decode` 的模块缺 `utf8_decode` helper 注册 ⇒ 后端报伪信号 `Unsupported{function:"run",feature:"stdlib helper layout"}`。STEP-0610 手写 fixture `IS_UTF8_GATE_FIXTURE_SOURCE` 恰如此；真实 `formatter.sico:819` 直接调 `utf8_decode` ⇒ oracle 从不触发。
- **修复（+6/−0）**：在 `helper_dependencies` match 中、`"sico.bytes.equal"` 之后加：
  ```rust
  "sico.bytes.is_utf8" => &["sico.bytes.utf8_decode"],
  ```
  与既有 helper 声明（`utf8_decode`/`bytes_equal` 等）同构。
- **影响**：`is_utf8` 臂的 helper 注册现在正确；对**不**用 `is_utf8` 的模块零影响（`helper_dependencies` 只被 `is_utf8` 命中）。oracle 侧**无其他改动**。

## 3. G1 — guest 最小 multi-block `run`（`selfhost/codegen.sico`）

### 3.1 挡点与 lift

| # | 挡点（STEP-0612 定位） | lift |
|---|---|---|
| ① | `cgs_result_return_hex` 多块硬拒（`if less_than(0, array_item(blocks,1)): return ERR`） | 函数改为**逐返回值**签名 `(bytes, run, value, vindex)`，不再查 blocks ⇒ 多块拒收自然消失 |
| ② | `cgs_run_return_value` 只读块 0 返回值 | 函数**删除**（唯一调用方是旧 `cgs_result_return_hex`）；返回值改由每个 `return` 终止子的 `data` 直接传入 |
| ③ | `cgs_block_terminator_hex` 对 `run` 终止子一律拒 | 移除 blanket 拒；`run` 的 `return` 终止子分派到 `cgs_result_return_hex`（Script result-area），其余终止子（branch/jump/match/unreachable）走通用路径 |
| ④ | `cgs_run_instr_hex` 以 `terms=0` 调 `cgs_function_instr_hex`（无分支终止子） | 改为 `terms=1` ⇒ 每块发 `<终止子>` + `0b`（关 `if`） |

### 3.2 落点（`selfhost/codegen.sico` 净 −31 行：+20/−51）

- `cgs_run_instr_hex`（原 :4121）：`U64.literal(0)` → `U64.literal(1)`（terms 开关）。
- `cgs_run_return_value`（原 :4388-4417）：**整体删除**（30 行）。
- `cgs_result_return_hex`（原 :4786-4890）：签名 `(bytes, run)` → `(bytes, run, crrh_value: Text, vindex: List[U64])`；删除 `cgs_value_ids_refusal` 首检、`cgs_value_index` 计算、`crrh_blocks` 检查与多块拒；ok/error 臂**逐字节不变**。
- `cgs_run_body_hex`（原 :4894-4908）：删除 `crbh_run` 检查与 `cgs_result_return_hex` 尾调；epilogue `0b000b0b000b` → `000b0b000b`。
- `cgs_block_terminator_hex`（原 :6623-6632）：删除 `if run: return ERR`；`return` 臂内加 `if run: return cgs_result_return_hex(...)`。

**净函数数**：−1（`cgs_run_return_value` 删除）。**改造函数**：4（`cgs_run_instr_hex`、`cgs_result_return_hex`、`cgs_run_body_hex`、`cgs_block_terminator_hex`）。规模在 STEP-0612 预估（150–300 行）之下，因采取**复用既有 `cgs_branch_hex`/`cgs_flat_return_hex` 分派 + 最小参数化**而非新增发射器。

### 3.3 关键设计决策

- **委托而非内联**：`cgs_block_terminator_hex` 只加一个 `if run` 分派（调既有 `cgs_result_return_hex`），未把 result-area 逻辑内联进终止子函数。这符合 STEP-0610 的教训（委托是承重的：inline 会推过自举语句前沿）且保持函数小。
- **单块字节恒等**：run 从「尾部一次 result-area + `0b000b0b000b`」改为「每块终止子 + `000b0b0b000b`」，单块时字节**完全一致**（证明见 §4.1），故既有单块 `run` 发射钉（STEP-0504/0509/0530 等）不受影响。
- **result-area 逐 return 点**：`emit_script_return`（oracle `lib.rs:3373-3463`）本就是**每个 `Return` 终止子各发一份**、共用同一对 scratch 局部（每时刻只走一条路径，无冲突）。guest 现同样逐 return 点发；每点复用同一 `cgs_dispatcher_slot+1/+2`（area/copy）scratch 槽号，语义同 oracle。

## 4. 字节级正确性论证（静态，未实测）

### 4.1 单块 `run` 字节恒等（无回归）

旧单块 run 布局：`41<entry> 21<disp> 02400340` + `20<disp> 4100 4604` + `<instr0>` + `<resultseq 尾 0f>` + `0b`（关 if，来自 epilogue 首字节）` 00 0b 0b 00 0b`。
新单块 run 布局：`41<entry> 21<disp> 02400340` + `20<disp> 4100 4604` + `<instr0>` + `<term0 = resultseq 尾 0f>` + `0b`（关 if，来自 `cgs_function_instr_hex` terms=1）+ `00 0b 0b 00 0b`（epilogue `000b0b000b`）。
⇒ **逐字节相同**。oracle 单块 run（`compile_function` 的通用路径）即此布局 ⇒ 旧 guest == oracle 保持不变。

### 4.2 多块 `run` 与 oracle 同构

oracle `compile_function`（`lib.rs:1691-1748`）对**所有**函数统一：`block; loop;` 每块 `local.get disp; i32.const id; i32.eq; if; <instr>; <terminator>; end`，尾 `unreachable; end; end; unreachable; end`。`compile_terminator`（`:2263-2323`）已支持 `Return`/`Jump`/`Branch`/`Match`/`Unreachable`。
guest 新布局（terms=1）与之一一对应：
- 分支终止子 → `cgs_branch_hex`（已逐字节等同 oracle，STEP-0526 钉）；
- run `return` → `cgs_result_return_hex`（ok/error 臂逐字节等同 oracle `emit_script_return`，STEP-0503/0509 钉）；
- 尾 epilogue `000b0b000b` == oracle 尾。
⇒ 最小形状（块 0 分支、块 1 `return ok`、块 2 `return error`）预期逐字节等同 oracle。

### 4.3 覆盖的指令面

`IS_UTF8_GATE_FIXTURE_SOURCE` 的块 0 含 `sico.bytes.is_utf8`（STEP-0610 臂，`cgs_is_utf8_hex`），块 1 `ok(ScriptOutput(...))`（3 字段 construct），块 2 `error(ScriptError(code:..., message:...))`（enum 字段 + Text 字段）。ok/error 臂与 `cgs_result_fields_hex`/`cgs_result_error_fields_hex`/`cgs_result_arm_slots` 均既有（单块已测），本片不改其体。

## 5. 测试（`runner/sico-runner/tests/selfhost_compiler.rs`）

- **改钉**：`sico_is_utf8_lowers_byte_exactly_but_the_gated_run_emission_is_refused`（STEP-0610，双侧拒收定位）→ **`sico_is_utf8_gated_run_lowers_byte_exactly`**：断言 oracle 发射成功（O1）+ guest `--emit-component-hex` 逐字节等同 + 真实 `wasmparser::Validator`。
- **新钉（red-first 契约）**：`sico_run_emits_the_minimal_multiblock_run_component_byte_exactly`：断言最小多块 run 组件逐字节等同 oracle + Validator + **canonical IR 全等**（前端面，`rust_ir` vs `run_guest`）+ **typed-refusal 对照**（非 `Result` 的 `run` 两侧仍拒，fail-closed 未放宽）。
- **自举前沿钉**（改动波及，未跑）：`sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`（含 `cgs_result_return_hex`/`cgs_run_instr_hex`）、`sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly`（含 `cgs_block_terminator_hex`/`cgs_run_body_hex`）、`sico_compiler_lowers_the_step0513_codegen_function_byte_exactly`。
- **既有单块 run 钉**（应保持绿）：`sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly`（STEP-0504，ECHO 单块）、`sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly`（STEP-0509）。

## 6. Validation

### 6.1 已执行（本机，均可复现）

```bash
# comment discipline（Sico 注释纪律：函数体内禁止 `//` 注释）
python tools/validate-selfhost-comment-discipline.py selfhost/codegen.sico
# => SELFHOST_COMMENT_DISCIPLINE_GREEN files=2   （初版曾 RED：函数体内注释，已移除）
python tools/validate-selfhost-comment-discipline.py selfhost/parser.sico
# => SELFHOST_COMMENT_DISCIPLINE_GREEN files=2
git diff --check
# => 干净（exit 0）
```

**comment discipline 抓到并修掉 1 条自造缺陷**：初版在 `cgs_block_terminator_hex` 函数体内加了 `//` 注释 ⇒ 校验器判 RED（guest 语句扫描器把函数体内注释读作源码）⇒ 移除（改到函数体外 doc comment）。

### 6.2 未执行（硬阻塞，见 §7）

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"   # 不存在
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --exact sico_is_utf8_gated_run_lowers_byte_exactly sico_run_emits_the_minimal_multiblock_run_component_byte_exactly --test-threads=1
& $cargo test ... -- --exact sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly --test-threads=1
& $cargo test ... -- --exact sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly --test-threads=1
& $cargo test ... -- --exact sico_compiler_lowers_the_step0513_codegen_function_byte_exactly --test-threads=1
& $cargo fmt --all --manifest-path .\runner\sico-runner\Cargo.toml -- --check
& $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- -D warnings
```

**红→绿契约未能行使**：新钉 `sico_run_emits_the_minimal_multiblock_run_component_byte_exactly` 写于实现前（预期 oracle `.unwrap()` 先红，O1 后 oracle 绿/guest 红，G1 后双绿），但因无工具链**一次都未运行**。

## 7. 诚实收窄（本片最重要的诚实声明）

1. **宿主机无 Rust 工具链（硬阻塞）**：全盘（C:/D:/E:/F:）无 `cargo.exe`/`rustc.exe`/`rustup`，无 `~/.cargo`/`~/.rustup`，无 crate registry，无 WSL。主仓 `target/debug` 产物日期 2026-11-04..07（工具链此前存在，现已移除）。⇒ **所有 `cargo` 验证（钉测试、bootstrap 守卫、fmt、clippy）均未执行**。本片改动**未验证**，不得视为完成。
2. **自举形状前沿风险未实测**：`cgs_block_terminator_hex` 新增嵌套 `if`（`if return: if run: return cgs_result_return_hex(...)`），`cgs_result_return_hex` 改签名。结构上与既有模式同构（`cgs_flat_return_hex` 已是「user call in return」、`vindex: List[U64]` 实参已用于 `cgs_branch_hex`），**但未跑 `step0507`/`step0526`/`step0513` 守卫** ⇒ 前沿回归**未排除**。若守卫红，最可能是 `cgs_block_terminator_hex` 的嵌套分派形状 —— 下一片优先跑该守卫。
3. **单块 `run` 无回归**：仅静态证明（§4.1），未实测 `sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly` 等既有单块钉。
4. **词表/账本绑定成本未处理**：`selfhost/emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` 随 `codegen.sico` 变更而过期（`census-m22-emitter-vocabulary.py --check` 将判 `EMITTER_VOCABULARY_STALE`）。**本片按纪律未刷新**（冻结 census 工件，除非 slice 明确授权）；STEP-0612 §6 已登记该绑定成本。词表**内容**未变（未新增/删除发射 helper/intrinsic）。`match-component-coverage-v1.json` 只键控 `parser.sico` 站点且本片未改 `parser.sico` ⇒ 账本行号不受影响；组件 sha 仅对 `run` 发射相关站点可能变，需账本复跑确认（未跑）。
5. **W3-a 未收口**：本片只到 G1（最小多块 run）。**G2**（formatter 实形 2 分支 + 3 return、约 5-6 块、逐字节 == oracle 28459 B）与 **G3**（W3-a 收口：整份 formatter 全等 + `wasmparser::Validator` + 词表/账本刷新）**未做**。
6. **未翻任何门**：M22 overall 仍 **NO-GO**；门 1-7 状态不变。本片未 commit/push。

## 8. 下一刀（具名、未动工）

1. **（优先）修复工具链 / 重跑验证**：装回 `1.98.0-x86_64-pc-windows-gnu`（`rustup toolchain install 1.98.0 --profile minimal --component clippy,rustfmt`）+ `cargo fetch`，再跑 §6.2 全部命令。**在验证通过前，本片改动不得合并**。
2. **G2**（guest 嵌套 if / 多分支多 return，formatter 实形）：以 `--emit-ir-plan` 先复核 formatter `run` 精确块数（5 vs 6），再扩 `cgs_result_return_hex`/终止子分派到嵌套 `if`（oracle `compile_terminator` 已支持，无新增 orace 面）。
3. **G3**（W3-a 收口）：整份 `formatter.sico` renamed `main`→`run` 逐字节 == oracle 28459 B + `wasmparser::Validator`；`emitter-vocabulary-v1.json` `--update` 刷新 + 账本 digest 复核；`script-component-emission-v0.md` §6 补「本序列在每个 return 终止子处各发一次」。

## 9. Files changed

| 文件 | 变更 | 说明 |
|---|---|---|
| `crates/sico-codegen-wasm/src/lib.rs` | +6/−0 | O1：`helper_dependencies` 加 `is_utf8 → utf8_decode` |
| `selfhost/codegen.sico` | +20/−51（净 −31，6844 行） | G1：4 挡点 lift + `cgs_run_return_value` 删除 |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 改钉 1 条 + 新钉 1 条 | `sico_is_utf8_gated_run_lowers_byte_exactly`（flip）+ `sico_run_emits_the_minimal_multiblock_run_component_byte_exactly` |
| `docs/steps/STEP-0613-m22-w3a-o1-oracle-hygiene-g1-guest-multiblock-run.md` | 新建 | 本文档 |

**未改**：`docs/STATUS.md`、`docs/handoff-m22.md`、`docs/steps/README.md`（coordinator 中心更新）、`selfhost/parser.sico`（热文件，本片非写入方）、`selfhost/emitter-vocabulary-v1.json`（冻结 census，绑定成本待后续片）。

## 10. 证据标签 / 诚实口径

- 本片改动为 **`internal-fixture`** 级证据（合成 `IS_UTF8_GATE_FIXTURE_SOURCE` 形状），**非** `clean-room-consumer`/`external-pilot`。
- 所有字节目标以 oracle 冻结源码/既有逐字节钉为地面真值；**未手写任何 hex**。
- 未上调 fuel/内存/超时限额（owner D-B 否决维持）；未改 M22 七门、ADR-0015 `A==B==C`、证据标签、冻结 census/digest。
