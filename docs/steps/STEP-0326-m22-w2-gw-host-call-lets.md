# STEP-0326: M22 W2 gw 宿主调用 let/set 与 bytes.at 参数 cursor

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（完成计划 A 队列 host-call let-RHS 首批）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

- `while_bytes_at_rhs_packed`：cursor 从"仅 u64 cell"放宽为"u64 cell 或 u64 参数"
  （参数 cursor 无 read 指令、值 id 直用参数号）；返回格式扩为
  `kind\nresult\ncount\npieces` 四行，两个消费者（`gw_rhs_packed` bytes.at 分支与
  `gw_compare_packed` 内层）同步更新为按 count 分配 id。
- 验证落地（探针字节全等，均 general-while 函数）：
  - 顶层 `let b = sico.text.encode(w)` + 循环体 `set n = sico.bytes.length(b)`；
  - `set x = sico.bytes.at(b, i, I64.literal(0))`（参数 cursor）。

## Measured results（本机真实 runner）

- 两形状对 Rust oracle 字节全等；typed 拒绝（I64 参数 cursor → CALL-TYPE）+
  恢复全等；套件 **58/58** 绿（含 bytes.at 两个旧消费者的格式迁移回归）。

## Honest residuals

- 本步无新增翻绿函数：parser 维持 12/115，**R3 连续停滞 2/4**（连续第二个
  canary 零增长实现步；按预登记止损规则，再两步无增长将触发 M23 Route B 评估）。
- 探针精确映射的残余缺口（下一切片输入）：
  - 嵌套宿主调用实参（`bytes.length(text.encode(w))` 内嵌、`bytes.at` 的
    cell-src 路径在 while 体内）→ GWPACK-OTHER / EXPRESSION；
  - if 分支内的 `let`（while 体内）→ STATEMENT；
  - 无 while 函数（is_digit_token/is_string_token/has_return_at 等）走
    control_if 机器，其宿主调用支持是独立缺口；
  - json_string_token/is_canonical_magnitude/decimal_at_most 需上述组合。
- M22 NO-GO；前缀前沿 `lex_words`/CALL-TARGET 不变。
