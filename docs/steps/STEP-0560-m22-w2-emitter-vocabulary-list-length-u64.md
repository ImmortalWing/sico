# STEP-0560 · M22 W2 · 发射词表 C 补刀：`sico.list.length[U64]` 完整 collection-helper 发射

> **本片性质** = 发射词表 C 的**第 7 项（末项）集合 helper** 落地：`sico.list.length[U64]`。
> **不推进任何出口门**；**`crates/*` 零改动**；**`selfhost/parser.sico` 零改动**（只动发射端 ⇒ 前端前沿不动）；**M22 仍为 NO-GO**；**`match` 族仍未闭合** ⇒ **D10 不得降级**。
> **与 STEP-0542 的关系** = STEP-0542 落地了 6 项集合 helper（append / append[U64] / empty[U64] / get[U64] / text.join / text.split_lines）把**缺的算子**清零；本片补上被漏的第 7 项 `sico.list.length[U64]`（它**不在** `cgs_collection_call_hex` 的已知名字表里，是**独立的 collection-helper**，发射形状与 get/empty 同族但调用点形状不同）。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| STEP-0559 已定稿（诊断片，工作树 codegen.sico 无 diff） | ✅ | STEP-0559 §4：回退全部实验，登记 `List[U64]` Result-match 前端能力缺口 |
| oracle 生产代码零改动 | ✅ | `git diff --stat` 只含 `selfhost/`、`runner/`、`tools/`、`docs/` |
| 本片对象是真实词表缺口 | ✅ | STEP-0550 披露：340 站点口径下 `sico.list.length[U64]` 在 **7 个 `general_while_function_ir` 站点**上缺失（145 站点口径不显示） |

## 1. 本片方法

1. **先读冻结 oracle 源码**（不复刻、不猜）：`crates/sico-codegen-wasm/src/stdlib.rs:1093` 的 `emit_collection_length`
   ```rust
   fn emit_collection_length() -> Function {
       let mut body = Function::new(vec![]);
       ops!(body;
           Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::End,
       );
       body
   }
   ```
   ⇒ helper 体 = `local.get 0; local.get 1; end` = **`200020010b`**（加 0-decl 前缀 `00` = **`00200020010b`**，**无 `call` ⇒ 可逐字冻结**）；helper 签名 `(i32,i32)->(i32,i32)` ⇒ 类型段 **`60027f7f027f7f`**（透传 `(table, count)` 对）。
2. **调用点形状**（`emit_collection_call` 的 ListLength 臂）：helper 返回 `(table, count)` 对，调用点**只保留 count** 并 `i64.extend_i32_u`（`ad`）升成标量 U64 结果槽；被丢弃的 table 在 return 时留在操作数栈上（与 oracle 一致）。字节序列：
   `local.get table(slot-1)` · `local.get count(slot)` · `call helper` · `local.set scratch(dispatcher+1)` · `local.get scratch` · `ad` · `local.set result0`。
3. **按 STEP-0542 既有形状落地**：helper 体函数 → `cgs_helper_index` 的**名字排序位**（`length[U64]` 排在 `get[U64]` 之后、`text.concat` 之前）→ **类型段 / 函数段 / code 段**三处接线 → 独立发射器 `cgs_list_length_u64_hex` → `cgs_intrinsic_hex` 的分派行。
4. **闸门全 fail-closed**：实参个数 = 1 / 首实参存在 / 无第二实参 / 首实参是 `List[U64]`（`cgs_collection_arg_is_list_of`）/ 结果槽数 = 1 / helper 索引 ≠ 0 / dispatcher ≠ 0；任一不过 ⇒ `ERR:E-SH-CG-IR-EMIT`。
5. **自举形状前沿纪律**：发射器里**不得出现 call 作 call 实参**（STEP-0507 形状规则）。`cgs_type_slots(bytes, cgj_member(...))` 与 `U64.equal(cgs_collection_arg_is_list_of(...), 0)` 两处嵌套**一律先 `let` 提升**再传。

## 2. 实测结果

### 2.1 合同（读自冻结源码）

| 项 | 值 | 来源 |
|---|---|---|
| oracle 函数 | `emit_collection_length()` | `stdlib.rs:1093` |
| helper 体 | `00200020010b`（`local.get 0; local.get 1; end`） | `stdlib.rs:1095-1097` |
| helper 类型 | `(i32,i32)->(i32,i32)` = `60027f7f027f7f` | `stdlib.rs:989`（ListLength 归入 `(...)->(i32,i32)` 组） |
| IR 实参 | `List[U64]` = 2 flat 槽 [table, count] | `cgs_collection_arg_is_list_of(..., "u64")` |
| IR 结果 | `U64` = 1 槽 | `cgs_type_slots(... instr.ty) == 1` |

### 2.2 落点（`selfhost/codegen.sico` **+97 行**）

| 站点 | 改动 |
|---|---|
| `cg_list_length_helper_body_hex()` | **新增**：返回冻结体 `00200020010b` |
| `cgs_list_length_u64_hex(bytes, run, instr, data, result0, vindex)` | **新增**：7 道闸门 + 调用点字节发射 |
| `cgs_intrinsic_hex` 分派 | 加 `sico.list.length[U64]` → `cgs_list_length_u64_hex` 臂 |
| `cgs_helper_index` | 加 `length[U64]` 排序位（`get[U64]` 之后）+ rank 递增 |
| `cgs_core_types_hex` | 加 flag + 总数递增 + 类型条目 `60027f7f027f7f` |
| `cgs_core_funcs_hex` | 加 flag + 总数递增 |
| `cgs_core_code_hex` | 加 flag + 总数递增 + helper 体发射块 |

### 2.3 逐字节钉（TDD，红→绿）

新钉 `the_emitter_vocabulary_c_covers_the_list_length_u64_shape`：
- 源：`function render(items: List[U64]) returns U64: return sico.list.length[U64](items) end function`
- oracle 组件 **1048 B**，两侧都过真实 `wasmparser::Validator`
- 钉**先红**（guest 1040 B / 缺 helper fn5 / 类型段签名错）→ 逐项修 → **绿（1048 B 逐字节等同）**

**钉在交付前抓到的 3 个真实缺陷**：

1. **缺 type 段条目**：`cgs_core_types_hex` 未发 length 类型 ⇒ 组件 1040 B（差 8 B 空函数体条目）。
2. **缺 funcs 段计数**：`cgs_core_funcs_hex` 未递增 ⇒ 函数索引错位。
3. **helper 类型签名写错**：初版写成 `60027f7f017f7f`（`(i32,i32)->(i32)`，1 结果）⇒ oracle 是 `60027f7f027f7f`（2 结果）。由**逐字节钉**在偏移 **408**（type 段 length 条目的返回值个数：guest `01` vs oracle `02`）抓到。**「hex 不得手改、以 oracle 为准」**这条 STEP-0542 纪律再次兑现。

### 2.4 词表刷新

- `tools/census-m22-emitter-vocabulary.py` 的 `GUEST_HELPER_BODIES` 登记第 13 项：`"sico.list.length[U64]": "cg_list_length_helper_body_hex"`（否则普查报 `unlisted=['cg_list_length_helper_body_hex']`）。
- `selfhost/emitter-vocabulary-v1.json` 由 `--update` 刷新（`guest_emitter.source_sha256` 随 codegen.sico 变更），实测 `EMITTER_VOCABULARY_WRITTEN sites=336`；`sico.list.length[U64]` 从各站点 `missing_helpers` 移除（→ `[]`）。

## 3. 诚实收窄

- **组件级全等仍未测量**：本片只交付**发射能力**（length 形状可逐字节发射），未跑组件轴逐站差分 ⇒ 那 7 个 `general_while_function_ir` 站点是否真闭合**待 P3 实测**（与 STEP-0542 的「发射能力 ≠ 站点闭合」同口径）。
- **5 个 pre-existing 红测（非本片引入，已用 `git stash` 基线实证）**：`step0507`（`cgs_result_return_hex` → `E-SH-IR-EXPRESSION`）/ `step0519`（`cgs_value_ty` → `E-SH-IR-CALL-SHAPE`）/ `step0520`（`cgs_value_slot`）/ `step0526`（`cgs_function_instr_hex`）/ MS-225 ledger。这些是 STEP-0558/0559 记录的 CALL-SHAPE / 前端能力缺口既有红，本片**未新增**任何失败（我的改动只加新函数 + length 发射，不改既有函数体）。
- **`cgs_list_length_u64_hex` 的两处嵌套 call 已 hoist** ⇒ 该函数自身在自举前沿内；但**未把它加进自举前沿钉的函数清单**（STEP-0542 的 31 函数钉未扩），留待下一片统一收口。
- **注释纪律校验器 RED（pre-existing，非本片引入）**：`tools/validate-selfhost-comment-discipline.py` 报 `SELFHOST_COMMENT_DISCIPLINE_RED`（EXIT=1），违规在 `selfhost/parser.sico:11364-11365`（STEP-0558 未提交的 `record_missing_field_name` 函数体内注释含结构词 `record`）。stash 掉本片 codegen.sico 改动后 RED **依然存在** ⇒ 归 STEP-0558 的既有工作树改动，**不在本片范围**（本片只改 codegen.sico，未引入任何新违规）。
- **runner clippy 未在本机跑（环境限制，非本片引入）**：`cargo clippy` 需从源码编 `wasmtime-internal-fiber` 的 C 构件 ⇒ cc-rs 找 `gcc.exe`，而本机（Git Bash host）无 mingw gcc（msys2-pkgs 空、无 msys64/mingw64；rustup gnu 自带的是 `x86_64-w64-mingw32-gcc.exe`，且缺运行时 DLL）。本片 Rust 改动（仅测试文件）此前 `cargo test` 已编译通过并跑绿（用缓存的 wasmtime 构件，不走 gcc 重编）⇒ 改动本身有效；完整 clippy 待有 gcc 的环境/独立 CI 复验。`.sico`/`.py`/文档不被 clippy 检查。
- **`match` 族未闭合**、门 3 NO-GO、M22 NO-GO、七门不变。

## 4. 本片结论

- **`sico.list.length[U64]` 发射词表缺口闭合**：第 7 项（末项）集合 helper 落地，length 形状整组件与 oracle **逐字节等同**（1048 B）+ 过真实 validator。
- **词表缺口 7 → 0**（340 站点口径的最后一个名字；站点级闭合待 P3 组件轴实测）。
- **钉**：+1 条逐字节钉（length shape）；census 表 +1 项 helper 登记。
- **不翻门**；**不承诺站点闭合**；**未提交未推送**。
