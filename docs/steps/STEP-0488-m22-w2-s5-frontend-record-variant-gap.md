# STEP-0488 m22-w2 S5 IR 驱动发射的前置缺口定位（前端 record/variant 降低）

> - status: diagnosis / registration（实现未开工；本片把 S5 波次下一刀的**前置阻塞**与**发射算法规格**钉死）
> - date: 2026-10-20
> - owners: autonomous-agent
> - depends: [STEP-0486](./STEP-0486-m22-w2-s5-codegen-wave.md)（波次合同）、[STEP-0487](./STEP-0487-m22-w2-s5-codegen-skeleton.md)（骨架刀）、RFC-0011、ADR-0015/0016
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次第 2 刀 = IR 驱动 run 体

## 1. 本片目标与结论

**目标**：把 S5 波次第 2 刀的定义收紧为可施工的合同——guest codegen 消费**已证的 canonical IR JSON**（`sico.ir.v0`）并发射与 Rust backend 逐字节相同的 Component，而不是像 STEP-0487 那样对冻结源形硬编码。

**结论（实测）**：该刀被一个**前端缺口**挡在前面——guest 前端（`selfhost/parser.sico` 的降低机器）目前**不能**降低 Script-profile 形态（record 形参投影 / `ok(...)`·`some(...)` 变体 / record 构造）。没有这一步产出的 IR，"IR 驱动发射"没有输入。

因此本片做两件事：①用可复现探针把缺口边界钉死；②把 Rust 侧发射算法**逆向规格化**（本片逐行核对 `crates/sico-codegen-wasm/src/lib.rs` 与 `canonical.rs` 并反汇编 guest 骨架产出的 981 B 组件），使下一刀只剩"照规格实现"，不再有摸索成本。

## 2. 前置缺口（硬证据）

探针（可复现）：`tools/probe-m22-frontend-frontier.ps1`（exit 0 = 前沿未移动；探针把每个形态的判定与 typed 拒绝身份一起钉住）。

| 形态 | guest 结果 | 判定 |
|---|---|---|
| `function probe(value: I64) returns I64` | 降低成功，IR 与 Rust oracle 逐字节相同 | 已支持 |
| record 声明（`record ScriptInput: field stdin: Bytes end record`） | 降低成功，且 **IR 的 `"records"` 表已按 Rust canonical 格式产出** | 已支持 |
| record 形参 + 字段投影（`input.stdin`） | typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | **缺口 1 —— 已由 [STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md) 闭合**（现降低为 canonical `project` op，与 Rust oracle 逐字节相同）；投影基名硬编码由 [STEP-0492](./STEP-0492-m22-w2-s5-generic-record-projection.md) 解除 |
| 命名返回类型（`returns <已声明 record>`） | typed 拒绝 `ERR:E-SH-IR-RETURN-TYPE` / `ERR:E-SH-IR-PARAMETER-TYPE` | **缺口 1 的构造侧前置 —— 已由 [STEP-0493](./STEP-0493-m22-w2-s5-named-record-return-type.md) 闭合**（直返同名形参逐字节等同 oracle；带语句体仍 typed 拒绝，属**收窄**而非能力对齐） |
| `Result[I64, NumericError]` 返回 + `ok(...)` | typed 拒绝 `ERR:E-SH-IR-CALL-TARGET` | **缺口 2 —— 已由 [STEP-0495](./STEP-0495-m22-w2-s5-result-variant.md) 闭合**（现降低为 canonical `variant` op，与 Rust oracle 逐字节相同；payload 类型与声明分臂不符时 typed 拒绝 `ERR:E-SH-IR-VARIANT-TYPE`，属**更严**而非能力对齐） |
| `Option[I64]` 返回 + `some(...)` | typed 拒绝 `ERR:E-SH-IR-CALL-TARGET` | **不是缺口** —— 实测 Rust oracle 对 `some(...)` 直接报 `E2031 unresolved call target some`（语义期拒绝），即 **`Option` 的 `some` 变体在当前 Sico 语言里不存在**；guest 与 oracle 是**同一裁决**（都拒），只是诊断通道不同 |
| record 构造（`ScriptOutput(stdout: ..., ...)`） | typed 拒绝 `ERR:E-SH-IR-CALL-TARGET` | **缺口 3 —— 已由 [STEP-0494](./STEP-0494-m22-w2-s5-record-construct.md) 闭合**（现降低为 canonical `construct` op，与 Rust oracle 逐字节相同；字段值语法比 oracle 窄，属**收窄**） |
| Script-profile `run`（record 构造 + 投影 + `ok(...)`） | typed 拒绝 `ERR:E-SH-IR-CALL-TARGET` | **缺口 1+2+3 —— 已全部闭合**（缺口 1 由 STEP-0491/0492 闭合、缺口 3 由 STEP-0494 闭合、缺口 2 由 STEP-0495 闭合；Script-profile `run` 现逐字节等同 Rust oracle，前沿钉已由 typed refusal 重钉为逐字节等同） |

缺口 3 = record **构造**（`ScriptOutput(stdout: ..., ...)`）本身被当作未知 call target。

缺口 3 的构造侧还依赖**命名返回类型**（`returns ScriptOutput`）——该前置已由 [STEP-0493](./STEP-0493-m22-w2-s5-named-record-return-type.md) 闭合；缺口 3 本身已由 [STEP-0494](./STEP-0494-m22-w2-s5-record-construct.md) 闭合（canonical `construct` op，与 Rust oracle 逐字节相同）；缺口 2（`ok(...)` / `error(...)` 两个 `Result` 变体）已由 [STEP-0495](./STEP-0495-m22-w2-s5-result-variant.md) 闭合（canonical `variant` op，与 Rust oracle 逐字节相同）。**STEP-0488 登记的前端三项缺口至此全部闭合**。

**新增的一条前置缺口（STEP-0495 实测登记）——IR 的传输容量**：全模块 canonical IR 已从 `HEAD` 的 8,231,922 B 增长到 **8,608,383 B**，**越过 Script v0 单通道 8 MiB 的冻结上限**（RFC-0029 `MAX_CHANNEL_BYTES` = 8,388,608 B）→ guest 编译器**无法再把整模块 IR 经单通道送回宿主**（结局是确定性的传输拒绝 `Trap("malformed result: stdout exceeds 8 MiB")`，**不是**降低失败）。STEP-0495 的绕行是 `selfhost/compiler.sico` 新增的 **`--emit-ir-split` 驱动模式**（IR 按字节中点切两半走 stdout/stderr，拼接即完整 IR；拒绝仍按同一 typed identity 走 `ScriptError`）。**真正的修法是 S5 的「guest IR 摄取」**——IR 在 guest 进程内被消费，不再往返 Script 通道；这条缺口与 IR 摄取、run 体发射一起构成门 3 的剩余清单。

**「guest IR 摄取」缺口的进展（STEP-0496 实测登记）**：IR 摄取**已完成第一刀**——`selfhost/codegen.sico` 现在能在 guest 进程内**扫描 canonical IR JSON**（字节扫描器）并从中**提取 `run` 函数的发射计划**（`param`/`inst`/`op`/`term`/`record` 五类行），该计划文本与 Rust oracle 从同一 `sico_ir::Module` 推出的文本**逐字节相同**（见 [STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md)）。**尚未完成**：槽位/布局计划与 **run 体字节发射**（STEP-0487 的 `--emit-component-hex` 仍是冻结的最小 echo 骨架，未接到计划上）。计划子集外的形状各有 typed identity（`ERR:E-SH-CG-IR-PLAN` / `-TYPE` / `-OP` / `-TERMINATOR`），新探针 `tools/probe-m22-ir-plan-frontier.ps1` 把这层前沿单独钉住。

补充事实：Script-profile 的任何合法程序都必须有 `run(input: ScriptInput) -> Result[ScriptOutput, ScriptError]`，即 record 形参 + record 投影 + record 构造 + Result 变体四件套同时出现。**结论：门 3 的 IR 驱动路径必须先补齐前端这三类形状的降低**（含与 Rust oracle 的逐字节 IR 等价钉），否则 guest 端永远拿不到可发射的 Script-profile IR。

## 3. 发射算法规格（Rust 侧逐行核对 + 字节反汇编）

目标字节：最小 echo fixture 的 Component = **981 B**（core module 530 B，其中 run 体 223 B + alloc 84 B + realloc 72 B + post 32 B；code 段 403 B）。

### 3.1 局部槽位计划（`LocalLayout::plan`，lib.rs:1810-1938）

1. 形参按 `flat_ir_types` 展平并**依次**占槽（run 形参 `ScriptInput` → 4 × i32：槽 0..3 = arguments(ptr,len) / stdin(ptr,len)）。
2. 随后按**指令顺序**，每条指令的值按其 flat 依次占槽（`project`→2、2；`const_i64`→1（i64）；`construct`→5 = 4×i32 + 1×i64；`variant`→9 = tag i32 + ok 5 + error 3）。
3. 之后是函数局部 cell（echo fixture 为 0）。
4. `dispatcher` = 上述槽位总数（echo fixture = 23）；`scratch` = (24, 25) = (return_area, copy_destination)。
5. **局部声明分组**：每个槽单独成组 `(1, ty)`，仅 scratch 成组 `(2, I32)`——echo fixture = 21 组 / 22 个局部。

### 3.2 指令发射（`compile_function` + `compile_instructions`）

- 全体函数都是**调度器**结构：`i32.const <entry>` → `local.set dispatcher` → `block` → `loop` → （每块：`local.get dispatcher; i32.const <block id>; i32.eq; if` … `end`）→ `unreachable` → `end` → `end` → `unreachable` → `end`。
- `project` = `copy_slots(base 的该字段槽, 结果槽)`：逐槽 `local.get` + `local.set`。
- `construct` = 逐字段 `copy_slots(源值槽, 目标连续区间)`（目标区间按 IR 字段顺序累加）。
- `const_i64` = `i64.const <v>` + `local.set <槽>`。
- `variant` = `i32.const <tag>` + `local.set <tag 槽>`；随后 `start`（`error` 变体 = 右对齐）区零填 → 载荷 `copy_slots` → 尾部零填。零填按槽类型选 `i32.const 0` / `i64.const 0`（`zero_local`）。

### 3.3 Script 返回（`emit_script_return` + `emit_result_fields`）

`Result` 的 Canonical ABI 事实（来自 `wit/script-profile-v0/world.wit`，Rust 由嵌入 WIT 解析，guest 侧应按同一冻结事实硬编码）：

| 记录 | 字段（声明序） | 字节偏移 | 尺寸/对齐 |
|---|---|---|---|
| `ScriptInput` | arguments(list\<string>) 0/4，stdin(list\<u8>) 8/12 | 0, 8 | 16 / 4 |
| `ScriptOutput` | stdout 0/4，stderr 8/12，exit_code(s64) 16 | 0, 8, 16 | 24 / 8 |
| `ScriptError` | code(enum) 0，message(string) 4/8 | 0, 4 | 12 / 4 |

结果区：`result_payload_offset = align_to(1, max(align)) = 8`；`result_size = align_to(8 + max(output.size, error.size), 8) = 32`；tag 存于字节 0（`i32.store8`）。

发射序列（echo fixture 实际字节）：

```
i32.const 32 ; call alloc ; local.set <return_area>       # 分配 32 B 结果区
local.get <return_area> ; local.get <tag> ; i32.store8 offset=0
# 逐字段（ScriptOutput 声明序）：
#   list 字段：local.get len ; call alloc ; local.set <copy_dst>
#              local.get copy_dst ; local.get ptr ; local.get len ; memory.copy
#              local.get <return_area> ; local.get copy_dst ; i32.store offset=(8+byte_offset)
#              local.get <return_area> ; local.get len    ; i32.store offset=(8+byte_offset+4)
#   s64 字段 ：local.get <return_area> ; local.get <slot> ; i64.store offset=(8+byte_offset) align=3
local.get <return_area> ; return
```

`has_ok`/`has_error` 判定 = 返回值的字段表是否**全部**含该记录字段名（kebab→snake 归一：`exit-code` → `exit_code`）；两者皆不满足 → typed 拒绝。

### 3.4 guest 侧还需要复刻的 Rust 细节

- `record(name)` **优先**取 IR 声明的 `user_records`（`register_user_records`：flat = 字段顺序拼接，byte_offset = slot_start×8，size = flat×8，align = 8），只有不可展平（如 `ScriptError.code` 的 enum 不可展平）时才回落到 WIT 冻结布局。guest 必须复刻这一优先级，否则布局与 Rust 不一致。
- `flat_ir_types`：bool→i32；i64/u64→i64；string/bytes/map/set→(i32,i32)；list→(i32,i32)；result→i32 + ok + error。
- 变体布局：`ok` 变体 = tag + 载荷 + error 区；`error` 变体 = tag + ok 区 + 载荷（右对齐）。

## 4. 有序工作清单（下一刀起）

> 编号说明：本工作树在 2026-10-20 23:48 出现了**并行 M23 会话**的 `tools/validate-step-0489.ps1`（M23 residual closure A，诊断协议扩展），即 **STEP-0489 已被 M23 占用**；M22 线本线下一片从 **STEP-0490** 起取号。**2026-10-21 更新**：并行 M23 会话先占用了 STEP-0490，故 M22 的 record 投影片实际落在 **[STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md)**（本清单第 1 项，已完成）。

1. **前端 record 投影**：`input.stdin` → IR `project` op（`{"op":"project","data":{"base":N,"field":"stdin"}}`），与 Rust oracle 逐字节等价钉。**（STEP-0491 完成；仅标量字段 + `ScriptInput` 形参，且只在无语句的 straight 形态上识别）**
2. **前端 record 构造**：`ScriptOutput(stdout: ..., ...)` → IR `construct` op（字段序 = 源序，字段值 = 结果 id）。**（STEP-0494 完成；字段值语法比 oracle 窄，属收窄）**
3. **前端变体**：`ok(x)` / `error(x)` → IR `variant` op（`{"op":"variant","data":{"name":"ok","payload":[id]}}`）。**（STEP-0495 完成；payload 类型必须校验，否则产出畸形 IR）**
4. **前端 Result 形参/返回**：形参 `Result[...]`、返回 `Result[...]` 的类型 JSON 与 slot 展平。
5. **guest codegen IR 摄取**：`selfhost/codegen.sico` 新增 IR JSON 扫描器（对象/数组/字符串/数字，深度 1 键查找）+ 布局/槽位计划 + op 发射 + Script 返回发射；`--emit-component-hex` 由"冻结源形"切换为"IR 驱动"，非支持形状一律 fail-closed。**（STEP-0496 完成前半：IR JSON 扫描器 + `run` 发射计划提取已落地且与 Rust oracle 逐字节等同；布局/槽位计划、op 发射与 Script 返回发射仍未做，`--emit-component-hex` 仍是"冻结源形"）**
6. **canary 阶梯**：最小 echo fixture → 37 语料 → formatter → 全源 bundle → ADR-0015 `A == B == C` + `.sapp`（ADR-0016 第七项顺序纪律）。

## 5. 证据链修复（0262 harness 的 BOM 注入）

跑固件电池时 `tools/validate-step-0262.ps1` 在本机红了：`formatter prefix IR diverged from the Rust canonical dump`。逐层定位后确认**不是 guest 缺陷、也不是冻结 fixture 过期**，而是**校验器的 stdin 投递方式**：

- Windows PowerShell 5.1（.NET Framework）的 `Process.StandardInput` 会在写入前吐出 UTF-8 **BOM 前导**；实测 guest 看到的 `source_len` = 68 → 71（+3 字节），于是**全部 source range 整体右移 3**。
- 同一份组件、同一份字节，改用**文件重定向**投递（`cmd /c "runner ... < src > out"`）时 guest 看到 `source_len` = 68，range 与冻结 fixture（`tools/fixtures/step-0262/fc_expected.json`）**逐字节相同**。
- 因此：guest lowering 正确、fixture 当前有效；红的是 harness。`.NET Framework` 无 `ProcessStartInfo.StandardInputEncoding`（PS 5.1 实测属性不存在），无法从写入侧关闭前导。

**修复**：0262 校验器改用**字节精确的文件重定向**投递 stdin（新增 `Invoke-GuestWithStdin` 助手，5 处进程启动全部替换），使电池在 PS 5.1 与 PS 7 上都按同一字节语义运行。

**系统性登记**：同一投递模式还出现在 `tools/report-m22-canary.ps1`、`tools/probe-m22-budget.ps1`、`tools/update-m22-corpus.ps1` 与 `tools/validate-step-0246..0261.ps1`。本片只修 0262（当前红且被 STATUS 引用的电池）；其余脚本要么只做覆盖/存在性判定（对 3 字节前导不敏感），要么需要同样的字节精确改造——登记为后续工程项，**不得**默认它们在 PS 5.1 上与 PS 7 等价。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | `FRONTIER_UNCHANGED`，exit 0（6/6 判定与记录一致） |
| 固件电池 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` | **83 passed / 0 failed**（含新前沿钉；60.89 s） |
| 固件电池（0262） | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`**，exit 0（BOM 修复后；selfhost_compiler / selfhost_parser / local-bounds + formatter 前缀 IR 冻结对拍全绿） |
| 组件字节对拍基座 | `sico_codegen_guest_emits_component_bytes_byte_exactly` | guest 981 B 与 Rust oracle 逐字节相同 + `wasmparser` 通过 |
| IR 地面真值 | `crates/sico-codegen-wasm/tests/scratch_dump.rs`（临时）+ `target/tmp/wasm_dump.py`（临时反汇编器） | echo fixture 的 canonical IR JSON 与 223 B run 体的逐条指令反汇编 |

## 7. 诚实性

- 本片**没有**新增 guest 发射能力；门 3 仍为 NO-GO。
- 探针记录的是**当前前沿**（三个缺口），不是能力声明；缺口一旦闭合，探针必须随之前移并重钉（与既有"钉随推进重钉"纪律一致）。
- 规格中的每个数字都来自 Rust 源码核对与字节反汇编，**不是**设计文档推断；凡本片未在字节层验证的细节（如 `user_records` 覆盖优先级对非 echo 形状的影响）已在 §3.4 单独标注为"guest 必须复刻"的清单项。
