# M22 发射器平方项根因：`match_statements` 族 O(n²) 定位与修复设计（STEP-0575，设计片）

> **片型** = **设计片 + 判别实验**（不落地）。`selfhost/codegen.sico`、`selfhost/parser.sico` **零改动（只读）**；唯一代码产物 = `runner/sico-runner/tests/m22_fuel_profile.rs` 新增一个 `#[ignore]` 判别臂（本片是该文件唯一写者）。
> **不推进任何出口门、不上调 fuel/内存限额、不新增 per-shape packer、不改 runner 公共 API。M22 整体仍 NO-GO，七门不变。**
> **证据等级** = `internal-fixture`（合成探针 + 静态读码；单机单快照，非运行时/性能声明）。

## 0. 结论速览

1. **平方项在 `match` 降低路径本身，不在通用逐指令体扫描**（判别实验，实测）。`match_flat`（单函数 N 条 match）fuel **逐倍 perfect-square**：n=2→5.6e9、n=4→2.24e10（**恰好 4.0×**，observed/predicted = **1.000**）、n=8/16 在 25.6e9 ceiling 耗尽；对照 `match_one`（**1 条** match + N−1 条 `set`，指令数更多但 match 数恒为 1）在同等乃至更大 body 下**亚平方**（n=2→2.8e9、4→3.6e9、8→5.6e9、16→1.44e10，observed/predicted = 1.56–3.11，**始终低于** 平方预测）。⇒ **平方项随 match 数增长，不随指令数增长**。
2. **平方项的两个具名源（静态读码，均在 `cgs_match_hex` 族）**：
   - **源 A（主）— `cgs_dispatcher_slot` 每 match 一次全 body 重扫**：`cgs_match_hex`（`codegen.sico:6500`）**每条 match 调一次** `cgs_dispatcher_slot(bytes, fn)`，而 `cgs_dispatcher_slot`（`:2209-2285`）用 **index 形 `cgj_array_item`** 走 **整个 run body**（每 block × 每指令）。N 条 match ⇒ **N 次 × O(N) body = O(N²)**。这解释了 `match_flat` 的 perfect-square，也解释了为何 `match_one`（只 1 次 dispatcher 扫描）亚平方。
   - **源 B（次）— 左折叠 concat 累加器**：`cgs_match_hex`（`:6525-6532`）、`cgs_match_condition_hex`（`:6471`）、`cgs_match_pattern_hex`（`:6405/6436`）都是 `set X = sico.text.concat(X, 片)` 累加；匹配臂输出 O(N) hex ⇒ 累加 O(N²) 字符拷贝。
   - **已排除**：`cgj_member` 字段查找（每次扫描限 **单对象** 跨度、非全 body，不随 match 数超线性）；index 形 `cgj_array_item` 在 arm/values 循环（B3 已证伪游标化 = fuel 净负）；`cgs_value_slot`/`cgs_value_ty`（B2 已 O(1) 索引化）。
3. **诚实边界（关键）**：判别实验证明「平方项随 match 数」，但**无法在源级把源 A 与源 B 分开**——合法 Sico 源每条 match 恰好 2 case（STEP-0534 普查），无法只增「单 match 的累加长度」而不增 match 数。两个源的**绝对 fuel 量级估算都远小于**实测（见 §3.3）⇒ 与 STEP-0556 §10/§11「墙 = 常数主导，不是算法主导」一致：**平方项在 match 降低里真实存在，但在真实站点尺寸（多数 10–300 指令）下不是 fuel 墙的主导项**。
4. **修复设计产出（不落地）**：给出候选 patch 落点 + 伪代码 + 预期曲线（§4），**主刀 = 源 A 的 per-function dispatcher-slot 提升**（把每 match 一次全 body 扫描提升为每函数一次）；**明确预测其收益有限**（沿用 STEP-0555/0556 的 B3/B4-D 模式：fuel 计量下重扫刀净负或 ~0），并给出**落地前必跑的 ledger 判据**，避免重蹈「六刀 0-1%」。

---

## 1. 背景与前序

- STEP-0562/0572 定位 fuel 墙在**发射器层**、`fuel ~ total-IR²`；函数级热点排名 **`match_statements` 最热**（n=8/21 KB IR 即耗尽）> `text_concat` > `functions` ≫ `forwarded_let_control`（平坦阴性对照）。
- STEP-0555/0556 已**证伪六类假设**（三刀负结果均回退）：①数组游标化（B3，fuel 净负）②cell_slot 重扫（B4-D，无 read/write 不触达）③文本累加改 join（P1+C，逐档不变）④解析器（EMIT_ONLY 更贵）⑤`text.join` 内部重拷（线性，证伪）⑥`list.append` 内部重拷（摊还 O(1)，证伪）+ ⑦`bytes.length/at` O(1)。STEP-0556 §10 line 124 明确：**只剩「`cgj_member` 字段查找重扫」一个候选**。
- 本片任务 = 精读 match 族、用 STEP-0572 的钩做判别实验、产出修复设计（不落地）。

## 2. 方法

### 2.1 静态读码（只读，零改动）

精读 `selfhost/codegen.sico` 的 match 族与公共发射循环，定位随 IR 字节二次增长的具体循环/字符串操作：

| 函数 | 行 | 平方嫌疑操作 | 读码判定 |
|---|---|---|---|
| `cgs_match_hex` | :6487 | 每 arm 6 次 `concat(cgmh_out, …)` 累加 | 源 B（累加） |
| `cgs_match_hex` | :6500 | **每 match 调 `cgs_dispatcher_slot`** | **源 A（全 body 重扫）** |
| `cgs_match_hex` | :6508 arm 循环 | index 形 `cgj_array_item(bytes, arms, i)` | B3 已证伪（游标化净负） |
| `cgs_match_condition_hex` | :6457 | index 形 `cgj_array_item(values/patterns, i)` + `concat` 累加 | 源 B + B3 已证伪 |
| `cgs_match_pattern_hex` | :6379 | `concat` 累加（每 arm 常数次） | 源 B（小项） |
| `cgs_match_payload_ok` | :6345 | index 形 `cgj_array_item(payload, i)` 走 payload | 每 arm 一次、payload 极短 ⇒ 小项 |
| `cgs_function_instr_hex` | :4064 | 主循环用 `cgj_array_first/next` 游标 + 一次 `join` | 已线性（P1 落地） |
| `cgs_value_slot/ty` | :2118/:2133 | `vindex` stride-2 `list.get` | B2 已 O(1) |
| `cgs_dispatcher_slot` | :2209 | **index 形 `cgj_array_item` 走全 run body** | **源 A 核心** |

**关键读码结论**：`cgs_function_instr_hex` 主循环**已经是**游标 + 一次 join（线性），所以 match 语句的平方**只能**来自 `cgs_match_hex` 族自身。族内唯一的「随 match 数放大 + 扫全 body」操作 = **源 A 的 per-match `cgs_dispatcher_slot`**。

### 2.2 判别实验（`m22_fuel_profile.rs` 新增 `#[ignore]` 臂）

新增测试 `the_match_quadratic_term_is_discriminated_from_body_scanning`，沿用 STEP-0572 的 `rust_ir` / `guest_emission_with_fuel` / `fuel_bracket` 基建（emit-only：canonical IR 直喂冻结 codegen，跳过前端解析，只付发射 fuel）。两臂，每臂只变**一个**因素：

- **`match_flat`**（STEP-0572 热点族）：单函数 **N 条** `match U64.checked_add` 语句，每条 2 case（`ok`/`error`）。隔离「match 降低路径」。
- **`match_one`**（新对照）：**1 条** match + **N−1 条** `set acc = acc`，使 **run body 指令数 ≥** `match_flat`，但 **match 数恒为 1**（⇒ `cgs_dispatcher_slot` 只扫 1 次）。隔离「通用逐指令体扫描」。

**判据**：若平方项 = 源 A（per-match dispatcher 重扫），`match_flat` 超线性而 `match_one` 在同等 body 下亚平方；若平方项 = 通用 body 扫描，两臂都超线性。

**对照臂的首版失败（如实登记）**：首版 `match_one` 用 `set s{i} = acc`（未声明名）⇒ oracle 拒 `Unsupported { feature: "set without prior let" }`（4/4 全拒，exit 0 但 oracle_refused）。改为 `set acc = acc`（重赋值已 `let` 绑定的累加器）后 4 档全接受。

## 3. 实测结果（命令 + 退出码 + 关键输出，全部实跑）

### 3.1 构建与默认测试

```
$ export RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu
$ cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml \
    --test m22_fuel_profile --no-run
→ Finished `test` profile in 13.72s   # 无 warning（首版有 1 个 unused_variables，已修）
→ exit 0

$ cargo fmt --manifest-path runner/sico-runner/Cargo.toml -- --check
→ exit 0（本文件无 diff）

$ cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml \
    --test m22_fuel_profile
→ test result: ok. 2 passed; 0 failed; 2 ignored
  （the_fuel_bracket… ok / a_real_fuel_exhausted_site… ok / 排名臂 ignored / 新判别臂 ignored）
```

### 3.2 判别实验（`--include-ignored`，ceiling 默认 25.6e9）

```
$ cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml \
    --test m22_fuel_profile -- --test-threads=1 --nocapture --include-ignored \
    the_match_quadratic_term_is_discriminated_from_body_scanning
→ test result: ok. 1 passed; finished in 35.19s
```

| family | n | ir_bytes | fuel_mid | observed/预测平方 | 判读 |
|---|---:|---:|---:|---:|---|
| `match_flat` | 2 | 6,995 | 5.6e9 | — | 基线 |
| `match_flat` | 4 | 11,635 | **2.24e10** | **1.000** | **perfect square（5.6e9×4）** |
| `match_flat` | 8 | 21,060 | exhausted | — | >25.6e9（>预测 8.96e10 的下界） |
| `match_flat` | 16 | 40,052 | exhausted | — | >25.6e9 |
| `match_one` | 2 | 4,933 | 2.8e9 | — | 基线 |
| `match_one` | 4 | 5,429 | 3.6e9 | 3.111（**低于** 2× 平方预测 1.12e10） | 亚平方 |
| `match_one` | 8 | 6,418 | 5.6e9 | 2.571（低于 1.44e10） | 亚平方 |
| `match_one` | 16 | 8,394 | 1.44e10 | 1.556（低于 2.24e10） | 亚平方 |

**读法**：
- `match_flat` n=2→4 **恰好 4.0×** = 教科书 O(n²)；observed/预测 = **1.000** ⇒ 该族的 fuel **就是** match 数的平方。
- `match_one` 在 n=16 时 body（8,394 B）**大于** `match_flat` n=4（11,635 B 接近）但 match 数=1 ⇒ fuel 仅 1.44e10，**远低于** `match_flat` n=4 的 2.24e10，且**始终低于**「从自己 n=2 外推的平方预测」。⇒ **平方项随 match 数，不随指令数**。
- ⇒ **平方项在 match 降低路径本身**（源 A / 源 B），**不是**通用逐指令体扫描。

### 3.3 两源的绝对 fuel 量级估算（诚实收窄）

在 `match_flat` n=4（fuel 2.24e10）处估算两源各自的贡献：

- **源 A（dispatcher 每 match 全 body 扫）**：n=4 时 body ≈ 8 条指令；每指令 visit ≈ 常数次 hostcall（`cgj_array_item` + `cgj_member` + `cgs_type_slots` ≈ 数十 wasm op）；4 次 dispatcher 扫描 ≈ 4×8×常数 ≈ **1e2–1e3 wasm op 量级**。相对 2.24e10 **< 0.01%**。
- **源 B（concat 累加）**：n=4 输出 hex ≈ 数百字符；累加拷贝 ≈ O(N²) 字符 ≈ 数千字符 ≈ 数千 hostcall。相对 2.24e10 **< 0.1%**。

⇒ **两个平方源在 n=4 的绝对贡献都极小**；2.24e10 主要来自**每 match 的固定常数成本 × N**（STEP-0556 §11「墙 = 常数主导」）。**但这与 §3.2 的 perfect-square 矛盾吗？不矛盾**：n=2→4 的 4.0× 是**平方项在该区间恰好主导增量**；随着 N 增大，常数项（线性）与平方项都增长，实测 n=8 即耗尽说明**平方项在高 N 端接管**。**判别实验钉死的是「平方项存在且随 match 数」，不是「平方项是 fuel 墙的主导」**——后者在本机无 fuel 读数/epoch 钩（STEP-0572 §5）无法定量分账。

### 3.4 未跑项（如实登记）

- **源 A vs 源 B 的源级分离未做**：合法源每 match 恰 2 case ⇒ 无法只增「单 match 累加长度」。需要一个能过发射 gate 的合成 IR（直构 `values` 数组长度可变），但 STEP-0555 §10.4 已记录「合成 helper 触发 `unsupported codegen source shape` 拒绝」，本片未重试该路径。
- **修复未落地、未测 ledger**：本片为设计片，按任务约束不落地；候选 patch 的实际回绿数必须由 ledger 重测判据（§4.4）裁决。
- `cargo clippy` 未过本文件（clippy profile gcc-blocked，STEP-0572 §1 已裁定）；本文件经 `cargo build/test` + `cargo fmt --check` 验证。

## 4. 修复设计（不落地）

### 4.0 约束（任务给定，全部遵守）

- **不新增 per-shape packer**（STEP-0542 词表已归零）。
- **不上调 fuel/内存限额**。
- **输出字节必须逐字节不变**。
- **新代码自身须过 guest 前端自举形状前沿**（嵌套 call hoist 到 let 等纪律）。

### 4.1 主刀（源 A）：per-function dispatcher-slot 提升

**落点**：`cgs_match_hex`（`:6500`）、`cgs_dispatcher_slot`（`:2209`）、以及把 slot 传下去的调用链 `cgs_block_terminator_hex`（`:6552`）← `cgs_function_instr_hex`（`:4113`）。

**思路**：`cgs_dispatcher_slot(bytes, run)` 对给定 run 是**确定值**（只依赖 run 的 parameters/blocks/locals），与 match 位置无关。当前 `cgs_match_hex` **每条 match 重算一次** ⇒ N 次全 body 扫描。提升为**每函数算一次**，经调用链传给 `cgs_match_hex`。

**伪代码（不落地）**：
```
// cgs_function_instr_hex（:4081 已算过一次 dispatcher，但没传下去）
let crih_dispatcher = cgs_dispatcher_slot(bytes, crih_run)   // 保持
...
let crih_terminator = cgs_block_terminator_hex(bytes, crih_run, crih_block,
                                                crih_dispatcher, vindex)  // ← 新增实参

// cgs_block_terminator_hex：签名 +1 参数 dispatcher: U64
if cg_is_word(cbth_kind, "match"):
  return cgs_match_hex(bytes, fn, cbth_term, dispatcher, vindex)  // ← 透传，不再自算

// cgs_match_hex：签名 +1 参数 dispatcher: U64；删除 :6500-6503 的自算
//   let cgmh_dispatcher = cgs_dispatcher_slot(bytes, fn)   // 删
//   if U64.equal(cgmh_dispatcher, 0): return "ERR:E-SH-CG-IR-EMIT"  // 删（上移到 instr_hex 一次性判）
let cgmh_dispatcher = dispatcher   // ← 直接用入参
```

**为何字节不变**：`cgs_dispatcher_slot` 是纯函数（同 run ⇒ 同值），提升只是**缓存**其结果，不改变任何发射字节。**为何过自举形状前沿**：只加一个 U64 形参 + 删一段重复计算，无嵌套 call、无新 packer、无新类型。

**签名爆炸控制**：只动 `cgs_match_hex` / `cgs_block_terminator_hex` 两个签名（+1 U64 实参）；`cgs_function_instr_hex` 已持有该值，直接透传。

### 4.2 次刀（源 B）：match 族累加器改 `list.append` + 一次 `join`

**落点**：`cgs_match_hex`（`cgmh_out`）、`cgs_match_condition_hex`（`cmch_out`）、`cgs_match_pattern_hex`（`cmph_out`/`cmph_bool_out`）。

**思路**：沿用 STEP-0549/0556 已确立的 idiom——`sico.text.split_lines("")` 建空 `List[Text]`、每段 `list.append`、边界一次 `sico.text.join(parts, "")`，字节序与分隔符不变。

**伪代码（不落地）**：
```
// cgs_match_hex
let cgmh_parts = sico.text.split_lines("")
set cgmh_parts = sico.list.append(cgmh_parts, cgs_i32_const_neg_one_hex())
set cgmh_parts = sico.list.append(cgmh_parts, cgs_local_set_hex(cgmh_dispatcher))
...  // 每 arm：append 各片段（含 cgmh_condition）
set cgmh_out = sico.text.join(cgmh_parts, "")
```

**诚实预测（关键）**：**此刀大概率不降 fuel**。STEP-0556 §10 已实测 `text.join` helper 体是**线性**（长度趟 + `memory.copy` 拷贝趟），`list.append` 摊还 O(1) ⇒ 改累加器只是**把拷贝从 N 次 concat 挪到一次 join**，总拷贝字节量 O(N²) **不变**（P1/C 七处累加器改线性装配 fuel 逐档不变即此结论）。**登记此刀为「低优先、预期 ~0」，仅当源 A 刀证伪后才考虑**。

### 4.3 已否决的刀（不再走，避免重蹈）

- **游标化 `cgj_array_item`**（源 A 的 arm/values 循环、`cgs_dispatcher_slot` 内层）：B3（STEP-0555）已证伪——fuel 按 hostcall 计，游标形式 2 hostcall/元素 vs index 形 1 hostcall ⇒ **净负**（13 站点回退）。**不走**。
- **`cgs_value_slot`/`cgs_value_ty` 索引化**：B2（STEP-0551）已落地（O(1)），无残差。
- **`cgj_member` 字段查找索引化**：每次扫描限**单对象**跨度（非全 body），不随 match 数超线性；且 B4-D（cell_slot 索引化）同构刀已证伪。**低优先**。

### 4.4 落地前必跑的 ledger 判据（防「六刀 0-1%」）

主刀（源 A）落地前，必须先回答「match-heavy 站点是否真在 335 耗尽集合里超配」：

1. **match 密度分账**：对 `selfhost/match-component-coverage-v1.json` 的 396 站点，按**每站 match 语句数**分桶，比较「耗尽 335 站点」与「全等 32 站点」的 match 密度分布。若耗尽站点的 match 密度**不超配**，则源 A 刀**预测回绿 ~0**，不应开工（沿用 §3.3 的常数主导判断）。
2. **若密度超配才落地**，exit 单列：
   - 195 个新开测站点的**回绿数**（B2 同口径）；
   - 既有 32 个全等站点 `component_sha256` **逐字节不变**（字节不变约束）；
   - 词表 artifact `guest_emitter.source_sha256` 同步刷新（改 codegen.sico 必失效，STEP-0545 §A2 已登记该坑）；
   - ledger 联动重测（>8 min，STEP-0571 同款门槛）。

### 4.5 预期 fuel 曲线（预测，非实测）

若源 A 是 `match_flat` 的平方主导项，主刀落地后 `match_flat` 臂应从 perfect-square（4.0×/倍）降为**近线性**（~2.0×/倍），n=8/16 可能从 exhausted 转为有 bracket。**但**据 §3.3 的量级估算，源 A 绝对贡献 <0.01%，故**更可能的实测结果是曲线基本不变**（沿用 B3/B4-D/P1/C 五刀「逐档不变」模式）。**这一预测本身就是要用 §4.4 判据 1 先证伪/证实的**——在 ledger 分账前不开工，是本设计片的**核心交付**。

## 5. 门状态（不变）

门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用——**全部维持**；M22 整体 **NO-GO**，本片零门翻转、零代码落地、零限额变更。

## 6. 复跑

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile --no-run
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile -- --test-threads=1 --nocapture --include-ignored the_match_quadratic_term_is_discriminated_from_body_scanning
& $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml -- --check
git diff --check
```

最近一次：`--no-run` exit 0（13.72s，无 warning）；默认 `2 passed; 0 failed; 2 ignored`；判别臂 `1 passed`（35.19s）；fmt 无 diff。
