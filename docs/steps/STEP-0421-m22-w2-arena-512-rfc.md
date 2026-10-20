# STEP-0421 — M22: RFC-0029 第二次修订——guest 内存 256→512 MiB，trap 类再瓦解，census 117/136、0 DIFF（NON-DOMAIN 4→1）

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，RFC 预算表修订片 2）
> - 证据级别：internal-fixture / runtime-verified（实验阶梯 + census）

## 1. 交付内容

**512MiB 实验阶梯**（复用 0401 法）：4 个 256MiB 下仍 trap 的最深
walker 闭包（gw_condition/general_while/fixed_guard_chain/scalar_ir）
在 512MiB arena + 足燃料下**全部到达 typed 拒绝**（STATEMENT /
CALL-ARGUMENT / CALL-SHAPE / GWPACK-OTHER）——trap 类 = 纯 arena
耗尽再证（非值损坏，二重确认）。

**RFC-0029 第二次修订**（0351/0402 同步面全量）：
- codegen `ARENA_LIMIT` 256→512 MiB、`MEMORY_PAGES` 4096→8192；
- runtime `ARENA_LIMIT` 256→512 MiB、canonical tests 页数 ×5；
- `HostLimits` 默认 memory 256→512 MiB；runner 默认 512MiB→1GiB；
- census 探针燃料 120B→400B（注释登记）；
- **RFC amendment 段含 treadmill 关切正式登记**：单调-per-run arena
  （仅 cabi_post_run 回收）随每波机器函数解锁而增长，逐波加倍非长
  策；正解 = 自宿主编译循环内按函数复用 arena（机器重设计，另行
  跟踪）。

## 2. 前沿更新

- census **117/136、0 DIFF 维持**；**NON-DOMAIN 4→1**（3 户走出
  trap 至 typed 门）；GWPACK-OTHER 13→16（级联）；残余 trap 1 户
  （最深端，下片重定）。
- 下一刀主线：GWPACK-OTHER 16 户（gw 机器族形状手术模板③④）+
  残余 trap 1 户。

## 3. 事故与登记

- 无新事故；treadmill 关切入 RFC（见 §1）。

## 4. 验证与预算

- census 终值：flipped=117 diffs=0 total=136；HIST GWPACK-OTHER 16 /
  CALL-SHAPE 2 / NON-DOMAIN 1；探针副本按纪律删除。
- 固件：无新测试（常量修订零测试面）；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 5. STEP-0422 维护片（同日）

control_if_function_ir 两处 `set __h356 = text.length(cell)`（深嵌上下
文）改 fresh-let——门未移动（GWPACK-OTHER 维持），改动无害保留
（消两处潜在门）；census 117/136、0 DIFF 维持。16 户 GWPACK-OTHER
全属深嵌族，确认为模板③④的体量工作（多片 grind），无快刀。
