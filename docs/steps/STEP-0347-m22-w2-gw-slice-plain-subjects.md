# STEP-0347 — M22 W2 S3/S4: gw slice 主题 plain 第三参 + concat utf8_decode 实参，json_string_token 推进两门

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

三处 gw 机器加宽，全部以合成形状字节差分冻结（本片无 census 函数翻绿——
json_string_token 被更深的 after 区控制流挡住，见前沿）：

1. **`gw_slice_subject_packed` 第三参三分发**：arg3 此前只认
   `sub(bytes.length(...))`（上游已分流）与**两参用户函数调用**（其余一律
   CALL-TARGET@9828）。现按形态分流：单 token（param/cell/字面量走
   `fixed_literal_operand_json`，cell 发 read_local）、`X.literal(N)` 跨度
   （复用 `fixed_literal_const_instruction` 发 const_u64，anchor 对 =
   (gss_a2, 表达式字节起点)，line_end 边界 = slice 闭括号）、用户调用（原路径，
   由 `gss_arg3_call`/`gss_arg3_lit` 双 flag 保护）。
2. **`gw_text_concat_packed` 实参文法新增 `sico.bytes.utf8_decode`**：镜像
   trim 分支，复用 `gw_intrinsic_call_packed`（param/cell 实参，count/value/
   kind/json pack 布局）。
3. **flag 类型教训复刻**：`gss_arg3_call` 首版用 U64 声明 + I64.equal 比较，
   触发 E2001 expected I64 found U64（0344 会话 H3 同型错误）；分支内重名
   `gss_a2_off` 触发 unsupported cell redeclared（函数级 cell 表），重命名
   `gss_a2l_off` 解决。

**验证形状（3 个合成 probe 全部字节相等）**：G2 slice+cell 第三参
（json_string_token 循环内形状）、G3 slice+`U64.literal(2)` 第三参（const 跨度）、
G1 concat+utf8_decode 实参。**census fixpoint 无免费翻转**（47 函数验证前缀，
1 轮收敛；json_string_token 未翻则 lex_words/gw_decimal_value 链不解锁）。

## 2. 前沿更新

- `json_string_token`：CALL-TARGET →（slice 主题已通）→ CALL-ARGUMENT →
  （concat decode 已通）→ **STATEMENT@10760**：`gw_match_open_pack` 拒绝
  region=="after" —— while 循环**之后**尾段的 match/if（if 处理器同样拒
  after@11300）。这是 gw 机器 CFG 装配的独立片（after/join 区开新菱形），
  下一片解。**剩余 68 不变**（本片是使能片，无翻绿）。
- 合成形状证实：straight 机器 loop-free 单函数的 return 位一般调用
  （I_encode/I_bytes_length/I_bytes_at/I_text_concat 全 EXPRESSION）与
  match-slice+return（I_bytes_slice CALL-TARGET@3785 list_get 形状校验门）
  是**另一族缺口**（sef/scc 机器），与 gw 无关。
- M22 整体 NO-GO 不变（S5/S6/S7 未动）。

## 3. 验证与预算

- `--test selfhost_compiler`：**59/59 绿**（新增 STEP-0347 测试：G2/G3/G1
  三合成形状，next_index 单被调前缀）。
- `selfhost_local_bounds` 绿：`gw_slice_subject_packed` 新增约 11 cell
  （双 flag + arg3 形态绑定），仍低于最忙 `general_while_function_ir`
  248/256，地板 8 不变。
- 完整 runner 工作区套件全绿（35 个测试目标 ok，0 失败）；`cargo fmt --check` 绿；
  `validate-module-boundaries.ps1` OK；`git diff --check` 绿。
- 探针方法：intrinsic 逐一最小 probe（I_* 系列）+ awk 行号标记
  （CALL-TARGET→9828/3785 双门分解）；`selfhost_probe_tmp.rs` 按纪律删除。
