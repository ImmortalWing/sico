# STEP-0558 m22-w2 **W4 声音洞修复：record 构造缺字段必拒（missing-field/E2010）**

| 项 | 值 |
|---|---|
| 片型 | **实现片**：修复 STEP-0557 发现的 W4 声音洞——guest 前端把 Rust oracle 以 `E2010`（missing field）拒绝的缺字段 record 构造降为 canonical IR。修复 = 新增字段完备性检查，**只落在 record 构造路由**（`record_construct_function_ir`），不触碰共享的 variant 载荷路由 |
| exit | **达成**：全量 150 项复跑 `checked=150, mismatch=3`（3 燃料墙）、**`lowered=0`**（`missing-field.sico` 由 `lowered` 翻 `refused:ERR:E-SH-IR-CONSTRUCT-FIELD`，parity 146→**147/150**）；runner 固件 181 passed / 5 failed（4 个 CALL-SHAPE = 既有红 + 1 个 MS-225 ledger 行移位 = 本修复后续项，见 §4.2/§4.3） |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**（无变化） |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 测试构建，`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB） |

## 1. 问题（STEP-0557 的 1 个声音洞）

STEP-0557 W4 verdict parity 发现 1 项 `lowered`（越权接受）：

> `syntax-candidates/b/nominal-invariants/invalid/missing-field.sico`：`record Feature: field name: Text; field enabled: Bool`，`main` 只构造 `Feature(name: "search")`（缺 `enabled`）。Rust oracle 拒绝 `E2010`（missing field）；guest 前端把它降为 canonical IR（`op":"construct"`，`fields:[{"name":"name",...}]`）。

**根因**：`construct_lowering`（`parser.sico:11206`）对**每个提供的字段**检查其是否已声明（`record_field_type_word` 为空 ⇒ `ERR:E-SH-IR-CONSTRUCT-FIELD`，即拒未知字段），但**从不检查已声明字段是否全部被提供**——缺字段构造静默通过。

## 2. 修复（`selfhost/parser.sico`）

- **新助手 `record_missing_field_name(words, record_name, provided)`**（插入于 `record_construct_function_ir` 前）：按 `record_field_kind`/`record_field_type_word` 的既有 record 声明遍历模式，枚举 `record <name>:` 下每个 `field <name>: <type>`，返回**第一个未出现在 `provided` 字段名列表中的已声明字段名**；全部提供（或 record 无字段/未声明）返回 `""`。
- **检查点放在共享的 `construct_lowering`**（不是 record 路由）：字段循环里把每个 `cl_field_name` 追加进 `cl_provided`（`sico.list.append`，空列表 = `sico.text.split_lines("")`）；循环后、发 IR 前，若 `record_declared(words, cl_record_name)` 且 `record_missing_field_name(...)` 非空 ⇒ `return "ERR:E-SH-IR-CONSTRUCT-FIELD"`。
- **作用域裁决**：检查**只在 `record_declared(cl_record_name)` 为真时**触发——既覆盖 record 构造路由（`record_construct_function_ir` → `construct_lowering`），也覆盖 variant 内含 record 载荷（`record_variant_function_ir`:11520 → `construct_lowering`，`cl_record_name` = 载荷首词，为已声明 record 时触发）；非 record 载荷（字面量/enum case）`record_declared` 为假 ⇒ 不触发。**语言无默认字段值**（前端无 default-field 支持）⇒ 已声明字段全部必供，严格完备性检查与语义一致。
- **基线**（开工前）：HEAD `33cc283386c8178adf67391ea8d15bd37878636b`；`parser.sico` SHA 前 `3550C84E262B0187ED9A386B67AB652F28B70F4D28616B12815629F30FAEF54C` → 中途（record 路由版）`E2BB7740E8FF834AA51888C55D192B6EFB5A832811031577475B1CBB7B25A211` → 最终（construct_lowering 版）`CE0A74811CC62501BF733B42C415FE68DC6BDBC4E02BA7976250B27A420C56CA`；`parser.sico` 无并发写入方（工作树干净）。

## 3. 验证（关键用例，本机直接跑 guest，最终版本）

| 用例 | 源 | 结果 | 判定 |
|---|---|---|---|
| A 完整 1 字段 | `OnlyName(name: "x")` | `{"schema":"sico.ir.v0",...}` | **仍降（无回归）** |
| B 空实参 | `OnlyName()` | `ERR:E-SH-IR-CONSTRUCT-FIELD` | 缺 `name` 必拒 |
| C 缺字段（目标） | `Feature(name: "search")` | `ERR:E-SH-IR-CONSTRUCT-FIELD` | **由 `lowered` 翻 `refused`** |
| D 未知字段 | `OnlyName(nope: "x")` | `ERR:E-SH-IR-CONSTRUCT-FIELD` | 既有行为保持 |
| V1 variant 缺 `message` | `error(ScriptError(code: ...))` | `ERR:E-SH-IR-CONSTRUCT-FIELD` | **第二个洞也关**（原 `lowered`） |
| V2 variant 完整 | `error(ScriptError(code:..., message:"A"))` | `{"schema":"sico.ir.v0",...}` | **仍降（无回归）** |

**发现**：除 corpus 的 record 路由洞外，**variant 内含 record 载荷**（`error(ScriptError(...))` / `ok(ScriptOutput(...))`，即固件 `sico_compiler_lowers_error_return_variant_construct_byte_exactly`/`ok_return_construct_byte_exactly` 的形状）有**同一类洞**（V1 实测 `lowered`）——corpus 未覆盖（无此类拒绝项），但语义同类。最终版检查点放在共享 `construct_lowering` ⇒ **两处一起关**；V1/V2 实测证明。

**方向安全性**：本片只**新增拒绝路径**（`return "ERR:..."`），**不新增任何降级路径** ⇒ 能降（lowered）的语料集合**只会缩小**，verdict parity **只会改善或保持**，不会把 `refused` 翻成 `lowered`。残余回归风险 = 新代码在其它语料上 trap/挂起 ⇒ 由全量 150 项复跑（`bash-oai2r7fz`）兜底（见 §4 待回填）。

## 4. 结果（全量 150 项复跑 + runner 固件回归）

### 4.1 全量 150 项 parity 复跑（最终版）

`tools/probe-m22-corpus-parity.ps1` 全量重跑（`bash-oai2r7fz`，2026-11-05 本机，`sico_runner` 限额不变：fuel 800e9 / timeout 120 s / 内存 2560 MiB），重写 `selfhost/corpus-parity-v0.json`（前置工件备份 `/tmp/corpus-parity-v0.prefix.json`，35453 B）。

| 汇总 | 修复前（STEP-0557） | 修复后（本片） |
|---|---|---|
| `checked_count` | 150 | **150** |
| `mismatch_count` | 4 | **3** |
| `lowered` 数 | 1 | **0** |
| `match` 数 | 146 | **147** |

**3 个 `mismatch` 全部为燃料墙**（与修复前相同、与 verdict 正交）：`semantic-cases/future-task/invalid/scope-nesting-limit.sico`（LEXICAL）、`syntax-candidates/b/future-task/invalid/scope-nesting-limit.sico`（E5105）、`syntax-candidates/c/future-task/invalid/scope-nesting-limit.sico`（LEXICAL），均 `failed:…FuelExhausted…exit:125`。

**关键翻转**：`syntax-candidates/b/nominal-invariants/invalid/missing-field.sico`（rust `E2010`）由 `lowered`（修复前越权接受）翻为 **`refused:ERR:E-SH-IR-CONSTRUCT-FIELD`（parity `match`）**——**W4 声音洞关闭**（工件该条 `guest_verdict` 已逐字确认）。**无回归**：其余 147 项 `refused:ERR:E-SH-IR-*` 全 `match`；工件中无任何新增 `lowered`/非燃料墙 `failed`/trap；原 3 燃料墙不变。

### 4.2 runner `selfhost_compiler` 固件回归（`bash-0vrpy5cg`）

**181 passed / 5 failed / 5 ignored**（`--test-threads=1`，约 272 s + 构建）。5 失败逐项判定：

| 测试 | 失败文本 | 判定 |
|---|---|---|
| `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly` | `codegen function … must compile through the guest frontend` | **既有红** |
| `…step0519_codegen_functions_byte_exactly` | `cgs_value_ty … ERR:E-SH-IR-CALL-SHAPE` | **既有红** |
| `…step0520_codegen_functions_byte_exactly` | `cgs_value_slot …`（CALL-SHAPE） | **既有红** |
| `…step0526_codegen_functions_byte_exactly` | `cgs_function_instr_hex …`（CALL-SHAPE） | **既有红** |
| `sico_compiler_reproduces_the_frozen_match_site_ledger` | `MS-225: the frozen site line no longer holds the match head` | **本修复引入的 ledger 行移位**（见 §4.3） |

**4 个 CALL-SHAPE 既有红判定（实证）**：换回 HEAD 版 `parser.sico`（`git show HEAD:… > parser.sico`）→ 强制重编译测试二进制（`selfhost_compiler.rs:23` 的 `include_str!` 烘焙 `parser.sico`；重建二进制 mtime 21:03，`record_missing_field_name`（本修复新增函数名）在二进制中 0 次 ⇒ 确认烘焙 HEAD 版）→ 重跑 5 测试 ⇒ 4 个 CALL-SHAPE 测试**仍 FAILED** ⇒ **既有红**（HEAD 版同样失败，非本修复回归）。逻辑一致：本修复 diff 只新增 `CONSTRUCT-FIELD` 拒绝路径（0 处 `CALL-SHAPE`、0 删除），且若新检查触发，错误码是 `CONSTRUCT-FIELD` 而非 `CALL-SHAPE`。**后续收窄（[`STEP-0559`](./steps/STEP-0559-m22-w2-callshape-u64-list-get-frontend-gap.md)）**：其中 `step0519`/`step0520`（`cgs_value_ty`/`cgs_value_slot`）的根因已收窄为「guest 前端无法 materialize + match `sico.list.get[U64]` 的 `Result[U64,_]`」（前端能力缺口，非语法重塑可修），维持既有红；`step0507`/`step0526` 待同法收窄。

### 4.3 MS-225 ledger（本修复的行移位后果 → 已收口）

本修复在 `selfhost/parser.sico` 插入字段完备性检查（新助手 `record_missing_field_name` + `construct_lowering` 字段循环内的完备性检查行），使冻结 ledger `selfhost/match-component-coverage-v1.json`（schema `sico.m22.match-coverage.v1`，396 站点）的 **172 个 `parser.sico` 站点行号错位**（精确分布：**169×(+91)、2×(+2)、1×(+8)**；`construct_lowering` 自身 `function_line` 不变仍 11206，其内 3 站点 `MS-225/226/227` 行号 +2/+8）。实证：387 个 `parser.sico` 站点在 **HEAD 版** `parser.sico` 下 `site.line` 行全部仍含 `match…checked_add`（0 失败）⇒ MS-225 在 HEAD 版下**通过**；修复版下错位 ⇒ MS-225 报 `MS-225: the frozen site line no longer holds the match head`。**判定：MS-225 为修复引入的行移位回归，非既有红**。

**收口（本片后续执行，已实证）**：按 M22 源编辑后标准步骤，用 `tools/census-m22-match-sites.py` 重推导 ledger——站点身份 = `path:function:ordinal`（与行号无关），重推导自动保留未动站点全部字段；备份 `/tmp/match-coverage-v1.prefix.json`，`census --check` = `LEDGER_CURRENT`（396 站点），**172 站点行号变、0 站点 verdict/形状/参数变、3 站点 digest 变**（变更完全落在预期范围）。`construct_lowering` 闭包文本已变 ⇒ 其内 3 站点的 `source/ir/component` 哈希需重测量：用临时探针 `match_compare_site("parser.sico","construct_lowering",…)` 单测该闭包（烘焙**修改版** `parser.sico`；探针已回退，测试文件回 HEAD）——verdict **不变**（`ir-equal+component-other:FuelExhausted`，与修复前同——修复不改变燃料墙）、digest 刷新为 `bc568163…`/`6caab772…`/`90f24262…`（旧 `7f97c080…`/`b3b36e48…`/`2b781624…`），经 `--update` 合并进 ledger（MS-225/226/227 三站点已更新）。其余 92 组函数闭包逐字节未变（仅 `construct_lowering` 变）⇒ 其 digests 保留即有效。**MS-225 完整复跑（权威验证器）已确认绿：95 组全匹配（396 站点），`test result: ok. 1 passed; 0 failed`，EXIT=0，6114.51s，修正后 ledger 被当前二进制完整复现；本节定稿**。

### 4.4 本片结论

- **W4 声音洞关闭**：`missing-field`/`E2010` 由 `lowered` 翻 `refused:ERR:E-SH-IR-CONSTRUCT-FIELD`；parity 146→**147/150**；`lowered=0`；无 corpus 回归。
- **固件**：4 个 CALL-SHAPE 失败 = **既有红**（实证）；MS-225 = 本修复的 ledger 行移位后果（登记后续重推导，见 §4.3）。
- **方向安全性成立**：本片只新增拒绝路径，能降集合只缩小；150 项复跑无新 trap/挂起。
- **M22 仍 NO-GO、七门不变**（门 1/5/6 GO、门 2 PARTIAL、门 3/6 NO-GO、门 4 未恢复）；组件轴 36/396、331 `FuelExhausted` + 24 8MiB trap + 5 refused-guest + 4 既有红（CALL-SHAPE）+ MS-225 ledger 行移位（后续项）+ 7 词表缺口。
