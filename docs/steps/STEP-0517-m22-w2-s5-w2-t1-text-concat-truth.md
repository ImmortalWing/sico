# STEP-0517 m22-w2 S5 **W2 族 T1 真值测量片（`sico.text.concat`）**：**前端 return 位置不是「缺能力」，是「缺可达性」**；组件层仍全拒

> - status: **measurement complete locally（本机实测绿，未提交未推送）**
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0516](./STEP-0516-m22-w2-s5-w1-scale-feasibility.md)（上一片 · W1 判读 = 可行）、[STEP-0511](./STEP-0511-m22-w2-s5-host-call-truth.md)（宿主调用真值测量片的方法学先例）
> - gate: **不推进任何出口门** —— W2 的**第 1 拍（真值测量片 T）**，按计划 §3.3.5 的「每族四拍」模板执行；**guest 生产代码零改动**、**oracle 生产代码零改动**
> - numbering: **STEP-0517**（取号前已核对 `docs/steps/`：最大号 0516；*0517 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| owner 已授权 W2 族按「每族四拍」推进（决策 1–3 + D4–D15 全部裁决关闭）。 | ✅ 计划 §3.3.5 |
| W1 的判读已给出（否则 W2 的实施方式可能整体改写）。 | ✅ STEP-0516：**可行** |
| 本拍是**纯测量**：只读 oracle **生产输出**，不写 guest 生产代码。 | ✅ 方法学先例 STEP-0501 / 0505 / 0511 |
| 出口测试有界。 | ✅ §3：5 条钉 |

⇒ **取号合法**。*测量已完成*，见 §4。

## 1. 这一片回答的问题（阶段的决策棱镜）

计划 §3.3.5 把 W2 定义为 **20 个宿主内建族**的逐字节编解码，按 **S 内调用点降序**排序：

| 序 | 内建 | S 内调用点 | 本片状态 |
|---|---|---|---|
| **T1** | **`sico.text.concat`** | **2,882** | **本片（真值测量片）** |
| T2 | `sico.text.length` | 318 | 未开始 |
| … | … | … | … |

**T1 是最高杠杆的一族**：guest 自己拼字节靠它（`selfhost/codegen.sico` 里 `sico.text.concat` 出现 **数十次**，`selfhost/parser.sico` 里更多），且 STEP-0516 实测证明「guest 现有的拼接路径**就是** O(n²) 的增量拼接」⇒ **不闭合 T1，W5（组件级尺度 rung）在规模上根本走不到**。

本片要回答的**不是**「guest 能不能降 T1」，而是：

1. **规则表**：T1 的各个形状，Rust 前端的 canonical IR 与 Rust 后端的**生产组件**分别是什么字节？
2. **子集边界**：T1 的「名字 / 实参个数 / 实参类型 / 结果槽数」四道边界分别卡在哪一层？
3. **下一刀切点**：guest 侧缺的是**能力**、**接线**、还是**可达性**？

**本片的结论（先说结果）**：**guest 前端已经实现了 T1 的 return 位置下降，而且逐字节等同；它只是「够不着」** —— 路由梯把整个 `straight_cell_*` 家族门控在「函数体除 return 外还有一条语句」的条件下。**组件层则完全没闭合**（`sico.text.concat` 不在 guest codegen 的支持子集内，fail-closed 拒绝）。

## 2. 测量设计

### 2.1 探针落点（D5）

| 要求 | 本片做法 |
|---|---|
| 探针**不进 `selfhost/`** | ✅ 探针源在**测试进程内**生成 |
| 探针组件**不进 A/B/C 链** | ✅ 不产 `.sapp`、不产 bundle |
| 探针**不进自举形状前沿** | ✅ 只读 `selfhost/` 既有函数，未新增/改写 guest 函数 |
| 测量路径与生产路径**分离** | ✅ 临时探针 `runner/sico-runner/tests/scratch_step0517_probe.rs`（**本片收尾时已删除**，不进仓）；永久证据落成**固件钉**（§3）与 `target/tmp/step0517/` 工件 |
| 证据路径**可移植** | ✅ 用 `CARGO_MANIFEST_DIR` 上溯两级推导 `target/tmp/step0517`（**不得**写死机器绝对路径 —— 同 STEP-0516 §4.11 的教训） |

### 2.2 真值来源

| 真值 | 取法 |
|---|---|
| canonical IR | Rust 前端 `lower_core` + `canonical_json`（**生产路径**） |
| 整组件字节 | Rust 后端 `sico_codegen_wasm::compile_script_program`（**生产输出**，非重实现） |
| core module code 段 | 用 `wasmparser` 解析**生产组件**，导出 operator 序列（**反汇编**） |
| guest 判读 | **真实 runner**（fuel 800 B / timeout 120 s / 内存 2560 MiB）—— 权威通道 |

### 2.3 形状谱系（10 个变体）

T1 在 S 内的真实形态不止一种。本片一次性量完 **10 个变体**，覆盖「位置 × 实参形态 × 实参个数」三个维度。

## 3. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 |
|---|---|---|
| 1 | `sico_compiler_lowers_the_text_concat_in_a_let_binding_byte_exactly` | let 绑定位置（实参为形参 / 实参为 cell）⇒ guest IR **逐字节** == Rust canonical IR（**2295 B**）；intrinsic 形状 = `{"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}`；**无 local** |
| 2 | `sico_compiler_lowers_the_text_concat_in_a_return_position_byte_exactly` | **本片的关键钉**：return 位置（体内另有 `let`）⇒ guest IR **逐字节** == Rust canonical IR（**2295 B**）；`"terminator":{"kind":"return","data":2}` |
| 3 | `sico_codegen_guest_refuses_the_text_concat_component_outside_the_subset` | oracle **生产组件 = 1094 B**（钉住）+ 过真实 `wasmparser::Validator`；guest codegen 侧 **fail-closed 拒绝** `unsupported codegen source shape` |
| 4 | `sico_compiler_keeps_the_unclosed_text_concat_shapes_typed_refused` | 4 个未闭合形状仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（**不静默放宽**） |
| 5 | `the_text_concat_arity_is_refused_by_the_rust_frontend_itself` | 1 个 / 3 个实参 ⇒ **Rust 前端自己**拒 `stdlib intrinsic arity`（**元数边界在语义层，不在 IR 层**） |

⇒ 固件从 **122** 增至 **127** 条。

## 4. 实施记录（2026-10-23）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `runner/sico-runner/tests/selfhost_compiler.rs` | **新增 5 条钉**（§3），纯追加 |
| `tools/probe-m22-frontend-frontier.ps1` | 前沿用例 **23 → 26 条**（新增 T1 三条） |
| `runner/sico-runner/tests/scratch_step0517_probe.rs` | **临时探针，收尾时已删除**（不进仓） |
| `selfhost/*.sico` | **零改动**（纯测量） |
| oracle 生产代码（`crates/*`） | **零改动** |
| `docs/plans/M22-compiler-self-host.md` / `docs/STATUS.md` / `docs/ROADMAP.md` / `docs/handoff-m22.md` / `docs/steps/README.md` | 同步本片结论 |

### 4.2 T1 规则表（实测）

| # | 变体 | 位置 | 实参 | Rust 前端 | canonical IR | oracle 组件 | **guest 前端** |
|---|---|---|---|---|---|---|---|
| a | `concat-a` | return（体内**只有** return） | 形参 ×2 | OK | 2295 B | 1094 B | ❌ `ERR:E-SH-IR-EXPRESSION` |
| b | `concat-b` | let 绑定 | 形参 ×2 | OK | 2295 B | 1094 B | ✅ **逐字节等同** |
| c | `concat-c` | return（体内**只有** return） | 字面量 ×2 | OK | 2372 B | 1124 B | ❌ `ERR:E-SH-IR-EXPRESSION` |
| d | `concat-d` | let 绑定 | cell ×2 | OK | 2295 B | 1094 B | ✅ **逐字节等同** |
| e | `concat-e` | return（体内**只有** return） | **嵌套 concat** | OK | 2566 B | 1133 B | ❌ `ERR:E-SH-IR-EXPRESSION` |
| f1 | `concat-f1` | return | **1 个实参** | ❌ **Rust 自己拒** `stdlib intrinsic arity`（365..384） | — | — | ❌ |
| f3 | `concat-f3` | return | **3 个实参** | ❌ **Rust 自己拒** `stdlib intrinsic arity`（383..408） | — | — | ❌ |
| h | `concat-h` | return（体内**只有** return） | 形参 + 字面量 | OK | 2334 B | 1113 B | ❌ `ERR:E-SH-IR-EXPRESSION` |
| **g1** | `concat-g1` | return（体内**另有** `let`） | 形参 ×2 | OK | 2295 B | 1094 B | ✅ **逐字节等同** |
| **g2** | `concat-g2` | return（体内**另有** `let` + `set`） | 形参 ×2 | OK | 2630 B | 1114 B | ✅ **逐字节等同** |

**逐字节等同的口径**：guest 的 canonical IR 输出与 Rust 前端 `canonical_json` 的输出**按字节相等**（不只是同形、不是同长）。

### 4.3 关键发现：return 位置**不是缺能力，是缺可达性**

`g1` / `g2` 是本片最关键的两个变体。**只要函数体里除 `return` 外还有一条语句，同一个 `return sico.text.concat(a, b)` 就立刻逐字节等同**（2295 B / 2630 B，**与 Rust 前端按字节相等**）。

⇒ **guest 已经实现了 T1 的 return 位置下降**，而且实现得**完全正确**（Rust 前端把 g1 里的死绑定 `let x = a` 消掉，intrinsic 直接留在 return 位置，guest 与之逐字节相同）。

**分叉点在哪（源码定位）**：

- `selfhost/parser.sico:9974` —— `straight_cell_function_ir` 里**确实有** return 位置的 concat 分支：
  ```
  if is_intrinsic_path(words, scc_return_value_idx, "text", "concat"):
    let scc_rconcat_pack = straight_text_concat_packed(..., "cell", ...)
  ```
- `selfhost/parser.sico:20579`–`20583` —— 但**路由梯**只在这个条件下才进入 `straight_cell_*` 家族：
  ```
  let body_statement_count = let_count
  if U64.less_than(body_statement_count, set_count):
    set body_statement_count = set_count
  end if
  if U64.less_than(U64.literal(0), body_statement_count):   # ← 门控
  ```
  `body_statement_count = max(let_count, set_count)`；**函数体只有一条 `return` 时，`let_count = set_count = 0` ⇒ 门控不成立 ⇒ `straight_cell_function_ir` 根本不被调用** ⇒ 落到路由梯尾部的标量 return 分析 ⇒ 以 `ERR:E-SH-IR-EXPRESSION` 收场。

⇒ **下一刀切点（精确）**：**把 T1 的 return 位置做成可达**（例如：在路由梯里为「体内只有一条 return、且该 return 是 `sico.text.concat`」开一条**专用顶层路由**，与 STEP-0512/0515 的 `direct_host_call_return_ir` / `direct_host_call_let_ir` 同族），**不需要新造 concat 机器** —— `straight_text_concat_packed`（`selfhost/parser.sico:8929`）与 `gw_text_concat_packed`（`:5808`）两台**既有**机器已经能正确处理 N 个实参（形参 / cell / 字面量）与嵌套调用。

⇒ 这条发现把 T1 前端刀的工作量从「新造机器 + 新造接线」压缩到「**开一条可达路径**」，是本片最值钱的一条结论。

### 4.4 T1 子集边界（登记进 D15 支持面登记表）

| 维度 | 边界 | 卡在哪一层 |
|---|---|---|
| **名字** | 仅 `sico.text.concat` | guest codegen 的 `cgs_intrinsic_hex` 只认 `sico.bytes.length`（STEP-0513）⇒ 其余 `ERR:E-SH-CG-IR-EMIT` |
| **实参个数** | 恰好 **2** | **语义层**（Rust 前端自己拒 `stdlib intrinsic arity`）⇒ guest **不得**自造元数检查 |
| **实参类型** | 两个都是 `Text`（`string`） | guest 前端 `straight_text_concat_packed` 的实参分支（形参 `string` / cell `string`） |
| **结果槽数** | **2**（pointer, length） | backend `emit_intrinsic`（`crates/sico-codegen-wasm/src/lib.rs:4473`） |
| **语料覆盖** | **0 条**（S 内 2,882 个调用点**一个都没覆盖**） | — |
| **未覆盖形状** | ①「体内只有 return」的 return 位置（**可达性门控**）；②嵌套 concat；③字面量实参（return 位置）；④**组件层全部** | — |

### 4.5 组件层真值（core module code 段反汇编）

oracle **生产组件** = **1094 B**（6 个 core 函数）。`join` 函数（core func 0）的 operator 序列**逐条**为：

```
I32Const { value: 0 }
LocalSet { local_index: 6 }          # dispatcher 外壳
Block { blockty: Empty }
Loop { blockty: Empty }
LocalGet { local_index: 6 }
I32Const { value: 0 }
I32Eq
If { blockty: Empty }
LocalGet { local_index: 0 }          # a_pointer
LocalGet { local_index: 1 }          # a_length
LocalGet { local_index: 2 }          # b_pointer
LocalGet { local_index: 3 }          # b_length
Call { function_index: 5 }           # sico.text.concat helper
LocalSet { local_index: 5 }          # 结果 length 槽
LocalSet { local_index: 4 }          # 结果 pointer 槽
LocalGet { local_index: 4 }          # 返回 (pointer, length)
LocalGet { local_index: 5 }
Return
End
Unreachable
End
End
Unreachable
End
```

**槽位事实**：

- 形参 `a: Text` = 槽 **0**（pointer）/ **1**（length）；`b: Text` = 槽 **2** / **3**；
- 结果对 = 槽 **4**（pointer）/ **5**（length）；dispatcher 状态 = 槽 **6**；
- `join` 的局部声明组 = `1xI32, 1xI32, 1xI32, 2xI32`。

**helper 事实（core func 5）**：`sico.text.concat` 的 helper **不是 import，而是编译进 core module 的函数**：

```
LocalGet { local_index: 1 }          # a_length
LocalGet { local_index: 3 }          # b_length
I32Add
LocalTee { local_index: 5 }
Call { function_index: 2 }           # arena 分配器（guest canonical 的 bump allocator）
LocalSet { local_index: 4 }          # 结果 pointer
LocalGet { local_index: 4 }
LocalGet { local_index: 0 }
LocalGet { local_index: 1 }
MemoryCopy { dst_mem: 0, src_mem: 0 }  # 拷 a
LocalGet { local_index: 4 }
LocalGet { local_index: 1 }
I32Add
LocalGet { local_index: 2 }
LocalGet { local_index: 3 }
MemoryCopy { dst_mem: 0, src_mem: 0 }  # 拷 b
LocalGet { local_index: 4 }          # 返回 (pointer, length)
LocalGet { local_index: 5 }
End
```

⇒ **对 codegen 刀的直接含义**：与 STEP-0513 的 `sico.bytes.length`（**内联**发射 `local.get` / `i64.extend_i32_u` / `local.set`，**不需要 helper**）不同，**`sico.text.concat` 必须发射一个 helper 函数体**（core func 5）+ **调用点**（`call <helper index>`）⇒ codegen 刀的工作量比 STEP-0513 大一档，且 helper 里的 `MemoryCopy` 是**线性拷贝**（**正好落在 STEP-0516 要求的「线性组装」约束之内**，与 guest 现有的增量拼接路径相反）。

### 4.6 自造缺陷 ①：**探针写死了机器绝对路径**

探针初版把证据写到 `E:/github/sico-w2/target/tmp/step0517/...`。**怎么发现的**：本机跑得通，但独立 CI 跑在 Linux + 不同工作目录 ⇒ 该路径不存在 ⇒ 测试会因写文件失败而红（**同 STEP-0516 §4.11 的同族问题**）。**修法**：用 `CARGO_MANIFEST_DIR`（= `<repo>/runner/sico-runner`）**上溯两级**推导仓库根，再拼 `target/tmp/step0517`。

### 4.7 自造缺陷 ②：**wasmparser API 版本差异**

反汇编探针初版按 `ImportSection` 迭代器直取 `import.ty` / `import.module` / `import.name`，并按 `entry.type_index()` 取函数类型 —— 本仓的 `wasmparser 0.252/0.253` 都不这么暴露（`Imports` 结构没有这些字段、`type_index` 叫 `type_id`）⇒ 编译错 3 处。**修法**：去掉对 import 段的依赖（本片只需要 export 段定位 `run` + code 段反汇编），函数段改用 `reader.count()`。

### 4.8 自造缺陷 ③：**只量了「位置」，漏了「可达性」**

第一轮探针量完 8 个变体后，最初的结论写成了「guest 前端**未实现** T1 的 return 位置」。**这个结论是错的**：源码里 `straight_cell_function_ir:9974` **本来就有** return 位置的 concat 分支。**怎么发现的**：读 `selfhost/parser.sico` 的路由梯时发现 `body_statement_count > 0` 的门控，于是补了 **g1 / g2** 两个决定性变体 —— 一测就**逐字节等同**。⇒ **教训**：在 guest 这类「同一函数有多个入口路径」的前端里，「某个形状被拒绝」**不等于**「该形状没实现」；**必须用「加一条无关语句」这类最小扰动去探测可达性**，否则会把「可达性缺口」误报成「能力缺口」，把下一刀的工作量估错一个数量级。

## 5. 诚实性

### 5.1 本片**只测量，不实现**

- `selfhost/*.sico` **零改动**；oracle 生产代码（`crates/*`）**零改动**；**不推进任何出口门**。
- 产出是**规则表 + 子集边界 + 下一刀切点**，**不是能力**。
- 证据等级：全部 `internal-fixture`；运行时主张只来自**真实 runner**（本机 Windows x64 GNU + Wasmtime）。

### 5.2 未闭合的（承自本片）

- **组件层完全没闭合**：`sico.text.concat` 不在 guest codegen 的支持子集内 ⇒ `--emit-component-hex` 仍 fail-closed 拒绝 `unsupported codegen source shape`（`ERR:E-SH-CG-IR-EMIT` 的外层表现）。**前端闭合 ≠ 族闭合**：按计划 §3.3.5 的族级出口判据，**五项缺一不算闭合**。
- **「体内只有 return」的 return 位置**仍 typed 拒绝（可达性门控，见 §4.3）。
- **嵌套 concat**（`concat-e`）、**字面量实参的 return 位置**（`concat-c` / `concat-h`）仍 typed 拒绝。
- **语料覆盖 0 条**：S 内 2,882 个 T1 调用点一个都没覆盖。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- **M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。

### 5.3 下一刀（W2 族 T1 的第 2 拍 / 前端刀）

按 §4.3 的切点：

1. **前端刀（F）**：为「函数体**只有一条 return**、且该 return 是 `sico.text.concat`」开一条**专用顶层路由**（与 `direct_host_call_return_ir` / `direct_host_call_let_ir` 同族），**纯插入、零既有行重排**；复用既有 `straight_text_concat_packed` / `gw_text_concat_packed` ⇒ **不新造机器**；新函数必须自证「经 guest 自己逐字节降低 == Rust 前端」（自举形状前沿钉）。
2. **codegen 刀（C）**：让 `cgs_intrinsic_hex` 认 `sico.text.concat`（**四道闸门全 fail-closed**：名字 / 实参个数 = 2 / 实参类型 = `string` / 结果槽数 = 2），并**发射 helper 函数体**（§4.5 的 core func 5：分配 + 两段 `MemoryCopy`）—— 注意这是**线性**拷贝，符合 STEP-0516 的「线性组装」约束，但 guest 现有的**增量拼接**路径（`cg_leb_hex` / `cg_hex_byte` / `cg_section_hex`）**不得**沿用。
3. **收口片（X）**：钉住 + 纠偏 + 前沿探针期望值翻转 + 固件电池 + 独立 CI（提交后）+ 诚实收窄。

## 6. 验证记录（2026-10-23）

| 验证 | 口径 | 结果 |
|---|---|---|
| T1 固件钉（5 条） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --exact <5 条> --test-threads=1` | ✅ **5 passed / 0 failed** |
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1` | 见 §6.2 |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | **26/26** ⇒ `FRONTIER_UNCHANGED`（见 §6.2） |
| clippy | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --tests -- -D warnings` | ✅ 无警告 |
| rustfmt | `cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml --all -- --check` | ✅ exit 0 |
| 证据文件 | `target/tmp/step0517/`（10 组 rust-ir / oracle-component / guest-ir 工件 + `core-code-section.txt` 反汇编） | ✅ 见 §6.2 |

### 6.1 T1 真值速查

| 项 | 值 |
|---|---|
| canonical IR（let 绑定 / return 位置 + `let`） | **2295 B** |
| canonical IR（return 位置 + `let` + `set`） | **2630 B** |
| oracle 生产组件（let 绑定 / return 位置 + `let`） | **1094 B** |
| oracle 生产组件（return 位置 + `let` + `set`） | **1114 B** |
| oracle 生产组件（两个字面量 / 嵌套 / 形参+字面量） | 1124 / 1133 / 1113 B |
| canonical intrinsic 形状 | `{"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}` |
| guest 前端判读 | let 绑定 ✅ / return 位置（体内另有语句）✅ / return 位置（体内只有 return）❌ `ERR:E-SH-IR-EXPRESSION` |
| guest codegen 判读 | ❌ `unsupported codegen source shape`（fail-closed） |
| 错元数 | **Rust 前端自己拒** `stdlib intrinsic arity` |

### 6.2 电池结果（如实记录）

本轮电池 **11/11 全绿**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 426 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试 ⇒ **固件 127 条在本步已跑过**） |
| `validate-0245` | 130 | 0 | 内含固件全量 + clippy（`-D warnings` 零警告） |
| `validate-0262` | 151 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 172 | 0 | 前端前沿 **26/26** ⇒ `FRONTIER_UNCHANGED`（含本片新增的 3 条 T1 用例） |
| `probe-ir-plan-frontier` | 25 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN`（**24** 个文件，含本片新增的 STEP-0517 文档） |

⇒ `BATTERY_GREEN 11/11`。

**这一轮是整轮重跑**：本片收尾前跑过一轮，那一轮的 `fmt-runner` 红了（rustfmt 要求把一条超长 `panic!` 消息折行）⇒ 改完格式后按纪律**整轮重跑**，上表是重跑那一轮的结果。

### 6.3 未做的事

- **只量了 10 个变体**，不是 T1 形状的穷举（S 内 2,882 个调用点的分布未逐条清点）；
- **组件层未实现**（本片只测量，见 §5.2）；
- **未提交、未推送**；**独立 CI 尚未运行**；
- **不推进任何出口门**，M22 仍为 NO-GO。

