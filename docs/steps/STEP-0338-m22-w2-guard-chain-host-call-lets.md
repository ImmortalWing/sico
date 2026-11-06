# STEP-0338 — M22 W2 S3/S4: 守卫链机器宿主调用 let + cell/宿主调用守卫操作数

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）
> - 编号说明：STEP-0330–0337 已由 M24 并行轨注册（worktree `sico-m24`，提交 `095227b`）；本片取下一个空闲号 0338。会话起点为 `1db4624`（此前中断会话的 gw-decline WIP 快照，本片验证其与 58/58 套件共存且承载 is_ident_byte 翻绿路径）。

## 1. 交付内容

守卫链机器（`fixed_guard_chain_ir`）三项加宽，全部走 oracle 字节对照后落地：

1. **宿主调用 let**：守卫之间/之前的 `let <name> = <expr>` 行经 `gw_rhs_packed` 复用打包（`sico.text.encode`/`sico.bytes.length`/`sico.bytes.at` 等），RHS 指令 + `write_local` 进入所在守卫条件块前缀；cell 注册采用 gw 同款三列表 + 独立 cell 计数（cell local = cell 下标）。
2. **条件打包器 `chain_condition_packed`**（新函数，`fixed_guard_condition_json`/`user_call_expression_json` 双 SKIP 后的第三 fallback）：左操作数支持 `sico.text.length`/`sico.bytes.length`（经 `gw_intrinsic_call_packed`）与 `sico.bytes.at`（经 `while_bytes_at_local_src_packed`，cell 接收者 + 字面量/参数/cell cursor）；右操作数支持参数/cell/字面量；其余形状原样委托字节已证的 `gw_compare_packed`。守卫条件与 final-return 比较两处接线。
3. **locals 发射**：有 cell 时函数 JSON 在 `"effects":[]` 与 `"entry"` 之间发射 `"locals":[{"name":…,"ty":…,"range":…}]`（range = 绑定名 token 跨度）；无 cell 时保持省略（既有字节不变）。

**翻绿（parser 17→20/115，canary +3，R3 保持 0）**：`is_digit_token`（encode/length 宿主调用 let + 宿主调用左值守卫 + cell 左值 final 比较）、`is_string_token`（双宿主调用 let + cell 左值守卫 + bytes.at 左值 final 比较）、`is_ident_byte`（control_if-declined → general-while 重试路径，7 守卫纯比较链）。前两者冻结语料以单函数原文与 Rust oracle 字节全等登记于 `selfhost_compiler::sico_compiler_lowers_parser_rearranged_function_subsets_byte_exactly`。

**附带验证修复**：`while_bytes_at_local_src_packed` 新增**字面量 cursor 形态**（form 2）——此前字面量 cursor 直接拒 `CALL-ARGUMENT`（STEP-0327 遗留"已实现但字节错误"疑点按 oracle 转储裁决：字面量 cursor → `const_u64`，布局与 cell-cursor 形态逐槽一致，`bytes.at` 实参表 [read_local, const_u64, const_i64]）。游标为字面量时按 `fixed_operation_operand_width` 宽度重走逗号。

## 2. 机器事实（本片钉死）

- **宽度约定**：`fixed_operation_operand_width` 返回操作数的 **token 数**；`atom + width` 落在操作数自身 `)` 之后的**下一个** token。守卫条件中字面量为末参，故 `right_idx + width` 恰为条件自身的 `)`（既有 `word_at(close)==")"` 检查成立的原因）；`bytes.at` 游标后为逗号，故逗号 = `cursor + width` 本身（非 next）。
- **cell local 编号**：守卫链机器 cell 的 local id = cell 注册下标（与 gw 的 `gcell_count` 方案一致），与指令 result id（自 `parameter_count` 起）分域。
- **oracle 对照法**：`rust_ir`（`lower_core` + `canonical_json`）直接转储期望 IR，逐指令对齐后实现，禁止猜测 id 分配。is_digit_token/is_string_token 的 oracle 全文已入探针记录。
- **`fixed_operation_operand_width` 的 "sico" 宽度规则（1db4624 移植件）存疑**：其对 `sico.<family>.<member>` 以 +4 判 `(`，而实际 open 在 +5——当前不在任何已证字节路径上（`chain_condition_packed` 经 close/逗号直接走位），**待后续片修复或按 oracle 验证**，不得在修复前扩用。
- **scalar_ir 门梯死代码**：general_route 分支中 gw SKIP 后的 `control_while_function_ir` 调用不可达（两分支均 continue/return）；gw 机器才是 while 族唯一入口，`control_if` 内部先试 `control_while`。
- **拒绝面迁移**：`is_digit_token`/`is_string_token` EXPRESSION→**翻绿**；`has_return_at` EXPRESSION→CALL-TARGET（需用户调用 let（`next_index`）+ else 分支守卫 + 参数接收者 bytes.at——下片候选）；`json_string_token` 维持（while+match 嵌套，CONTROL 级，需 match 栈机器——独立片）；`lossless_tokens`/`append_metadata` 在 compiler_parser.sico（不在 parser.sico）。

## 3. 验证

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler`：**58/58 绿**（含新冻结语料 3 函数字节全等）。
- 完整 runner 套件：见提交时记录（后台全量跑）。
- 探针纪律：临时标记二分（`ERR:MARK-*`）定位 `bytes.at` 字面量游标逗号走位后全部还原；`scratch_step0330_probe.rs`（含 oracle 转储/字节对比/普查三测试）按"探针不提交"纪律删除。
- `git diff --check`：提交前净。

## 4. 残余与下片

- `has_return_at` 翻绿片：用户调用 let + else 守卫 + 参数接收者 at（守卫链再扩 or gw 直连）。
- `json_string_token`/嵌套 match CONTROL 族：match 栈机器（STEP-0326 残余映射的"嵌套 match 落在开臂内"）。
- 守卫链 final 位置 let（三函数不需要，暂拒 `SKIP:FINAL-RETURN` 不变）。
- Bool 参数守卫（`if f:` → read_local bool，无比较操作）——T2 族入口，仍未动。
- R3 止损计数 0/4；M22 整体 NO-GO（S6/S7 未进入，外部门不变）。
