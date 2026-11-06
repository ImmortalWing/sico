# STEP-0343 — M22 W2 S3/S4: 双栈物理截断，`expression_shape` 链三函数翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

general 机器的两处栈簿记修复——此前**弹出只缩计数、不动物理列表**，兄弟结构的重压栈
读到陈旧槽位：

1. **while 帧栈（`gwh_frames`）**：`end while` 只 `gwh_len = gwfr_top`；顺序第二个顶层
   `while` 压栈 append 到物理尾部，top 索引却指向陈旧的第一帧 → 第二个 while 头块的
   branch 终结器登记了陈旧数据（gwt_entries 出现两条同 header/cond 的条目），终结器
   装配循环在无主块上拒 `STATEMENT`。**嵌套 while 不受影响**（压栈在物理尾部之上，
   索引一致）；只有"弹出后再压"的兄弟 while 踩坑。修复：弹出时以 `gwf_keep` 累积器
   + `gdefer_scan` 复用游标把 `gwh_frames` 物理重建为前 `gwfr_top` 项。
2. **match 栈（`gmt_stack`）**：`end match` 的 pop 读 `stack[depth-1]` 且只减 depth。
   同层先出现过一对 2 深嵌套（m2{m3}）后，兄弟对（m4{m5}）的 m4 收口恢复到陈旧
   pack（物理尾部以下），外层 match 的 entry 从此错位 → 无主块 STATEMENT。修复：
   pop 改读**最后物理槽位**并物理截断（与 while 帧栈同一重建惯用法）；depth 递减
   解耦为独立 match（`gmt_depth_next`）。

**A/B 证据**（最小形状矩阵，同一探针）：HEAD（bd19063）上 E1（两个顺序顶层 while）
与 M5（同层两对 2 深嵌套 match）均拒 STATEMENT；修复后字节全等。既有绿形状
（break/深 else-if/循环头 if-else/R3 双臂 return/count_params/find_body_end）全部保持。

**翻绿（parser 37→40/120，canary +3，R3 0）**：`expression_shape_depth`（两个顺序
顶层 while + 递归自调用作 concat 实参）、`expression_shape`、`return_shapes`
（累积组合，line_end + is_string_token/is_digit_token/matching_close 七函数前缀字节全等）。

## 2. 调查副产品（未落地，登记方向裁决）

为解锁 `fn_entry`（被调已全绿仍拒）试验过**直行机器 EXPRESSION 拒绝 → gw 回落**
（镜像 control_if 的 SKIP 接线）：路由生效，但字节永不可能全等——gw 的 let/set
处理器**无条件发 `write_local`**（~10806 行），而 Rust oracle 对直行函数走
`lower_straight_line` 全程 SSA、无 cell 无 locals（lower.rs ~1000 注释即此规则）。
方向按 no-unverified-refusal-drift 纪律**整体撤回**。正确方向：直行机器自身的
表达式加宽——最小缺口 F2（let-RHS 嵌套 `text.concat`）与 F1（concat 实参位
`u64.to_text`），覆盖 `fn_entry`/`find_text_from`/`word_span_offset`/
`is_canonical_magnitude`/`parameter_name_start`/`argument_*` 等约 15 个
EXPRESSION/CALL-ARGUMENT 前沿函数（下片目标）。

普查（37 函数前缀 4-pass）至不动点：`local_binding_index` 复证，其余零新翻——
组合不动点未移动。

## 3. 验证与预算

- `--test selfhost_compiler`：**58/58 绿**（新增累积链冻结语料条目 ×3）。
- 完整 runner 工作区套件两轮全绿（35 个测试目标 ok）。
- **本地预算**：`general_while_function_ir` 245→248 locals（`gwf_keep`、
  `gmt_depth_next`、一个 match 主题临时槽）；`selfhost_local_bounds` 头寸地板
  11→8 并登记：gw 再加宽须按 STEP-0249 模式抽辅助函数，不再增长主 walk。
- `cargo fmt` 绿；语料测试的 `parser_function_text` 提取器改为 `"function {name}("`
  锚点（`expression_shape` 前缀命中 `expression_shape_depth` 会导致 DuplicateId
  ——复用本片探针教训）；`selfhost_probe_tmp.rs` 按纪律删除。
