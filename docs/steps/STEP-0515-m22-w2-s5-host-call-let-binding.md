# STEP-0515 m22-w2 S5 宿主调用（**前端刀 · let 绑定位置**）：`let n = sico.bytes.length(data)` ⇒ canonical `intrinsic` IR **逐字节等同** + 整组件 **1025 B** 逐字节等同

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0512](./STEP-0512-m22-w2-s5-host-call-frontend.md)（前端刀：return 位置）、[STEP-0513](./STEP-0513-m22-w2-s5-host-call-component.md)（codegen 刀：`intrinsic` 算子 ⇒ 组件层）、[STEP-0511](./STEP-0511-m22-w2-s5-host-call-truth.md)（真值测量片：let 绑定位置的 IR 与组件真值）
> - gate: 门 3 的 S5 波次 —— **宿主调用线的第二个位置（let 绑定）**；return 位置已由 STEP-0512/0513 闭合
> - numbering: **STEP-0515**。`docs/steps/STEP-0514-m22-w2-s5-s6-closure-ladder.md`（**已提交的规划片**）已占用 STEP-0514，并在 §7 写明「**STEP-0514 已被本规划片占用 ⇒ 执行线下一片从 STEP-0515 起**」⇒ 本片按该规则顺取 0515（取号前已核对 `docs/steps/` 与 `docs/handoff-m22.md`）。**注意**：`docs/handoff-m22.md` 里上一片写的「下一刀 = STEP-0514」与该规划片冲突，已按证据改正（保留原句以便追溯，见 §4.6）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ STEP-0511 §1（let 绑定形状：IR **2215 B**、整组件 **1025 B**，过真实 `wasmparser::Validator`） |
| 有**有界出口测试** | ✅ §3（3 条新钉 + STEP-0512 邻接形状钉的更新 + 前沿探针新增 1 条用例） |
| 上游是否已就绪 | ✅ STEP-0512（IR 层逐字节等同）+ STEP-0513（codegen 认 `intrinsic` 算子 ⇒ 组件层就绪） |

⇒ 取号合法。**实现已完成**，见 §4。

## 1. 真值（backend **生产输出**，逐字节读出）

真值工件：`target/tmp/step0511/host-c-rust-ir.json.bin`（canonical IR，**2215 B**）、`target/tmp/step0511/host-c-oracle-component.hex.bin`（整组件 **1025 B**）。

Rust 前端把**直线型 `let` 绑定直接传播进 SSA 值**（`crates/sico-ir/tests/core_lowering.rs::straight_line_let_bindings_lower_to_ssa_values` 同款规则）⇒ let 绑定位置的 canonical IR 与 return 位置**同形**：

| 项 | return 位置（host-a） | **let 绑定位置（host-c）** |
|---|---|---|
| 指令 | 一条 `intrinsic`（`sico.bytes.length`，`arguments:[0]`，`ty` = `u64`） | **同一条** |
| 结果 id | 1 | **1** |
| terminator | `{"kind":"return","data":1}` | **同一条** |
| local | **无**（没有 `write_local`，没有 `locals`） | **无** |
| 指令 range | 368..391（**调用表达式的文本跨度**） | **369..392**（同样只是**平移**） |
| 函数 range | 319..405 | 319..417 |
| canonical IR 长度 | **2215 B** | **2215 B** |
| 整组件 | **1025 B** | **1025 B** |

⇒ **本片不需要新的 local 机器**：缺口只是「函数体第一条语句是 `let` 时，前端要认得同一台机器，并取**实参调用**的区间」。

## 2. 本片改了什么

`selfhost/parser.sico`：

| 改动 | 规模 |
|---|---|
| 新增**专用顶层路由** `direct_host_call_let_ir` | **+230 行** |
| `scalar_ir` 路由梯接线（放在 `direct_host_call_return_ir` 之后、`fixed_guard_chain_ir` 之前） | **+24 行** |
| 嵌套宿主调用拆成临时变量（见 §4.5） | **+2 行** |

合计 **+256 行 / −0 行** ⇒ **纯插入、零既有行重排**（与 `pre0514` 备份逐行比对：`added 256 / removed 0`）。

### 2.1 路由认领的形状（**恰好一个**）

```
function size(data: Bytes) returns U64:
  let n = sico.bytes.length(data)
  return n
end function
```

闸门（全部 fail-closed，任一不满足 ⇒ `SKIP` ⇒ 交给既有路由，**既有拒绝身份一个不变**）：

| # | 闸门 |
|---|---|
| 1 | 形参恰好 **1** 个 |
| 2 | 声明返回类型是 **`u64`** |
| 3 | 函数体第一条语句是 **`let`** |
| 4 | `let` 与 `=` 之间是**一个绑定名**；`=` 之后**同一行**内是接收者 |
| 5 | 第二条语句是 **`return`**，且它的操作数**就是那个绑定名**、且**独占该行** |
| 6 | 第三条语句是 **`end function`**，且函数在此结束 |
| 7 | 接收者路径是 **`sico.bytes.length`** |
| 8 | 实参**恰好一个**，且它是**那个 `Bytes` 形参**（`binding_parameter_kind` = `bytes`） |
| 9 | 字节级：`let` 关键字之后的三个字节**确实拼作 `let`**（锚点校验） |
| 10 | 字节级：绑定名与接收者表达式之间**确实存在 `=` 字节**（61） |

### 2.2 区间怎么取（**只用既有机器**）

- 指令区间 = **实参调用的文本跨度**：`[接收者起点, 行末]`。接收者起点由 `word_span_offset`（既有机器）从 `let` 关键字锚点走到接收者词；行末由既有机器 `return_expression_end` 给出。
- 函数区间 = `fn_start .. ('end function' 的偏移 + 13)`（与既有路由同款）。

### 2.3 为什么 `=` 要做**字节级**检查（guest 词模型的事实）

guest 的词法器 `is_punct_byte`（`selfhost/parser.sico`）覆盖 `+ - . ( ) , : [ ]`，**不含 `=`（61）** ⇒ **词表里根本没有 `=`**。这正是既有机器 `fixed_guard_chain_ir` 从绑定名**直接**跳到接收者表达式的原因（它也不检查 `=`）。本片因此把「`let` 后有 `=`」做成**字节级**闸门（§2.1 闸门 10）：在绑定名末尾与表达式起点之间**扫描到一个 `=` 字节**才认领 ⇒ 不会把「没有 `=` 的伪 let」误当成合法形状（那类输入 Rust 前端会直接语法报错）。

## 3. 出口测试（有界）

| # | 钉 | 口径 |
|---|---|---|
| 1 | `sico_compiler_lowers_the_host_call_in_a_let_binding_byte_exactly` | guest 的 canonical IR **== Rust 前端的 canonical IR 逐字节**（**2215 B**），并钉住 `"op":"intrinsic"` / `"name":"sico.bytes.length"` / `"arguments":[0]` / `"range":{"start":369,"end":392}` / **无 `write_local`、无 `locals`** |
| 2 | `sico_codegen_guest_emits_the_host_call_let_binding_component_byte_exactly` | guest 发射的**整个组件 == oracle `compile_script_program` 的生产输出逐字节**（**1025 B**）且过真实 `wasmparser::Validator` |
| 3 | `sico_compiler_lowers_the_step0515_parser_function_byte_exactly` | **自举形状前沿**：新路由连同其 **19 个闭包函数**（共 20 个，按文件顺序）经 guest 自己逐字节降低 == Rust 前端 |
| 4 | （更新）`sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused` | STEP-0512 的邻接形状钉：原「let 绑定位置」一条**已被本片认领**（按其承诺移出）⇒ 换成 **5** 条**本片不认领**的邻接形状（三实参 `sico.bytes.at` / let 绑定里 `return` 别的名字 / let 绑定后还有第二条语句 / let 绑定里别的内建 `sico.text.length` / let 绑定里的三实参 `sico.bytes.at`），身份仍是 `ERR:E-SH-IR-EXPRESSION` |
| 5 | （更新）`tools/probe-m22-frontend-frontier.ps1` | 前沿探针**新增 1 条用例** `host-call-in-let-binding`（期望 `lowered`）⇒ **22 → 23 条** |

## 4. 实施记录（2026-10-22）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/parser.sico` | 新增 `direct_host_call_let_ir`（+230 行）+ 路由梯接线（+24 行）+ 嵌套调用拆解（+2 行）—— **纯插入** |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新增 3 条钉；更新 1 条邻接形状钉（原「let 绑定位置」一条移出，换成 **5** 条不认领的邻接形状） |
| `tools/probe-m22-frontend-frontier.ps1` | 新增 1 条前沿用例 |
| oracle 生产代码 | **零改动** |
| `tools/probe-m22-ir-plan-frontier.ps1` | **零改动**（计划层早就认 `intrinsic`） |

### 4.2 自造缺陷 ①：**`=` 不在 guest 词表里**

初版按「`let` → 绑定名 → `=` → 接收者」四步走词表 ⇒ 永远走不到 `=`。**怎么发现的**：把 25 道 `SKIP` 闸门临时换成可区分的 `ERR:E-SH-IR-DHCL-NN` 标记（**测完即摘**）⇒ 落在闸门 5。**修法**：接收者直接从绑定名跳（与既有机器 `fixed_guard_chain_ir` 同款），`=` 改为**字节级**检查（§2.3）。

### 4.3 自造缺陷 ②：**类型不匹配**

`U64.equal(x, I64.literal(1))` ⇒ 编译器报 `expected U64, found I64`（`I64.literal` 产出 `I64`）。**修法**：涉及 `I64` 值的比较一律用 `I64.equal`（共 3 处）。

### 4.4 自造缺陷 ③：**偏移循环把已找到的偏移覆盖掉了**

跳过缩进的字节循环里，游标变量同时兼任「结果变量」；退出时把游标置成 `source_len` ⇒ **结果也被置成了 `source_len`** ⇒ 路由永远看不到关键字。**怎么发现的**：用临时诊断把 `kw/body/nl/fn/b0/b1/b2` 拼进错误码回读。**修法**：游标与结果**分开**（`dhcl_keyword` / `dhcl_keyword_found`）。

### 4.5 自造缺陷 ④：**自举形状前沿越界**（本片最硬的一条）

新路由的闭包 20 个函数被 guest 自己降低时，guest **typed 拒绝** `ERR:E-SH-IR-CALL-SHAPE`。**怎么发现的**：写临时诊断测试分别降低「整闭包」与「`word_span_offset` 自身闭包」⇒ 后者**能降**（`Output`）⇒ 越界点在**本片新代码**里。**根因**：单实参内建机器（`gw_intrinsic_call_packed`）要求**单词实参**，而本片写了 `sico.bytes.length(sico.text.encode(...))`（**宿主调用吃宿主调用**）⇒ 拒绝。**修法**：按 STEP-0507 实测的形状规则**先落临时变量**：

```
let dhcl_binding_text = word_at(words, dhcl_binding_idx)
let dhcl_binding_encoded = sico.text.encode(dhcl_binding_text)
let dhcl_binding_len = sico.bytes.length(dhcl_binding_encoded)
```

⇒ 闭包 20 个函数重新逐字节等同（钉 #3）。

### 4.6 自造缺陷 ⑤：**STEP 号冲突**（已改正，原句保留）

上一片在 `docs/handoff-m22.md` 写的「下一刀 = STEP-0514」与**已提交的规划片** `STEP-0514-m22-w2-s5-s6-closure-ladder.md` 冲突：该规划片 §7 明写「**STEP-0514 已被本规划片占用 ⇒ 执行线下一片从 STEP-0515 起**」。⇒ 本片取 **STEP-0515**，并把 handoff 里那句改成 STEP-0515（**原句保留 + 追加改正说明**，以便追溯）。

### 4.7 工具侧观察：**CLI 默认燃料（10 M）对大 fixture 临界**（**不是代码缺陷**）

`sico run` 走的是 runner 的**默认限额**（fuel = 10 M）。实测：602 B 的 Script-profile fixture，**return 位置**勉强通过、**let 绑定位置**超限（`resource-limit.fuel`）；而 96 B 的同形状 fixture（前沿探针用的那一条）**通过** ⇒ 结论：**CLI 默认燃料对 ~600 B 的 guest 编译输入本就临界**，本片的路由多做的字节扫描只是把它推过线。**权威路径不受影响**：runner 测试用 `fuel = 800 G`（固件全量 **122 passed / 0 failed**）。**处置**：不改任何预算常量（AGENT §8 / STEP-0513 §4.4 的既定口径：只降并发、不改断言与预算），如实记录。

## 5. 诚实性

### 5.1 支持面（**恰好一个形状**）

- 只覆盖 `sico.bytes.length` **一个内建**、**一个实参**（`Bytes` 形参）、**两个位置**（return 位置 = STEP-0512；**let 绑定位置 = 本片**）。
- 下列形状**仍** typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（钉 #4 实测）：三实参 `sico.bytes.at`、let 绑定里 `return` 别的名字、let 绑定后还有第二条语句、let 绑定里别的内建（`sico.text.length`）、let 绑定里的三实参 `sico.bytes.at`。
- **CRLF 输入的诚实收窄**：本路由用**字节级**扫描定位 `let` 关键字，只跳空格与制表符 ⇒ **CRLF 源**里首行字节是 `\r` ⇒ 该形状**被 typed 拒绝**（fail-closed，**不静默误降**）；而 return 位置那条路由（STEP-0512）会跳 `\r` ⇒ 两条路由在 CRLF 输入上**不对称**。本仓**强制 LF**（行尾审计 21 个文件全绿），故不影响本线；如需对称，后续可在本路由的空白跳过里补 `\r`（**13**）。
- **其它内建 / cell 实参 / 字面量实参 / 多语句函数体 / `while`·`if` 体内**仍不在本片范围（承自本线）。
- **组件层**的支持面仍是 STEP-0513 的那一个内建（`cgs_intrinsic_hex` 的闸门未动）。

### 5.2 其余缺口（承自本线）

- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）⇒ **门 3 未动**。
- oracle 生产代码零改动；**独立 CI 尚未运行**；**未提交未推送**。
- **M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。

## 6. 验证记录（2026-10-22）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉 ×3 + 更新后的邻接形状钉 | §3 | ✅ **4 passed / 0 failed** |
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | ✅ **122 passed / 0 failed**（STEP-0513 的 119 + 本片 3 条） |
| 前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | 见 §6.1 |
| IR 计划层前沿探针 | `tools/probe-m22-ir-plan-frontier.ps1` | 见 §6.1 |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | 见 §6.1 |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | 见 §6.1 |

### 6.1 固件与前沿结果

| 项 | 结果 |
|---|---|
| 固件全量 | **122 passed / 0 failed** |
| canonical IR | guest **2215 B** == Rust 前端 **逐字节相同**（含 `intrinsic` / `arguments:[0]` / range 369..392 / **无 local**） |
| 整组件 | guest **1025 B** == oracle 生产输出**逐字节相同** + 过真实 `wasmparser::Validator` |
| 自举前沿 | 新路由闭包 **20** 个函数逐字节等同（钉 #3）⇒ **未推进形状边界** |
| 前端前沿探针 | **23/23** `FRONTIER_UNCHANGED`（新增 `host-call-in-let-binding` ⇒ `lowered`） |
| IR 计划层前沿探针 | **8/8** `PLAN_FRONTIER_UNCHANGED` |
| 注释纪律 | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2`（新代码在函数体内**零注释**） |
| 行尾审计 | ✅ `LINE_ENDING_AUDIT_GREEN` |

### 6.2 电池结果（如实记录）

见 §6.4（电池在本片收尾时整轮重跑）。

### 6.3 未做的事

- **只覆盖一个内建 / 一个实参类型 / 两个位置**（§5.1）。
- **未提交、未推送**；**独立 CI 尚未运行**。

### 6.4 本机验证电池（11 步）

本轮电池 **11/11 全绿**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`，2026-10-22 23:52–00:09）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 4 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 428 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试 ⇒ **固件 122 条已在本步复跑**） |
| `validate-0245` | 144 | 0 | 内含固件全量 **122 passed / 0 failed** + clippy |
| `validate-0262` | 156 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 165 | 0 | 前端前沿 **23/23** ⇒ `FRONTIER_UNCHANGED` |
| `probe-ir-plan-frontier` | 27 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 0 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN` |

⇒ `BATTERY_GREEN 11/11`（`BATTERY_EXIT=0`）。
