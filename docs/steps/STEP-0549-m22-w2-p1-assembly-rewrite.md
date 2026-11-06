# STEP-0549 m22-w2 P1 **装配改写片**：四个自左折叠换成「分片 `List[Text]` + 边界一次 `join`」，64 KiB 字面量由 `unreachable` trap 变为字节全等

| 项 | 值 |
|---|---|
| 片型 | P1 实现：`selfhost/codegen.sico` 四个装配函数的累加器改写 + 1 条新常驻钉（`runner/sico-runner/tests/selfhost_compiler.rs`）；`crates/`、`wit/` 零改动 |
| 决策门 | 依 STEP-0548 的 **D1 = 绿** 开工；本片是 D1 判读的第一个落地片 |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**；本片不改变任何门的结论 |
| 证据等级 | `internal-fixture`：本机 2026-10-26，Windows x64，debug 测试构建，`sico_runner` 限额定为合格跑值（fuel 800e9 / timeout 120 s / 内存 2560 MiB / hostcall 8 GiB） |

## 1. 要解决的事

STEP-0547 判定墙在装配：`selfhost/codegen.sico` 的 hex 文本用 `set X = sico.text.concat(X, …)` 左折叠累加，单次 `concat` 拷贝整段，故输出 N 字节要 O(N²) 拷贝。STEP-0544 的 Y 族把它量成「每翻倍字面量墙钟 ×2.45、24,576 B 起 trap」；STEP-0548 的 S4 证明「分片 `list.append` + 一次 `join`」在 1 MiB 只要 42,474,275 fuel（严格线性 40.5 fuel/字节）。

## 2. 改了什么（`selfhost/codegen.sico`，54 插入 / 51 删除）

只改**在 gate-3 的 1 MiB 路径上升维**的四个累加器：迭代次数随程序规模增长的折叠。

| 函数 | 行 | 累加器 → 分片列表 | 迭代基线 | 改写后边界 |
|---|---|---|---|---|
| `function cgj_string_hex(bytes: Bytes, start: U64, close: U64) returns Text:` | 2719 | `cjsh_out` → `cjsh_parts` | **每个字面量字节**一次 | `:2768 return sico.text.join(cjsh_parts, "")` |
| `function cgs_function_instr_hex(bytes: Bytes, fn: U64, terms: U64) returns Text:` | 3907 | `crih_out` → `crih_parts` | 每条指令一次 | `:3972` |
| `function cgs_data_hex(bytes: Bytes) returns Text:` | 4738 | `cdh_out` → `cdh_parts` | 每个字面量一次 | `:4798` |
| `function cgs_core_code_hex(bytes: Bytes, arena_base: U64) returns Text:` | 6453 | `cccsh_out` → `cccsh_parts` | 每个函数体一次 | `:6641` |

- 43 处 `set X = sico.text.concat(X, …)` 改为 `set X_parts = sico.list.append(X_parts, …)`，加 4 处边界 `sico.text.join(X_parts, "")`；初值 `let X = ""` / `let X = cg_leb_hex(…)` 改为 `let X_parts = sico.text.split_lines("")` 加一次 `append`。
- 分片顺序、分片内容、早退分支（`return ""` / `return "ERR:…"` / `return cccsh_body`）全部不变 ⇒ 输出文本逐字节不变，这是本片唯一的正确性主张。
- `sico.text.concat` 引用 512 → 468；`sico.list.append` 站点 3 → 67；`sico.text.join` **调用点** 0 → 4（其余 9 处是 `cg_is_word(…, "sico.text.join")` 的识别字面量）。
- **刻意不改**：`cg_component_hex`（9 段）、`cg_core_module_hex`（6 段）这类「段数固定、每段本身是 MiB 级成品」的折叠 —— 它们的迭代次数与程序规模无关，段数 × 段长是常数倍，不是二次项。`cgp_run_plan` 的 `cgpr_text` 是计划文本，不在组件字节路径上。

## 3. 新常驻钉（`runner/sico-runner/tests/selfhost_compiler.rs`）

`fn sico_codegen_guest_emits_a_large_literal_component_byte_exactly()`：`large_literal_source(bytes)` 生成「Script ABI 前奏 + 一个带 `bytes` 字节字面量的普通函数 `label` + 冻结的 `run`」，阶梯 **16 KiB / 64 KiB / 256 KiB**。每级都要求：guest `--emit-component-hex` 成功（exit 0）→ `decode_hex` → 与 `codegen_oracle::compile_script_program` 的 Rust 字节**逐字节相等** → `wasmparser::Validator::validate_all` 通过。

A/B 实测（同一台机器、同一限额、`include_str!` 重编后）：

| 字面量 | 改前（HEAD `4449efa` 的 codegen.sico） | 改后 | 组件字节 |
|---|---|---|---|
| 16 KiB | 通过（6,355 ms） | 通过（4,474 ms） | 17,436 |
| **64 KiB** | **`Trap(UnreachableCodeReached)`**（`downcast=Some(UnreachableCodeReached)`） | **通过（458 ms）** | 66,588 |
| 256 KiB | 未达（改前先 trap） | 通过（2,261 ms） | 263,196 |

- 失败类型是 **arena/分配 trap（Wasm `unreachable`）**，不是 `FuelExhausted`：与 STEP-0544 的分类一致（A 路是时间/内存墙，B 路才是 fuel 墙）。
- 64 KiB 的点在改后比 16 KiB 更快，是进程内首次 guest 实例化的一次性开销（16 KiB 那格含预热），不是超线性。

## 4. 回归

- `tools/validate-m22-s5-dev.ps1 -Set fast` → **`DEV_SUBSET_GREEN 8/8`**，指纹 `1ed42e580eba8bcd9a41dc674cb1b738612997245c74cb5cce0bb38b9b0ba45d`（fmt ×2、clippy ×2、comment-discipline、match-site-census、emitter-vocabulary-census、git-diff-check）。
- `selfhost/emitter-vocabulary-v1.json` 由 `census-m22-emitter-vocabulary.py --update` 重生成：**只有 `source_sha256` 变**（`da0fec5f…` → `6be9bf98…`），145 个与词表相关的站点不变 —— 说明本片没有新增发射形状，只换了装配策略。
- `tools/validate-m22-s5-dev.ps1 -Set firmware` → **`DEV_SUBSET_GREEN 11/11`**（同指纹 `1ed42e58…`；新增三步 `probe-frontend-frontier` 681 s、`probe-ir-plan-frontier` 35 s、`firmware-selfhost` 2,061 s）；`firmware-selfhost` = `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` 全绿，即 396 站点账本里 32 个 `component-equal` 站点在 P1 之后仍逐字节相等，另有 16 / 64 / 256 KiB 大字面量阶梯钉同轮通过。**该子集不是出口证据**（完整 15 拍电池 2.7 h 未跑）。

## 5. 边界（本片**没有**证明的事）

- **只关了 A 墙。** B 墙（`cgs_value_slot` → `cgj_array_item` 对 IR 的逐指令 O(index) 重扫，STEP-0544 §6）未动；140 个 `FuelExhausted` 站点的 artifact 只有 1–5 KB，其成本来自 IR 扫描而不是装配，**本片后它们仍然会 `FuelExhausted`**。
- **396 站点账本未复算**，这是 P2 的事；本片不声称任何站点 verdict 改变。
- **219 个 `closure-too-large` 未动**（`runner/sico-runner/tests/selfhost_compiler.rs:9286-9287` 的夹具上限 64 fn / 256 KiB）。
- **1 MiB 组件没测**：阶梯到 256 KiB 字面量（263,196 B 组件）为止。S4 的线性证据（40.5 fuel/字节、1 MiB = 42.5 M fuel）支持外推，但外推不是测量。
- `cg_component_hex` / `cg_core_module_hex` 的段级折叠仍是 `concat`：段数固定（9 / 6），1 MiB 组件下约 9 × 1 MiB 拷贝，可接受但未测。
- 本片**未推送**：`codex/m22-w2` 仍领先 origin，推送需 owner 授权。

## 6. 下一步

- **P2（批量复算）**：把 `MATCH_CLOSURE_FUNCTION_LIMIT` / `MATCH_CLOSURE_BYTE_LIMIT` 提到 128 fn / 1 MiB，重跑 396 站点账本，并与本片前的 32/140/5/219 逐条对比。判读分两种：站点 verdict 变化必须归因到具体墙；不变则说明墙仍主导、P2 不能当完成。
- B 墙（`cgi_*` 索引或游标化）作为 P2 之后的独立小片，不并入 P1 的改动面。
