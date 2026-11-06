# STEP-0344 — M22 W2 S3/S4: 直行机器 concat RHS，`fn_entry` 链收口

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

新共享打包器 **`straight_text_concat_packed`**（~200 行，Sico 侧新函数头使分母 120→121）：
直行机器的 `sico.text.concat` RHS 支持，镜像 `gw_text_concat_packed` 的几何（开括号
+5、matching_close、双实参段扫描、后序发射）但**SSA 语义**——按 `mode` 参数适配两机器：

- 实参种类：字符串字面量（const_string）、Text param、局部别名（ssa 模式 = 值转发零指令；
  **cell 模式 = `read_local` 指令**——sef 是 oracle `lower_straight_line` 的镜像（纯 SSA、
  无 locals），scc 是 oracle 一般路径直行体的镜像（cell 物化 + read_local/write_local），
  这是 STEP-0343 撤回试验钉死的模型分界）、嵌套 `sico.text.concat`（递归）、
  `sico.u64.to_text(<u64 param|u64 局部|规范数字字面量>)`（u64 字面量按 oracle 转储
  发 `const_u64` 数字 data 形态，非 scalar_const 带引号形态）。
- 打包 5 行：`advance / value / kind / inner_instrs / final_instr`（inner 为空时 "-" 标记）。

接线四处：sef 的 let-RHS 与 **return 位**；scc 的 set/let-RHS（守卫式插入，不重排既有
缩进）与 **return 位**（此前 return 位只有 `X.literal` 分支）。类型拒绝保持有类型
（u64 进 concat 位 = CALL-TYPE；H3 探针形状最初误用 u64 即被正确拒绝）。

**翻绿（parser 40→43/121，canary +3，R3 0）**：
- **`fn_entry`**（8 个计数 let + 7 个嵌套 to_text 的 concat set + concat return，
  挂在 STEP-0343 累积链上字节全等）——函数入口摘要整链（counters → return_shapes →
  expression_shape → fn_entry）至此全部翻绿；
- `scalar_const_instruction_json`、`write_local_instruction_json`（指令 JSON 辅助器，
  **单独**字节全等）。

普查（38 函数前缀 4-pass）至不动点：pass1 上 3 翻（含 local_binding_index 复证），
pass2 零新翻；剩余 82。

## 2. 前沿更新

- 直行类剩余（EXPRESSION/CALL-ARGUMENT）：`find_text_from`、`word_span_offset`、
  `is_canonical_magnitude`、`parameter_name_start`、`argument_*`、`json_string_token`
  （while+match）、`has_return_at` 等仍有各自形状缺口（非 concat 类：I64/U64 比较 set、
  bytes.at、嵌套 match 等），逐片推进。
- 机器自宿主层（gw_* packers、straight_*、checked_*_ir）不动。
- R3 0/4；M22 整体 NO-GO 不变。

## 3. 验证与预算

- `--test selfhost_compiler`：**58/58 绿**（新增 fn_entry 累积链条目 + 两辅助器单独条目）。
- 完整 runner 工作区套件全绿（35 个测试目标 ok）；`cargo fmt` 绿；local bounds 守卫绿
  （`straight_text_concat_packed` 为独立函数，`general_while_function_ir` 保持 248，
  硬顶 256 不变，地板 8 不动）。
- `selfhost_probe_tmp.rs` 按纪律删除。
