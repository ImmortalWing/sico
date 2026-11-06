# STEP-0505 m22-w2 S5 `error` 臂真值：`ScriptError` 结果区偏移从「冻结推导」升级为「实测」

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0504](./STEP-0504-m22-w2-s5-derived-component.md)（派生组件）、[STEP-0503](./STEP-0503-m22-w2-s5-run-body-result-return.md)（结果区返回）、[STEP-0501](./STEP-0501-m22-w2-s5-run-body-truth.md)（run 体真值）、[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位/局部声明计划）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是发射侧的第五刀（`error` 臂真值测量）**
> - numbering: M22 线顺延为 **STEP-0505**（本线已占用 STEP-0491..0499、STEP-0501..0504；取号时已核对 `docs/steps/`、全部分支与 `E:\github` 递归，**均无 0505**）

## 1. 本片目标与结论

**目标**（owner 2026-10-21 拍板）：把组件发射扩到非参考形态，**先取 `error` 臂的真值**——把 `ScriptError` 的结果区偏移从 STEP-0501/0503 登记的「**冻结推导**」升级为「**实测**」。

**结论（实测）**：

- Rust backend 对一个返回 `error(...)` 的 Script-profile `run` **真的发射组件**（**939 B**），**不是**拒绝 ⇒ `error` 臂**有真值**，不再是「没有实测真值所以拒绝」。
- **`ScriptError` 结果区偏移（实测）**：`code` → **结果区 8**（`i32.store8`，**1 字节**）；`message` → **结果区 12（指针）/ 16（长度）**（`i32.store` align=2）。
- `error` 臂的**结果区返回段 = 59 B（118 hex）**（ok 臂 89 B）；**尾部仍是 6 字节 dispatcher 尾声**（与 ok 臂同形）。
- `error` 臂的**局部声明段 = 17 组 / 18 槽**（17 i32 + 1 i64）（ok 臂 21 组 / 22 槽，19 i32 + 3 i64）⇒ **槽位计划随载荷变**，不是固定字节。
- **guest 当前的拒绝点已精确定位**：**不在 codegen，而在 IR 构造阶段**——`--emit-locals-hex` 与 `--emit-run-body-hex` 对该 fixture 都返回 `ERR:E-SH-IR-CONSTRUCT-VALUE`。
- **根因**：Rust 前端把**枚举 case** 降级为 **空载荷的 `variant` op**（`{"op":"variant","data":{"name":"ScriptErrorCode.InvalidInput","payload":[]}}`），而 guest 的 `construct` 字段值只支持 `const_string` / 字段投影 / `I64`·`U64` 字面量 ⇒ **缺「枚举 case 操作数」这一种形状**。

**本片只测量，不改 guest 行为**（与 STEP-0501 同规）：真值 + 缺口定位 + 下一片的切点。

**门 3 仍为 NO-GO**：本片没有让 guest 支持 `error` 臂（guest 仍然 typed 拒绝），更没有动 37 语料 / formatter / 全源 canary 阶梯与 ADR-0015 的 `A == B == C` + `.sapp` 链。

## 2. 实测数据（全部读自 backend 的**生产输出**）

### 2.1 fixture

Script-profile 入口不变，只把返回值换成 `error` 臂：

```
function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "boom"))
end function
```

**这不是「推导出來的」**：`compile_script_program` 对它**真的发射**了组件，组件通过了 `wasmparser::Validator`（新固件钉里断言）。

### 2.2 组件与核心模块

| 量 | `error` 臂（实测） | `ok` 臂（实测） |
|---|---|---|
| 组件总长 | **939 B** | 981 B |
| 核心模块 code section 的 `run` **code** | **134 B** | 180 B |
| `run` 的**局部声明段** | **34 B（17 组 / 18 槽）** | 42 B（21 组 / 22 槽） |
| `run` 的**结果区返回段** | **59 B（118 hex）** | 89 B（178 hex） |

`alloc` / `realloc` / `post` 三个 helper 体**不变**（仍是冻结段）。

### 2.3 局部声明段（实测）

```
017f017f017f017f017f017f017f017f017f017f017e017f017f017f017f017f027f
```

= 17 组：**10 × i32 + 1 × i64 + 5 × i32 + 1 组 2 × i32** ⇒ **18 槽（17 i32 + 1 i64）**。

对比 ok 臂（STEP-0499 实测）：`017f×4, 017e, 017f×4, 017e, 017f×5, 017e, 017f×4, 027f` = 21 组 / **22 槽（19 i32 + 3 i64）**。

⇒ **`error` 臂比 `ok` 臂少 3 组 / 4 槽**，其中 **i64 少 2 个**（ok 臂的 3 个 i64 槽来自 `ScriptOutput` 的 `exit_code` 与布局；`error` 臂只剩 1 个）。

### 2.4 结果区返回段（59 B，逐条解码）

实测 hex：

```
4120100121142014200a3a0000201420103a0008201210012115201520112012fc0a00002014201536020c2014201236021020140f0b000b0b000b
```

| 偏移 | 字节 | 指令 | 含义 |
|---|---|---|---|
| 0 | `41 20` | `i32.const 32` | 结果区大小（**冻结**） |
| 2 | `10 01` | `call 1` | 分配器（**冻结**） |
| 4 | `21 14` | `local.set 20` | `return_area` = scratch 对第一格 |
| 6 | `20 14` | `local.get 20` | |
| 8 | `20 0a` | `local.get 10` | **判别值槽 = 10**（IR 派生；ok 臂是 14） |
| 10 | `3a 00 00` | `i32.store8 align=0 offset=0` | 判别值 → 结果区 **0** |
| 13 | `20 14` | `local.get 20` | |
| 15 | `20 10` | `local.get 16` | **`ScriptError.code` 槽 = 16** |
| 17 | `3a 00 08` | `i32.store8 align=0 offset=8` | **code → 结果区 8（1 字节）** |
| 20 | `20 12` | `local.get 18` | **`message` 长度槽 = 18** |
| 22 | `10 01` | `call 1` | 向分配器要一份拷贝缓冲 |
| 24 | `21 15` | `local.set 21` | `copy_destination` = scratch 对第二格 |
| 26 | `20 15` | `local.get 21` | |
| 28 | `20 11` | `local.get 17` | **`message` 指针槽 = 17** |
| 30 | `20 12` | `local.get 18` | 长度槽 |
| 32 | `fc 0a 00 00` | `memory.copy` | 有界拷贝 |
| 36 | `20 14 20 15` | `local.get` | |
| 40 | `36 02 0c` | `i32.store align=2 offset=12` | **message 指针 → 结果区 12** |
| 43 | `20 14 20 12` | `local.get` | |
| 47 | `36 02 10` | `i32.store align=2 offset=16` | **message 长度 → 结果区 16** |
| 50 | `20 14` | `local.get 20` | |
| 52 | `0f` | `return` | 返回结果区指针 |
| 53 | `0b 00 0b 0b 00 0b` | `end`/`unreachable`/`end`/`end`/`unreachable`/`end` | **dispatcher 尾声（6 字节）** |

### 2.5 与 `ok` 臂的**结构**差异（不是字节差异，是规则差异）

| 维度 | `ok` 臂 | `error` 臂 |
|---|---|---|
| 结果区里的字段 | 2 个 list（`stdout`/`stderr`）+ 1 个 i64（`exit_code`） | 1 个 **i32**（`code`）+ 1 个 list（`message`） |
| i64 字段的存法 | `i64.store align=3`（`37 03 18`） | **没有** i64 字段 ⇒ 段里**不含** `37 03` |
| i32 字段的存法 | — | `i32.store8 align=0`（**1 字节**，不是 4 字节） |
| 运行时分派 | 无（`has_ok && has_error` 为假 ⇒ 静态选 `ok` 记录） | 无（同样静态选 `error` 记录）⇒ **同样没有 `if/else`** |
| 尾部 | 6 字节 dispatcher 尾声 | **同样 6 字节** |

⇒ **一条新规则（实测）**：结果区里的**标量 i32 字段用 `i32.store8` 存 1 字节**（不是 `i32.store` 存 4 字节）。这与 `ScriptErrorCode` 是枚举（canonical ABI 下 1 字节判别）一致，但本片是**实测**，不是推导。

## 3. guest 侧的缺口：**IR 构造阶段**（不是 codegen）

**实测的 guest 行为**（对该 fixture）：

| 驱动模式 | guest 的结果 |
|---|---|
| `--emit-locals-hex` | `Domain { code: "invalid-input", message: "ERR:E-SH-IR-CONSTRUCT-VALUE" }` |
| `--emit-run-body-hex` | 同上 |
| `--emit-component-hex` | `Domain { code: "invalid-input", message: "unsupported codegen source shape" }` |

⇒ guest 在**构造自己的 canonical IR 时**就拒绝了，**根本没走到 codegen**。因此 STEP-0503 登记的「`error` 臂一律拒绝」在 guest 侧是**双重拒绝**：前端先拒（`ERR:E-SH-IR-CONSTRUCT-VALUE`），codegen 后拒（`ERR:E-SH-CG-IR-EMIT`）。

**根因（对着 Rust 前端的真实 IR 读出来）**：

```json
{"result":1,"ty":{"kind":"named","data":"ScriptErrorCode"},
 "operation":{"op":"variant","data":{"name":"ScriptErrorCode.InvalidInput","payload":[]}},
 "range":{"start":485,"end":513}}
```

⇒ **枚举 case 在 IR 里是「空载荷的 `variant` op」**，名字是 `"<枚举类型>.<case>"`（点分）。

而 guest 的 `construct` 字段值降级（`selfhost/parser.sico` 的 `construct_lowering`）目前只认**三种**形状：

| 形状 | 产生的 op | guest 支持？ |
|---|---|---|
| 字符串字面量 | `const_string` | ✅ |
| 字段投影 `base.field` | `project` | ✅ |
| `I64.literal(n)` / `U64.literal(n)` | `const_i64` / `const_u64` | ✅ |
| **枚举 case `Enum.Case`** | **`variant`（空载荷）** | ❌ **缺** |

⇒ **下一片的切点已经找准（两刀）**：

1. **前端刀**：`construct_lowering` 增加「枚举 case 操作数」形状——基名是**已声明的 enum**（**不是 record**）。guest 现在的点分分支**会**被走进（`record_named_type` 只校验标识符形态，**不判断是否已声明 record**），但 `record_field_kind` **只扫 `record` 声明**（`parser.sico:267`）⇒ 对 enum 返回**空串** ⇒ 分支守卫 `if U64.less_than(0, cfv_field_kind_len)` 不成立 ⇒ 落到 `ERR:E-SH-IR-CONSTRUCT-VALUE`。⇒ 新分支必须先判「基名是**已声明的 enum**」再取 case 名，发出 `{"op":"variant","data":{"name":"Enum.Case","payload":[]}}`。
2. **codegen 刀**：`cgs_result_return_hex` 的 `error` 臂分支——按 §2.4 的规则发射 `ScriptError` 的两个字段（`code` 用 `i32.store8` @8、`message` 用「分配 + memory.copy + 两次 i32.store」@12/16），并让槽位计划产出 §2.3 的 **17 组 / 18 槽** 局部声明段。

## 4. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| `error` 臂真值钉（局部声明 + 整段 run 体 + 结果区段 + 三个 store + 无 i64 store + 6 字节尾声 + 59 B 长度） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler oracle_error_arm_result_area_truth_is_pinned` | **ok** |
| 组件真的发射且通过真实验证器 | 同上（`compile_script_program` + `wasmparser::Validator::validate_all`） | **ok** |
| 结果区标记在 `error` 臂里唯一 | 同上（`hex.matches("41201001").count() == 1`） | **ok** |
| guest 当前仍 typed 拒绝（IR 构造阶段） | `--emit-locals-hex` / `--emit-run-body-hex` / `--emit-component-hex` | `ERR:E-SH-IR-CONSTRUCT-VALUE` / `unsupported codegen source shape`（**实测**） |
| 固件套件 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds -- --test-threads=1` | **9 + 104 + 1 + 2 = 116 passed / 0 failed**（selfhost_compiler 由 103 增至 104，+1 即本片的新钉） |

> 本机验证电池结果见 §4.1。

## 4.1 本机验证电池

`target/tmp/step0504-battery.ps1`（一键 CI 放在最前、机器最空闲时跑）：

| 步骤 | 耗时 | 结果 |
|---|---|---|
| `fmt (runner)` | 00:00:01 | PASS |
| one-key CI（`tools/run-ci.ps1`，**12 步**） | 00:05:10 | **CI GREEN** |
| M22 regression（0245） | 00:00:58 | PASS（内含固件 **104 passed**） |
| M22 checker（0262） | 00:02:13 | PASS |
| 前端前沿探针 | 00:02:11 | PASS（`FRONTIER_UNCHANGED`） |
| IR 计划层前沿探针 | 00:00:25 | PASS（`PLAN_FRONTIER_UNCHANGED`） |
| 规划合同（0124） | 00:00:00 | PASS |
| 文档校验器（0131） | 00:00:01 | PASS |
| 文档校验器（0156） | 00:00:00 | PASS |
| 空白检查（`git diff --check`） | 00:00:00 | PASS |
| 行尾审计（纯 LF、零 CRLF、零 BOM，11 文件） | 00:00:01 | PASS（`LINE_ENDING_AUDIT_GREEN`） |

⇒ **11/11 BATTERY_GREEN**。

> **第一遍电池在第一步就红了**（`fmt (runner)` exit=1）——见 §5.3 的第 3 条自造缺陷（程序化回填常量破坏了 rustfmt 排版）。**这正是电池该做的事**：它把一个会被独立 CI 抓到的排版违规拦在了本机。修好后整轮重跑，上表是修好后的结果。

## 5. 诚实性

- **本片只测量，不实现**。guest 对 `error` 臂**仍然 typed 拒绝**（前端 `ERR:E-SH-IR-CONSTRUCT-VALUE`）⇒ **guest 并不支持 `error` 臂**，本片没有改变任何 guest 行为（`selfhost/parser.sico` 与 `selfhost/codegen.sico` **零改动**）。
- **本片踩到并修正的 1 条自造缺陷**：为了避免手抄长 hex 出错，我用脚本**程序化**把实测的局部声明常量回填进测试文件——但脚本写入的换行/缩进**不符合 rustfmt**，⇒ **第一遍电池在第一步 `fmt (runner)` 就红了**（exit=1）。修法 = 用 `cargo fmt` 让 rustfmt 自己排版（而不是手改），并复核文件仍是**纯 LF**（4966 LF / 0 CRLF）⇒ 整轮重跑电池 11/11 绿。**教训**：**程序化改源码后必须让 formatter 过一遍**，否则「自动化省事」会把排版违规带进提交。

- **真值只覆盖这一个 fixture**。`ScriptError` 的两个字段（`code` 是枚举、`message` 是 list）是 Script profile 里**唯一**的 `error` 臂载荷形状 ⇒ 但「枚举 case 作为字段值」这一形状本身在别的记录里也会出现（任何 `enum` 字段的 record），本片的实测**只**证明了这个具体组合的字节。
- **槽位索引（10 / 16 / 17 / 18 / 20 / 21）是实测字节事实**，**不是**槽位分配规则。本片**没有**推导 backend 的槽位分配规则 ⇒ 下一片要产出「与 `error` 臂局部声明段逐字节相同」的结果，**必须**先重算槽位计划，而不是把本片记下的数字写死进 guest。
- **`i32.store8` 存 1 字节是实测**，但本片**没有**验证「其它 1 字节标量字段」是否一律如此（Script profile 的 `error` 臂里只有 `code` 一个）。
- oracle 生产代码**零改动**；读取器与真值都写在测试文件内。
- 本片**没有**触碰 37 语料阶梯、formatter、全源 canary 或 `.sapp` 链。**门 3 仍为 NO-GO**。

## 6. 下一片

1. **STEP-0506 = 前端刀 + codegen 刀**（owner 已拍板的方向：「扩到非参考形态」）：
   - 前端：`construct_lowering` 增加「枚举 case 操作数」形状（基名是**已声明 enum**），发出空载荷 `variant` op；
   - codegen：`cgs_result_return_hex` 增加 `error` 臂分支（§2.4 的规则）+ 槽位计划产出 `error` 臂的局部声明段（§2.3）；
   - 出口：guest 对该 fixture 发射的**整个组件**与 backend 生产输出**逐字节相同**（含 17 组 / 18 槽的局部声明段与 59 B 的结果区返回段）。
2. 之后是**多函数 / data 段模块**（需要把 guest 的**函数段**与 **data 段**做成派生的）。
3. 再往后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。

> 提醒（承接 [handoff-m22.md](../handoff-m22.md) §4/§5）：guest 前端的**形状前沿**是「新代码必须能被 guest 自己降低」的全部已知约束之一。本片的发现（枚举 case 是一种新的**操作数形状**）说明：**前端的形状表比 codegen 的发射规则更早成为瓶颈**——发射规则再完备，前端造不出 IR 就用不上。
