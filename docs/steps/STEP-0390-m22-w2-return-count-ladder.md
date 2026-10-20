# STEP-0390 — M22: RETURN-COUNT 19 户门槛标定（诊断片，源码净）——单返回假设证伪，真触发器在机器体深度结构

> - 状态：diagnostic in progress（源码净，census 维持 101/129、0 DIFF）
> - 里程碑：M22 compiler self-host（S4 lowering 片，RETURN-COUNT 门槛标定）
> - 证据级别：internal-fixture（探针阶梯 + 闭包组合二分）

## 1. 标定结论（探针阶梯，全部 while/多返回上下文实测）

| 形状 | 结果 |
|---|---|
| if 臂 return + 尾 return（2 返回） | IDENTICAL |
| 6 重平铺守卫 return + 尾 return | exit 0 |
| while 体内 if return + 尾 return | IDENTICAL |
| 嵌套 if（3 层）内 return + while + 尾 return | exit 0 |
| while + match + error 臂 return + 尾 return | exit 0 |

**"scalar_ir 只收单返回输入"假设证伪**——guest 对常规多返回形状的支持
远超预期。RETURN-COUNT 真触发器在机器函数本体的**深度组合结构**：
while_call_rhs_packed 本体＝双重嵌套 while（46/49 行）+ 深度 ~5 的
match 臂内 return（160-222 行段）+ 全函数 ~20 返回的复合体。

## 2. 组合二分数据

- 全 34 函数并集（file order）→ RETURN-COUNT；**移除任一成员**均变为
  他种拒绝（STATEMENT/CALL-TARGET/EXPRESSION）——即 RETURN-COUNT 是
  走查**深端**才被触发的门槛，函数集序/成员数决定走查深度。
- 每成员单独闭包对拍：除 gw_instr_count / while_call_rhs_packed 本体
  报 STATEMENT（更浅门槛）外全 IDENTICAL—— again 指向深端组合。

## 3. 下片计划（STEP-0391+）

- while_call_rhs_packed 本体**截体二分**（保被调齐全，按语句块切口）
  定位首个触发 return 的嵌套深度/构造组合（候选：双重 while 内 match
  臂 return；match 嵌套 match；深度 ≥4 的臂 return）。
- 对照 general_while 机器 gcur_term 状态检查（13578 行）与
  scalar_ir return 计数（18827 行）两发射点，判定属哪道。
- 机器函数单返回重构（gic 模板）降级为备选：38 返回/函数 ×19 户成本
  过高，仅在机器侧确认不可行后重启评估。

## 4. 验证

- 本片源码零改动（parser.sico 与 0389 终态字节一致，cmp 实证）；
  census 维持 101/129、0 DIFF；git diff --check 净。
- 探针文件留存 target/tmp/wq_tworet / retinwhile2 / ret3flat /
  nesteddeep / whilematch / sixret / errguard / tlen* / listempty 系列。
