# STEP-0350 — M22 W2 S3/S4: 直行 concat 用户调用实参，3 个 JSON 构造器翻绿；登记两个既有 guest trap

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

`straight_text_concat_packed`（STEP-0344 packer）实参文法新增**单参用户调用**
分支（原兜底 CALL-ARGUMENT@5852）：`type_json(kind)` 形状——被调存在性检查
（`function_declaration_index` < len 且下一 token 为 "("）、返回类型须 "string"
（CALL-TYPE）、恰好一参（多参 CALL-ARITY typed refusal）、实参四形态 =
param（任意类型）/ 局部 cell 模式（read_local）/ 局部 ssa 模式（值转发）/
字符串字面量（scalar_const）；调用经 `call_instruction_json` 以被调 ordinal
发射，span end = 调用闭括号偏移+1。

**翻绿（parser 55→58/121，+3，R3 0）**：`read_local_instruction_json`、
`call_instruction_json`、`intrinsic_instruction_json`（均 type_json 单被调前缀，
字节全等 8119/9150/8730 字节）。`fixed_operation_operand_width` 系（需
word_span_offset 等未翻被调）与 summarize（gw 位多参调用 fn_entry）、
record_named_type（gw 位）留待后续片。

## 2. 既有 guest trap 登记（本片 bisect 副产品，HEAD 09faacc 上复现，非本片引入）

三个 trap 全部为 **typed-refusal 违规**（应拒绝却 wasm unreachable），最小复现：

1. **纯 return `sico.text.split_lines("x")`**：单函数 fixture 即 trap
   （`function q(src: Bytes) returns Text: return sico.text.split_lines("x")`）。
   路由 = nested_intrinsic_return_ir SKIP → general_fallback 跳过 → scalar_ir
   内联 return walker——分析显示 walker 应落 EXPRESSION typed refusal，实测
   trap 在 codegen 辅助函数（wasm 帧 34→207→208；207=scalar_ir，208+=codegen
   生成辅助）。
2. **sef 降低后接 scc 降低**：`let words = split_lines("") return words` 的
   let-only 函数（sef 路由）与后续 scc 函数（如 call_instruction_json）同
   fixture 即 trap；调序（scc 先于 sef）则 OK。同 scalar_ir 运行内跨函数状态
   污染嫌疑（wasm 全局/codegen CoW arena 方向）。
3. **缺被调用户调用 → trap 而非 CALL-TARGET**：`line_end` 单独 fixture（其被调
   is_word/word_at 缺席）trap。gw 用户调用发射路径某分支缺被调存在性检查。

影响：S6 bootstrap（全源差分）编译整份 parser.sico 时，lex_words（早退
split_lines）之后的函数降低会踩 trap 2/3，**S6 前必须修**。下一片（STEP-0351）
优先攻坚。census 暂停一轮（trap 使含 lex_words 的 52+ 组合不可用），0350 三翻
以 standalone 组合冻结。

**bisect 工具新教训**：EXPRESSION 拒绝点**不可后缀行号标记**——ladder 在
16862 用 `is_word(json, "ERR:E-SH-IR-EXPRESSION")` 精确匹配捕获 control_if 的
拒绝并降级 gw，标记破坏匹配后行为改变（假象：全部 fixture 死于 13765）。改用
范围标记法（跳过 control_if 函数体 13512..16917）。

## 3. 验证与预算

- `--test selfhost_compiler`：**62/62 绿**（新增 STEP-0350 测试：三构造器
  standalone + set 位 concat 用户调用最小卫兵）。
- `selfhost_local_bounds` 绿（packer 新增约 15 cell，仍非最忙函数，
  gw 保持 248/256）。
- 完整 runner 工作区套件全绿（34 个测试目标 ok，0 失败——探针文件按纪律删除
  后目标数-1）；`cargo fmt --check` 绿；`validate-module-boundaries.ps1` OK；
  `git diff --check` 绿。
