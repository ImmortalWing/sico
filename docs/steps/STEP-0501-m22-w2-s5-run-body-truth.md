# STEP-0501 m22-w2 S5 run 体发射真值：逐字节实测、规则表与下一片的可对拍切点

> - status: **diagnosis / registration + oracle truth measured and pinned**（本片**不**发射任何 run 体字节）——**本机全绿但未提交、未推送**（待 owner 授权；独立 CI 裁决尚未取得）
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位/局部声明计划）、[STEP-0487](./STEP-0487-m22-w2-s5-codegen-skeleton.md)（冻结的最小 echo 组件骨架）、[STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md)（IR 摄取与发射计划）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是 run 体发射的"真值测量刀"**
> - numbering: M22 线顺延为 **STEP-0501**（本线已占用 STEP-0491..0499；并行 M23 会话占用 STEP-0489/0490/0498/0500；已核对 `docs/steps/` 无 0501 冲突）

## 1. 本片目标与结论

**目标**：把 Rust backend **实际发射**的 `run` 函数体逐字节取出来、解码成**可复刻的发射规则表**，并给下一片留一个**无歧义、可对拍**的切点。

**结论（实测）**：已闭合。

- 取值方式与 STEP-0496/0499 同规：**读 oracle 的生产输出**（`compile_script_program` 真正产出的组件 → wasmparser → 导出 `run` 的 core module 函数体 → 跳过局部声明后的 code 字节），**不是**在测试里复刻发射器。
- 参考形态（Script profile echo）的 `run` 体 code = **360 个 hex 字符 = 180 字节**。
- 体内部有一条**唯一命中**的分界标记 **`41201001`**（`i32.const 32` = 结果区大小；`call 1` = 分配器），据此切成：
  - **指令段** = 182 hex（91 B）
  - **终止符（Script 结果区返回）+ dispatcher 尾声** = 178 hex（89 B）
- **新固件钉** `oracle_run_body_emission_table_is_pinned`：把整段真值、标记唯一性、指令段切分三件事**一起钉在 oracle 的生产输出上**；同时提供下一片直接可用的读取器 `oracle_run_instruction_segment_hex(source)`。
- 附带确认一件对门 3 关键的事实：**参考形态的组件里，run 体是唯一的 IR 相关部分**（核心模块无 data 段；类型/函数/内存/全局/导出全是冻结常量）。也就是说，**只要 run 体能从 IR 派生地逐字节等同，参考形态的整个组件就能逐字节等同** —— 门 3 的 `A == B` 对参考形态是可达的。

**门 3 仍为 NO-GO**：本片**没有**让 guest 发出任何 run 体字节；37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。

## 2. 为什么先量真值，而不是直接写发射

STEP-0499 已把**槽位**钉死（局部声明段逐字节等同）。剩下的 run 体有三类字节来源，风险各不相同：

| 来源 | 依赖 | 出错时的症状 |
|---|---|---|
| dispatcher 外壳（固定） | 入口块号、块号、dispatcher 槽位 | 整段偏移全错 |
| 指令段（IR 派生） | 结果 id → 槽位映射、记录字段槽偏移、op 种类 | 局部字节错，且错法与"槽位算错"混淆 |
| Script 结果区返回（组件级事实） | 分配器函数索引、结果区大小、**冻结的 WIT 字段偏移** | 尾部偏移错 |

先把三段**分别**量出来并钉住，下一片才能把"对不上"归因到具体一段，而不是在 180 字节里盲找。

## 3. 实测真值（oracle 生产输出）

### 3.1 整段（180 字节 / 360 hex）

```
4100211702400340201741004604402002210420032105200221062003210742002108200421092005210a2006210b2007210c2008210d4100210e2009210f200a2110200b2111200c2112200d21134100211441002115410021164120100121182018200e3a00002010100121192019200f2010fc0a0000201820193602082018201036020c201210012119201920112012fc0a000020182019360210201820123602142018201337031820180f0b000b0b000b
```

### 3.2 指令段（91 字节 / 182 hex）—— 下一片的对拍目标

```
4100211702400340201741004604402002210420032105200221062003210742002108200421092005210a2006210b2007210c2008210d4100210e2009210f200a2110200b2111200c2112200d2113410021144100211541002116
```

### 3.3 终止符 + 尾声（89 字节 / 178 hex）

```
4120100121182018200e3a00002010100121192019200f2010fc0a0000201820193602082018201036020c201210012119201920112012fc0a000020182019360210201820123602142018201337031820180f0b000b0b000b
```

## 4. 逐字节解码：发射规则表

### 4.1 槽位真值（与 STEP-0499 的局部声明段互为印证）

| 归属 | 局部槽位 | 说明 |
|---|---|---|
| 形参 `input`（`ScriptInput` = arguments(ptr,len) + stdin(ptr,len)） | 0–3 | 4 槽，**不进**局部声明段 |
| 结果 1（`project`，bytes） | 4–5 | |
| 结果 2（`project`，bytes） | 6–7 | |
| 结果 3（`const_i64`） | 8 | i64 |
| 结果 4（`construct ScriptOutput`） | 9–13 | 9,10 i32 / 11,12 i32 / **13 i64**（exit_code） |
| 结果 5（`variant ok`） | 14–22 | 14 = tag(i32)；15–18 i32、**19 i64**（ok 载荷的 exit_code）；20–22 i32（error 载荷） |
| dispatcher | 23 | |
| scratch（return_area, copy_destination） | 24, 25 | |

⇒ 局部声明段 = 19 组（16 × i32 + 3 × i64）+ dispatcher + scratch 对 = **21 组**，与 STEP-0499 实测的 `017f…027f` **逐字节相同**。i64 槽位正是 8 / 13 / 19（局部声明段的第 5 / 10 / 15 组）。

### 4.2 指令段逐条

| 字节 | 指令 | 规则（来自 `crates/sico-codegen-wasm/src/lib.rs`） |
|---|---|---|
| `41 00` / `21 17` | `i32.const 0` / `local.set 23` | 入口块号写 dispatcher（`compile_function` 开头） |
| `02 40` / `03 40` | `block` / `loop` | dispatcher 外壳（空块类型） |
| `20 17` / `41 00` / `46` / `04 40` | `local.get 23` / `i32.const 0` / `i32.eq` / `if` | 每个块的分派头（块号 = 块的 id） |
| `20 02 21 04` / `20 03 21 05` | `local.get 2`→`local.set 4` / `local.get 3`→`local.set 5` | 结果 1 = `project input.stdin`：`copy_slots` 逐槽复制 base 的 `stdin` 字段槽（base 槽 0 + 字段槽偏移 2 = 槽 2,3） |
| `20 02 21 06` / `20 03 21 07` | 同上 → 槽 6,7 | 结果 2 = 第二次 `project input.stdin` |
| `42 00` / `21 08` | `i64.const 0` / `local.set 8` | 结果 3 = `const_i64` |
| `20 04 21 09` … `20 08 21 0d` | 5 次 `local.get`/`local.set` | 结果 4 = `construct ScriptOutput`：**按 op 的 `fields` 顺序**把各字段值的源槽连续拷进结果槽（`copy_slots`；源槽 4–8 → 目标槽 9–13） |
| `41 00` / `21 0e` | `i32.const 0` / `local.set 14` | 结果 5 = `variant ok`：先写 tag（`variant_tags`，按声明顺序 `ok` 在前 ⇒ 0） |
| `20 09 21 0f` … `20 0d 21 13` | 5 次 `local.get`/`local.set` | 拷贝 ok 载荷（结果 4 的槽 9–13 → 15–19） |
| `41 00 21 14` / `41 00 21 15` / `41 00 21 16` | 3 次 `i32.const 0`/`local.set` | **清零**未激活臂（error 载荷）的 3 个槽（`zero_local`，按类型选 i32/i64） |

### 4.3 终止符（Script 结果区返回）+ 尾声

| 字节 | 指令 | 说明 |
|---|---|---|
| `41 20` / `10 01` / `21 18` | `i32.const 32` / `call 1` / `local.set 24` | 向分配器要 32 字节结果区，落到 scratch 的 `return_area` |
| `20 18` / `20 0e` / `3a 00 00` | `local.get 24` / `local.get 14` / `i32.store8` | 把 tag 写到结果区偏移 0 |
| （ok 分支，无 `if/else`） | | **实测**：本形态**没有** `i32.eqz`/`if`/`else` —— `has_ok && has_error` 为假（variant 的值布局只带**被构造那一臂**的字段），故直接按 `output` 记录落字段 |
| `20 10 10 01 21 19` + `… fc 0a 00 00` + `36 02 08` / `36 02 0c` | 分配 + `memory.copy` + 两次 `i32.store` | `stdout`（ptr,len）→ 结果区 **8**（ptr）与 **12**（len） |
| 同上（第二组） | | `stderr`（ptr,len）→ 结果区 **16** / **20** |
| `20 18 20 13 37 03 18` | `i64.store` | `exit_code` → 结果区 **24** |
| `20 18` / `0f` | `local.get 24` / `return` | 返回结果区指针 |
| `0b` / `00` / `0b` / `00` / `0b` / `00` / `0b` | `end`(if) `unreachable` `end`(loop) `unreachable` `end`(block) `unreachable` `end`(fn) | dispatcher 尾声 |

### 4.4 终止符依赖的**组件级事实**（不是 IR 派生的）

| 事实 | 值 | 来源 | 实测？ |
|---|---|---|---|
| 分配器函数索引 | `1` | 核心模块函数序：0=run, 1=alloc, 2=realloc, 3=post（冻结的组件骨架） | **是**（观测到 `call 1`） |
| 结果区大小 | `32` | `ScriptAbi::load()` 解析**冻结的 Script WIT** 后派生：`align_to(payload_offset + max(output.size, error.size), payload_align)`；仓库内 `canonical.rs` 的冻结布局测试断言 `(32, 8)` | **是**（观测到 `i32.const 32`） |
| 结果区载荷起始偏移 | `8` | 同上：`payload_offset = align_to(1, max(output.align, error.align))` | **是**（观测到首个字段落在 8） |
| `ScriptOutput` 结果区字段偏移 | `stdout` 0（ptr@0,len@4）、`stderr` 8（ptr@8,len@12）、`exit_code` 16（i64） | **冻结的 WIT 布局**（`record_layout` 从 WIT 类型算出：i32 占 4 字节、i64 占 8 字节） | **是**（观测到 store 落在 8 / 12 / 16 / 20 / 24） |
| `ScriptError` 结果区字段偏移 | `code` 0（i32）、`message` 4（ptr@4,len@8） | 同上（**由 WIT 派生，参考形态不触发 ⇒ 未实测**） | **否** |

> **两套布局必须分清**：§4.1 的 8 字节 flat 槽布局用于**局部变量与槽位**；§4.4 的 4 字节 WIT 布局用于**结果区**。这正是 STEP-0499 §6 修掉的那处规范矛盾所指的同一件事（`script-component-emission-v0.md` §3 的 flat 规则 vs §6 的结果区布局）。
>
> **组件级事实的"冻结"含义**：它们由**冻结的 Script WIT + 冻结的组件骨架**决定，**不随模块源码变化**，因此 guest 可以携带它们（就像 STEP-0487 携带冻结的 alloc/realloc/post 体与核心模块段）；但它们**不是**从 IR 派生的，guest 必须另有通道提供（STEP-0503 的工作）。

## 5. 下一片的分解（已按上表切好，每刀都有可对拍切点）

| 片 | 内容 | 对拍对象 | 依赖的冻结事实 |
|---|---|---|---|
| **STEP-0502（下一片）** | guest 发射**指令段**（外壳 + 分派 + project/const_i64/construct/variant） | `oracle_run_instruction_segment_hex` 的 182 hex | 无（全部 IR 派生 + STEP-0499 的槽位计划） |
| **STEP-0503** | guest 发射**Script 结果区返回** | 终止符 + 尾声的 178 hex | §4.4 的五项冻结事实（分配器索引、结果区大小/偏移、两个 Script 记录的 WIT 字段偏移） |
| **STEP-0504** | 用派生 run 体替换 STEP-0487 的**冻结 run 体 hex**，让 `--emit-component-hex` 对参考形态变成 IR 派生 | 整个组件逐字节等同（STEP-0487 已证明冻结组件与 oracle 逐字节相同） | 前两片 |
| 之后 | 37 语料 → formatter → 全源 canary 阶梯 → ADR-0015 `A == B == C` + `.sapp` | | |

**STEP-0502 的 fail-closed 子集**（超出即 typed 拒绝，不静默近似）：
- 只支持参考形态用到的四个 op：`project`（标量聚合字段）、`const_i64`、`construct`（按 op 的 fields 顺序）、`variant`（`ok` 臂左对齐 / `error` 臂右对齐 —— 后者实测规则见 `lib.rs` 的 `start = destination.len() - payload_len`）。
- 不支持：`const_int`/`const_u64`/`const_bool`/`const_string`/`const_bytes`、`copy`、`intrinsic`、`call`、多块函数、函数局部 cell。
- 新 typed 身份建议：`ERR:E-SH-CG-RUN-BODY`（形状超子集）、`ERR:E-SH-CG-RUN-SLOT`（结果 id → 槽位映射不成立）。

## 6. 本片的产物

- `runner/sico-runner/tests/selfhost_compiler.rs` 新增：
  - `oracle_run_body_code_hex(source)` —— 从 oracle 的生产输出取 `run` 的 code 字节（跳过局部声明）并 hex 化；
  - `oracle_run_instruction_segment_hex(source)` —— 按 `41201001` 标记切出**指令段**（下一片直接可用）；
  - 固件钉 `oracle_run_body_emission_table_is_pinned` —— 整段真值 + 标记唯一性 + 两段切分，**三者一起钉在生产输出上**。
- **本片没有改动 guest 源码**，没有改动 oracle 生产代码；`crates/sico-codegen-wasm` 的生产代码**零改动**（读取器写在测试文件内，与 STEP-0496/0499 同规）。

## 7. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 真值钉 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler oracle_run_body_emission_table_is_pinned` | **ok**（1 passed / 0 failed） |
| 整段实测 | 同上（临时测量后固化） | run 体 code = **360 hex = 180 B**；标记 `41201001` 命中 **1** 次；指令段 **182 hex = 91 B**；终止符+尾声 **178 hex = 89 B** |
| 局部声明段互证 | STEP-0499 的 `017f…027f` | 与 §4.1 的槽位表**逐字节相同**（i64 槽 8/13/19） |
| 组件无 data 段 | `selfhost/codegen.sico` 的 `cg_core_module_hex` | 段号集合 {1,3,5,6,7,10}，**无 11（data）** ⇒ 参考形态无 IR 派生的数据段 |
| 固件套件 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **9 + 97 + 1 + 2 = 109 passed / 0 failed**（selfhost_compiler 由 96 增至 97，+1 即本片的新真值钉） |
| M22 回归 / checker / 两个前沿探针 | `validate-step-0245.ps1` / `validate-step-0262.ps1` / `probe-m22-frontend-frontier.ps1` / `probe-m22-ir-plan-frontier.ps1` | 全绿（0245 exit 0；0262 `STEP_0262_OK`；18/18 `FRONTIER_UNCHANGED`；8/8 `PLAN_FRONTIER_UNCHANGED`） |
| 一键 CI | `tools/run-ci.ps1` | **12/12 全 PASS**（exit 0）：`fmt` / `fmt (runner)` / `clippy (workspace)` / `clippy (runner)` / `build (runner debug)` / `test (workspace)` 1:35 / `test (runner)` 3:01 / `module boundaries` / `planning contract (M14-M25)` / `application-profile matrix` / `cross-host matrix + UI corpus` / `whitespace (git diff --check)` |

> **关于本机偶发红点的诚实说明**：本片第一次跑一键 CI 时，`test (runner)` 红在 `prepared_program_reruns_are_isolated_and_survive_a_trap` 的 warm median 预算断言（23.5 ms vs 20 ms 预算）——与 STEP-0499 §8 记录的是**同一个**已登记的环境敏感断言。**本片的新钉不参与其中**：它是**纯 Rust** 测试（只调 `compile_script_program` 与 wasmparser，**不跑 guest、不产生 2 GiB 线性内存预留**），因此**不增加** guest 实例冲刷量；失败二进制的源码与 `HEAD` 逐位相同。权威裁决仍是独立 CI。
>
> **重跑结果：全绿（12/12，exit 0）** —— 在机器空闲时重跑 `tools/run-ci.ps1`，`test (runner)` 通过，整套 12 步全 PASS。这与 STEP-0499 §8 记录的红点归属一致：该断言的红点是**本机负载/页池压力**相关，不是本片引入的代码回归；**预算常量与该断言均未改动**。

## 8. 诚实性

- **本片只测量与登记，没有实现**。guest 侧**仍然不会**发射 run 体字节；STEP-0487 的 `--emit-component-hex` 仍是**冻结的最小 echo 骨架**（run 体是硬编码 hex 常量），未接到计划上。
- 真值只覆盖 **Script profile 的参考形态**（`compile_script_program` 只接受 Script v0 入口签名，其它形态 oracle 根本不发射组件，故无法取真值）。§4 的规则表是从**这一种形态**解出来的，**不宣称**已覆盖一般形态。
- §4.2/§4.3 的规则来自 oracle 的**源码 + 实测字节**两侧对照；其中"`variant` 的 error 臂右对齐"、"清零未激活臂"、"construct 按 op 的 fields 顺序"三条**只有源码依据**（参考形态不触发），下一片实现前应各自再取一次真值。
- §4.4 的 `ScriptError` 结果区字段偏移（`code`@0 / `message`@4）是**由冻结 WIT 派生**的，**未实测**（参考形态不触发 error 臂）；其余四项**均已实测**。
- 本片**没有**触碰 37 语料阶梯、`.sapp` 链或 ADR-0015。**门 3 仍为 NO-GO**。
