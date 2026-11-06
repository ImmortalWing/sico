# STEP-0339 — M22 W2 S3/S4: 全量函数普查前沿 + 9 函数翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**全量普查（4 pass 至不动点）**：对 `parser.sico` 全部函数逐个经 guest 编译器差分（pass 1 单独、pass 2/3/4 递增被调前缀，oracle 字节对比），得到精确前沿：

- **pass 1（单独）**：8 FLIP（全部已登记）/ 77 CALL-TARGET（缺被调）/ 15 CONTROL / 12 EXPRESSION / 5 GWPACK-OTHER 等零星。
- **pass 2（4 被调前缀 `is_word`/`word_at`/`next_index`/`line_end`）**：15 FLIP = 8 已登记 + **7 新翻**：`is_declaration_start`、`literal_value_index`、`list_aware_parameter_index`、`is_intrinsic_path`、`record_scalar_kind`、`record_close_bracket`、`record_map_comma`。
- **pass 3（11 函数前缀 = 前缀 + pass2 七个）**：+2 新翻：`function_declaration_index`、`function_declaration_ordinal`。
- **pass 4（14 函数前缀，含 is_ident_byte）**：零新增——**不动点到达**。

**9 个新翻绿**全部经既有机器字节全等，无新机器手术；冻结语料按普查原组合登记（7 个用 `line_end` 前缀、2 个用 11 函数前缀）。parser 覆盖 **20→29**。

## 2. 分母修正（登记差异）

覆盖率分母此前各步写作 "/115"，实测函数头计数：STEP-0319/0321 时点为 **117**、STEP-0324 为 118、STEP-0325/0327 为 **119**、本片（新增 `chain_condition_packed`）为 **120**。历史 "/115" 为陈旧口径，自本片起以实测计数为准：**parser 29/120**；历史条目不改写（当时快照），以本条为修正登记。

## 3. 前沿地图（不动点态）

- **41 函数 CALL-TARGET**（传递闭包阻塞在更深被调）：`lex_words`、`parameters_ir`、`declared_return_kind`、`binding_parameter_kind`、`parameter_kind_by_id`、`fixed_operation_operand_atom_idx/value`、`argument_segment_kind`、三台机器本体（`fixed_guard_chain_ir`/`direct_bytes_equal_encode_ir`/gw 族打包器）与 `scalar_ir`——这些即 mid-layer 突破口：翻掉任一层即解锁其消费者。
- **机器级缺口**（普查探针证据）：`has_return_at` 需 is_ident_byte 前缀 + **else 分支守卫**（then 臂写 cell、else 臂 return）+ 嵌套 if（其 CALL-TARGET 来自缺 `is_ident_byte` 被调）；`json_string_token` 维持 while+match CONTROL（match 栈机器）；`u64_list_at` MATCH-SHAPE、`type_json` SCICOND、4 函数 STATEMENT、5 函数 GWPACK-OTHER（机器本体自举态）。
- **前缀前沿测试澄清**：`sico_compiler_lowers_parser_is_ident_byte_prefix_byte_exactly` 覆盖文件头至 `lex_words`（line 70）的连续前缀——仅前几个函数；`lex_words` 仍是连续前缀前沿，与本普查逐函数口径正交。

## 4. 验证

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler`：**58/58 绿**（含 9 个新冻结语料字节全等条目）。
- `git diff --check` 净；探针文件 `scratch_census_probe.rs` 按纪律删除（普查方法与前沿地图记录于本文 §1/§3）。

## 5. 下片

- mid-layer 突破片：`declared_return_kind`/`parameters_ir`/`binding_parameter_kind` 族（解锁 41 个 CALL-TARGET 消费者的传递闭包）——先探针定位其各自缺失被调，再选最省手术者。
- `has_return_at` 片：else 分支守卫 + 嵌套 if（oracle 转储已就绪，块结构待实现）。
- R3 0/4；M22 整体 NO-GO 不变（S6/S7 未进入）。
