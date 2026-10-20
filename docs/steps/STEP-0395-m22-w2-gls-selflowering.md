# STEP-0395 — M22: gw_list_sort_rhs_packed 自降低翻绿——直行上下文 bytes.length 操作数形三分化，census 102→103/129、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，直行机器操作数形分片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**门形微复现（截体扫描逐语句定位 K=36）**：gw_list_sort_rhs_packed
（0384 新打包器）本体在直行上下文拒 CALL-SHAPE——根因 =
**直行机器对 `sico.bytes.length` 操作数形三分化**：
- `let l = bytes.length(cell)`（提升后 cell 实参）→ EXPRESSION；
- `bytes.length(encode(word_at(..)))` 嵌套三层（user 调用作 encode 实参）
  → CALL-SHAPE；
- **`match checked_add(.., bytes.length(encode(word_at(..))))` 主语形**
  → 唯一已建模形态（word_span_offset 同款，全库通行）。

**修改**：gls 两处 token 尾计算改为主语内联形（sweep2 提升形态回退）。
顺带把 gls 的 5 重 next_index 嵌套链摊平（直行 let 深嵌 user 调用同样
不被建模，截体 K=2-6 的 EXPRESSION 段锁定）。

**回归排查（负结果更正）**：e2 微探针在 0382 旧 guest 同样拒
EXPRESSION——非本片/0388 引入的回归，是直行机器从来只建模主语形
（0388 sweep2 提升的 71 处无一在直行上下文的临界路径上，census 全程
无回归与此一致）。

## 2. 前沿更新

- census **102/129 → 103/129、0 DIFF**；CALL-SHAPE 4→3；无回归。
- 下一刀主线：CALL-SHAPE 残余 3 户（checked_arithmetic_set_match_ir /
  gw_ok_wrap_rhs_packed / checked_arithmetic_match_ir，疑似同族
  bytes.length/嵌套操作数形）+ GWPACK-OTHER 2 户 + RETURN-COUNT 19 户
  （机器族本体）。

## 3. 事故与登记

- 截体扫描第一遍把"let 新名 + set"误判为 set-vs-let 门——实际是
  bytes.length 操作数形门；教训 = 变换后必须重跑截体确认门已移动
  而非仅改名。
- 直行上下文 token 尾计算**一律用 match 主语形**，禁止提升 cell
  （登记为编码纪律，sweep 类脚本须跳过或回退此类站点）。

## 4. 验证与预算

- census 终值：flipped=103 diffs=0 total=129；HIST RETURN-COUNT 19 /
  CALL-SHAPE 3 / GWPACK-OTHER 2 / NON-DOMAIN 2；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_gw_list_sort_rhs_packed_closure_byte_exactly`
  （12 函数闭包字节全等 + 0384 sort/join 测试原位保持）。
- `--test selfhost_compiler`：**79/79 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （79/79 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
