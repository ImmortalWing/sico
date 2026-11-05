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

### 10.4 根因：发射器的文本累加拼接是平方级

发射器拼组件字节用 `set out = sico.text.concat(out, frag)`（`selfhost/codegen.sico` 457 处，典型如 `cgs_write_local_hex` 循环）。guest 的 `sico.text.concat` 后端实现 `emit_concat`（`crates/sico-codegen-wasm/src/stdlib.rs:147`）每次调用**分配 `len(a)+len(b)` 新缓冲、两次 `memory.copy` 把 a 和 b 整个拷入**。

累加式拼接 ⇒ 拼出总长 L 的串共拷 ~L²/2 字节、分配 ~L/2 次 ⇒ **O(L²)**。3-4MB 组件在按操作计的 fuel 下必然耗尽 800e9。这与 §10.3 的平方数据吻合。

**这不是建索引能解的**：索引省重复扫描（B2/B3 的思路），省不了字符串重拷。B4 若要动这堵墙，方向是**累加缓冲改写**（让 `out` 不再每次重拷，如 rope / 分段 append 到固定缓冲），属 guest 前端层（B2/B3 同层，非跨组件架构），但比 B2/B3 重——要引入新的累积结构，且改 guest_emitter sha ⇒ 词表 + ledger 失效需重测。

### 10.5 B4 候选裁决（拆账后）

| B4 候选 | 裁决 |
|---|---|
| `sico.list.length[U64]` 词表缺口（7 站点） | over-report（guest 已有内联 `cgs_list_length_hex`:5643）；7 站点本就 fuel-blocked ⇒ 补是死代码，不回绿 |
| `cgs_data_offset` 索引化（:2896，O(字面量数²)） | 目标组 `general_while_function_ir` 的 IR 里 const_string 计数 = 0，触达不了这批站点；331 站点的 const_string 分布未测 |
| `cgs_dispatcher_slot` 索引化（:2199） | 只在**每函数**层调 1 次（非每指令），非平方热点，杠杆低于初判 |
| **文本累加拼接平方（§10.4）** | **头号根因**，覆盖 331 FuelExhausted；修法 = 累加缓冲改写，非建索引 |
| 8 MiB trap / refused-guest / 4 既有红 | 与 fuel 正交，各自独立 |

### 10.6 B4 若开工的最小刀（未开工，等 owner）

先写**最小复现**量「拼一个 L 字节串实际烧多少 fuel vs L 的曲线」，确认平方（秒级、零风险）；确认后再做累加缓冲改写，用 emitter-only 探针看 `general_while_function_ir` 的 fuel bracket 是否下降（秒级），下降明显才值得烧 2h ledger 测量。

Open objectives: M22 自举 B3（用户 m00001）B3 负结果已归档 + B4 拆账已钉死根因（文本累加平方），B4 改写未开工，等 owner 对累加缓冲改写口径。
