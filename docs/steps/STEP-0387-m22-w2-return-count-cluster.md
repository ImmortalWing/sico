# STEP-0387 — M22: RETURN-COUNT 簇清零——gw_intrinsic_call_packed 单返回重构，census 97→98/128、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，R3 单返回纪律片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**门族定位**：RETURN-COUNT 2 户（gw_intrinsic_call_packed / gw_ok_wrap_rhs_packed）
经逐成员闭包对拍定位根因 = **gw_intrinsic_call_packed 自身函数体**的
if/else 双臂 return 形状（param 臂 `return __flat14` / else 臂
`return __flat13`）——R3 族"guest 机器不建模双臂 return join"。
该户是 25 函数闭包的公共被调，毒化 gw_ok_wrap 等全部含它的闭包
（0362 教训的又一实例：机器函数自身体形必须首先自降低）。

**修改**（R3 单返回纪律重构，语义等价）：
- 函数顶 `let gic_out = ""`；两个成功臂改为 `set gic_out = ...`；
  函数尾单一 `return gic_out`。错误臂的 typed return 保留
  （`else: return CALL-ARGUMENT` 属已建模守卫形状）。
- gw_ok_wrap_rhs_packed 本体顺手三处提升（bytes.length(encode(word_at))
  拆 let+set、嵌套 concat 实参提升、intrinsic_instruction_json 的
  next_index 实参提升）——门推进至更深 CALL-SHAPE（该函数 ~280 行
  机器体，剩余形状下片续）。

## 2. 前沿更新

- census **97/128 → 98/128、0 DIFF**；**RETURN-COUNT 2→0 清零**（族灭）；
  gw_intrinsic_call_packed 翻绿；CALL-SHAPE 27→28（gow 级联入族）。
- 下一刀主线：CALL-SHAPE 28 户（gw_ok_wrap 续攻 / checked_arithmetic_* /
  user_call_expression_json / while_call_rhs_packed）+
  control_if_function_ir（GWPACK-OTHER 独苗）。

## 3. 事故与登记

- 固件测试块被 rustfmt 重排后按文本替换失败且残留双链语法错——
  纪律：测试块改动后必须 cargo fmt 并单跑验证；整块重写优于
  模式补丁。

## 4. 验证与预算

- census 终值：flipped=98 diffs=0 total=128；HIST CALL-SHAPE 28 /
  GWPACK-OTHER 1 / NON-DOMAIN 1；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_gw_intrinsic_call_packed_closure_byte_exactly`
  （13 函数闭包字节全等；闭包清单以 extract_closure.py 输出为准）。
- `--test selfhost_compiler`：**75/75 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （75/75 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
