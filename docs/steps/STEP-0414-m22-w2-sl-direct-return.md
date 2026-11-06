# STEP-0414 — M22: 模板执行 #1——split_lines 直返重构（tail 折入 emit + 链反缩进顶层化），census 117/136、0 DIFF 维持

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌手术片 6）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**split_lines dispatch 臂直返化**（0413 模板②首执行）：
- `gw_rhs_split_lines_emit` 折入 tail 逻辑（空 pieces → "-"，与 main 尾
  装配语义对齐）→ 返回即 multi_pack 终值；
- dispatch 臂改 `if split_lines: return emit(ctx) end if`；
- **else-drop + 链反缩进**：split_lines 的 else（encode/length/fallback
  链）依 orientation(b) 顶层化（链自身内部嵌套留下片消解）。

## 2. 前沿更新

- census **117/136、0 DIFF 维持**；无回归；HIST GWPACK-OTHER 13 /
  CALL-SHAPE 2 / NON-DOMAIN 4。
- 下一片：encode/length 臂同法（各成 helper 含 tail）+ fallback
  （gwsdf 级联逐级 helper）。

## 3. 事故与登记

- else-drop 首跑留闭合 `end if` 致 MismatchedClose——orientation(b)
  必须同步丢弃容器闭合行（closer 与 else 同命运）。

## 4. 验证与预算

- census 终值：flipped=117 diffs=0 total=136；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 5. STEP-0415 事故与发现（同日，源码回退 = 0414 验证态）

- **0411 at_dispatch 拼接的潜伏腐蚀实锤**：then 臂缩进混合（at_arg/at_param
  缩进 4 + at_dispatch consume 缩进 2）+ dangling else——宿主宽松解析
  （E8010）且**语义碰巧正确**（bytes.at 与其余路径互斥，consume 恰为
  then 臂内容、链为 else 内容；split_lines 经互斥性在 else 内检查等价）；
  guest 因其形状拒（STATEMENT）。清理 = 纯形状手术，非正确性修复。
- **清理尝试两轮失败**（else 定位误配缩进 0 变体 / 行删除错位）——
  按纪律整文件回退 0414 验证态（cmp + sanity probe 双确认）；教训 =
  嵌套腐蚀区的清理必须整块重写（ then 臂全段提取 + 干净再发射），
  禁行级删除；下次动刀前先把目标区域的完整缩进图打印存档。
