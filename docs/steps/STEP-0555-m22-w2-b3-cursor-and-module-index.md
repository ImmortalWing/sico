# STEP-0555 m22-w2 **B3 负结果：游标化与模块索引未回绿，测量证伪核心假设**（回退）

| 项 | 值 |
|---|---|
| 片型 | **负结果 + 回退**：B3 假设「游标化省 fuel ⇒ 更多站点回绿」被 ledger 测量（7454s）证伪。`selfhost/codegen.sico` + 两 census JSON 已回退到 STEP-0551 基线；`codegen_stub` 的 `List[U64]` arm 保留（前向兼容无害）；B3 工具 + 本片证据留档 |
| exit | **否**：FuelExhausted 331→**344**（+13），component-equal 36→**23**（−13）。**0 站点回绿，13 站点回退** |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**（无变化） |
| 证据等级 | `internal-fixture`：本机 2026-11-04，Windows x64，debug 测试构建，`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB / hostcall 8 GiB） |

## 1. 要回答的问题（B3 的墙有多厚）

STEP-0551 §4 数出：`selfhost/codegen.sico` 有 **88 处 `cgj_array_item`，其中 45 处在循环内**（每次取第 i 个元素从数组头重扫，O(n²)），另有一类「每次调用全模块走」的查询（`cgs_module_has_intrinsic`/`cgs_function_has_intrinsic`/`cgs_block_has_intrinsic`/`cgs_function_position`/`cgs_run_index`）。

B3 exit = STEP-0551 §8 定义的两条可判定量：

1. **195 个 P2 新开测站点**（原 `closure-too-large`）里可回收的部分回绿数单列；
2. **既有 36 个全等站点逐字节不变**。

本片假设：**把遍历策略（游标化）与全模块查询（模块索引）修掉，能回绿多少。**

## 2. 方法（已实施，后被测量证伪）

### 2.1 游标化（45 处循环内 `cgj_array_item` → `cgj_array_first`/`cgj_array_next`）

变换由 `tools/_b3_cursorize.py` 完成（APPLIED sites=44 edits=133 + 1 处双游标特殊 `cgs_match_condition_hex`）。三形态：

- **A-continue**：零检查后 `set X_more = 0; continue`（如 `cgp_id_list`、`cgp_field_list`）；
- **B-else**：零检查后走 else 分支（如 `cgs_dispatcher_slot`、`cgs_data_offset`、`cgs_data_hex`）；
- **C-count**：`while U64.less_than(idx, count)` 计数循环，保留 idx 递增并**追加**游标推进。

原语定义（codegen.sico 847/878/889）：

```
function cgj_array_item(bytes: Bytes, arr: U64, index: U64) returns U64   # 直接索引
function cgj_array_first(bytes: Bytes, arr: U64) returns U64              # 游标起点，空数组 0
function cgj_array_next(bytes: Bytes, item: U64) returns U64              # 推进游标，末尾 0
```

### 2.2 模块索引（`cgs_module_meta` = `List[U64]` 13 项）

- `entry0` = count；`entry1..12` = 12 个 helper 存在位（字典序 = BTreeSet rank 序 ⇒ helper 编号稳定）；
- 新增 `cgs_meta_count(meta)`/`cgs_meta_flag(meta, idx)` 读索引；
- `cgs_helper_index` 改读 meta flag（不再逐次全模块走）；15 函数 threading 末尾加 `meta: List[U64]`。

### 2.3 `cgs_function_position` 恢复

恢复按 id **文本**遍历（签名 `(bytes, id)`，canonical 数字换算是错的 —— call target 是函数名）。游标化该遍历。

### 2.4 run-only driver arity

`cgs_run_instr_hex`/`cgs_run_body_hex`/`cg_run_body_hex_from_ir` 恢复单参（compiler.sico 侧只 1 参调用），体内自建 `sico.list.empty[U64]()`。

### 2.5 `codegen_stub` 补 `List[U64]`

`runner/sico-runner/tests/selfhost_compiler.rs:codegen_stub`(3404) 补：

```rust
"List[U64]" => "sico.list.empty[U64]()".to_string(),
```

使 step0509/0513/0521/0522/0535 由红转绿（guest 前端识别 `List[U64]` 类型）。**保留**（回退后仍适用）。

## 3. 测量（7454s，2h4m）

命令：`SICO_SELFHOST_COMPONENT_CACHE=0 CARGO_PROFILE_TEST_DEBUG=0 runner/.../selfhost_compiler-3ee9bc92fdb5f988.exe sico_compiler_reproduces_the_frozen_match_site_ledger --test-threads=1 2>&1 | tee target/tmp/b3/b3-ledger-run3.log`

**结果：FAILED（预期收数时机）**——测试把**现测现 B3 代码**的 outcome 断言到**嵌入旧 ledger**，新 verdict 在 `%TEMP%/sico-m22-match-coverage/actual.json`（测试内部 flush）。

### 3.1 verdict 分布（站点级，396 总）

| verdict | 基线（STEP-0551） | B3 实测 | Δ |
|---|---:|---:|---:|
| `ir-equal+component-equal` | 36 | **23** | **−13** |
| `ir-equal+component-other:FuelExhausted` | 331 | **344** | **+13** |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | 24 | 24 | 0 |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 5 | 5 | 0 |

**B3_GREEN = 0**。**0 站点回绿，13 站点回退**。STEP-0551 §8 定义的两条可判定量均**未达成**：

1. 195 个 P2 新开测站点回绿数：**0**（FuelExhausted 反而 +13）；
2. 既有 36 个全等站点逐字节不变：**否，13 个回退**（23 个仍 equal）。

### 3.2 13 个回退站点（`equal → FuelExhausted`）

```
MS-019  parser.sico:gw_intrinsic_call_packed
MS-021  parser.sico:fixed_literal_operand_json
MS-022  parser.sico:fixed_literal_operand_json
MS-023  parser.sico:fixed_operation_operand_width
MS-037  parser.sico:operand_cell_or_literal_json
MS-038  parser.sico:fixed_operation_operand_atom_idx
MS-063  parser.sico:argument_instruction_count
MS-074  parser.sico:argument_region_instruction_count
MS-081  parser.sico:fixed_literal_const_segment
MS-082  parser.sico:fixed_literal_const_segment
MS-083  parser.sico:fixed_literal_const_segment
MS-084  parser.sico:fixed_literal_const_segment
MS-363  parser.sico:fixed_operation_operand_value
```

## 4. 根因（结构性）

> **§10.1 订正**：本节把 FuelExhausted 归因到「hostcall fuel」，口径不准确——真实是 `consume_fuel(true)` 按**每条 wasm 指令操作**计。B3 否决的结论（游标化净负）仍成立（游标多一次 hostcall ⇒ 多一批指令操作 ⇒ 更耗），但根因表述以 §10 为准。

FuelExhausted 来自 **Wasmtime hostcall fuel**（`runner/sico-runner/src/lib.rs`），不是代码行数或遍历次数。

- **`cgj_array_item`**：**1 个** hostcall（直接索引 `arr[i]`）。
- **游标形式**：**2 个** hostcall/元素（`array_first` + `array_next`）。

⇒ 游标化**增加**每元素 fuel，不减少。原假设（「O(n²) → O(n)」在 fuel 计量下不成立——fuel 按 hostcall 计，不按算法复杂度）。

**模块索引**只帮了 codegen 发射站点（helper_index 查询），而 331 FuelExhausted 站点主要是 `parser.sico:scalar_ir`（**parser** IR 构造，从不调 codegen helper）——meta win 无法触达 FuelExhausted 桶。游标化却在 13 个边界站点上把它们推过 budget。

**结论**：游标化在 fuel 计量下是**净负优化**。STEP-0551 §8 的 exit oracle 否决了 B3。

## 5. 回退与留档

- `git checkout HEAD -- selfhost/codegen.sico selfhost/emitter-vocabulary-v1.json selfhost/match-component-coverage-v1.json`：三文件回 STEP-0551 基线（`git diff` 空，vocab/ledger sha 回 HEAD）。
- `runner/sico-runner/tests/selfhost_compiler.rs`：保留 `codegen_stub` 的 `List[U64]` arm（前向兼容无害，后续 B4 可用）。
- **保留留档**：`tools/_b3_cursorize.py`、`tools/_b3_meta_thread.py`、`tools/_b3_stub_patch.py`（B3 工具）、本 STEP。
- **不保留**：B3 代码改动（codegen.sico/census JSON 已回退）。

## 6. 验证

| 测试 | 结果 |
|---|---|
| codegen_guest（28 站点） | 基线已绿（B3 前已过） |
| call 发射 + helper_index + keeps_modules | 4/4（B3 前已过） |
| `every_match_site_is_measured_on_the_component_axis` | 静态断言通过（ledger 396 站点无 skip/空 digest/空 verdict） |
| `sico_compiler_reproduces_the_frozen_match_site_ledger` | **FAILED**（B3 实测 vs 旧 ledger：13 回退站点）——这是**负结果证据**，不是回归 |
| 全套件（回退后） | 181 passed，4 failed（step0507/0519/0520/0526，CALL-SHAPE 前端既有红，非 B3） |

## 7. 下一步（B4 方向）

**不再走游标化**（fuel 计量否决）。B4 应转向：

1. **`sico.list.length[U64]` 词表缺口**（7 站点）：补 `sico.list.length` 的 guest 返回分支，让 `sico.list.length[U64](...)` 直达而非靠 length 降级。这能真正减少 FuelExhausted 站点的 hostcall。
2. **8 MiB trap（24 站点，`parser.sico:scalar_ir`）**：需流式 IR 构造或提高 stdout 上限，与 fuel 正交。
3. **5 refused-guest 站点**：需扩 codegen 支持面（非 fuel 问题）。
4. **既有 4 红**（step0507/0519/0520/0526，CALL-SHAPE）：前端问题，与 fuel/trap/refused-guest 均正交。

**残差**：331 FuelExhausted 站点（全 parser.sico）+ 24 8 MiB trap + 5 refused-guest + 4 既有红 + 7 词表缺口。

## 8. 边界（不变）

- 静态断言 `every_match_site_is_measured_on_the_component_axis`：ledger 396 站点无 `closure-too-large` skip、无空 digest/verdict（STEP-0550 P2 exit 达成）。
- FuelExhausted verdict 语义（selfhost_compiler.rs:11975+）：`guest_component_with_fuel` 二分/倍增 budget 直到出 Component 或 exhausted/refused/trap；`component-equal` 站点全持 component_sha256（B3 改动后 ledger 用它断言逐字节不变——**本片因此否决 B3**）。
- fuel/timeout/内存/hostcall 四限额不变。

## 9. 交付

**代码 0 净改动**（B3 代码已回退）。留档：本 STEP（负结果证据）+ B3 工具脚本（供后续参考，不进主路径）。

- `git status`：`M runner/sico-runner/tests/selfhost_compiler.rs`（`codegen_stub` 的 `List[U64]` arm）；`?? docs/steps/STEP-0555-*.md`、`?? tools/_b3_*.py`。
- 不推送（领先 origin，需 owner 授权）。

## 10. B4 拆账（2026-11-05，秒级探针 + 静态读码，不烧 ledger）

owner 同意做瓶颈拆账后，用现成 `#[ignore]` 探针 + 静态读码把「FuelExhausted 的墙在哪」从假设钉成数据。**结论修正了本 STEP §4/§7 的部分归因。**

### 10.1 fuel 计量口径订正

§4 写「fuel 按 hostcall 计」不准确。真实口径是 **Wasmtime `consume_fuel(true)`**（`runner/sico-runner/src/lib.rs:1438`），按**每条 wasm 指令操作**计（hostcall 只是其中一类）。所以「省 fuel」= 「减少总 wasm 操作数」，不只 hostcall。

### 10.2 墙在发射器（emitter），不在前端解析

用 **emitter-only 注入探针**（`the_component_emission_fuel_of_a_real_match_group_is_measured`，把 canonical IR 直接喂发射器，跳过 guest 前端）测 `parser.sico:general_while_function_ir`：

| ceiling | 结果 |
|---|---|
| 512e9 | exhausted |
| 204.8e9 | exhausted |
| **800e9（STEP0544_FUEL_CEILING）** | exhausted，search_ms=180197 |

⇒ 跳过前端仍耗尽 800e9，**墙在发射器本身**，不是 parser/frontend。这修正了 §4「FuelExhausted 主要来自 parser 侧」的归因——parser 只负责造出大 IR，烧钱的是发射器把它变成组件。

### 10.3 fuel 随 IR 大小**平方级**放大（决定性数据）

用注入探针对「能出组件的小站点」和「出不来的大站点」各测最小 fuel bracket，对照：

| 站点 | IR 字节 | 组件字节 | 最小 fuel bracket | fuel/IR字节 |
|---|---:|---:|---|---:|
| `codegen.sico:cgs_add` | 3,457 | 1,161 | `1.0e9…1.2e9` | ~347k |
| `parser.sico:fixed_literal_const_segment` | 92,055 | 0（exhausted） | `>800e9`（耗尽） | >8.7M（下限） |

IR 大 **26.6×**，但 fuel 需求涨 **>660×**（1.2e9 → >800e9）。线性缩放本该只涨 26 倍到 ~3.2e9 就够，实际 800e9 仍不够 ⇒ **超线性（平方）确认**。

### 10.4 根因候选：read_local helper 在 N≥2048 是平方（合成 scaling 探针，2026-11-05）

§10.4 最初把平方归到"文本累加拼接"，本轮用合成探针实测。新探针 `the_emitter_fuel_scaling_in_read_local_count_is_measured`（`runner/sico-runner/tests/selfhost_compiler.rs`，`#[ignore]`，秒级）生成含 N 个 `read_local` 指令的合成 helper（`let n{i} = n{i-1}` 链），bracket 最小 fuel：

**注意**：`cgs_read_local_hex` 每指令同时做 cell_slot 重扫 + 文本累加（见 §10.4a 混淆告警），所以这条曲线是两个候选的**总和**，未单独裁决主导项。实测数据：

| read_locals (N) | 最小 fuel bracket（中点） | 相对上档倍数 | 增长形态 |
|---:|---|---:|---|
| 64 | 200M…400M（300M） | — | 基线 |
| 128 | 200M…400M（300M） | 1.0× | 平 |
| 256 | 200M…400M（300M） | 1.0× | 平（线性开销主导） |
| 512 | 400M…800M（600M） | 2.0× | 线性起步 |
| 1024 | 800M…1600M（1200M） | 2.0× | 线性 |
| 2048 | 3200M…6400M（4800M） | **4.0×** | **平方起步** |
| 4096 | 6400M…12800M（9600M） | **4.0×** | **平方确认** |

判读：N ≤ 256 时每指令线性开销（JSON value_slot 索引查询、const hex 发射）主导，N² 潜伏在下面；N ≥ 2048 时 rescan 成本超越线性地板、**每次翻倍 fuel 烧 4 倍**，O(N²) 确认。这是**每函数内**的平方（rescan 范围 = 当前函数的 JSON 跨度，不是整模块）。诚实边界：真实 parser.sico 函数大多是 10-300 指令，**在平方膝点之下**，所以 331 FuelExhausted 站点的墙**不能只归到 read_local 重扫平方**——单函数平方项在真实 IR 里是小项。§10.3 的「IR 大 26.6× 但 fuel 涨 >660×」主导项仍在 §10.4a 之外（整模块逐函数线性项 × 大常数，或另一未测的平方项），B4 开工前需先测「整函数发射 fuel vs 函数大小」的真实曲线，不能靠外推。

### 10.4a 具体重扫路径（静态读码）+ 混淆告警

- `cgs_read_local_hex`（`selfhost/codegen.sico:3835`）/ `cgs_write_local_hex`（:3805 附近）每指令调 `cgs_cell_slot(bytes, fn, local)`。
- `cgs_cell_slot`（:2283）每次做：①`cgs_dispatcher_slot(bytes, fn)`（:2199，**重扫整个 run 的 params/blocks/instructions/locals**）；②`cgs_cell_slots(bytes, fn)`（:2168，重扫全部 locals）；③再自己扫 locals 定位 index。
- B2 已把 `cgs_value_slot`（按 value-id 查 slot，:2108）索引化（`cgs_value_index`:2041，每 run 一次建表），**但 `cgs_cell_slot` 和 `cgs_dispatcher_slot` 未索引化**，每 read_local/write_local 指令都重跑全 run 扫描 ⇒ 这是一个平方源。
- **混淆告警（本轮诚实边界）**：`cgs_read_local_hex` 在 cell_slot 之外**还做文本累加**（:3856-3857 `set ccrlh_out = sico.text.concat(ccrlh_out, ccrlh_get); set ccrlh_out = sico.text.concat(ccrlh_out, ccrlh_set)`，每 slot 两次 concat、累加到 `ccrlh_out`）。所以探针里的平方**同时含** cell_slot 重扫与文本累加两个候选，本轮探针没能把两者分开。想用 const_int 臂（无 cell_slot、无 per-op concat）做判别，但合成 helper 触发 `unsupported codegen source shape` 拒绝（合成 helper 形状没过发射器 gate），未成功。
- ⇒ **两个候选都未单独裁决**：①cell_slot JSON 重扫（B2 同类索引化刀）；②文本累加拼接（`emit_concat` 后端重拷）。§10.4 探针证明"合成 read_local helper 在 N≥2048 是平方"，但没证明主导项是哪一个。B4 开工前需先补一个能过 gate 的 const_int 判别臂，或直接做整函数发射 fuel vs 函数大小的真实曲线。

### 10.4b 模块级平方：`cgs_function_at_index` → `cgj_array_item`（本轮最强候选，2026-11-05）

**这是本轮的决定性发现**。新探针加了 module-level arm（F 个互不调用的小 helper 函数，每个只有几条 `let` 链、无 read_local、无文本累加），测「发射整模块 vs 函数数 F」的最小 fuel：

| functions (F) | IR 总字节 | 最小 fuel bracket（中点） | 相对上档倍数 | 增长形态 |
|---:|---:|---|---:|---|
| 4 | ~320 | 400M…800M（600M） | — | 基线 |
| 8 | ~640 | 800M…1600M（1200M） | 2.0× | 线性 |
| 16 | ~1280 | 1600M…3200M（2400M） | 2.0× | 线性 |
| 32 | ~2560 | 6400M…12800M（9600M） | **4.0×** | **平方起步** |
| 64 | ~5120 | 12800M…25600M（19200M） | 2.0× | （64→128 才见 4×） |
| 128 | ~10240 | 51200M…102400M（76800M） | **4.0×** | **平方确认** |
| 256 | ~20480 | exhausted（>200e9 probe ceiling） | — | 耗尽 |

判读：每个 helper 只有 ~80 字节 IR、无 read_local、无文本累加，**唯一的跨函数共享成本是 `cgs_core_code_hex` 用 `cgs_function_at_index(bytes, i)` 逐函数发射（:4069 附近），而它 → `cgj_array_item`（:）每次从 functions 数组起点重扫到第 i 个 ⇒ 第 i 个函数重扫 O(i)，整模块 O(F²)**。F=128（仅 10KB IR、256 条指令总量）就烧到 7.7e10，F=256（20KB IR）耗尽 200e9 probe ceiling ⇒ 这是纯模块级平方，且**随模块函数数增长**，正好对上「真实 3-4MB IR 模块（数百函数）耗尽 800e9」。

**与 §10.4a 的关系**：§10.4a 是**每函数内**的平方（rescan 范围 = 单函数 JSON 跨度），真实函数在膝点之下；§10.4b 是**模块级**平方（rescan 范围 = 整个 functions 数组），随函数数增长。真实 parser.sico 函数多、模块大 ⇒ §10.4b 是比 §10.4a 更可能的主导项。§10.3「IR 大 26.6× 但 fuel 涨 >660×」由 §10.4b 解释：IR 翻 26 倍 ≈ 函数数也翻几十倍 ⇒ fuel 按 F² 翻几百到上千倍。

**B4 刀（本轮实施并回退，2026-11-05）**：把 `cgs_core_code_hex` 的函数遍历从「`cgs_function_at_index(bytes, i)` for i in 0..count」改成「`cgj_array_first` + `cgj_array_next` 游标遍历」（`tools/_b4_function_cursorize.py`），`cgs_function_body_hex` 改收函数节点 `fn` 而非 index。改动**语义正确**——28/28 `sico_codegen_guest_*` 组件字节等价测试全过，发射组件逐字节不变。但**fuel 完全没降**：module-level arm 重测 F=16/32/64/128/256 → 2400M / 9600M / 19200M / 76800M / exhausted，与改动前逐档相同。⇒ **负结果**：`cgs_function_at_index`→`cgj_array_item` 的数组重扫平方真实存在，但对总 fuel 的贡献是 **~81K wasm ops vs 单函数固定成本 ~128M**，完全被淹没、可忽略。module-level 平方的主导项**不是**数组重扫，另有其源。已回退 codegen.sico（`git checkout HEAD --`），保留 `tools/_b4_function_cursorize.py` 留档。

**module-level 平方的真实主导项（未裁决）**：探针 fuel 覆盖「guest 前端解析源 + 构 IR + 发射组件」全程，module-level arm 有 F 个函数 ⇒ 解析器要处理 F 个函数。主导项可能是 ①解析器处理 F 函数的平方成本（非发射器层，本片管不了）；②发射器里另一随模块大小增长的平方项（如 `cgs_module_has_intrinsic` 调 ~10 次 × 每次扫整模块、或 helper body 生成随 count 增长）。两者都未裁决，需 emitter-only 的 module-level 注入探针（注入预建多函数 IR，跳过解析）才能把解析器与发射器分开——本轮未做，属下一步。

### 10.4c parse vs emit 分账：主导项在发射器，fuel ~ IR²（本轮钉死，2026-11-05）

**这是本轮第二个决定性发现**，直接回答 §10.4b 留下的问题。给 scaling 探针加了 **emit-only arm**：同一 F 函数模块的 IR 由 Rust oracle `rust_ir()`（宿主侧、不烧 guest fuel）预建，再经 `guest_emission_with_fuel(&ir, fuel)` **免解析发射**。对比 module-level arm（source 路径，付 guest parse+IR+emit 全程）：

| F | IR 字节 | EMIT_ONLY（免解析）fuel | EMITTER_SCALING（含 parse）fuel | 判读 |
|---:|---:|---|---|---|
| 16 | 7405 | 3.2e9…6.4e9（4800M） | 1600M…3200M（2400M） | emit-only **更贵** |
| 32 | 12509 | 12800M…25600M（19200M） | 6400M…12800M（9600M） | emit-only 2× |
| 64 | 22717 | 25600M…51200M（38400M） | 12800M…25600M（19200M） | emit-only 2× |
| 128 | 43468 | 102400M…200000M（**1.5e11**） | 51200M…102400M（76800M） | emit-only 2× |
| 256 | 85324 | exhausted（>200e9） | exhausted | 两者都耗尽 |

**结论 1：主导模块级平方在发射器，不是解析器。** emit-only 免解析仍平方且更贵（每档 2×），⇒ 推翻「主导项是解析器处理 F 函数」的假设。331 FuelExhausted 墙在发射器层，本片（guest 前端 codegen.sico）可触及。

**结论 2：fuel ~ IR²（发射器 fuel 随 IR 字节平方增长）。** emit-only 列：IR 7405→4800M / 12509→19200M / 22717→38400M / 43468→1.5e11 / 85324→exhausted。IR 每 ~2× ⇒ fuel ~4×（线性本该 2×）⇒ **平方**。这解释了 §10.3「`cgs_add` IR 3457B→1.2e9 vs `fixed_literal_const_segment` IR 92055B→>800e9」（IR 大 26.6× 但 fuel 涨 >660×，26²≈676）。

**为什么 emit-only > 含 parse**：注入组件（`injection_component`）走 `cg_run_body_hex_from_ir` 直发路径，可能少一层 run 函数包装；且注入组件与 emit-component-hex 组件的 slot/ABI 常量不同。差异是常数倍（~2×），不改变平方形态 ⇒ 不影响结论。

**B4 刀（本轮钉死目标层，仍未实施改写）**：主导项是发射器里随 IR 总量平方的某项。两个剩余候选（都在 codegen.sico guest 前端层）：
- **候选 C（模块级 hex/join 累加）**：`cgs_core_code_hex` 把 F 个函数 body 经 `sico.list.append` 收集后 `sico.text.join(crih_parts, "")`（:4053 附近）一次性拼接；若 join 后端是反复 concat（每次重拷已累计串），总代价 O(总 IR²)。emit-only 平方且随 IR 总量增长，与此吻合。
- **候选 D（模块级 byte-offset 重算）**：`cgj_member`/`cgj_array_item` 系列每次调用从 JSON 起点用 `sico.bytes.length` + 逐字节 `cgj_ws` 重扫；发射器里每函数/每指令反复查模块级字段 ⇒ O(总 IR²)。

两者需下一个刀分辨：候选 C 改「join/累加缓冲」（一次性重写发射器文本装配），候选 D 改「模块级字段索引化」（B2 同类）。二者都会改 guest_emitter sha ⇒ 词表 + ledger 失效需重测（2h）。

**C-vs-D 判别臂（EMIT_SPLIT，固定总 256 lets 变 F，2026-11-05）**：EMIT_SPLIT 把总 `let` 数固定在 256（F=4→4×64、F=16→16×16、F=64→64×4），只变函数数，per-function 开销（name/locals/return ~60B 各）让 IR 略涨：

| F | IR 字节 | EMIT_SPLIT fuel | 判读 |
|---:|---:|---|---|
| 4 | 3619 | 800M…1600M（1200M） | 基线 |
| 16 | 7433 | 3200M…6400M（4800M） | 4× |
| 64 | 22740 | 25600M…51200M（38400M） | 32× |

判读（**诚实纠错，本节不裁决 C vs D**）：固定总 lets 256 时 IR **并非常数**——per-function 开销（name/locals/return ~60B 各）随 F 涨，IR 3619→22740（6.3×）。fuel 涨 4×/8× 落在 C(total²：4.2×/9.4×) 与 D(F×total：8.2×/18×) 之间，**两候选都不能被这批数据排除**。EMIT_SPLIT 的控制变量失效（想固定 total 但 total 随 F 漂移），⇒ **C vs D 未分主次**。干净的 C-vs-D 需要按固定 total-IR 预算 padding 函数，本轮未做。

**已确证的（不含 C/D 裁决）**：
- §10.4b/c 的 emit-only arm 证明 **fuel ~ IR²（随总 IR 字节平方）**，且主导在发射器层（免解析仍平方）。这条不受 C/D 歧义影响。
- 但 **module-level arm 那批 helper 走 `cgs_value_slot`（O(1) 索引，B2 索引化）不触达 `cgs_cell_slot`**（真 per-function 重扫）⇒ module-level 平方的源**不是** cell_slot 重扫。读码确认：`cgs_function_instr_hex`(:4059) 主循环用 `cgs_value_slot`(:2108，O(1))；只有 read/write 走 `cgs_cell_slot`(:2283→`cgs_dispatcher_slot`:2199 扫全 run + `cgs_cell_slots`:2168 扫全 locals = per-function 平方，但 module-level arm 无 read/write)。
- ⇒ **两个未合拢的平方**：①module-level（无 read/write，随总 IR²，源未定）；②read/write per-function（cell_slot 重扫，真实函数在膝点之下，§10.4a）。331 站点主要 parser.sico:scalar_ir（read/write 密集）⇒ ② 可能对 331 有贡献，但 module-level ① 在无 read/write 的合成探针也平方，说明 ① 独立存在。

**B4 改写门槛（本轮钉死，未实施）**：主导项 C/D 未裁决、module-level ① 的源未定。下一刀（二选一或并行）：
1. **干净 C-vs-D**：按固定 total-IR 预算 padding 函数（如每档补空函数到 IR=20000B），排除 per-function 开销漂移，看固定 IR 下 fuel 是否随 F 涨（D）还是平（C）。
2. **module-level ① 源定位**：在 module-level arm 上做减法（去掉 run_probe 尾部的 helper_0/helper_{F-1} 串联、或去掉某类 section），看 fuel 跳变，锁定随 F 增长的发射器项（候选：`cg_section_hex` 包装每函数、`cgs_module_has_intrinsic` 调 ~10× 各扫全模块、`cg_concat_helper_body_hex(count)` 等 helper body 随 count 参数增长）。

改 codegen.sico 前先落这两刀裁决；改了会动 guest_emitter sha ⇒ 词表+ledger 失效需重测（2h）。

**EMIT_PADDED 补刀（固定 total lets=64 + 空函数 padding，2026-11-05）**：每档用 1 个真实函数（前 64/per lets）+ (64/per - 1) 个空 padding 函数：

| F | IR 字节 | EMIT_PADDED fuel |
|---:|---:|---|
| 8 | 4846 | 1600M…3200M（2400M） |
| 32 | 12489 | 12800M…25600M（19200M） |
| 64 | 22697 | 25600M…51200M（38400M） |

padding 没真正固定 total IR（空函数仍带 ~350B header）⇒ IR 仍涨 4.7×。但和 EMIT_SPLIT 合并算 **fuel/IR² ratio** 得出关键判读：

| 臂 | F=4 | F=16 | F=64 | 趋势 |
|---|---:|---:|---:|---|
| EMIT_SPLIT fuel/IR² | **91.7** | **86.9** | **74.2** | **微降 19%**（16× F 才降 19%）|

ratio 微降而非恒定 ⇒ **C（total²）主导**（若纯 C ratio 恒定；若纯 D，ratio 会随 F 涨），D 小贡献（read/write 站点 per_fn=16 时 D~256 vs C~total²，D 被稀释 4×）。空函数 padding 的 EMIT_PADDED 也印证（F=8/32/64 的 ratio 102→123→75，趋势相同）。

**B4 实施目标（本轮收口）**：两候选都贡献，分工清楚：
- **C（模块级文本累加 ~ total²）**：`cgs_function_local_hex`(:6502 之前) 尾部 `set cgslh_body = sico.text.concat(cgslh_body, cgslh_cell)`、`cgs_read_local_hex`:3859-3860 的 `set ccrlh_out = sico.text.concat(...)` 等循环内累加。改法：收集到 list 再一次 join。
- **D（per-function 模块重扫 O(K²)）**：`cgs_function_local_hex` 扫全 blocks/instructions，`cgj_array_item` 每次从起点重扫到第 i 个 = 每函数 O(K²)；`cgs_cell_slot`(:2283) → `cgs_dispatcher_slot`(:2199) 扫全 run + `cgs_cell_slots`(:2168) 扫全 locals。改法：B2 同类索引化（每函数一次建 cell/dispatcher 索引表）。
- 真实 parser.sico 函数大（100+ 指令、read/write 密集）⇒ **D 在真实站点比合成小函数更显著**，是 331 FuelExhausted 的主要回绿路。合成探针的 C 主导不能代表真实负载。
- B4 改 codegen.sico 会动 guest_emitter sha ⇒ 词表 + ledger 失效需重测（2h）。下一轮可直接开工（目标已钉死：①C 改线性装配 ②D 索引化）。

### 10.5 B4 候选裁决（拆账后）

| B4 候选 | 裁决 |
|---|---|
| `sico.list.length[U64]` 词表缺口（7 站点） | over-report（guest 已有内联 `cgs_list_length_hex`:5643）；7 站点本就 fuel-blocked ⇒ 补是死代码，不回绿 |
| `cgs_data_offset` 索引化（:2896，O(字面量数²)） | 目标组 `general_while_function_ir` 的 IR 里 const_string 计数 = 0，触达不了这批站点；331 站点的 const_string 分布未测 |
| `cgs_dispatcher_slot` 索引化（:2199） | 只在**每函数**层调 1 次（非每指令），非平方热点，杠杆低于初判 |
| **read_local helper 平方（§10.4，cell_slot 重扫 ∥ 文本累加，未分主次）** | 合成探针确认 N≥2048 平方，但真实函数在膝点之下；主导项未裁决 |
| 8 MiB trap / refused-guest / 4 既有红 | 与 fuel 正交，各自独立 |

### 10.6 B4 若开工的最小刀（未开工，等 owner）

**先补裁决**：①做一个能过发射器 gate 的 const_int 判别臂（无 cell_slot、无 per-op concat），若它线性而 read_local 平方 ⇒ 主导项是 cell_slot 重扫，B4 = B2 同类索引化刀（`cgs_cell_slot`/`cgs_dispatcher_slot` 建每函数索引表）；②若 const_int 也平方 ⇒ 主导项是文本累加，B4 = 累加缓冲改写。两者都是 guest 前端层（B2/B3 同层），但都会改 guest_emitter sha ⇒ 词表 + ledger 失效需重测。**当前不开工**，因主导项未裁决、真实函数在平方膝点之下，盲改风险 > 收益。

**已实施并回退的一条刀（负结果，2026-11-05）**：把 `cgs_core_code_hex` 从 `cgs_function_at_index(bytes, i)` 索引遍历改成 `cgj_array_first`/`cgj_array_next` 游标遍历（`tools/_b4_function_cursorize.py`），`cgs_function_body_hex` 改收函数节点。改动语义正确（28/28 `sico_codegen_guest_*` 组件字节等价全过），但 **fuel 逐档不变**（F=256 仍 exhausted）⇒ 数组重扫贡献 ~81K ops 被单函数 ~128M 固定成本淹没，**关闭这条候选**。已 `git checkout HEAD -- selfhost/codegen.sico` 回退，脚本留档。

**下一步（B4 门槛，最短裁决路径）**：module-level 平方真实（F=128→7.7e10、F=256 exhausted）但不来自数组重扫 ⇒ 主导项在「解析器处理 F 函数」或「发射器另一随模块增长项」二者之一。下一刀是 **emitter-only module-level 注入探针**（注入预建多函数 IR，跳过解析），把解析器 vs 发射器的 fuel 分账。那一刀之后才能定 B4 改写目标层。

Open objectives: M22 自举 B3（用户 m00001）B3 已完整闭环；B4 拆账本轮新增 module-level scaling 探针 + 实施并回退 `cgs_function_at_index` 游标化刀（负结果，28/28 字节等价但 fuel 不变），排除数组重扫主导，把候选收敛到「解析器 ∥ 发射器另一随模块增长项」，下一步 emitter-only module-level 注入探针分账。
