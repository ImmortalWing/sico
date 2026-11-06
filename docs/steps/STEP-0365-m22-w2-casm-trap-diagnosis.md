# STEP-0365 — M22: `checked_arithmetic_set_match_ir` wasm trap 定位（缺陷类，专项待续）

> - 状态：**diagnostic in progress（未落地）**——trap 独立复现（exit 125，22 函数闭包探针留存 target/tmp/probe_casm.sico；平衡截体定位至语句 348–350 边界）；源码净状态不变（census 76/127、0 DIFF 与 STEP-0363/0364 终值一致）
> - 里程碑：M22 compiler self-host（S4 lowering 片，缺陷修复专项前哨）
> - 证据级别：internal-fixture / runtime-verified（真实 runner 探针）

## 1. 复现与定位尝试

**复现**：census 报 `checked_arithmetic_set_match_ir` NON-DOMAIN trap
（wasm unreachable，backtrace 函数 213/226/114/194/212/34）——非 typed
拒绝，属 0350 登记的"唯一真实 trap"缺陷类（S6 前必修清单第三项）。
22 函数闭包探针独立复现（exit 125）。

**定位尝试**：
- 截体二分被**不平衡切口混淆**（k=700 拒 / k=720+ trap，但 700–720 行
  为纯 concat 发射链——不可能为 trap 本体；截断产生的非法块结构本身
  驱动 gw 机器走未建模路径，二分信号不可信）；
- 静态扫描闭包 22 函数无"无 return 兜底"路径（oracle 不发 unreachable
  终结子的显然候选排除）；
- 推断：trap 在 **casm 机器自身执行**中（casm-the-source 经 scalar_ir
  路由到 checked_arithmetic_set_match_ir-the-machine，机器码内某
  oracle 发射的 unreachable 路径被实输入走到）——需语句级平衡截体或
  wasm 函数索引映射（name section / wasm-tools）定位。

**STEP-0366 平衡截体补充（2026-10-12）**：顶层语句粒度截体（481 句，pickle 留存 target/tmp/casm_stmts.pkl）定位——trap 引入边界在语句 348–350（left/right 操作数 off/end 计算与 match/单臂 if 区之后）：b342/346 过、b344/348 拒、b350 起 trap。信号解读：**casm 机器处理自身源到操作数发射区后走入 unreachable 路径**——trap 不在单一语句，是机器自身逻辑的深层状态（非截体混淆）。下片直读机器 848 行后半与其自身源的形状对照（own-source 审查法）。

**STEP-0367 oracle-IR 反查（2026-10-12）**：全 parser.sico oracle 降低（7.3MB dump）仅 8 个 unreachable 块，全部位于**双臂皆 return 的尾 match 之后的死 join**（fixed_operation_operand_atom_idx/straight_text_concat_packed/while_call_rhs_packed/gw_instr_count/gw_text_concat_packed/gw_intrinsic_call_packed/gw_slice_subject_packed/gw_call_chain_packed——0354 probe C 已实证此类 join 为 oracle 正常发射）。**关键推论**：死 join 无前驱，唯一可达途径 = match 判别式运行时非法（ok/error 不覆盖）——即 **Result 值被上游损坏**，trap 是下游症状而非这 8 处缺陷。**与 0364 登记的嵌套分支值编号损坏（result 跳 425/args[60,0]）同根因类**：gw 机器在大/深输入上的状态损坏。专项方向合并：解 gw 机器值/状态损坏根因（0364 gdt 截体 + 0366 casm 语句 348–350 两个现场互为对照）。

**STEP-0368/0369 更正（2026-10-12，STEP-0370 实锤）**：0368 的 n1/n2/n3 与 0369 的 n4/n5/n6/n10/n11 探针**全部漏写 `end function` 尾行**（mkprobe 实参体末尾遗漏，oracle 如实 E1001、guest 前端宽容放行）——**"三态谱系"与"入口区 match 门"均为工件误判，作废**。0364 gdt 值损坏与 0365-0367 casm trap 探针（显式带 end function）**不受影响，仍然成立**。教训登记（两次）：**探针文件必须用 oracle（sico check）验证有效性后再驱动结论**；guest 前端对缺 end function 的宽容 = guest 自身缺陷候选（另册）。

**STEP-0369 更正（2026-10-12）**：n 系探针工件误判（漏 end function），作废；有效对照（t3/t4 正确文件）见 STEP-0370——入口区 match 摘除显式门后即 byte-exact，无次道门。

**STEP-0371 有效阶梯专项（2026-10-12）**——0364 值损坏的**有效最小现场复现**（探针全程 sico check 前置 + mkprobe.sh 已加 end function 自动补尾/断言）：
- l2/l3（**双层 while** 内 concat 实参 `to_text(调用)`，callee 分别为 u64_list_at / next_index）：字节发散同 gdt 签名（`?`/args 含 0/result 跳号）——**损坏触发 = while 嵌套深度 2**（c2f 单层 byte-exact 对照成立），与 callee 无关；
- l4/l5（**直接 set RHS** `set y = to_text(调用(...))`，单层 while 即拒 CALL-ARGUMENT）——嵌套分支的 while_call_rhs_packed 接线在 set-RHS 路（gws_ut）的 line_end/bound 语义失配（疑：line_end 应传**行尾词索引**而非 gic_close，尾检查/参数校验依赖之）。
- **根因方向收窄**：嵌套分支本体的 while_call_rhs_packed 参数接线（两处现场互补：concat-arg 路深度 2 值损坏 / set-RHS 路深度 1 拒）——下片先修接线（line_end 传行尾、line_end_offset 传语句尾），再以 l4→l5→l2 阶梯复测。
分支已回退（census 76/127、0 DIFF、无残留）。

**STEP-0372 接线修复片（2026-10-12，未闭环全回退净）**：DBG 包裹实证——l5 的裸 CALL-ARGUMENT **不来自嵌套分支本体**（DBG-NEST/DBG-ELSE 均未现），而来自 **`gw_rhs_packed` 的 `u64.to_text` 分支自身的实参后验**（9495 行：param/cell 分支后硬落 `return CALL-ARGUMENT`，调用实参形态未委托 gic）。放宽该 fallback（pack kind 校验过 u64 即放行）后 l5 改拒 CALL-TYPE（我的 kind 校验误用 gic 返回的 intrinsic 结果 kind=string；正解 = 用 `declared_return_kind` 查被调返回 kind）；**l2/l3 concat-arg 深度 2 值损坏仍在**（gws_ut fallback 不涉 concat 路——gtc_ut 直接读 gic pack；其 `?`/args[12,0] 根因未触）。l6（纯用户调用 set-RHS 有效基线）byte-exact 确认基线无损。全回退净（76/127、0 DIFF、无残留）。

**STEP-0373 双门修复尝试（2026-10-12，未闭环全回退净）**：(a) l4/l5 的 set-RHS 拒门定位到 **gw_rhs_packed `u64.to_text` 分支自身实参后验**（9495 行硬落 CALL-ARGUMENT，不涉嵌套分支本体——DBG 包裹实证）；(b) gws_ut fallback 改 `declared_return_kind` 校验后 m5 可 exit 0 但**字节仍损坏**；(c) **关键回归实证：0358 byte-exact 的 c2f 在当前树 + 同分支下亦损坏**（`?`/args[8,0]）——0364 以来归因于分支本体的判断需修正：**某片 0361–0370 变更与嵌套分支存在相互作用**（0361 span 修复〔gcursor/gw_find_code_text〕嫌疑最大）；0372 的 line_end"修复"证伪（改 gic_call_after 更糟，gic_close 为正确接线）。两次探针自伤（m5 y 类型错〔E2001〕、 surgery 残留）均按 sico check 纪律捕获。全回退净（76/127、0 DIFF、无残留）。

**STEP-0374 分支运行实证 + 0361 相互作用嫌疑洗清（2026-10-12，全回退净）**：DBGP 回显实证嵌套分支**确实执行且其返回值流入损坏位**（DBGP 单行的 `?`/value=0 签名与正式版完全一致）——0364 以来"0361 span 修复相互作用"嫌疑**洗清**：同分支 + 模板 bigret（4 行完好）+ relax 下 c2f 仍坏，且损坏签名不变 → **腐败在分支返回的 pack 内容内部**（pieces/value 计算在 while_call_rhs_packed 返回值处理上，疑 gtc_value/base 传入值或 parts 解析在嵌套调用栈上的错位）。下片：回显 while_call_rhs_packed 的**原始返回内容**（非仅存在性）+ base/gtc_value 数值，定点 pack 内容腐败。全回退净（76/127、0 DIFF、无残留）。

**STEP-0375 回显未现 + 数据事故与恢复（2026-10-12）**：(a) GIC/PK 入口回显在 c2f/c1 输出中均未出现——**gic 对 concat 路 to_text 实参似乎未被调用**（freshness 双测证实构建无缓存，回显阴性成未解之谜；因随后的数据事故，本线实验记录保留为未决）；(b) **数据事故**：新鲜度测试中 `git checkout -- parser.sico` 把未提交工作树恢复到 HEAD（0351 时代），**0352–0370 全部落地修改一度丢失**；幸有 `/tmp/parser_backup.sico`（动手前备份）完整恢复——清除 4 处调试残留后 census 76/127、0 DIFF、histogram 与 0370 终值逐格一致、selfhost_compiler 70/70 绿，**工作树与 0370 净态字节等价**。(c) 新纪律：**任何"恢复/还原"操作只用动手前的显式备份副本，禁用 `git checkout`/`git restore` 触碰未提交工作树**；动手前备份列为强制步骤。

**STEP-0376 派发路径重审（2026-10-12，备份恢复净）**：gtc 实参循环 ATOM2 回显对 c1/c2f **均不触发**——gw_text_concat_packed 的实参循环对这些探针**根本未执行** → 0375"阴性"升级为结构性发现：**c1/c2f 的 concat 发射不经 gw 机器族**（疑 scalar_ir 形状路由把无 if+set 重的 while 函数分到 straight 机器族〔17400 一带 set_count>0 && if_count==0 分支〕，或 gw_rhs_packed 更早分支截获）。**0364–0375 整线（gic 嵌套分支/gws_ut/gtc_ut）疑似一直在错误的机器家族上施工**——0358 c2f"byte-exact"亦需在此框架下重审（其通过或源于 straight 路独立可解）。下片：①scalar_ir 路由判定回显（c2f 实进哪台机器）；②若 straight 家族 → 在 straight_text_concat_packed（5816 自有 to_text 分支）上重开嵌套实参专项。已按新纪律从 /tmp/parser_backup_0376 恢复（census 76/127、0 DIFF 逐格一致、diff-check 净）。

**STEP-0377 回显方法论缺陷实证（2026-10-12，备份恢复净）**：ROUTE-GW 回显因提前返回把整个函数 JSON 替换为标记——**下游全部回显（RHS/SETHIT）阴性均系上游回显毒化**，机器实际正常入 gw。由此回溯：**0376"结构性发现"（错误机器家族）与 0375 阴性同受此方法论缺陷污染，全部降级为不可靠**（0376 的 ATOM2 无上游回显，但其阴性仍可能来自第一个实参派发的合法提前返回）。**方法论登记：guest 侧早返回回显会毒化下游观测——追踪须用非中止标记**（如把标记写入 cell 随最终 JSON 发出，或构造性二分）。c2f 腐败专项的可靠资产仍限于：现象签名（`?`/args[8,0]/result 跳号）与触发上下文（while 体内 concat 实参嵌套调用）。全回退净（76/127、0 DIFF、无残留）。

**STEP-0378 决定性基线（2026-10-12，备份在手未改树）**：净树（0370 验证态）上 c2f = **exit 122 CALL-SHAPE（诚实 typed 拒绝）**——**全部 exit-0-损坏观测均发生在嵌套分支落地态**；腐败 100% 为**分支自身 bug**，机器行为正确（gic 单实参检查拒嵌套调用）。0364–0377  mystery 栈就此定性：分支 pack 构造缺陷（疑 `while_call_rhs_packed` 对 u64_list_at 三参〔param+cell+literal〕返回非预期 → 分支内 parts 解析出 `?` → 下游 `?`/value=0/跳号）。**下片定义清晰**：修分支 pack 构造（先非中止追 while_call_rhs_packed 对三参调用的返回内容——最终输出里已含的 `?` 即天然非中止标记），全绿后 l4→l5→c2f→l2 阶梯 + census 收口。0375–0377 受污染结论不再影响本路径。

**STEP-0379 相互作用单变量二分（2026-10-12，备份恢复净）**：分支 + 逐一回退落地变更测 c2f——**gcursor 排除**（回退 fn_start 仍坏）、**gfc/fn_end 27 处排除**（回退 naive 仍坏）；剩余嫌疑 = **0361 的 gtc_ut to_text 分支本身的路由语义**（line_end=gtc_seg_end 传入 gic，与 0358 无此分支时的实际路径不同——0358 c2f byte-exact 走的可能是另一条到 gic 的路〔其 line_end 语义未知〕）+ 0361 gws_ut + 0370 entry 门（c2f 无 match，理论无关）。**关键待查**：0358 时 to_text concat 实参**经哪个 gtc 分支到达 gic**（该分支未经 git 留档——全在工作树未提交态，无法 git diff 回溯）。下片：构造性重放（把 gtc_ut 的 line_end 传参改为行尾词索引对照实验）或直接审 gtc_ut 的 line_end 语义对 gic 尾检查的影响。净树恢复验证：census 76/127、0 DIFF、histogram 逐格一致。

**STEP-0380 相互作用变量全排除 + 专项停车登记（2026-10-12，备份恢复净）**：单变量二分完成——**gcursor / gfc-27 处 / entry 门全部排除**（分支+逐一回退测 c2f 均仍坏），而分支代码经模板法核对与 0358 byte-exact 版一致、0358 路径（gtc_ut 0358 已存在、语义相同）已澄清——**相互作用在现有变量空间中无解**，剩余可能 = 多变更联合作用或 0358 观测本身的未控差异。**专项停车判定**：嵌套调用实参分支**未落地、只影响未落地分支本身**（净树对嵌套调用诚实 CALL-SHAPE 拒绝，零用户影响）；八片（0364–0380）证据链完整留存（现象签名/触发上下文/排除矩阵/方法论缺陷实证），任何后续会话可按档重开。**主前沿回转**：EXPRESSION 27 户机器本体 + CALL-SHAPE 14 户（有具体翻绿抓手）。净树恢复验证：census 76/127、0 DIFF、histogram 逐格一致。

**STEP-0381 主前沿首刀：R3-if 门实证（2026-10-12，探针未动树）**：`operand_nonparam_kind`（EXPRESSION 户、helper 级）截体三变体定门——A（去尾调用）/B（臂 return 改常量）均仍拒、**C（去整个 if）翻绿** → 门 = **函数入口区 `if cond: return X end if` + 兜底 return 的臂内 return 收口**（R3 形状在入口/while-less 上下文；0341 登记 (a) 的正体）。这是 EXPRESSION 27 户中大片机器本体的公共门（机器体遍布守卫 if-return）。**下片 = R3 if 收口机器实现**（gfr 后支配 return 建模，0341 登记方向；属大手术，独占一片）。本刀仅探针，树净（census 76/127、0 DIFF 不变）。

## 2. 下片专项输入

- 探针留存：`target/tmp/probe_casm.sico`（22 函数闭包，exit 125 复现）；
- 方法：①用 guest 自己的块结构做**平衡截体**（以 `end if/match/while`
  为切点，保持 oracle 可降低）而非裸行切；②或 `wasm-tools objdump
  --name-section` 映射 backtrace 函数索引到 parser.sico 函数名；
- 修复方向：trap 路径补 typed 返回（机器守卫），或修机器逻辑使该状态
  不可达。

## 3. 验证

- 本片无源码变更；census 复测 76/127、0 DIFF、histogram 与 0363/0364
  终值逐格一致；git diff --check 净。
