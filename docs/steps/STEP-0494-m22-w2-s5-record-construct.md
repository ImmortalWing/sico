# STEP-0494 m22-w2 S5 record 构造 `construct` op

> - status: implementation complete locally
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0493](./STEP-0493-m22-w2-s5-named-record-return-type.md)（命名返回类型管道）、[STEP-0492](./STEP-0492-m22-w2-s5-generic-record-projection.md)（投影泛化）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)（缺口定位 + 发射规格）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次第 4 刀 = IR 驱动 run 体 —— **本片闭合 Script-profile 三类形状中的第 2 类（record 构造）**
> - numbering: M22 线顺延为 **STEP-0494**（本线已占用 STEP-0491/0492/0493；并行 M23 会话占用 STEP-0489/0490；已核对 `docs/steps/` 无冲突）

## 1. 本片目标与结论

**目标**：闭合 [STEP-0493](./STEP-0493-m22-w2-s5-named-record-return-type.md) §8 登记的 record 构造缺口——guest 前端能降低 `ScriptOutput(stdout: ..., ...)`，并产出与 Rust oracle **逐字节相同**的 canonical IR。

**结论（实测）**：record 构造已闭合。

`ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0))` 现在降低为：

```json
{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":275,"end":281}},
{"result":2,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":296,"end":302}},
{"result":3,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":0},"range":{"start":327,"end":328}},
{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct","data":{"name":"ScriptOutput","fields":[{"name":"stdout","value":1},{"name":"stderr","value":2},{"name":"exit_code","value":3}]}},"range":{"start":249,"end":330}}
```

terminator 为 `{"kind":"return","data":4}`（返回 construct 的结果 id）。**与 Rust oracle 逐字节相同**（固件钉 `sico_compiler_lowers_record_construct_byte_exactly`，344 B fixture）。

**门 3 仍为 NO-GO**：`ok(...)` / `some(...)` 变体、guest IR 摄取、run 体发射、37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。

## 2. 地面真值（Rust oracle 逐字节）

fixture（344 B）：

```
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

function probe(input: ScriptInput) returns ScriptOutput:
  return ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0))
end function
```

oracle 的 canonical IR（`sico_ir::lower_core` + `canonical_json`）：

```json
{"schema":"sico.ir.v0","source_name":"echo.sico","source_len":344,"functions":[{"id":1,"name":"probe","parameters":[{"id":0,"name":"input","ty":{"kind":"named","data":"ScriptInput"},"range":{"start":198,"end":203}}],"return_type":{"kind":"named","data":"ScriptOutput"},"effects":[],"entry":0,"blocks":[{"id":0,"instructions":[{"result":1,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":275,"end":281}},{"result":2,"ty":{"kind":"bytes"},"operation":{"op":"project","data":{"base":0,"field":"stdin"}},"range":{"start":296,"end":302}},{"result":3,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":0},"range":{"start":327,"end":328}},{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct","data":{"name":"ScriptOutput","fields":[{"name":"stdout","value":1},{"name":"stderr","value":2},{"name":"exit_code","value":3}]}},"range":{"start":249,"end":330}}],"terminator":{"kind":"return","data":4},"range":{"start":183,"end":344}}],"range":{"start":183,"end":344}}],"records":{"ScriptInput":[{"name":"arguments","ty":{"kind":"list","data":{"kind":"string"}}},{"name":"stdin","ty":{"kind":"bytes"}}],"ScriptOutput":[{"name":"stdout","ty":{"kind":"bytes"}},{"name":"stderr","ty":{"kind":"bytes"}},{"name":"exit_code","ty":{"kind":"i64"}}]}}
```

字节偏移（脚本核对，非手算）：

| 项 | 偏移 | 含义 |
|---|---|---|
| `construct` range | **249..330** | 构造名标识符起点 .. 右括号之后（构造表达式的 token 跨度） |
| `project` range ×2 | **275..281** / **296..302** | `"." + 字段名` 这一对 token（与 STEP-0491 同一约定） |
| `const_i64` range | **327..328** | 字面量 `0` 的 token |

oracle 侧的发射规格（`crates/sico-ir/src/lower.rs:3822-3848`）：`definitions.constructors` 命中后，**逐个字段先 `self.expression(...)` 递归降低子表达式**（子指令先入列），再发射 `Operation::Construct { name, fields }`；字段名取自 `name: value` 形态的具名实参，缺失时退化为 `#index`。

## 3. 关键设计决策：走 **helper 家族**，不改 `scalar_ir` 的深层 return 分析

STEP-0492 §7 登记的剩余两项缺口是「构造实参的子表达式降低」与「多指令序列的结果 id 记账」。两者都要求**把 `scalar_ir` 里那条内联的 return 表达式分析链拆成可复用的子表达式降低器**——那是 `scalar_ir` 最深处的一段巨型 if/else 链，且落在 guest 自举降级的**形状前沿**上（见 [`../handoff-m22.md` §4](../handoff-m22.md)）。

本片改用另一条路：**新增独立 helper `record_construct_function_ir`，返回完整函数 JSON**，与既有 `straight_cell_function_ir` / `straight_environment_function_ir` / `control_if_function_ir` 同构。helper 是**顶层函数**（浅嵌套），不加深 `scalar_ir` 的嵌套层级；`scalar_ir` 侧只多一个调用点。

| 新增函数 | 职责 | 位置 |
|---|---|---|
| `construct_field_comma(words, from_idx, end_idx) returns U64` | 找构造实参区的**顶层逗号**（同时追踪 `(` 与 `[` 深度；`record_map_comma` 只追踪 `[`，不能直接复用） | 顶层 |
| `construct_field_value_ir(...) returns Text` | 把**一个字段值**降低成一条指令（result id 由调用方分配），返回该指令的 JSON | 顶层 |
| `record_construct_function_ir(...) returns Text` | 识别"函数体唯一语句是 `return <构造调用>`"形态，返回完整函数 JSON；不匹配返回 `SKIP` | 顶层 |

**结果 id 记账**因此是**显式**的：`rcf_next_result` 从 `parameter_count` 起，每个字段值消耗一个 id，construct 消耗最后一个 id，terminator `return` 引用它。这与 `straight_cell_function_ir` 的 `scc_next_result` 范式一致。

**形态闸门**：helper 要求函数体里**恰好一条** `return` 语句（`rcf_return_count == 1`）；带 `let` / `set` / `match` / `if` / `while` 的形态不匹配 → `SKIP` → 交回既有 helper 家族处理。

## 4. 实现（`selfhost/parser.sico`）

### 4.1 构造名与返回类型的一致性

helper 要求 `declared_return_named_type(words, body_from)`（STEP-0493 新增）返回的记录名与构造名**逐字相同**，否则 `ERR:E-SH-IR-CONSTRUCT-TYPE`；返回类型不是命名类型 → `SKIP`（不是错误——标量返回类型的函数不是构造）。

### 4.2 字段名与字段值

- 字段名必须存在于该 record 且是**标量字段**（`record_field_kind` 判定），否则 `ERR:E-SH-IR-CONSTRUCT-FIELD`。
- 字段值目前只支持两种形态：
  - **投影** `base.field`（base 必须是形参，field 必须是该 record 的标量字段）→ `project` op；
  - **常量** `I64.literal(n)` / `U64.literal(n)`（`n` 必须是规范十进制）→ `const_i64` / `const_u64` op。
  其余形态 → `ERR:E-SH-IR-CONSTRUCT-VALUE`。

### 4.3 `scalar_ir` 侧的接入点

在 `scalar_ir` 的 helper 分派链**最前**（`direct_bytes_equal_encode_ir` 之前）接入：

```sico
let record_construct_json = record_construct_function_ir(words, src, name_idx, body_from, body_end_word, fn_start, function_ordinal, parameter_count)
if is_word(record_construct_json, "SKIP"):
  let no_record_construct = I64.literal(1)
else:
  if sico.text.starts_with(record_construct_json, "ERR:"):
    return record_construct_json
  end if
  ...
end if
```

放在最前，是因为 helper 的形态判定足够严格（唯一一条 `return` + 构造名 + 括号），不会抢走其它 helper 的活；放在最前则保证"确实是构造调用但字段不被支持"时给出**明确的** typed 拒绝，而不是掉进 `call` 路径后报 `ERR:E-SH-IR-CALL-TARGET`。

**`script-profile-run` 的拒绝身份不变**：Script-profile 的 `run` 外层是 `ok(...)`，`ok` 不是 record 名 → helper `SKIP` → 仍报 `ERR:E-SH-IR-CALL-TARGET` ✓（STEP-0488 的固件钉与前沿探针无需重钉）。

## 5. 实现过程中踩到的三条真约束（本片的隐藏成本）

1. **`->` 不是合法的返回类型写法**。`selfhost/parser.sico` 全部 154 个函数用 `returns X:`；首版新增的三个函数误写成 `-> X:`。后果**不是语法错误**，而是**语义模型静默丢函数**：`parse_function`（`crates/sico-semantics/src/lib.rs:830`）用 `header.iter().position(|t| t.kind == TokenKind::Returns)?` 定位返回类型，定位失败即返回 `None`，该函数**不进 `model.functions`**，于是**每个调用点**报 `E2031 unresolved call target`；lexer 把 `->` 当普通标点，所以除了 unresolved 之外**没有任何其它诊断**。诊断路径：`sico outline` 会列出该函数（HIR 收得到），但语义模型没有它。
2. **同一函数内 `let` 不可重名**（即使在互斥分支里）。首版在投影分支与字面量分支各 `let cfv_json`，编译报 `unsupported cell redeclared`；改为顶层声明 + 分支内 `set`。
3. **token 计数边界**：判断"表达式恰好 N 个 token"时，**最后一个 token 的后继索引**才等于段尾。首版把 3-token 投影的边界写成 `third == value_end`，导致投影永远落空（全部 `ERR:E-SH-IR-CONSTRUCT-VALUE`）。

另有一条**观测**：拒绝身份取决于**第一个**失败点——当 `record ScriptOutput` 已声明但 `record ScriptInput` 未声明时，构造会先在**字段值**处 fail-closed（`CONSTRUCT-VALUE`），而不是在字段名处（`CONSTRUCT-FIELD`）。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| construct 字节等同 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_lowers_record_construct_byte_exactly` | **ok**（与 Rust oracle 逐字节相同） |
| fail-closed 钉（3 个负例） | `... --test selfhost_compiler sico_compiler_refuses_unresolved_record_constructs_with_a_typed_identity` | **ok** |
| 全源自举对拍 | `... --test selfhost_compiler sico_guest_arena_accommodates_full_parser_selfhost_input` | **ok**（`selfhost/parser.sico` 全量降低 exit 0 + 与 Rust oracle 逐字节相同） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **90 + 2 + 9 + 1 = 102 passed / 0 failed** |
| 0245 电池 | `tools/validate-step-0245.ps1` | **exit 0**（含 runner fmt/clippy） |
| 0262 电池 | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`** |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | **15/15** 判定一致，`FRONTIER_UNCHANGED` exit 0（新增 `record-construct` → lowered、`record-construct-unknown-field` → refused、`record-construct-unsupported-value` → refused） |
| canary | `tools/report-m22-canary.ps1` | coverage 100%、`full_source_exit` 0 |
| 规划合同 | `tools/validate-step-0124.ps1` | `STEP_0124_OK` |
| 根工作区 | `cargo fmt --all -- --check` + `clippy -D warnings` + `cargo test --workspace --all-targets --all-features` | 全绿 |

## 7. 诚实性

- 本片**收窄**了 oracle 的构造语义，必须显式记录：
  - **字段值语法收窄**：oracle 用通用的 `self.expression(...)` 递归降低字段值（任意表达式）；guest 目前只支持**投影**与**`I64/U64.literal`**两种，其余 typed 拒绝 `ERR:E-SH-IR-CONSTRUCT-VALUE`。
  - **字段名校验收窄**：oracle 的 lower 阶段**不校验字段名是否存在**（`expected_field` 查不到就是 `None`）；guest 要求字段存在于该 record 且为标量字段，否则 `ERR:E-SH-IR-CONSTRUCT-FIELD`。
  - **形态收窄**：只支持"函数体唯一一条 `return`"的构造；带语句体的构造仍 typed 拒绝。
- 本片**没有**触碰变体（`ok(...)` / `some(...)`）、guest IR 摄取、run 体发射或语料阶梯。门 3 仍为 NO-GO。

## 8. 下一片：变体 `ok(...)` / `error(...)` —— 已测真值与剩余缺口

### 8.1 oracle 真值（Rust oracle 实测）

**`ok(...)` 包标量**（fixture 137 B，`enum NumericError: case Invalid end enum` + `function probe() returns Result[I64, NumericError]: return ok(I64.literal(1))`）：

```json
"return_type":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"NumericError"}}}
{"result":0,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":1},"range":{"start":120,"end":121}},
{"result":1,"ty":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"NumericError"}}},"operation":{"op":"variant","data":{"name":"ok","payload":[0]}},"range":{"start":105,"end":123}}
```

terminator `{"kind":"return","data":1}`。

**`error(...)` 包 record 构造**（fixture 202 B）：

```json
"return_type":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"ScriptError"}}}
{"result":0,"ty":{"kind":"i64"},"operation":{"op":"const_i64","data":2},"range":{"start":168,"end":169}},
{"result":1,"ty":{"kind":"string"},"operation":{"op":"const_string","data":"bad"},"range":{"start":181,"end":186}},
{"result":2,"ty":{"kind":"named","data":"ScriptError"},"operation":{"op":"construct","data":{"name":"ScriptError","fields":[{"name":"code","value":0},{"name":"message","value":1}]}},"range":{"start":138,"end":187}},
{"result":3,"ty":{"kind":"result","data":{"ok":{"kind":"i64"},"error":{"kind":"named","data":"ScriptError"}}},"operation":{"op":"variant","data":{"name":"error","payload":[2]}},"range":{"start":132,"end":188}}
```

**Script-profile `run`**（fixture 503 B）：

```json
"return_type":{"kind":"result","data":{"ok":{"kind":"named","data":"ScriptOutput"},"error":{"kind":"named","data":"ScriptError"}}}
{"result":1,...,"operation":{"op":"project",...}},
{"result":2,...,"operation":{"op":"project",...}},
{"result":3,...,"operation":{"op":"const_i64","data":0},...},
{"result":4,"ty":{"kind":"named","data":"ScriptOutput"},"operation":{"op":"construct",...},"range":{"start":407,"end":488}},
{"result":5,"ty":{"kind":"result",...},"operation":{"op":"variant","data":{"name":"ok","payload":[4]}},"range":{"start":404,"end":489}}
```

terminator `{"kind":"return","data":5}`。

### 8.2 从真值读出的六条规格

1. `Result[T, E]` 的类型 JSON = `{"kind":"result","data":{"ok":<T>,"error":<E>}}`（`T`/`E` 各自是完整类型 JSON）。
2. `variant` op = `{"op":"variant","data":{"name":"ok"|"error","payload":[<ids>]}}`；**payload 是 id 列表**。
3. `variant` 的 `ty` 是**整个 Result 类型**，不是 payload 类型。
4. **嵌套顺序**：先降 payload（`ok(ScriptOutput(...))` 里 construct 先入列，result 4），再降 variant（result 5）—— 与本片 construct helper 的"字段值先降、op 后降"完全同构，**可复用同一套记账**。
5. **result id 起点 = `parameter_count`**：`ok(I64.literal(1))` 无形参 → 第一个指令 result **0**；Script-profile 有一个形参 → 第一个指令 result **1** ✓ 与本片 `rcf_next_result = parameter_count` 一致。
6. `variant` 的 range = `ok(...)` / `error(...)` 的整个表达式跨度（105..123 / 132..188 / 404..489），与 construct 的"构造名..右括号后"同族规则。

### 8.3 剩余缺口

1. **`Result[T, E]` / `Option[T]` 的类型 JSON 通道**：`declared_return_kind` 目前只认标量与 list/map；`Result[I64, NumericError]` 需要新的 `{"kind":"result",...}` 形态，且**必须与标量 kind 通道隔离**（同 STEP-0493 的教训）。
2. **变体名到已声明 enum 变体的解析**：`ok` / `error` 是 `Result` 的**内建**变体（不需要 enum 声明）；`NumericError` 是错误类型（命名类型），**不是** enum 的 case 列表 —— 所以这一步只要求识别 `ok` / `error` 两个内建名，且 `error` 的 payload 必须是错误类型。
3. **payload 的单指令降低**：payload 可以是标量常量、record 构造（嵌套 construct）、`const_string` 等 —— `construct_field_value_ir` 当前只支持投影与 `I64/U64.literal`，需扩展到 `const_string` 与"嵌套构造"两种新形态。

### 8.4 一条重要的事实修正：`some(...)` **不是**缺口

实测 Rust oracle 对 `some(...)` 直接报 **`E2031 unresolved call target some`**（语义期拒绝，不是 lower 期拒绝）——即 **`Option` 的 `some` 变体在当前 Sico 语言里根本不存在**。

因此 [STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md) 缺口表里"`Option[I64]` 返回 + `some(...)`"一行**不是缺口**：guest 的 `ERR:E-SH-IR-CALL-TARGET` 与 oracle 的 `E2031` 是**同一裁决**（都被拒），只是诊断通道不同（guest 是 lower 期 typed 拒绝，oracle 是语义期 typed 拒绝）。**缺口 2 的实际内容只有 `ok(...)` / `error(...)` 两个 `Result` 变体。**

> 提醒（承接 [STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md) §4、STEP-0494 §5 与 [`../handoff-m22.md` §4/§5](../handoff-m22.md)）：guest 前端的**形状前沿**与**语法/语义纪律**（返回类型必须写 `returns X:`；同函数内 `let` 不可重名；token 计数边界）共同构成"新代码必须能被 guest 自己降低"的全部已知约束。
