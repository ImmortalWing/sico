# STEP-0529 m22-w2 S5 **W2 族 T2 前端刀（F）**：`sico.text.length` 的**语句位**逐字节降下来了

> - status: **本机实测绿（随本片提交并推送；独立 CI 裁决待下一批回填）**
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0528](./STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（上一片 · **T2 拍 1 真值测量**）、[STEP-0525](./STEP-0525-m22-w2-s5-w2-t1-call-arg-nested-call.md)（G-f2 刀：**新形状必须落在既有可降形状的延长线上**）、[STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）
> - gate: **不推进任何出口门** —— W2 的**第 2 拍（前端刀 F）**，按计划 §3.3.5 的「每族四拍」模板执行；**oracle 生产代码（`crates/*`）零改动**
> - numbering: **STEP-0529**（取号前已核对 `docs/steps/`：最大号 0528；*0529 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t2-text-length.md)（D15 风格，**族仍未闭合**）

## 0. 入口门槛

| 入口条件 | 状态 |
|---|---|
| T2 族的**拍 1（真值测量）**已完成（STEP-0528）⇒ 允许开**拍 2（前端刀）** | ✅ |
| 上一刀**只登记缺口、不预支能力**：STEP-0528 §5.3 明确写下「下一刀 = 让直线路由家族认这个形状，**不新造机器**」 | ✅ |
| 出口测试有界 | ✅ §3：2 条新钉 + 1 条改钉 + 前沿探针 46 → **50** 条用例 |
| oracle 生产代码零改动 | ✅ §4.1 |

⇒ **取号合法**。*实现已完成并本地实测绿*，见 §5。

> **口径**：计划 §3.3.5 的族表记 **318** 个 `sico.text.length(` 调用点，STEP-0528 实测 **322** （S 在计划成文后的增长）⇒ 本片沿用 **322** 这个实测口径。

## 1. 这一片回答的问题

STEP-0528 的结论是：**guest 前端在「条件位」上已经能逐字节降 `sico.text.length`，在「语句位」上不能**；根因是**路由分家**——通用宿主调用路由 `gw_intrinsic_call_packed` 认这个算子，而**直线路由家族**只认 `sico.text.concat`。

**本片要回答的问题**：

1. 能不能**只做接线**（不新造机器）就让**语句位**（`let` 绑定 / `set` / 体内只有一条 `return`）逐字节降下来？
2. 新写的 guest 代码**能不能被 guest 自己降下来**（自举形状前沿）？
3. 认领这个形状之后，**邻居形状的拒绝身份**会怎样变？

**本片的结论（先说结果）**：

- **能**。本片**零新造机器**：复用既有 `is_intrinsic_path` / `intrinsic_instruction_json` / `read_local_instruction_json` / `scalar_const_instruction_json` 这几台既有机器，新增 **2 个函数 + 5 处接线**，覆盖 S 内 **320 / 322** 个 `sico.text.length(` 调用点。
- **能**（但**不是免费的**）：新代码第一次把**整 `parser.sico` 自举**打红了（`ERR:E-SH-IR-RETURN-COUNT`）。根因是**新函数的函数体里有一处 `match` 嵌套在另一个 `match` 的分支内**——这是 guest 通用路由**降不下的形状**。修法见 §4.3。
- **会变，且只变一个**：邻居形状 `let n = sico.text.length(data)`（`data: Bytes`）的拒绝身份从 `ERR:E-SH-IR-EXPRESSION` 变成 **`ERR:E-SH-IR-CALL-TYPE`**（**仍然拒绝**，只是诊断变准了）。oracle 对同一形状的判词在**语义层**：`E2001 TYPE_MISMATCH`（"expected Text, found Bytes"）。**两层分别记**（§4.4）。

## 2. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 | 状态 |
|---|---|---|---|
| 1 | `sico_compiler_lowers_the_text_length_in_a_statement_position_byte_exactly`（**新**） | **5 个语句位形状**（return 形参 2212 B / let 形参 2212 B / 字面量 2252 B / let 局部 2212 B / `set` 形参 2765 B）⇒ guest IR **逐字节** == Rust canonical IR | ✅ 新增 |
| 2 | `sico_compiler_keeps_the_nested_text_length_arguments_typed_refused`（**新**，替代 STEP-0528 的 7 形状钉） | **2 个嵌套调用实参形状**（用户调用 / 宿主调用）仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（**身份不变**） | ✅ 新增 |
| 3 | `the_text_length_argument_type_is_refused_by_the_rust_frontend_itself`（**新**） | oracle 侧：`Bytes` 实参 ⇒ **Rust 前端自己**拒 `E2001` / `TYPE_MISMATCH` / "expected Text" / "found Bytes"；guest 侧：同一形状 `ERR:E-SH-IR-CALL-TYPE` | ✅ 新增 |
| 4 | `sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused`（**改钉**） | 5 个邻居形状里 **4 个**身份不变（`ERR:E-SH-IR-EXPRESSION`），**`let-other-intrinsic` 1 个**改为 `ERR:E-SH-IR-CALL-TYPE`（§4.4） | ✅ 改动 |
| 5 | `sico_guest_arena_accommodates_full_parser_selfhost_input`（**既有**） | **整 `parser.sico` 自举**（1 MB 输入）仍绿 | ✅ 恢复（§4.3） |

⇒ 固件从 **157** 增至 **159** 条；前沿探针从 **46** 增至 **50** 条用例（3 条期望翻转 + 4 条新增）。

## 3. 实施设计

### 3.1 为什么是「接线」而不是「新机器」

STEP-0528 §4.3 已证明：**通用宿主调用路由 `gw_intrinsic_call_packed` 早就认 `sico.text.length`**（`selfhost/parser.sico:7001`），而且它对「形参 / 局部 cell」两种实参的处理是**正确的**（条件位三个形状逐字节等同就是证据）。缺的只是**直线路由家族够不着那三种位置**。

⇒ 本片**复用同一台既有机器的语义**（形参 ⇒ 转发值 id；局部 cell ⇒ `read_local`；字面量 ⇒ `const_string`），把它**接进**三条语句位路由 + 一条整体表达式路由。

### 3.2 两个直线路由家族的差别（决定了「局部实参」在两种模式下的写法）

| 家族 | 镜像的 oracle 路径 | `let` 的表示 | 局部实参怎么读 |
|---|---|---|---|
| `straight_environment_function_ir` | `lower_straight_line`（`crates/sico-ir/src/lower.rs:2727`） | **纯绑定**，不建 local | **转发局部已持有的值 id**（mode `"ssa"`） |
| `straight_cell_function_ir` | `lower_general`（`crates/sico-ir/src/lower.rs:995-1023`） | **每个 `let` 成一个 cell** | **发一条 `read_local` 指令**（mode `"cell"`） |

⇒ 这就是 oracle 对同一段源码给出不同字节的原因：`b-let-param`（let 绑定）与 `d-let-local`（局部实参）字节相同（都是 2212 B），但 `g-set-param`（`set` 位）必须落 local 0（2765 B）。

### 3.3 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/parser.sico` | **+326 / −6 行**（文件 22509 → **22829** 行）：**新增 2 个函数**（`direct_text_length_return_ir` :9040–9144、`straight_text_length_packed` :9540–9640）+ **5 处接线** + **4 处标志位改名 / 新增**（1 处改名 + 3 处新增） |
| `runner/sico-runner/tests/selfhost_compiler.rs` | **+133 / −22 行**：2 条新钉 + 1 条改钉 + 1 条新语义钉（§2） |
| `tools/probe-m22-frontend-frontier.ps1` | **+28 / −2 行**：用例 **46 → 50**（3 条期望翻转 + 4 条新增） |
| oracle 生产代码（`crates/*`） | **零改动** |

**接线点一览**：

| 路由 | 位置 | mode | 说明 |
|---|---|---|---|
| `scalar_ir` 路由梯（:21037） | 整体表达式（体内只有一条 `return`） | — | 新增 `direct_text_length_return_ir`（**U64 专用**，其它返回种类一律 `"SKIP"`），排在 `direct_text_concat_return_ir` 之后 |
| `straight_cell_function_ir` | 语句位 | `"cell"` | `let` / `set` 的右侧识别 `sico.text.length` |
| `straight_cell_function_ir` | return 位 | `"cell"` | 同上 |
| `straight_environment_function_ir` | let 位 | `"ssa"` | `let` 的右侧识别 `sico.text.length` |
| `straight_environment_function_ir` | return 位 | `"ssa"` | 同上 |

**标志位语义**：`scc_concat_done` 改名 **`scc_host_done`**（语义从「已处理过 concat」变成「已由打包型宿主调用处理」）；新增 `scc_return_host_done` / `sef_return_host_done`；`sef_host_done` **每轮循环重置**（env 路由一条语句一个 `let`）。

### 3.4 拒绝身份（刻意保持）

| 形状 | 身份 | 理由 |
|---|---|---|
| 实参是**嵌套调用**（多 token） | `ERR:E-SH-IR-EXPRESSION`（**与改动前一致**） | 元数边界属**语义层**（§4.4），guest **不得自造**元数检查 |
| 实参**类型不符**（`Bytes` 喂给 `Text` 算子） | `ERR:E-SH-IR-CALL-TYPE` | 新路由**会查实参类型**；身份与**同族孪生路由** `straight_text_concat_packed`（:9253）对 `u64.to_text` 的处理**一致** |

## 4. 实施记录（2026-10-24）

### 4.1 第一轮实现：整 `parser.sico` 自举被打破

新代码写完后，固件从「157 全绿」变成 **155 passed / 3 failed**：

| 失败的钉 | 症状 |
|---|---|
| `sico_compiler_emits_the_module_ir_split_across_both_channels_byte_exactly` | `ERR:E-SH-IR-RETURN-COUNT` |
| `sico_guest_arena_accommodates_full_parser_selfhost_input` | `ERR:E-SH-IR-RETURN-COUNT` |
| `sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused` | `let-other-intrinsic` 身份从 `ERR:E-SH-IR-EXPRESSION` 变成 `ERR:E-SH-IR-CALL-TYPE` |

前两条是**同一个根因**：**guest 降不下自己新写的代码**（自举形状前沿被打破）。

### 4.2 定位（方法学：三层二分，不靠猜）

1. **函数体平凡化**：把两个新函数的函数体换成平凡体 ⇒ 整 `parser.sico` 自举**通过** ⇒ 问题在新函数体。
2. **只平凡化 `straight_text_length_packed`** ⇒ 仍**通过** ⇒ **元凶就是它**。
3. **依赖闭包探针**（`target/tmp/step0529/closure_probe.py`：抽出被测函数的**被调闭包**喂给 guest，走 runner CLI，绕开整模块的 hostcall fuel 上限）⇒ `straight_text_length_packed` 的闭包（18 个函数）⇒ `ERR:E-SH-IR-RETURN-COUNT`；对照组 `straight_text_concat_packed` 的闭包（30 个函数）⇒ **OK**。
4. **固定闭包、只换根函数体** ⇒ `resolve-only` OK、`guard-resolve` OK、**`as-written` RETURN-COUNT**、`restructured` CALL-TYPE、`pack-only` CALL-ARGUMENT ⇒ **问题段 = 最终打包段**。

> **踩过的坑（已修）**：①自写的解析器最初**丢掉了 `function` 声明行**，据此得出的二分结论全部作废（模块必须完整）；②按 `text.index("end function")` 切函数会命中函数体**里**的字符串 `"end function"` ⇒ 必须按**行精确匹配** `line == "end function"`。

### 4.3 根因与修法：`match` 嵌套在 `match` 分支内

最终打包段原本长这样（**内层 `match` 在外层 `match` 的 `case` 分支里**）：

```
  match U64.checked_add(base, stl_advance):
    case ok(stl_result):
      ...
      match U64.checked_add(stl_advance, U64.literal(1)):     <-- 嵌套 match
        case ok(stl_total):
          ...
          return __flat26
        case error(_):
          return "ERR:E-SH-IR-RANGE"
      end match
    case error(_):
      return "ERR:E-SH-IR-RANGE"
  end match
```

**guest 通用路由（`general_while_function_ir`）降不下这个形状**：它把 `match` 当作一帧 CFG，**分支里的 `match` 不被展开** ⇒ 外层那一块**不以 `return` 终结** ⇒ 命中 `parser.sico:17132` 的 `ERR:E-SH-IR-RETURN-COUNT`。

**修法（最小改动）**：把**内层** `U64.checked_add(stl_advance, 1)` 换成一个 `if`（`stl_advance` 只可能是 0 或 1 ⇒ 计数就是 1 或 2，**没有溢出面**），于是**整个函数体里不再有嵌套 `match`**：

```
      let stl_total = U64.literal(1)
      if U64.less_than(U64.literal(0), stl_advance):
        set stl_total = U64.literal(2)
      end if
```

**验证方式**：闭包探针先把三个候选形态各跑一遍（`flat-tail` / `match-tail` / `as-written`）⇒ **`match-tail: OK`**、`as-written` 红；把 `match-tail` 落到真实源码 ⇒ **Rust oracle 接受**（`sico build` 成功）⇒ **整 `parser.sico` 自举恢复绿**（`sico_guest_arena_accommodates_full_parser_selfhost_input ... ok`）。

> **教训（与 STEP-0525 同源）**：**新形状必须落在既有可降形状的延长线上**。这次不是「新机器不可降」，而是**新代码自身落在了 guest 够不着的形状上**——自举形状前沿**不是新功能的验收项，而是新代码的写作约束**。

### 4.4 邻居形状的拒绝身份变化（**一层 vs 一层，分开记**）

| | oracle（Rust） | guest |
|---|---|---|
| `let n = sico.text.length(data)`（`data: Bytes`） | **前端降下来**，语义层拒：`E2001` / `TYPE_MISMATCH` / "expected Text, found Bytes"（区间 386..390） | **前端拒**：`ERR:E-SH-IR-CALL-TYPE` |

- **guest 侧**：新路由**会查实参类型**（`binding_parameter_kind`），所以诊断从「表达式不认识」变成「**实参类型不对**」。
- **oracle 侧**：它**降**下来了，再由**语义层**报类型不匹配。
- ⇒ **两个拒绝不在同一层**（前端拒绝 vs 发射端/语义拒绝），**分别记录**；**程序结局相同**（都被拒），**诊断指向同一个缺陷**。

**取舍**：保留 `ERR:E-SH-IR-CALL-TYPE`（**不退回** `ERR:E-SH-IR-EXPRESSION`）。理由：①仍然是 **typed refusal**（fail-closed，**没有放宽**）；②它**就是** oracle 给出的诊断（类型不符），比笼统的 `EXPRESSION` 更准；③与**同族孪生路由** `straight_text_concat_packed` 的既有做法一致（`u64.to_text` 遇到非 `u64` 实参也是 `ERR:E-SH-IR-CALL-TYPE`）。**代价**：STEP-0512 的钉要改一行——已在 §2 #4 登记，并加了一条新钉把 **oracle 的语义层判词**钉住（§2 #3）。

### 4.5 前沿探针有 **1 条旧用例**跟着翻转

前沿探针跑到 `FRONTIER_MOVED count=1`：移动的是 **`host-call-in-return-position`** —— 一条
**STEP-0512 时代**钉住的用例，源码是

```
function probe() returns U64:
  return sico.text.length("abc")
end function
```

它当时被钉成 `refused:ERR:E-SH-IR-EXPRESSION`，而它**正是**本片认领的形状（return 位 + 字面量实参）
⇒ 期望值必须翻成 `lowered`。

**前沿探针只判「降 / 拒」，不判字节** ⇒ 这一条**另做逐字节验证**：
`target/tmp/step0529/moved_byte_check.py` 用 Rust 前端的 canonical JSON 与 guest 的输出对同一份 fixture
逐字节比对 ⇒ **574 B 完全相同**（`MOVED_BYTE_CHECK_GREEN`）。

⇒ 本片共 **3 条期望翻转**（`text-length-in-a-let-binding` / `text-length-in-a-set-position` /
`host-call-in-return-position`）+ **4 条新增用例**。

### 4.6 组件层：**本片零改动**

`selfhost/codegen.sico` **一行没动** ⇒ 发射器仍**不认识** `sico.text.length` ⇒ 组件层仍是 typed 拒绝（`unsupported codegen source shape`）。**本片只推进前端**，组件层留给 **codegen 刀（C）**。

## 5. 验证记录（2026-10-24）

| 验证 | 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`，冷构建） | 见 §6.2 |
| 闭包探针 | `python target/tmp/step0529/closure_probe.py straight_text_length_packed` | ✅ `guest: OK`（18 个函数 / 181343 B IR）；对照组 `straight_text_concat_packed` ✅ `OK` |
| oracle 真值 | `cargo test -p sico-ir --test dump_oracle_ir`（`ORACLE_SOURCE` / `ORACLE_OUT`） | 5 个语句位形状 = **2212 / 2212 / 2252 / 2212 / 2765** B（与 STEP-0528 一致） |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | 见 §6.2 |
| 本机电池 | `target/tmp/step0529/battery.ps1`（**19 步**） | 见 §6.2 |

### 6.1 T2 规则表更新（语句位行翻转）

| # | 形状（位置 · 实参） | IR | 组件 | guest 前端（改前 → 改后） | guest 组件 |
|---|---|---|---|---|---|
| a | return（体内只有 return） · 形参 | 2212 | 1091 | ❌ `EXPRESSION` → ✅ **逐字节等同** | — |
| b | let 绑定 · 形参 | 2212 | 1091 | ❌ → ✅ **逐字节等同** | — |
| c | return（体内只有 return） · 字面量 | 2252 | 1111 | ❌ → ✅ **逐字节等同** | — |
| d | let 绑定 · 局部（cell） | 2212 | 1091 | ❌ → ✅ **逐字节等同** | — |
| e | let 绑定 · 嵌套用户调用 | 2656 | 1145 | ❌ `EXPRESSION` → ❌ **同身份** | — |
| f | let 绑定 · 嵌套宿主调用 | 2369 | 1570 | ❌ `EXPRESSION` → ❌ **同身份** | — |
| g | `set` · 形参 | 2765 | 1113 | ❌ → ✅ **逐字节等同** | — |
| h/i/j | 条件位 3 形状 | 2991 / 3446 / 3437 | 1167 / 1187 / 1187 | ✅ 逐字节等同（**不变**） | ❌ typed 拒绝 |

⇒ **前端逐字节等同 3 → 8 行**；**仍 typed 拒绝 7 → 2 行**（且身份不变）。

### 6.2 电池

本片电池 = `target/tmp/step0529/battery.ps1`（**19 步** = STEP-0528 的 19 步原样复用，把
`step0528-doc-check` 换成 `step0529-doc-check`），日志 `target/tmp/step0529/battery.log`。

**首轮电池红了 3 步**（`fmt-runner` / `ci-one-key` / `validate-0245`）—— **同一个根因**：本片改的固件文件
没过 `cargo fmt`（`validate-step-0245.ps1` 内部也会跑一次 runner 工作区的 fmt 检查）⇒ 修完格式后
**按纪律整轮重跑** ⇒ 上表是**重跑那一轮**的结果。

| 步骤 | 耗时 | 裁决 |
|---|---|---|
| `workspace-integrity` | 52s | ✅ |
| `fmt-runner` | 1s | ✅ |
| `ci-one-key` | 409s | ✅ |
| `validate-0245` | 122s | ✅ |
| `validate-0262` | 139s | ✅ |
| `probe-frontend-frontier` | 333s | ✅ |
| `probe-ir-plan-frontier` | 26s | ✅ |
| `validate-0124` | 1s | ✅ |
| `validate-0131` | 2s | ✅ |
| `validate-0156` | 1s | ✅ |
| `git-diff-check` | 0s | ✅ |
| `line-ending-audit` | 0s | ✅ |
| `selfhost-comment-discipline` | 0s | ✅ |
| `step0526-footprint-audit` | 0s | ✅ |
| `step0526-census-branches` | 0s | ✅ |
| `step0526-differential` | 83s | ✅ |
| `step0528-census-t2` | 0s | ✅ |
| `step0529-doc-check` | 0s | ✅ |
| `workspace-integrity-final` | 51s | ✅ |

⇒ **BATTERY_GREEN 19/19**

### 6.3 独立 CI 回填（run #35 / run #36）

- **STEP-0528** 推送（`25e752a`）触发的 **run #35**（run id `40004257414`，head_sha `25e752ab765951bd9be157c8462095ed16b5a303`）= ✅ **success**（2026-10-24 本机经 GitHub API 复核）。
- **本片**推送（`e5f657a`）触发的 **run #36**（run id `40016972272`，head_sha `e5f657a7455e9f173032ada52d5f43ccc75d0266`）= ✅ **success**（2026-10-24 本机经 GitHub API 复核，**由 STEP-0530 回填**）。

## 7. 诚实性

### 7.1 本片**只推进前端**

- `selfhost/codegen.sico` **零改动** ⇒ 组件层**未闭合**。
- oracle 生产代码（`crates/*`）**零改动**。
- **不推进任何出口门**；**M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。

### 7.2 未闭合的（承自本片）

- **T2-g2 嵌套调用实参**（S 内 **2** 个调用点：`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))`、`compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`）仍 typed 拒绝 ⇒ **T2 版的 G-f2**，需要**另一刀**。
- **T2-c1 发射端不认识 `sico.text.length`** ⇒ 组件层全族 typed 拒绝 ⇒ **codegen 刀（C）**。
- **语料覆盖仍 0 条**：S 内 322 个 T2 调用点一个都没进差分语料。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- **guest 前端比 oracle 前端窄一格**：`sico.text.length` 喂 `Bytes` 时，oracle 降下来再由语义层拒，guest 在前端拒（§4.4）——**不是能力缺口，是层次不同**，但**记在支持面登记表上**。

### 7.3 下一刀（W2 族 T2 的第 3 拍 / codegen 刀）

**codegen 刀（C）**：让 `cgs_intrinsic_hex` 认 `sico.text.length`（**四道闸门全 fail-closed**：名字 / 实参个数 = 1 / 实参类型 = `string` / 结果槽数 = 1），并**发射 helper 函数体**（STEP-0528 §4.5 实测 **51 B**，数的是 **UTF-8 码点**）+ **调用点**（`local.get ×2; call; 0xad; local.set`）+ **两个内建 helper 的名字排序索引**（`helper_count + 3 + 名字排序位`）。之后是 **收口片（X）**。
