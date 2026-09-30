# STEP-0318: M22 W2 set-arm checked_arithmetic match family

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（`parser.sico` 降低前沿；22-D 队列「entry-region statement match with `set` arms」）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`，repo-local msys2-binutils）

## Objective

`lex_words` 入场前沿的第一个形状族：`T.checked_add` 全标量 match 的 ok 臂为
`set <cell> = <bind>`（此前 `let+set+match` 组合在语句路由处整体
`ERR:E-SH-IR-STATEMENT` 拒绝）。

## Changes

- `selfhost/parser.sico` 新增 `checked_arithmetic_set_match_ir` 整函数族与
  `literal_value_index` 辅助。接受的有界形状：
  `let <cell> = <T>.literal(<k>)` 建格（cell 为首个 local）；
  `match <T>.checked_add(<左操作数>, <右操作数>)`，操作数各自为
  参数/同名 cell/非负 canonical literal；ok 臂 `set <cell> = <bind>`；
  error 臂 `return <参数 | T.literal(k')>`；`end match` 后单一
  `return <cell | 参数>` 收尾。IR 发射：locals
  `[cell, #match1, #match<bind>]`，block0（const/write/操作数/checked_add/
  write #match1 + match 终结）、ok 块（read/project/write bind/read bind/
  write cell + jump join）、error 块、join 块（range 复用 match 语句区间）。
- 分发路由插入在 `let+set+match` 的 STATEMENT 拒绝门之前：五计数门
  （match==let==set==1、if==while==0）加 ok 臂下一行首词为 `set` 的结构预检；
  族内全部守卫失败给出 typed 拒绝并逐级传播，游标按既有族惯例推进。
- 复用现有 `scalar_const_instruction_json` / `write_local_instruction_json` /
  `read_local_instruction_json` / `fixed_literal_argument` /
  `word_span_offset` / `parameters_ir` 等 helper；记录三处机器层事实供后续
  切片复用：selfhost 词流**不含独立 `=` token**（let/set 行索引布局据此）；
  match 临时/绑定 local 命名为 `#match<local序号>` 与 `#match<绑定名>`；
  `word_span_offset` 不能跨行（`<nl>` token 无对应源文本），跨行地标一律用
  `find_text_from` 链。

## Declared bounds（typed 拒绝边界）

- 仅 `checked_add`；sub/mul/div 仍 `ERR:E-SH-IR-CALL-TARGET`。
- 操作数、fallback、tail 限定为参数/同名 cell/非负 canonical literal；
  负字面量在跨类型时 `CALL-TYPE`、同类型时 `CALL-SHAPE`（声明边界）。
- fallback 不接受 cell（`ERR:E-SH-IR-EXPRESSION`）；多词操作数（嵌套调用）
  `EXPRESSION`；第三实参 `EXPRESSION`；溢出 `U64-RANGE`/`I64-RANGE`；
  非规范量级（前导零）`CALL-SHAPE`；cell 遮蔽参数 `CELL-SHADOW`；
  返回类型不匹配 `TYPE-MISMATCH`；set 其它 cell / 通配绑定 `MATCH-SHAPE`。

## Executed validation（本机真实 runner）

- 六变体对 Rust oracle 字节全等：param/cell/literal × 左右操作数组合、
  I64/U64、单/双参数、param/literal fallback、cell/param tail。
- 11 个 typed 拒绝逐一断言，拒绝后同 prepared 编译器恢复字节全等。
- `selfhost_compiler` 套件 53/53 绿（51 旧 + 2 新测试）。
- 完整 runner 套件串行（CI 形态）结果见 STATUS 当前步记录。
- `report-m22-canary.ps1`：coverage 100%（formatter canary 保持关闭）。

## Honest residuals

- `lex_words` 本体未降低：其 let-RHS 宿主调用（`bytes.length`/
  `text.split_lines`/`bytes.at`）、while、嵌套 if/else、带 append 调用的
  `bytes.slice` set 臂、`split_lines` error 返回均仍在
  EXPRESSION/STATEMENT 前沿之后；本族是其 ok-set-臂子形状的首块积木。
- parser.sico 自身函数无一因本族翻绿：S6 清单 parser 5/115、
  compiler_lexer 15/19、compiler_parser 0/32、parser_driver 0/2、
  checker 0/1 均不变；前沿仍为 `lex_words`/CALL-TARGET。
- 按计划 §3.2 纪律：本实现 STEP canary 零增长且拒绝前沿未动，
  **R3 连续停滞计数 +1（1/4）**。后续切片应优先移动前沿的形状
  （`lex_words` 首个拒绝前的 let-RHS 宿主调用族）。
- M22 维持 NO-GO；独立 CI 待 GitHub 可达后裁决。
