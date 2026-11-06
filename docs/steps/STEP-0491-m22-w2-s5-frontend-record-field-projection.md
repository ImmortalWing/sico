# STEP-0491 m22-w2 S5 前端 record 字段投影降低（Script-profile 第 1 类形状闭合）

> - status: implementation complete locally
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0486](./STEP-0486-m22-w2-s5-codegen-wave.md)（波次合同）、[STEP-0487](./STEP-0487-m22-w2-s5-codegen-skeleton.md)（骨架刀）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)（缺口定位 + 发射规格）、RFC-0011、ADR-0015/0016
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次第 2 刀 = IR 驱动 run 体 —— **本片只闭合其输入侧的第 1 类形状**
> - numbering: 原计划取 **STEP-0490**；本工作树的并行 M23 会话于 2026-10-21 先占用了 STEP-0490（`docs/steps/STEP-0490-m23-item3-bare-literal-preparation.md`），故本片顺延为 **STEP-0491**（两会话改动面不重叠：M23 触 `docs/ROADMAP.md`、`docs/plans/M23-*`、M23 普查工具与 bare-literal 测量工具，未触 `selfhost/parser.sico` 与 M22 计划/手稿）。

## 1. 本片目标与结论

**目标**：闭合 [STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md) §2 登记的**缺口 1**——guest 前端（`selfhost/parser.sico` 的降低机器）能降低 record 形参的字段投影，并产出与 Rust oracle **逐字节相同**的 canonical IR。

**结论（实测）**：缺口 1 已闭合。`input.stdin`（`input: ScriptInput`，`ScriptInput` 为已声明 record，字段 `stdin: Bytes`）现在降低为 canonical 的 `project` op：

```json
{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":147,"end":153}}
```

与 `sico_ir::lower_core` + `canonical_json` 的 Rust oracle **逐字节相同**（固件钉 `sico_compiler_lowers_script_input_field_projection_byte_exactly`）。缺 record 声明、字段不存在、基名不是 `ScriptInput` 形参、字段非标量四种情形**一律保持 typed 拒绝** `ERR:E-SH-IR-EXPRESSION`（fail-closed，无静默半降低）。

**门 3 仍为 NO-GO**：Script-profile 还需要 record 构造（`ScriptOutput(...)`）与 `ok(...)`/`some(...)` 变体，本片未触及。

## 2. 地面真值（Rust oracle 逐字节）

最小 fixture（167 B）：

```
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

function probe(input: ScriptInput) returns Bytes:
  return input.stdin
end function
```

Rust oracle 的 canonical IR（`sico_ir::lower_core` + `canonical_json`）：

```json
{"schema":"sico.ir.v0","source_name":"probe.sico","source_len":167,"functions":[{"id":1,"name":"probe","parameters":[{"id":0,"name":"input","ty":{"kind":"named","data":"ScriptInput"},"range":{"start":98,"end":103}}],"return_type":{"kind":"bytes"},"effects":[],"entry":0,"blocks":[{"id":0,"instructions":[{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":147,"end":153}}],"terminator":{"kind":"return","data":1},"range":{"start":83,"end":167}}],"range":{"start":83,"end":167}}],"records":{"ScriptInput":[{"name":"arguments","ty":{"kind":"list","data":{"kind":"string"}}},{"name":"stdin","ty":{"kind":"bytes"}}]}}
```

关键事实（决定了实现形状）：

| 事实 | 值 | 来源 |
|---|---|---|
| op | `project`，`data = {base, field}` | `crates/sico-ir/src/lower.rs:3322-3330` |
| `base` | 基表达式的结果 id = 形参 id（0） | 同上（`self.resolve_name` 返回形参 id） |
| `result` | 1 = 形参数（`parameter_count`） | guest 侧 `scalar_ir` 的 `result_value = parameter_count` |
| `ty` | 字段的标量 kind（`Bytes` → `bytes`） | 记录声明 |
| `range` | **"." + 字段名**这一对 token 的字节跨度（147..153） | `token_range(&tokens[index..index + 2])`（`index` = 点号下标） |
| 记录表 | `"records"` 表按 canonical 格式产出（STEP-0488 已支持） | `records_table_json` |

**range 陷阱**：`token_range(&tokens[index..index+2])` 的 `index` 是**点号**的下标，区间从点号开始、到字段名结束。因此 end = 字段名偏移 + 字段名长度（不是点号偏移 + 字段名长度）。首版实现按后者算，产出 end=152，与 oracle 的 153 差 1 字节——由固件字节对拍当场抓出。

## 3. 实现（`selfhost/parser.sico`）

### 3.1 新助手 `record_field_kind(words, record_name, field_name) -> Text`

沿用 `records_table_json` 的同款词法遍历（`record` → 名字 → 逐字段 → `end record`），返回该已声明 record 中某字段的**标量 kind**（`Bytes` → `bytes`）；记录不存在 / 字段不存在 / 字段类型非标量时返回 `""`（不抛错、不猜）。

### 3.2 `scalar_ir` 的 return 表达式分析

在多 token 分支里新增投影识别（3 token 形状 `base . field`）：

1. `binding_parameter_index` 解析基名 → 形参下标；非形参则不进入投影路径。
2. `parameter_type_word` 取该形参的声明类型；仅 `ScriptInput` 进入投影路径（与 guest 既有 `parameters_ir` 对 `ScriptInput` 的硬编码一致）。
3. `record_field_kind` 取字段标量 kind；空串则不进入投影路径。
4. `word_span_offset` 两次求点号偏移与字段名偏移；`match U64.checked_add(字段名偏移, 字段名字节长度)` 求 range 末偏移。
5. 产出 `expression_kind = "project"`、`expression_type = 字段标量 kind`，并把 `project_base` / `project_field` / `project_kind` / `project_range_start` / `project_range_end` 写入函数级局部。

**链旁路**：原 `if is_int_add:` 这个"跳过 call/fixed-op 链"的守卫被泛化为 `skip_expression_chain`（int-add 与投影两种情形都置真），使投影可以旁路链而不必新增嵌套层级。

**指令发射**：在 `expression_kind` 分派链中新增 `project` 分支，产出 `{"result":N,"ty":{"kind":K},"operation":{"op":"project","data":{"base":B,"field":"F"}},"range":{...}}`；`result` 沿用 `parameter_count`，terminator 返回该 id。

## 4. 本片最硬的工程发现：guest **自举降级机器**有形状前沿

`selfhost/parser.sico` 自身被 guest 逐函数降低并与 Rust oracle 逐字节对拍（`sico_guest_arena_accommodates_full_parser_selfhost_input`）。因此**新代码必须同时能被 guest 自己降低**，否则全源对拍从 exit 0 变成 typed 拒绝（`ERR:E-SH-IR-CALL-SHAPE`）。本片用逐项二分实测出三条前沿（每条都是"guest 降不动自己"的形状）：

| # | 形状 | guest 自举结果 | 规避方式 |
|---|---|---|---|
| 1 | 深层嵌套块内的 `match`（while 体 → else → 5 层 if → match） | `ERR:E-SH-IR-CALL-SHAPE` | 把 checked 加法放进**已证形状**的 `match` 主语：`U64.checked_add(<偏移>, sico.bytes.length(sico.text.encode(<名字>)))`——与同函数内既有的 `match U64.checked_add(range_start, sico.bytes.length(sico.text.encode(range_lexeme))):` 同形 |
| 2 | 深层块内 `set` 的右值是**嵌套 intrinsic 调用**（`sico.bytes.length(sico.text.encode(x))`） | `ERR:E-SH-IR-CALL-SHAPE` | 同 #1：让嵌套 intrinsic 只出现在 `match` 主语里，不再单独作为 `set` 右值 |
| 3 | 函数尾部的 `match` 主语操作数是**只在循环体内赋值**的局部 | `ERR:E-SH-IR-CALL-SHAPE` | 不在尾部做这次加法；改在嵌套块内一次算完（#1 的形状） |

实测顺序（每步一次全源 census）：去掉 `match` → 绿；把嵌套 intrinsic 提到浅层 `let` → 绿；把 `match` 提到函数尾 → 红；最终形状（`match` 留在嵌套块、主语带嵌套 intrinsic）→ 绿。

> 结论（M22 资产）：**guest 前端的自举可降级性是"形状前沿"而非"功能开关"**。后续刀（record 构造 / 变体 / IR 摄取）必须沿用已证形状，不能想当然地把 Rust 侧的写法直译过去。`docs/development/script-component-emission-v0.md` 记录的是 Rust 侧发射算法，**不代表** guest 侧能照抄同样的 Sico 写法。

## 5. 本片修掉的两个真缺陷

1. **死循环（fuel 耗尽）**：首版 `record_field_kind` 把 `rfk_type_end` 声明在**内层** `if` 块里，却在**外层**使用；作用域外读取得到 0 → 扫描游标被写回 0 → 内层 `while` 永不推进 → guest 跑满燃料（`FuelExhausted`）。修法：把 `let rfk_type_start` / `let rfk_type_end` 上提到 `if rfk_is_field:` 同层。**教训：Sico 的 `let` 是块作用域，跨块使用必须显式提升声明位置。**
2. **注释语法不存在**：Sico 源码**不支持** `#` 注释（词法错误 `UnexpectedCharacter('#')`），首版插入的 `# STEP-0490: ...` 直接让整棵树编译失败。已删除（`selfhost/*.sico` 全库无注释）。

## 6. 前沿探针推进（钉随推进重钉）

`tools/probe-m22-frontend-frontier.ps1`（exit 0 = 前沿未移动）：

| 形态 | STEP-0488 判定 | STEP-0491 判定 |
|---|---|---|
| `record-parameter-projection`（**带** record 声明） | `refused:ERR:E-SH-IR-EXPRESSION` | **`lowered`** |
| `record-parameter-projection-missing-record`（无 record 声明） | —（新用例） | `refused:ERR:E-SH-IR-EXPRESSION` |
| `record-parameter-projection-unknown-field`（字段不存在） | —（新用例） | `refused:ERR:E-SH-IR-EXPRESSION` |
| `result-variant-return` / `option-variant-return` / `script-profile-run` | `refused:ERR:E-SH-IR-CALL-TARGET` | 不变 |

`script-profile-run` 的拒绝身份**不变**（`ok(...)` 变体仍先撞 `CALL-TARGET`），故 STEP-0488 的固件前沿钉 `sico_compiler_refuses_the_script_profile_echo_fixture_with_a_typed_identity` **无需重钉**。

## 7. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 投影字节等同 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_lowers_script_input_field_projection_byte_exactly` | **ok**（与 Rust oracle 逐字节相同） |
| fail-closed 钉 | `... --test selfhost_compiler sico_compiler_refuses_unresolved_record_field_projections_with_a_typed_identity` | **ok**（3 个拒绝身份稳定） |
| 全源自举对拍 | `... --test selfhost_compiler sico_guest_arena_accommodates_full_parser_selfhost_input` | **ok**（`selfhost/parser.sico` 全量降低 exit 0 + 与 Rust oracle 逐字节相同） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **85 + 2 + 9 + 1 = 97 passed / 0 failed** |
| 0262 电池 | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`**，exit 0 |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | 8/8 判定与记录一致，`FRONTIER_UNCHANGED` exit 0（记录表已随本片前移） |

## 8. 诚实性

- 本片**只**闭合 Script-profile 三类形状中的第 1 类，且只覆盖**标量字段**。仍未支持：record 构造（`ScriptOutput(...)`）、`ok(...)`/`some(...)` 变体、`Result[...]` 返回类型、命名返回类型、非标量字段（`List[Text]`/`Map`）、以及"带语句的函数体"里的投影（投影只在无语句的 straight 形态上被识别）。
- 投影的基名目前只认 `ScriptInput` 形参（与 guest 既有 `parameters_ir` 的硬编码一致）；其它已声明 record 的形参投影仍 typed 拒绝——这是**已知收窄**，不是能力声明。
- 探针的 `lowered` 判定 = guest 成功降低并产出 canonical IR；**逐字节等同**由固件钉单独承担，探针不做字节比较。
- 门 3 仍为 NO-GO。R3（停滞计数）未变。
