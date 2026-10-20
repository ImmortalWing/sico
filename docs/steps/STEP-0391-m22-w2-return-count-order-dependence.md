# STEP-0391 — M22: RETURN-COUNT 顺序依赖复现与泄漏定位（诊断片 2，源码净）——跨函数状态泄漏实锤

> - 状态：diagnostic in progress（源码净，census 维持 101/129、0 DIFF）
> - 里程碑：M22 compiler self-host（S4 lowering 片，前端跨函数状态泄漏定位）
> - 证据级别：internal-fixture（位置矩阵 + 前缀/全序对照）

## 1. 位置矩阵（同 34 函数集合，仅变 while_call_rhs_packed 位次）

| 排列 | 结果 |
|---|---|
| file order（wcr 在 index 30/34） | RETURN-COUNT |
| wcr 提到末尾 | STATEMENT |
| wcr 提到最前 | RETURN-COUNT |
| wcr 提到中点 | RETURN-COUNT |
| 字母序（wcr 近末尾） | STATEMENT |

**结论**：wcr 非末位 ⇒ 其后继函数编译报 RETURN-COUNT；wcr 末位 ⇒ 只报
wcr 自身门槛（STATEMENT）。即 **wcr（general_while 机器处理的深结构函数）
编译后留下未复位状态，毒化下一个函数的"找 return"扫描**（straight 机器
族 #1 型 RETURN-COUNT = 扫不到函数尾 return）。

## 2. 泄漏面排查（scalar_ir 走查）

- scalar_ir 的 while 内每函数状态全为循环内 `let`（新鲜）；跨函数持久
  变量仅 `source_cursor`（设计如此，find_text_from 前向走查）。
- 机器上下文全参数化（gcell_names/gcur_term 等按调用新鲜传入）。
- 嫌疑转向：**#1 型 RETURN-COUNT 的扫描起点/终点**（body_end 计算、
  return 扫描游标）在某处读到陈旧值；或 general_while 机器经可变 cell
  列表参数（gcell_names 等 list.append 原地变异）把状态带回 scalar_ir
  的循环变量——list 参数是引用传递，机器内 append 可能污染调用方持有的
  列表（下一函数复用同名累积列表时读到上一函数的残尾）。

## 3. 下片计划（STEP-0392）

- 微复现：green 小函数 A（单 return 直函数）紧跟 general_while 深函数 B
  （最小 while+深嵌形状），验证"A 在 B 后 ⇒ A 报 RETURN-COUNT"最简对。
- 候选变异二分：B 依次摘除（a）list.append 累积；（b）match 深嵌；
  （c）双 while，定位泄漏载体。
- 修复方向：机器列表参数改为返回新列表（调用方重赋值）或驱动循环每次
  重建累积列表——按 0364 纪律先微复现再动刀。

## 4. 验证

- 本片源码零改动；census 维持 101/129、0 DIFF。
- 探针留存 target/tmp/cl_wcr_fileorder.sico（file-order 复现件）。

## 5. STEP-0392 追加数据（同日）

- **IF-TAIL 假设证伪**：19 户尾形扫描仅 wcr/gcc 为"尾 if 无 fall-through
  return"，其余 17 户均尾 return/end-match——非共享模板。
- **截体二分**：wcr 本体三个后缀切口（中点 / 3/4 / 距尾 3 句）全部把
  34-集从 RETURN-COUNT 变 STATEMENT——泄漏载体在 wcr 被切除的后缀段
  （含 flat23 收尾链 + 嵌套 return），但未能进一步收束到单一语句。
- **尾 return 实验**：给 wcr 补 fall-through `return "ERR:..."` 后 34-集
  仍 RETURN-COUNT——排除"缺尾 return"本身为因。
- **事故**：回退脚本误删 5695 行合法 error return——立即整文件从
  0390 备份（=0389 终态）恢复并 cmp 确认；纪律重申：**revert/删除一律
  整文件从显式备份恢复 + cmp 确认，不做行级手术**。
- 结论：顺序依赖属实但载体未定位；0393 换向——直接审查 general_while
  机器对"函数内 if 嵌套 while 且无 else"形状的终结子装配（gcur_term
  在 fall-through 路径的复位），用 oracle IR 对照 guest 对该形状的
  输出差异（若 guest 静默错发射即实锤机器缺陷类）。

## 6. STEP-0393 修正（同日）——泄漏假设撤销，首错语义 + wcr 本体门槛

- **首错语义实证**（oc1/oc2 对照：CALL-SHAPE 函数与 CALL-TARGET 函数
  两种排列各报各的错）——guest 模块走查**停在第一个失败函数**。
- 重读全部数据：file order 首败 = wcr（RETURN-COUNT）；alpha order
  首败 = gw_instr_count（'gw'<'wh' 排 wcr 前，STATEMENT 门槛）——
  wcr 从未被求值。**"跨函数状态泄漏"假设撤销**（0391 §1/§2 结论作废，
  保留作负结果）。
- **wcr 本体真门槛 = RETURN-COUNT**（其被调齐全时可复现）；0390 阶梯
  全部通过的形状之外，触发构造仍在 wcr 本体深段（0392 截体二分指向
  flat23 收尾链之前的深嵌段）。
- 下片：wcr 本体**前向截体**（从函数头逐段保留，而非从尾切除）+
  与 oracle IR 逐块对照，定位首个未建模 return 构造。
