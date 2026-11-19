# STEP-0636 · M22 · dispatcher+cindex 全线程化（闭合 STEP-0627 登记的 A4 遗留缺口）+ match-payload 游标化（A5 未覆盖项）

> **owner 授权依据**：owner 指令 2026-11-19「推进 M22，重大问题问我，小问题先尝试再做方案，不需要问我，可使用子代理」。燃料墙优化轨循 owner D-A（2026-11-08，AGENTS §10.1.4 addendum，op 级燃料仪器化剖析已授权）；D-B（不上调 fuel 限额）全程遵守，未上调任何 fuel/内存/timeout 限额。
>
> **改动面**：仅 `selfhost/codegen.sico`（**逐字节恒等**，两次字节级守卫 pin 全绿）。**未 commit/push**（§9 留 owner 裁决）；**未翻任何 gate**，M22 overall 仍 **NO-GO**，仅 internal-fixture 证据。
>
> **本片 = 纯延续**：闭合 STEP-0627（2026-11-19）在其 A4 项**明确诚实收窄**的遗留缺口「**dispatcher+cindex 穿线到 read/write/branch/jump，见下 → 未做**」，并补一处 STEP-0627 A5 未覆盖的游标化。无并发冲突（`cgs_match_condition_hex` 已被 A5 游标化，本片未触碰它）。

## 1. 背景与动机

M22 gate 2 的主要阻塞是**发射器燃料墙**（owner D-B 已裁不上调限额买回站点，STEP-0600/0606）。STEP-0611 把热叶钉死在 `cgj_value_end`（JSON 逐字节游走，由 `cgj_member` value-skip 与 `cgj_array_item` 按索引重扫驱动）；STEP-0625 用 intrinsic-membership mask 把三级 intrinsic 重扫换成预计算 mask；STEP-0626 闭环整份 `formatter.sico` 组件（28,459 B 逐字节、486.2e9 fuel = 标准预算 60.8%）；STEP-0627 落地 A1/A2/A4/A5 六函数游标化，但**A4 的 dispatcher+cindex 全线程化被诚实收窄为未做**。

本片用 op 级燃料剖析（D-A 授权的三探针）重新归因后，把这一遗留缺口作为最高杠杆落地。

## 2. 数据驱动归因（三探针，`--ignored --exact` 亲跑，未上调限额）

| 探针 | 结论 |
|---|---|
| `the_named_operators_are_attributed_to_op_locations` | `match_flat` n=16 消耗 **210.73e9**（每字节 46.3e9/KB）vs `text_concat` n=64 仅 **3.77e9**（0.738e9/KB）⇒ match 发射燃料 ≈ **63×/字节**，IR 体量相当 |
| `the_emitter_cost_centers_are_ranked_by_fuel_slope` | **match_statements 全尺寸（n=8..64）在 800e9 封顶下耗尽**；`text_concat` slope=**1.00**（线性，总 3.6e9）；`functions` slope=1.22；`forwarded_let_control` slope=0.50 ⇒ match 是唯一超线性成本中心 |
| `the_match_quadratic_term_is_discriminated_from_body_scanning` | **决定性**：`match_flat`（N match，每个付一次全 body 重扫）**精确二次**（n=4 observed/quad=**1.000**、n=8 即耗尽）；`match_one`（同 body、仅 1 match）几乎平坦（0.7→1.4e9）⇒ 二次项 = **每个含分支/match 的 block 头/jump/return 各付一次全 body `cgs_dispatcher_slot` 重扫** |

⇒ 数据把杠杆指向 STEP-0627 遗留的 **dispatcher 槽全线程化**（消除剩余逐-block 全 body 重扫），非 `text_concat` 装配（发射时线性）。

## 3. 改动（仅 `selfhost/codegen.sico`，逐字节恒等）

### 3.1 dispatcher 槽全线程化（闭合 STEP-0627 A4 遗留）
`cgs_function_instr_hex` 已在函数顶算一次 `crih_dispatcher = cgs_dispatcher_slot(bytes, crih_run)`（`:4520`，纯函数、非零已验）。本片把它沿**逐-block 终结符路径**线程化到三个仍各自重算全 body 扫描的子函数：

- `cgs_block_terminator_hex`（已收 `dispatcher`）→ 传入三个子函数
- `cgs_result_return_hex(bytes, run, value, vindex, dispatcher)`：`crrh_dispatcher`/`crrhe_dispatcher` 两处 `cgs_dispatcher_slot(bytes, run)` → `dispatcher`
- `cgs_branch_hex(bytes, fn, term, vindex, dispatcher)`：`cgbh_dispatcher` 一处 → `dispatcher`
- `cgs_jump_hex(bytes, fn, term, dispatcher)`：`cgjh_dispatcher` 一处 → `dispatcher`

三个子函数签名各加 `dispatcher: U64` 末参；`cgs_block_terminator_hex` 三个调用点（result_return/branch/jump）各传 `dispatcher`。

**逻辑恒等**：`cgs_dispatcher_slot` 是 run body 的纯函数，传入的 `crih_dispatcher` 与各处原重算值逐位相同，且已在 `:4520` 验证非零 ⇒ 每处 `if X==0 return ERR` 守卫行为不变（永不触发）。

### 3.2 A4-cindex 线程化（read/write_local）
承接本片同批完成的前置：`cgs_op_hex` 加 `dispatcher` 末参（唯一调用点 `cgs_function_instr_hex:4544` 传入 `crih_dispatcher`），`cgs_write_local_hex`/`cgs_read_local_hex` 各加 `dispatcher` 末参，`cgs_cell_slot` 重构为 `cgs_cell_slot_with_dispatcher(bytes, fn, index, dispatcher)` 函数体 + 薄包装（布局算术逐位相同，含 `width==0` 守卫），read/write_local 改调 with_dispatcher 变体。消除每 local-op 重算 `cgs_dispatcher_slot` 的 O(N) 项。

### 3.3 match-payload 游标化（STEP-0627 A5 未覆盖）
`cgs_match_payload_ok`（`:6751`）原为 `while` + `cgj_array_item(bytes, payload, index)` 按索引重扫（O(m²)）；改为 `cgj_array_first`/`cgj_array_next` 游标遍历（O(m)）。`cgj_array_first/next` 返回与 `cgj_array_item` 相同的字节偏移（`:943/:954` 注释明说"cursor form…linear instead of rescanning from the array head per index"）⇒ 遍历语义、提前返回（bool/variant payload）、字节输出恒等。

### 3.4 剩余 `cgs_dispatcher_slot` 调用点
线程化后全文件仅剩 4 处：定义（`:2271`）、`cgs_cell_slot` 薄包装（`:2366`）、`cgs_local_lookup` 族（`:3878`）、`cgs_function_instr_hex` 单次（`:4520`）—— **无逐-block 重扫**。

## 4. 验证（本地单线程，MSVC `--locked --offline`）

- **`sico_compiler_emits_the_whole_formatter_component_byte_exactly` = ok**（每片后复跑均绿）
- **`sico_codegen_guest_emits_map_and_list_cell_component_byte_exactly` = ok**（每片后复跑均绿）
- **整份 formatter 测量**（`step0608_measure_whole_formatter_component_emission`，标准预算 800e9/120s/2560MiB 未上调）：`A:main`/`B:run` 均 **28,459 B / byte_equal=yes / validator=pass** ⇒ STEP-0626 W3-a 闭环保持
- `git diff --check` 净（仅 CRLF 警告）；收尾 `codegen.sico` SHA `3930217…`（开工 `110cf7f…`）

### 燃料下降实证（受控 match 探针，字节不变）
| family | 尺寸 | 线程化前 | 线程化后 | Δ |
|---|---|---|---|---|
| match_flat | n=8 | 31.08e9 | **20.18e9** | −35% |
| match_flat | n=16 | 210.73e9 | **131.01e9** | −38% |

二次指数 2.76 → **2.70**。`match-payload` 游标（§3.3）对 match_flat 无感（其 payload 极小），收益在**大 payload 的 match**上；组件字节两次均恒等。

### 整份 formatter 总燃料实测（决定性）
`m22_w3a_whole_formatter_total_fuel`（本片收尾复跑，标准预算未上调）：**consumed = `127.07e9`**，较 STEP-0626 基线 `486.2e9` 降 **−73.9%**；组件仍 **28,459 B / Output(0)**、wall 从 ~19s 降到 **14.8s**。这是本片三切片（dispatcher 全线程化 + cindex read/write_local 线程化 + match-payload 游标化）叠加 A1/A2/A4/A5/intrinsic-mask 既有优化的累计效果，**组件字节不变**。

## 5. 部分账本重测（诚实，非里程碑级）

曾尝试全量账本固件重测，但环境有外部干扰（一个 bash.exe 反复启停固件、出现双写者竞争 `actual.json`、运行被终止于 ~16:42）⇒ **完整 97 组重测未取得**。捕获的**部分数据（39/97 组、96 站点，均对未变的工作树 codegen.sico）**方向一致：

- 96 站点中 **12 翻转（FuelExhausted→component-equal）、2 改善（refused-guest→equal）、0 回退**
- 翻转率 12/64 = **18.8%**（与受控探针 −35~38% 方向吻合）
- 5 个翻转组：`parser.sico::{while_bytes_at_rhs_packed, while_bytes_at_local_src_packed, environment_call_arguments, argument_values_at, fixed_guard_condition_json}`

**诚实边界**：捕获的 39 组是排序在前的快组，重型 FuelExhausted 尾部未覆盖 ⇒ 真实总翻转数与完整分布**未知**；18.8% 是方向性信号，非里程碑数字。**整份 formatter 精确燃料已在本片收尾复测 = `127.07e9`**（较 STEP-0626 基线 486.2e9 降 −73.9%，见 §4）。

## 5.1 决定性负结果：燃料优化未解锁任何 match 站点（本片收尾实测）

用只读探针 `sico_compiler_probes_the_match_site_closure_bound`（写 `actual-probe.json`、不断言、不重冻结冻结账本，`SICO_M22_MATCH_GROUPS` 过滤），实测 4 个代表 FuelExhausted 组（标准预算未上调）：

| 组 | 站点 | 实测 verdict |
|---|---|---|
| `parser.sico::fixed_guard_chain_ir` | 7 | 全 `ir-equal+component-other:FuelExhausted` |
| `parser.sico::gw_rhs_multi_pack` | 7 | 全 `FuelExhausted` |
| `parser.sico::general_while_function_ir` | 7 | 全 `FuelExhausted` |
| `parser.sico::gw_slice_subject_packed` | 8 | 全 `FuelExhausted` |

⇒ **本片（及 STEP-0618/0625/0626/0627 的逐字节恒等优化刀）让小组件（formatter，28,459 B）从燃料墙解放（127e9 ≪ 800e9），但 337/406 match 站点组件面仍被燃料墙闸住**。原因：match 站点测的是**整份 `parser.sico` 大函数**，组件体量大得多，燃料 ~ 组件²，去掉所有已识别超线性算子（dispatcher 重扫 / intrinsic 三级重扫 / index 遍历）后**仍超 800e9**。要让这批站点翻转，需**数量级**的燃料削减（非当前常数/线性因子优化），或 owner 重议 D-B 限额——两者均超出本片范围、留 owner 裁决。**未翻任何门。**

## 6. 残余与下一刀（按数据）

子代理只读归因（`target/tmp/goal/match-residual-quadratic.md`）定位残余二次项：**主导（实测，扫描）= `cgj_value_end`（`:731`）**，由 `cgj_member` value-skip（`:854`）与 `cgj_array_item` 按索引重扫（`:912`）驱动；全文件仍剩 **68 处 index 形态 `cgj_array_item`**（STEP-0627 A5 只游标化 6 函数）。次级（结构推断）= `cgs_match_condition_hex`/`cgs_match_hex` 的左折叠 concat，但对 match_flat 是常数、非其曲线驱动项。

**下一刀候选**（字节恒等、局部、低风险，循本片同一手法）：
- 系统性游标化剩余 68 处 index 形态 `cgj_array_item`（逐处红-first 差分 + 两 pin 兜底）
- `cgs_write_local_hex:4320`/`cgs_read_local_hex:4356` 的 `cgj_array_item(fn.locals, local)` 重扫 —— `local` 为数据相关任意索引，游标化需 locals 偏移预计算结构，风险高于本片 A，单独评估

## 7. 门纪律

**未翻任何门**，M22 overall 仍 **NO-GO**（门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不沿用 GO）。仅主张 internal-fixture 证据（两个字节级 pin + 整份 formatter 测量 + 受控探针）。未上调任何 fuel/内存/timeout 限额（D-B 遵守）；未 commit/push（§9 留 owner）；未触碰并发 M24 g6-converter 文件。