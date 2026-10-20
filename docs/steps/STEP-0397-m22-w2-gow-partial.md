# STEP-0397 — M22: gw_ok_wrap_rhs_packed 部分推进（两处门形清除，CS 残余未定位），census 104/129、0 DIFF 维持

> - 状态：implementation complete locally（门形推进有实证、户未翻绿，如实登记 partial；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，门形推进片）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容（两处门形清除，户未翻绿）

- gow 两处 5 重 `next_index` 嵌套链摊平（直行 let 深嵌 user 调用不建模
  族，gls 模板）。
- `gow_enc2` 站点（命名非 `__enc_` 未被 0396 折叠捕获）手工折入
  match 主语形。
- 排查记录：`text.length(cell)` let 形态微探针 IDENTICAL（排除 line 10
  嫌疑）；`gw_decimal_value(word_at(parts, U64.literal(N)))` 与 cam
  绿体同款（排除）；gow = 单巨大 if（3–293 行深嵌），**平衡切口二分
  受结构伪影干扰**（截断缺尾 end if 时错误类别即变），残余 CS 未定位，
  留待下片用 callee-guard 角度（gow 本体内 15 参
  `gw_intrinsic_call_packed` 等调用的实参形守卫）重审。

## 2. 前沿更新

- census **104/129、0 DIFF 维持**；CALL-SHAPE 残余 1 户（gow）；无回归。
- 下一刀主线：gow CS 残余（callee 实参守卫角度）+ casm trap 专项 +
  RETURN-COUNT 19 户机器族本体 + GWPACK-OTHER 2 户。

## 3. 事故与登记

- 深嵌单 if 函数（gow 型）的截体二分不可靠：切口在闭合边界前时
  结构重解析改变路由；下片改用"callee 守卫返回码反查"（CS 发射点
  在 gw_rhs_packed/gw_intrinsic_call_packed 的守卫，逐 callee 核
  实参形）。

## 4. 验证与预算

- census 终值：flipped=104 diffs=0 total=129；HIST RETURN-COUNT 19 /
  CALL-SHAPE 1 / GWPACK-OTHER 2 / NON-DOMAIN 3；探针副本按纪律删除。
- 本片无新固件（户未翻绿）；既有 80 项固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （80/80 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 5. STEP-0398 追加数据（同日，源码净）

- GWPACK-OTHER 两户复核：control_if_function_ir（34 闭包）+
  straight_cell_function_ir（50 闭包）。
- scc 逐成员对拍：发射函数 = scc 本体（其余成员全 IDENTICAL；被调
  straight_text_concat_packed 在此组合中显自身 STATEMENT 门槛）。
- scc 本体扫描：全部 set/let RHS 已净（text.length(cell) 五处 =
  tlcell 实证形；其余为 pack 串/callee 返回）——残余 GWPACK-OTHER
  指向 scc while 体的**深嵌 if 结构门**（与 wcr 同族），非操作数形。
- 结论：GWPACK-OTHER 两户与 gow/RETURN-COUNT 19 户汇聚为同一前沿类
  ——**机器走查函数的深嵌结构容量**（多重嵌套 if/while + 多返回的
  组合）。下一片主线建议：停止逐户清操作数形，转攻 general_while/
  control_if 机器的深嵌容量（或按 0343 纪律抽辅助函数拆深嵌体）。

## 6. STEP-0399 深度阈值标定（同日，源码净 = 0397 终态）——顶嵌 if 深度 2 = STATEMENT 实锤

探针矩阵（函数顶层嵌套 if + 臂 return）：
| 形状 | 结果 |
|---|---|
| 顶层嵌套 depth 1 + 臂 return | IDENTICAL |
| 顶层嵌套 depth 2 + 臂 return | **STATEMENT** |
| 顶层嵌套 depth 4/6/8/10 + set 尾 | STATEMENT（const-set 混入变量，仅作旁证） |
| **while 体内**嵌套 depth 2/3 + 臂 return | **exit 0（全绿）** |

**结论**：门 = **函数顶层（无 while 包围）的嵌套 if**（control_if 机器
容量 = 深度 1）；while 上下文内嵌套容量充足（general_while 族）。这
解释了 wcr/gow/scc 的形态：三者均有一个**函数级巨大 if**（wcr 的
`if tail_ok==1` 3-293 行 / gow 同型），其臂内再嵌 if/while 即超限。

** remediation 方向（下片）**：函数级"主工作块在 else 臂"的巨大 if
反转为守卫形（`if cond: return ERR end if` + 主体反缩进顶层化）——
wcr 实操发现其两臂均为复合体（then 臂内还套 `if params==0: return
/ else:`），需块感知变换器（缩进栈解析 + 臂边界配对），手工正则
两次误切（小守卫误中/臂边界错配）已回退净态。
