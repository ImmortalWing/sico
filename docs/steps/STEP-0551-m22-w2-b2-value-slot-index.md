# STEP-0551 m22-w2 **B2 值索引片**：`cgs_value_slot` 由逐指令重扫改为每函数一次索引 ⇒ 回绿 **4 / 364 = 1.10%**（全部来自既有 140 个 `FuelExhausted`，P2 新开测的 195 个 **0 回绿**）⇒ **B2 是正确但不是主体的刀，下一刀 B3**

| 项 | 值 |
|---|---|
| 片型 | B2 实现 + 测量：`selfhost/codegen.sico`（每函数一次 stride-2 值索引、`cgs_value_slot`/`cgs_value_ty` 改 O(1) 查表、两个发射根的数组遍历改游标、canonical id 前置门）、`runner/sico-runner/tests/selfhost_compiler.rs`（新拒绝钉、X 族夹具改密集 id、ledger 路径燃料探针）、`selfhost/match-component-coverage-v1.json`（4 条 verdict + MS-002 行钉）；`crates/`、`wit/`、其它 `selfhost/*.sico` **零改动** |
| exit | **部分达成**：回绿 **4 站点**（`parser.sico:fixed_literal_const_segment`，MS-081…084），既有 **32 个全等站点逐字节不变**（29 个全等组 `component_sha256` 全同、0 漂移），**无站点退化**；但 195 个 P2 新开测站点 **0 回绿** |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**；组件轴 32 → **36 / 396** |
| 证据等级 | `internal-fixture`：本机 2026-10-28，Windows x64，debug 测试构建，`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB / hostcall 8 GiB） |

## 1. 要回答的问题（B2 的墙有多厚）

P2 判 **D2 = 结构性**后，主路径被限定为 B2：为函数级「值 id → 槽号」建一次索引，把 `cgs_value_slot` 的逐指令重扫从 O(n) 降到 O(1)、每函数总量降到 O(n)。

B2 的 exit 不是「墙倒」，而是两条可判定的量：

1. **195 个 P2 新开测站点**（原 `closure-too-large`）里**可回收的部分回绿**；
2. **既有 32 个全等站点逐字节不变**。

本片回答：**只把查槽这一项修成 O(1)，能回绿多少？** 这个数决定下一刀落在索引之外的重扫，还是落在索引本身的形状。

## 2. 方法

- **索引形状**：`cgs_value_index(bytes, run) returns List[U64]`，步长 2。条目 `2k` = 值 id `k` 的 1-based 首个 Wasm 槽号，条目 `2k+1` = 该值类型在 IR 文本里的偏移。槽半 0 表示「unit / 不存在 / 函数在此值或之前停止扁平化」；类型半始终完整。
- **语义等价**：poison 规则逐条复刻旧重扫 —— 参数宽 0 ⇒ 该值与之后全部槽号 0 并停止；指令宽 0 且 unit ⇒ 0 并继续；指令宽 0 非 unit ⇒ 从该值起全部 0。旧实现返回 0 的三类情形（不存在、unit、poison）在新实现里读数一致。
- **canonical 门（B2 引入的新拒绝面）**：`cgs_value_ids_refusal` 要求参数 id 为 `0..p-1`、指令 `result` 为 `p..` 且按定义序密集递增 —— 这正是 Rust verifier 的 `NonCanonicalId` 规则。索引按 id 定位，稀疏 / 重排 / 前导零会读到别的值的条目，因此 fail-closed 返回 `ERR:E-SH-CG-IR-EMIT`，而不是「查不到再兜底」。`cgs_value_id_canonical` 同时补了溢出保护（`cgs_decimal_u64` 溢出返回 0，所以只有字面 `"0"` 可以携带 0）。
- **游标**：新增 `cgj_array_first` / `cgj_array_next`，取代 `cgs_function_instr_hex` 内两处 `cgj_array_item(..., i)` 的从头重扫（块、指令各一处）。
- **线程化**：39 个函数带 `vindex: List[U64]` 形参；40 个 `cgs_value_slot` 调用点与 36 个 `cgs_value_ty` 调用点改传索引；两个发射根（`cgs_function_instr_hex`、`cgs_result_return_hex`）各自在入口调用 `cgs_value_index` 一次。
- **测量**：ledger 路径全量 95 组，与 P2 同一夹具、同一闭包上限（256 fn / 1 MiB）、同一限额。命令：

```
SICO_SELFHOST_COMPONENT_CACHE=0 \
  runner/sico-runner/target/debug/deps/selfhost_compiler-b475c95ccc0fc974.exe \
  sico_compiler_reproduces_the_frozen_match_site_ledger --test-threads=1
```

- 实测 **6,040.99 s**（约 1 h 41 min，与 P2 的 6,052.86 s 同量级）。原始日志 `target/tmp/b2/b2-ledger-run1.log`。

## 3. 结果

### 3.1 组级直方图（95 组）

| verdict | P2 | B2 | 变化 |
|---|---|---|---|
| `ir-equal+component-equal` | 29 | **30** | +1 |
| `ir-equal+component-other:FuelExhausted` | 63 | **62** | −1 |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 2 | **2** | ±0 |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | 1 | **1** | ±0 |

### 3.2 站点级直方图（396 站点）

| verdict | P2 | B2 |
|---|---|---|
| `ir-equal+component-equal` | 32 | **36** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 5 | **5** |
| `ir-equal+component-other:FuelExhausted` | 335 | **331** |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | 24 | **24** |

逐站转移（396 条逐条比较）：

| 转移 | 站点数 |
|---|---|
| `component-equal → component-equal` | 32 |
| `refused-guest → refused-guest` | 5 |
| `FuelExhausted → FuelExhausted` | 331 |
| `8 MiB trap → 8 MiB trap` | 24 |
| **`FuelExhausted → component-equal`** | **4** |

**回绿 = 4 / 364 = 1.10%。**

### 3.3 回绿的 4 个站点全部来自「既有 140」，不在 P2 新开测集合里

| 站点 | 路径 | 函数 | 行 |
|---|---|---|---|
| MS-081…MS-084 | `parser.sico` | `fixed_literal_const_segment` | 4577 / 4590 / 4622 / 4629 |

对照 P2 之前的账本：这 4 个站点**在 P2 之前就是 `FuelExhausted`**，属于既有的 140 个；P2 新开测的 195 个（闭包 65 fn ~ 164 fn 的大函数）**一个都没有回绿**。

> 这条是 B2 最重要的读数：索引对**小闭包**足以跨过预算，对**大闭包**不足。同一把刀在小 n 处见效、在大 n 处被别的二次项吃掉。

### 3.4 既有 32 个全等站点逐字节不变

- 29 个 P2 已全等组的 `component_sha256` 与 P2 账本**完全相同**（0 漂移），`ir_sha256` 亦相同。
- 新增的第 30 个全等组即 3.3 的回绿组（4 站点），其 `component_sha256` = `9ab5f5e51e0d0714…`，与 Rust 生产输出一致（verdict 即该比较的结果）。
- 无任何站点从更优 verdict 退化，也没有新的 refused / trap。

### 3.5 控制变量：X 族燃料

同一份**密集 id** 夹具，改前 / 改后（bracket 下限）：

| n | pre-B2 fuel_low | B2 fuel_low | 比值 |
|---|---|---|---|
| 8 | 2.560e9 | 2.048e9 | 1.25× |
| 16 | 3.840e9 | 3.200e9 | 1.20× |
| 32 | 6.912e9 | 5.760e9 | 1.20× |
| 64 | 23.040e9 | 15.360e9 | 1.50× |

n 32 → 64 的隐含指数从 **1.58** 降到 **1.32**：索引拿掉了一部分二次项，但**整体仍是超线性**。日志 `target/tmp/b2/{preb2-x-dense,b2-x-dense}.log`。

### 3.6 账本刷新

`tools/census-m22-match-sites.py --update` 只改了 **6 行**：4 条 verdict（MS-081…084）与 MS-002 的两条行钉（`line 1723 → 1755`、`function_line 1722 → 1754`，由 B2 插入新函数造成）。其余 392 条记录的 verdict / 三个摘要 / 行号**逐条未动**。

## 4. 判读

- **1.10% 远低于 P2 用的 25% 参照线**（B2 的 exit 本身不设阈值，此数只作横向对照）⇒ B2 是**正确的局部刀，但不是墙的主体**。
- 原因可以直接数出来：`selfhost/codegen.sico` 仍有 **88 处 `cgj_array_item`，其中 45 处在循环内** —— 每次取第 i 个元素都从数组头重扫，一次全遍历就是 O(n²)。分布最密的是 `cgp_run_plan`（5）、`cgs_dispatcher_slot`（4）、`cgs_function_local_hex`（3）、`cgs_data_offset`（3）、`cgs_data_hex`（3）、`cgs_instruction_for_value`（2）、`cgs_variant_hex`（2）、`cgs_collection_call_hex`（2）、`cgs_match_condition_hex`（2）。另有一类**每次调用都要走全模块**的查询：`cgs_module_has_intrinsic`、`cgs_function_has_intrinsic`、`cgs_block_has_intrinsic`、`cgs_function_position`、`cgs_run_index`。
- B2 只动了「查槽」与两个发射根的主遍历；上面这些重扫**一条没动**，而它们在每个操作数 / 每条指令上被反复触发 ⇒ 大闭包仍在同一堵墙上。
- 因此下一刀应当是 **B3 = 把剩余的循环内 `cgj_array_item` 全部改成游标，并把「每指令一次的全模块查询」改成每函数一次建立的索引**。这把刀与 B2 同类（机械替换），但覆盖面是 B2 的 5 倍以上。
- 三选一仍然取「继续」，理由与 P2 相同：没有连续零增长实现片的新计数（本片回绿 4 > 0），也没有合同缺口。**但「继续」的含义进一步收窄为 B3**，不再是「继续 B2」。

## 5. 固化与修复

- 账本 396 条由 `tools/census-m22-match-sites.py --update` 合并实测报告；`--check` 复验 `LEDGER_CURRENT sites=396`。
- **新常驻钉** `sico_codegen_guest_refuses_value_ids_outside_the_canonical_dense_sequence`：
  - 密集 canonical id 的 `match_shape_module()` 必须与 `codegen_oracle::compile_script_program` 逐字节等同；
  - 在同一模块最后一个 block 追加一条 `id = 10_000` 的指令（稀疏 id 双胞胎）必须得到 `Refusal("ERR:E-SH-CG-IR-EMIT")`。
  - 该钉把「索引按 id 定位」的安全前提写死：id 不密集时**拒绝**，不允许别名读到别的值的条目。
- **X 族夹具修正**：`x_family_ir` 原来给每个 block 注入 `10_000 + i`（既稀疏又跨 block 重复），只有「按文本扫描」的旧查表才容忍；现在改为在最后一个 block 里追加**密集** id（`max(id)+1 …`）。这是夹具缺陷，不是实现放宽。
- **新增 ledger 路径燃料探针** `the_component_emission_fuel_of_a_real_match_group_through_the_compiler_is_measured`（`#[ignore]`，`SICO_M22_PROBE_GROUPS` / `SICO_M22_PROBE_FUEL`），走 `--emit-component-hex` 全流程、与 ledger verdict 同一路径。之所以要新增：注入路径夹具会让发射器**跑两遍**（`component_refusal_for_ir` 后 `component_hex_for_ir`）且不过前端，数值约为 ledger 的 2×，不能直接与 verdict 对比。

## 6. 验证

| 项 | 结果 |
|---|---|
| `cargo fmt --all --manifest-path runner/sico-runner/Cargo.toml -- --check` | exit 0 |
| `cargo clippy --test selfhost_compiler -- -D warnings` | exit 0（2026-10-30 恢复检查点复跑；修掉 3 处新测试的 clippy 告警） |
| `sico_compiler_reproduces_the_frozen_match_site_ledger`（刷新后复跑） | 未在本检查点复跑；**6,040.99 s 即本片的测量运行** |
| `sico_codegen_guest*` 全部 + 新拒绝钉 | 新拒绝钉 **1/1 通过**（`sico_codegen_guest_refuses_value_ids_outside_the_canonical_dense_sequence`）；其余 guest 用例本轮筛选未跑 |
| `census-m22-match-sites.py --update` / `--check` | `LEDGER_WRITTEN sites=396 pending=0` / `LEDGER_CURRENT sites=396` |
| `every_match_site_is_measured_on_the_component_axis` | 未在本检查点复跑（随 ledger 测量运行） |
| `git diff --check` | exit 0 |
| 全轮电池（15 拍） | **未跑**（B2 不是族收口片，按 §10.1.2 分层不跑整轮） |

## 7. 边界（本片**没有**证明的事）

- **没有翻墙**：组件轴 36 / 396；195 个 P2 新开测站点回绿 **0**。
- canonical id 拒绝是**行为收窄**：旧实现会对非 canonical id 做「尽力扫描」，新实现直接 `ERR:E-SH-CG-IR-EMIT`。合法 S 的 canonical IR 不受影响（Rust verifier 本就拒绝非 canonical id），但这是新增的拒绝面，按 AGENTS §10.2 记在此处。
- 只覆盖 `U64.checked_add` 头的 396 个 `match` 站点；T2/T3–T20 的其余形状面不在本片口径内。
- 索引只覆盖当前冻结 S 的形状；S 变更后 `cgs_value_ids_refusal` 若触发，ledger 会以 `ERR:E-SH-CG-IR-EMIT` 显式记录，不会静默回退。
- 8 MiB trap（`parser.sico:scalar_ir`，24 站点）与 B2 正交，未动。
- 未测 1 MiB 组件；256 KiB 仍是实测上限（STEP-0549）。
- **词表工件待刷新**：B2 改了 `selfhost/codegen.sico`，`selfhost/emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` 因此过期；本机无可用 Python，未运行 `census-m22-emitter-vocabulary.py --update`。恢复 M22 时须先刷新，否则电池第 13 拍会红。
- **本片未推送**：`codex/m22-w2` 仍领先 origin，推送需 owner 授权。
- **2026-10-30 检查点**：owner 决定暂停自举、优先视觉与原生 UI；本片作为暂停前的最后一个 M22 提交落库，避免工作区丢失。

## 8. 下一步

1. **B3（B 路遍历策略）**：把剩余 **45 处循环内 `cgj_array_item`** 改游标（`cgj_array_first` / `cgj_array_next`），并把 `cgs_module_has_intrinsic` / `cgs_function_has_intrinsic` / `cgs_block_has_intrinsic` / `cgs_function_position` / `cgs_run_index` 这类「每次调用全模块走」改成每函数一次建立的索引。exit = **195 个 P2 新开测站点的回绿数单列** + 既有 36 个全等站点逐字节不变。
2. **8 MiB 残差单独裁决**：先量 `scalar_ir` 的 IR 文本真实大小与构成，再决定子输出 / 流式校验，或登记为门 3 之外的测量限制；**不得直接上调结果上限**。
3. **5 个 refused-guest 站点**：按 P3 具名残差逐族处理。
4. **`sico.list.length[U64]`**：340 站点口径下的唯一词表缺口（7 个 `general_while_function_ir` 站点），按四道闸门补 helper；它不改变 D2，但必须单独出账。
