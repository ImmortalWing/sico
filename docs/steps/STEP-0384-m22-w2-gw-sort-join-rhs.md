# STEP-0384 — M22: GWPACK-OTHER 门族第一刀——gw RHS 机器学 `sico.list.sort`/`sico.text.join`，records_table_json 翻绿，census 88→89/128、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，gw 机器族 RHS 扩族片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**门形定位**（GWPACK-OTHER = gw set/let 处理器的 `gw_rhs_packed` 回 EXPRESSION
且绑定名非 "out"）：微复现三探针（while 上下文，命中 gw 机器路径）——
`set V = sico.list.sort(cell)` 拒、`set V = sico.text.join(cell, lit)` 拒、
`set V = next_index(next_index(j, n), n)`（嵌套用户调用实参）拒；
对照 `sico.list.append` set 收、`next_index(i, i)` 收。

**机器扩族**（parser.sico）：
- `gw_list_sort_rhs_packed` 新打包器（+~55 行）：单 cell 实参
  `sico.list.sort`——arity 守卫（多实参 CALL-SHAPE typed 拒绝）、
  read_local（range = cell token 跨度）+ intrinsic（range = 调用全跨度，
  首版漏取 close 括号偏移致 end 差 22 字节，oracle 对拍定位修复）、
  返回 count/value/kind/pieces 四段 pack 串接 gw set 处理器既有消费格式。
- `gw_rhs_packed` 加两行调度：`text.join` 委托既有
  `gw_text_list_binary_packed`（binary 形状与 append 同构，
  result_kind 改 string）；`list.sort` 委托新打包器。
- 打包器 ty 暂写死 list-string（cell_kinds 按索引读需 `list.get[Text]`，
  guest 尚不支持——list_get_match_ir 户仍在 flip 表；sort 唯一使用点
  records_table_json 的 cell 恰为 List[Text]，如实现录）。

**源码绕行**：records_table_json 的 `set j = next_index(next_index(j, n), n)`
拆为两条直 set（语义等价，省一次机器扩族）。

## 2. 前沿更新

- census **88/127 → 89/128、0 DIFF**（+1 = 新打包器入分母）；
  **records_table_json FLIP**（GWPACK-OTHER → 绿，12 函数闭包）；
  GWPACK-OTHER 35→34、CALL-SHAPE 4→5（一户级联推进）。
- 下一刀主线：GWPACK-OTHER 34 户逐族微复现（argument_* 族 / straight 族 /
  scalar_ir 本体）+ CALL-SHAPE 5 户。

## 3. 事故与登记

- 首版打包器 arity 未守卫：`sort(entries, entries)` 静默编译（实参多余
  被吞）——固件守卫探针当场抓获，补 arity 检查（CALL-SHAPE）。
- 直上下文探针（无 while）走直行机器、gw 上下文探针才走 gw 机器——
  微复现必须按真实故障上下文包裹（本片 wp_ 直探针全白跑，wq_ while
  探针才命中）。
- 固件：新测试 `sico_compiler_lowers_gw_list_sort_and_text_join_byte_exactly`
  （sort/join 单元形状字节全等 + arity 守卫 + 拒绝后恢复 +
  records_table_json 闭包字节全等）。

## 4. 验证与预算

- census 终值：flipped=89 diffs=0 total=128；HIST GWPACK-OTHER 34 /
  CALL-SHAPE 5；探针副本按纪律删除。
- `--test selfhost_compiler`：**72/72 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt、validate-step-0262（72/72 + parser 2/2 +
  bounds 1/1）、validate-step-0245（runner fmt/clippy/test）、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
