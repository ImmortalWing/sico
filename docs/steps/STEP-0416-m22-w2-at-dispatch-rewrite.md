# STEP-0416 — M22: at_dispatch 腐蚀区整块重写落地（0415 教训执行）——census 117/136、0 DIFF 维持

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌手术片 7）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**整块重写三步**（0415 纪律执行：缩进图先存档 → 整块提取 → 干净再发射）：
1. `gw_rhs_at_dispatch` 自给化：at_arg/at_param 计算入 helper、
   `gws_at_param` 形参摘除；
2. 腐蚀区（bytes.at if 至容器闭合 end if，缩进图 10121-10240 存档于
   STEP-0415/0416 记录）整块替换：`if bytes.at: return at_dispatch(ctx)
   end if` + 链内容整体反缩进顶层化（else 标记与容器闭合行随块丢弃）；
3. host check/build 双闸 + census 复拍确认无回归。

## 2. 前沿更新

- census **117/136、0 DIFF 维持**；无回归；形态干净化后深层门显形
  （encode/length 臂内 guard、gwsdf 级联、gwf 操作数对——下片模板
  逐个执行）。
- 下一刀主线：encode/length 臂 helper 化（模板②）→ fallback/gwsdf
  级联（模板④）→ gwf_l/gwf_r 操作数析取（模板③）。

## 3. 事故与登记

- 0415 行级删除的教训验证：整块重写一次成功（缩进图 + 程序化边界
  定位 + 丢弃集合显式枚举）——腐蚀区手术标准流程定型。

## 4. 验证与预算

- census 终值：flipped=117 diffs=0 total=136；HIST GWPACK-OTHER 13 /
  CALL-SHAPE 2 / NON-DOMAIN 4；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 5. STEP-0417 事故（同日，源码回退 = 0416 验证态）

encode/length 臂 helper 化三连败（else 边界匹配误配 → 缩进算术错
〔实测 i2 内容被 -4 剥到 i0〕→ min-indent 重排仍混入了非 fallback 的
i2 行〔else 定位根本错误〕）——每次 MismatchedClose 均由 **else 定位
错误**引发（匹配到非目标 else）。三败后按纪律整文件回退（cmp+sanity
双确认）。**教训**：嵌套链区的 else 定位必须**从容器头逐层配对**
（每遇到一个同缩进 if/else 计数 ±1 至零），禁止 next() 裸扫；该片
改用手工 Edit 逐行精修（读精确行 → 手写替换），不再用脚本批量。

## 6. STEP-0418 发现（同日，源码净 = 0416 终态）

**encode 臂 fall-through 语义实锤**（精确读行 10135-10172）：
encode 臂 consume 后**不 return**、直接跌出 if-else → 函数顶层的
`if is_word(word_at(words, gws_second), ".")`（i2，gws_second 在 enc
匹配时**未初始化**——宿主函数级 cell 默认初始化使其编译通过，实践中
enc 输入在后继检查快速失败）。含义：
1. 该臂改 return-helper = **行为变化**（边缘路径），非纯形状手术——
   helper 化必须连带重构 fall-through 后继链，工作面比估测大；
2. guest 在此 STATEMENT 可能不仅是深度门，而是**前端对此形状的语义
   缺口**（未初始化 cell 读 + 兄弟检查）——下片先做微复现区分二者
   （`if cond: consume-不return end if` + 后继兄弟检查形状探针），
   再定机器修复 vs 源语重构。

**0417 教训执行验证**：手工精确读行避免了第四败（脚本在 else 定位
错误下会继续静默破坏；手工在第一步就撞见语义陷阱）。

## 7. STEP-0419 微复现（同日，源码净 = 0416 终态）

0418 假设②（未初始化 cell 读 + 兄弟检查 = 前端语义缺口）**证伪**：
uninit2/3/4/5 探针矩阵全绿（含 uninit5 = y 仅在 else 臂声明、链后
兄弟 if 直读——与 gw_rhs gws_second 完全同构）。guest 对该形状支持
完好。

结论：gw_rhs 残余 STATEMENT 另有其因（候选：fallback gwsdf 5 深层级
联 / gwf 操作数块嵌套守卫——即 0399 深度门在本地的具体实例）。
下片：对 0416 净态 multi_pack 做截体二分（顶层语句粒度，闭包现取）。

## 8. STEP-0420 组合二分陷阱再证（同日，源码净 = 0416 终态）

对 gw_rhs 闭包（59+ 函数）的组合前缀二分：首 STATEMENT @K=59
（word_span_offset 位），但**任何单删不清零、整段 [15:30) 删除仅
以 CALL-TARGET 提前死替代 STATEMENT**——callee 完整性与首错语义
使组合二分不可靠（0391 教训在 helpers 增多的新闭包上再证）。可靠
路径仍 = 模板逐函数形状手术（不依赖组合上下文）。下片回到
fallback/gwf 的模板③④执行。
