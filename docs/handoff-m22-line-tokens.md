# M22 format_code 嵌套 while 攻坚 — 交接手稿

> 状态：**STEP-0260 已提交并推送双远端（`c39b6d8`），line_tokens 区域 byte-exact，canary 钉在 `SKIP:GENERAL-WHILE-NESTED`**。下一前沿是 `format_code` 的嵌套 while。本文档交接给下一个 AI 继续。
> 上一轮手稿（停在 else 悬空 blocker 的状态）在 git 历史中，提交 `7a274dd` 可查看。

## 一句话状态

M22 仍 **NO-GO**（诚实）。S4 收敛推进中：formatter 前 16 个函数（52 个 block，至 `line_tokens` 为止）已自举 byte-exact；全量 `formatter.sico` canary 以 exit=122 + `SKIP:GENERAL-WHILE-NESTED` 停在 `format_code` 的第一个嵌套 while。基线提交 `c39b6d8`，工作树干净。

## 环境 / 快速迭代环

```bash
cd /d/SorftWare/sico
# 工具链（本机缓存是 msvc，gnu 缺 gcc/dlltool——见 STEP-0254 注记）
export RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-msvc'
CARGO="$USERPROFILE/.cargo/bin/cargo.exe"

# 快速环（不重编 Rust 测试；产物拒绝覆盖，先 rm）：
rm -f /tmp/m22iter/compiler.component.wasm
./target/debug/sico.exe build --profile script-v0 --output /tmp/m22iter/compiler.component.wasm selfhost/compiler.sico
cat /tmp/m22iter/<t>.sico | ./runner/sico-runner/target/debug/sico-runner.exe --fuel 2000000000 /tmp/m22iter/compiler.component.wasm > out.txt 2>err.txt
# 错误 exit=122 + stderr JSON；typed 拒绝必须字节稳定（validator 钉了）
```

- Grep/Read 工具走 Windows 路径访问不到 `/tmp`，用 bash grep 或 `cygpath -w` 转换；python 需 `C:\Users\SUN-OF~1\AppData\Local\Temp\m22iter\` 形式。
- 校验脚本 `tools/validate-step-0260.ps1` 整跑超过 300s 前台上限，分段跑（钉检查→build→各测试套件→自举+fixture 比对+canary）。
- 本机 PowerShell 5.1：`New-Item` 不支持 `-LiteralPath`，用 `-Path`。

## STEP-0260 已落地（勿回滚）

1. `gw_condition_packed` 通用调用条件：非 bytes.at 实参走 `while_call_rhs_packed(...)`，arity/i64 检查只守 bytes.at 分支。
2. `gw_slice_subject_packed` 逗号扫描修复：`next_index` 连续扫，支持 3 实参 slice 主语。
3. match 绑定注册为 `sico.text.concat("#match", <name>)`（oracle 要求 `#matchword_bytes` 式）；`local_binding_index` 回退查 `#match`+name；locals 类型判定走 `is_word(cell_kinds, "result")`。
4. List 类型面：`declared_return_kind`/`binding_parameter_kind`/`parameter_kind_by_id` 返回完整 JSON `{"kind":"list","data":{"kind":"<elem>"}}`（elem_close 需 next×2，三处 off-by-one 已修）；`binding_parameter_index` 对 List 类型跨 4 词步进；新增 `type_json()` 辅助（kind 以 `{` 开头则逐字嵌入）；read_local/call/intrinsic 三个发射器 + gw 函数头 return_type/locals 全走 type_json；split_lines 分支 gws_kind 硬编码完整 list JSON。
5. `gw_rhs_packed` 固定运算：`I64/U64.bit_and/bit_or/bit_xor/shl/shr` 的 set RHS（cell/param/literal 操作数，移植旧 swl 路径）。
6. break/continue：删 general_while 入口 SKIP 守卫；合并 `gbc_*` locals；continue 立即 seal→ghdr_id，break 记入 `gbrk_from` 在 end while 拿到 gafter_id 后 seal；区域守卫拒 entry/after/terminator 后。
7. locals 上限：MAX_LOCALS_PER_FUNCTION=256；25 个一次性 `let <x>_ok` 换成共享 `let gok`（**set 必须在 let 之后**）。
8. 诊断后缀全清：729 处 `-L<n>` + 5 处 STATEMENT 实验后缀剥净。

## 下一前沿：format_code 嵌套 while

- canary 事实：全量 `formatter.sico` stdin → exit=122，stderr `SKIP:GENERAL-WHILE-NESTED`（format_code 的嵌套 while），无 trap。
- **设计方向**：现在 while 头/尾块 id 是全局单槽（`ghdr_id`/`gafter_id`），嵌套会互相覆盖。需要改成**栈**（List[U64] 头尾配对压栈/弹栈，或 packed frame 列表）。这是一轮中较大的结构性改动。
- **locals 预算红线**：`general_while_function_ir` 已贴近 256 上限，任何新 let 必须先回收 dummy/合并现有 cell。
- **oracle 要重新生成**：`tools/fixtures/step-0260/lt_expected.json` 是 445 行前缀的 dump（source_len 10873，与完整 827 行版不同）。做 format_code 时用 `runner/sico-runner/tests/selfhost_compiler.rs` 里的 `dump_lt_region_ir`（`-- --nocapture` 跑）打印 Rust 侧 IR，仿照 line_tokens 测试的做法新起一个 STEP（下一个号预计 STEP-0261）+ `tools/fixtures/step-0261/` + `tools/validate-step-0261.ps1`。
- 嵌套 while 之后还剩 11 个函数：`source_has_lex_error`、`no_space_before/after`、`call_left`、`repeat_indent`、`Map[Text,U64]` 区域（`nearest_match`/`set_nearest_match`/`match_arm_levels`）、`close_code`/`direct_close`/`opener_close`、`normalize_source`、`main`。注意 **`Map[Text,U64]` 参数面未扩**：`scalar_type_kind` 不认 Map，参数类型面要专门补。

## 验证矩阵（STEP-0260 基线全绿，重跑命令）

```bash
export RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-msvc'
"$CARGO" test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1   # 17/17
"$CARGO" test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_parser   -- --test-threads=1   # 2/2
"$CARGO" test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml --test selfhost_local_bounds -- --test-threads=1 # 1/1
"$CARGO" test --locked --offline --workspace --all-targets --all-features  #  broader suite（control_flow/bit_ops/list_*/map_set/...）
```

## 纪律与已定决策（不要重开）

- refusal-first、typed 拒绝、拒绝码字节稳定；scan_space 回归必须 exit=0。
- 不要复活 `gjoin_else_ok` 标志方案（已回滚）；region 直接赋 then/else/join。
- Sico 对 elif 风格 else 链配对有限制（历史 blocker）：多分支 intrinsic 识别参考 5536 行 bytes.at 分支的 if/else/end if 布局。
- 迭代中间文件全在 `/tmp/m22iter/`。
- STEP 文档照 `docs/steps/STEP-0260-*.md` / STEP-0256 格式；提交信息风格 `feat(selfhost): STEP-0261 ...`；推送双远端（github 偶断，重试即可）。

## 缺口

- S5（Core-Wasm seam 拓宽）、S6（A=B=C 自举闭环）未进；M22 S7 退出审计未进。
- M23/M24 完全未动。
- M14–M18 计划边界照 AGENTS.md：M14 实现须等 M12 full GO 与 M13 结论，不得提前。
