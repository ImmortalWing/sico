# STEP-0340 — M22 W2 S3/S4: `sico.text.starts_with` 条件 + 5 函数翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**`gw_starts_with_condition_packed`（新函数，接入 `gw_condition_packed` 分派）**：general-while 机器的 if 条件现接受 `sico.text.starts_with(<expr>, "<literal>")`——parser.sico 中最普遍的条件形状（**185 处**，此前全部拒 `E-SH-IR-SCICOND`）。打包路径：

- 接收者 `<expr>`：参数（零指令）/ cell（read_local，string-kind 校验）/ 用户调用或复杂表达式（`gw_rhs_packed` 复用 + string-kind 校验）；
- 第二参：字符串字面量（`is_string_token` 校验 + `json_string_token` 解码 + `scalar_const_instruction_json` const_string）；
- 条件本体：`intrinsic sico.text.starts_with [recv_value, const_id]`（bool，range = [cond_start, 外层 close+1]）。

**id 记账钉死**：const/intrinsic 各占一格，intrinsic result = const_id + 1（首版按 `base+total` 误算一格，探针 EMIT-DIFF 定位后修正）；pack 消费约定与 `gw_compare_packed` 相同（`<result>\n<total>\n<pieces>`，total = recv_count + 2）。

**5 函数翻绿（parser 29→34/120，canary +5，R3 保持 0）**：`is_punct_byte`、`type_json`（14 函数普查前缀；type_json 此前拒 SCICOND）、`declared_return_kind`（15 函数前缀）、`binding_parameter_kind`、`parameter_kind_by_id`（28 函数全翻前缀）——mid-layer 连锁解锁。冻结语料按普查原组合登记（census14/census15/census28 三档前缀）。

**附带钉死**：嵌套用户调用实参（`scalar_type_kind(word_at(words, …))`）let-RHS 经既有 `gw_rhs_packed` 已字节全等（此前标记的"嵌套调用实参"残余部分关闭）；`declared_return_kind` 的 CALL-TARGET 拒绝实为缺 `scalar_type_kind` 被调（前缀补齐后达 SCICOND→本片翻绿），非机器缺口。

## 2. 验证

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler`：**58/58 绿**（含 5 个新冻结语料条目）。
- `git diff --check` 净；`scratch_bisect_probe.rs`/`scratch_census2_probe.rs` 按纪律删除。
- 钉扎保护：main-entry `sico.bytes.length(input.stdin)` 条件的 SCICOND 钉扎不受影响（新分支只认 text.starts_with）。

## 3. 前沿更新（不动点态）

- 剩余：34 CALL-TARGET / 20 CONTROL / 18 EXPRESSION / 6 STATEMENT / 6 GWPACK-OTHER / 5 CALL-ARGUMENT / MATCH-SHAPE / RETURN-COUNT——深层机器档（counters 的 while+多 match、机器本体自举、span/字面量助手）。
- `has_return_at` 待 else 分支守卫 + 嵌套 if（is_ident_byte 被调现已可用）。
- Bool 参数守卫（`if f:`，T2）未动。
- R3 0/4；M22 整体 NO-GO 不变（S6/S7 未进入）。
