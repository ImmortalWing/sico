# STEP-0493 m22-w2 S5 命名返回类型管道（named return type）

> - status: implementation complete locally
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0492](./STEP-0492-m22-w2-s5-generic-record-projection.md)（投影泛化）、[STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md)（投影闭合）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)（缺口定位）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次第 3 刀 = IR 驱动 run 体 —— **本片闭合 record 构造 / 变体的前置缺口 1（命名返回类型）**
> - numbering: M22 线顺延为 **STEP-0493**（并行 M23 会话占用 STEP-0489/0490，本线已占用 STEP-0491/0492；已核对 `docs/steps/` 无冲突）

## 1. 本片目标与结论

**目标**：闭合 [STEP-0492](./STEP-0492-m22-w2-s5-generic-record-projection.md) §7 登记的**前置缺口 1**——`declared_return_kind` 对 `returns <NamedRecord>` 返回空串，命名返回类型不可降低。

**结论（实测）**：缺口 1 已闭合，且**顺带修掉一个真实的 fail-closed 违规**。

- 命名返回类型现在走**类型 JSON 通道**：函数 JSON 的 `return_type` 发射 `{"kind":"named","data":"ScriptOutput"}`，与 Rust oracle **逐字节相同**（固件钉 `sico_compiler_lowers_named_record_return_type_pass_through_byte_exactly`）。
- 改动前，"命名返回类型 + 语句体"的函数会让 guest 输出**畸形 IR**（`"return_type":{"kind":""}`）——不是 typed 拒绝。现在一律 typed 拒绝 `ERR:E-SH-IR-RETURN-TYPE`。
- 未声明的 record 仍**不**被当成命名类型（`ERR:E-SH-IR-PARAMETER-TYPE` / `ERR:E-SH-IR-RETURN-TYPE`），命名返回类型也**不**削弱返回/形参类型检查（标量返回仍报 `ERR:E-SH-IR-TYPE-MISMATCH`）。

**门 3 仍为 NO-GO**：record 构造（`construct` op）、`ok(...)`/`some(...)` 变体、guest IR 摄取、run 体发射、语料阶梯与 `.sapp` 链全部未做。

## 2. 地面真值（Rust oracle 逐字节）

fixture（158 B）：

```
record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
end record

function probe(out: ScriptOutput) returns ScriptOutput:
  return out
end function
```

Rust oracle（`sico_ir::lower_core` + `canonical_json`）：

```json
{"schema":"sico.ir.v0","source_name":"echo.sico","source_len":158,"functions":[{"id":1,"name":"probe","parameters":[{"id":0,"name":"out","ty":{"kind":"named","data":"ScriptOutput"},"range":{"start":92,"end":95}}],"return_type":{"kind":"named","data":"ScriptOutput"},"effects":[],"entry":0,"blocks":[{"id":0,"instructions":[],"terminator":{"kind":"return","data":0},"range":{"start":77,"end":158}}],"range":{"start":77,"end":158}}],"records":{"ScriptOutput":[{"name":"stdout","ty":{"kind":"bytes"}},{"name":"stderr","ty":{"kind":"bytes"}}]}}
```

关键事实：

| 事实 | 值 |
|---|---|
| `return_type` | `{"kind":"named","data":"ScriptOutput"}`（类型 JSON，不是裸 kind 串） |
| 形参 `ty` | `{"kind":"named","data":"ScriptOutput"}`（STEP-0492 已落地） |
| `instructions` | **空数组**（透传不产生指令） |
| `terminator` | `{"kind":"return","data":0}` —— 返回的是**形参 id**，不是新结果 id |
| `records` 表 | 记录声明照常进入表 |

## 3. 关键设计决策：命名类型走"类型 JSON 通道"，**不动**标量 kind 通道

`declared_return_kind` 在 `selfhost/parser.sico` 里有 **19 处调用点**，全部把返回值当作**裸标量 kind** 使用，典型形态：

- `is_word(kind, "u64")` / `is_word(kind, "bool")` —— 类型判定；
- `"ty":{"kind":"` + kind + `"}"` —— 直接拼进 IR 字段；
- `sico.text.length(kind) == 0` —— "能否解析该类型"的门闸。

若让 `declared_return_kind` 对命名类型返回 `{"kind":"named",...}`，这些点会把 JSON 片段**拼进 IR 字段**（畸形 IR），或把"不可解析的类型"误判成"已知类型"。这是 STEP-0492 §7 记为"blast radius 大"的真实原因。

因此本片的形状是**旁路 + 闸门**，标量通道语义**一字未改**：

| 新增函数 | 职责 | 依赖 |
|---|---|---|
| `record_declared(words, record_name) returns Bool` | 该 record 是否**已声明**（行首 `record` + 名字） | `word_at` / `is_word` / `next_index` |
| `declared_return_named_type(words, body_from) returns Text` | 签名里的 `returns <Name>` 是否是**已声明的** record；是则返回其名，否则 `""` | `record_declared` / `record_named_type` / `line_end` |
| `binding_parameter_named_type(words, function_name_idx, binding) returns Text` | 形参 `binding` 的类型是否是**已声明的** record | `record_declared` / `record_named_type` |
| `named_return_shape_gate(words, body_from, let_count, set_count, match_count, if_count) returns Text` | 形态闸门：命名返回类型但函数体带语句 → `ERR:E-SH-IR-RETURN-TYPE`；否则返回记录名或 `""` | `declared_return_named_type` / `count_until_end_function` |

命名类型的类型 JSON 只在**两个**地方发射：函数 `return_type` 字段、binding 的 `expression_type`。19 处标量通道调用点**零改动**。

> 拓扑顺序：新函数全部插在调用者之前（`record_declared` ≈ 350 行、`declared_return_named_type` ≈ 1350 行、`binding_parameter_named_type` ≈ 1570 行、`named_return_shape_gate` ≈ 1360 行，均早于 `scalar_ir` ≈ 18800 行）。`scalar_ir` **不在**任何隔离函数组合清单里，因此新增依赖不需要登记组合（STEP-0492 §4 的 `E2031` 陷阱不适用）；`parameters_ir` / `binding_parameter_kind` 本片**未改**，其组合清单也无需动。

## 4. 实现（`selfhost/parser.sico`，4 处改动）

### 4.1 闸门（`scalar_ir`，helper 调用之前）

```sico
let named_return_kind = named_return_shape_gate(words, body_from, let_count, set_count, match_count, if_count)
if sico.text.starts_with(named_return_kind, "ERR:"):
  return named_return_kind
end if
```

位置在 `let_count/set_count/match_count/if_count` 统计之后、**第一个 helper 调用**（`direct_bytes_equal_encode_ir`）之前。闸门必须在这里：helper 家族（`general_while_function_ir` / `straight_environment_function_ir` / `straight_cell_function_ir` / `checked_arithmetic_*` / `control_*`）各自按**标量 kind** 拼 `return_type`，收到空 kind 就会产出畸形 IR。

### 4.2 binding 表达式的类型（`scalar_ir` 的 return 表达式分析）

`binding_parameter_kind` 对命名类型返回 `ERR:E-SH-IR-PARAMETER-TYPE`。改为：先试标量 kind，失败再试 `binding_parameter_named_type`，命中则把 `expression_type` 设为 `{"kind":"named","data":"<Name>"}`；两者皆失败 → 保持原 typed 拒绝。

### 4.3 返回类型（`scalar_ir`）

`declared_return_kind` 为空时，若闸门已确认这是命名返回类型，则把 `return_type_kind` 置为 `{"kind":"named","data":"<Name>"}`；否则保持 `ERR:E-SH-IR-RETURN-TYPE`。随后的 `is_word(return_type_kind, expression_type)` 类型比对因此能正确判等。

### 4.4 `return_type` 发射（`scalar_ir`）

原：`",\"return_type\":{\"kind\":\"" + return_type_kind + "\"}"`（把 kind 串硬塞进 `kind` 字段）。
改为：先按 `starts_with(return_type_kind, "{")` 判断——是 JSON 就直接用，否则包一层 `{"kind":"<kind>"}`。**顺带修正**了 list/map 返回类型在标量路径上会拼出畸形 `{"kind":"{...}"}` 的老问题（该路径此前不可达，故无行为回归）。

## 5. 修掉的真实缺陷（畸形 IR）

STEP-0493 之前对命名返回类型的实测（CLI 与固件两条通道一致）：

| 形态 | 改动前 | 改动后 |
|---|---|---|
| 直返同名形参（record 已声明） | `ERR:E-SH-IR-PARAMETER-TYPE`（先撞形参类型） | **正常降低，字节等同 oracle** |
| 命名返回类型 + `let` | `ERR:E-SH-IR-TYPE-MISMATCH`（空 kind 与标量比对） | `ERR:E-SH-IR-RETURN-TYPE` |
| 命名返回类型 + `while` | **畸形 IR**：`"return_type":{"kind":""}` | `ERR:E-SH-IR-RETURN-TYPE` |
| 未声明 record + 直返 | `ERR:E-SH-IR-PARAMETER-TYPE` | 不变 |
| 未声明 record + 标量返回 | `ERR:E-SH-IR-RETURN-TYPE` | 不变 |

畸形 IR 那一条是 ADR-0016 决策 6（越界形状 typed fail-closed、禁止静默回退）的真实违规：guest 交出了一个**不可被 verifier 接受**的 IR，而不是一个带类型的拒绝。闸门把它收敛成 `ERR:E-SH-IR-RETURN-TYPE`。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 命名返回类型字节等同 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_lowers_named_record_return_type_pass_through_byte_exactly` | **ok**（与 Rust oracle 逐字节相同） |
| fail-closed 钉（5 个负例 + 类型检查） | `... --test selfhost_compiler sico_compiler_refuses_named_record_return_types_with_a_typed_identity` | **ok** |
| 全源自举对拍 | `... --test selfhost_compiler sico_guest_arena_accommodates_full_parser_selfhost_input` | **ok**（28.27s；`selfhost/parser.sico` 全量降低 exit 0 + 与 Rust oracle 逐字节相同） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **88 + 2 + 9 + 1 = 100 passed / 0 failed** |
| 0245 电池 | `tools/validate-step-0245.ps1` | **exit 0**（含 runner fmt/clippy + 88/88） |
| 0262 电池 | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`** |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | **12/12** 判定一致，`FRONTIER_UNCHANGED` exit 0（新增 `named-record-return-type` → lowered、`named-record-return-type-undeclared` → refused、`named-record-return-type-statements` → refused） |
| canary | `tools/report-m22-canary.ps1` | coverage 100%、`full_source_exit` 0 |
| 规划合同 | `tools/validate-step-0124.ps1` | `STEP_0124_OK` |

## 7. 诚实性

- 本片**只**支持"命名返回类型 + 直返同名形参"这一种形态。带 `let` / `set` / `match` / `if` / `while` 的命名返回类型函数一律 typed 拒绝 `ERR:E-SH-IR-RETURN-TYPE`。
- **这是收窄，不是能力对齐**：Rust oracle **能**降低其中一部分（例如 `while` 形态，oracle 产出完整的多块 IR，含 `locals` 与 `branch` 终结符）。guest 目前没有多指令序列的结果 id 记账，诚实选择 typed 拒绝。
- 未声明的 record 不被当成命名类型：guest 拒绝（`ERR:E-SH-IR-PARAMETER-TYPE` / `ERR:E-SH-IR-RETURN-TYPE`），而 **Rust oracle 接受**未声明的 record 形参类型（产出 `{"kind":"named","data":"ScriptOutput"}` 且 `records` 表为空）。这同样是 guest 的 fail-closed 收窄。
- 命名返回类型**不**削弱类型检查：`returns ScriptOutput` 但返回标量仍报 `ERR:E-SH-IR-TYPE-MISMATCH`。
- 本片**没有**新增可降低的 op 种类，也**没有**触碰 record 构造 / 变体 / IR 摄取 / 发射。门 3 仍为 NO-GO。

## 8. 下一片：record 构造 `construct` op —— 已测真值与剩余缺口

**Rust oracle 真值**（fixture 344 B）：

```json
{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":275,"end":281}},
{"result":2,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":296,"end":302}},
{"result":3,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":0},"range":{"start":327,"end":328}},
{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct","data":{"name":"ScriptOutput","fields":[{"name":"stdout","value":1},{"name":"stderr","value":2},{"name":"exit_code","value":3}]}},"range":{"start":249,"end":330}}
```

terminator 为 `{"kind":"return","data":4}`（返回 construct 的结果 id）。实测字节偏移（脚本核对，非手算）：

- `construct` range = **249..330** = `ScriptOutput` 标识符起点 .. 右括号之后（构造表达式的 token 跨度，`token_range(tokens)`）；
- `project` range = **275..281** / **296..302** = `"." + 字段名` 这一对 token（与 STEP-0491 同一约定）；
- `const_i64` range = **327..328** = 字面量 `0`。

oracle 侧的发射规格（`crates/sico-ir/src/lower.rs:3822-3848`）：`definitions.constructors` 命中后，逐个字段先 `self.expression(...)` **递归降低子表达式**（子指令先入列），再发射 `Operation::Construct { name, fields }`；字段名取自 `name: value` 形态的具名实参，缺失时退化为 `#index`。

实现它还需要两件当前**不具备**的能力（STEP-0492 §7 的缺口 2、3 仍未做）：

1. **构造实参的子表达式降低**：每个字段值要先各自降低成一条指令（结果 id 1/2/3），construct 再引用这些 id（结果 id 4）——guest 目前只有"return 表达式"一条降低路径，没有可复用的子表达式降低器。
2. **多指令序列的结果 id 记账**：现有 `result_value = parameter_count` 的单表达式记账不适用于多指令序列。

**变体**（`ok(...)` / `some(...)`）在其上再加一层 `Operation::Variant { name, payload }`（`lower.rs:3850-3864`）与 `Result` / `Option` 的类型 JSON。

> 提醒（承接 [STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md) §4）：guest 前端有**自举降级形状前沿**——新代码必须同时能被 guest 自己降低。构造/变体两片会大量用到"嵌套表达式 + 多指令序列 + 命名类型"，落笔前先查 [`../handoff-m22.md` §4](../handoff-m22.md) 的形状前沿表；本片新增的 4 个函数刻意放在顶层（浅嵌套），闸门也刻意做成独立函数，就是为了不加深 `scalar_ir` 的嵌套层级。
