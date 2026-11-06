# STEP-0495 m22-w2 S5 变体 `ok(...)` / `error(...)` + Script v0 通道容量边界

> - status: implementation complete locally
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0494](./STEP-0494-m22-w2-s5-record-construct.md)（record 构造）、[STEP-0493](./STEP-0493-m22-w2-s5-named-record-return-type.md)（命名返回类型）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)（缺口定位 + 发射规格）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次第 5 刀 = IR 驱动 run 体 —— **本片闭合 Script-profile 三类形状中的第 3 类（变体）**
> - numbering: M22 线顺延为 **STEP-0495**（本线已占用 STEP-0491/0492/0493/0494；并行 M23 会话占用 STEP-0489/0490；已核对 `docs/steps/` 无冲突）

## 1. 本片目标与结论

**目标**：闭合 [STEP-0494](./STEP-0494-m22-w2-s5-record-construct.md) §8 登记的变体缺口——guest 前端能降低 `ok(...)` / `error(...)`，并产出与 Rust oracle **逐字节相同**的 canonical IR。

**结论（实测）**：变体已闭合。

`ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))` 现在降低为：

```json
{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":433,"end":439}},
{"result":2,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":454,"end":460}},
{"result":3,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":0},"range":{"start":485,"end":486}},
{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct","data":{"name":"ScriptOutput","fields":[{"name":"stdout","value":1},{"name":"stderr","value":2},{"name":"exit_code","value":3}]}},"range":{"start":407,"end":488}},
{"result":5,"ty":{"kind":"result","data":{"ok":{"kind":"named","data":"ScriptOutput"},"error":{"kind":"named","data":"ScriptError"}}},"operation":{"op":"variant","data":{"name":"ok","payload":[4]}},"range":{"start":404,"end":489}}
```

terminator 为 `{"kind":"return","data":5}`（返回 variant 的结果 id）。**与 Rust oracle 逐字节相同**（固件钉 `sico_compiler_lowers_result_variant_byte_exactly`，4 个 fixture）。

**Script-profile 三类形状（投影 / 构造 / 变体）至此全部闭合**：STEP-0488 缺口表的第 2 项已无残留。

**但门 3 仍为 NO-GO**：guest IR 摄取、run 体发射、37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。

## 2. 本片同时暴露并修复的一条**容量边界**（比变体本身更重要）

### 2.1 事实

| 项 | 数值 |
|---|---|
| Script v0 单通道上限（RFC-0029 `MAX_CHANNEL_BYTES`，冻结契约） | **8,388,608 B**（8 MiB） |
| `HEAD` 版 `selfhost/parser.sico` 源码 | 943,734 B |
| `HEAD` 版源码的 canonical IR（Rust oracle 实测） | **8,231,922 B**（余量仅 156,686 B ≈ 1.9%） |
| 本片完成后 `selfhost/parser.sico` 源码 | 989,595 B |
| 本片完成后源码的 canonical IR（Rust oracle 实测） | **8,608,383 B**（**超限 219,775 B**） |

### 2.2 后果

全源自举 census 钉（`sico_guest_arena_accommodates_full_parser_selfhost_input`）此前把**整个模块的 IR** 经 Script v0 `stdout` 通道取回再做逐字节对拍。IR 越过 8 MiB 后，runner 在**提升返回值**时按冻结上限拒绝 → 结局从 `Output(...)` 变成 `Trap("malformed result: stdout exceeds 8 MiB")`。

**这不是降低失败**：trap 发生在 guest **已经返回**之后（提升阶段），所以 guest 的全模块降低确实跑完了；但**逐字节证据**取不回来了。

**根因不是"变体实现太啰嗦"，而是这条 census 通道从 STEP-0491 起就只剩 ~1.9% 余量**（HEAD 基线 8,231,922 B vs 上限 8,388,608 B），STEP-0491..0495 的累积增长必然把它顶穿。

### 2.3 修复：新增 `--emit-ir-split` 驱动模式（双通道分片）

`selfhost/compiler.sico` 新增一个**驱动模式**（与 STEP-0487 的 `--emit-component-hex` 同族）：

- `--emit-ir-split`：把 canonical IR **按字节中点切成两半**，`stdout` 承载前半、`stderr` 承载后半；
- 调用方**按字节拼接 stdout + stderr** 即得完整模块 IR（字节级无损：`stdout`/`stderr` 的 Script v0 载体是 `Bytes`（u8 列表），runner 不做 UTF-8 校验，中点落在多字节字符内部也只会让两半各自不完整，拼接后仍是原始字节）；
- 降低失败（`ERR:` 前缀）仍然以**同样的 typed identity** 走 `ScriptError` 返回，**不因分片而隐藏拒绝**（固件钉 `sico_compiler_emits_the_module_ir_split_across_both_channels_byte_exactly` 同时钉住 fail-closed 行为）。

**为什么是这个方案**：RFC-0029 的 8 MiB 通道上限是**冻结的协议契约**，本片无权也不应抬高；`stdout` + `stderr` 两个通道各 8 MiB 合计 16 MiB，足以承载 8.6 MB 的模块 IR。

**诚实的边界**：**默认单通道路径**（不给 `--emit-ir-split`）在全模块规模上**已经装不下**了——这是登记在案的限制，不是被掩盖的失败。S5 的最终形态本来就是"guest 内部 source → IR → Component"，IR JSON 走 Script 通道只是**自举诊断**通道；真正的修法是 S5 的 **IR 摄取**切片（guest 内部消费 IR，不再让它往返 Script 通道）。

## 3. 地面真值（Rust oracle 逐字节）

### 3.1 `ok(...)` 包标量（fixture 137 B）

```
enum NumericError:
  case Invalid
end enum

function probe() returns Result[I64, NumericError]:
  return ok(I64.literal(1))
end function
```

```json
"return_type":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"NumericError"}}}
{"result":0,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":1},"range":{"start":120,"end":121}},
{"result":1,"ty":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"NumericError"}}},"operation":{"op":"variant","data":{"name":"ok","payload":[0]}},"range":{"start":105,"end":123}}
```

terminator `{"kind":"return","data":1}`，函数 range `44..137`，`source_len` 137。

> 同一形状**去掉 enum 声明**时（固件钉的另一个 fixture，93 B）：函数 range `0..93`，`const_i64` range `76..77`，`variant` range `61..79`。**enum 声明不影响分臂类型 JSON**（错误臂是**命名类型**，不是 enum 的 case 列表）。

### 3.2 `error(...)` 包 record 构造（fixture 202 B）

```json
"return_type":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"ScriptError"}}}
{"result":0,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":2},"range":{"start":168,"end":169}},
{"result":1,"ty":{"kind":"string"},"operation":{"op":"const_string","data":"bad"},"range":{"start":181,"end":186}},
{"result":2,"ty":{"kind":"named","data":"ScriptError"},"operation":{"op":"construct","data":{"name":"ScriptError","fields":[{"name":"code","value":0},{"name":"message","value":1}]}},"range":{"start":138,"end":187}},
{"result":3,"ty":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"ScriptError"}}},"operation":{"op":"variant","data":{"name":"error","payload":[2]}},"range":{"start":132,"end":188}}
```

terminator `{"kind":"return","data":3}`。

### 3.3 Script-profile `run`（fixture 503 B）

```json
"return_type":{"kind":"result","data":{"ok":{"kind":"named","data":"ScriptOutput"},"error":{"kind":"named","data":"ScriptError"}}}
{"result":1,...,"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":433,"end":439}},
{"result":2,...,"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":454,"end":460}},
{"result":3,...,"operation":{"op":"const_i64","data":0},"range":{"start":485,"end":486}},
{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct",...},"range":{"start":407,"end":488}},
{"result":5,"ty":{"kind":"result",...},"operation":{"op":"variant","data":{"name":"ok","payload":[4]}},"range":{"start":404,"end":489}}
```

terminator `{"kind":"return","data":5}`。

### 3.4 从真值读出的七条规格

1. `Result[T, E]` 的类型 JSON = `{"kind":"result","data":{"ok":<T>,"error":<E>}}`（`T`/`E` 各自是完整类型 JSON）。
2. `variant` op = `{"op":"variant","data":{"name":"ok"|"error","payload":[<ids>]}}`；**payload 是 id 列表**。
3. `variant` 的 `ty` 是**整个 Result 类型**，不是 payload 类型。
4. **`return_type` 恒按声明顺序 `ok` 在前、`error` 在后**——**不随实际构造的是哪个分臂而交换**（`error(...)` 的 `return_type` 仍是 `ok:<T>, error:<E>`）。首版实现把两臂对调了，由 oracle 真值当场抓出。
5. **嵌套顺序**：先降 payload（`ok(ScriptOutput(...))` 里 construct 先入列，result 4），再降 variant（result 5）——与 STEP-0494 construct helper 的"字段值先降、op 后降"完全同构，**复用同一套记账**。
6. **result id 起点 = `parameter_count`**：`ok(I64.literal(1))` 无形参 → 第一个指令 result **0**；Script-profile 有一个形参 → 第一个指令 result **1** ✓。
7. `variant` 的 range = `ok(...)` / `error(...)` 的整个表达式跨度（105..123 / 132..188 / 404..489），与 construct 的"构造名..右括号后"同族规则。

## 4. 关键设计决策

### 4.1 走 helper 家族，不改 `scalar_ir` 的深层 return 分析（沿用 STEP-0494）

本片新增**顶层浅嵌套**的 helper：

| 新增函数 | 职责 |
|---|---|
| `type_word_json(words, type_idx) returns Text` | 一个类型词 → 完整类型 JSON（标量或命名类型） |
| `declared_result_arm_type_json(words, body_from, arm) returns Text` | `Result[T,E]` 的 `ok` / `error` 分臂类型 JSON |
| `construct_field_count(words, open_idx, close_idx) returns U64` | 构造字段数（决定 payload 占几个 result id） |
| `construct_lowering(...) returns Text` | 构造的指令串（逗号分隔、无外层括号）——从 STEP-0494 的 `record_construct_function_ir` 里**提取**出来复用 |
| `payload_type_json(words, value_first, value_end) returns Text` | payload 类型判定（已声明 record → named；`I64/U64.literal` → i64/u64；字符串 → string） |
| `record_variant_function_ir(...) returns Text` | 变体 helper：变体名限 `ok` / `error`；校验 payload 类型与声明分臂一致，否则 `ERR:E-SH-IR-VARIANT-TYPE` |

### 4.2 **标量 kind 通道与类型 JSON 通道必须隔离**（同 STEP-0493 教训）

`declared_return_kind` 有 19 处调用点，全部把返回值当**裸标量 kind** 用。因此 `Result[T,E]` 的类型 JSON **另开旁路**（`declared_result_arm_type_json`），绝不让 `declared_return_kind` 返回 JSON。

### 4.3 payload 类型**必须校验**（否则产出畸形 IR）

若不校验，`ok(I64.literal(1))` 配 `Result[ScriptOutput, ScriptError]` 会产出 `ty` 声明与 payload 不符的畸形 IR——违反 ADR-0016 决策 6（越界形状 typed fail-closed、禁止静默回退）。故：

- payload 类型 ≠ 声明分臂类型 → `ERR:E-SH-IR-VARIANT-TYPE`；
- payload 形状不被支持 → `ERR:E-SH-IR-VARIANT-PAYLOAD`；
- 函数**没有声明 `Result[...]`** → helper `SKIP` → 落到 `call` 路径 → `ERR:E-SH-IR-CALL-TARGET`（与 oracle 的 `E2031 unresolved call target ok` 是同一裁决）。

### 4.4 接入点位置

`record_variant_function_ir` 放在 `scalar_ir` helper 分派链中 **construct 之后、`direct_bytes_equal_encode_ir` 之前**。形态判定足够严格（唯一一条 `return` + 变体名 `ok`/`error` + 括号 + 声明了 `Result`）不会抢活；好处是"确实是变体但 payload 不被支持"时给出**明确的** typed 拒绝，而不是掉进 `call` 路径报笼统的 `ERR:E-SH-IR-CALL-TARGET`。

## 5. 实现过程中踩到并修正的**两条真缺陷 + 一条新语法纪律**

1. **`type_word_json` 的判定顺序错了（真缺陷）**：首版写成"先看 `record_scalar_kind` 的返回值长度，为 0 就当命名类型"。但 `record_scalar_kind` 对**未知词返回的是 ERR 串**（`ERR:E-SH-IR-RECORD-TYPE`），长度**不是 0**，于是命名类型（`NumericError`）被判成错误 → 所有 `Result[...]` 形状统一报 `ERR:E-SH-IR-RECORD-TYPE`。修正：先看 `record_scalar_kind` 是否以 `ERR:` 开头，是则转命名类型判定（`record_named_type`），否则才是标量。
2. **`return_type` 的两臂被对调（真缺陷）**：首版按"实际构造的分臂"排 `ok`/`error`。oracle 真值证明 `return_type` **恒按声明顺序**（§3.4 第 4 条）。由字节对拍当场抓出。
3. **match 分支绑定与 `let` 单元同名也会报 `cell redeclared`（新语法纪律）**：本片在 `selfhost/compiler.sico` 里写 `case ok(split_tail): set split_tail_len = split_tail`，随后又 `let split_tail = ...` → `unsupported cell redeclared at bytes 36631..36668`。**match 分支的 `case ok(x)` / `case error(x)` 绑定与函数的 `let` 单元共用同一个命名空间**——这补齐了 [handoff-m22.md](../handoff-m22.md) §5「语法与语义纪律」表的第三条（同函数内 `let` 不可重名）的**另一半**。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 变体字节等同（4 个 fixture） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_lowers_result_variant_byte_exactly` | **ok**（与 Rust oracle 逐字节相同） |
| 变体 fail-closed（4 个负例） | `... --test selfhost_compiler sico_compiler_refuses_unresolved_result_variants_with_a_typed_identity` | **ok** |
| IR 双通道分片契约 | `... --test selfhost_compiler sico_compiler_emits_the_module_ir_split_across_both_channels_byte_exactly` | **ok**（拼接后与 oracle 逐字节相同 + 中点断言 + 拒绝不隐藏） |
| Script-profile 前沿重钉 | `... --test selfhost_compiler sico_compiler_lowers_the_script_profile_echo_fixture_byte_exactly` | **ok**（前沿由 typed refusal 移到逐字节降低） |
| 全源自举对拍 | `... --test selfhost_compiler sico_guest_arena_accommodates_full_parser_selfhost_input` | **ok**（31.25s，`--emit-ir-split` 双通道） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **93 + 2 + 9 + 1 = 105 passed / 0 failed** |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | **18/18** 判定一致，`FRONTIER_UNCHANGED` exit 0（`result-variant-return` 与 `script-profile-run` 由 refused → **lowered**；新增 3 个负例） |
| 0245 电池 | `tools/validate-step-0245.ps1` | **exit 0**（含 runner fmt/clippy） |
| 0262 电池 | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`** |
| 规划合同 | `tools/validate-step-0124.ps1` | **`STEP_0124_OK`** |
| canary | `tools/report-m22-canary.ps1` | coverage 100%、`full_source_exit` 0 |
| 根工作区 | `cargo fmt --all -- --check` + `clippy -D warnings` + `cargo test --workspace --all-targets --all-features` | 全绿 |

## 7. 诚实性

- 本片**收窄**了 oracle 的变体语义，必须显式记录：
  - **变体名收窄**：只支持 `Result` 的两个**内建**变体 `ok` / `error`；`Option` 的 `some` **在当前 Sico 语言里不存在**（oracle 直接报 `E2031 unresolved call target some`），故不是缺口。
  - **payload 语法收窄**：只支持**投影**、**`I64/U64.literal`**、**字符串字面量**、**record 构造**四种形态；其余 typed 拒绝 `ERR:E-SH-IR-VARIANT-PAYLOAD`。
  - **分臂类型收窄**：`Result` 的 `ok` / `error` 类型只支持标量或命名类型；`List[...]` / `Map[...]` 类型的分臂不支持（typed 拒绝）。
  - **形态收窄**：只支持"函数体唯一一条 `return`"的变体；带语句体的变体仍 typed 拒绝。
  - **类型校验比 oracle 更严**：oracle 不校验 payload 类型与声明分臂是否一致（会产出 `ty` 与 payload 不符的 IR）；guest 校验，不一致即 typed 拒绝。同为收窄，但**方向是更安全**。
- 本片**没有**触碰 guest IR 摄取、run 体发射、`.sapp` 链或语料阶梯。门 3 仍为 NO-GO。
- 本片**暴露**了 Script v0 单通道装不下全模块 IR 的容量边界，并给出了 `--emit-ir-split` 的绕行；**默认单通道路径在全模块规模上仍不可用**，这一限制已登记（§2.2），没有被掩盖。

## 8. 下一片：guest IR 摄取 + run 体发射

STEP-0488 缺口表的前端三项已全部闭合，缺口表**只剩 IR 摄取与 run 体发射**：

1. **IR 摄取**：guest 需要在**自己进程内**消费 canonical IR（而不是让它往返 Script v0 通道）——§2 的容量边界正是这条路的直接动机。
2. **run 体发射**：把 `run` 函数的 IR（投影 / 构造 / 变体）发射成 Component 的 run 体，接上 STEP-0487 的 `--emit-component-hex` 骨架。
3. 之后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。

> 提醒（承接 [STEP-0491](./STEP-0491-m22-w2-s5-record-field-projection.md) §4、STEP-0494 §5 与 [`../handoff-m22.md`](../handoff-m22.md) §4/§5）：guest 前端的**形状前沿**与**语法/语义纪律**（返回类型必须写 `returns X:`；同函数内 `let` 不可重名；match 分支绑定与 `let` 单元共用命名空间；token 计数边界）共同构成"新代码必须能被 guest 自己降低"的全部已知约束。
