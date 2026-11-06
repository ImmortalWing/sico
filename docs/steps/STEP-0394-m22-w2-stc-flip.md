# STEP-0394 — M22: straight_text_concat_packed 翻绿 + wcr 截体法证伪记录，census 101→102/129、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，CALL-SHAPE 簇续片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**straight_text_concat_packed 两处整形翻绿**：
- `set stc_scan = next_index(matching_close(...), ...)` 嵌套用户调用
  set 拆 let+set（aic 模板同款）。
- `concat(.., to_text(gw_decimal_value(..)))` 实参内嵌用户调用提升
  `let stc_t_dv = gw_decimal_value(..)`。
- 门连锁：CALL-SHAPE → STATEMENT →（census 翻绿，其自身及下游 shape
  全通）。

**wcr RETURN-COUNT 截体法证伪（负结果登记）**：前向截体（从函数头
逐段保留 + `return ""` 收尾）全 K 值恒报 STATEMENT/EXPRESSION——
截体改变函数**路由**（空体走另一机器先到 STATEMENT），无法隔离深段
门槛；该方法对路由敏感的机器函数失效，wcr  hunt 暂停，0393 末段的
oracle IR 对照法下次启用。

## 2. 前沿更新

- census **101/129 → 102/129、0 DIFF**；CALL-SHAPE 7→4；级联：
  GWPACK-OTHER 1→2、NON-DOMAIN 1→2；RETURN-COUNT 19 不变（机器族
  本体，wcr 个案待 oracle 对照法）；无回归。
- 下一刀主线：CALL-SHAPE 残余 4 户 + wcr oracle 对照 + GWPACK-OTHER 2 户。

## 3. 事故与登记

- 截体探针顺序错误一巡（wcr 放末位致 walk 先撞 others 的 STATEMENT）
  ——所有组合探针必须保持自然 file order。

## 4. 验证与预算

- census 终值：flipped=102 diffs=0 total=129；HIST RETURN-COUNT 19 /
  CALL-SHAPE 4 / GWPACK-OTHER 2 / NON-DOMAIN 2；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_straight_text_concat_packed_closure_byte_exactly`
  （27 函数闭包字节全等；清单 extract 生成）。
- `--test selfhost_compiler`：**78/78 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （78/78 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
