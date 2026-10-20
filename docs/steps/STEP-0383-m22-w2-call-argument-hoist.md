# STEP-0383 — M22: CALL-ARGUMENT 27 户攻坚——嵌套 intrinsic 实参门形微复现 + 源码变换链（flatten/hoist/reuse），census 82→88、0 DIFF、CALL-ARGUMENT 27→0 清零

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，guest 前端源码整形片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**门形微复现**（探针矩阵，旧 guest 与新 guest 双向对照）：

- 门族 1 = **user/builtin 外呼 + 实参内嵌任何调用** → `E-SH-IR-CALL-ARGUMENT`
  （`is_word(a, sico.text.concat(t,"x"))`、`is_ident_byte(sico.bytes.at(...))` 均拒；
  而 `is_word(word_at(...), word_at(...))` user-in-user 收——实参打包协议只拒 intrinsic）。
- 门族 2 = **intrinsic 外呼 + intrinsic 嵌实参** → 按操作数分化：
  `sico.text.concat(a, concat(b,c))` 拒（EXPRESSION），
  `sico.bytes.equal(encode(w), encode(t))` **收**（is_word 本体一直绿的根因）。
  结论：提升（hoist）必须**外科式**——只提升门族 1 站点，全量提升会把
  guest 已支持的 equal-inline 形状破坏成不支持的 cell 实参形状。

**源码变换链**（selfhost/parser.sico，工具脚本在 target/tmp/，gitignored）：

1. `flatten.py`：纯 concat 树（≥3 叶）重结合为 init+顺序 append
   （string concat 可结合；156 语句；**累加器读后写守卫**：`set V` 时 V 不得
   出现在叶 1.. 之外——第 2 版修复了 while_bytes_at_rhs_packed 的
   `concat(sbi_cursor_json, concat(",", sbi_json))` 破坏）。
2. `hoist.py`：门族 1 站点提升——intrinsic 调用处于 user/builtin 外呼实参区
   时提升为前置 `let`（family-1 限定；while 条件站点 5 处人工前置
   loop-invariant length，逐个核验循环体不变异被迭代表）。
3. `reuse.py`：类型分组 + 支配路径前缀匹配的 let→set 复用
   （降 `MAX_LOCALS_PER_FUNCTION=256` 压力；**同语句多代读防碰撞**——
   use 行剥 let-LHS 后按 alias 解析，只有不产生同 canonical 双读才复用；
   修掉了 is_word `equal(__h1,__h1)` 这类重命名碰撞）。

**回归回退**：全量变换首跑 30 户绿→异色门回退（revert_fns.py 按函数从
0383 前备份嫁接），保留净增益。回退原则：只回退"基线绿被变换打绿"的函数，
不动"基线 ERR 被变换推进"的函数。

## 2. 前沿更新

- census **82/127 → 88/127、0 DIFF**；**CALL-ARGUMENT 27→0 清零**（本片目标）；
  EXPRESSION 保持 0；CALL-SHAPE 16→4；GWPACK-OTHER 1→35（大片级联推进至
  下一真实门，含 general_while/control_while/scalar_ir 本体）。
- 修复户示例：record_named_type/record_type_json（CALL-ARGUMENT→绿）、
  summarize/straight_environment_operand_value/direct_bytes_equal_encode_ir/
  fixed_guard_condition_json（→绿）。
- 下一刀主线：GWPACK-OTHER 35 户（gate 族细分待微复现）+ CALL-SHAPE 4 户。

## 3. 事故记录（本会话血泪增量）

- **reuse 重命名碰撞**：同一 use 行读两代同型 var 时 `\b__h2\b→__h1` 把
  两代揉成同名 → guest 前端语义腐化（全部输入 FUNCTION-COUNT/RECORD-END）。
  修复=alias 感知的单代读检查（§1.3）。
- **flatten 读后写**：`set V = concat(A, concat(",", V))` 型被重排成
  先覆盖 V 再读 → lex_words 闭包 guest 发射重复指令（`result:25` 双发 + `,,`）。
  二分定位（FLAT_SKIP 按函数跳过）至 while_bytes_at_rhs_packed；修复=累加器
  出现守卫。教训：变换必须做闭包级 guest-vs-oracle 字节对拍（cmp_probe.sh）。
- **Windows python 不识别 MSYS /tmp**：脚本一律用绝对路径或 cygpath。
- hoist-only  guests 经对照证明语义等价（IR 同构），全部语义破坏均出自
  reuse/flatten 两整形器，已修。

## 4. 验证与预算

- 微复现探针：g1/g3/b1/b2/p1/p2/p3/pa/pb 矩阵（门形判定依据，全部保留在
  target/tmp/ 探针文件）。
- census 终值（3 轮迭代后）：flipped=88 diffs=0 total=127；
  HIST GWPACK-OTHER 35 / CALL-SHAPE 4；探针副本按纪律删除
  （runner tests/ 下 scratch 副本残留会卡 0245 fmt，已清理并复跑）。
- 闭包级对拍工具 cmp_probe.sh（guest 输出 vs oracle 转储字节比较，
  `printf '%s'` 剥尾换行——sed 剥法会假 DIFF）。
- 固件同步（selfhost_compiler.rs 同片变更）：
  `sico_guest_arena_accommodates_full_parser_selfhost_input` 燃料
  20B→40B（注释登记 STEP-0383 原因），前沿断言
  CALL-ARGUMENT→GWPACK-OTHER；40B 燃料实测全源降低 4.7s typed 拒绝
  （非 trap）。
- 本片全量电池重跑全绿：fmt、`validate-step-0262`（selfhost_compiler
  71/71 + selfhost_parser 2/2 + selfhost_local_bounds 1/1，含固件改写）、
  `validate-step-0245`（runner fmt/clippy/test）、`report-m22-canary`
  （FORMATTER-CLOSED 30/30、full_source_exit 0）、
  `validate-step-0124 -SelfTest`、`git diff --check`。
- R3 连续停滞数保持 0（parser 覆盖 82→88 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
