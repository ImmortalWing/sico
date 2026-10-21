# STEP-0492 m22-w2 S5 投影泛化到任意已声明 record（去除 ScriptInput 硬编码）

> - status: implementation complete locally
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md)（投影闭合）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)（缺口定位）
> - gate: 门 3 的 S5 波次第 2 刀 = IR 驱动 run 体 —— **本片闭合投影的"记录名泛化"，不新增 op 种类**
> - numbering: M23 并行会话已占用 STEP-0490/0491 之外的 M23 轨编号；STEP-0491 已被本线占用，本片顺延为 **STEP-0492**（M22 线与 M23 线编号序列共享，已核对 `docs/steps/` 无冲突）。

## 1. 本片目标与结论

**目标**：STEP-0491 落地的字段投影只认**基名形参类型恰为 `ScriptInput`** 的情形（`parameters_ir` 里 `"ScriptInput"` 是硬编码字面量，投影分支里也有同一处硬编码）。Script profile 只需要 `ScriptInput` 这一个实例，但**硬编码记录名是收窄而非能力**：任何已声明 record 的形参都应当能投影自己的已声明字段。

**结论（实测）**：本片去掉两处硬编码，投影对**任意已声明 record** 生效，且与 Rust oracle **逐字节相同**。

- `parameters_ir`：命名形参的类型从固定 `"ScriptInput"` 改为**实际声明的类型名**（检测条件同步从"恰为 ScriptInput"放宽为"是一个命名类型"）。
- `scalar_ir` 投影分支：基名形参的类型判定从 `is_word(..., "ScriptInput")` 放宽为 `record_named_type(...)`；真正的"这个 record 有这个字段、且字段是标量"的判定一直由 `record_field_kind` 承担（找不到 record / 找不到字段 / 字段非标量 → 返回空串 → 不进入投影路径 → typed 拒绝）。

## 2. 地面真值（Rust oracle 逐字节）

fixture（159 B）：

```
record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
end record

function probe(out: ScriptOutput) returns Bytes:
  return out.stdout
end function
```

Rust oracle（`sico_ir::lower_core` + `canonical_json`）：

```json
{"schema":"sico.ir.v0","source_name":"probe.sico","source_len":159,"functions":[{"id":1,"name":"probe","parameters":[{"id":0,"name":"out","ty":{"kind":"named","data":"ScriptOutput"},"range":{"start":92,"end":95}}],"return_type":{"kind":"bytes"},"effects":[],"entry":0,"blocks":[{"id":0,"instructions":[{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdout"}},"range":{"start":138,"end":145}}],"terminator":{"kind":"return","data":1},"range":{"start":77,"end":159}}],"range":{"start":77,"end":159}}],"records":{"ScriptOutput":[{"name":"stdout","ty":{"kind":"bytes"}},{"name":"stderr","ty":{"kind":"bytes"}}]}}
```

与 STEP-0491 的 `ScriptInput` 情形相比，**唯一实质差别** = 形参类型报的是声明的记录名（`"data":"ScriptOutput"`），而不是固定串。

## 3. 实现（`selfhost/parser.sico`）

两处，均为收窄的解除，不改 op 发射与 range 约定：

1. `parameters_ir`（命名形参类型判定 + `"data"` 发射）：
   - 判定：`is_word(parameter_item_type, "ScriptInput")` → `record_named_type(parameter_item_type)`。到达该分支的类型词已经不是标量（`scalar_type_kind` 失败）也不是 `List`/`Map`（前面分支已分流），因此只能是**命名类型**；`Result[...]`/`Option[...]` 形参仍在前面的 `[` 分支被拒（`ERR:E-SH-IR-PARAMETERS`），不受本片影响。
   - 发射：`"data":"ScriptInput"` → `"data":"<parameter_item_type>"`。
2. `scalar_ir` 投影分支：`is_word(proj_record_name, "ScriptInput")` → `record_named_type(proj_record_name)`。

**未改**：op 形状、range 约定（"." + 字段名 token 对）、`record_field_kind` 的字段查找与标量判定、fail-closed 语义（缺 record / 字段不存在 / 字段非标量 → `ERR:E-SH-IR-EXPRESSION`）。

## 4. 连带修正（真实的测试耦合缺陷）

`parameters_ir` 新增了对 `record_named_type` 的调用，而两个**隔离函数组合**测试（`sico_compiler_lowers_gw_find_code_text_byte_exactly`、`sico_compiler_lowers_checked_arithmetic_match_ir_closure_byte_exactly`）把 `parameters_ir` 抽出来单独喂给 Rust oracle——组合里没有 `record_named_type`，于是 **Rust oracle 自己先报 `E2031 unresolved call target record_named_type`**（两条测试红）。

修法：在那两个组合里补上 `record_named_type` 及其依赖 `is_ident_byte`（组合是"逐函数文本 + 其被调函数"的显式清单，新增依赖必须显式登记）。这暴露了一条既有纪律：**guest 前端每个函数对其它函数的依赖都是显式的组合清单，新加依赖必须同步登记**，否则隔离测试会先于功能测试报错。

## 5. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 泛化后投影字节等同 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_lowers_any_declared_record_field_projection_byte_exactly` | **ok**（与 Rust oracle 逐字节相同） |
| fail-closed 钉 | `... --test selfhost_compiler sico_compiler_refuses_unresolved_record_field_projections_with_a_typed_identity` | **ok** |
| 全源自举对拍 | `... --test selfhost_compiler sico_guest_arena_accommodates_full_parser_selfhost_input` | **ok**（`selfhost/parser.sico` 全量降低 exit 0 + 与 Rust oracle 逐字节相同） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **86 + 2 + 9 + 1 = 98 passed / 0 failed** |
| 0245 电池 | `tools/validate-step-0245.ps1` | **exit 0**（含 runner fmt/clippy + 86/86） |
| 0262 电池 | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`** |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | 9/9 判定一致，`FRONTIER_UNCHANGED` exit 0（新增 `record-parameter-projection-generic-record` → lowered） |

## 6. 诚实性

- 本片**没有**新增可降低的 op 种类，也没有触碰 record 构造 / 变体 / IR 摄取 / 发射；门 3 仍为 NO-GO。
- 命名形参类型现在报**声明的记录名**（此前固定报 `ScriptInput`）。这与 Rust oracle 同形，但对"声明了但没被使用的 record"也会报出其名字——这是 oracle 的行为，不是本片的收窄。
- 投影仍只覆盖**标量字段**（`List[...]`/`Map`/命名类型的字段仍 typed 拒绝），且只在**无语句的 straight 形态**上被识别。
- 基名形参的类型若是**泛型**（`Result[...]`）仍被 `parameters_ir` 拒绝；`declared_return_kind` 仍不返回命名类型 → **命名返回类型**（`returns ScriptOutput`）仍不可降低，这是 record 构造与变体两片的前置缺口（见 §7）。

## 7. 下一片的前置缺口（已测得真值，未实现）

**record 构造**（Rust oracle 实测，fixture 344 B）：

```json
{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct","data":{"name":"ScriptOutput","fields":[{"name":"stdout","value":1},{"name":"stderr","value":2},{"name":"exit_code","value":3}]}},"range":{"start":249,"end":330}}
```

实现它需要三件当前**不具备**的能力：

1. **命名返回类型**：`declared_return_kind` 对 `returns ScriptOutput` 返回空串 → `ERR:E-SH-IR-RETURN-TYPE`；需要返回 `{"kind":"named","data":"ScriptOutput"}`，并让 `scalar_ir` 的函数 JSON `return_type` 与 `binding_parameter_kind` 用同形 JSON 发射（`type_json` 已是现成的"kind 或 JSON"发射助手）。
2. **构造实参的表达式降低**：`ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0))` 的每个字段值要先各自降低成一条指令（结果 id 1/2/3），构造 op 再引用这些 id（结果 id 4）——guest 目前只有"return 表达式"一条降低路径，没有可复用的子表达式降低器。
3. **结果 id 记账**：现有 `result_value = parameter_count` 的单表达式记账不适用于多指令序列。

**变体**（`ok(...)`/`some(...)`）在其上再加一层 `variant` op 与 `Result`/`Option` 类型 JSON。

> 提醒（承接 STEP-0491 §4）：guest 前端有**自举降级形状前沿**——新代码必须同时能被 guest 自己降低。构造/变体两片会大量用到"嵌套表达式 + 多指令序列 + 命名类型"，落笔前先查 [`../handoff-m22.md` §4](../handoff-m22.md) 的形状前沿表。
