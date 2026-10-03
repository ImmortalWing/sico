# M22 交接手稿（living handoff，STEP-0294 本机基线）

> - 更新：2026-10-03（STEP-0338）；工作目录 `E:\github\sico-w2`（git worktree），分支 `codex/m22-w2`（0ef908e 历史与原权威线 `codex/m22-w1-ci` 相同；w1-ci 本地分支已清理、远端仍在；远端快照推 gitcode `origin`）。
> - `origin/dev` 已前进至 `550059a`，包含另一套 STEP-0279–0283 W1/W2 历史，与本分支源码及 STEP 编号冲突。不得覆盖或隐式合并。STEP-0330–0337 为 M24 并行轨（worktree `sico-m24`，提交 `095227b`）占用；M22 线自 STEP-0338 起编。
> - 权威裁决：[STATUS](./STATUS.md)、[M22 计划](./plans/M22-compiler-self-host.md)、[STEP-0292](./steps/STEP-0292-m22-w1-independent-ci.md)、[STEP-0293](./steps/STEP-0293-m22-w2-u64-to-text-while-rhs.md) 与 [STEP-0294](./steps/STEP-0294-m22-w2-map-get-match-subject.md)。本手稿只提供恢复路径。

## 0. 当前状态

**STEP-0340（2026-10-03）**：`gw_starts_with_condition_packed` 落地——gw 机器 if 条件接受 `sico.text.starts_with(<expr>, "<literal>")`（parser.sico 最普遍条件形状，185 处，此前拒 SCICOND）。接收者 param（零指令）/cell（read_local）/用户调用（gw_rhs_packed 复用）+ 字面量参（scalar_const const_string）+ bool intrinsic；**id 记账：intrinsic result = const_id + 1**（total = recv_count + 2）。5 函数翻绿（is_punct_byte/type_json/declared_return_kind/binding_parameter_kind/parameter_kind_by_id）→ parser **34/120**，R3 0，套件 58/58。附带钉死：嵌套用户调用实参 let-RHS 已通；declared_return_kind 的 CALL-TARGET = 缺 scalar_type_kind 被调，非机器缺口。下片：counters（count_params/find_body_end/count_until_end_function）的 gw while+多 match CONTROL（match 栈机器，解锁 fn_entry 等）、has_return_at else 分支守卫、Bool 参数守卫（T2）、机器本体自举（GWPACK-OTHER 类）。

**STEP-0339（2026-10-03）**：全量函数普查 4 pass 至不动点（单独 → 4被调前缀 → 11函数前缀 → 14函数前缀，oracle 字节对比）——**9 函数新翻绿**（is_declaration_start、literal_value_index、list_aware_parameter_index、is_intrinsic_path、record_scalar_kind、record_close_bracket、record_map_comma、function_declaration_index、function_declaration_ordinal），全经既有机器、无新手术，冻结语料按普查原组合登记。**分母修正**：实测函数头 0319 起 117、0325 起 119、本片起 120；历史 "/115" 为陈旧口径。**前沿地图**（不动点）：深层 CALL-TARGET 传递阻塞于 mid-layer；`has_return_at` 的 CALL-TARGET 实为缺 `is_ident_byte` 被调；`json_string_token` 维持 while+match CONTROL；`u64_list_at` MATCH-SHAPE、`type_json` SCICOND。普查探针已按纪律删除；方法：all_function_names() 逐个 + oracle_ir 对比，前缀递增逼近闭包。

**STEP-0338（2026-10-03）**：守卫链机器（`fixed_guard_chain_ir`）三加宽落地——宿主调用 let（守卫间/前 `let` 经 `gw_rhs_packed` 复用打包 + `write_local` 入条件块前缀 + cell 三列表注册，cell local = cell 下标）、条件打包器 `chain_condition_packed`（双 SKIP 后第三 fallback：`sico.text.length`/`sico.bytes.length`/`sico.bytes.at` 左值 + param/cell/字面量右值，其余形状委托字节已证 `gw_compare_packed`；守卫与 final 两处接线）、有 cell 时发射 `"locals"` 数组。`while_bytes_at_local_src_packed` 字面量 cursor 形态（form 2）按 oracle 转储落地（STEP-0327 遗留疑点裁决）。`is_digit_token`/`is_string_token`/`is_ident_byte` 翻绿，canary +3，R3 0，套件 58/58、runner 全量绿。**宽度约定已钉死**：`fixed_operation_operand_width` 返回 token 数，`atom+width` = 操作数自身 `)` 之后的下一个 token——一切操作数走位依赖此约定（`bytes.at` 字面量游标的逗号 = 游标+width 本身）。**存疑标记**：同函数的 "sico" 规则（1db4624 移植件）对 `sico.<f>.<m>` 以 +4 判 `(`（实际 open 在 +5），当前不在已证路径，修复/验证前不得扩用。会话从 `1db4624`（中断会话 WIP 快照：scalar_ir gw-decline fallback + rustfmt 修正）续起——该快照承载 is_ident_byte 的 control_if-declined→gw 翻绿路径，勿回退。

M22 整体 **NO-GO**。S1/S2 声明子集完成，S3/S4/S5 partial，S6/S7 未进入。R0 已接受 ADR-0017 Option A 与 RFC-0047。STEP-0292 对 `e8495f9` 的独立 Windows GNU [GitHub CI](https://github.com/ImmortalWing/sico/actions/runs/36087222284) 成功，**W1 在本隔离分支快照裁为 GO**。该裁决不覆盖 `origin/dev` 的并行历史。

STEP-0293 是 22-C 首个 R3 计数实现片：while 体 sico.u64.to_text RHS 与 Rust oracle byte-exact，错误参数类型 typed 拒绝；formatter canary 22/30，nearest_match 从 GWPACK-OTHER 移至 CALL-TARGET，整份 exit 122。cb371c5 的独立 Windows GNU CI 运行 36090403437 成功，S1 于本隔离分支裁为 GO。R3 连续停滞数 0。

STEP-0294 是 22-C S2：`sico.map.get[Text,U64]` 的受限 match 主语与 Rust oracle 字节一致；错误 Map/key 类型、参数数和泛型数 typed 拒绝。formatter canary 仍 22/30，`nearest_match` 前沿 CALL-TARGET → STATEMENT，整份 exit 122，R3 连续停滞数仍为 0。本机 0261/0262/0245 全绿，`5a6d0bb` 的独立 Windows GNU [CI 运行 36147743130](https://github.com/ImmortalWing/sico/actions/runs/36147743130) 成功，**S2 于本隔离分支裁为 GO**。

## 1. 证据与边界

| 检查 | 当前结果 | 证据边界 |
|---|---|---|
| STEP-0292 fresh-runner CI | `e8495f9` 的完整 `run-ci.ps1` 作业 success | W1 快照；Windows 2025 GNU |
| STEP-0293 fresh-runner CI | 修复版 `cb371c5` 的完整 `run-ci.ps1` 作业 success（[运行 36090403437](https://github.com/ImmortalWing/sico/actions/runs/36090403437)） | S1 本片；Windows 2025 GNU |
| STEP-0294 fresh-runner CI | 最终 `5a6d0bb` 的完整 `run-ci.ps1` 作业 success（[运行 36147743130](https://github.com/ImmortalWing/sico/actions/runs/36147743130)） | S2 本片；Windows 2025 GNU |
| `selfhost_checker` | 9/9，原 215 项 + W1 五项 Rust/guest 对照 | Windows x64 GNU 真实 runner；internal-fixture |
| S1 新增 compiler 用例 | while 体 `u64.to_text` canonical IR byte-exact，双参数与错误类型 parameter/local typed 拒绝 | 真实 runner；internal-fixture |
| S2 新增 compiler 用例 | literal/parameter/local Text key 字节相等，extra arg/generic、wrong Map/key type typed 拒绝，拒绝后恢复 | 真实 runner；internal-fixture |
| `validate-step-0262.ps1` | compiler 23/23、parser 2/2、local-bounds 1/1、22 函数 IR 快照、现行前沿均绿 | S2 本机复跑；gw 最繁忙函数 240 locals |
| `validate-step-0261.ps1` | 同上及 17 函数历史快照绿 | S2 本机复跑 |
| `validate-step-0245.ps1` | semantic 6+11、bundle 4、checker 9、compiler 23、runner clippy 全绿 | S2 本机复跑 |
| `report-m22-canary.ps1` | 22/30，`nearest_match` / `STATEMENT`，exit 122 | S2 当前树；栈高水位仍未测得 |

## 2. 下一步

1. 按 [STEP-0291](./steps/STEP-0291-m22-nearest-match-design.md) 开始 S3：match 臂内嵌 if/return 的控制流降低，重新钉定 `nearest_match` 字节差分与下一个前沿，并取得本片独立 CI。
2. 合并 `origin/dev` 的并行历史前逐项处理重复 STEP 编号、235 项语料和 `compiler_semantics.sico` 差异；不得把本分支 CI 声称为远端 dev 的验证结果。
3. M23 的 RFC-0048/0049、M24 的 RFC-0050/0051/0052 与 ADR-0018、M26 的 RFC-0053/0054 仍各自等待 owner 接受；它们不阻挡本分支 22-C S3。

## 3. 本机复核命令（PowerShell，仓库根目录）

```powershell
git status --short
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$binutils = Join-Path (Get-Location) 'target\tooling\msys2-binutils\mingw64\bin'
$env:Path = "$binutils;$env:Path"
.\tools\validate-step-0245.ps1
.\tools\validate-step-0261.ps1
.\tools\validate-step-0262.ps1
.\tools\report-m22-canary.ps1
.\tools\validate-step-0124.ps1 -SelfTest
git diff --check
```

STEP-0261/0262 会重复执行较慢的 compiler/parser 差分；只在源码或当前前沿变化后重跑。S6 还必须实测 guest memory/stack 预算，不能将缺失的栈高水位写成已有证据。
