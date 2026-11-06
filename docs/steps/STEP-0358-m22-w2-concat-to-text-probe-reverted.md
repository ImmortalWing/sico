# STEP-0358 — M22: concat 实参 `u64.to_text` 族探针（负面结果，全量回退，发现登记）

> - 状态：**reverted（未落地）**——隔离形状字节全等但冻结语料出现 3 例 DIFF，
>   按 0-DIFF 纪律整片回退；census 恢复 68/126、0 DIFF、histogram 与 STEP-0357
>   终值逐格一致（回退净验证）
> - 里程碑：M22 compiler self-host（S4 lowering 片，负结果探针片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 探针内容与结果

**目标门**：`gw_deferred_tail_json` 的 CALL-ARGUMENT 门 = concat 实参族缺
`sico.u64.to_text`（gw_text_concat_packed 实参派发只收 concat/join/trim/
utf8_decode/用户调用/参数/cell/字面量）；其体内还有
`to_text(u64_list_at(...))` 嵌套调用形状。

**两片实现与隔离验证**：
- (a) `gw_text_concat_packed` 新增 `s64.u64.to_text` 实参分支（镜像
  utf8_decode 委托 `gw_intrinsic_call_packed`）——微复现 c1
  `set acc = concat(acc, to_text(n))`（cell 实参）**byte-exact**；
- (b) `gw_intrinsic_call_packed` 实参放宽（单 token 检查容许可声明函数
  原子）+ 嵌套调用实参分支（while_call_rhs_packed + call + intrinsic
  三件套发射）——微复现 c2f
  `concat(acc, to_text(u64_list_at(...)))` **byte-exact**；
- gw_deferred_tail_json 两片齐时从拒绝变为 exit 0（20,654B），但 oracle
  对比在 result 60 区域出现发散（guest 发射字面 `?`〔word_at 越界回退〕
  与 `concat arguments [60,0]`，值编号错位）——**深上下文（match 细胞 +
  嵌套 while + if 块）下嵌套调用实参的解析发散**，隔离形状不可复现。

**整片回退**：census 实测 flipped=72 diffs=3（冻结语料 3 例字节不等 +
  CALL-ARGUMENT 36→8 大盘异动）→ 按"不发不可验证或语义相异代码"纪律
  回退全部本片修改；复测 census = 68/126、0 DIFF、histogram
  CALL-ARGUMENT 36 / CALL-SHAPE 7 / EXPRESSION 14 / STATEMENT 1 与
  STEP-0357 终值逐格一致；构建绿、无残留（gtc_ut/gic_call/GICD 等标识
  零残留）、git diff --check 净。

## 2. 登记发现（下片输入）

- **门地图更新**：concat 实参 `u64.to_text` 与嵌套调用实参的隔离形状
  字节全等可解（本片已证），但**深上下文发散根因未定位**——疑点在
  嵌套调用实参于 match 细胞存在时的 cell/值编号路径（guest 把某实参
  解析为 value 0/参数 0 且 pieces 出现越界 `?`）；下片须先以 gdt 截体
  二分定位发散语句，再谈落地。
- **方法教训**：扩族改动必须立即跑 census（本片隔离 byte-exact 后
  未即跑，多走一轮）；探针脚本改动后必须校验输出文件字节数（STEP-0357
  教训重申）。

## 3. 验证

- 回退后：component 构建绿；census 68/126、0 DIFF、四格 histogram 与
  0357 终值一致；git diff --check 净；无调试残留。
- 本片未增减固件（无落地变更）。
