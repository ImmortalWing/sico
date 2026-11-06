# STEP-0556 m22-w2 **B4 候选 C 负结果：线性装配（`list.append` + 一次 `join`）未降 fuel**（回退）

| 项 | 值 |
|---|---|
| 片型 | **负结果 + 回退**：B4 候选 C 的假设「循环内文本累加拼接（`set X = sico.text.concat(X, 新片)`）是 331 个 `FuelExhausted` 站点的主要 fuel 墙」被 module-level scaling 探针（`the_emitter_fuel_scaling_in_read_local_count_is_measured`）实测**证伪**。`selfhost/codegen.sico` 已回退到 STEP-0555 基线（`git checkout HEAD --`，工作树净） |
| exit | **否**：三处累加器（`cgs_function_local_hex` / `cgs_core_types_hex` / `cgs_core_funcs_hex`）由自左折叠 concat 改为 `list.append` + 末尾一次 `join`，**组件字节等价 28/28 全过**（无回归、无退化），但 **module-level fuel bracket 逐档不变**（`functions` / `EMIT_ONLY` / `EMIT_SPLIT` / `EMIT_PADDED` 四臂全部与 STEP-0555 §10.4b/c 基线逐档相同） |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**（无变化） |
| 证据等级 | `internal-fixture`：本机 2026-11-05，Windows x64，debug 测试构建（`selfhost_compiler-3ee9bc92fdb5f988`，构建时 `include_str!` 已含本片改动），`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB / hostcall 8 GiB） |

## 1. 要回答的问题（C 假设的墙有多厚）

STEP-0555 §10.7 收敛结论：主导项是**循环内文本累加拼接**（`cgs_read_local_hex`:3859-3860 等 `set X = sico.text.concat(X, 新片)`，全 `codegen.sico` **416 个**该形态站点），下一刀 = **C 线性装配**（`list.append` + 末尾一次 `join`）。

本片把 C 刀**先落到三处模块级累加器**（`cgs_function_local_hex` / `cgs_core_types_hex` / `cgs_core_funcs_hex`，都是自左折叠、随程序规模升维的），问：**去掉 guest 侧的 O(n²) 文本重拷，fuel 能降多少。**

## 2. 方法（已实施，后被测量证伪）

三处改动（工作树 diff，31 插入 / 27 删除）：

- `cgs_function_local_hex`：`cgslh_hex` 累加器（每指令/每 cell 一次 concat）→ `cgslh_parts = sico.text.split_lines("")` + 每段 `list.append` + 末尾一次 `sico.text.join(cgslh_parts, "")`。
- `cgs_core_types_hex`：`cctsh_out` → `cctsh_parts` + `list.append` + 一次 `join`（type 段）。
- `cgs_core_funcs_hex`：`ccfsh_out` → `ccfsh_parts` + `list.append` + 一次 `join`（func 段）。

变换遵循 STEP-0549 已确立的 idiom（`sico.text.split_lines("")` 建空 `List[Text]`、`sico.list.append` 收集、边界一次 `sico.text.join(parts, "")`），字节序与分隔符不变 ⇒ 语义等价。

## 3. 测量（决定性负结果）

**探针** = `the_emitter_fuel_scaling_in_read_local_count_is_measured`（`runner/sico-runner/tests/selfhost_compiler.rs:11848`，`#[ignore]`，本机单线程 224.78 s），`SICO_M22_PROBE_FUEL` ceiling 200e9（同 STEP-0555 §10.4b/c）。

`functions` 臂（F 个 ~80B IR 小 helper，无 read_local、无 per-op concat —— 唯一跨函数共享成本是 types/funcs/code 段逐函数发射）：

| F | STEP-0555 §10.4b 基线 | 本片（C 刀） | Δ |
|---|---:|---:|---|
| 16 | 1.6e9…3.2e9（2.4e9） | 1.6e9…3.2e9（2.4e9） | **0** |
| 32 | 6.4e9…1.28e10（9.6e9） | 6.4e9…1.28e10（9.6e9） | **0** |
| 64 | 1.28e10…2.56e10（1.92e10） | 1.28e10…2.56e10（1.92e10） | **0** |
| 128 | 5.12e10…1.024e11（7.68e10） | 5.12e10…1.024e11（7.68e10） | **0** |
| 256 | exhausted | exhausted | **0** |

`EMIT_ONLY`（Rust oracle 预建 IR、免解析发射）：

| F | §10.4c 基线 | 本片 | Δ |
|---|---:|---:|---|
| 16 | 3.2e9…6.4e9（4.8e9） | 3.2e9…6.4e9（4.8e9） | **0** |
| 32 | 1.28e10…2.56e10（1.92e10） | 1.28e10…2.56e10（1.92e10） | **0** |
| 64 | 2.56e10…5.12e10（3.84e10） | 2.56e10…5.12e10（3.84e10） | **0** |
| 128 | 1.024e11…2e11（1.5e11） | 1.024e11…2e11（1.5e11） | **0** |
| 256 | exhausted | exhausted | **0** |

`EMIT_SPLIT` / `EMIT_PADDED`（C-vs-D 判别臂）：

| 臂 | §10.4 基线 | 本片 | Δ |
|---|---|---|---|
| EMIT_SPLIT 4 | 0.8e9…1.6e9（1.2e9） | 0.8e9…1.6e9（1.2e9） | **0** |
| EMIT_SPLIT 16 | 3.2e9…6.4e9（4.8e9） | 3.2e9…6.4e9（4.8e9） | **0** |
| EMIT_SPLIT 64 | 2.56e10…5.12e10（3.84e10） | 2.56e10…5.12e10（3.84e10） | **0** |
| EMIT_PADDED 8 | 1.6e9…3.2e9（2.4e9） | 1.6e9…3.2e9（2.4e9） | **0** |
| EMIT_PADDED 32 | 1.28e10…2.56e10（1.92e10） | 1.28e10…2.56e10（1.92e10） | **0** |
| EMIT_PADDED 64 | 2.56e10…5.12e10（3.84e10） | 2.56e10…5.12e10（3.84e10） | **0** |

**B4_C_GREEN = 0**。四臂**逐档相同**（±0，bracket 分辨率 2×，即任何 <2× 的改善不会显现，但 C 假设预测的是数量级级差异——module-level 的 O(n²) 若真在文本累加，去掉后 `EMIT_ONLY` 应从平方掉到线性，远大于 2× 分辨率）。

## 4. 根因（与 B3 / B4-D 同构：被否决的「墙」不是墙）

- **C 假设（文本累加）被证伪**：`functions` 臂的 F 个小 helper **无 read_local、无 per-op concat**，唯一跨函数共享成本是逐函数发射 types/funcs/code 段；把三处模块级累加器改线性装配后 fuel **逐档不变** ⇒ 模块级平方**不在 guest 侧文本累加**。
- **与 STEP-0549 的 P1 结果合拢**：P1（STEP-0549）把 `cgj_string_hex` / `cgs_function_instr_hex` / `cgs_data_hex` / `cgs_core_code_hex` 四个累加器（含最大的 code 段）也改成 `list.append` + `join`，P2（STEP-0550）实测 **0/364 回绿** ⇒ **code 段的线性装配同样不降 fuel**。
- ⇒ **自 P1 起所有文本累加器（四 + 三 = 七处）都已线性装配，fuel 无一变化**；「文本累加是墙」这条假设**整体不成立**。

**已排除的候选（逐一负结果）**：① 数组重扫（B3 游标化，§10.6）② cell_slot 重扫（B4-D 索引化，§10.7）③ **文本累加（P1 + 本片 C）** ④ 解析器处理 F 函数（`EMIT_ONLY` 免解析**更贵** 2×，§10.4c ⇒ 解析器不是主导）。

**尚未排除的候选（下一次裁决）**：
1. **`cgj_member` / `cgj_value_text` / `cgj_ident_text` 字段查找重扫**：每次调用从 JSON 起点按字段名/值重扫（`cgj_array_item` 族同源），发射器里每函数/每指令反复查模块级字段 ⇒ O(module) 每次 × O(module) 次 = **O(module²)**。`cgj_array_item`（数组按下标重扫）已被 B3 排除，但 **`cgj_member`（按字段名重扫）未被单独测过** —— 这是下一步最强的候选。
2. **core-module helper 体自身 O(n²)**：`sico.text.join` / `sico.list.append` 是 core module 的 wasm helper（STEP-0542），其体若内部反复 concat 重拷，最终一次 `join` 仍 O(总字节²)。这属 core helper 层，guest 改不了（需另一层刀），但也说明**「改 guest 累加器」对 fuel 无效是必然的**——fuel 墙在 helper 执行层，不在 guest 的调用形状。

## 5. 回退与留档

- `git checkout HEAD -- selfhost/codegen.sico`：回到 STEP-0555 基线（`git status` 净，`codegen.sico` sha = `4ac9b3c1…` = HEAD）。**理由**：fuel 中性改动却会改 `guest_emitter.source_sha256` ⇒ `emitter-vocabulary-v1.json` 指纹失效 + `match-component-coverage-v1.json` 需要 2h 重测，而本片**不推进任何门**，保留 = 纯成本。
- **不保留** C 代码改动；无新增工具脚本（本片只改三处累加器，无独立变换脚本）。
- 本 STEP 留档为负结果证据。

## 6. 验证（回退前，C 刀 applied 状态）

| 测试 | 结果 |
|---|---|
| `sico_codegen`（28 个固件，单线程） | **28 passed / 0 failed** —— 组件字节等价全部保持，无回归无退化 |
| `the_emitter_fuel_scaling_in_read_local_count_is_measured` | 四臂逐档与 STEP-0555 §10.4b/c 基线**相同**（负结果证据） |
| `git diff --check` | 净 |
| 完整 runner 套件 | **未跑**（本片为负结果测量片；回退后无需全量） |

## 7. 下一步（B4 继续裁决）

**改 `codegen.sico` 前必须先落 `cgj_member` 字段查找重扫的判别探针**（§4 候选 1），不能盲改。最短裁决路径：

1. **`cgj_member` 判别臂**：在 module-level scaling 探针上做减法——固定 F，对比「经 `cgj_member` 查字段」与「查表/游标化字段访问」两版的 fuel bracket；若游标化/索引化降 fuel ⇒ 候选 1 成立，B5 = `cgj_member` 族索引化刀（B2 同类，guest 前端层）。
2. **core-helper 判别**：确认 `sico.text.join` / `sico.list.append` 的 helper 体是否为内部反复 concat（读 `cg_*_helper_body_hex` 冻结体 + oracle `stdlib.rs`），若 O(n²) 则 fuel 墙在 helper 执行层，需在 **runner/core-helper 层**另立刀（不同层，不属 `codegen.sico` 写入方，可并行）。

**诚实收窄（不变）**：331 `FuelExhausted` + 24 8MiB trap + 5 refused-guest + 4 既有红 + 7 词表缺口；组件轴仍 **36 / 396**；M22 仍 **NO-GO**、七门不变。**本片把「墙在文本累加」这条已登记假设正式否决**，并把下一个裁决点收到 `cgj_member` 字段查找重扫与 core-helper 执行层二者之一。

## 8. 边界（不变）

- 静态断言 `every_match_site_is_measured_on_the_component_axis`：ledger 396 站点无 `closure-too-large` skip、无空 digest/verdict（STEP-0550 P2 exit 达成）。
- FuelExhausted verdict 语义（selfhost_compiler.rs:11975+）：二分/倍增 budget 直到出 Component 或 exhausted/refused/trap；`component-equal` 站点全持 `component_sha256`。
- fuel / timeout / 内存 / hostcall 四限额不变；不上调、不追认。

## 9. 交付

**代码 0 净改动**（C 代码已回退，工作树净）。留档：本 STEP（负结果证据）。

- `git status`：净（无修改、无未跟踪）。
- 不推送（领先 origin，需 owner 授权）。

## 10. B4-C 后续复核（2026-11-05，同一轮）：join/append/bytes 原语 O 度 + 穷举静态路径分析 ⇒ 墙不在 IR 走查算法（§4 候选 2 亦证伪）

本片先按 §7 第二步读 `cg_text_join_helper_body_hex` 冻结体 + oracle `stdlib.rs` 确认 core-helper 层 —— **结论与 §4 候选 2 相反，候选 2 证伪**：

| 原语 | oracle（`crates/sico-codegen-wasm/src/stdlib.rs`） | guest 冻结体（`selfhost/codegen.sico`） | O 度 |
|---|---|---|---|
| `sico.text.join` | `emit_join` :670：长度趟（累加 total）+ 拷贝趟（`MemoryCopy` 逐元素、`dst += len`）| `cg_text_join_helper_body_hex` :316：**同一形状**（第二循环 `…2007 2008 2009 fc0a0000 2007 2009 6a 2107…`，`fc0a0000` = `memory.copy` **不是** concat 调用）| **线性** |
| `sico.list.append` | `emit_list_append` :591：几何扩容（capacity 翻倍，:635-639）| `cg_list_append_helper_body_hex` :259：同一形状 | **摊还 O(1)** |
| `sico.bytes.length` | `lib.rs:4443`：`local.get length` | —（内联）| **O(1)** |
| `sico.bytes.at` | `lib.rs:4800`：边界检查 + `I32Load8U` | —（内联）| **O(1)** |
| `sico.text.length` | `emit_text_length` :390：**逐字节 UTF-8 计数循环** | `cg_text_length_helper_body_hex` :173：同一形状 | **O(n)** |

**§4 候选 2（core-module helper 体自身 O(n²)）被证伪**：join 与 append 都是线性/摊还 O(1)，**不是**「内部反复 concat 重拷」。§4 候选 1（`cgj_member` 字段查找重扫）因此**只剩这一个**候选。

**穷举静态路径分析（本片新增，决定性）**：读完整条 probe 路径（`cgs_core_code_hex` :6536 / `cgs_core_funcs_hex` :5199 / `cgs_function_body_hex` :6502 / `cgs_function_at_index` :4922 / `cgs_function_instr_hex` :3990（已游标化）/ `cgs_op_hex` :3870 / `cgs_write_local_hex` :3785 / `cgs_value_index` :2041（已游标化）/ `cgs_value_ids_refusal` :1984（已游标化）/ `cgs_dispatcher_slot` :2199 / `cgs_type_slots` :1931 / `cgs_module_has_intrinsic` :3211（**仍用 `cgj_array_item` 重扫**，×36 次调用：`cgs_core_code_hex` 12 + `cgs_core_funcs_hex` 12 + `cgs_core_types_hex` 12））：

- 每个 probe 函数（`let a0=data; let a1=a0; let a2=a1; return a2`，~80B IR）**无 read_local、无 call、无 intrinsic** ⇒ 每指令路径只到 `cgs_write_local_hex`（O(1)）。
- 每函数路径 = `cgs_function_at_index`（`cgj_member(bytes,0,"functions")` O(module) + `cgj_array_item` O(index)）+ 局部段 + 指令段（已游标化，O(per_fn)）+ `cgs_value_index`/`cgs_dispatcher_slot`/`cgs_value_ids_refusal`（均 O(per_fn)，已游标化或 O(per_fn²)）。
- 模块级 = `cgs_module_has_intrinsic` ×36（每个 O(F²) 数组重扫 + O(module) 字段重扫）**+ 文本累加（`cgs_core_code_hex` 已 append+join，线性）**。

**关键判读**：①B4 的 `~81K wasm ops` 数组重扫估计（§10.6）与上面全部重扫相加同量级（~几万–几十万），而实测 F=16 是 **2.4e9** ⇒ **数组/字段重扫全部加起来 ≈ 0.01%**，不是墙；②`sico.text.length` O(n) 的所有调用点都在**小数据**（类型/字段/单函数体 hex）或**一次**（`codegen.sico:63` 最终组件 payload），无「大字符串 × 循环」站点 ⇒ 不是墙；③per_fn 对 probe 是常数（3 lets）⇒ 实测 O(F²) 不可能来自「每函数 O(per_fn) 工作 × F 函数」；④静态可读的每函数路径无 O(module) 项（除 `cgs_function_at_index`，B4 已移除且 fuel 不变）⇒ **静态分析无法解释 2.4e9 = ~1.2e6 wasm ops / 输出字节**。

**裁决（本片）**：**fuel 墙不在 guest 发射器的 IR 走查算法**（所有可读路径均为小常数或小 O(F²)，与 2.4e9 相差 3–4 个数量级）。§4 的**两个候选均已证伪**（候选 1 的「重扫」贡献可忽略，候选 2 的「helper O(n²)」不存在）。**下一刀不再是任何单一重扫游标化/索引化**（B3/B4/D/P1/C 已证该类全部无效）——需要**仪器化 profile**（wasmtime epoch/opcode 级、或对发射器执行做逐函数计费）把 2.4e9 定位到具体指令，或接受「Sico→wasm 常数 × 大量小走查」作为主导项并转**结构预索引**（每模块一次建函数/块/指令/类型索引，替换所有 `cgj_member`/`cgj_array_item` 重扫）。

**诚实收窄（更新）**：组件轴仍 **36 / 396**、331 `FuelExhausted` + 24 8MiB trap + 5 refused-guest + 4 既有红 + 7 词表缺口；**M22 仍 NO-GO、七门不变**。已排除候选增至：①数组重扫（B3）②cell_slot 重扫（B4-D）③文本累加（P1+C）④解析器（EMIT_ONLY 更贵）⑤join 内部重拷（本片，证伪）⑥append 内部重拷（本片，证伪）⑦bytes.length/at 重扫（本片，O(1)）⑧text.length 大字符串循环（本片，无站点）。

## 11. B4-C 后续复核（续，2026-11-05）：六刀 0-1% 模式 + STEP-0544 归因已由 B2 修复 ⇒ 墙是常数主导，不是算法主导

**六刀模式（跨片交叉，决定性）**：每把「消除一个 O(n²) 重扫/累加」的刀都只回绿 **0-1%** —— B2（STEP-0551，`cgs_value_slot` 逐指令重扫 → stride-2 值索引，O(1) 查表）复算 **回绿 4 / 364 = 1.10%**；B3（STEP-0555 §10.6 游标化）/ B4-D（§10.7 cell_slot 索引化）/ P1（STEP-0549 文本累加 → append+join）/ C（本片三处累加器）**逐档不变**。**一致模式**：fuel 墙**不是任何单一 O(n²) 重扫/累加**，而是**常数主导** = Sico→wasm 每操作开销 × 操作数（发射器执行 ~1000+ 个高层操作/函数，每操作 ~千级 wasm 指令，单函数固定成本 ~1.5e8–6e8，随 F 增长源于模块级走查分摊）。

**STEP-0544 归因已由 B2 修复**：STEP-0544（2026-10-25）把 X 族（单函数 n 条指令）的 O(n²) 归因于 `cgs_value_slot` → `cgj_array_item`（codegen.sico:3793/846）；**B2（STEP-0551，2026-10-28）已把 `cgs_value_slot` 改成 O(1) 查表** ⇒ X 族的该 O(n²) **已消除**（B2 实测只回绿 1.10%，说明它也不是主体墙）。**残留的 X 族 O(n²) 候选** = `cgs_function_local_hex`（:1664）的三处 **未游标化** `cgj_array_item` 循环（:1674/:1686/:1714，每指令 O(instr_pos) 重扫；C刀只改了它的累加器、没改它的重扫，B4 只游标化了 code 循环、没碰 local_hex）——但它是**每函数内**的 O(n²)，对 functions 族探针（每函数 3 指令）是 O(1)，**不会移动 functions 族墙**。

**裁决（更新）**：**墙 = 常数主导（Sico→wasm 每操作开销 × 操作数），不是算法主导**。任何局部 O(n²) 优化（游标化/索引化/线性装配）预测 ≤2× 且已被六刀逐一证伪到 0-1%。**下一步不是再一把重扫刀**，而是二选一：
1. **结构预解析**（每模块一次把 IR 预解析成紧凑结构，替换全部逐字段 `cgj_member`/`cgj_value_text` 重读，操作数降 10–100×）——这是唯一能实质性降 fuel 的方向，但改动面大（几乎每个 `cgs_*` 函数签名 + 指纹 + 2h ledger）。
2. **owner 决策**（D8 候选：fuel 1.6T vs 800e9 = 2×，或收窄语料/闭包范围）——若常数主导且预解析成本过高，需 owner 在「上调限额」与「收窄口径」间裁决。

**诚实收窄（不变）**：组件轴 36 / 396、331 `FuelExhausted` + 24 8MiB trap + 5 refused-guest + 4 既有红 + 7 词表缺口；M22 仍 NO-GO、七门不变。已排除候选增至 ⑨cgs_value_slot 重扫（B2，1.10%）⑩任何局部 O(n²)（六刀合计 0-1%）。
