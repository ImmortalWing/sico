# M22 formatter 尾部攻坚 — 交接手稿

> 状态：**STEP-0261 已完成（嵌套 while 栈帧，formatter 前十七函数 byte-exact，canary 钉在 `call_left` 的 `ERR:E-SH-IR-EXPRESSION`）**。下一前沿有两个：`call_left`（guard-chain 机扩展）与 `format_code`（gw 机 body-if 用户调用条件）。本文档交接给下一个 AI 继续。
> 更早的手稿在 git 历史中（`7a274dd` 停在 else 悬空、STEP-0260 基线见 `a43378e` 的上一版）。

## 一句话状态

M22 仍 **NO-GO**（诚实）。S4 收敛推进中：formatter 前 17 个函数（至 `source_has_lex_error`——首个嵌套 while 函数）已自举 byte-exact；全量 `formatter.sico` canary 以 exit=122 + `ERR:E-SH-IR-EXPRESSION` 停在 `call_left`。

## 环境 / 快速迭代环（**2026-09-23 换机，重要**）

```bash
cd /e/github/sico
# 本机（新证据机）没有 MSVC/VS；GNU 工具链 + 仓库自带 msys2 binutils（gcc/dlltool）
export RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-gnu'
export PATH="/e/github/sico/target/tooling/msys2-binutils/mingw64/bin:$USERPROFILE/.cargo/bin:$PATH"
CARGO="$USERPROFILE/.cargo/bin/cargo.exe"
# runner 测试还需：
export SICO_TEST_WASMTIME="$(powershell -ExecutionPolicy Bypass -File tools/ensure-wasmtime.ps1 | tail -1)"

# python 用仓库自带的（系统 python 是 WindowsApps 存根）：
# /e/github/sico/target/tooling/python-3.12/python.exe
# Git Bash 的 /tmp 与 Windows python 路径不互通——python 侧用 cygpath -w 转换

# 快速环（不重编 Rust 测试；产物拒绝覆盖，先 rm）：
rm -f /tmp/m22iter/compiler.component.wasm
./target/debug/sico.exe build --profile script-v0 --output /tmp/m22iter/compiler.component.wasm selfhost/compiler.sico
cat /tmp/m22iter/<t>.sico | ./runner/sico-runner/target/debug/sico-runner.exe --fuel 5000000000 /tmp/m22iter/compiler.component.wasm
```

- 校验脚本 `tools/validate-step-0261.ps1` 整跑 ~5 分钟（本机 GNU 流可前台跑完；0260 版的 300s 限制是旧机 msvc 重建）。
- 本机 PowerShell 5.1：`New-Item` 不支持 `-LiteralPath`，用 `-Path`。
- Rust oracle dump 探针：`selfhost_compiler.rs` 的 `dump_shle_region_ir`（`-- --nocapture`），改边界字符串即可为下一个区域出 oracle。

## STEP-0261 已落地（勿回滚）

1. **while 帧栈**：`gwh_frames`（打包 `hdr\nbody\ncond\nstart\nend\nbrk_base`）+ `gwh_len` 弹帧计数；`ghdr_id`/`gwhile_start`/`gwhile_end` 是栈顶镜像（close 时恢复），`continue` 封到栈顶 header。while-open 合法区域 = entry/body/after；**if 臂内开 while = `SKIP:GENERAL-WHILE-IF-REGION`**（opener_close/normalize_source 在其后）。
2. `end while` 弹帧：after 块在 emission 前沿分配（与 Rust `sort_by_key((never_current, became_current, creation))` 一致——盖章块按流序、空块沉底）；break 按 `brk_base` 作用域只 seal 本帧的；branch 终结子登记进 `gwt_entries`（hdr/cond/body/after），终结符组装扫描该表（优先级位置在最前）。
3. `sico.list.length` 参数/cell 主语经 `gw_intrinsic_call_packed`（fixed_op 空名挡板处直通）；`sico.text.split_lines` 非字面量实参同样直通（ret kind = list-of-string JSON）。
4. **嵌套用户调用实参**：`while_call_rhs_packed` 实参段若是声明函数调用，递归打包（内层 span 精确），内层 call 用 `function_declaration_ordinal`；call 尾随检查接受行尾/`)`/`,`。
5. locals：`general_while_function_ir` **恰 240**（cap：busiest+16≤256）。回收手段：删 `gbody_id`、`gwf` 双用（帧字段+终结子字段）、`gbsi` 复用为终结子扫描下标。**再加 cell 前先看 `selfhost_local_bounds --nocapture` 的 top-5 输出**（本 STEP 加的常驻 println）。
6. fixture：`tools/fixtures/step-0261/shle_expected.json`（117,109 字节，17 函数前缀）；校验器 `tools/validate-step-0261.ps1`（GNU 流 + call_left 前沿探针 + canary）。

## 下一前沿（二选一入口，建议先 format_code 走 gw 机）

### A. format_code（gw 机，推荐先做）
- canary 探针事实：`no_space_before(item(tokens, cursor))` 作 body-if 条件 → `ERR:E-SH-IR-CALL-ARGUMENT`。gw 的 if 条件路径（`gw_condition_packed` 系）不支持用户调用（while 条件已支持，STEP-0260）。
- 需要形状（见 oracle dump，`format_code` 16 块）：body 内 `let kind = item(tokens, cursor)`（✓ 已支持）、4 层嵌套 if/else（if 帧机制已有多级？需验证——line_tokens 只有链式）、`set output = sico.text.concat(output, raw)`（✓）、`set cursor = add(cursor, U64.literal(2))`（✓）、B15 after 读 output return（✓ 形）。
- `repeat_indent` 无新形状（应随 format_code 一起绿）。

### B. call_left（guard-chain 机扩展，量级更大）
- 事实：`SKIP:ARITY`（2 参；机器只收 1 参）+ set 臂（`set left = true`，机器只收 return 字面量臂）+ 裸 cell 条件（`if left:`）+ then 臂内嵌套 guard 链 + bool 字面量尾（`return false`——机器尾只收 string 字面量或 call）。
- oracle 布局：22 块；无 else 的 if（空 else 块沉底 jmp join）、`if left:` read_local 分支、链内 5 个 same-guard + `return same(previous,"RightBracket")` 尾、最终 `return false`。
- 机器模板硬编码块数 = guard_count*2+jumps，扩展要先重读块布局公式。

### 嵌套 while 之后的全景（执行队列）
`call_left`、`format_code`、`repeat_indent` → `Map[Text,U64]` 参数面（nearest_match/set_nearest_match/match_arm_levels；`scalar_type_kind` 不认 Map，参数类型面专扩）→ `close_code`/`direct_close`/`opener_close`/`normalize_source`（其中 opener_close=while-in-if、normalize_source=嵌套+deep while，触及 `SKIP:GENERAL-WHILE-IF-REGION` 边界）→ `main`（无 while，纯调用编排）。

## 验证矩阵（STEP-0261 基线全绿，重跑命令）

```bash
export RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-gnu'
export PATH="/e/github/sico/target/tooling/msys2-binutils/mingw64/bin:$USERPROFILE/.cargo/bin:$PATH"
export SICO_TEST_WASMTIME="$(powershell -ExecutionPolicy Bypass -File tools/ensure-wasmtime.ps1 | tail -1)"
"$CARGO" test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1   # 19/19
"$CARGO" test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_parser   -- --test-threads=1   # 2/2（99 例）
"$CARGO" test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_local_bounds -- --test-threads=1 --nocapture  # 1/1 + top-5
powershell -ExecutionPolicy Bypass -File tools/validate-step-0261.ps1   # 整链
```

## 纪律与已定决策（不要重开）

- refusal-first、typed 拒绝、拒绝码字节稳定；每区域 = 新 STEP 号 + `tools/fixtures/step-0NNN/` + `tools/validate-step-0NNN.ps1`（下一个号 STEP-0262）。
- oracle 生成：改 `dump_shle_region_ir` 的边界字符串（或新加 dump 测试）跑 `-- --nocapture`，python 解析块布局（`/tmp/m22iter/oracle_nm_raw.txt` 有现成的到 nearest_match 的 dump）。
- Sico 对 elif 风格 else 链配对有限制（历史 blocker）：多分支 intrinsic 识别参考 5536 行 bytes.at 分支的 if/else/end if 布局。
- 迭代中间文件全在 `/tmp/m22iter/`。提交信息风格 `feat(selfhost): STEP-0262 ...`；推送双远端（github 偶断，重试即可）。
- 格式：owner 指令（2026-09-23）「完成M22-M23」+「简单任务的子代理使用GLM5.3flash」——简单子任务走 CreateWorkflow `subagent_model=account:bigmodel-individual-coding-plan/GLM-5.3-Flash`，设计/降级硬活留主模型。

## 缺口

- S5（Core-Wasm seam 拓宽）、S6（A=B=C 自举闭环）未进；M22 S7 退出审计未进。
- M23 实现 gated on M22 S7；但 M23 的 kickoff 库存盘点 STEP（§7：四个探针的 ceremony 测量）可随时并行做（纯测量，不改语言面）。
- M24/M25 未动。M14–M18 计划边界照 AGENTS.md。
