# STEP-0611 · M22 · op 级燃料归因（op-level fuel profiling）：把燃料墙从「具名超线性算子」进一步钉死到具名 op —— `cgj_value_end` JSON 逐字节扫描，由 intrinsic-membership 三级重扫驱动

> **片型** = 测量片（measurement slice）。本片**落地一个加法式 runner 钩 + 三个 `#[ignore]` 测量探针**，产出 op 级归因数据；**不改 `selfhost/**`、不改任何冻结语料/账本、不上调任何限额、不翻任何门**；M22 整体仍 **NO-GO**，未提交未推送。
>
> **owner 授权依据**：D-A（2026-11-08，AGENTS §10.1.4 addendum）——op 级燃料仪器化剖析已授权。**gcc 授权未行使**（见 §6.5：本片用 wasmtime 既有 backtrace 捕获，无需 C 编译）。
>
> **改动面**：`runner/sico-runner/src/lib.rs`（**加法式钩**：`FuelTrace::exhaustion: Vec<FuelOpFrame>` + `RawEngineFrame.function_name` + 在燃料 trap 时捕获调用栈）、`runner/sico-runner/tests/m22_fuel_profile.rs`（**新增 3 个 `#[ignore]` 探针** + 1 个 trace-return 辅助）。原始日志在 `target/tmp/step0611/{real-site-ladder,operator-locations,name-resolution}.log`。
>
> **复现路径**：探针 = `runner/sico-runner/tests/m22_fuel_profile.rs`（`the_real_site_fuel_exhaustion_ops_are_attributed_by_ladder`、`the_named_operators_are_attributed_to_op_locations`、`the_exhaustion_offsets_are_resolved_to_sico_function_names`）；`--exact` 亲跑，`SICO_M22_PROBE_FUEL` 默认 = 冻结资格上限 `800_000_000_000`（未上调）。

## 0. 结论摘要（一句话）

**M22 燃料墙的主导 op 已被钉死**：燃料烧在 **`cgj_value_end`（codegen.sico:675-702）的 JSON 逐字节 `while` 循环**（每字节一次 `sico.bytes.at` hostcall），由 **intrinsic-membership 三级重扫**（`cgs_module_has_intrinsic`:3221 → `cgs_function_has_intrinsic`:3192 → `cgs_block_has_intrinsic`:3167 → `cgs_instruction_is_intrinsic`:3141，被 `cgs_helper_index`:3255 对每个词表 helper 各调一次）驱动 ⇒ **下一个实现刀 = 把「每次查询重扫整份 IR JSON」换成「每模块一次 O(N) 单遍预计算 + O(1) 查询」**（intrinsic-membership 索引/位掩码；STEP-0551 B2 值索引的直接延续，但本轮 op 级数据证明主体是 intrinsic 查询而非 value-id→槽号）。**未翻任何门，M22 overall 仍 NO-GO。**

## 1. 背景：为什么做 op 级剖析

STEP-0606 把燃料墙定位到**两个具名超线性发射器算子**（match-dispatcher 重扫 + `text_concat` 装配），但只通过**括号中点燃料**（`run_with_fuel`，±12.5% 带）——没有 op 级来源。STEP-0608 证明**燃烧率平坦（~21M fuel/ms）**，即燃料是确定性的逐 wasm-op 计数，但「**没有 opcode 钩**」——`FuelTrace::timeline`（`run_with_fuel_timeline`）是**逐 watchdog tick（5 ms）epoch 采样**，只能给出**相位/燃烧率形状**，**不能命名 op**。STEP-0572 曾判 op 级剖析需要 gcc 与 runner 生产钩。

本片解决这两个阻塞：**不需要 gcc**（见 §6.5），且**runner 钩是加法式的最小改动**（`FuelTrace::exhaustion`）。

## 2. 能力评估：`run_with_fuel_timeline` 采样粒度是否足够

**不足。** `run_with_fuel_timeline` 的采样 = epoch-deadline watchdog 每 `TICK=5 ms` 记录一次 `ctx.get_fuel()`（`lib.rs:2606-2627`）。这是**相位级**（燃料-时间曲线），不是 **epoch/opcode 级**——一个 5 ms tick 跨越数百万个 wasm op，不能把燃料归因到具体 op。STEP-0608 已如实登记「no per-opcode hook exists in the public API」。

**本片补齐的 op 级机制**：因为 guest 执行是**确定性的**，**同一输入在燃料预算 k 下每次都在 op #k 耗尽**（同 op 序列）。wasmtime 在燃料耗尽时产生 trap，其 **`WasmBacktrace`**（`config.wasm_backtrace_max_frames(256)`，`lib.rs:1602`，backtrace 默认开启）在**耗尽点**捕获调用栈——每帧携带 **`func_index` + `module_offset`**（精确的 wasm 指令字节偏移）。因此：**跑一个燃料预算阶梯 k₁<k₂<…，每步读耗尽 trap 的顶帧 = op #k 所在的确切（函数, 指令）**。这就是**无引擎 opcode 钩的 op 级采样**，且**无需 gcc**（无 C 编译、无 cc-rs；只复用既有 backtrace 捕获）。

**runner 改动**（加法式，默认关闭、不影响既有行为、类型化）：
- 新公开类型 `FuelOpFrame { module, function_index, function_name, instruction_offset }`（`lib.rs`，位于 `FuelSample` 后）。
- `FuelTrace` 新增公开字段 `exhaustion: Vec<FuelOpFrame>`（`Default` 为空；仅当 run 以燃料 trap 结束时由 `invoke` 错误路径填充，顶帧 = 最内层耗尽 op）。既有两个 `FuelTrace` 字面量补 `exhaustion: Vec::new()`（成功路径）/填充（错误路径）；其余构造点经 `Default` 不变。
- `RawEngineFrame` 新增 `function_name`（`frame.func_name()`；本 guest 无 name section ⇒ 恒 `None`，见 §3.2）。
- **行为不变**：plain `run`/`run_observed` 不读燃料；`run_with_fuel`/`run_with_fuel_timeline` 只多带一个空/填充字段；无限额变化。

## 3. 证据

### 3.1 真实站点阶梯：燃料集中在一个函数、一个指令偏移

`parser.sico:fixed_literal_const_segment`（MS-081 组，ir 92057 B，emit-only，`SICO_M22_PROBE_FUEL` 未设 = 800e9）：几何阶梯 100M → 800e9 共 **14 样本，全 FuelExhausted，14/14 携带精确 `instruction_offset`**。顶帧直方图：

| 顶帧函数（= wasm func#） | 样本数 | 指令偏移示例 |
|---|---|---|
| `cgj_value_end`（#44） | **7/14** | **13549（重复）、13726、13333** |
| `cg_next`（#37） | 4/14 | 11485（重复） |
| `cg_list_at`（#38） | 2/14 | 11539 |
| `cgj_container_end`（#43） | 1/14 | 12998 |

**关键读数**：`cgj_value_end` 的同一指令偏移 **13549 在多个预算样本重复出现** ⇒ 燃料在**一个紧密循环**里烧（逐字节 JSON 扫描）。调用栈一致为 `cgj_value_end ← cgj_span_text ← cgj_array_first ← cgs_*`（`cgs_instruction_is_intrinsic`#111 / `cgs_module_has_intrinsic`#114 / `cgs_function_has_intrinsic`#113 / `cgs_helper_index`#115 / `cgs_arena_base`#167 / `cgs_hex_bytes`#106）。

### 3.2 名字解析：wasm func index == Sico 声明序（精确验证）

guest 模块**无 wasm name section**（codegen 不发射 name），`Frame::func_name()` 恒 `None` ⇒ 用 `wasmparser` 解析 injection 组件 core module 的 function/code section 布局，把耗尽 `module_offset` 映射到函数体范围。**验证**：Sico 声明序索引与 wasm defined-function 索引**逐一对上**（5/5 抽样全等：`cg_byte_is_space`#39 body[11336..11590]、`cgj_container_end`#43 body[12570..13019]、`cgj_value_end`#44 body[13021..14300]、`cgj_value_text`#46 body[14979..15516]、`cgj_array_next`#52 body[19960..20145]）。core module 有 **218 defined vs 201 Sico 函数**（17 个适配器 shim 在 0..200 区间外），故 1:1 映射在 0..200 内成立。

### 3.3 合成算子族：三者都汇聚到 `cgj_value_end`

| 族 | 尺寸 | 预算 | 结果 | 耗尽顶帧栈 |
|---|---|---|---|---|
| `match_flat` | n=16 | 400e9 | FuelExhausted | `cg_next@11485 ← cgj_container_end@13188 ← cgj_value_end@13726 ← cgj_span_text@15000 ← cgj_array_first@20027` |
| `text_concat` | n=64 | 800e9 | FuelExhausted | `cgj_value_end@13726 ← cgj_span_text@15000 ← cgj_array_first@20027 ← cgs_function_has_intrinsic@83630` |
| `match_flat` | n=8 | 800e9 | **emitted** | —（consumed 113.5e9） |
| `match_flat` | n=16 | 800e9 | **emitted** | —（consumed **748.5e9**） |
| `text_concat` | n=32 | 800e9 | **emitted** | —（consumed 527.9e9） |

**match 族与 concat 族在 sub-flip 预算下都在 `cgj_value_end` 耗尽** ⇒ 两个 STEP-0606 具名算子的**共同底层 = `cgj_value_end` JSON 逐字节扫描**（经 `cgs_function_has_intrinsic`/`cgs_instruction_is_intrinsic` 查询路径）。

**口径调和**：STEP-0606 记 `match_flat n=16 EXHAUSTED(>800e9)` 是**其 25.6e9 探针上限**下的读数；本片 800e9 下 n=16 **emitted**（consumed 748.5e9 < 800e9）。**不矛盾**——是探针上限不同；「二次增长」定性不变。

### 3.4 根因定位（源码级）

`cgs_helper_index`（codegen.sico:3255）对**每个词表 helper** 调一次 `cgs_module_has_intrinsic`（:3221）算 rank；`cgs_module_has_intrinsic` 循环**所有函数**→每函数 `cgs_function_has_intrinsic`（:3192）循环**所有块**→每块 `cgs_block_has_intrinsic`（:3167）循环**所有指令**→每指令 `cgs_instruction_is_intrinsic`（:3141）读 op/name（`cgj_member`/`cgj_ident_text`→`cgj_span_text`→**`cgj_value_end`**:675，其 `while` 每字节一次 `sico.bytes.at` hostcall）。⇒ **总燃料 ≈ O(H_used × H_vocab × F × B × I)**（H=helper、F/B/I=函数/块/指令），热 op = `cgj_value_end` 的逐字节 `sico.bytes.at`。

## 4. 下一个实现刀（可执行建议）

**主体**：`codegen.sico` 的 intrinsic-membership 查询族（`cgs_module_has_intrinsic`/`cgs_function_has_intrinsic`/`cgs_block_has_intrinsic`/`cgs_instruction_is_intrinsic`/`cgs_helper_index`）在**每次查询时重扫整份冻结 IR JSON**（通过 `cgj_*` 扫描器，热叶 = `cgj_value_end`）。

**修什么能降复杂度**：把「每次查询重扫」换成「**每模块一次 O(N) 单遍预计算**」——一次遍历所有指令，把每个函数/块/模块含哪些 intrinsic 预计算成紧凑索引/位掩码（`List[U64]`），使 `cgs_*_has_intrinsic` 变成 O(1) 读。把 O(H_used × H_vocab × F × B × I) 降为 O(N + H²)。这是 **STEP-0551 B2 值索引的直接延续**（同样「一次建索引、查询 O(1)」），但本轮 op 级数据证明**主体是 intrinsic 查询族，不是 value-id→槽号**（B2 只回绿 4/364）。

**次要杠杆**：`cgj_value_end` 的逐字节 `sico.bytes.at` hostcall 每字节一次——若改用**批量 bytes 访问**（一次 hostcall 读一段 span，如既有 `sico.bytes.slice` 类原语），把逐字节 hostcall 换成批量读，进一步降低常数。但**主杠杆是消除重扫**（与 STEP-0548 装配策略、STEP-0600「上调限额买不回站点」、STEP-0606「真正杠杆=降复杂度」三者收敛）。

## 5. 与 STEP-0606 的调和（收窄，不推翻）

STEP-0606 定位到「match-dispatcher 重扫 + `text_concat` 装配」两个**算子级**二次项。本片把**共同底层 op** 钉死：两者（以及真实站点）的燃料都汇聚到 `cgj_value_end` JSON 逐字节扫描（经 intrinsic-membership 查询路径）。这不是推翻 STEP-0606——STEP-0606 的**算子标度**（match_flat 二次、text_concat 二次）仍然成立；本片把「算子」进一步**下钻到「op」**：两个算子之所以二次，是因为它们都反复调用同一个 O(IR) 的 JSON 重扫。

## 6. 诚实收窄（必须记录的口径边界）

1. **op 级阶梯只在 1 个真实站点**（`fixed_literal_const_segment`，MS-081 组）+ 2 个合成族；**非**完整 396 站点账本、**非** live guest-前端降级。
2. **逐预算单点采样**：每样本只给**耗尽点**的 1 个 op（该预算下的 op #k）；顶帧直方图是**稀疏抽样估计**（14 样本中 7 个落 `cgj_value_end`），**非**全 op 计数直方图。
3. **名字解析**：guest 无 wasm name section，`function_name` 恒 None；名字由 `wasmparser` code-section 布局解析（Sico 声明序 == wasm defined 索引，0..200 内，5/5 抽样全等验证）。**未解析** 201..217 的 17 个 shim；`module_offset` 是 core module 内偏移，跨模块归属未单独钉（模块名 None）。
4. **STEP-0606 的 `match_flat n=16 EXHAUSTED`** 是其 **25.6e9** 探针上限；本片 800e9 下 emitted（748.5e9）。口径调和见 §3.3。
5. **gcc 未安装、未需要**：op 级剖析用 wasmtime **既有** backtrace 捕获（`wasm_backtrace_max_frames(256)` 已设、backtrace 默认开启），无 C 编译、无 cc-rs 调用。STEP-0572 §1 的「需要 gcc」判定在本方法下**不成立**（那只阻塞 `cargo check --tests` 的 build-dir 哈希；`cargo build`/`cargo test` 本机可用并已用）。D-A 的 gcc 授权**未行使**。
6. **验证范围**：`fmt --all --check` 净、`clippy --test m22_fuel_profile -- -D warnings` 净（修 1 处 `manual_flatten`）、默认 3 测全绿（`the_fuel_bracket_brackets_a_known_threshold`/`a_real_fuel_exhausted_site_is_bracketed_emit_only`/`the_anchor_site_bracket_is_anchored_on_measured_consumption`）、3 新探针 `--exact` 亲跑。**未跑**全量 runner 套件/run-ci/电池。
7. **未改** `selfhost/parser.sico`/`codegen.sico`/任何冻结语料/账本；`match-component-coverage-v1.json`/`emitter-vocabulary-v1.json` 未触碰；**未上调任何燃料/内存限额**。
8. **并发注意**：STEP-0612（W3-a multi-block-`run` scoping）的 agent 与本片**并发**，已改 `docs/steps/README.md`/`STEP-0610`/`handoff-m22.md` 并新建 `STEP-0612`；本片只取 **STEP-0611**（不冲突）、只追加本 STEP 行 + `docs/STATUS.md` dated 条目，**不覆盖** STEP-0612 内容。
9. **未翻任何门**，M22 overall 仍 **NO-GO**；未提交未推送。

## 7. 原始日志

`target/tmp/step0611/real-site-ladder.log`、`target/tmp/step0611/operator-locations.log`、`target/tmp/step0611/name-resolution.log`。
