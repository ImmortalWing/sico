# STEP-0506 m22-w2 S5 `error` 臂组件刀：前端枚举 case + codegen `error` 臂 ⇒ **整组件逐字节等同**

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0505](./STEP-0505-m22-w2-s5-error-arm-truth.md)（`error` 臂真值测量）、[STEP-0504](./STEP-0504-m22-w2-s5-derived-component.md)（派生组件）、[STEP-0503](./STEP-0503-m22-w2-s5-run-body-result-return.md)（结果区返回）、[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位/局部声明计划）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是发射侧的第六刀（`error` 臂整组件）**
> - numbering: M22 线顺延为 **STEP-0506**（本线已占用 STEP-0491..0499、STEP-0501..0505；取号时已核对 `docs/steps/` 与并行 worktree `E:\github\sico-m24` 占用的 STEP-0330–0337，**均无 0506**）

## 1. 本片目标与结论

**目标**（owner 2026-10-21 拍板的方向「扩到非参考形态」）：把 STEP-0505 §6 预切计划的**第一刀**落地 —— **前端刀 + codegen 刀**一起做，出口 = **guest 对 `error` 臂 fixture 发射的整个组件，与 Rust backend 的生产输出逐字节相同**。

**结论（实测）**：

- **出口达成**：guest 发射的 `error` 臂组件 = **939 B**，与 oracle `compile_script_program` 的**生产输出逐字节相同**（新固件钉 `sico_codegen_guest_emits_the_error_arm_component_byte_exactly`，含 `wasmparser::Validator` 校验）。
- **前端刀**：枚举 case 操作数降级为**空载荷 `variant` op**（`{"op":"variant","data":{"name":"ScriptErrorCode.InvalidInput","payload":[]}}`），guest IR 与 Rust IR **逐字节相同**（新固件钉 `sico_compiler_lowers_the_enum_case_operand_as_a_variant_byte_exactly`）。
- **codegen 刀**：`cgs_result_return_hex` 的 `error` 臂分支落地（判别值槽、`ScriptError` 两个字段的存法、`ok` 区右对齐后的载荷基槽），局部声明段产出 **17 组 / 18 槽**，结果区返回段产出 **59 B**。
- **组件级补齐（本片新增）**：guest 学会**派生静态 data 段**（distinct 字面量按指令序、单段、offset 0）与 **arena base**（`align_up(data_len, 8)`），并据此派生 **globals[0]** 与 **post-return helper** 的常量 ⇒ 这三处（oracle 有、guest 没有或写死）补齐后组件才可能全等。
- **根因定性（三条，全部实测）**：
  1. **guest 源码 bug（不是前端降级缺陷、不是缓存陈旧）**：`cgs_result_return_hex` 的 `error` 臂里 `crrhe_base` 用了 **ok 臂的 cell** `crrh_tag`（该 cell 在 `error` 路径**从未赋值** ⇒ 恒 0），而不是 `error` 臂自己的 `crrhe_tag` ⇒ base = `0 + (1 + ok_slots 5)` = **6**，而不是 **16** ⇒ 三个 `error` 载荷槽（code/ptr/len）整体错位。**决定性证据**：临时把 `cgs_abi_error_field_slot_offset` 改成返回 5/6 后，guest 产出**立即跟着变**（槽位变 11/12/13）⇒ 运行时确实执行当前源码，不是组件缓存陈旧。
  2. **缺 data 段派生**：oracle 的核心模块有 **data 段（段号 11，12 B）**，guest 的核心模块只有段 {1,3,5,6,7,10} ⇒ 差 12 B。
  3. **arena base 是冻结常量**：guest 的 `cg_core_globals_hex` / `cg_post_body_hex` 把 arena base 写死成 **0**，而 oracle 是 **8**（`align_up(4, 8)`，4 = `"boom"` 的字节数）⇒ 2 处常量各差 1 字节。

**门 3 仍为 NO-GO**：本片没有动 37 语料 / formatter / 全源 canary 阶梯，也没有动 ADR-0015 的 `A == B == C` + `.sapp` 链。

## 2. 实测数据（全部读自 backend 的**生产输出**）

### 2.1 fixture

与 STEP-0505 §2.1 同一个 Script-profile 入口（只把返回值换成 `error` 臂）：

```
function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "boom"))
end function
```

### 2.2 组件解剖（guest 产出 == oracle 产出，逐字节相同）

| 量 | 实测值 |
|---|---|
| 组件总长 | **939 B**（guest 与 oracle **全等**） |
| 核心模块 | **488 B**（组件内 section id 1 的载荷） |
| 核心模块段号 | **{1,3,5,6,7,10,11}** —— 比参考形态（ok 臂）**多出 data 段（11）** |
| `run` 的局部声明段 | **35 B**（组数 LEB 1 B + 17 组 x 2 B；= 17 组 / 18 槽） |
| `run` 的结果区返回段 | **59 B（118 hex）** |
| 静态 data 段 | **12 B**（段头 2 B + 载荷 10 B） |

⇒ **「参考形态无 data 段」这条事实（STEP-0501 §3.2）只对 ok 臂成立**：`error` 臂有字符串字面量 ⇒ backend 把字面量放进 data 段 ⇒ **组件级派生的对象不止 run 体**。

### 2.3 静态 data 段（实测 hex）

```
0b 0a 01 00 41 00 0b 04 62 6f 6f 6d
```

| 字节 | 含义 |
|---|---|
| `0b` | 段号 **11**（core module 的 data 段） |
| `0a` | 载荷长度 **10** |
| `01` | 段数 **1** |
| `00` | 段种类 = **active，内存 0** |
| `41 00 0b` | 偏移表达式 = `i32.const 0` / `end` ⇒ **段放在内存 0** |
| `04` | 字节数 **4** |
| `62 6f 6f 6d` | `"boom"` |

⇒ **arena base = `align_up(4, 8)` = 8**：backend 的 bump 分配器从 8 开始（`arena_base = (data_bytes.len() + 7) & !7`）。

⇒ arena base 出现在**两处**：核心模块 **globals[0]** 的初值（`i32.const 8`）与 **`cabi_post_run` helper** 的第一条常量（`i32.const 8`）。guest 原先把这两处写死成 **0**。

### 2.4 `error` 臂的槽位（IR 派生，不是写死的数字）

| 槽 | 值 | 来源 |
|---|---|---|
| 形参 | 0–3 | `ScriptInput` 的 4 槽 |
| 结果 1（枚举 case） | 4 | 判别值槽 |
| 结果 2（string） | 5–6 | 指针 / 长度 |
| 结果 3（`ScriptError`） | 7–9 | code / 指针 / 长度 |
| 结果 4（`variant error`） | **10–18** | 判别值 **10**；`ok` 区 **11–15**（5 槽）；**error 载荷 16/17/18** |
| dispatcher | 19 | |
| scratch 对 | 20 / 21 | `return_area` / `copy_destination` |

⇒ **载荷基槽 = 判别值槽 + 1 + `ok` 区的槽数** = 10 + 1 + 5 = **16**。这正是根因 1 里算错的那一步（guest 算成 6）。

## 3. 实现（guest 侧）

### 3.1 前端刀（`selfhost/parser.sico`）

| 新函数 | 作用 |
|---|---|
| `record_field_type_word` | 取已声明形状的字段**类型词**（与「标量 kind」分开：`ScriptError.code` 的类型词是 `ScriptErrorCode`，不是标量 kind） |
| `enum_case_declared` | 判断「`Enum.Case`」是否指向**已声明的 enum 成员**（不是 record） |
| `enum_case_end` | 定位 enum 成员的声明结束 |
| `enum_case_instruction_json` | 发出 `{"op":"variant","data":{"name":"Enum.Case","payload":[]}}` |
| `construct_enum_case_ir` | 在 `construct` 的字段值位置接上枚举 case 这一形状 |

新 typed 身份：`ERR:E-SH-IR-RANGE`（枚举 case 的源码范围不可解析）。原有的 `ERR:E-SH-IR-CONSTRUCT-VALUE` 仍是「形状不认识」的兜底身份。

### 3.2 codegen 刀（`selfhost/codegen.sico`）

| 族 | 函数 |
|---|---|
| 枚举 case 发射 | `cgs_enum_variant_hex` / `cgs_enum_case_tag`（**冻结的 canonical ABI 判别值**，不是源码声明序） |
| 字面量与 data 段 | `cgs_literal_text` / `cgs_data_offset` / `cgs_const_string_hex` / `cgs_data_hex` / `cgs_text_bytes_hex` / `cg_hex_digit_i64` / `cg_hex_byte_i64` / `cgs_data_section_hex` |
| arena base | `cgs_arena_base`（`align_up(len, 8)`）、`cg_core_globals_hex(arena_base)`、`cg_post_body_hex(arena_base)` —— **两处从冻结常量改为派生** |
| `error` 臂结果区 | `cgs_result_arm_slots` / `cgs_abi_error_field_name|kind|area|area_len|slot_offset` / `cgs_abi_error_field_matches` / `cgs_result_scalar8_hex` / `cgs_result_error_field_hex` / `cgs_result_error_fields_hex` |

**派生 vs 冻结的分界**（与 STEP-0487/0503 同纪律）：

- **组件级冻结事实**由 guest 携带：结果区 32 B、分配器索引 1、`ScriptError` 字段名/顺序/flat 种类、**结果区偏移**（code @8、message 指针 @12 / 长度 @16）。
- **IR 派生事实**由 guest 重算：判别值槽、**载荷基槽**、字段的 **flat 槽偏移**（`ScriptError` = code @+0、message 指针 @+1 / 长度 @+2 —— 这是**局部变量的 flat 布局**，与上面的**结果区偏移**是两套，必须分清）。
- **交叉校验 fail-closed**：字段名/顺序/flat 种类/槽数任一与冻结事实不符即 typed 拒绝（`ERR:E-SH-CG-IR-EMIT`），**不静默取一侧**。

### 3.3 根因 1 的修复（一行）

```
-        let crrhe_base = cgs_add(crrh_tag, cgs_add(U64.literal(1), crrhe_ok_slots))
+        let crrhe_base = cgs_add(crrhe_tag, cgs_add(U64.literal(1), crrhe_ok_slots))
```

⇒ base = 10 + 1 + 5 = **16**。

## 4. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 前端刀：枚举 case 操作数逐字节 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_lowers_the_enum_case_operand_as_a_variant_byte_exactly` | **ok** |
| codegen 刀：**整组件**逐字节 | 同上 `sico_codegen_guest_emits_the_error_arm_component_byte_exactly` | **ok**（939 B == oracle 生产输出 + `wasmparser` 校验） |
| 既有参考形态（ok 臂）组件钉仍绿 | 同上 `sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly` | **ok** |
| 固件套件 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1` | **106 passed / 0 failed**（删掉 8 个临时探针后；STEP-0505 时为 104 + 2 新钉） |
| 全量电池 | 见 §4.1 | 见 §4.1 |

> 本机验证电池结果见 §4.1。

## 4.1 本机验证电池

`target/tmp/step0504-battery.ps1`（一键 CI 放在最前、机器最空闲时跑）：

| 步骤 | 结果 |
|---|---|
| `fmt (runner)` | PASS |
| one-key CI（`tools/run-ci.ps1`，**12 步**） | **CI GREEN** |
| M22 regression（0245） | PASS |
| M22 checker（0262） | PASS |
| 前端前沿探针 | PASS |
| IR 计划层前沿探针 | PASS |
| 规划合同（0124） | PASS |
| 文档校验器（0131） | PASS |
| 文档校验器（0156） | PASS |
| 空白检查（`git diff --check`） | PASS |
| 行尾审计（纯 LF、零 CRLF、零 BOM） | PASS |

⇒ **11/11 BATTERY_GREEN**（第二轮，宿主空闲虚拟内存恢复到 ~31 GB 时跑的；日志：`target/tmp/step0506/battery-2.log`）。

**第一次电池跑红过一次，如实记录**：`one-key CI` 与 `M22 regression (0245)` 报 `selfhost_checker` 的
`sico_checker_matches_the_complete_frozen_diagnostic_partition` 失败，断言内容是
`Launch("页面文件太小，无法完成操作。 (os error 1455)")` —— **Windows 提交限额（commit limit）耗尽**，
**不是诊断分区不匹配**：宿主上每个 guest 预留 **2 GiB** 线性内存（提交额度），当时宿主的空闲虚拟内存只剩
**~7.8 GB**（用户的 ComfyUI 进程常驻 ~3.5 GB）。处置：①**同一测试串行重跑 9/9 绿**；②空闲内存恢复后重跑整轮电池
**11/11 绿**。本片**没有**触碰 checker 的代码路径（`selfhost/checker.sico` 零改动），该红**不构成本片的缺陷**，
但按纪律**记为环境敏感项**（宿主提交额度），不写成"全绿即无风险"。

## 5. 诚实性

- **本片只覆盖一个 fixture**。`error` 臂的载荷是 `ScriptError(code: 枚举, message: list)`；`code` 是**枚举 case**、`message` 是 **list**。别的记录里若出现「枚举 case 作为字段值」，走的是同一条前端路径，但**没有被独立测量**。
- **组件里仍有冻结骨架**：类型/导入/别名/导出段 + `alloc`/`realloc`/`post` 三个 helper 体**仍是冻结常量**。IR 相关字节 = `run` 体 **169 B**（局部声明 **35 B** + code **134 B**）+ data 段 **12 B** = **181 B / 939 ≈ 19.3%** ⇒ guest 现在是「**冻结骨架 + IR 派生的 run 体与 data 段**」，**不是**通用组件发射器。
- **重复字面量仍 typed 拒绝**：`cgs_data_offset` 对「出现两次以上的字面量」返回 0 ⇒ `ERR:E-SH-CG-IR-EMIT`（共享偏移不可从字面量本身派生）。本片**没有**放宽这条。
- **多函数模块仍 typed 拒绝**（STEP-0504 的守卫仍在）。
- **根因 1 是 guest 自己的源码 bug**，不是前端降级缺陷、也不是组件缓存陈旧（标记实验证明运行时执行当前源码）。**这条不能被读成「guest 前端能降级就等于发射正确」**——两件事要分别测。
- **测试文件里 53 处字符曾被按 CP936 误解码**（em-dash / 箭头变成 `U+95B3` + `?`，并吞掉了行尾空格或换行）。本片已按 `HEAD` 逐行对齐复原（47 个 em-dash + 6 个箭头，与 `HEAD` 的非 ASCII 清单**完全一致**），并复核三个改动文件**纯 LF、零 CRLF、零 BOM**。
- oracle 生产代码**零改动**；读取器与真值都写在测试文件内。
- **独立 CI 尚未运行**：本片**未提交、未推送**（owner 未授权）⇒ 结论只到「本机全绿」。
- 本片**没有**触碰 37 语料阶梯、formatter、全源 canary 或 `.sapp` 链。**门 3 仍为 NO-GO**。

## 6. 下一片

1. **多函数 / data 段模块**：guest 的**函数段**目前是冻结的（run/alloc/realloc/post 四个），backend 遇到第二个源函数会多发一个核心函数 ⇒ 需要把**函数段**也做成派生的（并同步 data 段在多函数下的顺序规则：backend 的 `script_data` 按「模块内函数顺序 → 块顺序 → 指令顺序」收集字面量）。
2. 之后才是 **37 语料 → formatter → 全源 canary → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。

> 提醒（承接 [handoff-m22.md](../handoff-m22.md) §4/§5）：guest 前端的**形状前沿**是「新代码必须能被 guest 自己降低」的全部已知约束之一。本片新增的两条实战约束：①**同函数内 `let` 无块作用域** ⇒ 派生变量要在分支外声明并给显式默认值；②**调用目标必须写对全名**（`cgs_list_holds` 少写一个 `s`` ⇒ `E2031 unresolved call target`，且该诊断来自 Rust 语义层，不是 guest 前端）。
